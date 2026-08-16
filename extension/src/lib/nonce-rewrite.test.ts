import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import bs58 from "bs58";
import {
  address,
  appendTransactionMessageInstructions,
  compileTransaction,
  compressTransactionMessageUsingAddressLookupTables,
  createTransactionMessage,
  generateKeyPairSigner,
  getBase64EncodedWireTransaction,
  getCompiledTransactionMessageDecoder,
  getCompiledTransactionMessageEncoder,
  getTransactionDecoder,
  setTransactionMessageFeePayer,
  setTransactionMessageLifetimeUsingBlockhash,
  pipe,
} from "@solana/kit";
import { getNonceEncoder, getTransferSolInstruction, SYSTEM_PROGRAM_ADDRESS } from "@solana-program/system";

import { buildCreateNonceAccountTx, buildDurableNonceTransferTx } from "./nonce";
import { rewriteDappTxToDurableNonce } from "./nonce-rewrite";
import { decodeBase64 } from "./solana";

// Canonical public example addresses (same ones nonce.test.ts uses — from the
// @solana/kit durable-nonce docs plus a BIP39-universe recipient). Not real
// account holders.
const WALLET = "4KD1Rdrd89NG7XbzW3xsX9Aqnx2EExJvExiNme6g9iAT";
const RECIPIENT = "HAgk14JpMQLgt6rVgv7cBQFJWFto5Dqxi472uT3DKpqk";
const NONCE_ACCOUNT = "EGtMh4yvXswwHhwVhyPxGrVV2TkLTgUqGodbATEPvojZ";
const NONCE_VALUE = "6NsWSF3vC6z8VNJXR2vP4KcJhc8ZDgY1YyGvXbSQz9Vy";
const BLOCKHASH = "9zorxPPnQ7gY6bV6Yd5hV8n7d2mHbi7XkX4wF7HcMhV8";
const ALT_ADDRESS = "3vZ4RjNi3vJ8Y8bV6Yd5hV8n7d2mHbi7XkX4wF7HcAAA";

const from = address(WALLET);
const to = address(RECIPIENT);

// ─── chrome.storage.local mock (nonce-account lookup) ──────────────────────

function stubChromeStorage(nonceAccounts: Record<string, string> = {}): void {
  const store: Record<string, unknown> = {
    "faraday:state:v1": {
      pairedPubkey: WALLET,
      approvedOrigins: [],
      nonceAccounts,
    },
  };
  vi.stubGlobal("chrome", {
    storage: {
      local: {
        get: (keys: string[], cb: (items: Record<string, unknown>) => void) => {
          const out: Record<string, unknown> = {};
          for (const key of keys) {
            if (key in store) out[key] = store[key];
          }
          cb(out);
        },
        set: (items: Record<string, unknown>, cb?: () => void) => {
          Object.assign(store, items);
          cb?.();
        },
      },
    },
  });
}

// ─── @solana/kit RPC fetch mock ─────────────────────────────────────────────
//
// `solanaRpc` (createSolanaRpc) reads the response body via `.text()`, not
// `.json()` like the hand-rolled `fetch` calls in tx-risk.ts — verified
// against the installed @solana/kit 5.5.1 http transport.

function bytesToBase64(bytes: ArrayLike<number>): string {
  let binary = "";
  for (let i = 0; i < bytes.length; i += 1) binary += String.fromCharCode(bytes[i]);
  return btoa(binary);
}

interface RpcMockOpts {
  /** Nonce account's stored blockhash. Omit to make `getAccountInfo` return a missing account. */
  nonceValue?: string;
  /** Lookup-table address → addresses it contains, for `getMultipleAccounts` (ALT fetch). */
  lookupTables?: Record<string, string[]>;
  /** When set, every RPC call rejects with this error instead of responding. */
  rpcError?: Error;
  /** When true, every RPC call returns a promise that never resolves (timeout test). */
  stall?: boolean;
}

function jsonRpcResponse(id: unknown, result: unknown) {
  return {
    ok: true,
    status: 200,
    text: async () => JSON.stringify({ jsonrpc: "2.0", id, result }),
  };
}

