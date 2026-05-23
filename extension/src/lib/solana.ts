import bs58 from "bs58";
import nacl from "tweetnacl";

const BASE58_RE = /^[1-9A-HJ-NP-Za-km-z]+$/;
const SIGN_MESSAGE_QR_PREFIX = 0xff;
const MIN_SIGN_MESSAGE_QR_BASE64_LENGTH = 51;
const SOLANA_OFFCHAIN_DOMAIN = new Uint8Array([
  0xff, 0x73, 0x6f, 0x6c, 0x61, 0x6e, 0x61, 0x20,
  0x6f, 0x66, 0x66, 0x63, 0x68, 0x61, 0x69, 0x6e
]);
const SOLANA_OFFCHAIN_HEADER_LENGTH = 20;

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

export interface SolanaOffchainMessage {
  version: number;
  format: number;
  bodyBytes: Uint8Array;
  bodyText: string;
}

export interface SignMessagePreview {
  title: string;
  text: string;
  wrapped: boolean;
  ika?: IkaApprovalDetails;
}

export type IkaAction = "propose" | "approve" | "cancel";

export type IkaContent =
  | { kind: "transfer"; amountLamports: bigint; to: string }
  | { kind: "spl-transfer"; amount: string; mint: string; to: string }
  | { kind: "add-intent"; definitionHash: string }
  | { kind: "remove-intent"; index: string }
  | { kind: "update-intent"; index: string; definitionHash: string }
  | { kind: "other"; text: string };

