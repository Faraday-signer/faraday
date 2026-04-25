/**
 * Faraday Squads Indexer — Cloudflare Worker.
 *
 * Maintains a `member pubkey -> multisig PDAs` reverse index that the
 * dashboard queries on wallet connect. Solana RPC can't answer this
 * question efficiently (the members vec lives at a variable byte offset
 * inside Squads' Multisig account), so we build the index ourselves by
 * watching new on-chain events.
 *
 * Two intake paths:
 *   1. Helius webhook  — POST /webhook
 *      Helius is configured to fire on every Squads V4 program tx.
 *      We parse `multisig_create_v2` instructions and add an entry for
 *      each member.
 *   2. Dashboard report — POST /report
 *      The dashboard already discovers multisigs via tx-history walks
 *      and Add-by-ID. It POSTs anything it learns so the index covers
 *      historical multisigs created before the webhook existed.
 *
 * Query path:
 *   GET /multisigs/<memberPubkey>
 *      Returns `[{ accountId, label?, signature, createdAt }]`.
 *
 * The index is *additive* — entries never get removed, so the worst
 * staleness mode is "shows a multisig you've been removed from". The
 * dashboard re-fetches each on-chain to filter that.
 */

import bs58 from "bs58";

interface Env {
  MULTISIG_INDEX: KVNamespace;
  HELIUS_AUTH_TOKEN: string;
}

interface MemberEntry {
  accountId: string;
  signature: string;
  label?: string;
  createdAt: number;
}

const SQUADS_PROGRAM_ID = "SQDS4ep65T869zMMBKyuUq6aD6EgTu8psMjkvj52pCf";
const MULTISIG_CREATE_V2_DISC_HEX = "32ddc75d28f58be9";

const CORS = {
  "Access-Control-Allow-Origin": "*",
  "Access-Control-Allow-Methods": "GET, POST, OPTIONS",
  "Access-Control-Allow-Headers": "Content-Type, Authorization",
};

export default {
  async fetch(req: Request, env: Env): Promise<Response> {
    const url = new URL(req.url);

    if (req.method === "OPTIONS") {
      return new Response(null, { status: 204, headers: CORS });
    }

    try {
      if (req.method === "POST" && url.pathname === "/webhook") {
        return await handleWebhook(req, env);
      }
      if (req.method === "POST" && url.pathname === "/report") {
        return await handleReport(req, env);
      }
      if (req.method === "GET" && url.pathname.startsWith("/multisigs/")) {
        return await handleQuery(url.pathname.slice("/multisigs/".length), env);
      }
      if (req.method === "GET" && url.pathname === "/health") {
        return json({ ok: true });
      }
      return new Response("Not found", { status: 404, headers: CORS });
    } catch (e) {
      const message = e instanceof Error ? e.message : String(e);
      return json({ error: message }, 500);
    }
  },
};

// ── Webhook intake (Helius) ─────────────────────────────────────────────────

async function handleWebhook(req: Request, env: Env): Promise<Response> {
  const auth = req.headers.get("authorization") ?? "";
  if (!env.HELIUS_AUTH_TOKEN || auth !== env.HELIUS_AUTH_TOKEN) {
    return new Response("Unauthorized", { status: 401, headers: CORS });
  }

  const events = await req.json<unknown>();
  if (!Array.isArray(events)) {
    return json({ error: "expected array body" }, 400);
  }

  let indexed = 0;
  for (const event of events as HeliusEnhancedTx[]) {
    if (event.source !== "SQUADS_V4") continue;
    for (const ix of event.instructions ?? []) {
      const created = parseCreateInstruction(ix);
      if (!created) continue;
      const entry: MemberEntry = {
        accountId: created.accountId,
        signature: event.signature ?? "",
        label: created.label,
        createdAt: event.timestamp ?? Math.floor(Date.now() / 1000),
      };
      for (const member of created.members) {
        await appendForMember(env, member, entry);
      }
      indexed += 1;
    }
  }

  return json({ ok: true, indexed });
}

// ── Dashboard report (covers historical multisigs) ──────────────────────────

interface ReportBody {
  accountId: string;
  members: string[];
  signature?: string;
  label?: string;
  createdAt?: number;
}

async function handleReport(req: Request, env: Env): Promise<Response> {
  const body = await req.json<ReportBody>().catch(() => null);
  if (!body || typeof body.accountId !== "string" || !Array.isArray(body.members)) {
    return json({ error: "invalid body" }, 400);
  }
  if (!isValidPubkey(body.accountId)) return json({ error: "invalid accountId" }, 400);
  if (body.members.some((m) => !isValidPubkey(m))) {
    return json({ error: "invalid member pubkey" }, 400);
  }

  const entry: MemberEntry = {
    accountId: body.accountId,
    signature: body.signature ?? "",
    label: body.label,
    createdAt: body.createdAt ?? Math.floor(Date.now() / 1000),
  };
  for (const member of body.members) {
    await appendForMember(env, member, entry);
  }
  return json({ ok: true, members: body.members.length });
}

