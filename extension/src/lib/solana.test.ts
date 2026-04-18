import { describe, expect, it } from "vitest";
import bs58 from "bs58";

import {
  decodeBase64,
  encodeBase64,
  isValidSolanaAddress,
  pubkeyToBytes,
  validateSignedTransactionMatch,
  validateUnsignedTransactionPayload
} from "./solana";

/* --------------------------------------------------------------------- *
 * Fixture helpers
 *
 * These hand-roll just enough of the Solana wire format to exercise the
 * parser without pulling @solana/web3.js into the test tree. Kept in sync
 * with parseEnvelope / parseSignerAddresses in solana.ts.
 * --------------------------------------------------------------------- */

interface Account {
  bs58: string;
  bytes: Uint8Array;
}

function makeAccount(seed: number): Account {
  const bytes = new Uint8Array(32);
  for (let i = 0; i < 32; i += 1) {
    bytes[i] = (seed + i * 7) & 0xff;
  }
  return { bs58: bs58.encode(bytes), bytes };
}

function concat(parts: Uint8Array[]): Uint8Array {
  let total = 0;
  for (const p of parts) total += p.length;
  const out = new Uint8Array(total);
  let offset = 0;
  for (const p of parts) {
    out.set(p, offset);
    offset += p.length;
  }
  return out;
}

interface BuildTxOpts {
  signers: Account[];
  readonlyAccounts?: Account[];
  versioned?: boolean;
  /** One Uint8Array per signer slot, each 64 bytes. If omitted, zero-filled. */
  signatures?: Uint8Array[];
  /** Bytes appended after the account key table (instruction section stub). */
  tail?: Uint8Array;
}

function buildTxBytes(opts: BuildTxOpts): Uint8Array {
  const sigCount = opts.signers.length;
  const sigCountByte = new Uint8Array([sigCount]);
  const sigBlock = new Uint8Array(sigCount * 64);
  if (opts.signatures) {
    opts.signatures.forEach((sig, i) => sigBlock.set(sig, i * 64));
  }

  const versionPrefix = opts.versioned ? new Uint8Array([0x80]) : new Uint8Array();
  const header = new Uint8Array([sigCount, 0, opts.readonlyAccounts?.length ?? 0]);

  const keys = [...opts.signers, ...(opts.readonlyAccounts ?? [])];
  const keyCountByte = new Uint8Array([keys.length]);
  const keyBlock = new Uint8Array(keys.length * 32);
  keys.forEach((k, i) => keyBlock.set(k.bytes, i * 32));

  const tail = opts.tail ?? new Uint8Array([0]);
  const message = concat([versionPrefix, header, keyCountByte, keyBlock, tail]);
  return concat([sigCountByte, sigBlock, message]);
}

function b64(bytes: Uint8Array): string {
  return encodeBase64(bytes);
}

function nonZeroSig(fill: number): Uint8Array {
  return new Uint8Array(64).fill(fill);
}

/* --------------------------------------------------------------------- *
 * isValidSolanaAddress
 * --------------------------------------------------------------------- */

describe("isValidSolanaAddress", () => {
  it("accepts the system program", () => {
    expect(isValidSolanaAddress("11111111111111111111111111111111")).toBe(true);
  });

  it("accepts the USDC mainnet mint", () => {
    expect(isValidSolanaAddress("EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v")).toBe(true);
  });

  it("accepts the SOL wrapped mint", () => {
    expect(isValidSolanaAddress("So11111111111111111111111111111111111111112")).toBe(true);
  });

  it("rejects empty string", () => {
    expect(isValidSolanaAddress("")).toBe(false);
  });

  it("rejects whitespace-only", () => {
    expect(isValidSolanaAddress("   ")).toBe(false);
  });

  it("trims leading/trailing whitespace on otherwise-valid input", () => {
    expect(isValidSolanaAddress("  11111111111111111111111111111111  ")).toBe(true);
  });

  it("rejects strings under 32 chars", () => {
    expect(isValidSolanaAddress("1111111111111111")).toBe(false);
  });

  it("rejects strings over 44 chars", () => {
    const over = "1".repeat(45);
    expect(isValidSolanaAddress(over)).toBe(false);
  });

  it("rejects non-base58 characters (0, O, I, l)", () => {
    expect(isValidSolanaAddress("0".repeat(32))).toBe(false);
    expect(isValidSolanaAddress("O".repeat(32))).toBe(false);
    expect(isValidSolanaAddress("I".repeat(32))).toBe(false);
    expect(isValidSolanaAddress("l".repeat(32))).toBe(false);
  });

  it("rejects base58 with non-32-byte decoded length", () => {
    // 31 bytes of data, base58-encoded — passes regex but decodes to 31 bytes
    const short = bs58.encode(new Uint8Array(31).fill(5));
    expect(isValidSolanaAddress(short)).toBe(false);
  });

  it("generated fixture addresses are recognised as valid", () => {
    const a = makeAccount(42);
    expect(isValidSolanaAddress(a.bs58)).toBe(true);
  });
});

