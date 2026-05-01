import { describe, expect, it, vi, afterEach } from "vitest";
import bs58 from "bs58";

import { analyzeTxRisk, normalizeSymbol } from "./tx-risk";
import { riskLevelLabel } from "./risk-display";

// ─── constants ────────────────────────────────────────────────────────────────

const RPC_URL = "https://test.rpc";
const USER    = "11111111111111111111111111111111";
const OTHER   = "So11111111111111111111111111111111111111112";

const TOKEN_PROG = "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA";
const CB_PROG    = "ComputeBudget111111111111111111111111111111";

const USDC_MINT = "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v";
const USDT_MINT = "Es9vMFrzaCERmJfrF4H2FYD4KCoNkY11McCe8BenwNYB";
const JUP_MINT  = "JUPyiwrYJFskUPiHa7hkeR8VUtAeFoSYbKedZNsDvCN";
const FAKE_MINT = "4k3Dyjzvzp8eMZWUXbBCjEvwSkkk59S5iCNLY3QrkX6R"; // valid address, not USDC

const U64_MAX = (1n << 64n) - 1n;

// ─── binary helpers ───────────────────────────────────────────────────────────

function u64LE(v: bigint): Uint8Array {
  const out = new Uint8Array(8);
  for (let i = 0; i < 8; i++) { out[i] = Number(v & 0xffn); v >>= 8n; }
  return out;
}

/**
 * Builds a minimal valid legacy Solana transaction and returns it as base64.
 * Compact-u16 lengths are single-byte (all test fixtures stay under 128).
 */
function buildTx(
  accountKeys: string[],
  instructions: Array<{
    programIndex: number;
    accountIndices: number[];
    data: Uint8Array;
  }>,
): string {
  const parts: Uint8Array[] = [];

  // Signature section: 1 slot, zero-filled
  parts.push(new Uint8Array([1]));
  parts.push(new Uint8Array(64));

  // Message header: [numReqSigs, numReadonlySigned, numReadonlyUnsigned]
  parts.push(new Uint8Array([1, 0, 0]));

  // Account keys
  parts.push(new Uint8Array([accountKeys.length]));
  for (const k of accountKeys) parts.push(bs58.decode(k));

  // Recent blockhash (32 zero bytes)
  parts.push(new Uint8Array(32));

  // Instructions
  parts.push(new Uint8Array([instructions.length]));
  for (const ix of instructions) {
    parts.push(new Uint8Array([ix.programIndex]));
    parts.push(new Uint8Array([ix.accountIndices.length]));
    parts.push(new Uint8Array(ix.accountIndices));
    parts.push(new Uint8Array([ix.data.length]));
    parts.push(ix.data);
  }

  const total = parts.reduce((n, p) => n + p.length, 0);
  const out = new Uint8Array(total);
  let offset = 0;
  for (const p of parts) { out.set(p, offset); offset += p.length; }

  let binary = "";
  for (const byte of out) binary += String.fromCharCode(byte);
  return btoa(binary);
}

/** Minimal single-account tx with no instructions. */
function emptyTx(): string {
  return buildTx([USER], []);
}

/** Token Program instruction. Assumes TOKEN_PROG at accountKeys index 1. */
function tokenIx(
  data: Uint8Array,
  accountIndices: number[] = [],
): { programIndex: number; accountIndices: number[]; data: Uint8Array } {
  return { programIndex: 1, accountIndices, data };
}

/** Compute Budget instruction. Assumes CB_PROG at accountKeys index 1. */
function cbIx(
  data: Uint8Array,
): { programIndex: number; accountIndices: number[]; data: Uint8Array } {
  return { programIndex: 1, accountIndices: [], data };
}

// ─── simulation helpers ───────────────────────────────────────────────────────

type TokenBalance = {
  accountIndex: number;
  mint: string;
  owner: string;
  uiTokenAmount: { uiAmount: number | null };
};

type SimOverride = {
  err?: unknown;
  logs?: string[] | null;
  preBalances?: number[];
  postBalances?: number[];
  preTokenBalances?: TokenBalance[];
  postTokenBalances?: TokenBalance[];
  unitsConsumed?: number;
};

function makeSim(overrides: SimOverride = {}) {
  return {
    err: null,
    logs: [],
    preBalances: [2_000_000_000],
    postBalances: [1_999_000_000],
    preTokenBalances: [] as TokenBalance[],
    postTokenBalances: [] as TokenBalance[],
    unitsConsumed: 100_000,
    ...overrides,
  };
}

function mockFetch(sim: object, symbols: Map<string, string> = new Map()): void {
  vi.stubGlobal(
    "fetch",
    vi.fn(async (url: string) => {
      if (url === RPC_URL) {
        return {
          ok: true,
          json: async () => ({ result: { value: sim } }),
        };
      }
      const mint = url.split("/").pop() ?? "";
      const symbol = symbols.get(mint);
      if (symbol) return { ok: true, json: async () => ({ symbol }) };
      return { ok: false, status: 404, json: async () => ({}) };
    }),
  );
}

interface MintFlags {
  hasMintAuthority: boolean;
  hasFreezeAuthority: boolean;
}

interface MarketEntry {
  usdPrice?: number;
  liquidity?: number;
}

/**
 * Build a 50-byte buffer matching the SPL Mint layout up to the freeze
 * authority tag. Only the COption discriminators at offsets 0 and 46
 * matter for the active-authority detector — everything else is zeroed.
 */
function buildMintAccountBase64(flags: MintFlags): string {
  const bytes = new Uint8Array(82);
  bytes[0] = flags.hasMintAuthority ? 1 : 0;
  bytes[46] = flags.hasFreezeAuthority ? 1 : 0;
  let bin = "";
  for (const b of bytes) bin += String.fromCharCode(b);
  return btoa(bin);
}

/**
 * Extended fetch mock that handles every endpoint the analyzer touches:
 *   - RPC_URL POST simulateTransaction
 *   - RPC_URL POST getMultipleAccounts (mint authority info)
 *   - tokens.jup.ag/token/<mint> (symbol)
 *   - lite-api.jup.ag/price/v3 (USD price + liquidity)
 *
 * Pass only the slices you need — the others default to empty / 404.
 */
