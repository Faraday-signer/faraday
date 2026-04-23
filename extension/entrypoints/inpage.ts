import bs58 from "bs58";
import { registerWallet } from "@wallet-standard/wallet";

import {
  decodeHexSignature,
  decodeBase64,
  encodeBase64,
  formatSiwsMessage,
  isValidSolanaAddress,
  SUPPORTED_SOLANA_CHAINS
} from "../src/lib/solana";
import type { SiwsInput } from "../src/lib/solana";
import type {
  ConnectCheckResult,
  CreateSignSessionResult,
  GetSignResult,
  RuntimeRequest,
  RuntimeResponse
} from "../src/lib/types";

const LOG_PREFIX = "[Faraday][inpage]";
const BRIDGE_TIMEOUT_MS = 15_000;

function debug(message: string, meta?: unknown): void {
  if (meta === undefined) {
    console.debug(`${LOG_PREFIX} ${message}`);
    return;
  }
  console.debug(`${LOG_PREFIX} ${message}`, meta);
}

function warn(message: string, meta?: unknown): void {
  if (meta === undefined) {
    console.warn(`${LOG_PREFIX} ${message}`);
    return;
  }
  console.warn(`${LOG_PREFIX} ${message}`, meta);
}

function messageTypeOf(message: RuntimeRequest | unknown): string {
  if (!message || typeof message !== "object") {
    return "<invalid>";
  }
  const maybe = message as { type?: unknown };
  return typeof maybe.type === "string" ? maybe.type : "<missing>";
}

const ICON_SVG =
  "data:image/svg+xml," +
  encodeURIComponent(
    '<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 64 64">' +
      '<rect width="64" height="64" rx="16" fill="#0f172a"/>' +
      '<path d="M16 22h32l-5 6H11z" fill="#22d3ee"/>' +
      '<path d="M16 32h32l-5 6H11z" fill="#a3e635"/>' +
      '<path d="M16 42h32l-5 6H11z" fill="#f59e0b"/>' +
    "</svg>"
  );

type BridgeRequestMessage = {
  target: "faraday:content";
  id: string;
  message: RuntimeRequest;
};

type BridgeResponseMessage = {
  target: "faraday:inpage";
  id: string;
  response: RuntimeResponse;
};

function callBackground<T = unknown>(message: RuntimeRequest): Promise<RuntimeResponse<T>> {
  const requestId = `${Date.now()}-${Math.random().toString(16).slice(2)}`;
  const type = messageTypeOf(message);

  return new Promise((resolve) => {
    let settled = false;
    const cleanup = () => {
      window.removeEventListener("message", onMessage);
      window.clearTimeout(timeoutId);
    };

    const onMessage = (event: MessageEvent) => {
      if (event.source !== window) {
        return;
      }

      const payload = event.data as BridgeResponseMessage;
      if (!payload || payload.target !== "faraday:inpage" || payload.id !== requestId) {
        return;
      }

      settled = true;
      cleanup();
      if (!payload.response.ok) {
        warn("Background returned error", {
          type,
          requestId,
          error: payload.response.error
        });
      }
      resolve(payload.response as RuntimeResponse<T>);
    };

    const timeoutId = window.setTimeout(() => {
      if (settled) {
        return;
      }

      cleanup();
      warn("Background bridge timeout", {
        type,
        requestId,
        timeoutMs: BRIDGE_TIMEOUT_MS
      });
      resolve({
        ok: false,
        error: `Background request timed out: ${type}`
      });
    }, BRIDGE_TIMEOUT_MS);

    window.addEventListener("message", onMessage);

    const payload: BridgeRequestMessage = {
      target: "faraday:content",
      id: requestId,
      message
    };
    debug("Dispatching bridge request", { type, requestId });
    window.postMessage(payload, "*");
  });
}

function toUint8Array(value: unknown): Uint8Array {
  if (value instanceof Uint8Array) {
    return value;
  }

  if (value instanceof ArrayBuffer) {
    return new Uint8Array(value);
  }

  if (ArrayBuffer.isView(value)) {
    return new Uint8Array(value.buffer, value.byteOffset, value.byteLength);
  }

  if (Array.isArray(value) && value.every((item) => Number.isInteger(item))) {
    return Uint8Array.from(value as number[]);
  }

  if (value && typeof value === "object") {
    const maybeTx = value as {
      serialize?: (config?: { requireAllSignatures?: boolean; verifySignatures?: boolean }) => Uint8Array;
    };
    if (typeof maybeTx.serialize === "function") {
      try {
        return maybeTx.serialize({ requireAllSignatures: false, verifySignatures: false });
      } catch {
        return maybeTx.serialize();
      }
    }
  }

  throw new Error("Unsupported transaction type. Expected serialized bytes.");
}