/* --------------------------------------------------------------------- *
 * encodeBase64 / decodeBase64
 * --------------------------------------------------------------------- */

describe("base64 round-trip", () => {
  it("round-trips empty input", () => {
    const input = new Uint8Array();
    expect(decodeBase64(encodeBase64(input))).toEqual(input);
  });

  it("round-trips a small byte array", () => {
    const input = new Uint8Array([1, 2, 3, 4, 5, 250, 251, 252]);
    expect(decodeBase64(encodeBase64(input))).toEqual(input);
  });

  it("round-trips 1KB of pseudo-random bytes", () => {
    const input = new Uint8Array(1024);
    for (let i = 0; i < input.length; i += 1) input[i] = (i * 31 + 7) & 0xff;
    expect(decodeBase64(encodeBase64(input))).toEqual(input);
  });

  it("decoding invalid base64 throws", () => {
    expect(() => decodeBase64("@@@not-base64!!!")).toThrow();
  });
});

/* --------------------------------------------------------------------- *
 * pubkeyToBytes
 * --------------------------------------------------------------------- */

describe("pubkeyToBytes", () => {
  it("returns 32 bytes for a valid address", () => {
    const a = makeAccount(1);
    expect(pubkeyToBytes(a.bs58)).toEqual(a.bytes);
  });

  it("throws for addresses that decode to the wrong length", () => {
    const short = bs58.encode(new Uint8Array(30).fill(1));
    expect(() => pubkeyToBytes(short)).toThrow(/length/i);
  });
});

/* --------------------------------------------------------------------- *
 * validateUnsignedTransactionPayload
 * --------------------------------------------------------------------- */

describe("validateUnsignedTransactionPayload", () => {
  const signer = makeAccount(10);
  const other = makeAccount(20);

  it("accepts a legacy transaction with one signer slot", () => {
    const bytes = buildTxBytes({ signers: [signer] });
    expect(() => validateUnsignedTransactionPayload(b64(bytes))).not.toThrow();
  });

  it("accepts a v0 (versioned) transaction", () => {
    const bytes = buildTxBytes({ signers: [signer], versioned: true });
    expect(() => validateUnsignedTransactionPayload(b64(bytes))).not.toThrow();
  });

  it("accepts a tx when the expected signer is present", () => {
    const bytes = buildTxBytes({ signers: [signer, other] });
    expect(() =>
      validateUnsignedTransactionPayload(b64(bytes), signer.bs58)
    ).not.toThrow();
  });

  it("rejects when the expected signer is not in the accounts", () => {
    const bytes = buildTxBytes({ signers: [other] });
    expect(() =>
      validateUnsignedTransactionPayload(b64(bytes), signer.bs58)
    ).toThrow(/paired signer/i);
  });

  it("rejects invalid base64", () => {
    expect(() => validateUnsignedTransactionPayload("@@@not-base64")).toThrow(
      /base64/i
    );
  });

  it("rejects a tx with zero signer slots", () => {
    // sigCount=0, tiny message body
    const body = concat([
      new Uint8Array([0]), // sigCount = 0
      new Uint8Array([0, 0, 0]), // header
      new Uint8Array([0]), // key count = 0
      new Uint8Array([0]) // tail
    ]);
    expect(() => validateUnsignedTransactionPayload(b64(body))).toThrow(
      /no signer slots/i
    );
  });

  it("rejects a truncated signature block", () => {
    // Claim 2 signatures but only include 1 × 64 bytes (64 bytes missing)
    const truncated = concat([
      new Uint8Array([2]),
      new Uint8Array(64),
      new Uint8Array([1, 0, 0]),
      new Uint8Array([1]),
      new Uint8Array(32)
    ]);
    expect(() => validateUnsignedTransactionPayload(b64(truncated))).toThrow(
      /malformed/i
    );
  });

  it("rejects when numRequiredSignatures > keyCount", () => {
    // sigCount=1 (slot exists), header claims 1 required sig but keys table has 0
    const bytes = concat([
      new Uint8Array([1]),
      new Uint8Array(64), // 1 sig slot
      new Uint8Array([1, 0, 0]), // numReq=1
      new Uint8Array([0]), // keyCount=0
      new Uint8Array([0]) // tail
    ]);
    expect(() => validateUnsignedTransactionPayload(b64(bytes))).toThrow(
      /signer count/i
    );
  });
});

