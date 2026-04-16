import {
  approveOrigin,
  clearApprovedOrigins,
  clearPairedPubkey,
  getExtensionState,
  revokeOrigin,
  setPairedPubkey
} from "../src/lib/storage";
import {
  isValidSolanaAddress,
  validateSignedTransactionMatch,
  validateUnsignedTransactionPayload
} from "../src/lib/solana";
import type {
  ConnectCheckResult,
  CreateSignSessionResult,
  GetSignResult,
  GetSignSessionResult,
  RuntimeRequest,
  RuntimeResponse,
  SignSession
} from "../src/lib/types";

const SESSION_TTL_MS = 5 * 60 * 1000;
const sessions = new Map<string, SignSession>();
const LOG_PREFIX = "[Faraday][background]";

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

function cleanupExpiredSessions(now = Date.now()): void {
  for (const [sessionId, session] of sessions.entries()) {
    if (session.expiresAt > now) {
      continue;
    }

    sessions.set(sessionId, {
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
  cleanupExpiredSessions();
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
        txBase64: message.txBase64,
        expectedPubkey: state.pairedPubkey,
        status: "pending",
        createdAt: Date.now(),
        expiresAt: Date.now() + SESSION_TTL_MS
      };
      sessions.set(sessionId, session);

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

      const session = sessions.get(message.sessionId);
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

    case "faraday:get-sign-session": {
      const session = sessions.get(message.sessionId);
      if (!session) {
        return { ok: false, error: "Signing session not found." };
      }

      const data: GetSignSessionResult = {
        sessionId: session.id,
        origin: session.origin,
        txBase64: session.txBase64,
        expectedPubkey: session.expectedPubkey,
        status: session.status,
        error: session.error
      };
      return { ok: true, data };
    }

    case "faraday:get-sign-result": {
      const session = sessions.get(message.sessionId);
      if (!session) {
        return { ok: false, error: "Signing session not found." };
      }

      const data: GetSignResult = {
        status: session.status,
        signedTxBase64: session.signedTxBase64,
        error: session.error
      };
      return { ok: true, data };
    }

    case "faraday:complete-sign-session": {
      const session = sessions.get(message.sessionId);
      if (!session) {
        return { ok: false, error: "Signing session not found." };
      }
      if (session.status !== "pending") {
        return { ok: false, error: `Session is already ${session.status}.` };
      }

      try {
        validateSignedTransactionMatch(
          session.txBase64,
          message.signedTxBase64,
          session.expectedPubkey
        );
      } catch (error) {
        const msg = error instanceof Error ? error.message : "Invalid signed transaction.";
        sessions.set(session.id, {
          ...session,
          status: "failed",
          error: msg
        });
        return { ok: false, error: msg };
      }

      sessions.set(session.id, {
        ...session,
        status: "completed",
        signedTxBase64: message.signedTxBase64,
        error: undefined
      });
      return { ok: true, data: { status: "completed" } };
    }

    case "faraday:cancel-sign-session": {
      const session = sessions.get(message.sessionId);
      if (!session) {
        return { ok: false, error: "Signing session not found." };
      }

      sessions.set(session.id, {
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