function mockRpcFetch(opts: RpcMockOpts): ReturnType<typeof vi.fn> {
  const nonceDataBase64 = opts.nonceValue
    ? bytesToBase64(
        getNonceEncoder().encode({
          version: 1,
          state: 1,
          authority: from,
          blockhash: address(opts.nonceValue),
          lamportsPerSignature: 5000n,
        })
      )
    : null;

  const fetchMock = vi.fn(async (_url: string, init?: RequestInit) => {
    if (opts.stall) return new Promise(() => undefined);
    if (opts.rpcError) throw opts.rpcError;

    const body = JSON.parse(String(init?.body)) as {
      id: unknown;
      method: string;
      params: unknown[];
    };

    if (body.method === "getAccountInfo") {
      if (!nonceDataBase64) {
        return jsonRpcResponse(body.id, { context: { slot: 1 }, value: null });
      }
      return jsonRpcResponse(body.id, {
        context: { slot: 1 },
        value: {
          data: [nonceDataBase64, "base64"],
          executable: false,
          lamports: 1_447_680,
          owner: SYSTEM_PROGRAM_ADDRESS,
          rentEpoch: 0,
          space: 80,
        },
      });
    }

    if (body.method === "getMultipleAccounts") {
      const requested = body.params[0] as string[];
      const value = requested.map((addr) => {
        const addresses = opts.lookupTables?.[addr];
        if (!addresses) return null;
        return {
          data: {
            program: "address-lookup-table",
            parsed: { type: "lookupTable", info: { addresses } },
            space: 56,
          },
          executable: false,
          lamports: 1_000_000,
          owner: "AddressLookupTab1e1111111111111111111111111",
          rentEpoch: 0,
          space: 56,
        };
      });
      return jsonRpcResponse(body.id, { context: { slot: 1 }, value });
    }

    throw new Error(`nonce-rewrite.test mock: unexpected RPC method ${body.method}`);
  });

  vi.stubGlobal("fetch", fetchMock);
  return fetchMock;
}

afterEach(() => {
  vi.unstubAllGlobals();
});

// ─── fixture builders ────────────────────────────────────────────────────────

/** Plain unsigned legacy SOL transfer — fee payer configurable for the wrong-signer case. */
function legacyTransferTx(feePayer: string, recipient: string = RECIPIENT): string {
  const payer = address(feePayer);
  const transferIx = getTransferSolInstruction({
    source: { address: payer, role: 3 } as never,
    destination: address(recipient),
    amount: 1_000_000_000n,
  });
  const message = pipe(
    createTransactionMessage({ version: "legacy" }),
    (m) => setTransactionMessageFeePayer(payer, m),
    (m) => setTransactionMessageLifetimeUsingBlockhash({ blockhash: BLOCKHASH as never, lastValidBlockHeight: 100n }, m),
    (m) => appendTransactionMessageInstructions([transferIx], m)
  );
  return getBase64EncodedWireTransaction(compileTransaction(message));
}

/** v0 unsigned transfer whose destination is compressed into an ALT reference. */
function v0TransferWithAltTx(): string {
  const transferIx = getTransferSolInstruction({
    source: { address: from, role: 3 } as never,
    destination: to,
    amount: 1_000_000_000n,
  });
  const message = pipe(
    createTransactionMessage({ version: 0 }),
    (m) => setTransactionMessageFeePayer(from, m),
    (m) => setTransactionMessageLifetimeUsingBlockhash({ blockhash: BLOCKHASH as never, lastValidBlockHeight: 100n }, m),
    (m) => appendTransactionMessageInstructions([transferIx], m),
    (m) => compressTransactionMessageUsingAddressLookupTables(m, { [address(ALT_ADDRESS)]: [to] })
  );
  return getBase64EncodedWireTransaction(compileTransaction(message));
}

/** Legacy tx with `n` distinct-recipient transfers — used to push past the size limit after rewrite. */
async function manyRecipientLegacyTx(n: number): Promise<string> {
  const recipients = await Promise.all(Array.from({ length: n }, () => generateKeyPairSigner()));
  const ixs = recipients.map((signer) =>
    getTransferSolInstruction({
      source: { address: from, role: 3 } as never,
      destination: signer.address,
      amount: 1n,
    })
  );
  const message = pipe(
    createTransactionMessage({ version: "legacy" }),
    (m) => setTransactionMessageFeePayer(from, m),
    (m) => setTransactionMessageLifetimeUsingBlockhash({ blockhash: BLOCKHASH as never, lastValidBlockHeight: 100n }, m),
    (m) => appendTransactionMessageInstructions(ixs, m)
  );
  return getBase64EncodedWireTransaction(compileTransaction(message));
}

