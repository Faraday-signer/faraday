import {
  approveOrigin,
  clearApprovedOrigins,
  clearPairedPubkey,
  getExtensionState,
  revokeOrigin,
  setPairedPubkey
} from "@/lib/storage";
import {
  buildSignMessageQrPayload,
  decodeBase64,
  isValidSolanaAddress,
  validateSignedMessage,
  validateSignedTransactionMatch,
  validateUnsignedTransactionPayload
} from "@/lib/solana";
import type {
  ConnectCheckResult,
  CreateSignSessionResult,
  GetSignResult,
  GetSignSessionResult,
  RuntimeRequest,
  RuntimeResponse,
  SignSession
} from "@/lib/types";

const SESSION_TTL_MS = 5 * 60 * 1000;
const LOG_PREFIX = "[Faraday][background]";

/**
 * Sign-session store. Writes land in `chrome.storage.session` so sessions
 * survive MV3 service-worker eviction (they don't survive browser restart,
 * which is what we want — a 5-min TTL session shouldn't outlive the browser).
 * A process-local Map acts as a fast cache; on the first access after a
 * wake-up we rehydrate it from storage.
 *
 * `chrome.storage.session` (not `.local`) is deliberate: session payloads
 * can contain message or tx bytes the dapp sent us and we don't want them
 * persisted to disk.
 */
const STORAGE_KEY = "faraday_sessions";
const memoryCache = new Map<string, SignSession>();
let rehydrated = false;

async function rehydrate(): Promise<void> {
  if (rehydrated) return;
  try {
    const raw = await chrome.storage.session.get(STORAGE_KEY);
    const stored = (raw?.[STORAGE_KEY] ?? {}) as Record<string, SignSession>;
    for (const [id, session] of Object.entries(stored)) {
      memoryCache.set(id, session);
    }
  } catch (error) {
    warn("Failed rehydrating sessions from storage", { error: String(error) });
  }
  rehydrated = true;
}

async function persist(): Promise<void> {
  const serialized: Record<string, SignSession> = {};
  for (const [id, session] of memoryCache.entries()) {
    serialized[id] = session;
  }
  try {
    await chrome.storage.session.set({ [STORAGE_KEY]: serialized });
  } catch (error) {
    warn("Failed persisting sessions to storage", { error: String(error) });
  }
}

const sessions = {
  async get(id: string): Promise<SignSession | undefined> {
    await rehydrate();
    return memoryCache.get(id);
  },
  async set(id: string, session: SignSession): Promise<void> {
    await rehydrate();
    memoryCache.set(id, session);
    await persist();
  },
  async entries(): Promise<Array<[string, SignSession]>> {
    await rehydrate();
    return Array.from(memoryCache.entries());
  }
};

/// Sentinel for sessions originated by the extension sidepanel rather
/// than a dapp. Never matches a real HTTP(S) origin, so the existing
/// origin-comparison checks naturally reject cross-flow access.
const EXTENSION_ORIGIN = "ext:sidepanel";

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

function errorLog(message: string, meta?: unknown): void {
  if (meta === undefined) {
    console.error(`${LOG_PREFIX} ${message}`);
    return;
  }
  console.error(`${LOG_PREFIX} ${message}`, meta);
}

async function cleanupExpiredSessions(now = Date.now()): Promise<void> {
  const entries = await sessions.entries();
  for (const [sessionId, session] of entries) {
    if (session.expiresAt > now) {
      continue;
    }
    if (session.status !== "pending") {
      continue;
    }

    await sessions.set(sessionId, {
      ...session,
      status: "failed",
      error: session.error || "Signing session expired."
    });
  }
}

function parseOriginFromSender(sender: chrome.runtime.MessageSender): string | null {
  if (!sender.url) {
    return null;
  }

  try {
    return new URL(sender.url).origin;
  } catch {
    return null;
  }
}

function assertSenderOrigin(
  sender: chrome.runtime.MessageSender,
  claimedOrigin: string
): void {
  const senderOrigin = parseOriginFromSender(sender);
  if (senderOrigin && senderOrigin !== claimedOrigin) {
    throw new Error("Origin mismatch.");
  }
}

function messageTypeOf(message: RuntimeRequest | unknown): string {
  if (!message || typeof message !== "object") {
    return "<invalid>";
  }

  const maybe = message as { type?: unknown };
  return typeof maybe.type === "string" ? maybe.type : "<missing>";
}

function senderMeta(sender: chrome.runtime.MessageSender): Record<string, unknown> {
  return {
    url: sender.url || null,
    origin: parseOriginFromSender(sender),
    id: sender.id || null,
    tabId: sender.tab?.id ?? null,
    frameId: sender.frameId ?? null,
    documentId: sender.documentId ?? null,
  };
}