function mockFetchExtended(opts: {
  sim?: object;
  symbols?: Map<string, string>;
  mintFlags?: Map<string, MintFlags>;
  market?: Record<string, MarketEntry | null>;
  /** When true, every fetch returns a never-resolving promise (timeout test). */
  stall?: boolean;
}): void {
  const sim = opts.sim;
  const symbols = opts.symbols ?? new Map<string, string>();
  const mintFlags = opts.mintFlags ?? new Map<string, MintFlags>();
  const market = opts.market ?? {};

  vi.stubGlobal(
    "fetch",
    vi.fn(async (url: string, init?: RequestInit): Promise<Response> => {
      if (opts.stall) return new Promise<Response>(() => undefined);

      if (url === RPC_URL && init?.body) {
        const body = JSON.parse(String(init.body)) as { method?: string; params?: unknown[] };
        if (body.method === "simulateTransaction") {
          return {
            ok: true,
            json: async () => ({ result: { value: sim ?? null } }),
          } as Response;
        }
        if (body.method === "getMultipleAccounts") {
          const requestedMints = (body.params?.[0] ?? []) as string[];
          const value = requestedMints.map((m) => {
            const flags = mintFlags.get(m);
            if (!flags) return null;
            return { data: [buildMintAccountBase64(flags), "base64"] as [string, string] };
          });
          return { ok: true, json: async () => ({ result: { value } }) } as Response;
        }
        return { ok: true, json: async () => ({ result: { value: null } }) } as Response;
      }

      if (url.startsWith("https://lite-api.jup.ag/price/v3")) {
        // Only return the entries the test asked for; absent mints become
        // "fresh / unknown" via Jupiter's empty response.
        return {
          ok: true,
          json: async () => market,
        } as Response;
      }

      if (url.startsWith("https://tokens.jup.ag/token/")) {
        const mint = url.split("/").pop() ?? "";
        const symbol = symbols.get(mint);
        if (symbol) return { ok: true, json: async () => ({ symbol }) } as Response;
        return { ok: false, status: 404, json: async () => ({}) } as Response;
      }

      return { ok: false, status: 404, json: async () => ({}) } as Response;
    }),
  );
}

afterEach(() => vi.unstubAllGlobals());

// ─────────────────────────────────────────────────────────────────────────────
// normalizeSymbol
// ─────────────────────────────────────────────────────────────────────────────

describe("normalizeSymbol", () => {
  it("is a no-op for plain ASCII uppercase", () => {
    expect(normalizeSymbol("USDC")).toBe("USDC");
  });

  it("uppercases lowercase ASCII", () => {
    expect(normalizeSymbol("usdc")).toBe("USDC");
  });

  it("maps Cyrillic look-alikes to their Latin equivalents", () => {
    // С = U+0421 Cyrillic Capital С, looks identical to Latin C
    expect(normalizeSymbol("USDС")).toBe("USDC");
    // А = U+0410 Cyrillic Capital А
    expect(normalizeSymbol("АBC")).toBe("ABC");
  });

  it("maps Greek look-alikes to their Latin equivalents", () => {
    // Ο = U+039F Greek Capital Omicron
    expect(normalizeSymbol("SΟL")).toBe("SOL");
  });

  it("collapses fullwidth Latin via NFKD normalisation", () => {
    // ＵＳＤＣ — fullwidth (U+FF35 U+FF33 U+FF24 U+FF23)
    expect(normalizeSymbol("ＵＳＤＣ")).toBe("USDC");
  });

  it("strips zero-width space (U+200B)", () => {
    expect(normalizeSymbol("U​SDC")).toBe("USDC");
  });

  it("strips zero-width joiner (U+200D)", () => {
    expect(normalizeSymbol("U‍SDC")).toBe("USDC");
  });

  it("handles a combination of fullwidth and Cyrillic", () => {
    // fullwidth U + S + D + Cyrillic С — should all normalise to USDC
    expect(normalizeSymbol("ＵＳＤС")).toBe("USDC");
  });
});

// ─────────────────────────────────────────────────────────────────────────────
// simulation failure paths
// ─────────────────────────────────────────────────────────────────────────────

describe("analyzeTxRisk — simulation failure", () => {
  it("returns WARNING when the RPC call throws (network error)", async () => {
    vi.stubGlobal("fetch", vi.fn(async () => { throw new Error("Network error"); }));
    const report = await analyzeTxRisk(emptyTx(), RPC_URL, USER);
    expect(report.level).toBe("WARNING");
    expect(report.simulationFailed).toBe(true);
    expect(report.warnings[0].title).toBe("Could not analyze transaction");
  });

  it("returns DANGER when the transaction would fail on-chain (sim.err !== null)", async () => {
    mockFetch(makeSim({ err: { InstructionError: [0, "InvalidAccountData"] } }));
    const report = await analyzeTxRisk(emptyTx(), RPC_URL, USER);
    expect(report.level).toBe("DANGER");
    expect(report.simulationFailed).toBe(true);
    expect(report.warnings[0].title).toBe("This transaction looks broken");
  });
});

// ─────────────────────────────────────────────────────────────────────────────
// safe baseline
// ─────────────────────────────────────────────────────────────────────────────

describe("analyzeTxRisk — safe transaction", () => {
  it("returns SAFE with no warnings for a plain transfer", async () => {
    mockFetch(makeSim());
    const report = await analyzeTxRisk(emptyTx(), RPC_URL, USER);
    expect(report.level).toBe("SAFE");
    expect(report.warnings).toHaveLength(0);
    expect(report.simulationFailed).toBe(false);
  });
});

// ─────────────────────────────────────────────────────────────────────────────
// structural detectors
// ─────────────────────────────────────────────────────────────────────────────

describe("analyzeTxRisk — unlimited token approval", () => {
  it("flags Approve (disc 4) with U64_MAX amount as DANGER", async () => {
    const tx = buildTx([USER, TOKEN_PROG], [tokenIx(new Uint8Array([4, ...u64LE(U64_MAX)]))]);
    mockFetch(makeSim());
    const report = await analyzeTxRisk(tx, RPC_URL, USER);
    expect(report.level).toBe("DANGER");
    expect(report.warnings[0].title).toBe("Unlimited token approval");
  });

  it("flags ApproveChecked (disc 13) with U64_MAX amount as DANGER", async () => {
    const tx = buildTx([USER, TOKEN_PROG], [tokenIx(new Uint8Array([13, ...u64LE(U64_MAX)]))]);
    mockFetch(makeSim());
    const report = await analyzeTxRisk(tx, RPC_URL, USER);
    expect(report.level).toBe("DANGER");
    expect(report.warnings[0].title).toBe("Unlimited token approval");
  });

  it("does not flag Approve with a non-max amount", async () => {
    const tx = buildTx([USER, TOKEN_PROG], [tokenIx(new Uint8Array([4, ...u64LE(1_000_000_000n)]))]);
    mockFetch(makeSim());
    const report = await analyzeTxRisk(tx, RPC_URL, USER);
    expect(report.warnings.map((w) => w.title)).not.toContain("Unlimited token approval");
  });
});