function shortAddress(address: string): string {
  if (address.length <= 12) {
    return address;
  }
  return `${address.slice(0, 4)}...${address.slice(-4)}`;
}

type ChangeListener = (properties: { accounts?: readonly WalletAccount[] }) => void;

interface WalletAccount {
  address: string;
  publicKey: Uint8Array;
  chains: readonly string[];
  features: readonly string[];
  label: string;
  icon: string;
}

class FaradayWallet {
  readonly version = "1.0.0";
  readonly name = "Faraday";
  readonly icon = ICON_SVG;
  readonly chains = SUPPORTED_SOLANA_CHAINS;

  private connected = false;
  private account: WalletAccount | null = null;
  private listeners = new Set<ChangeListener>();

  get accounts(): readonly WalletAccount[] {
    return this.connected && this.account ? [this.account] : [];
  }

  get features() {
    return {
      "standard:connect": {
        version: "1.0.0",
        connect: this.connect
      },
      "standard:disconnect": {
        version: "1.0.0",
        disconnect: this.disconnect
      },
      "standard:events": {
        version: "1.0.0",
        on: this.on
      },
      "solana:signTransaction": {
        version: "1.0.0",
        supportedTransactionVersions: ["legacy", 0],
        signTransaction: this.signTransaction
      },
      "solana:signMessage": {
        version: "1.0.0",
        signMessage: this.signMessage
      },
      "solana:signIn": {
        version: "1.0.0",
        signIn: this.signIn
      }
    };
  }

  private notifyChange(): void {
    const payload = { accounts: this.accounts };
    for (const listener of this.listeners) {
      listener(payload);
    }
  }

  private makeAccount(pubkey: string): WalletAccount {
    return {
      address: pubkey,
      publicKey: bs58.decode(pubkey),
      chains: SUPPORTED_SOLANA_CHAINS,
      features: ["solana:signTransaction", "solana:signMessage", "solana:signIn"],
      label: "Faraday",
      icon: ICON_SVG
    };
  }

  private readonly on = (event: string, listener: ChangeListener) => {
    if (event !== "change") {
      return () => {};
    }

    this.listeners.add(listener);
    return () => {
      this.listeners.delete(listener);
    };
  };

  private readonly connect = async () => {
    const origin = window.location.origin;
    const check = await callBackground<ConnectCheckResult>({
      type: "faraday:connect-check",
      origin
    });
    if (!check.ok) {
      warn("Connect preflight failed", { origin, error: check.error });
      throw new Error(check.error);
    }

    const { pairedPubkey, approved } = check.data;

    if (!pairedPubkey || !isValidSolanaAddress(pairedPubkey)) {
      throw new Error("No paired Faraday account. Open the extension popup and pair a pubkey.");
    }

    if (!approved) {
      const accepted = window.confirm(
        `Allow ${origin} to connect to Faraday account ${shortAddress(pairedPubkey)}?`
      );
      if (!accepted) {
        warn("User rejected origin approval", { origin });
        throw new Error("Connection rejected by user.");
      }

      const approve = await callBackground({
        type: "faraday:approve-origin",
        origin
      });
      if (!approve.ok) {
        warn("Failed to approve origin", { origin, error: approve.error });
        throw new Error(approve.error);
      }
    }

    this.connected = true;
    this.account = this.makeAccount(pairedPubkey);
    debug("Wallet connected", { origin, account: pairedPubkey });
    this.notifyChange();

    return {
      accounts: this.accounts
    };
  };

  private readonly disconnect = async () => {
    this.connected = false;
    this.account = null;
    debug("Wallet disconnected");
    this.notifyChange();
  };