/* --------------------------------------------------------------------- *
 * validateSignedTransactionMatch
 * --------------------------------------------------------------------- */

describe("validateSignedTransactionMatch", () => {
  const signer = makeAccount(100);
  const other = makeAccount(200);

  it("returns the signed bytes when signed and unsigned messages match", () => {
    const unsigned = buildTxBytes({ signers: [signer] });
    const signed = buildTxBytes({
      signers: [signer],
      signatures: [nonZeroSig(0xab)]
    });
    const result = validateSignedTransactionMatch(b64(unsigned), b64(signed), signer.bs58);
    expect(result.length).toBe(signed.length);
  });

  it("rejects when signature count differs", () => {
    const unsigned = buildTxBytes({ signers: [signer] });
    const signed = buildTxBytes({
      signers: [signer, other],
      signatures: [nonZeroSig(1), nonZeroSig(2)]
    });
    expect(() =>
      validateSignedTransactionMatch(b64(unsigned), b64(signed), signer.bs58)
    ).toThrow(/signature count/i);
  });

  it("rejects when message length differs", () => {
    const unsigned = buildTxBytes({ signers: [signer] });
    const signed = buildTxBytes({
      signers: [signer],
      signatures: [nonZeroSig(0xab)],
      tail: new Uint8Array([0, 0, 0, 0, 0]) // extra bytes after keys
    });
    expect(() =>
      validateSignedTransactionMatch(b64(unsigned), b64(signed), signer.bs58)
    ).toThrow(/length mismatch/i);
  });

  it("rejects when message bytes differ at the same length", () => {
    const unsigned = buildTxBytes({ signers: [signer] });
    // Same structure but different key bytes → same length, different content
    const tamperedSigner = { ...signer, bytes: signer.bytes.map((b) => b ^ 0xff) };
    const signed = buildTxBytes({
      signers: [tamperedSigner],
      signatures: [nonZeroSig(0xab)]
    });
    expect(() =>
      validateSignedTransactionMatch(b64(unsigned), b64(signed), signer.bs58)
    ).toThrow(/does not match/i);
  });

  it("rejects when the expected signer is missing from the account list", () => {
    const unsigned = buildTxBytes({ signers: [other] });
    const signed = buildTxBytes({
      signers: [other],
      signatures: [nonZeroSig(0xab)]
    });
    expect(() =>
      validateSignedTransactionMatch(b64(unsigned), b64(signed), signer.bs58)
    ).toThrow(/paired signer/i);
  });

  it("rejects when the signer slot is all zero", () => {
    const unsigned = buildTxBytes({ signers: [signer] });
    const signed = buildTxBytes({
      signers: [signer]
      // signatures omitted → zero-filled
    });
    expect(() =>
      validateSignedTransactionMatch(b64(unsigned), b64(signed), signer.bs58)
    ).toThrow(/signature is missing/i);
  });

  it("rejects invalid base64 in either payload", () => {
    const unsigned = buildTxBytes({ signers: [signer] });
    expect(() =>
      validateSignedTransactionMatch(b64(unsigned), "@@@not-base64", signer.bs58)
    ).toThrow(/base64/i);
  });
});
