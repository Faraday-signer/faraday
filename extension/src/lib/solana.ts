import bs58 from "bs58";

const BASE58_RE = /^[1-9A-HJ-NP-Za-km-z]+$/;

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
  if (trimmed.length < 32 || trimmed.length > 44) {
    return false;
  }
  if (!BASE58_RE.test(trimmed)) {
    return false;
  }

  try {
    const decoded = bs58.decode(trimmed);
    return decoded.length === 32;
  } catch {
    return false;
  }
}

export function pubkeyToBytes(value: string): Uint8Array {
  const decoded = bs58.decode(value);
  if (decoded.length !== 32) {
    throw new Error("Invalid Solana pubkey length.");
  }
  return decoded;
}

function readShortVec(data: Uint8Array, offset: number): ShortVec {
  let value = 0;
  let size = 0;
  let shift = 0;

  while (true) {
    if (offset + size >= data.length) {
      throw new Error("Invalid shortvec encoding.");
    }

    const byte = data[offset + size];
    value |= (byte & 0x7f) << shift;
    size += 1;

    if ((byte & 0x80) === 0) {
      break;
    }

    shift += 7;
  }

  return {
    value,
    bytesRead: size
  };
}

function parseSignerAddresses(messageBytes: Uint8Array): string[] {
  if (messageBytes.length < 4) {
    throw new Error("Message too short.");
  }

  const isVersioned = (messageBytes[0] & 0x80) !== 0;
  const headerOffset = isVersioned ? 1 : 0;
  const headerEnd = headerOffset + 3;

  if (messageBytes.length < headerEnd) {
    throw new Error("Missing message header.");
  }

  const requiredSignatures = messageBytes[headerOffset];
  const keyCountInfo = readShortVec(messageBytes, headerEnd);
  const keyCount = keyCountInfo.value;
  const keyStart = headerEnd + keyCountInfo.bytesRead;
  const keyEnd = keyStart + keyCount * 32;

  if (keyEnd > messageBytes.length) {
    throw new Error("Invalid account key list in message.");
  }

  if (requiredSignatures > keyCount) {
    throw new Error("Invalid signer count in message header.");
  }

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

  if (signaturesEnd > txBytes.length) {
    throw new Error("Transaction signatures are malformed.");
  }

  const signatures: Uint8Array[] = [];
  for (let i = 0; i < signatureCount; i += 1) {
    const start = signaturesStart + i * 64;
    signatures.push(txBytes.slice(start, start + 64));
  }

  const messageBytes = txBytes.slice(signaturesEnd);
  if (messageBytes.length === 0) {
    throw new Error("Transaction message missing.");
  }

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
  if (parsed.signatureCount === 0) {
    throw new Error("Transaction has no signer slots.");
  }

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
