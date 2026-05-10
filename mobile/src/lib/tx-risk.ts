export type TxRiskLevel = "SAFE" | "WARNING" | "DANGER";

/**
 * Risk warnings come in three flavours, and the UI label/copy should
 * differ accordingly:
 *   - "fraud"    → tx shape matches a known scam pattern (drainer, etc.)
 *   - "failure"  → tx is broken (would fail simulation, can't analyze)
 *   - "quality"  → tx is unusual but not necessarily malicious
 *
 * Lumping all of these under "fraud detected" misleads the user. The
 * level banner aggregates categories to pick its own label.
 */
export type TxRiskCategory = "fraud" | "failure" | "quality";

export interface TxRiskWarning {
  severity: "critical" | "warning";
  category: TxRiskCategory;
  title: string;
  /** Single-sentence statement of what the detector found. */
  description: string;
  /**
   * Plain-English "why this matters + what to do." Always present. Aim for
   * a single calm paragraph that names the impact and gives the user a
   * decision frame, without accusing the dApp of fraud unless that's what
   * the detector is actually for.
   */
  explanation: string;
  /**
   * Optional verbose detail — raw simulation error, last failing log line,
   * etc. UI surfaces this behind a collapsible toggle so the default
   * WarningRow stays scannable but power users / debuggers can see
   * exactly what the detector saw.
   */
  details?: string;
}

/** Net balance change for a token in the user's wallet (negative = outgoing). */
export interface TokenChange {
  mint: string;
  symbol: string;
  amount: number;
}

export interface TxRiskReport {
  level: TxRiskLevel;
  warnings: TxRiskWarning[];
  /** Token balance changes (including SOL) derived from simulation. */
  tokenChanges: TokenChange[];
  /** Net SOL change for the user (negative = outgoing). Null when simulation failed. */
  solChangeSol: number | null;
  simulationFailed: boolean;
}

// --- Internal simulation types ---

interface SimTokenBalance {
  accountIndex: number;
  mint: string;
  owner: string;
  uiTokenAmount: { uiAmount: number | null };
}

interface SimulationResult {
  err: unknown;
  logs: string[] | null;
  preBalances: number[];
  postBalances: number[];
  preTokenBalances: SimTokenBalance[];
  postTokenBalances: SimTokenBalance[];
  unitsConsumed: number;
}

interface ParsedInstruction {
  programId: string;
  accounts: string[];
  data: Uint8Array;
}

interface ParsedTransaction {
  numSignatures: number;
  accountKeys: string[];
  instructions: ParsedInstruction[];
}

// --- Constants ---

const LAMPORTS_PER_SOL = 1_000_000_000;
const TOKEN_PROGRAM_ID = "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA";
const TOKEN_2022_PROGRAM_ID = "TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb";
const COMPUTE_BUDGET_PROGRAM_ID = "ComputeBudget111111111111111111111111111111";
const STAKE_PROGRAM_ID = "Stake11111111111111111111111111111111111111";

/**
 * Stake Program instruction discriminators (just the ones we care about
 * for risk analysis — the full enum is large).
 */
const STAKE_IX_AUTHORIZE = 1;
const STAKE_IX_AUTHORIZE_CHECKED = 10;
const STAKE_IX_AUTHORIZE_WITH_SEED = 8;
const STAKE_IX_AUTHORIZE_CHECKED_WITH_SEED = 11;
const U64_MAX = (1n << 64n) - 1n;

const TOKEN_IX_APPROVE = 4;
const TOKEN_IX_APPROVE_CHECKED = 13;
const TOKEN_IX_SET_AUTHORITY = 6;
const TOKEN_IX_CLOSE_ACCOUNT = 9;

const AUTH_TYPE_MINT_TOKENS = 0;
const AUTH_TYPE_FREEZE_ACCOUNT = 1;
const AUTH_TYPE_ACCOUNT_OWNER = 2;
const AUTH_TYPE_CLOSE_ACCOUNT = 3;

const DRAIN_WIPE_RATIO = 0.95;
/**
 * Anything between this and DRAIN_WIPE_RATIO surfaces a "Significant
 * Outflow" quality warning — not full fraud, but worth a pause. A
 * 70% sweep can be legitimate (paying off a vault, consolidating
 * funds) but is unusual enough that the user should glance at the
 * recipient.
 */
const SIGNIFICANT_OUTFLOW_RATIO = 0.5;
const HIGH_VALUE_SOL_THRESHOLD = 10;
const OVERSIZED_PRIORITY_FEE_SOL = 0.05;
const MULTI_ASSET_DRAIN_THRESHOLD = 3;
const SET_COMPUTE_UNIT_PRICE_DISCRIMINATOR = 3;
const SIMULATION_TIMEOUT_MS = 15_000;
/**
 * Hard deadline for the whole analyzer, including simulate + symbol
 * fetches + detector passes. Defensive: a hostile RPC or page can't
 * stall the preview indefinitely as a way to coerce the user into
 * signing without a risk check.
 */
const OVERALL_ANALYSIS_TIMEOUT_MS = 30_000;
const TOKEN_SYMBOL_TIMEOUT_MS = 5_000;

// --- Impersonator detection constants ---

/**
 * Official mints for high-value tokens most commonly spoofed by drainers.
 * Key is the normalized (uppercase) ticker; value is the set of official mints.
 * Only tokens where we are certain of the canonical mint are included to
 * avoid false positives.
 */
const CANONICAL_MINTS: Record<string, string[]> = {
  USDC:  ["EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v"],
  USDT:  ["Es9vMFrzaCERmJfrF4H2FYD4KCoNkY11McCe8BenwNYB"],
  SOL:   ["So11111111111111111111111111111111111111112"],
  JUP:   ["JUPyiwrYJFskUPiHa7hkeR8VUtAeFoSYbKedZNsDvCN"],
  BONK:  ["DezXAZ8z7PnrnRJjz3wXBoRgixCa6xjnB7YaB1pPB263"],
  RAY:   ["4k3Dyjzvzp8eMZWUXbBCjEvwSkkk59S5iCNLY3QrkX6R"],
};

/**
 * Cyrillic and Greek characters that are visually indistinguishable from
 * their Latin equivalents. NFKD normalization handles fullwidth Latin
 * (e.g. ＵＳＤＣ) automatically; this map covers the script-confusables.
 */
const CONFUSABLE_MAP: Record<string, string> = {
  // Cyrillic uppercase
  А: "A", В: "B", С: "C", Е: "E", Н: "H", І: "I",
  К: "K", М: "M", О: "O", Р: "P", Т: "T", Х: "X",
  // Cyrillic lowercase
  а: "a", е: "e", о: "o", р: "p", с: "c", х: "x",
  // Greek uppercase
  Α: "A", Β: "B", Ε: "E", Ζ: "Z", Η: "H", Ι: "I",
  Κ: "K", Μ: "M", Ν: "N", Ο: "O", Ρ: "P", Τ: "T", Υ: "Y", Χ: "X",
  // Greek lowercase
  ο: "o", ν: "v",
};