  private async waitForSession(sessionId: string): Promise<GetSignResult> {
    const timeoutMs = 5 * 60 * 1000;
    const pollIntervalMs = 500;
    const startedAt = Date.now();

    while (Date.now() - startedAt < timeoutMs) {
      const result = await callBackground<GetSignResult>({
        type: "faraday:get-sign-result",
        sessionId
      });
      if (!result.ok) {
        warn("Polling sign session failed", { sessionId, error: result.error });
        throw new Error(result.error);
      }

      if (result.data.status === "pending") {
        await new Promise((resolve) => setTimeout(resolve, pollIntervalMs));
        continue;
      }

      return result.data;
    }

    await callBackground({
      type: "faraday:cancel-sign-session",
      sessionId,
      reason: "Signing timed out."
    });
    throw new Error("Signing timed out.");
  }

  private readonly signTransaction = async (...inputs: unknown[]) => {
    if (!this.connected || !this.account) {
      throw new Error("Connect the wallet before requesting a signature.");
    }

    debug("signTransaction request received", {
      inputCount: inputs.length,
      account: this.account.address,
      origin: window.location.origin
    });

    const normalized =
      inputs.length === 1 && Array.isArray(inputs[0]) ? (inputs[0] as unknown[]) : inputs;

    if (normalized.length !== 1) {
      throw new Error("MVP supports one transaction per sign request.");
    }

    const first = normalized[0] as { transaction?: unknown } | undefined;
    const txBytes = toUint8Array(first?.transaction ?? first);
    const txBase64 = encodeBase64(txBytes);

    const create = await callBackground<CreateSignSessionResult>({
      type: "faraday:create-sign-session",
      origin: window.location.origin,
      txBase64
    });
    if (!create.ok) {
      warn("Failed creating sign session", { error: create.error });
      throw new Error(create.error);
    }

    debug("Created sign session", {
      sessionId: create.data.sessionId
    });

    const openWindow = await callBackground({
      type: "faraday:open-sign-window",
      origin: window.location.origin,
      sessionId: create.data.sessionId
    });
    if (!openWindow.ok) {
      warn("Background open-sign-window failed", {
        sessionId: create.data.sessionId,
        error: openWindow.error
      });
      throw new Error(
        `${openWindow.error} Reload Faraday in chrome://extensions and try again.`
      );
    }

    const done = await this.waitForSession(create.data.sessionId);
    if (done.kind !== "tx") {
      throw new Error("Unexpected signing session type.");
    }
    if (done.status !== "completed" || !done.signedTxBase64) {
      throw new Error(done.error || "Signing was not completed.");
    }

    return [
      {
        signedTransaction: decodeBase64(done.signedTxBase64)
      }
    ];
  };

  private readonly signMessage = async (...inputs: unknown[]) => {
    if (!this.connected || !this.account) {
      throw new Error("Connect the wallet before requesting a signature.");
    }

    debug("signMessage request received", {
      inputCount: inputs.length,
      account: this.account.address,
      origin: window.location.origin
    });

    const normalized =
      inputs.length === 1 && Array.isArray(inputs[0]) ? (inputs[0] as unknown[]) : inputs;

    if (normalized.length !== 1) {
      throw new Error("MVP supports one message per sign request.");
    }

    const first = normalized[0] as
      | { account?: { address?: unknown }; message?: unknown }
      | undefined;

    // Spec: if the caller specifies an account, the wallet must sign with
    // that account or throw. Faraday has a single paired account, so any
    // mismatch is a hard reject (prevents a dapp silently getting a
    // signature from the wrong signer).
    const requestedAddress = first?.account?.address;
    if (typeof requestedAddress === "string" && requestedAddress !== this.account.address) {
      throw new Error("Requested account does not match the connected Faraday wallet.");
    }

    const messageBytes = toUint8Array(first?.message ?? first);
    const messageBase64 = encodeBase64(messageBytes);

    const create = await callBackground<CreateSignSessionResult>({
      type: "faraday:create-sign-message-session",
      origin: window.location.origin,
      messageBase64
    });
    if (!create.ok) {
      warn("Failed creating sign-message session", { error: create.error });
      throw new Error(create.error);
    }

    const openWindow = await callBackground({
      type: "faraday:open-sign-window",
      origin: window.location.origin,
      sessionId: create.data.sessionId
    });
    if (!openWindow.ok) {
      warn("Background open-sign-window failed for message session", {
        sessionId: create.data.sessionId,
        error: openWindow.error
      });
      throw new Error(
        `${openWindow.error} Reload Faraday in chrome://extensions and try again.`
      );
    }

    const done = await this.waitForSession(create.data.sessionId);
    if (done.kind !== "message") {
      throw new Error("Unexpected signing session type.");
    }
    if (done.status !== "completed" || !done.signatureHex) {
      throw new Error(done.error || "Message signing was not completed.");
    }

    return [
      {
        signedMessage: messageBytes,
        signature: decodeHexSignature(done.signatureHex)
      }
    ];
  };

