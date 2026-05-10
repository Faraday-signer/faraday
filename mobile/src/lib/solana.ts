import bs58 from "bs58";
import nacl from "tweetnacl";

const BASE58_RE = /^[1-9A-HJ-NP-Za-km-z]+$/;
const SIGN_MESSAGE_QR_PREFIX = 0xff;
const MIN_SIGN_MESSAGE_QR_BASE64_LENGTH = 51;

export const FARADAY_SIG_PREFIX = "faraday:sig:";
const FARADAY_SIG_ENVELOPE_VERSION = 1;
const FARADAY_SIG_ENVELOPE_LENGTH = 1 + 32 + 64;

export const SUPPORTED_SOLANA_CHAINS = [
  "solana:mainnet",
  "solana:devnet",
  "solana:testnet"
] as const;

interface ShortVec {
  value: number;
  bytesRead: number;
}

interface TxEnvelope {
  signatureCount: number;
  signatures: Uint8Array[];
  messageBytes: Uint8Array;
  signerAddresses: string[];
}

export function encodeBase64(data: Uint8Array): string {
  return btoa(String.fromCharCode(...data));
}

export function decodeBase64(data: string): Uint8Array {
  const bin = atob(data);
  const out = new Uint8Array(bin.length);
  for (let i = 0; i < bin.length; i += 1) {
    out[i] = bin.charCodeAt(i);
  }
  return out;
}

export function isValidSolanaAddress(value: string): boolean {
  const trimmed = value.trim();
  if (trimmed.length < 32 || trimmed.length > 44) return false;
  if (!BASE58_RE.test(trimmed)) return false;
  try {
    const decoded = bs58.decode(trimmed);
    return decoded.length === 32;
  } catch {
    return false;
  }
}

export function pubkeyToBytes(value: string): Uint8Array {
  const decoded = bs58.decode(value);
  if (decoded.length !== 32) throw new Error("Invalid Solana pubkey length.");
  return decoded;
}

export function buildSignMessageQrPayload(messageBytes: Uint8Array): string {
  if (messageBytes.length === 0) throw new Error("Message must not be empty.");
  const payload = new Uint8Array(messageBytes.length + 1);
  payload[0] = SIGN_MESSAGE_QR_PREFIX;
  payload.set(messageBytes, 1);
  const encoded = encodeBase64(payload);
  if (encoded.length < MIN_SIGN_MESSAGE_QR_BASE64_LENGTH) {
    throw new Error(
      "Message is too short for current Faraday QR sign-message scan. Use at least 36 bytes."
    );
  }
  return encoded;
}

export function decodeHexSignature(signatureHex: string): Uint8Array {
  const normalized = signatureHex.trim().replace(/^0x/i, "");
  if (!/^[0-9a-fA-F]{128}$/.test(normalized)) {
    throw new Error("Signed payload is not a valid 64-byte hex signature.");
  }
  const out = new Uint8Array(64);
  for (let i = 0; i < 64; i += 1) {
    const offset = i * 2;
    out[i] = Number.parseInt(normalized.slice(offset, offset + 2), 16);
  }
  return out;
}

export function validateSignedMessage(
  messageBytes: Uint8Array,
  signatureHex: string,
  expectedSigner: string
): Uint8Array {
  const signatureBytes = decodeHexSignature(signatureHex);
  const pubkey = pubkeyToBytes(expectedSigner);
  const verified = nacl.sign.detached.verify(messageBytes, signatureBytes, pubkey);
  if (!verified) {
    throw new Error("Message signature does not match the paired signer account.");
  }
  return signatureBytes;
}

function readShortVec(data: Uint8Array, offset: number): ShortVec {
  let value = 0;
  let size = 0;
  let shift = 0;

  while (true) {
    if (offset + size >= data.length) throw new Error("Invalid shortvec encoding.");
    const byte = data[offset + size];
    value |= (byte & 0x7f) << shift;
    size += 1;
    if ((byte & 0x80) === 0) break;
    shift += 7;
  }

  return { value, bytesRead: size };
}

function parseSignerAddresses(messageBytes: Uint8Array): string[] {
  if (messageBytes.length < 4) throw new Error("Message too short.");

  const isVersioned = (messageBytes[0] & 0x80) !== 0;
  const headerOffset = isVersioned ? 1 : 0;
  const headerEnd = headerOffset + 3;

  if (messageBytes.length < headerEnd) throw new Error("Missing message header.");

  const requiredSignatures = messageBytes[headerOffset];
  const keyCountInfo = readShortVec(messageBytes, headerEnd);
  const keyCount = keyCountInfo.value;
  const keyStart = headerEnd + keyCountInfo.bytesRead;
  const keyEnd = keyStart + keyCount * 32;

  if (keyEnd > messageBytes.length) throw new Error("Invalid account key list in message.");
  if (requiredSignatures > keyCount) throw new Error("Invalid signer count in message header.");

  const signers: string[] = [];
  for (let i = 0; i < requiredSignatures; i += 1) {
    const start = keyStart + i * 32;
    const end = start + 32;
    signers.push(bs58.encode(messageBytes.slice(start, end)));
  }
  return signers;
}