const ZERO_WIDTH_RE = /[​-‍⁠﻿­]/g;

// --- Transaction parser (binary format, handles legacy and v0) ---

const BASE58_ALPHABET = "123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz";

function readCompactU16(bytes: Uint8Array, offset: number): { value: number; bytesRead: number } {
  let value = 0;
  let shift = 0;
  for (let i = 0; i < 3; i++) {
    if (offset + i >= bytes.length) throw new Error("readCompactU16: past end of buffer");
    const byte = bytes[offset + i];
    value |= (byte & 0x7f) << shift;
    if ((byte & 0x80) === 0) return { value, bytesRead: i + 1 };
    shift += 7;
  }
  throw new Error("readCompactU16: continuation past 3 bytes");
}

function base58Encode(bytes: Uint8Array): string {
  let num = 0n;
  for (const byte of bytes) num = num * 256n + BigInt(byte);
  let result = "";
  while (num > 0n) {
    const rem = num % 58n;
    num = num / 58n;
    result = BASE58_ALPHABET[Number(rem)] + result;
  }
  for (const byte of bytes) {
    if (byte === 0) result = "1" + result;
    else break;
  }
  return result || "1";
}

function parseTransaction(base64Tx: string): ParsedTransaction | null {
  try {
    const binary = atob(base64Tx);
    const bytes = new Uint8Array(binary.length);
    for (let i = 0; i < binary.length; i++) bytes[i] = binary.charCodeAt(i);

    let offset = 0;
    const sigsHeader = readCompactU16(bytes, offset);
    const numSignatures = sigsHeader.value;
    offset += sigsHeader.bytesRead + numSignatures * 64;

    if (bytes[offset] === 0x80) offset += 1; // versioned tx prefix

    offset += 3; // message header

    const accountsHeader = readCompactU16(bytes, offset);
    const numAccounts = accountsHeader.value;
    offset += accountsHeader.bytesRead;

    const accountKeys: string[] = [];
    for (let i = 0; i < numAccounts; i++) {
      if (offset + 32 > bytes.length) return null;
      accountKeys.push(base58Encode(bytes.slice(offset, offset + 32)));
      offset += 32;
    }

    offset += 32; // recent blockhash

    const ixHeader = readCompactU16(bytes, offset);
    const numIxs = ixHeader.value;
    offset += ixHeader.bytesRead;

    const instructions: ParsedInstruction[] = [];
    for (let i = 0; i < numIxs; i++) {
      const programIdIndex = bytes[offset++];
      const accountsLen = readCompactU16(bytes, offset);
      offset += accountsLen.bytesRead;
      const accountIndices: number[] = [];
      for (let j = 0; j < accountsLen.value; j++) accountIndices.push(bytes[offset + j]);
      offset += accountsLen.value;
      const dataLen = readCompactU16(bytes, offset);
      offset += dataLen.bytesRead;
      const data = bytes.slice(offset, offset + dataLen.value);
      offset += dataLen.value;
      instructions.push({
        programId: accountKeys[programIdIndex] ?? "",
        accounts: accountIndices.map((idx) => accountKeys[idx] ?? ""),
        data,
      });
    }

    return { numSignatures, accountKeys, instructions };
  } catch {
    return null;
  }
}

function readU64LEBigInt(bytes: Uint8Array, offset: number): bigint {
  let result = 0n;
  for (let i = 7; i >= 0; i--) result = (result << 8n) | BigInt(bytes[offset + i]);
  return result;
}

function readU64LE(bytes: Uint8Array, offset: number): number {
  let result = 0n;
  for (let i = 7; i >= 0; i--) result = (result << 8n) | BigInt(bytes[offset + i]);
  return Number(result);
}

// --- Simulation ---

async function simulate(txBase64: string, rpcUrl: string): Promise<SimulationResult> {
  const controller = new AbortController();
  const timer = setTimeout(() => controller.abort(), SIMULATION_TIMEOUT_MS);
  try {
    const response = await fetch(rpcUrl, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      signal: controller.signal,
      body: JSON.stringify({
        jsonrpc: "2.0",
        id: 1,
        method: "simulateTransaction",
        params: [
          txBase64,
          {
            encoding: "base64",
            commitment: "confirmed",
            replaceRecentBlockhash: true,
            sigVerify: false,
          },
        ],
      }),
    });
    if (!response.ok) throw new Error(`RPC ${response.status}`);
    const json = await response.json();
    if (json.error) throw new Error(json.error.message ?? "RPC error");
    return json.result.value as SimulationResult;
  } finally {
    clearTimeout(timer);
  }
}

// --- Token symbol resolution ---

function shortMint(mint: string): string {
  return `${mint.slice(0, 4)}…${mint.slice(-4)}`;
}

/** Wrapped SOL mint — used for SOL price lookup in the USD asymmetry detector. */
const WSOL_MINT = "So11111111111111111111111111111111111111112";
const PRICE_FETCH_TIMEOUT_MS = 5_000;

interface TokenMarketInfo {
  usdPrice: number;
  /** USD liquidity reported by Jupiter (sum across DEX pools they index). */
  liquidity: number;
}

/**
 * Fetch USD price + liquidity for a batch of mints from Jupiter Price
 * API v3. Best-effort — returns whatever it got. The map only contains
 * mints Jupiter recognises; *absence* is the signal a token is fresh /
 * unknown to Jupiter, which the fresh-token detector keys on.
 */
async function fetchTokenMarketInfo(mints: string[]): Promise<Map<string, TokenMarketInfo>> {
  const out = new Map<string, TokenMarketInfo>();
  if (mints.length === 0) return out;

  const controller = new AbortController();
  const timer = setTimeout(() => controller.abort(), PRICE_FETCH_TIMEOUT_MS);
  try {
    const res = await fetch(
      `https://lite-api.jup.ag/price/v3?ids=${mints.join(",")}`,
      { signal: controller.signal },
    );
    if (!res.ok) return out;
    const json = (await res.json()) as Record<
      string,
      { usdPrice?: number | string; liquidity?: number | string } | null
    >;
    for (const [mint, entry] of Object.entries(json)) {
      const rawPrice = entry?.usdPrice;
      const rawLiquidity = entry?.liquidity;
      if (rawPrice === undefined || rawPrice === null) continue;
      const usdPrice = typeof rawPrice === "number" ? rawPrice : Number(rawPrice);
      if (!Number.isFinite(usdPrice) || usdPrice <= 0) continue;
      const liquidity = typeof rawLiquidity === "number"
        ? rawLiquidity
        : rawLiquidity !== undefined && rawLiquidity !== null
          ? Number(rawLiquidity)
          : 0;
      out.set(mint, {
        usdPrice,
        liquidity: Number.isFinite(liquidity) ? liquidity : 0,
      });
    }
  } catch {
    // best-effort
  } finally {
    clearTimeout(timer);
  }
  return out;
}

