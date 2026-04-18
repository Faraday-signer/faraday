import type { RuntimeRequest, RuntimeResponse } from "./types";

export function sendRuntimeMessage<T = unknown>(
  message: RuntimeRequest
): Promise<RuntimeResponse<T>> {
  return new Promise((resolve) => {
    chrome.runtime.sendMessage(message, (response: RuntimeResponse<T> | undefined) => {
      const lastError = chrome.runtime.lastError;
      if (lastError) {
        resolve({ ok: false, error: lastError.message || "Runtime error" });
        return;
      }

      if (!response) {
        resolve({ ok: false, error: "No response from background." });
        return;
      }

      resolve(response);
    });
  });
}