// ── Query path (dashboard read) ─────────────────────────────────────────────

async function handleQuery(member: string, env: Env): Promise<Response> {
  if (!isValidPubkey(member)) {
    return json({ error: "invalid member pubkey" }, 400);
  }
  const stored = await env.MULTISIG_INDEX.get<MemberEntry[]>(`member:${member}`, "json");
  return json(stored ?? []);
}

// ── KV helpers ──────────────────────────────────────────────────────────────

async function appendForMember(env: Env, member: string, entry: MemberEntry): Promise<void> {
  const key = `member:${member}`;
  const existing = (await env.MULTISIG_INDEX.get<MemberEntry[]>(key, "json")) ?? [];
  // Replace any previous entry for this multisig (keeps signature/label fresh).
  const filtered = existing.filter((e) => e.accountId !== entry.accountId);
  filtered.push(entry);
  // Cap per-member to avoid runaway storage from a wallet that joins many DAOs.
  const trimmed = filtered.slice(-200);
  await env.MULTISIG_INDEX.put(key, JSON.stringify(trimmed));
}

// ── Squads instruction parser ───────────────────────────────────────────────

interface CreatedMultisig {
  accountId: string;
  members: string[];
  label?: string;
}

interface HeliusInstruction {
  programId: string;
  accounts: string[];
  data: string;          // base58
  innerInstructions?: HeliusInstruction[];
}

interface HeliusEnhancedTx {
  source?: string;
  signature?: string;
  timestamp?: number;
  instructions?: HeliusInstruction[];
}

function parseCreateInstruction(ix: HeliusInstruction): CreatedMultisig | null {
  if (ix.programId !== SQUADS_PROGRAM_ID) return null;
  let raw: Uint8Array;
  try {
    raw = bs58.decode(ix.data);
  } catch {
    return null;
  }
  if (raw.length < 8) return null;
  if (toHex(raw.slice(0, 8)) !== MULTISIG_CREATE_V2_DISC_HEX) return null;
  if (!ix.accounts || ix.accounts.length < 3) return null;

  const decoded = decodeCreateV2(raw);
  if (!decoded) return null;
  return {
    accountId: ix.accounts[2], // multisigCreateV2: [programConfig, treasury, multisig, createKey, creator, systemProgram]
    members: decoded.members,
    label: decoded.label,
  };
}

interface DecodedCreate {
  members: string[];
  label?: string;
}

function decodeCreateV2(data: Uint8Array): DecodedCreate | null {
  let p = 8; // skip discriminator
  if (p >= data.length) return null;

  // configAuthority: Option<Pubkey>
  const cfgTag = data[p++];
  if (cfgTag === 1) p += 32;
  else if (cfgTag !== 0) return null;

  // threshold: u16
  if (p + 2 > data.length) return null;
  p += 2;

  // members: Vec<Member { key: 32 + permissions: 1 }>
  if (p + 4 > data.length) return null;
  const n = readU32(data, p);
  p += 4;
  if (p + n * 33 > data.length) return null;
  const members: string[] = [];
  for (let i = 0; i < n; i++) {
    members.push(bs58.encode(data.slice(p, p + 32)));
    p += 33; // skip key + permissions
  }

  // timeLock: u32
  if (p + 4 > data.length) return null;
  p += 4;

  // rentCollector: Option<Pubkey>
  if (p >= data.length) return null;
  const rcTag = data[p++];
  if (rcTag === 1) p += 32;
  else if (rcTag !== 0) return null;

  // memo: Option<String>
  if (p >= data.length) return { members };
  const memoTag = data[p++];
  if (memoTag !== 1) return { members };
  if (p + 4 > data.length) return { members };
  const len = readU32(data, p);
  p += 4;
  if (p + len > data.length) return { members };
  let label: string | undefined;
  try {
    label = new TextDecoder("utf-8", { fatal: true, ignoreBOM: false }).decode(data.slice(p, p + len));
  } catch {
    // ignored — non-utf8 memo
  }
  return { members, label };
}

// ── Pure helpers ────────────────────────────────────────────────────────────

function isValidPubkey(s: string): boolean {
  if (typeof s !== "string" || s.length < 32 || s.length > 44) return false;
  try {
    return bs58.decode(s).length === 32;
  } catch {
    return false;
  }
}

function readU32(data: Uint8Array, off: number): number {
  return ((data[off] | (data[off + 1] << 8) | (data[off + 2] << 16)) >>> 0) +
    data[off + 3] * 0x01000000;
}

function toHex(bytes: Uint8Array): string {
  return Array.from(bytes, (b) => b.toString(16).padStart(2, "0")).join("");
}

function json(body: unknown, status = 200): Response {
  return new Response(JSON.stringify(body), {
    status,
    headers: { ...CORS, "Content-Type": "application/json" },
  });
}
