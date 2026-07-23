import { describe, expect, it } from "vitest";
import bs58 from "bs58";
import { generateKeyPairSigner } from "@solana/kit";
import { SYSTEM_PROGRAM_ADDRESS } from "@solana-program/system";

import { buildCreateNonceAccountTx, buildDurableNonceTransferTx } from "./nonce";
import { decodeBase64 } from "./solana";

// Canonical public example addresses (from the @solana/kit durable-nonce docs
// + a BIP39-universe recipient). Not real account holders.
const WALLET = "4KD1Rdrd89NG7XbzW3xsX9Aqnx2EExJvExiNme6g9iAT";
const NONCE_ACCOUNT = "EGtMh4yvXswwHhwVhyPxGrVV2TkLTgUqGodbATEPvojZ";
const RECIPIENT = "HAgk14JpMQLgt6rVgv7cBQFJWFto5Dqxi472uT3DKpqk";
const NONCE_VALUE = "9zorxPPnQ7gY6bV6Yd5hV8n7d2mHbi7XkX4wF7HcMhV8"; // synthetic 32-byte blockhash

const SYSTEM_PROGRAM_B58 = SYSTEM_PROGRAM_ADDRESS as string;

interface ShortVec {
  value: number;
  offset: number;
}

function readShortVec(data: Uint8Array, offset: number): ShortVec {
  let value = 0;
  let shift = 0;
  let cursor = offset;
  while (true) {
    const byte = data[cursor];
    cursor += 1;
    value |= (byte & 0x7f) << shift;
    if ((byte & 0x80) === 0) break;
    shift += 7;
  }
  return { value, offset: cursor };
}

interface DecodedIx {
  programId: string;
  accountIndices: number[];
  data: Uint8Array;
}

interface DecodedTx {
  signatures: Uint8Array[];
  accountKeys: string[];
  recentBlockhash: Uint8Array;
  instructions: DecodedIx[];
}

/** Minimal legacy-transaction decoder, just enough for these assertions. */
function decodeLegacyTx(txBase64: string): DecodedTx {
  const bytes = decodeBase64(txBase64);
  const sigCount = readShortVec(bytes, 0);
  const signatures: Uint8Array[] = [];
  let cursor = sigCount.offset;
  for (let i = 0; i < sigCount.value; i += 1) {
    signatures.push(bytes.slice(cursor, cursor + 64));
    cursor += 64;
  }

  // Message header (legacy — no version byte).
  cursor += 3; // numRequiredSignatures, numReadonlySigned, numReadonlyUnsigned

  const keyCount = readShortVec(bytes, cursor);
  cursor = keyCount.offset;
  const accountKeys: string[] = [];
  for (let i = 0; i < keyCount.value; i += 1) {
    accountKeys.push(bs58.encode(bytes.slice(cursor, cursor + 32)));
    cursor += 32;
  }

  const recentBlockhash = bytes.slice(cursor, cursor + 32);
  cursor += 32;

  const ixCount = readShortVec(bytes, cursor);
  cursor = ixCount.offset;
  const instructions: DecodedIx[] = [];
  for (let i = 0; i < ixCount.value; i += 1) {
    const programIdIndex = bytes[cursor];
    cursor += 1;
    const accs = readShortVec(bytes, cursor);
    cursor = accs.offset;
    const accountIndices: number[] = [];
    for (let a = 0; a < accs.value; a += 1) {
      accountIndices.push(bytes[cursor]);
      cursor += 1;
    }
    const dataLen = readShortVec(bytes, cursor);
    cursor = dataLen.offset;
    const data = bytes.slice(cursor, cursor + dataLen.value);
    cursor += dataLen.value;
    instructions.push({
      programId: accountKeys[programIdIndex],
      accountIndices,
      data,
    });
  }

  return { signatures, accountKeys, recentBlockhash, instructions };
}

function isAllZero(bytes: Uint8Array): boolean {
  return bytes.every((b) => b === 0);
}