/** Manually forge a v0 wire tx whose ALT lookup resolves to the wallet's own address. */
function v0TxWithWalletDuplicatedViaAlt(): string {
  const compiled = {
    version: 0 as const,
    header: {
      numSignerAccounts: 1,
      numReadonlySignerAccounts: 0,
      numReadonlyNonSignerAccounts: 0,
    },
    staticAccounts: [from, address(SYSTEM_PROGRAM_ADDRESS)],
    instructions: [
      {
        programAddressIndex: 1,
        accountIndices: [0, 2],
        data: new Uint8Array([2, 0, 0, 0, 0, 202, 154, 59, 0, 0, 0, 0]),
      },
    ],
    addressTableLookups: [
      { lookupTableAddress: address(ALT_ADDRESS), writableIndexes: [0], readonlyIndexes: [] },
    ],
    lifetimeToken: BLOCKHASH,
  };
  const messageBytes = getCompiledTransactionMessageEncoder().encode(compiled);
  const wire = new Uint8Array(1 + 64 + messageBytes.length);
  wire[0] = 1;
  wire.set(messageBytes, 1 + 64);
  return btoa(String.fromCharCode(...wire));
}

/** Decode a rewritten wire tx down to its compiled message, for byte-level assertions. */
function decodeCompiled(txBase64: string) {
  const bytes = decodeBase64(txBase64);
  const tx = getTransactionDecoder().decode(bytes);
  return getCompiledTransactionMessageDecoder().decode(tx.messageBytes);
}

// ─────────────────────────────────────────────────────────────────────────────