describe("analyzeTxRisk — SetAuthority detectors", () => {
  function setAuthData(authorityType: number): Uint8Array {
    // [disc=6, authorityType, hasNewAuthority=1, newAuthority (32 bytes)]
    const data = new Uint8Array(35);
    data[0] = 6;
    data[1] = authorityType;
    data[2] = 1;
    return data;
  }

  it("flags SetAuthority(AccountOwner=2) as DANGER — token account hijack", async () => {
    const tx = buildTx([USER, TOKEN_PROG], [tokenIx(setAuthData(2))]);
    mockFetch(makeSim());
    const report = await analyzeTxRisk(tx, RPC_URL, USER);
    expect(report.level).toBe("DANGER");
    expect(report.warnings[0].title).toBe("Token account ownership change");
  });

  it("flags SetAuthority(MintTokens=0) as DANGER — mint authority change", async () => {
    const tx = buildTx([USER, TOKEN_PROG], [tokenIx(setAuthData(0))]);
    mockFetch(makeSim());
    const report = await analyzeTxRisk(tx, RPC_URL, USER);
    expect(report.level).toBe("DANGER");
    expect(report.warnings[0].title).toBe("Mint authority change");
  });

  it("flags SetAuthority(FreezeAccount=1) as DANGER — freeze authority change", async () => {
    const tx = buildTx([USER, TOKEN_PROG], [tokenIx(setAuthData(1))]);
    mockFetch(makeSim());
    const report = await analyzeTxRisk(tx, RPC_URL, USER);
    expect(report.level).toBe("DANGER");
    expect(report.warnings[0].title).toBe("Freeze authority change");
  });
});

describe("analyzeTxRisk — CloseAccount to foreign address", () => {
  // accountKeys: [USER=0, TOKEN_PROG=1, token_account=2, OTHER=3]
  // CloseAccount accounts: [src=2, destination=3, authority=0]
  it("flags CloseAccount when the destination is not the user wallet", async () => {
    const tx = buildTx(
      [USER, TOKEN_PROG, USER, OTHER],
      [{ programIndex: 1, accountIndices: [2, 3, 0], data: new Uint8Array([9]) }],
    );
    mockFetch(makeSim());
    const report = await analyzeTxRisk(tx, RPC_URL, USER);
    expect(report.level).toBe("WARNING");
    expect(report.warnings.some((w) => w.title === "Account close to foreign address")).toBe(true);
  });

  it("does not flag CloseAccount when the destination is the user wallet", async () => {
    // accountKeys: [USER=0, TOKEN_PROG=1, token_account=2] — destination is index 0 = USER
    const tx = buildTx(
      [USER, TOKEN_PROG, USER],
      [{ programIndex: 1, accountIndices: [2, 0, 0], data: new Uint8Array([9]) }],
    );
    mockFetch(makeSim());
    const report = await analyzeTxRisk(tx, RPC_URL, USER);
    expect(report.warnings.map((w) => w.title)).not.toContain("Account close to foreign address");
  });
});

describe("analyzeTxRisk — oversized priority fee", () => {
  // prioritySol = floor(microLamports × unitsConsumed / 1_000_000) / 1_000_000_000
  // With microLamports=300_000_000 and unitsConsumed=200_000: 0.06 SOL ≥ 0.05 threshold
  it("flags a priority fee ≥0.05 SOL as WARNING", async () => {
    const tx = buildTx([USER, CB_PROG], [cbIx(new Uint8Array([3, ...u64LE(300_000_000n)]))]);
    mockFetch(makeSim({ unitsConsumed: 200_000 }));
    const report = await analyzeTxRisk(tx, RPC_URL, USER);
    expect(report.warnings.some((w) => w.title === "Unusually high priority fee")).toBe(true);
    expect(report.level).toBe("WARNING");
  });

  it("does not flag a normal priority fee", async () => {
    const tx = buildTx([USER, CB_PROG], [cbIx(new Uint8Array([3, ...u64LE(1_000n)]))]);
    mockFetch(makeSim());
    const report = await analyzeTxRisk(tx, RPC_URL, USER);
    expect(report.warnings.map((w) => w.title)).not.toContain("Unusually high priority fee");
  });
});

// ─────────────────────────────────────────────────────────────────────────────
// simulation-based detectors
// ─────────────────────────────────────────────────────────────────────────────

describe("analyzeTxRisk — token drain", () => {
  it("flags ≥95% token balance loss as DANGER", async () => {
    mockFetch(
      makeSim({
        preTokenBalances: [{ accountIndex: 0, mint: USDC_MINT, owner: USER, uiTokenAmount: { uiAmount: 1000 } }],
        postTokenBalances: [{ accountIndex: 0, mint: USDC_MINT, owner: USER, uiTokenAmount: { uiAmount: 40 } }],
      }),
      new Map([[USDC_MINT, "USDC"]]),
    );
    const report = await analyzeTxRisk(emptyTx(), RPC_URL, USER);
    expect(report.level).toBe("DANGER");
    expect(report.warnings[0].title).toBe("Possible token drain");
  });

  it("does not flag token loss below the 95% threshold", async () => {
    mockFetch(
      makeSim({
        preTokenBalances: [{ accountIndex: 0, mint: USDC_MINT, owner: USER, uiTokenAmount: { uiAmount: 1000 } }],
        postTokenBalances: [{ accountIndex: 0, mint: USDC_MINT, owner: USER, uiTokenAmount: { uiAmount: 100 } }],
      }),
      new Map([[USDC_MINT, "USDC"]]),
    );
    const report = await analyzeTxRisk(emptyTx(), RPC_URL, USER);
    expect(report.warnings.map((w) => w.title)).not.toContain("Possible token drain");
  });
});

describe("analyzeTxRisk — SOL drain", () => {
  it("flags ≥95% SOL loss when pre-balance ≥0.1 SOL as DANGER", async () => {
    // 1 SOL pre, 0.04 SOL post → 96% gone
    mockFetch(makeSim({ preBalances: [1_000_000_000], postBalances: [40_000_000] }));
    const report = await analyzeTxRisk(emptyTx(), RPC_URL, USER);
    expect(report.level).toBe("DANGER");
    expect(report.warnings[0].title).toBe("Possible SOL drain");
  });

  it("does not flag SOL drain when pre-balance is below 0.1 SOL", async () => {
    // 0.05 SOL → below the minimum balance guard
    mockFetch(makeSim({ preBalances: [50_000_000], postBalances: [1_000_000] }));
    const report = await analyzeTxRisk(emptyTx(), RPC_URL, USER);
    expect(report.warnings.map((w) => w.title)).not.toContain("Possible SOL drain");
  });
});