function assertExtensionSender(sender: chrome.runtime.MessageSender): void {
  const senderUrl = sender.url || "";
  const extensionBase = chrome.runtime.getURL("");
  if (!senderUrl.startsWith(extensionBase)) {
    throw new Error("Action allowed only from extension pages.");
  }
}

function makeSessionId(): string {
  if (typeof crypto !== "undefined" && typeof crypto.randomUUID === "function") {
    return crypto.randomUUID();
  }

  return `${Date.now()}-${Math.random().toString(16).slice(2)}`;
}

function formatError(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
}

async function openSigningUiWindow(signUrl: string): Promise<void> {
  let popupErr: string | null = null;
  let tabErr: string | null = null;

  try {
    if (chrome.windows?.create) {
      await chrome.windows.create({
        url: signUrl,
        type: "popup",
        width: 480,
        height: 860,
      });
      debug("Opened signing popup window", { signUrl });
      return;
    }
  } catch (error) {
    popupErr = formatError(error);
    warn("Popup open failed, attempting tab fallback", { error: popupErr, signUrl });
  }

  try {
    if (chrome.tabs?.create) {
      await chrome.tabs.create({
        url: signUrl,
        active: true,
      });
      debug("Opened signing tab fallback", { signUrl });
      return;
    }
  } catch (error) {
    tabErr = formatError(error);
    warn("Tab fallback open failed", { error: tabErr, signUrl });
  }

  const details = [popupErr, tabErr].filter(Boolean).join(" | ");
  throw new Error(
    details
      ? `Could not open Faraday signing UI. ${details}`
      : "Could not open Faraday signing UI."
  );
}