export interface IkaApprovalDetails {
  action: IkaAction;
  expires: string;
  walletName: string;
  proposalIndex: string;
  content: IkaContent;
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

export function buildSignMessageQrPayload(messageBytes: Uint8Array): string {
  if (messageBytes.length === 0) {
    throw new Error("Message must not be empty.");
  }

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

export function decodeSignMessageQrPayload(qrBase64: string): Uint8Array {
  const payload = decodeBase64(qrBase64.trim());
  if (payload[0] !== SIGN_MESSAGE_QR_PREFIX) {
    throw new Error("Sign-message QR payload is missing the 0xFF prefix.");
  }
  return payload.slice(1);
}

export function buildSolanaOffchainMessage(messageText: string): Uint8Array {
  const body = new TextEncoder().encode(messageText);
  if (body.length === 0) {
    throw new Error("Solana off-chain message must not be empty.");
  }
  if (body.length > 0xffff) {
    throw new Error("Solana off-chain message is too long.");
  }

  const out = new Uint8Array(SOLANA_OFFCHAIN_HEADER_LENGTH + body.length);
  out.set(SOLANA_OFFCHAIN_DOMAIN, 0);
  out[16] = 0;
  out[17] = 0;
  out[18] = body.length & 0xff;
  out[19] = body.length >> 8;
  out.set(body, SOLANA_OFFCHAIN_HEADER_LENGTH);
  return out;
}

export function parseSolanaOffchainMessage(
  messageBytes: Uint8Array
): SolanaOffchainMessage | null {
  if (messageBytes.length < SOLANA_OFFCHAIN_HEADER_LENGTH) {
    return null;
  }
  for (let i = 0; i < SOLANA_OFFCHAIN_DOMAIN.length; i += 1) {
    if (messageBytes[i] !== SOLANA_OFFCHAIN_DOMAIN[i]) {
      return null;
    }
  }
  // clear-msig-ika and the Solana off-chain spec both pin version=0,
  // format=0 (restricted ASCII). Reject anything else so a future spec
  // bump can't slip past the preview.
  if (messageBytes[16] !== 0 || messageBytes[17] !== 0) {
    return null;
  }

  const bodyLength = messageBytes[18] | (messageBytes[19] << 8);
  const expectedLength = SOLANA_OFFCHAIN_HEADER_LENGTH + bodyLength;
  if (messageBytes.length !== expectedLength) {
    throw new Error("Solana off-chain message length does not match the header.");
  }

  const bodyBytes = messageBytes.slice(SOLANA_OFFCHAIN_HEADER_LENGTH);
  let bodyText: string;
  try {
    bodyText = new TextDecoder("utf-8", { fatal: true }).decode(bodyBytes);
  } catch {
    throw new Error("Solana off-chain message body is not valid UTF-8.");
  }

  return {
    version: messageBytes[16],
    format: messageBytes[17],
    bodyBytes,
    bodyText
  };
}

/**
 * Recognize a clear-msig-ika approval message body. Master shape (verified
 * against `programs/clear-wallet/src/utils/message.rs:109-121` in
 * `Iamknownasfesal/clear-msig-ika`):
 *
 *   expires <YYYY-MM-DD HH:MM:SS>: <action> <content> | wallet: <name> proposal: <idx>
 *
 * `<action>` ∈ {propose, approve, cancel}. Returns `null` when the body
 * doesn't carry the Ika trailer — callers fall back to the raw body text.
 */
export function parseIkaApprovalMessage(bodyText: string): IkaApprovalDetails | null {
  if (!bodyText.startsWith("expires ")) {
    return null;
  }
  const afterExpires = bodyText.slice("expires ".length);
  const colonIdx = afterExpires.indexOf(": ");
  if (colonIdx < 0) {
    return null;
  }
  const expires = afterExpires.slice(0, colonIdx);
  const rest = afterExpires.slice(colonIdx + 2);

  const pipeIdx = rest.indexOf(" | ");
  if (pipeIdx < 0) {
    return null;
  }
  const actionAndContent = rest.slice(0, pipeIdx);
  const metadata = rest.slice(pipeIdx + 3);

  const spaceIdx = actionAndContent.indexOf(" ");
  if (spaceIdx < 0) {
    return null;
  }
  const action = actionAndContent.slice(0, spaceIdx);
  const content = actionAndContent.slice(spaceIdx + 1);
  if (action !== "propose" && action !== "approve" && action !== "cancel") {
    return null;
  }

  if (!metadata.startsWith("wallet: ")) {
    return null;
  }
  const walletAndProposal = metadata.slice("wallet: ".length);
  const proposalIdx = walletAndProposal.indexOf(" proposal: ");
  if (proposalIdx < 0) {
    return null;
  }
  const walletName = walletAndProposal.slice(0, proposalIdx);
  const proposalIndex = walletAndProposal.slice(proposalIdx + " proposal: ".length);

  return {
    action: action as IkaAction,
    expires,
    walletName,
    proposalIndex,
    content: classifyIkaContent(content)
  };
}

function classifyIkaContent(content: string): IkaContent {
  if (content.startsWith("transfer ")) {
    const rest = content.slice("transfer ".length);
    const lamportsIdx = rest.indexOf(" lamports to ");
    if (lamportsIdx >= 0) {
      const amountStr = rest.slice(0, lamportsIdx);
      const to = rest.slice(lamportsIdx + " lamports to ".length);
      try {
        return { kind: "transfer", amountLamports: BigInt(amountStr), to };
      } catch {
        // Unparseable amount — fall through to `other` so the user still sees text.
      }
    }
    const ofMintIdx = rest.indexOf(" of mint ");
    if (ofMintIdx >= 0) {
      const amount = rest.slice(0, ofMintIdx);
      const afterMint = rest.slice(ofMintIdx + " of mint ".length);
      const toIdx = afterMint.indexOf(" to ");
      if (toIdx >= 0) {
        return {
          kind: "spl-transfer",
          amount,
          mint: afterMint.slice(0, toIdx),
          to: afterMint.slice(toIdx + " to ".length)
        };
      }
    }
  }

  if (content.startsWith("add intent definition_hash: ")) {
    return {
      kind: "add-intent",
      definitionHash: content.slice("add intent definition_hash: ".length)
    };
  }
  if (content.startsWith("remove intent ")) {
    return { kind: "remove-intent", index: content.slice("remove intent ".length) };
  }
  if (content.startsWith("update intent ")) {
    const rest = content.slice("update intent ".length);
    const hashIdx = rest.indexOf(" definition_hash: ");
    if (hashIdx >= 0) {
      return {
        kind: "update-intent",
        index: rest.slice(0, hashIdx),
        definitionHash: rest.slice(hashIdx + " definition_hash: ".length)
      };
    }
  }

  return { kind: "other", text: content };
}

export function describeSignMessageBytes(messageBytes: Uint8Array): SignMessagePreview {
  const offchain = parseSolanaOffchainMessage(messageBytes);
  if (offchain) {
    const ika = parseIkaApprovalMessage(offchain.bodyText);
    return {
      title: "Solana off-chain message",
      text: offchain.bodyText,
      wrapped: true,
      ...(ika ? { ika } : {})
    };
  }

  try {
    return {
      title: "Message",
      text: new TextDecoder("utf-8", { fatal: true }).decode(messageBytes),
      wrapped: false
    };
  } catch {
    return {
      title: "Message",
      text: "(binary data)",
      wrapped: false
    };
  }
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

export interface SiwsInput {
  domain?: string;
  address?: string;
  statement?: string;
  uri?: string;
  version?: string;
  chainId?: string;
  nonce?: string;
  issuedAt?: string;
  expirationTime?: string;
  notBefore?: string;
  requestId?: string;
  resources?: readonly string[];
}

export interface SiwsResolved {
  domain: string;
  address: string;
  statement?: string;
  uri?: string;
  version?: string;
  chainId?: string;
  nonce?: string;
  issuedAt?: string;
  expirationTime?: string;
  notBefore?: string;
  requestId?: string;
  resources?: readonly string[];
}

/**
 * Build a Sign-In With Solana message per the Wallet-Standard / Phantom
 * SIWS spec. Mirrors EIP-4361 field ordering. Fields are emitted in the
 * spec's fixed order so two wallets producing the same input yield the
 * same bytes — dapps rely on that for server-side verification.
 *
 * `domain` and `address` are required by the output format; every other
 * field is optional and omitted (along with its whole section) when absent.
 */
export function formatSiwsMessage(input: SiwsResolved): string {
  if (!input.domain) {
    throw new Error("SIWS message requires a domain.");
  }
  if (!input.address) {
    throw new Error("SIWS message requires an address.");
  }

  const lines: string[] = [];
  lines.push(`${input.domain} wants you to sign in with your Solana account:`);
  lines.push(input.address);

  if (input.statement !== undefined) {
    lines.push("");
    lines.push(input.statement);
  }

  const fields: Array<[string, string | undefined]> = [
    ["URI", input.uri],
    ["Version", input.version],
    ["Chain ID", input.chainId],
    ["Nonce", input.nonce],
    ["Issued At", input.issuedAt],
    ["Expiration Time", input.expirationTime],
    ["Not Before", input.notBefore],
    ["Request ID", input.requestId]
  ];
  const presentFields = fields.filter(([, v]) => v !== undefined && v !== "");
  const hasResources = input.resources && input.resources.length > 0;

  if (presentFields.length > 0 || hasResources) {
    lines.push("");
    for (const [key, value] of presentFields) {
      lines.push(`${key}: ${value}`);
    }
    if (hasResources) {
      lines.push("Resources:");
      for (const resource of input.resources!) {
        lines.push(`- ${resource}`);
      }
    }
  }

  return lines.join("\n");
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
  return {
    pubkey: bytes.slice(1, 33),
    signature: bytes.slice(33, 97)
  };
}

/**
 * Reconstruct a full signed transaction from the compact `faraday:sig:`
 * envelope the Pi emits on the return leg. The extension already has the
 * unsigned tx in session state, so the Pi only ships the 1+32+64 byte
 * (version + pubkey + signature) payload — we splice the signature into
 * the matching signer slot, ed25519-verify it against the message bytes,
 * and return the normal base64-encoded signed tx. Downstream validation
 * (`validateSignedTransactionMatch`) still runs unchanged.
 */
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