describe("analyzeTxRisk — high value SOL transfer", () => {
  it("flags ≥10 SOL outflow as WARNING", async () => {
    // 15 SOL → 1 SOL: 14 SOL outflow, 93% loss (below 95% drain threshold)
    mockFetch(makeSim({ preBalances: [15_000_000_000], postBalances: [1_000_000_000] }));
    const report = await analyzeTxRisk(emptyTx(), RPC_URL, USER);
    expect(report.warnings.some((w) => w.title === "High-value transfer")).toBe(true);
  });

  it("does not flag outflows below 10 SOL", async () => {
    mockFetch(makeSim({ preBalances: [5_000_000_000], postBalances: [1_000_000_000] }));
    const report = await analyzeTxRisk(emptyTx(), RPC_URL, USER);
    expect(report.warnings.map((w) => w.title)).not.toContain("High-value transfer");
  });
});

describe("analyzeTxRisk — multiple tokens leaving wallet", () => {
  it("flags 3 or more distinct tokens decreasing as DANGER", async () => {
    mockFetch(makeSim({
      preTokenBalances: [
        { accountIndex: 0, mint: USDC_MINT, owner: USER, uiTokenAmount: { uiAmount: 100 } },
        { accountIndex: 1, mint: USDT_MINT, owner: USER, uiTokenAmount: { uiAmount: 200 } },
        { accountIndex: 2, mint: JUP_MINT,  owner: USER, uiTokenAmount: { uiAmount: 300 } },
      ],
      postTokenBalances: [
        { accountIndex: 0, mint: USDC_MINT, owner: USER, uiTokenAmount: { uiAmount: 50 } },
        { accountIndex: 1, mint: USDT_MINT, owner: USER, uiTokenAmount: { uiAmount: 100 } },
        { accountIndex: 2, mint: JUP_MINT,  owner: USER, uiTokenAmount: { uiAmount: 150 } },
      ],
    }));
    const report = await analyzeTxRisk(emptyTx(), RPC_URL, USER);
    expect(report.level).toBe("DANGER");
    expect(report.warnings.some((w) => w.title === "Multiple tokens leaving wallet")).toBe(true);
  });

  it("does not flag fewer than 3 tokens decreasing", async () => {
    mockFetch(makeSim({
      preTokenBalances: [
        { accountIndex: 0, mint: USDC_MINT, owner: USER, uiTokenAmount: { uiAmount: 100 } },
        { accountIndex: 1, mint: USDT_MINT, owner: USER, uiTokenAmount: { uiAmount: 200 } },
      ],
      postTokenBalances: [
        { accountIndex: 0, mint: USDC_MINT, owner: USER, uiTokenAmount: { uiAmount: 50 } },
        { accountIndex: 1, mint: USDT_MINT, owner: USER, uiTokenAmount: { uiAmount: 100 } },
      ],
    }));
    const report = await analyzeTxRisk(emptyTx(), RPC_URL, USER);
    expect(report.warnings.map((w) => w.title)).not.toContain("Multiple tokens leaving wallet");
  });
});

// ─────────────────────────────────────────────────────────────────────────────
// impersonator token detection
// ─────────────────────────────────────────────────────────────────────────────

describe("analyzeTxRisk — impersonator token detection", () => {
  function incomingTokenSim(mint: string) {
    return makeSim({
      postTokenBalances: [
        { accountIndex: 0, mint, owner: USER, uiTokenAmount: { uiAmount: 100 } },
      ],
    });
  }

  it("flags an incoming token with a Cyrillic-spoofed USDC symbol (С = U+0421)", async () => {
    mockFetch(incomingTokenSim(FAKE_MINT), new Map([[FAKE_MINT, "USDС"]]));
    const report = await analyzeTxRisk(emptyTx(), RPC_URL, USER);
    expect(report.level).toBe("DANGER");
    expect(report.warnings.some((w) => w.title === "Impersonator token")).toBe(true);
  });

  it("flags an incoming token with a fullwidth USDC symbol (ＵＳＤＣ via NFKD)", async () => {
    // U+FF35 U+FF33 U+FF24 U+FF23 — fullwidth ＵＳＤＣ
    mockFetch(incomingTokenSim(FAKE_MINT), new Map([[FAKE_MINT, "ＵＳＤＣ"]]));
    const report = await analyzeTxRisk(emptyTx(), RPC_URL, USER);
    expect(report.level).toBe("DANGER");
    expect(report.warnings.some((w) => w.title === "Impersonator token")).toBe(true);
  });

  it("does not flag real USDC incoming with the correct mint", async () => {
    mockFetch(incomingTokenSim(USDC_MINT), new Map([[USDC_MINT, "USDC"]]));
    const report = await analyzeTxRisk(emptyTx(), RPC_URL, USER);
    expect(report.warnings.map((w) => w.title)).not.toContain("Impersonator token");
  });

  it("does not flag an outgoing token even when its symbol is spoofed", async () => {
    // Token goes from 100 → 10 (outgoing, amount < 0 in TokenChange)
    mockFetch(
      makeSim({
        preTokenBalances: [{ accountIndex: 0, mint: FAKE_MINT, owner: USER, uiTokenAmount: { uiAmount: 100 } }],
        postTokenBalances: [{ accountIndex: 0, mint: FAKE_MINT, owner: USER, uiTokenAmount: { uiAmount: 10 } }],
      }),
      new Map([[FAKE_MINT, "USDС"]]),
    );
    const report = await analyzeTxRisk(emptyTx(), RPC_URL, USER);
    expect(report.warnings.map((w) => w.title)).not.toContain("Impersonator token");
  });
});

// ─────────────────────────────────────────────────────────────────────────────
// balance change report
// ─────────────────────────────────────────────────────────────────────────────