/**
 * SPL Token mint layout (165 bytes; Token-2022 starts the same way):
 *   byte  0..4  COption tag for mint_authority   (1 = Some, 0 = None)
 *   byte  4..36 mint_authority pubkey (when Some)
 *   byte 36..44 supply (u64 LE)
 *   byte 44     decimals
 *   byte 45     is_initialized
 *   byte 46..50 COption tag for freeze_authority (1 = Some, 0 = None)
 *   byte 50..82 freeze_authority pubkey (when Some)
 */
const MINT_AUTH_TAG_OFFSET = 0;
const FREEZE_AUTH_TAG_OFFSET = 46;
const MULTIPLE_ACCOUNTS_TIMEOUT_MS = 10_000;

interface MintAuthInfo {
  hasMintAuthority: boolean;
  hasFreezeAuthority: boolean;
}

function base64ToBytes(b64: string): Uint8Array {
  const binary = atob(b64);
  const out = new Uint8Array(binary.length);
  for (let i = 0; i < binary.length; i++) out[i] = binary.charCodeAt(i);
  return out;
}

/**
 * Fetch mint-account data for a batch of mints in a single
 * `getMultipleAccounts` RPC and read out which authorities are still
 * active. Best-effort — returns whatever it got. Failures degrade
 * silently so we never block signing on this call.
 */
async function fetchMintAuthorityInfo(
  mints: string[],
  rpcUrl: string,
): Promise<Map<string, MintAuthInfo>> {
  const out = new Map<string, MintAuthInfo>();
  if (mints.length === 0) return out;

  const controller = new AbortController();
  const timer = setTimeout(() => controller.abort(), MULTIPLE_ACCOUNTS_TIMEOUT_MS);
  try {
    const res = await fetch(rpcUrl, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      signal: controller.signal,
      body: JSON.stringify({
        jsonrpc: "2.0",
        id: "faraday-mint-info",
        method: "getMultipleAccounts",
        params: [mints, { encoding: "base64", commitment: "confirmed" }],
      }),
    });
    if (!res.ok) return out;
    const json = await res.json();
    const values: Array<{ data?: [string, string] | null } | null> = json.result?.value ?? [];
    for (let i = 0; i < mints.length; i++) {
      const acct = values[i];
      const dataField = acct?.data;
      if (!Array.isArray(dataField) || typeof dataField[0] !== "string") continue;
      const bytes = base64ToBytes(dataField[0]);
      if (bytes.length < 50) continue;
      out.set(mints[i], {
        hasMintAuthority: bytes[MINT_AUTH_TAG_OFFSET] === 1,
        hasFreezeAuthority: bytes[FREEZE_AUTH_TAG_OFFSET] === 1,
      });
    }
  } catch {
    // best-effort
  } finally {
    clearTimeout(timer);
  }
  return out;
}

async function fetchTokenSymbol(mint: string): Promise<string> {
  const controller = new AbortController();
  const timer = setTimeout(() => controller.abort(), TOKEN_SYMBOL_TIMEOUT_MS);
  try {
    const res = await fetch(`https://tokens.jup.ag/token/${mint}`, { signal: controller.signal });
    if (!res.ok) return shortMint(mint);
    const data = await res.json() as { symbol?: string };
    return typeof data.symbol === "string" && data.symbol.length > 0 ? data.symbol : shortMint(mint);
  } catch {
    return shortMint(mint);
  } finally {
    clearTimeout(timer);
  }
}

/**
 * Diffs pre/post token balances to get the user's net changes, then
 * resolves each mint to a human-readable symbol via the Jupiter token API.
 * Fetches all symbols in parallel; falls back to truncated mint on failure.
 */
async function computeTokenChanges(
  sim: SimulationResult,
  userPubkey: string,
): Promise<TokenChange[]> {
  // Pre-balance map keyed by "accountIndex-mint"
  const preMap = new Map<string, number>();
  for (const bal of sim.preTokenBalances) {
    if (bal.owner === userPubkey) {
      preMap.set(`${bal.accountIndex}-${bal.mint}`, bal.uiTokenAmount.uiAmount ?? 0);
    }
  }

  // Compute diffs: tokens present in post
  const seen = new Set<string>();
  const rawChanges: Array<{ mint: string; amount: number }> = [];

  for (const bal of sim.postTokenBalances) {
    if (bal.owner !== userPubkey) continue;
    const key = `${bal.accountIndex}-${bal.mint}`;
    const pre = preMap.get(key) ?? 0;
    const post = bal.uiTokenAmount.uiAmount ?? 0;
    const diff = post - pre;
    if (Math.abs(diff) > 0.000001) rawChanges.push({ mint: bal.mint, amount: diff });
    seen.add(key);
  }

  // Tokens that existed in pre but vanished entirely in post
  for (const [key, preAmount] of preMap) {
    if (seen.has(key) || preAmount <= 0) continue;
    const mint = key.slice(key.indexOf("-") + 1);
    rawChanges.push({ mint, amount: -preAmount });
  }

  if (rawChanges.length === 0) return [];

  // Fetch all symbols in parallel
  const uniqueMints = [...new Set(rawChanges.map((c) => c.mint))];
  const symbolMap = new Map<string, string>();
  await Promise.all(
    uniqueMints.map(async (mint) => symbolMap.set(mint, await fetchTokenSymbol(mint))),
  );

  return rawChanges.map((c) => ({
    mint: c.mint,
    symbol: symbolMap.get(c.mint) ?? shortMint(c.mint),
    amount: c.amount,
  }));
}

// --- Helpers ---

function isTokenProgram(programId: string): boolean {
  return programId === TOKEN_PROGRAM_ID || programId === TOKEN_2022_PROGRAM_ID;
}

function tokenDisc(inst: ParsedInstruction): number {
  return inst.data.length > 0 ? inst.data[0] : -1;
}

function getComputeUnitPriceMicroLamports(parsed: ParsedTransaction): bigint {
  for (const inst of parsed.instructions) {
    if (inst.programId !== COMPUTE_BUDGET_PROGRAM_ID) continue;
    if (inst.data[0] === SET_COMPUTE_UNIT_PRICE_DISCRIMINATOR && inst.data.length >= 9) {
      return readU64LEBigInt(inst.data, 1);
    }
  }
  return 0n;
}

// --- Risk detectors ---

function detectUnlimitedApproval(parsed: ParsedTransaction): TxRiskWarning[] {
  for (const inst of parsed.instructions) {
    if (!isTokenProgram(inst.programId)) continue;
    const disc = tokenDisc(inst);
    if ((disc === TOKEN_IX_APPROVE || disc === TOKEN_IX_APPROVE_CHECKED) && inst.data.length >= 9) {
      if (readU64LEBigInt(inst.data, 1) === U64_MAX) {
        return [{
          severity: "critical",
          category: "fraud",
          title: "Unlimited token approval",
          description: "This transaction grants a program permission to spend an unlimited amount of one of your tokens.",
          explanation:
            "An unlimited approval lets the receiving program move that token at any time in the future, " +
            "even after you close the dApp tab. If you only meant to spend a specific amount, ask the dApp " +
            "for a precise approval, or revoke this one once the action completes.",
        }];
      }
    }
  }
  return [];
}