describe("rewriteDappTxToDurableNonce", () => {
  beforeEach(() => {
    stubChromeStorage({ [WALLET]: NONCE_ACCOUNT });
  });

  it("rewrites a legacy tx: leading AdvanceNonceAccount, original instructions preserved", async () => {
    mockRpcFetch({ nonceValue: NONCE_VALUE });
    const original = legacyTransferTx(WALLET);

    const result = await rewriteDappTxToDurableNonce(original, WALLET);

    expect(result.reason).toBe("rewritten");
    expect(result.rewritten).toBe(true);
    expect(result.txBase64).not.toBe(original);

    const compiled = decodeCompiled(result.txBase64);
    expect(compiled.version).toBe("legacy");
    expect(compiled.lifetimeToken).toBe(NONCE_VALUE);

    const first = compiled.instructions[0];
    const firstProgram = compiled.staticAccounts[first.programAddressIndex];
    expect(firstProgram).toBe(SYSTEM_PROGRAM_ADDRESS);
    expect(Array.from(first.data ?? [])).toEqual([4, 0, 0, 0]);

    const second = compiled.instructions[1];
    expect(Array.from(second.data ?? []).slice(0, 4)).toEqual([2, 0, 0, 0]); // Transfer
  });

  it("rewrites a v0+ALT tx and preserves addressTableLookups + indices at the byte level", async () => {
    mockRpcFetch({ nonceValue: NONCE_VALUE, lookupTables: { [ALT_ADDRESS]: [to] } });
    const original = v0TransferWithAltTx();
    const originalCompiled = decodeCompiled(original);
    expect(originalCompiled.version).toBe(0);

    const result = await rewriteDappTxToDurableNonce(original, WALLET);

    expect(result.reason).toBe("rewritten");
    const compiled = decodeCompiled(result.txBase64);
    expect(compiled.version).toBe(0);
    expect(compiled.lifetimeToken).toBe(NONCE_VALUE);

    // The ALT reference survives the rewrite: same lookup table address, same
    // writable index (0) for the recipient.
    expect(compiled.version === 0 ? compiled.addressTableLookups : undefined).toEqual([
      { lookupTableAddress: ALT_ADDRESS, writableIndexes: [0], readonlyIndexes: [] },
    ]);

    // Leading instruction is still the advance-nonce ix.
    const first = compiled.instructions[0];
    expect(compiled.staticAccounts[first.programAddressIndex]).toBe(SYSTEM_PROGRAM_ADDRESS);
    expect(Array.from(first.data ?? [])).toEqual([4, 0, 0, 0]);

    // The original transfer instruction (now second) still resolves its
    // destination via the ALT-loaded account slot, not a static one.
    const second = compiled.instructions[1];
    const altLoadedIndex = compiled.staticAccounts.length; // first ALT-loaded account follows all static accounts
    expect(second.accountIndices).toContain(altLoadedIndex);
  });

  it("passes through a tx that already leads with AdvanceNonceAccount", async () => {
    const fetchMock = mockRpcFetch({ nonceValue: NONCE_VALUE });
    const original = buildDurableNonceTransferTx({
      from: WALLET,
      to: RECIPIENT,
      lamports: 1_000_000_000n,
      nonceAccountAddress: NONCE_ACCOUNT,
      nonceValue: NONCE_VALUE,
    });

    const result = await rewriteDappTxToDurableNonce(original, WALLET);

    expect(result.reason).toBe("already-durable-nonce");
    expect(result.rewritten).toBe(false);
    expect(result.txBase64).toBe(original);
    expect(fetchMock).not.toHaveBeenCalled();
  });

  it("passes through a tx with a partial (non-zero) signature already present", async () => {
    const fetchMock = mockRpcFetch({ nonceValue: NONCE_VALUE });
    // buildCreateNonceAccountTx pre-signs the ephemeral nonce-account signer
    // slot, leaving the wallet's slot empty — a genuine partially-signed,
    // multi-signer transaction.
    const { txBase64: original } = await buildCreateNonceAccountTx({
      payer: WALLET,
      rentLamports: 1_447_680n,
      recentBlockhash: { blockhash: BLOCKHASH as never, lastValidBlockHeight: 100n },
    });

    const result = await rewriteDappTxToDurableNonce(original, WALLET);

    expect(result.reason).toBe("has-partial-signatures");
    expect(result.rewritten).toBe(false);
    expect(result.txBase64).toBe(original);
    expect(fetchMock).not.toHaveBeenCalled();
  });

  it("passes through when the fee payer isn't the paired wallet", async () => {
    const fetchMock = mockRpcFetch({ nonceValue: NONCE_VALUE });
    const original = legacyTransferTx(RECIPIENT, WALLET);

    const result = await rewriteDappTxToDurableNonce(original, WALLET);

    expect(result.reason).toBe("fee-payer-not-wallet");
    expect(result.rewritten).toBe(false);
    expect(result.txBase64).toBe(original);
    expect(fetchMock).not.toHaveBeenCalled();
  });

  it("passes through with zero RPC calls when the wallet has no nonce account", async () => {
    vi.unstubAllGlobals();
    stubChromeStorage({}); // no entry for WALLET
    const fetchMock = mockRpcFetch({ nonceValue: NONCE_VALUE });
    const original = legacyTransferTx(WALLET);

    const result = await rewriteDappTxToDurableNonce(original, WALLET);

    expect(result.reason).toBe("no-nonce-account");
    expect(result.rewritten).toBe(false);
    expect(result.txBase64).toBe(original);
    expect(fetchMock).not.toHaveBeenCalled();
  });

  it("passes through when the rewritten tx would exceed the transaction size limit", async () => {
    mockRpcFetch({ nonceValue: NONCE_VALUE });
    const original = await manyRecipientLegacyTx(21);

    const result = await rewriteDappTxToDurableNonce(original, WALLET);

    expect(result.reason).toBe("oversize");
    expect(result.rewritten).toBe(false);
    expect(result.txBase64).toBe(original);
  });

  it("passes through with reason 'rewrite-failed' on an RPC error", async () => {
    mockRpcFetch({ rpcError: new Error("network down") });
    const original = legacyTransferTx(WALLET);

    const result = await rewriteDappTxToDurableNonce(original, WALLET);

    expect(result.reason).toBe("rewrite-failed");
    expect(result.rewritten).toBe(false);
    expect(result.txBase64).toBe(original);
  });

  it("passes through with reason 'timeout' when the RPC stalls past the deadline", async () => {
    mockRpcFetch({ stall: true });
    const original = legacyTransferTx(WALLET);

    const result = await rewriteDappTxToDurableNonce(original, WALLET, { timeoutMs: 20 });

    expect(result.reason).toBe("timeout");
    expect(result.rewritten).toBe(false);
    expect(result.txBase64).toBe(original);
  });

  it("never throws when the wallet's own address is also reachable via an ALT lookup (pins kit behavior)", async () => {
    // @solana/kit 5.5.1: decompiling + recompiling a message where a static
    // signer's address also resolves through a fetched lookup table doesn't
    // throw — it dedups the duplicate account into the single static entry
    // (dropping the now-unused lookup). If a future kit version starts
    // throwing here instead, this test documents the change rather than
    // letting it surface as a silent hard-fail for a dapp sign.
    mockRpcFetch({ nonceValue: NONCE_VALUE, lookupTables: { [ALT_ADDRESS]: [WALLET] } });
    const original = v0TxWithWalletDuplicatedViaAlt();

    await expect(rewriteDappTxToDurableNonce(original, WALLET)).resolves.toMatchObject({
      reason: "rewritten",
      rewritten: true,
    });
  });
});
