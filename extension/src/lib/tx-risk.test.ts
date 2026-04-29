import { describe, expect, it, vi, afterEach } from "vitest";
import bs58 from "bs58";

import { analyzeTxRisk, normalizeSymbol } from "./tx-risk";

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
    expect(report.warnings[0].title).toBe("Could Not Analyze Transaction");
  });

  it("returns DANGER when the transaction would fail on-chain (sim.err !== null)", async () => {
    mockFetch(makeSim({ err: { InstructionError: [0, "InvalidAccountData"] } }));
    const report = await analyzeTxRisk(emptyTx(), RPC_URL, USER);
    expect(report.level).toBe("DANGER");
    expect(report.simulationFailed).toBe(true);
    expect(report.warnings[0].title).toBe("Transaction Would Fail");
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
    expect(report.warnings[0].title).toBe("Unlimited Token Approval");
  });

  it("flags ApproveChecked (disc 13) with U64_MAX amount as DANGER", async () => {
    const tx = buildTx([USER, TOKEN_PROG], [tokenIx(new Uint8Array([13, ...u64LE(U64_MAX)]))]);
    mockFetch(makeSim());
    const report = await analyzeTxRisk(tx, RPC_URL, USER);
    expect(report.level).toBe("DANGER");
    expect(report.warnings[0].title).toBe("Unlimited Token Approval");
  });

  it("does not flag Approve with a non-max amount", async () => {
    const tx = buildTx([USER, TOKEN_PROG], [tokenIx(new Uint8Array([4, ...u64LE(1_000_000_000n)]))]);
    mockFetch(makeSim());
    const report = await analyzeTxRisk(tx, RPC_URL, USER);
    expect(report.warnings.map((w) => w.title)).not.toContain("Unlimited Token Approval");
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
    expect(report.warnings[0].title).toBe("Token Account Ownership Change");
  });

  it("flags SetAuthority(MintTokens=0) as DANGER — mint authority change", async () => {
    const tx = buildTx([USER, TOKEN_PROG], [tokenIx(setAuthData(0))]);
    mockFetch(makeSim());
    const report = await analyzeTxRisk(tx, RPC_URL, USER);
    expect(report.level).toBe("DANGER");
    expect(report.warnings[0].title).toBe("Mint Authority Change");
  });

  it("flags SetAuthority(FreezeAccount=1) as DANGER — freeze authority change", async () => {
    const tx = buildTx([USER, TOKEN_PROG], [tokenIx(setAuthData(1))]);
    mockFetch(makeSim());
    const report = await analyzeTxRisk(tx, RPC_URL, USER);
    expect(report.level).toBe("DANGER");
    expect(report.warnings[0].title).toBe("Freeze Authority Change");
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
    expect(report.warnings.some((w) => w.title === "Account Close to Foreign Address")).toBe(true);
  });

  it("does not flag CloseAccount when the destination is the user wallet", async () => {
    // accountKeys: [USER=0, TOKEN_PROG=1, token_account=2] — destination is index 0 = USER
    const tx = buildTx(
      [USER, TOKEN_PROG, USER],
      [{ programIndex: 1, accountIndices: [2, 0, 0], data: new Uint8Array([9]) }],
    );
    mockFetch(makeSim());
    const report = await analyzeTxRisk(tx, RPC_URL, USER);
    expect(report.warnings.map((w) => w.title)).not.toContain("Account Close to Foreign Address");
  });
});

