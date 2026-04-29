export type TxRiskLevel = "SAFE" | "WARNING" | "DANGER";

export interface TxRiskWarning {
  severity: "critical" | "warning";
  title: string;
  description: string;
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
const HIGH_VALUE_SOL_THRESHOLD = 10;
const OVERSIZED_PRIORITY_FEE_SOL = 0.05;
const MULTI_ASSET_DRAIN_THRESHOLD = 3;
const SET_COMPUTE_UNIT_PRICE_DISCRIMINATOR = 3;
const SIMULATION_TIMEOUT_MS = 15_000;
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

function getComputeUnitPriceMicroLamports(parsed: ParsedTransaction): number {
  for (const inst of parsed.instructions) {
    if (inst.programId !== COMPUTE_BUDGET_PROGRAM_ID) continue;
    if (inst.data[0] === SET_COMPUTE_UNIT_PRICE_DISCRIMINATOR && inst.data.length >= 9) {
      return readU64LE(inst.data, 1);
    }
  }
  return 0;
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
          title: "Unlimited Token Approval",
          description:
            "This transaction grants a program unlimited permission to spend your tokens now and in the future. " +
            "Do not approve unless you fully trust this dApp.",
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
        title: "Token Account Ownership Change",
        description:
          "This transaction transfers ownership of one of your token accounts to another address. " +
          "After this you will no longer control that account.",
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
      return [{ severity: "critical", title: "Mint Authority Change", description: "This transaction changes the mint authority of a token. Only proceed if you intentionally manage this token." }];
    }
    if (type === AUTH_TYPE_FREEZE_ACCOUNT) {
      return [{ severity: "critical", title: "Freeze Authority Change", description: "This transaction changes the freeze authority of a token. Only proceed if you intentionally manage this token." }];
    }
    if (type === AUTH_TYPE_CLOSE_ACCOUNT) {
      return [{ severity: "warning", title: "Close Authority Change", description: "This transaction changes who is allowed to close one of your token accounts." }];
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
      return [{ severity: "warning", title: "Account Close to Foreign Address", description: "This transaction closes one of your token accounts and sends the rent SOL to an address that is not your wallet." }];
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
    if (lost >= DRAIN_WIPE_RATIO) {
      const sym = symbolMap.get(pre.mint) ?? shortMint(pre.mint);
      return [{ severity: "critical", title: "Possible Token Drain", description: `This transaction would remove ${(lost * 100).toFixed(0)}% of your ${sym} balance.` }];
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
    if (lost >= DRAIN_WIPE_RATIO && pre / LAMPORTS_PER_SOL >= 0.1) {
      return [{ severity: "critical", title: "Possible SOL Drain", description: `This transaction would remove ${(lost * 100).toFixed(0)}% of your SOL balance.` }];
    }
  }
  return [];
}

function detectOversizedPriorityFee(parsed: ParsedTransaction, unitsConsumed: number): TxRiskWarning[] {
  const microLamports = getComputeUnitPriceMicroLamports(parsed);
  const prioritySol = Math.floor((microLamports * unitsConsumed) / 1_000_000) / LAMPORTS_PER_SOL;
  if (prioritySol >= OVERSIZED_PRIORITY_FEE_SOL) {
    return [{ severity: "warning", title: "Oversized Priority Fee", description: `This transaction sets a priority fee of ~${prioritySol.toFixed(4)} SOL — far above typical Solana fees. This may be an attempt to drain SOL via the fee mechanism.` }];
  }
  return [];
}

function detectHighValueSol(sim: SimulationResult, userPubkey: string, accountKeys: string[]): TxRiskWarning[] {
  for (let i = 0; i < accountKeys.length; i++) {
    if (accountKeys[i] !== userPubkey) continue;
    const outflowSol = ((sim.preBalances[i] ?? 0) - (sim.postBalances[i] ?? 0)) / LAMPORTS_PER_SOL;
    if (outflowSol >= HIGH_VALUE_SOL_THRESHOLD) {
      return [{ severity: "warning", title: "High Value Transfer", description: `This transaction sends approximately ${outflowSol.toFixed(2)} SOL from your wallet.` }];
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
        title: "Impersonator Token",
        description:
          `This transaction sends you a token with the symbol "${change.symbol}" but its mint address ` +
          `does not match the official ${normalized}. This is a common phishing tactic — ` +
          `the token is worthless and designed to look legitimate.`,
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
    return [{ severity: "critical", title: "Multiple Tokens Leaving Wallet", description: `${drainingMints.size} different tokens are leaving your wallet in one transaction — a common pattern in wallet drainer attacks.` }];
  }
  return [];
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

export async function analyzeTxRisk(
  txBase64: string,
  rpcUrl: string,
  userPubkey: string,
): Promise<TxRiskReport> {
  let sim: SimulationResult;
  try {
    sim = await simulate(txBase64, rpcUrl);
  } catch {
    return {
      level: "WARNING",
      warnings: [{ severity: "warning", title: "Could Not Analyze Transaction", description: "Transaction simulation failed (network error or timeout). Verify the transaction carefully before signing." }],
      tokenChanges: [],
      solChangeSol: null,
      simulationFailed: true,
    };
  }

  if (sim.err !== null) {
    return {
      level: "DANGER",
      warnings: [{ severity: "critical", title: "Transaction Would Fail", description: "This transaction would fail if submitted to the network now. Do not sign it." }],
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

  const warnings: TxRiskWarning[] = [];

  if (parsed) {
    warnings.push(
      ...detectUnlimitedApproval(parsed),
      ...detectAccountOwnerHijack(parsed),
      ...detectMintAuthorityChange(parsed),
      ...detectCloseAccountToOther(parsed, userPubkey),
      ...detectOversizedPriorityFee(parsed, unitsConsumed),
    );
  }

  warnings.push(
    ...detectImpersonatorToken(tokenChanges),
    ...detectDrainHeuristic(sim, userPubkey, accountKeys, symbolMap),
    ...detectHighValueSol(sim, userPubkey, accountKeys),
    ...detectMultiAssetDrain(sim, userPubkey),
  );

  const hasCritical = warnings.some((w) => w.severity === "critical");
  const level: TxRiskLevel = warnings.length === 0 ? "SAFE" : hasCritical ? "DANGER" : "WARNING";

  return { level, warnings, tokenChanges, solChangeSol, simulationFailed: false };
}