function detectAccountOwnerHijack(parsed: ParsedTransaction): TxRiskWarning[] {
  for (const inst of parsed.instructions) {
    if (!isTokenProgram(inst.programId)) continue;
    if (tokenDisc(inst) !== TOKEN_IX_SET_AUTHORITY) continue;
    if (inst.data.length >= 3 && inst.data[1] === AUTH_TYPE_ACCOUNT_OWNER) {
      return [{
        severity: "critical",
        category: "fraud",
        title: "Token account ownership change",
        description: "This transaction transfers ownership of one of your token accounts to another address.",
        explanation:
          "After this you will no longer control that account or anything in it. This is a common drainer step — " +
          "only proceed if you intentionally moved the account to a wallet you control.",
      }];
    }
  }
  return [];
}

function detectMintAuthorityChange(parsed: ParsedTransaction): TxRiskWarning[] {
  for (const inst of parsed.instructions) {
    if (!isTokenProgram(inst.programId)) continue;
    if (tokenDisc(inst) !== TOKEN_IX_SET_AUTHORITY) continue;
    if (inst.data.length < 2) continue;
    const type = inst.data[1];
    if (type === AUTH_TYPE_MINT_TOKENS) {
      return [{
        severity: "critical",
        category: "fraud",
        title: "Mint authority change",
        description: "This transaction changes who can mint new units of a token.",
        explanation:
          "Whoever holds mint authority can dilute the token's supply at will. " +
          "Only proceed if you intentionally manage this token's supply.",
      }];
    }
    if (type === AUTH_TYPE_FREEZE_ACCOUNT) {
      return [{
        severity: "critical",
        category: "fraud",
        title: "Freeze authority change",
        description: "This transaction changes who can freeze accounts holding this token.",
        explanation:
          "A freeze authority can lock your tokens at any time, blocking transfers. " +
          "Only proceed if you intentionally manage this token.",
      }];
    }
    if (type === AUTH_TYPE_CLOSE_ACCOUNT) {
      return [{
        severity: "warning",
        category: "fraud",
        title: "Close authority change",
        description: "This transaction changes who is allowed to close one of your token accounts.",
        explanation:
          "A close authority can return your token account's rent SOL to an address of their choice. " +
          "Only proceed if you intentionally delegated that ability.",
      }];
    }
  }
  return [];
}

function detectCloseAccountToOther(parsed: ParsedTransaction, userPubkey: string): TxRiskWarning[] {
  for (const inst of parsed.instructions) {
    if (!isTokenProgram(inst.programId)) continue;
    if (tokenDisc(inst) !== TOKEN_IX_CLOSE_ACCOUNT) continue;
    if (inst.accounts.length < 2) continue;
    const dest = inst.accounts[1];
    if (dest && dest !== userPubkey) {
      return [{
        severity: "warning",
        category: "fraud",
        title: "Account close to foreign address",
        description: "This transaction closes one of your token accounts and sends the rent SOL elsewhere.",
        explanation:
          "Closing a token account normally returns the rent SOL to you. Sending it to a different address " +
          "is unusual and is a known drainer pattern. Only proceed if you meant to gift the rent to that address.",
      }];
    }
  }
  return [];
}

function detectDrainHeuristic(
  sim: SimulationResult,
  userPubkey: string,
  accountKeys: string[],
  symbolMap: Map<string, string>,
): TxRiskWarning[] {
  const out: TxRiskWarning[] = [];

  // Token drains
  for (const pre of sim.preTokenBalances) {
    if (pre.owner !== userPubkey) continue;
    const preAmount = pre.uiTokenAmount.uiAmount ?? 0;
    if (preAmount <= 0) continue;
    const post = sim.postTokenBalances.find(
      (p) => p.owner === userPubkey && p.accountIndex === pre.accountIndex && p.mint === pre.mint,
    );
    const postAmount = post?.uiTokenAmount.uiAmount ?? 0;
    const lost = (preAmount - postAmount) / preAmount;
    const sym = symbolMap.get(pre.mint) ?? shortMint(pre.mint);
    if (lost >= DRAIN_WIPE_RATIO) {
      out.push({
        severity: "critical",
        category: "fraud",
        title: "Possible token drain",
        description: `This transaction would remove ${(lost * 100).toFixed(0)}% of your ${sym} balance.`,
        explanation:
          "Drainer dApps typically empty wallets in a single transaction. " +
          `If you didn't intend to send your full ${sym} balance, cancel immediately.`,
      });
      return out;
    }
    if (lost >= SIGNIFICANT_OUTFLOW_RATIO) {
      out.push({
        severity: "warning",
        category: "quality",
        title: "Significant token outflow",
        description: `This transaction would move ${(lost * 100).toFixed(0)}% of your ${sym} out of your wallet.`,
        explanation:
          "Moving more than half a balance in a single transaction is unusual. It can be legitimate (paying off a vault, " +
          "consolidating funds), but glance at the recipient before signing.",
      });
      return out;
    }
  }

  // SOL drain
  for (let i = 0; i < accountKeys.length; i++) {
    if (accountKeys[i] !== userPubkey) continue;
    const pre = sim.preBalances[i] ?? 0;
    const post = sim.postBalances[i] ?? 0;
    if (pre <= 0) continue;
    const lost = (pre - post) / pre;
    // Ignore tiny balances where legitimate account closures can wipe 100%
    if (pre / LAMPORTS_PER_SOL < 0.1) continue;
    if (lost >= DRAIN_WIPE_RATIO) {
      out.push({
        severity: "critical",
        category: "fraud",
        title: "Possible SOL drain",
        description: `This transaction would remove ${(lost * 100).toFixed(0)}% of your SOL balance.`,
        explanation:
          "Drainer dApps typically empty wallets in a single transaction. " +
          "If you didn't intend to send your full SOL balance, cancel immediately.",
      });
      return out;
    }
    if (lost >= SIGNIFICANT_OUTFLOW_RATIO) {
      out.push({
        severity: "warning",
        category: "quality",
        title: "Significant SOL outflow",
        description: `This transaction would move ${(lost * 100).toFixed(0)}% of your SOL out of your wallet.`,
        explanation:
          "Moving more than half your SOL in a single transaction is unusual. It can be legitimate (funding a vault, " +
          "topping up a hot wallet), but glance at the recipient before signing.",
      });
      return out;
    }
  }
  return out;
}

