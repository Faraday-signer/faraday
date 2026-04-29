import type { RuntimeRequest, RuntimeResponse } from "@/lib/types";

const LOG_PREFIX = "[Faraday][content]";

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

type BridgeRequest = {
  target: "faraday:content";
  id: string;
  message: RuntimeRequest;
};

type BridgeResponse = {
  target: "faraday:inpage";
  id: string;
  response: RuntimeResponse;
};

function injectInpageScript(): void {
  const script = document.createElement("script");
  script.src = chrome.runtime.getURL("inpage.js");
  script.async = false;
  script.dataset.faraday = "injected";

  const parent = document.head || document.documentElement;
  if (!parent) {
    warn("Could not find document root to inject inpage script");
    return;
  }

  parent.appendChild(script);
  debug("Injected inpage script", { src: script.src });
  script.remove();
}

function isBridgeRequest(value: unknown): value is BridgeRequest {
  if (!value || typeof value !== "object") {
    return false;
  }

  const msg = value as Partial<BridgeRequest>;
  return (
    msg.target === "faraday:content" &&
    typeof msg.id === "string" &&
    !!msg.message &&
    typeof msg.message === "object"
  );
}

function postBridgeResponse(payload: BridgeResponse): void {
  window.postMessage(payload, "*");
}

function startBridge(): void {
  window.addEventListener("message", (event: MessageEvent) => {
    if (event.source !== window) {
      return;
    }

    if (!isBridgeRequest(event.data)) {
      return;
    }

    chrome.runtime.sendMessage(event.data.message, (response: RuntimeResponse | undefined) => {
      const lastError = chrome.runtime.lastError;
      const messageType = messageTypeOf(event.data.message);

      if (lastError) {
        warn("Runtime bridge request failed", {
          type: messageType,
          requestId: event.data.id,
          error: lastError.message || "Runtime bridge error"
        });
        postBridgeResponse({
          target: "faraday:inpage",
          id: event.data.id,
          response: {
            ok: false,
            error: lastError.message || "Runtime bridge error"
          }
        });
        return;
      }

      if (!response) {
        warn("Runtime bridge returned no response", {
          type: messageType,
          requestId: event.data.id
        });
      } else if (!response.ok) {
        warn("Runtime bridge returned error response", {
          type: messageType,
          requestId: event.data.id,
          error: response.error
        });
      }

      postBridgeResponse({
        target: "faraday:inpage",
        id: event.data.id,
        response: response || { ok: false, error: "No response from background." }
      });
    });
  });
}

export default defineContentScript({
  matches: ["<all_urls>"],
  runAt: "document_start",
  main() {
    injectInpageScript();
    startBridge();
  }
});