  /**
   * Sign-In With Solana (SIWS). Builds a spec-compliant message text from
   * the dapp's input (filling in defaults the dapp omitted), then routes
   * it through the same create/open/wait/validate pipeline as signMessage.
   *
   * Anti-phishing: if the dapp specifies `domain`, it MUST match
   * window.location.host — otherwise a malicious origin could request
   * a signature for a *different* site's login challenge. When omitted,
   * we fill in window.location.host ourselves, which is trusted.
   */
  private readonly signIn = async (...inputs: unknown[]) => {
    if (!this.connected || !this.account) {
      throw new Error("Connect the wallet before requesting a sign-in.");
    }

    const normalized =
      inputs.length === 1 && Array.isArray(inputs[0]) ? (inputs[0] as unknown[]) : inputs;
    const rawInput = (normalized[0] ?? {}) as SiwsInput;

    const host = window.location.host;
    const origin = window.location.origin;

    if (rawInput.domain !== undefined && rawInput.domain !== host) {
      throw new Error(
        `SIWS domain "${rawInput.domain}" does not match current host "${host}".`
      );
    }

    if (
      typeof rawInput.address === "string" &&
      rawInput.address.length > 0 &&
      rawInput.address !== this.account.address
    ) {
      throw new Error("Requested SIWS account does not match the connected Faraday wallet.");
    }

    const resolved = {
      domain: rawInput.domain ?? host,
      address: rawInput.address ?? this.account.address,
      statement: rawInput.statement,
      uri: rawInput.uri ?? origin,
      version: rawInput.version ?? "1",
      chainId: rawInput.chainId,
      nonce: rawInput.nonce ?? randomSiwsNonce(),
      issuedAt: rawInput.issuedAt ?? new Date().toISOString(),
      expirationTime: rawInput.expirationTime,
      notBefore: rawInput.notBefore,
      requestId: rawInput.requestId,
      resources: rawInput.resources
    };

    const messageText = formatSiwsMessage(resolved);
    const messageBytes = new TextEncoder().encode(messageText);
    const messageBase64 = encodeBase64(messageBytes);

    debug("signIn request received", {
      domain: resolved.domain,
      account: this.account.address,
      origin
    });

    const create = await callBackground<CreateSignSessionResult>({
      type: "faraday:create-sign-message-session",
      origin,
      messageBase64
    });
    if (!create.ok) {
      warn("Failed creating SIWS session", { error: create.error });
      throw new Error(create.error);
    }

    const openWindow = await callBackground({
      type: "faraday:open-sign-window",
      origin,
      sessionId: create.data.sessionId
    });
    if (!openWindow.ok) {
      warn("Background open-sign-window failed for SIWS session", {
        sessionId: create.data.sessionId,
        error: openWindow.error
      });
      throw new Error(
        `${openWindow.error} Reload Faraday in chrome://extensions and try again.`
      );
    }

    const done = await this.waitForSession(create.data.sessionId);
    if (done.kind !== "message") {
      throw new Error("Unexpected signing session type.");
    }
    if (done.status !== "completed" || !done.signatureHex) {
      throw new Error(done.error || "SIWS signing was not completed.");
    }

    return [
      {
        account: this.account,
        signedMessage: messageBytes,
        signature: decodeHexSignature(done.signatureHex),
        signatureType: "ed25519" as const
      }
    ];
  };
}

/**
 * Generate a URL-safe base64-ish nonce for SIWS. Uses the page's crypto
 * source; falls back to `Math.random` only if the environment is missing
 * `crypto.getRandomValues`, which no modern browser is.
 */
function randomSiwsNonce(): string {
  const bytes = new Uint8Array(16);
  if (typeof crypto !== "undefined" && typeof crypto.getRandomValues === "function") {
    crypto.getRandomValues(bytes);
  } else {
    for (let i = 0; i < bytes.length; i += 1) {
      bytes[i] = Math.floor(Math.random() * 256);
    }
  }
  return Array.from(bytes)
    .map((b) => b.toString(16).padStart(2, "0"))
    .join("");
}

export default defineUnlistedScript(() => {
  const wallet = new FaradayWallet();
  registerWallet(wallet as never);
});
