/**
 * Maps Squads' on-chain "multisig" primitives onto finance-friendly
 * vocabulary surfaced in the UI:
 *   Squads "Multisig"     -> Account
 *   Squads "members"      -> approvers
 *   Squads "threshold"    -> approvalsRequired
 */

import { Connection, PublicKey } from "@solana/web3.js";
import * as multisig from "@sqds/multisig";

/**
 * RPC endpoint. Override via VITE_RPC_URL (a Helius / QuickNode / Triton
 * URL with an API key). Public mainnet rejects getProgramAccounts on big
 * programs like Squads, so account discovery only works against a custom
 * RPC. The fallback keeps the create-tx path working without config.
 */
const FALLBACK_RPC_URL = "https://api.mainnet-beta.solana.com";

function pickRpcUrl(): string {
  const fromEnv = typeof import.meta.env.VITE_RPC_URL === "string"
    ? import.meta.env.VITE_RPC_URL.trim()
    : "";
  return fromEnv.length > 0 ? fromEnv : FALLBACK_RPC_URL;
}

export const RPC_URL = pickRpcUrl();
export const IS_PUBLIC_RPC = RPC_URL === FALLBACK_RPC_URL;
export const connection = new Connection(RPC_URL, "confirmed");

/**
 * Cloudflare Worker URL hosting our member -> multisigs index. When unset,
 * the dashboard falls back to tx-history-only discovery (catches creators
 * + active members, misses passive approvers).
 */
export const INDEXER_URL: string =
  (typeof import.meta.env.VITE_INDEXER_URL === "string"
    ? import.meta.env.VITE_INDEXER_URL.trim()
    : "") || "";

export interface CompanyAccount {
  readonly accountId: string;
  readonly approvalsRequired: number;
  readonly approverCount: number;
  readonly label?: string;
}

/** Single-account fetch — works on every RPC since it's a getAccountInfo. */
export async function fetchAccountById(pda: PublicKey): Promise<CompanyAccount | null> {
  try {
    const ms = await multisig.accounts.Multisig.fromAccountAddress(connection, pda);
    return {
      accountId: pda.toBase58(),
      approvalsRequired: ms.threshold,
      approverCount: ms.members.length,
    };
  } catch {
    return null;
  }
}

export interface AccountDetails {
  readonly accountId: string;
  readonly approvalsRequired: number;
  readonly approvers: ReadonlyArray<{ key: string; permissions: number }>;
  readonly vaultId: string;        // default vault (index 0) — where funds live
  readonly vaultBalanceLamports: number;
}

export async function fetchAccountDetails(pda: PublicKey): Promise<AccountDetails | null> {
  try {
    const ms = await multisig.accounts.Multisig.fromAccountAddress(connection, pda);
    const [vault] = multisig.getVaultPda({ multisigPda: pda, index: 0 });
    const balance = await connection.getBalance(vault).catch(() => 0);
    return {
      accountId: pda.toBase58(),
      approvalsRequired: ms.threshold,
      approvers: ms.members.map((m) => ({
        key: m.key.toBase58(),
        permissions: m.permissions.mask,
      })),
      vaultId: vault.toBase58(),
      vaultBalanceLamports: balance,
    };
  } catch {
    return null;
  }
}

const MULTISIG_CREATE_V2_DISC = new Uint8Array([
  0x32, 0xdd, 0xc7, 0x5d, 0x28, 0xf5, 0x8b, 0xe9,
]);

/**
 * Discover multisigs the wallet *created*, by walking its recent tx history
 * and finding `multisig_create_v2` instructions. Reliable on Helius and
 * any RPC that exposes `getSignaturesForAddress` + `getTransaction`,
 * because we never touch `getProgramAccounts`.
 *
 * Limitation: doesn't surface multisigs the wallet was *added to* by
 * someone else. For that, the user uses "Add by ID".
 */
export interface DiscoveredMultisig {
  readonly accountId: string;
  readonly signature: string;
  readonly label?: string;
}

/**
 * Discover multisigs the wallet either *created* or *interacted with*. We
 * walk the wallet's tx history looking at any Squads instruction:
 *   - `multisigCreateV2` → multisig PDA is at account index 2 (and we can
 *     also pull the optional memo for free)
 *   - any other Squads instruction → multisig PDA is conventionally at
 *     account index 0 (proposal_create, proposal_approve, vault_tx_*, ...)
 * Each candidate is then verified on-chain (must deserialize as Multisig
 * AND list this wallet as a member). That single getAccountInfo per
 * candidate is the only price we pay for catching wallets that were
 * *added* to a multisig instead of creating it.
 */
