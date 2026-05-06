import { address as toAddress, type Address } from "@solana/kit";

import { RPC_URL, solanaRpc } from "./sol-client";

const TOKEN_PROGRAM_ID = "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA";
const TOKEN_2022_PROGRAM_ID = "TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb";

export const WSOL_MINT = "So11111111111111111111111111111111111111112";

const JUPITER_PRICE_URL = "https://lite-api.jup.ag/price/v3";
const JUPITER_VERIFIED_URL = "https://lite-api.jup.ag/tokens/v2/tag?query=verified";

const VERIFIED_SET_TTL_MS = 60 * 60 * 1000;
const PRICE_BATCH_SIZE = 100;

export type TokenProgram = "spl-token" | "spl-token-2022";

export interface Token {
  mint: string;
  programId: TokenProgram;
  symbol: string;
  name: string;
  logoUrl: string | null;
  decimals: number;
  amountRaw: bigint;
  amountUi: number;
  usdValue: number | null;
  pricePerToken: number | null;
  verified: boolean;
}

interface HeliusAsset {
  interface?: string;
  id?: string;
  content?: {
    metadata?: { name?: string; symbol?: string };
    links?: { image?: string };
    files?: Array<{ uri?: string; cdn_uri?: string }>;
  };
  token_info?: {
    balance?: number | string;
    decimals?: number;
    symbol?: string;
    token_program?: string;
  };
}

interface HeliusAssetsResponse {
  result?: { items?: HeliusAsset[]; total?: number };
  error?: { message?: string };
}

async function fetchHeliusAssets(owner: string): Promise<HeliusAsset[]> {
  const body = {
    jsonrpc: "2.0",
    id: "faraday-tokens",
    method: "getAssetsByOwner",
    params: {
      ownerAddress: owner,
      page: 1,
      limit: 1000,
      displayOptions: {
        showFungible: true,
        showNativeBalance: false,
        showZeroBalance: false
      }
    }
  };

  const res = await fetch(RPC_URL, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(body)
  });

  if (!res.ok) throw new Error(`Helius DAS HTTP ${res.status}`);

  const json = (await res.json()) as HeliusAssetsResponse;
  if (json.error) throw new Error(`Helius DAS error: ${json.error.message ?? "unknown"}`);

  const items = json.result?.items ?? [];
  return items.filter(
    (a) => a.interface === "FungibleToken" || a.interface === "FungibleAsset"
  );
}

function programFromHelius(token_program: string | undefined): TokenProgram {
  return token_program === TOKEN_2022_PROGRAM_ID ? "spl-token-2022" : "spl-token";
}

function pickLogoUrl(asset: HeliusAsset): string | null {
  const direct = asset.content?.links?.image;
  if (typeof direct === "string" && direct.length > 0) return direct;
  const file = asset.content?.files?.find((f) => typeof (f.cdn_uri ?? f.uri) === "string");
  return file ? (file.cdn_uri ?? file.uri ?? null) : null;
}

function bigintFromBalance(raw: number | string | undefined): bigint {
  if (raw === undefined || raw === null) return 0n;
  if (typeof raw === "bigint") return raw;
  return BigInt(typeof raw === "number" ? Math.floor(raw).toString() : raw);
}

type JupiterPriceResponse = Record<string, { usdPrice?: number | string } | null>;

export async function fetchJupiterPrices(mints: string[]): Promise<Map<string, number>> {
  if (mints.length === 0) return new Map();
  const out = new Map<string, number>();

  for (let i = 0; i < mints.length; i += PRICE_BATCH_SIZE) {
    const batch = mints.slice(i, i + PRICE_BATCH_SIZE);
    const url = `${JUPITER_PRICE_URL}?ids=${batch.join(",")}`;

    let json: JupiterPriceResponse;
    try {
      const res = await fetch(url, { method: "GET" });
      if (!res.ok) continue;
      json = (await res.json()) as JupiterPriceResponse;
    } catch {
      continue;
    }

    for (const [mint, entry] of Object.entries(json)) {
      const raw = entry?.usdPrice;
      if (raw === undefined || raw === null) continue;
      const num = typeof raw === "number" ? raw : Number(raw);
      if (Number.isFinite(num) && num > 0) out.set(mint, num);
    }
  }

  return out;
}

interface CachedVerified {
  set: Set<string>;
  fetchedAt: number;
}

let verifiedCache: CachedVerified | null = null;
let verifiedInflight: Promise<Set<string>> | null = null;

interface JupiterVerifiedItem {
  id?: string;
  address?: string;
}