function detectOversizedPriorityFee(parsed: ParsedTransaction, unitsConsumed: number): TxRiskWarning[] {
  const microLamports = getComputeUnitPriceMicroLamports(parsed);
  const priorityLamports = (microLamports * BigInt(unitsConsumed)) / 1_000_000n;
  const prioritySol = Number(priorityLamports) / LAMPORTS_PER_SOL;
  if (prioritySol >= OVERSIZED_PRIORITY_FEE_SOL) {
    return [{
      severity: "warning",
      category: "quality",
      title: "Unusually high priority fee",
      description: `This transaction sets a priority fee of ~${prioritySol.toFixed(4)} SOL — well above the typical < 0.0001 SOL.`,
      explanation:
        "Either the dApp is congested and willing to pay heavily for fast inclusion, or it's trying to extract SOL " +
        "through the fee mechanism (priority fees are unrecoverable once paid). Verify the dApp and the urgency before signing.",
    }];
  }
  return [];
}

function detectHighValueSol(sim: SimulationResult, userPubkey: string, accountKeys: string[]): TxRiskWarning[] {
  for (let i = 0; i < accountKeys.length; i++) {
    if (accountKeys[i] !== userPubkey) continue;
    const outflowSol = ((sim.preBalances[i] ?? 0) - (sim.postBalances[i] ?? 0)) / LAMPORTS_PER_SOL;
    if (outflowSol >= HIGH_VALUE_SOL_THRESHOLD) {
      return [{
        severity: "warning",
        category: "quality",
        title: "High-value transfer",
        description: `This transaction sends approximately ${outflowSol.toFixed(2)} SOL from your wallet.`,
        explanation:
          "This is a large absolute amount. Not necessarily suspicious — but worth double-checking the recipient address " +
          "before signing. SOL transfers are irreversible.",
      }];
    }
  }
  return [];
}

/**
 * Applies NFKD decomposition (handles fullwidth Latin like ＵＳＤＣ),
 * strips zero-width characters, and maps Cyrillic/Greek confusables to their
 * Latin equivalents. Returns the result in uppercase for case-insensitive
 * comparison against the canonical ticker list.
 */
export function normalizeSymbol(symbol: string): string {
  let s = symbol.normalize("NFKD");
  s = s.replace(ZERO_WIDTH_RE, "");
  s = [...s].map((ch) => CONFUSABLE_MAP[ch] ?? ch).join("");
  return s.toUpperCase();
}

/**
 * Detects incoming tokens whose symbol (after homoglyph normalization)
 * matches a canonical ticker but whose mint address is not the official one.
 * Catches spoofed symbols like "USDС" (Cyrillic С) or fullwidth "ＵＳＤＣ".
 * Only checks incoming tokens (amount > 0) — outgoing tokens are controlled
 * by the user and can't be spoofed this way.
 */
function detectImpersonatorToken(tokenChanges: TokenChange[]): TxRiskWarning[] {
  for (const change of tokenChanges) {
    if (change.amount <= 0) continue;
    const normalized = normalizeSymbol(change.symbol);
    const officialMints = CANONICAL_MINTS[normalized];
    if (!officialMints) continue;
    if (!officialMints.includes(change.mint)) {
      return [{
        severity: "critical",
        category: "fraud",
        title: "Impersonator token",
        description: `Incoming token claims to be "${change.symbol}" but its mint isn't the official ${normalized}.`,
        explanation:
          "Scam tokens copy the visual identity of real ones (USDC, USDT, SOL, …) but use a different mint address. " +
          "The mint is the source of truth — anyone can ship a token with the same symbol. " +
          "Treat this token as worthless until you've verified the mint independently.",
      }];
    }
  }
  return [];
}

function detectMultiAssetDrain(sim: SimulationResult, userPubkey: string): TxRiskWarning[] {
  const drainingMints = new Set<string>();
  for (const pre of sim.preTokenBalances) {
    if (pre.owner !== userPubkey) continue;
    const preAmount = pre.uiTokenAmount.uiAmount ?? 0;
    if (preAmount <= 0) continue;
    const post = sim.postTokenBalances.find(
      (p) => p.owner === userPubkey && p.accountIndex === pre.accountIndex && p.mint === pre.mint,
    );
    if ((post?.uiTokenAmount.uiAmount ?? 0) < preAmount) drainingMints.add(pre.mint);
  }
  if (drainingMints.size >= MULTI_ASSET_DRAIN_THRESHOLD) {
    return [{
      severity: "critical",
      category: "fraud",
      title: "Multiple tokens leaving wallet",
      description: `${drainingMints.size} different tokens would leave your wallet in this single transaction.`,
      explanation:
        "Bulk transfers like this are a known drainer pattern — clear out as much as possible before the user notices. " +
        "If you didn't intend a multi-token send, cancel.",
    }];
  }
  return [];
}

/**
 * Address-poisoning attack: the attacker dusts you with a near-duplicate
 * of a real recipient (same first/last 4 chars, different middle) so the
 * lookalike shows up in your tx history. The next time you copy-from-history
 * you might pick the attacker's address.
 *
 * We flag whenever the current tx's destination matches the prefix+suffix
 * of any prior recipient but isn't an exact match.
 *
 * Scoped to System Program transfers — covers the most common SOL-to-pubkey
 * case. SPL transfers go to token accounts (derived addresses), so the
 * lookalike vector applies less directly.
 */
const SYSTEM_PROGRAM_ID = "11111111111111111111111111111111";
const SYSTEM_IX_TRANSFER = 2;

function detectLookalikeDestination(
  parsed: ParsedTransaction,
  history: string[],
): TxRiskWarning[] {
  if (history.length === 0) return [];

  // Pull all System Program Transfer destinations from this tx.
  const destinations = new Set<string>();
  for (const inst of parsed.instructions) {
    if (inst.programId !== SYSTEM_PROGRAM_ID) continue;
    if (inst.data.length < 4) continue;
    // System Program uses 4-byte little-endian discriminator.
    const disc =
      inst.data[0] |
      (inst.data[1] << 8) |
      (inst.data[2] << 16) |
      (inst.data[3] << 24);
    if (disc !== SYSTEM_IX_TRANSFER) continue;
    if (inst.accounts.length < 2) continue;
    const dest = inst.accounts[1];
    if (dest) destinations.add(dest);
  }

  if (destinations.size === 0) return [];

  for (const dest of destinations) {
    if (dest.length < 9) continue;
    const destPrefix = dest.slice(0, 4);
    const destSuffix = dest.slice(-4);
    for (const known of history) {
      if (known === dest) continue;
      if (known.length < 9) continue;
      if (known.slice(0, 4) === destPrefix && known.slice(-4) === destSuffix) {
        return [{
          severity: "critical",
          category: "fraud",
          title: "Lookalike recipient",
          description:
            `The destination ${dest.slice(0, 4)}…${dest.slice(-4)} is NOT the same as a previous recipient ` +
            `with identical start/end characters that you've sent to before.`,
          explanation:
            "Address poisoning attacks rely on you picking a similar-looking address from your history " +
            "instead of the real one. This destination shares the first and last 4 characters with someone " +
            "you've sent to, but the middle is different. Compare the full addresses character-by-character " +
            "before signing — the safer assumption is this isn't who you think it is.",
        }];
      }
    }
  }
  return [];
}