describe("analyzeTxRisk — balance change report", () => {
  it("returns correct solChangeSol for a net SOL outflow", async () => {
    // 2 SOL → 1 SOL: net -1 SOL
    mockFetch(makeSim({ preBalances: [2_000_000_000], postBalances: [1_000_000_000] }));
    const report = await analyzeTxRisk(emptyTx(), RPC_URL, USER);
    expect(report.solChangeSol).toBeCloseTo(-1, 5);
  });

  it("returns tokenChanges with resolved symbol and correct amount", async () => {
    mockFetch(
      makeSim({
        preTokenBalances: [{ accountIndex: 0, mint: USDC_MINT, owner: USER, uiTokenAmount: { uiAmount: 500 } }],
        postTokenBalances: [{ accountIndex: 0, mint: USDC_MINT, owner: USER, uiTokenAmount: { uiAmount: 200 } }],
      }),
      new Map([[USDC_MINT, "USDC"]]),
    );
    const report = await analyzeTxRisk(emptyTx(), RPC_URL, USER);
    const usdc = report.tokenChanges.find((c) => c.mint === USDC_MINT);
    expect(usdc).toBeDefined();
    expect(usdc?.symbol).toBe("USDC");
    expect(usdc?.amount).toBeCloseTo(-300, 3);
  });
});

// ─────────────────────────────────────────────────────────────────────────────
// Framework — every warning has the category + explanation fields
// ─────────────────────────────────────────────────────────────────────────────

describe("TxRiskWarning shape — category + explanation", () => {
  it("populates category and explanation on the simulation-network-failure warning", async () => {
    // Mock fetch to throw — triggers the catch branch
    vi.stubGlobal("fetch", vi.fn().mockRejectedValue(new Error("network down")));
    const report = await analyzeTxRisk(emptyTx(), RPC_URL, USER);
    expect(report.warnings[0].category).toBe("failure");
    expect(report.warnings[0].explanation).toMatch(/RPC|simulate|blind/i);
  });

  it("populates category=failure on a sim.err DANGER", async () => {
    mockFetch(makeSim({ err: { InstructionError: [0, { Custom: 1 }] }, logs: ["Program 11111111111111111111111111111111 failed: custom program error: 0x1"] }));
    const report = await analyzeTxRisk(emptyTx(), RPC_URL, USER);
    expect(report.level).toBe("DANGER");
    expect(report.warnings[0].category).toBe("failure");
    expect(report.warnings[0].title).toBe("This transaction looks broken");
    // Description includes a brief decoded error name
    expect(report.warnings[0].description).toMatch(/program error \(code 1\)/);
    // Explanation hedges instead of accusing the dApp of fraud
    expect(report.warnings[0].explanation).toMatch(/fraud attempt|malformed|insufficient/i);
    // Details still has the raw blob for power users
    expect(report.warnings[0].details).toBeTruthy();
  });

  it("populates category=fraud + explanation on a SOL drain", async () => {
    mockFetch(makeSim({ preBalances: [1_000_000_000], postBalances: [40_000_000] }));
    const report = await analyzeTxRisk(emptyTx(), RPC_URL, USER);
    const drain = report.warnings.find((w) => w.title === "Possible SOL drain");
    expect(drain?.category).toBe("fraud");
    expect(drain?.explanation).toMatch(/Drainer dApps|cancel/i);
  });

  it("populates category=quality + explanation on a high-value transfer", async () => {
    mockFetch(makeSim({ preBalances: [15_000_000_000], postBalances: [1_000_000_000] }));
    const report = await analyzeTxRisk(emptyTx(), RPC_URL, USER);
    const high = report.warnings.find((w) => w.title === "High-value transfer");
    expect(high?.category).toBe("quality");
    expect(high?.explanation).toMatch(/large absolute amount|recipient/i);
  });
});

// ─────────────────────────────────────────────────────────────────────────────
// Framework — graduated drain detection (50%–95% → significant outflow)
// ─────────────────────────────────────────────────────────────────────────────

describe("analyzeTxRisk — significant outflow tier (50%–95%)", () => {
  it("flags 70% SOL outflow as WARNING / quality (not fraud DANGER)", async () => {
    // 1 SOL → 0.3 SOL = 70% loss. Below drain (95%), above significant (50%).
    mockFetch(makeSim({ preBalances: [1_000_000_000], postBalances: [300_000_000] }));
    const report = await analyzeTxRisk(emptyTx(), RPC_URL, USER);
    expect(report.level).toBe("WARNING");
    const w = report.warnings.find((w) => w.title === "Significant SOL outflow");
    expect(w).toBeDefined();
    expect(w?.severity).toBe("warning");
    expect(w?.category).toBe("quality");
    expect(w?.description).toMatch(/70%/);
  });

  it("flags 60% token outflow as WARNING / quality", async () => {
    mockFetch(
      makeSim({
        preTokenBalances: [{ accountIndex: 0, mint: USDC_MINT, owner: USER, uiTokenAmount: { uiAmount: 1000 } }],
        postTokenBalances: [{ accountIndex: 0, mint: USDC_MINT, owner: USER, uiTokenAmount: { uiAmount: 400 } }],
      }),
      new Map([[USDC_MINT, "USDC"]]),
    );
    const report = await analyzeTxRisk(emptyTx(), RPC_URL, USER);
    expect(report.level).toBe("WARNING");
    const w = report.warnings.find((w) => w.title === "Significant token outflow");
    expect(w).toBeDefined();
    expect(w?.category).toBe("quality");
    expect(w?.description).toMatch(/USDC/);
  });

  it("does not surface significant-outflow when 96% drain already fires", async () => {
    // 96% loss should produce ONLY the critical drain — not both
    mockFetch(makeSim({ preBalances: [1_000_000_000], postBalances: [40_000_000] }));
    const report = await analyzeTxRisk(emptyTx(), RPC_URL, USER);
    const titles = report.warnings.map((w) => w.title);
    expect(titles).toContain("Possible SOL drain");
    expect(titles).not.toContain("Significant SOL outflow");
  });

  it("does not flag <50% outflow at all", async () => {
    // 30% loss — well below significant tier
    mockFetch(makeSim({ preBalances: [1_000_000_000], postBalances: [700_000_000] }));
    const report = await analyzeTxRisk(emptyTx(), RPC_URL, USER);
    const titles = report.warnings.map((w) => w.title);
    expect(titles).not.toContain("Possible SOL drain");
    expect(titles).not.toContain("Significant SOL outflow");
  });
});

// ─────────────────────────────────────────────────────────────────────────────
// Framework — banner label aggregates by category
// ─────────────────────────────────────────────────────────────────────────────