export async function findKnownMultisigs(
  wallet: PublicKey,
  limit = 50,
): Promise<DiscoveredMultisig[]> {
  const sigs = await connection.getSignaturesForAddress(wallet, { limit });
  const txs = await Promise.all(
    sigs
      .filter((s) => !s.err)
      .map(async (s) => {
        const tx = await connection
          .getTransaction(s.signature, {
            maxSupportedTransactionVersion: 0,
            commitment: "confirmed",
          })
          .catch(() => null);
        return { signature: s.signature, tx };
      }),
  );

  // accountId -> { earliest signature, label if from create }
  const candidates = new Map<string, { signature: string; label?: string }>();
  for (const { signature, tx } of txs) {
    if (!tx) continue;
    const keys = tx.transaction.message.staticAccountKeys;
    for (const ix of tx.transaction.message.compiledInstructions) {
      if (!keys[ix.programIdIndex].equals(multisig.PROGRAM_ID)) continue;
      if (ix.accountKeyIndexes.length === 0) continue;

      const data = ix.data;
      const isCreate = data.length >= 8 && hasDiscriminator(data, MULTISIG_CREATE_V2_DISC);
      const pdaIndex = isCreate ? 2 : 0;
      if (ix.accountKeyIndexes.length <= pdaIndex) continue;

      const accountId = keys[ix.accountKeyIndexes[pdaIndex]].toBase58();
      const existing = candidates.get(accountId);
      const label = isCreate ? decodeMemo(data) : existing?.label;
      // Prefer the create signature when we have it (it's the canonical
      // origin); otherwise keep whatever we saw first.
      const sig = isCreate ? signature : existing?.signature ?? signature;
      candidates.set(accountId, { signature: sig, label });
    }
  }

  // Verify each candidate is a real Multisig the wallet is a member of.
  // Bad candidates (other Squads account types) just throw on deserialize.
  const verified = await Promise.all(
    [...candidates.entries()].map(async ([accountId, meta]): Promise<DiscoveredMultisig | null> => {
      try {
        const ms = await multisig.accounts.Multisig.fromAccountAddress(
          connection,
          new PublicKey(accountId),
        );
        if (!ms.members.some((m) => m.key.equals(wallet))) return null;
        return { accountId, signature: meta.signature, label: meta.label };
      } catch {
        return null;
      }
    }),
  );
  return verified.filter((v): v is DiscoveredMultisig => v !== null);
}

function hasDiscriminator(data: Uint8Array, disc: Uint8Array): boolean {
  for (let i = 0; i < 8; i++) if (data[i] !== disc[i]) return false;
  return true;
}

// ── Worker-backed indexer (covers passive approvers) ─────────────────────────

interface IndexerEntry {
  accountId: string;
  signature: string;
  label?: string;
  createdAt: number;
}

/** Read the member's known multisigs from our Cloudflare indexer. */
export async function fetchIndexedMultisigs(member: PublicKey): Promise<DiscoveredMultisig[]> {
  if (!INDEXER_URL) return [];
  const res = await fetch(`${INDEXER_URL}/multisigs/${member.toBase58()}`);
  if (!res.ok) return [];
  const rows = (await res.json()) as IndexerEntry[];
  return rows.map((r) => ({
    accountId: r.accountId,
    signature: r.signature,
    label: r.label,
  }));
}

/** Tell the indexer about a multisig we've learned of locally. */
export async function reportMultisig(
  accountId: string,
  members: string[],
  extras: { signature?: string; label?: string } = {},
): Promise<void> {
  if (!INDEXER_URL) return;
  try {
    await fetch(`${INDEXER_URL}/report`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({
        accountId,
        members,
        signature: extras.signature,
        label: extras.label,
        createdAt: Math.floor(Date.now() / 1000),
      }),
    });
  } catch {
    // Best-effort — failures don't block the user.
  }
}

/**
 * Walk the multisigCreateV2 instruction-data bytes and pull out the optional
 * memo. The memo isn't stored on-chain in the Multisig account itself, only
 * in this create-tx instruction — so this is the only place it can come from
 * for accounts not created from this dashboard session.
 */
function decodeMemo(data: Uint8Array): string | undefined {
  let p = 8; // skip 8-byte discriminator
  if (p >= data.length) return;

  // configAuthority: Option<Pubkey>
  const cfgTag = data[p++];
  if (cfgTag === 1) p += 32;
  else if (cfgTag !== 0) return;

  // threshold: u16
  if (p + 2 > data.length) return;
  p += 2;

  // members: Vec<Member { key: Pubkey(32) + permissions: u8 }>
  if (p + 4 > data.length) return;
  const n = readU32(data, p);
  p += 4 + n * 33;

  // timeLock: u32
  if (p + 4 > data.length) return;
  p += 4;

  // rentCollector: Option<Pubkey>
  if (p >= data.length) return;
  const rcTag = data[p++];
  if (rcTag === 1) p += 32;
  else if (rcTag !== 0) return;

  // memo: Option<String>
  if (p >= data.length) return;
  const memoTag = data[p++];
  if (memoTag !== 1) return;
  if (p + 4 > data.length) return;
  const len = readU32(data, p);
  p += 4;
  if (p + len > data.length) return;
  try {
    return new TextDecoder("utf-8", { fatal: true }).decode(data.slice(p, p + len));
  } catch {
    return;
  }
}

function readU32(data: Uint8Array, off: number): number {
  return ((data[off] | (data[off + 1] << 8) | (data[off + 2] << 16)) >>> 0) +
    (data[off + 3] * 0x01000000);
}

export class RpcDiscoveryUnsupported extends Error {
  constructor() {
    super("This RPC doesn't allow scanning Squads multisigs (public mainnet endpoints typically block getProgramAccounts on the Squads program). Use a custom RPC, or look up a known account by ID.");
    this.name = "RpcDiscoveryUnsupported";
  }
}

export async function listAccountsForApprover(approver: PublicKey): Promise<CompanyAccount[]> {
  let records;
  try {
    records = await multisig.accounts.Multisig.gpaBuilder().run(connection);
  } catch (e) {
    const msg = e instanceof Error ? e.message : String(e);
    if (/forbidden|disabled|unauthorized|overloaded|index service|try again|429|403|503/i.test(msg)) {
      throw new RpcDiscoveryUnsupported();
    }
    throw e;
  }

  const out: CompanyAccount[] = [];
  for (const { pubkey, account } of records) {
    try {
      const [ms] = multisig.accounts.Multisig.deserialize(account.data);
      const isApprover = ms.members.some((m) => m.key.equals(approver));
      if (!isApprover) continue;
      out.push({
        accountId: pubkey.toBase58(),
        approvalsRequired: ms.threshold,
        approverCount: ms.members.length,
      });
    } catch {
      // Other Squads account types share the program; skip non-Multisig data.
    }
  }
  return out;
}