function parseEnvelope(txBytes: Uint8Array): TxEnvelope {
  const sigCountInfo = readShortVec(txBytes, 0);
  const signatureCount = sigCountInfo.value;
  const signaturesStart = sigCountInfo.bytesRead;
  const signaturesEnd = signaturesStart + signatureCount * 64;

  if (signaturesEnd > txBytes.length) throw new Error("Transaction signatures are malformed.");

  const signatures: Uint8Array[] = [];
  for (let i = 0; i < signatureCount; i += 1) {
    const start = signaturesStart + i * 64;
    signatures.push(txBytes.slice(start, start + 64));
  }

  const messageBytes = txBytes.slice(signaturesEnd);
  if (messageBytes.length === 0) throw new Error("Transaction message missing.");

  return {
    signatureCount,
    signatures,
    messageBytes,
    signerAddresses: parseSignerAddresses(messageBytes)
  };
}

export function validateUnsignedTransactionPayload(
  unsignedTxBase64: string,
  expectedSigner?: string
): void {
  let unsignedBytes: Uint8Array;
  try {
    unsignedBytes = decodeBase64(unsignedTxBase64.trim());
  } catch {
    throw new Error("Transaction payload is not valid base64.");
  }
  const parsed = parseEnvelope(unsignedBytes);
  if (parsed.signatureCount === 0) throw new Error("Transaction has no signer slots.");
  if (expectedSigner && !parsed.signerAddresses.includes(expectedSigner)) {
    throw new Error("Transaction does not include the paired signer account.");
  }
}

function allZeroSignature(signature: Uint8Array): boolean {
  return signature.every((byte) => byte === 0);
}

export function validateSignedTransactionMatch(
  unsignedTxBase64: string,
  signedTxBase64: string,
  expectedSigner: string
): Uint8Array {
  let unsignedBytes: Uint8Array;
  let signedBytes: Uint8Array;

  try {
    unsignedBytes = decodeBase64(unsignedTxBase64.trim());
    signedBytes = decodeBase64(signedTxBase64.trim());
  } catch {
    throw new Error("Signed payload is not valid base64.");
  }

  const unsignedTx = parseEnvelope(unsignedBytes);
  const signedTx = parseEnvelope(signedBytes);

  if (unsignedTx.signatureCount !== signedTx.signatureCount) {
    throw new Error("Signature count mismatch between unsigned and signed transactions.");
  }
  if (unsignedTx.messageBytes.length !== signedTx.messageBytes.length) {
    throw new Error("Message length mismatch between unsigned and signed transactions.");
  }
  for (let i = 0; i < unsignedTx.messageBytes.length; i += 1) {
    if (unsignedTx.messageBytes[i] !== signedTx.messageBytes[i]) {
      throw new Error("Signed transaction message does not match unsigned request.");
    }
  }

  const signerIndex = signedTx.signerAddresses.indexOf(expectedSigner);
  if (signerIndex < 0) {
    throw new Error("Signed transaction does not include the paired signer account.");
  }

  const signerSig = signedTx.signatures[signerIndex];
  if (!signerSig || allZeroSignature(signerSig)) {
    throw new Error("Expected signer signature is missing.");
  }

  return signedBytes;
}

export function parseFaradaySigEnvelope(text: string): {
  pubkey: Uint8Array;
  signature: Uint8Array;
} {
  if (!text.startsWith(FARADAY_SIG_PREFIX)) {
    throw new Error("Missing faraday:sig: envelope prefix.");
  }
  const body = text.slice(FARADAY_SIG_PREFIX.length).trim();
  let bytes: Uint8Array;
  try {
    bytes = decodeBase64(body);
  } catch {
    throw new Error("Envelope payload is not valid base64.");
  }
  if (bytes.length !== FARADAY_SIG_ENVELOPE_LENGTH) {
    throw new Error(
      `Envelope payload length ${bytes.length} (expected ${FARADAY_SIG_ENVELOPE_LENGTH}).`
    );
  }
  if (bytes[0] !== FARADAY_SIG_ENVELOPE_VERSION) {
    throw new Error(`Unsupported envelope version: ${bytes[0]}.`);
  }
  return { pubkey: bytes.slice(1, 33), signature: bytes.slice(33, 97) };
}

export function spliceFaradaySignature(
  unsignedTxBase64: string,
  envelopeText: string,
  expectedSigner: string
): string {
  const { pubkey, signature } = parseFaradaySigEnvelope(envelopeText);

  const expectedBytes = pubkeyToBytes(expectedSigner);
  for (let i = 0; i < 32; i += 1) {
    if (pubkey[i] !== expectedBytes[i]) {
      throw new Error("Envelope signer does not match the paired account.");
    }
  }

  let unsignedBytes: Uint8Array;
  try {
    unsignedBytes = decodeBase64(unsignedTxBase64.trim());
  } catch {
    throw new Error("Unsigned transaction payload is not valid base64.");
  }

  const parsed = parseEnvelope(unsignedBytes);
  const signerIndex = parsed.signerAddresses.indexOf(expectedSigner);
  if (signerIndex < 0) {
    throw new Error("Transaction does not include the paired signer account.");
  }

  const verified = nacl.sign.detached.verify(parsed.messageBytes, signature, pubkey);
  if (!verified) {
    throw new Error("Envelope signature does not verify against transaction message.");
  }

  const sigCountInfo = readShortVec(unsignedBytes, 0);
  const slotStart = sigCountInfo.bytesRead + signerIndex * 64;

  const spliced = new Uint8Array(unsignedBytes.length);
  spliced.set(unsignedBytes);
  spliced.set(signature, slotStart);

  return encodeBase64(spliced);
}