describe("riskLevelLabel", () => {
  it("returns 'looks safe' for SAFE regardless of warnings", () => {
    expect(riskLevelLabel("SAFE", [])).toBe("Transaction looks safe");
  });

  it("returns 'fraud' label when any warning is fraud-category", () => {
    const warnings = [
      { severity: "warning" as const, category: "quality" as const, title: "x", description: "x", explanation: "x" },
      { severity: "critical" as const, category: "fraud" as const, title: "y", description: "y", explanation: "y" },
    ];
    expect(riskLevelLabel("DANGER", warnings)).toBe("Possible fraud — review before signing");
  });

  it("returns 'would not succeed' label when only failure-category warnings", () => {
    const warnings = [
      { severity: "critical" as const, category: "failure" as const, title: "x", description: "x", explanation: "x" },
    ];
    expect(riskLevelLabel("DANGER", warnings)).toBe("Transaction would not succeed — do not sign");
  });

  it("returns 'unusual' label for quality-only warnings", () => {
    const warnings = [
      { severity: "warning" as const, category: "quality" as const, title: "x", description: "x", explanation: "x" },
    ];
    expect(riskLevelLabel("WARNING", warnings)).toBe("Unusual transaction — review before signing");
  });

  it("prefers fraud over failure when both are present (worst-case framing)", () => {
    const warnings = [
      { severity: "critical" as const, category: "failure" as const, title: "x", description: "x", explanation: "x" },
      { severity: "critical" as const, category: "fraud" as const, title: "y", description: "y", explanation: "y" },
    ];
    expect(riskLevelLabel("DANGER", warnings)).toBe("Possible fraud — review before signing");
  });
});

// ─────────────────────────────────────────────────────────────────────────────
// Sub-dust incoming SOL
// ─────────────────────────────────────────────────────────────────────────────

describe("analyzeTxRisk — sub-dust incoming SOL", () => {
  it("flags incoming SOL < 0.001 as a fraud warning (drainer bait)", async () => {
    // 1 SOL pre, 1 SOL + 50_000 lamports post → +0.00005 SOL inflow
    mockFetchExtended({
      sim: makeSim({ preBalances: [1_000_000_000], postBalances: [1_000_050_000] }),
    });
    const report = await analyzeTxRisk(emptyTx(), RPC_URL, USER);
    const w = report.warnings.find((w) => w.title === "Sub-dust SOL incoming");
    expect(w).toBeDefined();
    expect(w?.severity).toBe("warning");
    expect(w?.category).toBe("fraud");
    expect(w?.explanation).toMatch(/poisoning|drainer/i);
  });

  it("does not flag normal incoming SOL (≥ 0.001)", async () => {
    mockFetchExtended({
      sim: makeSim({ preBalances: [1_000_000_000], postBalances: [1_500_000_000] }),
    });
    const report = await analyzeTxRisk(emptyTx(), RPC_URL, USER);
    expect(report.warnings.map((w) => w.title)).not.toContain("Sub-dust SOL incoming");
  });
});

// ─────────────────────────────────────────────────────────────────────────────
// Lookalike destination
// ─────────────────────────────────────────────────────────────────────────────

describe("analyzeTxRisk — lookalike destination", () => {
  // Two valid base58 strings sharing first 4 ("AAAA") and last 4 ("ZZZZ").
  // The detector only does string-prefix/suffix comparison so we don't
  // need real curve-checked pubkeys — the parser just sees these as
  // 32-byte account-key fields once we plug them into the buildTx helper.
  // We keep them realistic: 44 base58 chars, valid base58, distinct middles.

  it("flags a tx whose destination shares first/last 4 chars with a history entry", async () => {
    // Pick an arbitrary destination + a "history" entry sharing prefix/suffix
    const dest = USDC_MINT; // EPjFWdd5...wEGGkZwyTDt1v
    const known = USDT_MINT; // Es9vMFrza...e8BenwNYB — different prefix/suffix, so we need a synthetic match
    // Synthesize: take dest's prefix/suffix with a known-different middle (must be valid base58)
    // Instead of crafting one, we simulate: `historyEntry` constructed by hand from `dest`'s prefix/suffix + arbitrary middle of same char set.
    // dest = "EPjF" + "<middle>" + "Dt1v"  (first/last 4 of USDC_MINT)
    // historyMatch = "EPjF" + "00000000000000000000000000000000000" + "Dt1v"  → 4+35+4 = 43 chars; we need 44.
    const historyMatch = "EPjF" + "1".repeat(36) + "Dt1v";
    // Reality check: dest length === historyMatch length.
    expect(dest.length).toBe(historyMatch.length);

    // Use any source/dest layout — the SystemProgram Transfer ix needs
    // discriminator [2, 0, 0, 0] + lamports (u64). dest is account index 1.
    const tx = buildTx(
      [USER, dest],
      [
        {
          programIndex: 1, // we'll put System Program at index 1... actually we need it as program account
          accountIndices: [0, 0],
          data: new Uint8Array([2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]),
        },
      ],
    );
    // Above is wrong — the System Program needs to be one of the account keys
    // and its index used as programIndex. Rebuild with that.
    const tx2 = buildTx(
      [USER, dest, "11111111111111111111111111111111"],
      [
        {
          programIndex: 2,
          accountIndices: [0, 1],
          data: new Uint8Array([2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]),
        },
      ],
    );
    void tx; // unused; tx2 is the one we use

    mockFetchExtended({
      sim: makeSim(),
    });
    const report = await analyzeTxRisk(tx2, RPC_URL, USER, {
      recipientHistory: [historyMatch],
    });
    const w = report.warnings.find((w) => w.title === "Lookalike recipient");
    expect(w).toBeDefined();
    expect(w?.severity).toBe("critical");
    expect(w?.category).toBe("fraud");
  });

  it("does not flag when history is empty", async () => {
    const tx = buildTx(
      [USER, USDC_MINT, "11111111111111111111111111111111"],
      [
        {
          programIndex: 2,
          accountIndices: [0, 1],
          data: new Uint8Array([2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]),
        },
      ],
    );
    mockFetchExtended({ sim: makeSim() });
    const report = await analyzeTxRisk(tx, RPC_URL, USER, { recipientHistory: [] });
    expect(report.warnings.map((w) => w.title)).not.toContain("Lookalike recipient");
  });

  it("does not flag when destination is an exact match in history (re-send)", async () => {
    const tx = buildTx(
      [USER, USDC_MINT, "11111111111111111111111111111111"],
      [
        {
          programIndex: 2,
          accountIndices: [0, 1],
          data: new Uint8Array([2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]),
        },
      ],
    );
    mockFetchExtended({ sim: makeSim() });
    const report = await analyzeTxRisk(tx, RPC_URL, USER, {
      recipientHistory: [USDC_MINT], // exact same address as destination
    });
    expect(report.warnings.map((w) => w.title)).not.toContain("Lookalike recipient");
  });
});

