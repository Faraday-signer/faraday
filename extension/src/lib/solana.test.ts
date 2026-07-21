import { describe, expect, it } from "vitest";
import bs58 from "bs58";
import nacl from "tweetnacl";

import {
  buildSolanaOffchainMessage,
  buildSignMessageQrPayload,
  decodeSignMessageQrPayload,
  decodeHexSignature,
  decodeBase64,
  describeSignMessageBytes,
  encodeBase64,
  formatSiwsMessage,
  isValidSolanaAddress,
  parseFaradaySigEnvelope,
  parseIkaApprovalMessage,
  parseSolanaOffchainMessage,
  pubkeyToBytes,
  spliceFaradaySignature,
  validateSignedMessage,
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

/* --------------------------------------------------------------------- *
 * parseFaradaySigEnvelope / spliceFaradaySignature
 *
 * The Pi emits a compact `faraday:sig:<base64(version||pubkey||sig)>`
 * payload on the return leg. The extension already holds the unsigned tx
 * in session state, so we only need to splice the 64-byte sig into the
 * correct signer slot and verify it against the message bytes.
 * --------------------------------------------------------------------- */

function keypairAccount(seed: number): { bs58: string; bytes: Uint8Array; secretKey: Uint8Array } {
  const seedBytes = new Uint8Array(32).fill(seed);
  const kp = nacl.sign.keyPair.fromSeed(seedBytes);
  return { bs58: bs58.encode(kp.publicKey), bytes: kp.publicKey, secretKey: kp.secretKey };
}

function buildEnvelope(pubkey: Uint8Array, signature: Uint8Array, version = 1): string {
  const payload = new Uint8Array(1 + 32 + 64);
  payload[0] = version;
  payload.set(pubkey, 1);
  payload.set(signature, 33);
  return `faraday:sig:${encodeBase64(payload)}`;
}

describe("parseFaradaySigEnvelope", () => {
  const kp = keypairAccount(77);
  const sig = new Uint8Array(64).fill(0xab);

  it("parses a well-formed envelope", () => {
    const parsed = parseFaradaySigEnvelope(buildEnvelope(kp.bytes, sig));
    expect(parsed.pubkey).toEqual(kp.bytes);
    expect(parsed.signature).toEqual(sig);
  });

  it("rejects missing prefix", () => {
    const body = buildEnvelope(kp.bytes, sig).slice("faraday:sig:".length);
    expect(() => parseFaradaySigEnvelope(body)).toThrow(/prefix/i);
  });

  it("rejects unsupported version byte", () => {
    expect(() => parseFaradaySigEnvelope(buildEnvelope(kp.bytes, sig, 9))).toThrow(
      /version/i
    );
  });

  it("rejects wrong payload length", () => {
    const short = new Uint8Array(50);
    const text = `faraday:sig:${encodeBase64(short)}`;
    expect(() => parseFaradaySigEnvelope(text)).toThrow(/length/i);
  });

  it("rejects invalid base64", () => {
    expect(() => parseFaradaySigEnvelope("faraday:sig:@@@not-base64")).toThrow(
      /base64/i
    );
  });
});

describe("spliceFaradaySignature", () => {
  const signer = keypairAccount(33);
  const other = keypairAccount(44);

  function signTx(signerBytes: Uint8Array, secretKey: Uint8Array): {
    unsignedBase64: string;
    envelope: string;
    signature: Uint8Array;
  } {
    const unsigned = buildTxBytes({
      signers: [{ bs58: bs58.encode(signerBytes), bytes: signerBytes }]
    });
    // Mirror parseEnvelope: sig count is one byte here (small tx), so
    // signatures start at offset 1, message starts at 1 + sigCount*64.
    const sigCount = unsigned[0];
    const messageBytes = unsigned.slice(1 + sigCount * 64);
    const signature = nacl.sign.detached(messageBytes, secretKey);
    const envelope = buildEnvelope(signerBytes, signature);
    return { unsignedBase64: b64(unsigned), envelope, signature };
  }

  it("reconstructs a signed tx with the signature in the signer slot", () => {
    const { unsignedBase64, envelope, signature } = signTx(signer.bytes, signer.secretKey);
    const signedBase64 = spliceFaradaySignature(unsignedBase64, envelope, signer.bs58);
    const signedBytes = decodeBase64(signedBase64);
    expect(signedBytes.slice(1, 65)).toEqual(signature);
    // Downstream validation accepts the result without modification.
    expect(() =>
      validateSignedTransactionMatch(unsignedBase64, signedBase64, signer.bs58)
    ).not.toThrow();
  });

  it("rejects envelopes whose pubkey does not match the expected signer", () => {
    const { unsignedBase64 } = signTx(signer.bytes, signer.secretKey);
    const badEnvelope = buildEnvelope(other.bytes, new Uint8Array(64).fill(1));
    expect(() =>
      spliceFaradaySignature(unsignedBase64, badEnvelope, signer.bs58)
    ).toThrow(/signer/i);
  });

  it("rejects when the signature does not verify against the message", () => {
    const { unsignedBase64 } = signTx(signer.bytes, signer.secretKey);
    const forgedEnvelope = buildEnvelope(signer.bytes, new Uint8Array(64).fill(0xaa));
    expect(() =>
      spliceFaradaySignature(unsignedBase64, forgedEnvelope, signer.bs58)
    ).toThrow(/verify/i);
  });

  it("rejects when the expected signer is not listed in the tx accounts", () => {
    const { envelope } = signTx(signer.bytes, signer.secretKey);
    const unsignedOther = buildTxBytes({ signers: [{ bs58: other.bs58, bytes: other.bytes }] });
    expect(() =>
      spliceFaradaySignature(b64(unsignedOther), envelope, signer.bs58)
    ).toThrow(/paired signer/i);
  });

  it("splices into the correct slot for a multi-signer tx", () => {
    // Tx with [other, signer] — splice must target index 1.
    const unsigned = buildTxBytes({
      signers: [
        { bs58: other.bs58, bytes: other.bytes },
        { bs58: signer.bs58, bytes: signer.bytes }
      ]
    });
    const sigCount = unsigned[0];
    const messageBytes = unsigned.slice(1 + sigCount * 64);
    const signature = nacl.sign.detached(messageBytes, signer.secretKey);
    const envelope = buildEnvelope(signer.bytes, signature);

    const signedBase64 = spliceFaradaySignature(b64(unsigned), envelope, signer.bs58);
    const signedBytes = decodeBase64(signedBase64);
    // Slot 0 (other) still zeros; slot 1 (signer) holds our signature.
    expect(signedBytes.slice(1, 65)).toEqual(new Uint8Array(64));
    expect(signedBytes.slice(65, 129)).toEqual(signature);
  });
});

describe("buildSignMessageQrPayload", () => {
  it("builds a 0xFF-prefixed base64 payload", () => {
    const message = new Uint8Array(36).fill(0x41);
    const payload = buildSignMessageQrPayload(message);
    const decoded = decodeBase64(payload);
    expect(decoded[0]).toBe(0xff);
    expect(decoded.slice(1)).toEqual(message);
  });

  it("rejects messages below firmware scan threshold", () => {
    const message = new Uint8Array(35).fill(0x41);
    expect(() => buildSignMessageQrPayload(message)).toThrow(/too short/i);
  });

  it("decodes a 0xFF-prefixed sign-message QR payload", () => {
    const message = new TextEncoder().encode("expires 2030-01-01 00:00:00: approve transfer");
    const payload = buildSignMessageQrPayload(message);
    expect(decodeSignMessageQrPayload(payload)).toEqual(message);
  });
});

describe("Solana off-chain messages", () => {
  const text = "expires 2030-01-01 00:00:00: approve transfer 1000000000 lamports to 9abc | wallet: treasury proposal: 42";

  it("builds and parses Solana off-chain message bytes", () => {
    const message = buildSolanaOffchainMessage(text);
    expect(message[0]).toBe(0xff);
    expect(new TextDecoder().decode(message.slice(1, 16))).toBe("solana offchain");

    const parsed = parseSolanaOffchainMessage(message);
    expect(parsed?.version).toBe(0);
    expect(parsed?.format).toBe(0);
    expect(parsed?.bodyText).toBe(text);
  });

  it("describes wrapped messages by their human-readable body", () => {
    const message = buildSolanaOffchainMessage(text);
    const preview = describeSignMessageBytes(message);
    expect(preview.title).toBe("Solana off-chain message");
    expect(preview.text).toBe(text);
    expect(preview.wrapped).toBe(true);
    // The fixture body is a real Ika approval — parser should attach details.
    expect(preview.ika?.action).toBe("approve");
    expect(preview.ika?.proposalIndex).toBe("42");
  });

  it("rejects unknown off-chain version or format bytes", () => {
    const body = new TextEncoder().encode(text);
    const buildHeader = (version: number, format: number): Uint8Array => {
      const msg = new Uint8Array(20 + body.length);
      msg.set(new Uint8Array([
        0xff, 0x73, 0x6f, 0x6c, 0x61, 0x6e, 0x61, 0x20,
        0x6f, 0x66, 0x66, 0x63, 0x68, 0x61, 0x69, 0x6e
      ]), 0);
      msg[16] = version;
      msg[17] = format;
      msg[18] = body.length & 0xff;
      msg[19] = body.length >> 8;
      msg.set(body, 20);
      return msg;
    };
    expect(parseSolanaOffchainMessage(buildHeader(1, 0))).toBeNull();
    expect(parseSolanaOffchainMessage(buildHeader(0, 1))).toBeNull();
  });

  it("returns null for ordinary messages", () => {
    const message = new TextEncoder().encode("ordinary message");
    expect(parseSolanaOffchainMessage(message)).toBeNull();
    expect(describeSignMessageBytes(message)).toEqual({
      title: "Message",
      text: "ordinary message",
      wrapped: false
    });
  });

  it("rejects a wrapped message with the wrong body length", () => {
    const message = buildSolanaOffchainMessage(text).slice(0, -1);
    expect(() => parseSolanaOffchainMessage(message)).toThrow(/length/i);
  });
});

describe("parseIkaApprovalMessage", () => {
  it("parses an approve lamport transfer", () => {
    const details = parseIkaApprovalMessage(
      "expires 2030-01-01 00:00:00: approve transfer 1000000000 lamports to 9abcDEFghijKLMnopQRstuvWXyz12345678ABCDefgh | wallet: treasury proposal: 42"
    );
    expect(details).not.toBeNull();
    expect(details?.action).toBe("approve");
    expect(details?.expires).toBe("2030-01-01 00:00:00");
    expect(details?.walletName).toBe("treasury");
    expect(details?.proposalIndex).toBe("42");
    expect(details?.content).toEqual({
      kind: "transfer",
      amountLamports: 1_000_000_000n,
      to: "9abcDEFghijKLMnopQRstuvWXyz12345678ABCDefgh"
    });
  });

  it("parses propose and cancel actions", () => {
    expect(
      parseIkaApprovalMessage(
        "expires 2030-01-01 00:00:00: propose transfer 1 lamports to addr | wallet: t proposal: 1"
      )?.action
    ).toBe("propose");
    expect(
      parseIkaApprovalMessage(
        "expires 2030-01-01 00:00:00: cancel transfer 1 lamports to addr | wallet: t proposal: 1"
      )?.action
    ).toBe("cancel");
  });

  it("parses an SPL transfer", () => {
    const details = parseIkaApprovalMessage(
      "expires 2030-01-01 00:00:00: approve transfer 1500000 of mint EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v to 9abcDEFghijKLMnopQRstuvWXyz12345678ABCDefgh | wallet: treasury proposal: 12"
    );
    expect(details?.content).toEqual({
      kind: "spl-transfer",
      amount: "1500000",
      mint: "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v",
      to: "9abcDEFghijKLMnopQRstuvWXyz12345678ABCDefgh"
    });
  });

  it("parses meta-intent variants", () => {
    expect(
      parseIkaApprovalMessage(
        "expires 2030-01-01 00:00:00: approve add intent definition_hash: 0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef | wallet: t proposal: 1"
      )?.content
    ).toEqual({
      kind: "add-intent",
      definitionHash:
        "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"
    });
    expect(
      parseIkaApprovalMessage(
        "expires 2030-01-01 00:00:00: approve remove intent 3 | wallet: t proposal: 1"
      )?.content
    ).toEqual({ kind: "remove-intent", index: "3" });
    expect(
      parseIkaApprovalMessage(
        "expires 2030-01-01 00:00:00: approve update intent 2 definition_hash: deadbeef | wallet: t proposal: 1"
      )?.content
    ).toEqual({ kind: "update-intent", index: "2", definitionHash: "deadbeef" });
  });

  it("falls through to `other` for cross-chain content", () => {
    const details = parseIkaApprovalMessage(
      "expires 2030-01-01 00:00:00: approve send 12345 sats to bc1q-pkh:0xdead from utxo 0xabcd:0 | wallet: treasury proposal: 5"
    );
    expect(details?.content.kind).toBe("other");
  });

  it("returns null for non-Ika bodies", () => {
    expect(parseIkaApprovalMessage("just some text")).toBeNull();
    expect(
      parseIkaApprovalMessage(
        "expires 2030: approve transfer 1 lamports to x | wallet: t proposal: 1"
      )
    ).not.toBeNull(); // tolerant of timestamp shape (still parses)
    expect(
      parseIkaApprovalMessage(
        "expires 2030-01-01 00:00:00: revoke transfer 1 lamports to x | wallet: t proposal: 1"
      )
    ).toBeNull(); // unknown verb → reject
  });

  // ── Edge cases ───────────────────────────────────────────────────────

  it("uses the LAST ` | ` against content-injection spoofing", () => {
    // The on-chain trailer is always last. An injected fake trailer in the
    // content portion must not surface to the user — the device has to
    // display what binds on chain.
    const details = parseIkaApprovalMessage(
      "expires 2030-01-01 00:00:00: approve send free-text-payload | wallet: SPOOF proposal: 0 | wallet: REAL proposal: 99"
    );
    expect(details?.walletName).toBe("REAL");
    expect(details?.proposalIndex).toBe("99");
  });

  it("uses the LAST ` proposal: ` so wallet names cannot spoof the index", () => {
    const details = parseIkaApprovalMessage(
      "expires 2030-01-01 00:00:00: approve transfer 1 lamports to addr | wallet: weird name proposal: 7 proposal: 42"
    );
    expect(details?.walletName).toBe("weird name proposal: 7");
    expect(details?.proposalIndex).toBe("42");
  });

  it("handles the u64-max proposal index", () => {
    const details = parseIkaApprovalMessage(
      "expires 2030-01-01 00:00:00: approve transfer 1 lamports to addr | wallet: t proposal: 18446744073709551615"
    );
    expect(details?.proposalIndex).toBe("18446744073709551615");
  });

  it("rejects a verb-only action (no content)", () => {
    expect(
      parseIkaApprovalMessage(
        "expires 2030-01-01 00:00:00: approve | wallet: t proposal: 1"
      )
    ).toBeNull();
  });
});

describe("describeSignMessageBytes edge cases", () => {
  function wrapped(body: string): Uint8Array {
    return buildSolanaOffchainMessage(body);
  }

  it("describes an empty UTF-8 body as a Solana off-chain message", () => {
    // buildSolanaOffchainMessage rejects empty, so handcraft a 0-length
    // body the way the on-chain spec actually allows it.
    const out = new Uint8Array(20);
    out.set(new Uint8Array([
      0xff, 0x73, 0x6f, 0x6c, 0x61, 0x6e, 0x61, 0x20,
      0x6f, 0x66, 0x66, 0x63, 0x68, 0x61, 0x69, 0x6e
    ]), 0);
    // version=0, format=0, len=0
    const parsed = parseSolanaOffchainMessage(out);
    expect(parsed?.bodyText).toBe("");
    const preview = describeSignMessageBytes(out);
    expect(preview.title).toBe("Solana off-chain message");
    expect(preview.text).toBe("");
    expect(preview.ika).toBeUndefined();
  });

  it("handles a max-length body (u16 boundary, ~64 KiB)", () => {
    const body = "A".repeat(0xffff);
    const message = wrapped(body);
    const parsed = parseSolanaOffchainMessage(message);
    expect(parsed?.bodyText.length).toBe(0xffff);
    // Not an Ika body → generic preview, but must not OOM/crash.
    const preview = describeSignMessageBytes(message);
    expect(preview.wrapped).toBe(true);
    expect(preview.ika).toBeUndefined();
  });

  it("rejects a body that lies about its declared length", () => {
    // Build a header that claims 100 bytes but only 5 follow.
    const out = new Uint8Array(25);
    out.set(new Uint8Array([
      0xff, 0x73, 0x6f, 0x6c, 0x61, 0x6e, 0x61, 0x20,
      0x6f, 0x66, 0x66, 0x63, 0x68, 0x61, 0x69, 0x6e
    ]), 0);
    out[18] = 100;
    out[19] = 0;
    expect(() => parseSolanaOffchainMessage(out)).toThrow(/length/i);
  });

  it("returns null for bytes shorter than the off-chain header itself", () => {
    expect(parseSolanaOffchainMessage(new Uint8Array(5))).toBeNull();
    expect(parseSolanaOffchainMessage(new Uint8Array(0))).toBeNull();
  });
});

describe("message signature helpers", () => {
  const seed = new Uint8Array(32).fill(7);
  const keypair = nacl.sign.keyPair.fromSeed(seed);
  const signer = bs58.encode(keypair.publicKey);
  const message = new TextEncoder().encode("faraday message signing fixture");
  const signature = nacl.sign.detached(message, keypair.secretKey);
  const signatureHex = Buffer.from(signature).toString("hex");

  it("decodes a 64-byte hex signature", () => {
    const decoded = decodeHexSignature(signatureHex);
    expect(decoded.length).toBe(64);
    expect(decoded).toEqual(signature);
  });

  it("accepts a valid signature for the expected signer", () => {
    const verified = validateSignedMessage(message, signatureHex, signer);
    expect(verified).toEqual(signature);
  });

  it("rejects malformed signature hex", () => {
    expect(() => decodeHexSignature("not-hex")).toThrow(/hex signature/i);
  });

  it("rejects signature that does not match message", () => {
    const tampered = new TextEncoder().encode("faraday message signing fixture (tampered)");
    expect(() => validateSignedMessage(tampered, signatureHex, signer)).toThrow(/does not match/i);
  });
});

describe("formatSiwsMessage", () => {
  const address = "11111111111111111111111111111111";

  it("emits header-only message when no optional fields are set", () => {
    const text = formatSiwsMessage({ domain: "example.com", address });
    expect(text).toBe(
      `example.com wants you to sign in with your Solana account:\n${address}`
    );
  });

  it("includes a statement block separated by blank lines", () => {
    const text = formatSiwsMessage({
      domain: "example.com",
      address,
      statement: "Log in to Example."
    });
    expect(text).toBe(
      [
        `example.com wants you to sign in with your Solana account:`,
        address,
        ``,
        `Log in to Example.`
      ].join("\n")
    );
  });

  it("emits fields in the spec's fixed order", () => {
    const text = formatSiwsMessage({
      domain: "example.com",
      address,
      uri: "https://example.com",
      version: "1",
      chainId: "solana:mainnet",
      nonce: "abc123",
      issuedAt: "2026-01-01T00:00:00Z"
    });
    expect(text).toBe(
      [
        `example.com wants you to sign in with your Solana account:`,
        address,
        ``,
        `URI: https://example.com`,
        `Version: 1`,
        `Chain ID: solana:mainnet`,
        `Nonce: abc123`,
        `Issued At: 2026-01-01T00:00:00Z`
      ].join("\n")
    );
  });

  it("renders resources as a bulleted list after the field block", () => {
    const text = formatSiwsMessage({
      domain: "example.com",
      address,
      uri: "https://example.com",
      resources: ["https://example.com/tos", "https://example.com/privacy"]
    });
    expect(text).toContain("Resources:\n- https://example.com/tos\n- https://example.com/privacy");
  });

  it("rejects input missing the required domain or address", () => {
    expect(() => formatSiwsMessage({ domain: "", address })).toThrow(/domain/i);
    expect(() => formatSiwsMessage({ domain: "example.com", address: "" })).toThrow(/address/i);
  });
});