export async function fetchJupiterVerifiedSet(): Promise<Set<string>> {
  const now = Date.now();
  if (verifiedCache && now - verifiedCache.fetchedAt < VERIFIED_SET_TTL_MS) {
    return verifiedCache.set;
  }
  if (verifiedInflight) return verifiedInflight;

  verifiedInflight = (async () => {
    try {
      const res = await fetch(JUPITER_VERIFIED_URL, { method: "GET" });
      if (!res.ok) throw new Error(`Jupiter verified HTTP ${res.status}`);
      const json = (await res.json()) as JupiterVerifiedItem[] | { items?: JupiterVerifiedItem[] };
      const list: JupiterVerifiedItem[] = Array.isArray(json) ? json : (json.items ?? []);
      const set = new Set<string>();
      for (const item of list) {
        const mint = item.id ?? item.address;
        if (typeof mint === "string" && mint.length > 0) set.add(mint);
      }
      verifiedCache = { set, fetchedAt: Date.now() };
      return set;
    } catch {
      const empty = new Set<string>();
      verifiedCache = { set: empty, fetchedAt: Date.now() };
      return empty;
    } finally {
      verifiedInflight = null;
    }
  })();

  return verifiedInflight;
}

interface RpcParsedTokenAccount {
  account: {
    data: {
      parsed: {
        info: {
          mint: string;
          tokenAmount: { amount: string; decimals: number };
        };
      };
    };
    owner: string;
  };
}

async function fetchRpcTokensForProgram(
  owner: Address,
  programId: string
): Promise<Token[]> {
  const result = await (
    solanaRpc as unknown as {
      getTokenAccountsByOwner: (
        owner: Address,
        filter: { programId: Address },
        config: { encoding: "jsonParsed" }
      ) => { send: () => Promise<{ value: RpcParsedTokenAccount[] }> };
    }
  )
    .getTokenAccountsByOwner(
      owner,
      { programId: toAddress(programId) },
      { encoding: "jsonParsed" }
    )
    .send();

  const program: TokenProgram =
    programId === TOKEN_2022_PROGRAM_ID ? "spl-token-2022" : "spl-token";

  const out: Token[] = [];
  for (const entry of result.value) {
    const info = entry.account.data.parsed.info;
    const raw = BigInt(info.tokenAmount.amount);
    if (raw === 0n) continue;
    const decimals = info.tokenAmount.decimals;
    out.push({
      mint: info.mint,
      programId: program,
      symbol: "",
      name: "",
      logoUrl: null,
      decimals,
      amountRaw: raw,
      amountUi: Number(raw) / 10 ** decimals,
      usdValue: null,
      pricePerToken: null,
      verified: false
    });
  }
  return out;
}

async function fetchOwnedTokensFallback(owner: string): Promise<Token[]> {
  const addr = toAddress(owner);
  const [classic, token2022] = await Promise.all([
    fetchRpcTokensForProgram(addr, TOKEN_PROGRAM_ID).catch(() => []),
    fetchRpcTokensForProgram(addr, TOKEN_2022_PROGRAM_ID).catch(() => [])
  ]);
  return [...classic, ...token2022];
}

export async function fetchOwnedTokens(owner: string): Promise<Token[]> {
  toAddress(owner);

  let assets: HeliusAsset[];
  try {
    assets = await fetchHeliusAssets(owner);
  } catch {
    return fetchOwnedTokensFallback(owner);
  }

  const tokens: Token[] = [];
  for (const a of assets) {
    if (!a.id) continue;
    const decimals = a.token_info?.decimals ?? 0;
    const raw = bigintFromBalance(a.token_info?.balance);
    if (raw === 0n) continue;

    tokens.push({
      mint: a.id,
      programId: programFromHelius(a.token_info?.token_program),
      symbol: a.content?.metadata?.symbol ?? a.token_info?.symbol ?? "",
      name: a.content?.metadata?.name ?? "",
      logoUrl: pickLogoUrl(a),
      decimals,
      amountRaw: raw,
      amountUi: Number(raw) / 10 ** decimals,
      usdValue: null,
      pricePerToken: null,
      verified: false
    });
  }

  const mints = tokens.map((t) => t.mint);
  const [prices, verified] = await Promise.all([
    fetchJupiterPrices(mints).catch(() => new Map<string, number>()),
    fetchJupiterVerifiedSet()
  ]);

  for (const t of tokens) {
    const price = prices.get(t.mint);
    if (price !== undefined) {
      t.pricePerToken = price;
      t.usdValue = price * t.amountUi;
    }
    t.verified = verified.has(t.mint);
  }

  return tokens;
}