async function handleMessage(
  message: RuntimeRequest,
  sender: chrome.runtime.MessageSender
): Promise<RuntimeResponse> {
  await cleanupExpiredSessions();
  const messageType = messageTypeOf(message);
  debug("Handling message", {
    type: messageType,
    sender: senderMeta(sender),
  });

  switch (message.type) {
    case "faraday:get-state": {
      assertExtensionSender(sender);
      return { ok: true, data: await getExtensionState() };
    }

    case "faraday:set-paired-pubkey": {
      assertExtensionSender(sender);
      const pubkey = message.pubkey.trim();
      if (!isValidSolanaAddress(pubkey)) {
        return { ok: false, error: "Invalid Solana pubkey." };
      }

      const state = await setPairedPubkey(pubkey);
      return { ok: true, data: state };
    }

    case "faraday:clear-paired-pubkey": {
      assertExtensionSender(sender);
      const state = await clearPairedPubkey();
      return { ok: true, data: state };
    }

    case "faraday:approve-origin": {
      assertSenderOrigin(sender, message.origin);
      const state = await approveOrigin(message.origin);
      return { ok: true, data: state };
    }

    case "faraday:revoke-origin": {
      assertExtensionSender(sender);
      const state = await revokeOrigin(message.origin);
      return { ok: true, data: state };
    }

    case "faraday:clear-approved-origins": {
      assertExtensionSender(sender);
      const state = await clearApprovedOrigins();
      return { ok: true, data: state };
    }

    case "faraday:connect-check": {
      assertSenderOrigin(sender, message.origin);
      const state = await getExtensionState();
      const data: ConnectCheckResult = {
        pairedPubkey: state.pairedPubkey,
        approved: state.approvedOrigins.includes(message.origin)
      };
      return { ok: true, data };
    }

    case "faraday:create-sign-session": {
      assertSenderOrigin(sender, message.origin);

      const state = await getExtensionState();
      if (!state.pairedPubkey) {
        return { ok: false, error: "No paired pubkey. Open the extension and pair first." };
      }

      if (!state.approvedOrigins.includes(message.origin)) {
        return { ok: false, error: "Origin is not approved for Faraday." };
      }

      try {
        validateUnsignedTransactionPayload(message.txBase64, state.pairedPubkey);
      } catch (error) {
        const msg =
          error instanceof Error
            ? error.message
            : "Transaction payload is not a valid Solana transaction.";
        return { ok: false, error: msg };
      }

      const sessionId = makeSessionId();
      const session: SignSession = {
        id: sessionId,
        origin: message.origin,
        kind: "tx",
        txBase64: message.txBase64,
        expectedPubkey: state.pairedPubkey,
        status: "pending",
        createdAt: Date.now(),
        expiresAt: Date.now() + SESSION_TTL_MS
      };
      await sessions.set(sessionId, session);

      const data: CreateSignSessionResult = {
        sessionId,
        signUrl: chrome.runtime.getURL(`sign.html?session=${encodeURIComponent(sessionId)}`)
      };
      return { ok: true, data };
    }

    case "faraday:create-sign-message-session": {
      assertSenderOrigin(sender, message.origin);

      const state = await getExtensionState();
      if (!state.pairedPubkey) {
        return { ok: false, error: "No paired pubkey. Open the extension and pair first." };
      }

      if (!state.approvedOrigins.includes(message.origin)) {
        return { ok: false, error: "Origin is not approved for Faraday." };
      }

      let messageBytes: Uint8Array;
      let messageQrBase64: string;
      try {
        messageBytes = decodeBase64(message.messageBase64.trim());
        messageQrBase64 = buildSignMessageQrPayload(messageBytes);
      } catch (error) {
        const msg =
          error instanceof Error
            ? error.message
            : "Message payload is not valid for Faraday sign-message flow.";
        return { ok: false, error: msg };
      }

      const sessionId = makeSessionId();
      const session: SignSession = {
        id: sessionId,
        origin: message.origin,
        kind: "message",
        messageBase64: message.messageBase64,
        messageQrBase64,
        expectedPubkey: state.pairedPubkey,
        status: "pending",
        createdAt: Date.now(),
        expiresAt: Date.now() + SESSION_TTL_MS
      };
      await sessions.set(sessionId, session);

      const data: CreateSignSessionResult = {
        sessionId,
        signUrl: chrome.runtime.getURL(`sign.html?session=${encodeURIComponent(sessionId)}`)
      };
      return { ok: true, data };
    }

    case "faraday:open-sign-window": {
      assertSenderOrigin(sender, message.origin);

      const state = await getExtensionState();
      if (!state.approvedOrigins.includes(message.origin)) {
        return { ok: false, error: "Origin is not approved for Faraday." };
      }

      const session = await sessions.get(message.sessionId);
      if (!session) {
        return { ok: false, error: "Signing session not found." };
      }
      if (session.origin !== message.origin) {
        return { ok: false, error: "Signing session origin mismatch." };
      }
      if (session.status !== "pending") {
        return { ok: false, error: `Session is already ${session.status}.` };
      }

      const signUrl = chrome.runtime.getURL(`sign.html?session=${encodeURIComponent(session.id)}`);
      await openSigningUiWindow(signUrl);

      return { ok: true, data: { opened: true } };
    }

    case "faraday:ext-create-sign-session": {
      // Sidepanel Send flow. Sender must be the extension itself — we
      // don't need the approved-origins check because this isn't a dapp
      // request. `parseOriginFromSender` returns the chrome-extension://
      // origin for messages from our own surfaces.
      assertExtensionSender(sender);

      const state = await getExtensionState();
      if (!state.pairedPubkey) {
        return { ok: false, error: "No paired pubkey. Pair a wallet first." };
      }

      try {
        validateUnsignedTransactionPayload(message.txBase64, state.pairedPubkey);
      } catch (error) {
        const msg =
          error instanceof Error
            ? error.message
            : "Transaction payload is not a valid Solana transaction.";
        return { ok: false, error: msg };
      }

      const sessionId = makeSessionId();
      const session: SignSession = {
        id: sessionId,
        origin: EXTENSION_ORIGIN,
        kind: "tx",
        txBase64: message.txBase64,
        expectedPubkey: state.pairedPubkey,
        status: "pending",
        createdAt: Date.now(),
        expiresAt: Date.now() + SESSION_TTL_MS
      };
      await sessions.set(sessionId, session);

      const data: CreateSignSessionResult = {
        sessionId,
        signUrl: chrome.runtime.getURL(`sign.html?session=${encodeURIComponent(sessionId)}`)
      };
      return { ok: true, data };
    }

    case "faraday:ext-open-sign-window": {
      assertExtensionSender(sender);

      const session = await sessions.get(message.sessionId);
      if (!session) {
        return { ok: false, error: "Signing session not found." };
      }
      if (session.origin !== EXTENSION_ORIGIN) {
        return { ok: false, error: "Session is not an extension-originated session." };
      }
      if (session.status !== "pending") {
        return { ok: false, error: `Session is already ${session.status}.` };
      }

      const signUrl = chrome.runtime.getURL(`sign.html?session=${encodeURIComponent(session.id)}`);
      await openSigningUiWindow(signUrl);

      return { ok: true, data: { opened: true } };
    }

    case "faraday:get-sign-session": {
      const session = await sessions.get(message.sessionId);
      if (!session) {
        return { ok: false, error: "Signing session not found." };
      }

      const data: GetSignSessionResult = {
        sessionId: session.id,
        origin: session.origin,
        kind: session.kind,
        txBase64: session.txBase64,
        messageQrBase64: session.messageQrBase64,
        expectedPubkey: session.expectedPubkey,
        status: session.status,
        error: session.error
      };
      return { ok: true, data };
    }

    case "faraday:get-sign-result": {
      const session = await sessions.get(message.sessionId);
      if (!session) {
        return { ok: false, error: "Signing session not found." };
      }

      const data: GetSignResult = {
        kind: session.kind,
        status: session.status,
        signedTxBase64: session.signedTxBase64,
        signatureHex: session.signatureHex,
        error: session.error
      };
      return { ok: true, data };
    }

    case "faraday:complete-sign-session": {
      const session = await sessions.get(message.sessionId);
      if (!session) {
        return { ok: false, error: "Signing session not found." };
      }
      if (session.status !== "pending") {
        return { ok: false, error: `Session is already ${session.status}.` };
      }
      if (session.kind !== "tx" || !session.txBase64) {
        return { ok: false, error: "Session is not a transaction signing session." };
      }

      try {
        validateSignedTransactionMatch(
          session.txBase64,
          message.signedTxBase64,
          session.expectedPubkey
        );
      } catch (error) {
        const msg = error instanceof Error ? error.message : "Invalid signed transaction.";
        await sessions.set(session.id, {
          ...session,
          status: "failed",
          error: msg
        });
        return { ok: false, error: msg };
      }

      await sessions.set(session.id, {
        ...session,
        status: "completed",
        signedTxBase64: message.signedTxBase64,
        error: undefined
      });
      return { ok: true, data: { status: "completed" } };
    }

    case "faraday:complete-sign-message-session": {
      const session = await sessions.get(message.sessionId);
      if (!session) {
        return { ok: false, error: "Signing session not found." };
      }
      if (session.status !== "pending") {
        return { ok: false, error: `Session is already ${session.status}.` };
      }
      if (session.kind !== "message" || !session.messageBase64) {
        return { ok: false, error: "Session is not a message signing session." };
      }

      let messageBytes: Uint8Array;
      try {
        messageBytes = decodeBase64(session.messageBase64.trim());
      } catch {
        return { ok: false, error: "Stored sign-message payload is invalid base64." };
      }

      try {
        validateSignedMessage(messageBytes, message.signatureHex, session.expectedPubkey);
      } catch (error) {
        const msg = error instanceof Error ? error.message : "Invalid message signature.";
        await sessions.set(session.id, {
          ...session,
          status: "failed",
          error: msg
        });
        return { ok: false, error: msg };
      }

      await sessions.set(session.id, {
        ...session,
        status: "completed",
        signatureHex: message.signatureHex.trim(),
        error: undefined
      });
      return { ok: true, data: { status: "completed" } };
    }

    case "faraday:cancel-sign-session": {
      const session = await sessions.get(message.sessionId);
      if (!session) {
        return { ok: false, error: "Signing session not found." };
      }

      await sessions.set(session.id, {
        ...session,
        status: "canceled",
        error: message.reason || "Signing canceled."
      });
      return { ok: true, data: { status: "canceled" } };
    }

    default: {
      warn("Received unknown message type", {
        type: messageType,
        sender: senderMeta(sender),
      });
      return { ok: false, error: `Unknown message type: ${messageType}` };
    }
  }
}

export default defineBackground(() => {
  // Toolbar click opens the side panel instead of a popup.
  // Persists across restarts once set; safe to call on every service
  // worker wake-up.
  chrome.sidePanel
    .setPanelBehavior({ openPanelOnActionClick: true })
    .catch((error) => {
      errorLog("Failed to configure side panel behavior", {
        error: error instanceof Error ? error.message : String(error)
      });
    });

  chrome.runtime.onMessage.addListener(
    (
      message: RuntimeRequest,
      sender: chrome.runtime.MessageSender,
      sendResponse: (response: RuntimeResponse) => void
    ) => {
      const incomingType = messageTypeOf(message);
      void handleMessage(message, sender)
        .then((response) => {
          if (!response.ok) {
            warn("Message handled with error response", {
              type: incomingType,
              error: response.error,
            });
          }
          sendResponse(response);
        })
        .catch((error) => {
          const msg = error instanceof Error ? error.message : "Unhandled background error.";
          errorLog("Unhandled background exception", {
            type: incomingType,
            error: msg,
            stack: error instanceof Error ? error.stack : null,
            sender: senderMeta(sender),
          });
          sendResponse({ ok: false, error: msg });
        });

      return true;
    }
  );
});