/**
 * Stake Program (`Stake11111111111111111111111111111111111111`) Authorize
 * variants change who controls a stake account. Hijacking the staker /
 * withdrawer authority gives the attacker full control of the staked SOL.
 */
function detectStakeAuthorityChange(parsed: ParsedTransaction): TxRiskWarning[] {
  const STAKE_AUTH_DISCRIMINATORS = new Set<number>([
    STAKE_IX_AUTHORIZE,
    STAKE_IX_AUTHORIZE_CHECKED,
    STAKE_IX_AUTHORIZE_WITH_SEED,
    STAKE_IX_AUTHORIZE_CHECKED_WITH_SEED,
  ]);
  for (const inst of parsed.instructions) {
    if (inst.programId !== STAKE_PROGRAM_ID) continue;
    if (inst.data.length < 4) continue;
    // Stake Program uses 4-byte little-endian discriminators (not 1 byte).
    const disc =
      inst.data[0] |
      (inst.data[1] << 8) |
      (inst.data[2] << 16) |
      (inst.data[3] << 24);
    if (!STAKE_AUTH_DISCRIMINATORS.has(disc)) continue;
    return [{
      severity: "critical",
      category: "fraud",
      title: "Stake authority change",
      description: "This transaction changes who controls one of your stake accounts.",
      explanation:
        "Whoever holds the stake authority can move the stake, change its delegation, or withdraw " +
        "the SOL once it deactivates. Granting that to another wallet hands them control of your " +
        "staked SOL — a known drainer pattern for staked positions. Only proceed if you intentionally " +
        "delegated control.",
    }];
  }
  return [];
}

/**
 * Sum total USD outflow vs inflow across all token + SOL movements and
 * flag the tx when the user is sending substantially more value than
 * they're receiving. Catches:
 *   - bad swaps (slippage / MEV sandwiches)
 *   - swap-disguised drains (you "swap" your USDC for a worthless token)
 *   - asymmetric routing
 *
 * Skipped silently when prices are missing (Jupiter doesn't price every
 * mint) or when there's only outflow (a one-way send isn't a swap).
 */
function detectUsdAsymmetry(
  tokenChanges: TokenChange[],
  marketInfo: Map<string, TokenMarketInfo>,
  solChange: number | null,
): TxRiskWarning[] {
  let outflowUsd = 0;
  let inflowUsd = 0;

  for (const c of tokenChanges) {
    const info = marketInfo.get(c.mint);
    if (!info) continue;
    const usd = Math.abs(c.amount) * info.usdPrice;
    if (c.amount < 0) outflowUsd += usd;
    else inflowUsd += usd;
  }

  if (solChange !== null) {
    const solInfo = marketInfo.get(WSOL_MINT);
    if (solInfo) {
      const solUsd = Math.abs(solChange) * solInfo.usdPrice;
      if (solChange < 0) outflowUsd += solUsd;
      else inflowUsd += solUsd;
    }
  }

  // Need both sides to compare meaningfully — a one-way transfer isn't
  // a "swap" and would always look infinitely asymmetric.
  if (outflowUsd <= 0 || inflowUsd <= 0) return [];

  const ratio = outflowUsd / inflowUsd;

  if (ratio >= 10) {
    return [{
      severity: "critical",
      category: "fraud",
      title: "Severe USD value mismatch",
      description:
        `You'd send ~$${outflowUsd.toFixed(2)} but receive only ~$${inflowUsd.toFixed(2)} — ${ratio.toFixed(1)}× more out than in.`,
      explanation:
        "A swap or trade should be roughly value-neutral. Sending 10× the value you receive almost always " +
        "means you're being routed through a malicious token, hitting catastrophic slippage, or being drained " +
        "via a swap-shaped transaction. Cancel and rebuild from a trusted aggregator.",
    }];
  }

  if (ratio >= 2) {
    return [{
      severity: "warning",
      category: "quality",
      title: "USD value mismatch on swap",
      description:
        `You'd send ~$${outflowUsd.toFixed(2)} but receive only ~$${inflowUsd.toFixed(2)} — ${ratio.toFixed(1)}× more out than in.`,
      explanation:
        "Real swaps are usually value-neutral within ~1%. A 2× spread is unusual — possibly a bad route, " +
        "MEV sandwich, or an illiquid pair. Confirm the destination token's value before signing.",
    }];
  }

  return [];
}

/**
 * Receiving a token Jupiter has never indexed is a strong "fresh /
 * scam" signal — established projects show up in Jupiter within hours
 * of listing on any major DEX. We surface this only when the token is
 * actually being sent TO the user (positive amount); outgoing
 * transfers of unindexed tokens are the user's own choice.
 *
 * The `Impersonator Token` detector handles the case where a fresh
 * token *also* mimics a known ticker — that warning is more specific
 * and fires alongside this one.
 */
function detectFreshOrUnknownToken(
  tokenChanges: TokenChange[],
  marketInfo: Map<string, TokenMarketInfo>,
): TxRiskWarning[] {
  const out: TxRiskWarning[] = [];
  for (const c of tokenChanges) {
    if (c.amount <= 0) continue;
    if (marketInfo.has(c.mint)) continue;
    const sym = c.symbol || shortMint(c.mint);
    out.push({
      severity: "warning",
      category: "fraud",
      title: `Unknown incoming token (${sym})`,
      description: "Jupiter has no record of this token — it's brand-new or scam-only.",
      explanation:
        "Established tokens are indexed by Jupiter within hours of any DEX listing. Tokens that " +
        "Jupiter has never seen are typically created moments before the transaction lands in your " +
        "wallet — a hallmark of dust drainer setups. Treat it as worthless until you've verified it " +
        "exists in the wider Solana ecosystem.",
    });
  }
  return out;
}

/**
 * Receiving a token whose DEX liquidity is below ~$10k means you almost
 * certainly can't sell it back at the price Jupiter quotes — the order
 * book is too thin. Honeypot tokens often show a "real" price but have
 * zero liquidity to actually exit through.
 */
function detectLowLiquidityToken(
  tokenChanges: TokenChange[],
  marketInfo: Map<string, TokenMarketInfo>,
): TxRiskWarning[] {
  const LOW_LIQUIDITY_THRESHOLD_USD = 10_000;
  const out: TxRiskWarning[] = [];
  for (const c of tokenChanges) {
    if (c.amount <= 0) continue;
    const info = marketInfo.get(c.mint);
    if (!info) continue; // covered by detectFreshOrUnknownToken
    if (info.liquidity >= LOW_LIQUIDITY_THRESHOLD_USD) continue;
    const sym = c.symbol || shortMint(c.mint);
    out.push({
      severity: "warning",
      category: "fraud",
      title: `Low liquidity on ${sym}`,
      description: `This incoming token has only ~$${Math.round(info.liquidity).toLocaleString()} of DEX liquidity.`,
      explanation:
        "You can't sell tokens out of a pool that doesn't have liquidity. Honeypot tokens often show a " +
        "real price but no actual liquidity — meaning you can buy them but you'll never be able to swap " +
        "them back. If you didn't expect this token, treat it as worthless.",
    });
  }
  return out;
}