describe("buildDurableNonceTransferTx", () => {
  const tx = buildDurableNonceTransferTx({
    from: WALLET,
    to: RECIPIENT,
    lamports: 1_000_000_000n,
    nonceAccountAddress: NONCE_ACCOUNT,
    nonceValue: NONCE_VALUE,
  });
  const decoded = decodeLegacyTx(tx);

  it("has a single (wallet) signer slot", () => {
    expect(decoded.signatures.length).toBe(1);
    // Unsigned: the wallet signs on the device.
    expect(isAllZero(decoded.signatures[0])).toBe(true);
  });

  it("leads with AdvanceNonceAccount as the first instruction", () => {
    const first = decoded.instructions[0];
    expect(first.programId).toBe(SYSTEM_PROGRAM_B58);
    // SystemInstruction discriminant 4 = AdvanceNonceAccount, u32 LE, no data.
    expect(Array.from(first.data)).toEqual([4, 0, 0, 0]);
  });

  it("points the advance instruction at the nonce account and authority", () => {
    const first = decoded.instructions[0];
    // accounts: [nonce account, RecentBlockhashes sysvar, nonce authority].
    const nonceAcct = decoded.accountKeys[first.accountIndices[0]];
    const authority = decoded.accountKeys[first.accountIndices[2]];
    expect(nonceAcct).toBe(NONCE_ACCOUNT);
    expect(authority).toBe(WALLET);
  });

  it("pins the message lifetime to the nonce value, not a blockhash", () => {
    expect(bs58.encode(decoded.recentBlockhash)).toBe(NONCE_VALUE);
  });

  it("carries the SOL transfer after the advance instruction", () => {
    const transfer = decoded.instructions[1];
    expect(transfer.programId).toBe(SYSTEM_PROGRAM_B58);
    // discriminant 2 = Transfer.
    expect(transfer.data[0]).toBe(2);
    // fee payer / source is the wallet (account index 0).
    expect(decoded.accountKeys[0]).toBe(WALLET);
  });

  it("does not modify the caller's amount", () => {
    const transfer = decoded.instructions[1];
    // lamports are the trailing u64 LE after the 4-byte discriminant.
    const view = new DataView(
      transfer.data.buffer,
      transfer.data.byteOffset + 4,
      8
    );
    expect(view.getBigUint64(0, true)).toBe(1_000_000_000n);
  });
});

describe("buildCreateNonceAccountTx", () => {
  const recentBlockhash = {
    blockhash: NONCE_VALUE as never,
    lastValidBlockHeight: 123n,
  };

  it("builds a two-signer create+initialize tx, pre-signing only the nonce keypair", async () => {
    const nonceSigner = await generateKeyPairSigner();
    const { txBase64, nonceAccountAddress } = await buildCreateNonceAccountTx({
      payer: WALLET,
      rentLamports: 1_447_680n,
      recentBlockhash,
      nonceSigner,
    });

    expect(nonceAccountAddress).toBe(nonceSigner.address);

    const decoded = decodeLegacyTx(txBase64);
    // Two signers: wallet (fee payer) + nonce account keypair.
    expect(decoded.signatures.length).toBe(2);

    // Account index 0 is the fee payer (wallet) — unsigned here, signed on device.
    const walletIdx = decoded.accountKeys.indexOf(WALLET);
    expect(walletIdx).toBe(0);
    expect(isAllZero(decoded.signatures[0])).toBe(true);

    // The nonce-account signer slot is pre-filled by us.
    const nonceIdx = decoded.accountKeys.indexOf(nonceSigner.address);
    expect(nonceIdx).toBeGreaterThanOrEqual(0);
    expect(isAllZero(decoded.signatures[nonceIdx])).toBe(false);
  });

  it("emits CreateAccount then InitializeNonceAccount", async () => {
    const nonceSigner = await generateKeyPairSigner();
    const { txBase64 } = await buildCreateNonceAccountTx({
      payer: WALLET,
      rentLamports: 1_447_680n,
      recentBlockhash,
      nonceSigner,
    });
    const decoded = decodeLegacyTx(txBase64);

    expect(decoded.instructions.length).toBe(2);
    expect(decoded.instructions[0].programId).toBe(SYSTEM_PROGRAM_B58);
    // CreateAccount discriminant 0.
    expect(decoded.instructions[0].data[0]).toBe(0);
    // InitializeNonceAccount discriminant 6.
    expect(decoded.instructions[1].programId).toBe(SYSTEM_PROGRAM_B58);
    expect(decoded.instructions[1].data[0]).toBe(6);
  });
});