// ─────────────────────────────────────────────────────────────────────────────
// Stake authority change
// ─────────────────────────────────────────────────────────────────────────────

describe("analyzeTxRisk — stake authority change", () => {
  const STAKE_PROGRAM = "Stake11111111111111111111111111111111111111";

  it("flags Stake Program Authorize (disc 1) as DANGER", async () => {
    // Stake Program uses 4-byte little-endian discriminators
    const data = new Uint8Array([1, 0, 0, 0]); // Authorize
    const tx = buildTx(
      [USER, "So11111111111111111111111111111111111111112", STAKE_PROGRAM],
      [{ programIndex: 2, accountIndices: [0, 1], data }],
    );
    mockFetchExtended({ sim: makeSim() });
    const report = await analyzeTxRisk(tx, RPC_URL, USER);
    const w = report.warnings.find((w) => w.title === "Stake authority change");
    expect(w).toBeDefined();
    expect(w?.severity).toBe("critical");
    expect(w?.category).toBe("fraud");
  });

  it("flags Stake Program AuthorizeChecked (disc 10) as DANGER", async () => {
    const data = new Uint8Array([10, 0, 0, 0]);
    const tx = buildTx(
      [USER, "So11111111111111111111111111111111111111112", STAKE_PROGRAM],
      [{ programIndex: 2, accountIndices: [0, 1], data }],
    );
    mockFetchExtended({ sim: makeSim() });
    const report = await analyzeTxRisk(tx, RPC_URL, USER);
    expect(report.warnings.some((w) => w.title === "Stake authority change")).toBe(true);
  });

  it("does not flag a non-authority Stake instruction (disc 0 = Initialize)", async () => {
    const data = new Uint8Array([0, 0, 0, 0]);
    const tx = buildTx(
      [USER, "So11111111111111111111111111111111111111112", STAKE_PROGRAM],
      [{ programIndex: 2, accountIndices: [0, 1], data }],
    );
    mockFetchExtended({ sim: makeSim() });
    const report = await analyzeTxRisk(tx, RPC_URL, USER);
    expect(report.warnings.map((w) => w.title)).not.toContain("Stake authority change");
  });
});

// ─────────────────────────────────────────────────────────────────────────────
// Active mint / freeze authority on incoming tokens
// ─────────────────────────────────────────────────────────────────────────────

describe("analyzeTxRisk — active mint / freeze authority", () => {
  const someMint = JUP_MINT; // some valid mint address used as the incoming token

  it("flags both authorities active as a single combined warning", async () => {
    mockFetchExtended({
      sim: makeSim({
        preTokenBalances: [],
        postTokenBalances: [
          { accountIndex: 0, mint: someMint, owner: USER, uiTokenAmount: { uiAmount: 100 } },
        ],
      }),
      symbols: new Map([[someMint, "JUP"]]),
      mintFlags: new Map([[someMint, { hasMintAuthority: true, hasFreezeAuthority: true }]]),
    });
    const report = await analyzeTxRisk(emptyTx(), RPC_URL, USER);
    const w = report.warnings.find((w) => w.title.includes("Active mint + freeze authority"));
    expect(w).toBeDefined();
    expect(w?.category).toBe("fraud");
    expect(w?.severity).toBe("warning");
  });

  it("flags only mint authority as the mint-only warning", async () => {
    mockFetchExtended({
      sim: makeSim({
        preTokenBalances: [],
        postTokenBalances: [
          { accountIndex: 0, mint: someMint, owner: USER, uiTokenAmount: { uiAmount: 100 } },
        ],
      }),
      symbols: new Map([[someMint, "JUP"]]),
      mintFlags: new Map([[someMint, { hasMintAuthority: true, hasFreezeAuthority: false }]]),
    });
    const report = await analyzeTxRisk(emptyTx(), RPC_URL, USER);
    expect(report.warnings.some((w) => w.title.startsWith("Active mint authority"))).toBe(true);
    expect(report.warnings.some((w) => w.title.includes("freeze"))).toBe(false);
  });

  it("flags only freeze authority as the freeze-only warning", async () => {
    mockFetchExtended({
      sim: makeSim({
        preTokenBalances: [],
        postTokenBalances: [
          { accountIndex: 0, mint: someMint, owner: USER, uiTokenAmount: { uiAmount: 100 } },
        ],
      }),
      symbols: new Map([[someMint, "JUP"]]),
      mintFlags: new Map([[someMint, { hasMintAuthority: false, hasFreezeAuthority: true }]]),
    });
    const report = await analyzeTxRisk(emptyTx(), RPC_URL, USER);
    expect(report.warnings.some((w) => w.title.startsWith("Active freeze authority"))).toBe(true);
    expect(report.warnings.some((w) => w.title.startsWith("Active mint"))).toBe(false);
  });

  it("does not flag when both authorities are revoked", async () => {
    mockFetchExtended({
      sim: makeSim({
        preTokenBalances: [],
        postTokenBalances: [
          { accountIndex: 0, mint: someMint, owner: USER, uiTokenAmount: { uiAmount: 100 } },
        ],
      }),
      symbols: new Map([[someMint, "JUP"]]),
      mintFlags: new Map([[someMint, { hasMintAuthority: false, hasFreezeAuthority: false }]]),
    });
    const report = await analyzeTxRisk(emptyTx(), RPC_URL, USER);
    expect(report.warnings.some((w) => w.title.includes("authority"))).toBe(false);
  });
});

// ─────────────────────────────────────────────────────────────────────────────
// USD value asymmetry
// ─────────────────────────────────────────────────────────────────────────────