/**
 * Receiving a token whose mint or freeze authority is still active is a
 * rugpull / honeypot signal. Whoever holds those authorities can dilute
 * the supply or freeze your account at any time. Established tokens like
 * USDC keep freeze authority on purpose (compliance), but a no-name
 * token with both still set warrants caution.
 */
function detectActiveTokenAuthorities(
  tokenChanges: TokenChange[],
  mintInfo: Map<string, MintAuthInfo>,
): TxRiskWarning[] {
  const out: TxRiskWarning[] = [];
  for (const change of tokenChanges) {
    if (change.amount <= 0) continue; // outgoing tokens are user-controlled
    const info = mintInfo.get(change.mint);
    if (!info) continue;
    const sym = change.symbol || shortMint(change.mint);
    if (info.hasMintAuthority && info.hasFreezeAuthority) {
      out.push({
        severity: "warning",
        category: "fraud",
        title: `Active mint + freeze authority on ${sym}`,
        description: "Both the mint and freeze authority on this incoming token are still set.",
        explanation:
          "Whoever holds these can mint unlimited new units (diluting your holding to nothing) or freeze " +
          "your account (locking your tokens forever). Combined, they are the textbook honeypot setup. " +
          "Treat this token with extreme suspicion until you've verified the project independently.",
      });
      continue;
    }
    if (info.hasMintAuthority) {
      out.push({
        severity: "warning",
        category: "fraud",
        title: `Active mint authority on ${sym}`,
        description: "Someone can still mint more of this token.",
        explanation:
          "If the mint authority issues more tokens, your holding gets diluted proportionally. " +
          "Real established tokens (USDC, JUP, …) usually have this revoked. Verify the project " +
          "before counting this as a stable asset.",
      });
      continue;
    }
    if (info.hasFreezeAuthority) {
      out.push({
        severity: "warning",
        category: "fraud",
        title: `Active freeze authority on ${sym}`,
        description: "Someone can freeze your account holding this token.",
        explanation:
          "A freeze authority can lock your tokens at any moment, blocking transfers. Stablecoins " +
          "like USDC have this on purpose for compliance, but a random token having it is a honeypot " +
          "signal — you may not be able to sell when you want to.",
      });
    }
  }
  return out;
}

/**
 * Drainer crews and address-poisoning attackers send tiny amounts of SOL
 * to wallets to "season" them — once your wallet has interacted with their
 * lookalike address, you're more likely to copy it from history later.
 * Receiving < 0.001 SOL as part of any signed operation is suspicious.
 */
function detectSubDustIncomingSol(
  sim: SimulationResult,
  userPubkey: string,
  accountKeys: string[],
): TxRiskWarning[] {
  const SUB_DUST_THRESHOLD_SOL = 0.001;
  for (let i = 0; i < accountKeys.length; i++) {
    if (accountKeys[i] !== userPubkey) continue;
    const inflowLamports = (sim.postBalances[i] ?? 0) - (sim.preBalances[i] ?? 0);
    if (inflowLamports <= 0) continue;
    const inflowSol = inflowLamports / LAMPORTS_PER_SOL;
    if (inflowSol < SUB_DUST_THRESHOLD_SOL) {
      return [{
        severity: "warning",
        category: "fraud",
        title: "Sub-dust SOL incoming",
        description: `This transaction sends you a tiny amount of SOL (~${inflowSol.toFixed(7)}) — well below 0.001 SOL.`,
        explanation:
          "Drainer crews and address-poisoning attackers seed wallets with tiny SOL amounts to make their " +
          "lookalike addresses appear in your transaction history. The next time you copy a recipient from " +
          "history you might pick theirs by mistake. Don't treat this as a freebie.",
      }];
    }
  }
  return [];
}

/**
 * Pull a *short* human-readable error name out of an arbitrary `sim.err`
 * shape, for inline use in the warning description. Returns "" when
 * nothing useful can be extracted — the caller should still surface the
 * full err in `details` either way.
 *
 * Solana's RPC returns errors in a few shapes:
 *   - bare string: "BlockhashNotFound", "InsufficientFundsForFee"
 *   - { InstructionError: [<idx>, "InvalidAccountData"] }
 *   - { InstructionError: [<idx>, { Custom: <num> }] }  ← program-defined
 *   - { AccountInUse }, etc.
 */
function describeSimError(err: unknown): string {
  if (err === null || err === undefined) return "";
  if (typeof err === "string") return err;
  if (typeof err !== "object") return "";

  const obj = err as Record<string, unknown>;
  if ("InstructionError" in obj && Array.isArray(obj.InstructionError)) {
    const detail = obj.InstructionError[1];
    if (typeof detail === "string") return `instruction error (${detail})`;
    if (detail && typeof detail === "object" && "Custom" in detail) {
      const code = (detail as { Custom: unknown }).Custom;
      return `program error (code ${typeof code === "number" ? code : String(code)})`;
    }
    return "instruction error";
  }

  // For other object shapes, return the first key. Solana usually puts
  // the variant name as the only key.
  const keys = Object.keys(obj);
  return keys[0] ?? "";
}

/**
 * Format a `simulateTransaction` failure into a single readable string for
 * the WarningRow's collapsible details. Combines:
 *   - the raw `err` (string or stringified object — Solana RPC returns
 *     either depending on the failure kind)
 *   - the last log line that mentions a failure / error / insufficient,
 *     since that's almost always where the real reason is
 */
function formatSimFailure(err: unknown, logs: string[] | null): string {
  const parts: string[] = [];

  if (typeof err === "string") {
    parts.push(`Error: ${err}`);
  } else if (err !== null && typeof err === "object") {
    try {
      parts.push(`Error: ${JSON.stringify(err)}`);
    } catch {
      parts.push("Error: (could not serialize)");
    }
  }

  if (Array.isArray(logs) && logs.length > 0) {
    for (let i = logs.length - 1; i >= 0; i--) {
      const log = logs[i];
      if (typeof log === "string" && /fail|error|insufficient|invalid|reject|exceeded/i.test(log)) {
        parts.push(`Log: ${log.trim()}`);
        break;
      }
    }
  }

  return parts.length > 0 ? parts.join("\n") : "No additional context from the simulator.";
}