describe("analyzeTxRisk — oversized priority fee", () => {
  // prioritySol = floor(microLamports × unitsConsumed / 1_000_000) / 1_000_000_000
  // With microLamports=300_000_000 and unitsConsumed=200_000: 0.06 SOL ≥ 0.05 threshold
  it("flags a priority fee ≥0.05 SOL as WARNING", async () => {
    const tx = buildTx([USER, CB_PROG], [cbIx(new Uint8Array([3, ...u64LE(300_000_000n)]))]);
    mockFetch(makeSim({ unitsConsumed: 200_000 }));
    const report = await analyzeTxRisk(tx, RPC_URL, USER);
    expect(report.warnings.some((w) => w.title === "Oversized Priority Fee")).toBe(true);
    expect(report.level).toBe("WARNING");
  });

  it("does not flag a normal priority fee", async () => {
    const tx = buildTx([USER, CB_PROG], [cbIx(new Uint8Array([3, ...u64LE(1_000n)]))]);
    mockFetch(makeSim());
    const report = await analyzeTxRisk(tx, RPC_URL, USER);
    expect(report.warnings.map((w) => w.title)).not.toContain("Oversized Priority Fee");
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
    expect(report.warnings[0].title).toBe("Possible Token Drain");
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
    expect(report.warnings.map((w) => w.title)).not.toContain("Possible Token Drain");
  });
});

describe("analyzeTxRisk — SOL drain", () => {
  it("flags ≥95% SOL loss when pre-balance ≥0.1 SOL as DANGER", async () => {
    // 1 SOL pre, 0.04 SOL post → 96% gone
    mockFetch(makeSim({ preBalances: [1_000_000_000], postBalances: [40_000_000] }));
    const report = await analyzeTxRisk(emptyTx(), RPC_URL, USER);
    expect(report.level).toBe("DANGER");
    expect(report.warnings[0].title).toBe("Possible SOL Drain");
  });

  it("does not flag SOL drain when pre-balance is below 0.1 SOL", async () => {
    // 0.05 SOL → below the minimum balance guard
    mockFetch(makeSim({ preBalances: [50_000_000], postBalances: [1_000_000] }));
    const report = await analyzeTxRisk(emptyTx(), RPC_URL, USER);
    expect(report.warnings.map((w) => w.title)).not.toContain("Possible SOL Drain");
  });
});

describe("analyzeTxRisk — high value SOL transfer", () => {
  it("flags ≥10 SOL outflow as WARNING", async () => {
    // 15 SOL → 1 SOL: 14 SOL outflow, 93% loss (below 95% drain threshold)
    mockFetch(makeSim({ preBalances: [15_000_000_000], postBalances: [1_000_000_000] }));
    const report = await analyzeTxRisk(emptyTx(), RPC_URL, USER);
    expect(report.warnings.some((w) => w.title === "High Value Transfer")).toBe(true);
  });

  it("does not flag outflows below 10 SOL", async () => {
    mockFetch(makeSim({ preBalances: [5_000_000_000], postBalances: [1_000_000_000] }));
    const report = await analyzeTxRisk(emptyTx(), RPC_URL, USER);
    expect(report.warnings.map((w) => w.title)).not.toContain("High Value Transfer");
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
    expect(report.warnings.some((w) => w.title === "Multiple Tokens Leaving Wallet")).toBe(true);
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
    expect(report.warnings.map((w) => w.title)).not.toContain("Multiple Tokens Leaving Wallet");
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
    expect(report.warnings.some((w) => w.title === "Impersonator Token")).toBe(true);
  });

  it("flags an incoming token with a fullwidth USDC symbol (ＵＳＤＣ via NFKD)", async () => {
    // U+FF35 U+FF33 U+FF24 U+FF23 — fullwidth ＵＳＤＣ
    mockFetch(incomingTokenSim(FAKE_MINT), new Map([[FAKE_MINT, "ＵＳＤＣ"]]));
    const report = await analyzeTxRisk(emptyTx(), RPC_URL, USER);
    expect(report.level).toBe("DANGER");
    expect(report.warnings.some((w) => w.title === "Impersonator Token")).toBe(true);
  });

  it("does not flag real USDC incoming with the correct mint", async () => {
    mockFetch(incomingTokenSim(USDC_MINT), new Map([[USDC_MINT, "USDC"]]));
    const report = await analyzeTxRisk(emptyTx(), RPC_URL, USER);
    expect(report.warnings.map((w) => w.title)).not.toContain("Impersonator Token");
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
    expect(report.warnings.map((w) => w.title)).not.toContain("Impersonator Token");
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