describe("analyzeTxRisk — USD value asymmetry", () => {
  it("flags 10×+ asymmetry as DANGER / fraud", async () => {
    // Outflow: 100 USDC ($100). Inflow: token X at $1 per unit, 5 units = $5. Ratio: 20×
    mockFetchExtended({
      sim: makeSim({
        preTokenBalances: [
          { accountIndex: 0, mint: USDC_MINT, owner: USER, uiTokenAmount: { uiAmount: 100 } },
        ],
        postTokenBalances: [
          { accountIndex: 0, mint: USDC_MINT, owner: USER, uiTokenAmount: { uiAmount: 0 } },
          { accountIndex: 1, mint: JUP_MINT, owner: USER, uiTokenAmount: { uiAmount: 5 } },
        ],
      }),
      symbols: new Map([[USDC_MINT, "USDC"], [JUP_MINT, "JUP"]]),
      market: {
        [USDC_MINT]: { usdPrice: 1, liquidity: 999_999_999 },
        [JUP_MINT]: { usdPrice: 1, liquidity: 999_999_999 },
      },
    });
    const report = await analyzeTxRisk(emptyTx(), RPC_URL, USER);
    const w = report.warnings.find((w) => w.title === "Severe USD value mismatch");
    expect(w).toBeDefined();
    expect(w?.severity).toBe("critical");
    expect(w?.category).toBe("fraud");
  });

  it("flags 2×–10× asymmetry as WARNING / quality", async () => {
    // Outflow: 100 USDC ($100). Inflow: 30 JUP at $1 = $30. Ratio: 3.33×
    mockFetchExtended({
      sim: makeSim({
        preTokenBalances: [
          { accountIndex: 0, mint: USDC_MINT, owner: USER, uiTokenAmount: { uiAmount: 100 } },
        ],
        postTokenBalances: [
          { accountIndex: 0, mint: USDC_MINT, owner: USER, uiTokenAmount: { uiAmount: 0 } },
          { accountIndex: 1, mint: JUP_MINT, owner: USER, uiTokenAmount: { uiAmount: 30 } },
        ],
      }),
      symbols: new Map([[USDC_MINT, "USDC"], [JUP_MINT, "JUP"]]),
      market: {
        [USDC_MINT]: { usdPrice: 1, liquidity: 999_999_999 },
        [JUP_MINT]: { usdPrice: 1, liquidity: 999_999_999 },
      },
    });
    const report = await analyzeTxRisk(emptyTx(), RPC_URL, USER);
    const w = report.warnings.find((w) => w.title === "USD value mismatch on swap");
    expect(w).toBeDefined();
    expect(w?.severity).toBe("warning");
    expect(w?.category).toBe("quality");
  });

  it("does not flag a balanced swap (~1:1 USD)", async () => {
    // Outflow: 100 USDC ($100). Inflow: 99 JUP at $1 = $99. Ratio: 1.01×
    mockFetchExtended({
      sim: makeSim({
        preTokenBalances: [
          { accountIndex: 0, mint: USDC_MINT, owner: USER, uiTokenAmount: { uiAmount: 100 } },
        ],
        postTokenBalances: [
          { accountIndex: 0, mint: USDC_MINT, owner: USER, uiTokenAmount: { uiAmount: 0 } },
          { accountIndex: 1, mint: JUP_MINT, owner: USER, uiTokenAmount: { uiAmount: 99 } },
        ],
      }),
      symbols: new Map([[USDC_MINT, "USDC"], [JUP_MINT, "JUP"]]),
      market: {
        [USDC_MINT]: { usdPrice: 1, liquidity: 999_999_999 },
        [JUP_MINT]: { usdPrice: 1, liquidity: 999_999_999 },
      },
    });
    const report = await analyzeTxRisk(emptyTx(), RPC_URL, USER);
    expect(report.warnings.some((w) => w.title.includes("USD value"))).toBe(false);
  });
});

// ─────────────────────────────────────────────────────────────────────────────
// Fresh / unknown token (Jupiter has no record)
// ─────────────────────────────────────────────────────────────────────────────

describe("analyzeTxRisk — fresh / unknown token", () => {
  it("flags incoming token Jupiter doesn't index", async () => {
    mockFetchExtended({
      sim: makeSim({
        preTokenBalances: [],
        postTokenBalances: [
          { accountIndex: 0, mint: FAKE_MINT, owner: USER, uiTokenAmount: { uiAmount: 1000 } },
        ],
      }),
      symbols: new Map([[FAKE_MINT, "SCAM"]]),
      market: {}, // Jupiter has no entry
    });
    const report = await analyzeTxRisk(emptyTx(), RPC_URL, USER);
    const w = report.warnings.find((w) => w.title.startsWith("Unknown incoming token"));
    expect(w).toBeDefined();
    expect(w?.category).toBe("fraud");
  });

  it("does not flag tokens Jupiter recognizes (even with low liquidity is a different detector)", async () => {
    mockFetchExtended({
      sim: makeSim({
        preTokenBalances: [],
        postTokenBalances: [
          { accountIndex: 0, mint: USDC_MINT, owner: USER, uiTokenAmount: { uiAmount: 1000 } },
        ],
      }),
      symbols: new Map([[USDC_MINT, "USDC"]]),
      market: {
        [USDC_MINT]: { usdPrice: 1, liquidity: 100_000_000 },
      },
    });
    const report = await analyzeTxRisk(emptyTx(), RPC_URL, USER);
    expect(report.warnings.some((w) => w.title.startsWith("Unknown incoming token"))).toBe(false);
  });
});

// ─────────────────────────────────────────────────────────────────────────────
// Low liquidity
// ─────────────────────────────────────────────────────────────────────────────

describe("analyzeTxRisk — low liquidity token", () => {
  it("flags incoming token with < $10k DEX liquidity", async () => {
    mockFetchExtended({
      sim: makeSim({
        preTokenBalances: [],
        postTokenBalances: [
          { accountIndex: 0, mint: JUP_MINT, owner: USER, uiTokenAmount: { uiAmount: 1000 } },
        ],
      }),
      symbols: new Map([[JUP_MINT, "JUP"]]),
      market: {
        [JUP_MINT]: { usdPrice: 0.5, liquidity: 5_000 }, // $5k — below threshold
      },
    });
    const report = await analyzeTxRisk(emptyTx(), RPC_URL, USER);
    const w = report.warnings.find((w) => w.title.startsWith("Low liquidity"));
    expect(w).toBeDefined();
    expect(w?.category).toBe("fraud");
  });

  it("does not flag tokens with healthy liquidity", async () => {
    mockFetchExtended({
      sim: makeSim({
        preTokenBalances: [],
        postTokenBalances: [
          { accountIndex: 0, mint: USDC_MINT, owner: USER, uiTokenAmount: { uiAmount: 1000 } },
        ],
      }),
      symbols: new Map([[USDC_MINT, "USDC"]]),
      market: {
        [USDC_MINT]: { usdPrice: 1, liquidity: 100_000_000 },
      },
    });
    const report = await analyzeTxRisk(emptyTx(), RPC_URL, USER);
    expect(report.warnings.some((w) => w.title.startsWith("Low liquidity"))).toBe(false);
  });
});