function computeSolChange(sim: SimulationResult, userPubkey: string, accountKeys: string[]): number | null {
  let totalLamports = 0;
  let found = false;
  for (let i = 0; i < accountKeys.length; i++) {
    if (accountKeys[i] === userPubkey) {
      totalLamports += (sim.postBalances[i] ?? 0) - (sim.preBalances[i] ?? 0);
      found = true;
    }
  }
  return found ? totalLamports / LAMPORTS_PER_SOL : null;
}

// --- Main export ---

/**
 * Optional context passed in by the caller. Lets the analyzer reach
 * outside-the-tx data (recipient history for lookalike detection,
 * future signals like trusted-program lists) without having to be the
 * thing that owns that storage.
 */
export interface AnalyzeContext {
  /** Recent recipients the user has sent SOL to (most-recent first). */
  recipientHistory?: string[];
}

export async function analyzeTxRisk(
  txBase64: string,
  rpcUrl: string,
  userPubkey: string,
  ctx: AnalyzeContext = {},
): Promise<TxRiskReport> {
  // Hard outer deadline. Wins over any internal stall — the inner sim
  // already has its own 15s abort, but this catches anything downstream
  // (symbol fetches, detector passes, etc.) so a malicious page can't
  // freeze the analyzer to bypass the risk preview.
  const deadline = new Promise<TxRiskReport>((resolve) => {
    setTimeout(() => {
      resolve({
        level: "WARNING",
        warnings: [{
          severity: "warning",
          category: "failure",
          title: "Risk analysis timed out",
          description:
            `The analyzer exceeded its ${OVERALL_ANALYSIS_TIMEOUT_MS / 1000}s deadline before producing a report.`,
          explanation:
            "Without a completed analysis we can't tell what this transaction will actually do. " +
            "The most likely cause is a slow or unreachable RPC. Cancel and retry — if the stall " +
            "is consistent, switch your RPC endpoint.",
        }],
        tokenChanges: [],
        solChangeSol: null,
        simulationFailed: true,
      });
    }, OVERALL_ANALYSIS_TIMEOUT_MS);
  });
  return Promise.race([analyzeTxRiskInner(txBase64, rpcUrl, userPubkey, ctx), deadline]);
}

async function analyzeTxRiskInner(
  txBase64: string,
  rpcUrl: string,
  userPubkey: string,
  ctx: AnalyzeContext,
): Promise<TxRiskReport> {
  let sim: SimulationResult;
  try {
    sim = await simulate(txBase64, rpcUrl);
  } catch {
    return {
      level: "WARNING",
      warnings: [{
        severity: "warning",
        category: "failure",
        title: "Could not analyze transaction",
        description:
          "The risk analyzer needs an RPC connection to simulate the transaction, and that call timed out or failed.",
        explanation:
          "Without simulation, we can't tell what this transaction will actually do — you'd be signing blind. " +
          "The most likely cause is a slow or unreachable RPC endpoint. Wait a moment and retry, " +
          "or switch your RPC if this keeps happening.",
      }],
      tokenChanges: [],
      solChangeSol: null,
      simulationFailed: true,
    };
  }

  if (sim.err !== null) {
    const errBrief = describeSimError(sim.err);
    return {
      level: "DANGER",
      warnings: [{
        severity: "critical",
        category: "failure",
        title: "This transaction looks broken",
        description:
          `When we simulated this transaction the network reported an error${errBrief ? `: ${errBrief}` : ""}.`,
        explanation:
          "This isn't necessarily a fraud attempt — most often the transaction is malformed, references " +
          "an account that doesn't exist, or asks for more SOL than your wallet has (for the action itself or for the fee). " +
          "Either way, signing it would burn a fee and accomplish nothing. Cancel and ask the dApp to rebuild it. " +
          "If the dApp insists this should work, double-check that you have enough SOL and that you're on the network it expects.",
        details: formatSimFailure(sim.err, sim.logs),
      }],
      tokenChanges: [],
      solChangeSol: null,
      simulationFailed: true,
    };
  }

  const parsed = parseTransaction(txBase64);
  const accountKeys = parsed?.accountKeys ?? [];
  const solChangeSol = computeSolChange(sim, userPubkey, accountKeys);
  const unitsConsumed = sim.unitsConsumed ?? 0;

  // Resolve token symbols and compute balance changes in parallel with risk detection.
  // Symbol fetch is the only async step; detectors are synchronous.
  const tokenChanges = await computeTokenChanges(sim, userPubkey);

  // Build symbol map from resolved token changes for use in drain heuristic
  const symbolMap = new Map<string, string>(tokenChanges.map((c) => [c.mint, c.symbol]));

  // For incoming tokens (positive amount), pull mint authority info in
  // a single batched RPC. Used by the active-authority detector below.
  const incomingMints = tokenChanges
    .filter((c) => c.amount > 0)
    .map((c) => c.mint);

  // For ALL token movements + SOL (when present), pull USD prices in
  // one Jupiter call. Used by the USD-asymmetry detector. Both the
  // mint-info RPC and the price fetch run in parallel — neither blocks
  // detector execution.
  const allMints = Array.from(new Set(tokenChanges.map((c) => c.mint)));
  const priceMintsToQuery = solChangeSol !== null
    ? Array.from(new Set([...allMints, WSOL_MINT]))
    : allMints;

  const [mintAuthInfo, marketInfo] = await Promise.all([
    fetchMintAuthorityInfo(incomingMints, rpcUrl),
    fetchTokenMarketInfo(priceMintsToQuery),
  ]);

  const warnings: TxRiskWarning[] = [];

  if (parsed) {
    warnings.push(
      ...detectUnlimitedApproval(parsed),
      ...detectAccountOwnerHijack(parsed),
      ...detectMintAuthorityChange(parsed),
      ...detectCloseAccountToOther(parsed, userPubkey),
      ...detectStakeAuthorityChange(parsed),
      ...detectOversizedPriorityFee(parsed, unitsConsumed),
      ...detectLookalikeDestination(parsed, ctx.recipientHistory ?? []),
    );
  }

  warnings.push(
    ...detectImpersonatorToken(tokenChanges),
    ...detectDrainHeuristic(sim, userPubkey, accountKeys, symbolMap),
    ...detectHighValueSol(sim, userPubkey, accountKeys),
    ...detectMultiAssetDrain(sim, userPubkey),
    ...detectSubDustIncomingSol(sim, userPubkey, accountKeys),
    ...detectActiveTokenAuthorities(tokenChanges, mintAuthInfo),
    ...detectUsdAsymmetry(tokenChanges, marketInfo, solChangeSol),
    ...detectFreshOrUnknownToken(tokenChanges, marketInfo),
    ...detectLowLiquidityToken(tokenChanges, marketInfo),
  );

  const hasCritical = warnings.some((w) => w.severity === "critical");
  const level: TxRiskLevel = warnings.length === 0 ? "SAFE" : hasCritical ? "DANGER" : "WARNING";

  return { level, warnings, tokenChanges, solChangeSol, simulationFailed: false };
}
