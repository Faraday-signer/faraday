//! Helpers for the camera-permission UX.
//!
//! Chrome MV3 has no extension-level "I want the camera" manifest
//! permission — `getUserMedia` always goes through Chrome's per-origin
//! web prompt. The user can:
//!   - allow once → all extension surfaces get camera access (same origin)
//!   - dismiss the prompt → next call re-prompts (after a delay)
//!   - block → Chrome won't prompt again; user has to re-enable in
//!     `chrome://extensions/?id=<extensionId>` → Site access → Camera
//!
//! We can't programmatically open the camera site setting, but we CAN
//! deep-link to the extension's settings page where it lives.

export type CameraPermissionState = "granted" | "prompt" | "denied" | "unknown";

/**
 * Best-effort check of the user's current camera-permission state. Uses
 * the Permissions API where available; falls back to "unknown" so the
 * caller can decide whether to attempt `getUserMedia` and discover the
 * state from the resulting error.
 */
export async function getCameraPermissionState(): Promise<CameraPermissionState> {
  try {
    if (!navigator.permissions || typeof navigator.permissions.query !== "function") {
      return "unknown";
    }
    const status = await navigator.permissions.query({
      name: "camera" as PermissionName,
    });
    if (status.state === "granted") return "granted";
    if (status.state === "denied") return "denied";
    return "prompt";
  } catch {
    return "unknown";
  }
}

/**
 * Categorise a `getUserMedia` failure so the UI can pick the right
 * recovery copy. Chrome surfaces these as DOMExceptions with specific
 * `name` values.
 */
export type CameraFailureKind = "denied" | "no-device" | "in-use" | "other";

export function categorizeCameraError(error: unknown): {
  kind: CameraFailureKind;
  message: string;
} {
  const message = error instanceof Error ? error.message : String(error);

  if (error && typeof error === "object" && "name" in error) {
    const name = String((error as { name: unknown }).name);
    if (name === "NotAllowedError" || name === "PermissionDeniedError" || name === "SecurityError") {
      return { kind: "denied", message };
    }
    if (name === "NotFoundError" || name === "DevicesNotFoundError" || name === "OverconstrainedError") {
      return { kind: "no-device", message };
    }
    if (name === "NotReadableError" || name === "TrackStartError" || name === "AbortError") {
      return { kind: "in-use", message };
    }
  }

  // Some Chromium builds throw plain `Error` with the readable message.
  if (/permission|denied|not allowed/i.test(message)) {
    return { kind: "denied", message };
  }
  if (/no.*camera|device.*not.*found/i.test(message)) {
    return { kind: "no-device", message };
  }
  return { kind: "other", message };
}

/**
 * Open the extension's own settings page (where the user can re-enable
 * camera access if they previously blocked it). Returns whether the
 * tab open call succeeded — callers can use this to decide whether to
 * show a "manual instructions" fallback.
 */
export async function openExtensionSettings(): Promise<boolean> {
  try {
    const url = `chrome://extensions/?id=${chrome.runtime.id}`;
    if (chrome.tabs?.create) {
      await chrome.tabs.create({ url, active: true });
      return true;
    }
  } catch {
    // chrome:// tabs can't always be opened from the popup; let the
    // caller fall back to copy + manual paste instructions.
  }
  return false;
}
