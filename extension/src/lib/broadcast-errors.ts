//! User-facing translation of Solana RPC / broadcast errors.
//!
//! `@solana/kit` (via `@solana/errors`) throws errors whose `.message`
//! looks like `"-32002; Decode this error by running `npx @solana/errors
//! decode -- -32002 '<base64>'`"`. The base64 blob is URL-encoded form
//! data with the full RPC context (logs, units consumed, balances, ...).
//!
//! The two exports here are screen-agnostic — pure string→object
//! transforms — so any UI surface that surfaces a broadcast error
//! (sidepanel send, future SPL send, future nonce-account flows) can
//! call `explainBroadcastError(err.message)` and feed both fields into
//! the standard `ErrorBanner`.

export interface BroadcastErrorReport {
  /** One-line summary safe to render inline. Plain English. */
  summary: string;
  /** Verbose detail block — always readable, never raw base64. */
  details: string;
}

/**
 * Pull the base64 context blob out of an `@solana/errors`-style message
 * and decode it into a plain object. Returns null when the message
 * doesn't carry a blob, or the blob is malformed.
 */
export function decodeSolanaErrorContext(
  text: string
): Record<string, string> | null {
  const match = text.match(/decode\s+--\s+-?\d+\s+'([A-Za-z0-9+/=_-]+)'/);
  if (!match) return null;
  try {
    const decoded = atob(match[1]);
    const params = new URLSearchParams(decoded);
    const ctx: Record<string, string> = {};
    for (const [key, value] of params) {
      ctx[key] = value;
    }
    return ctx;
  } catch {
    return null;
  }
}

/** Best-effort JSON parse — returns null on failure rather than throwing. */
function safeParseJson(value: string | undefined): unknown {
  if (!value) return null;
  try {
    return JSON.parse(value);
  } catch {
    return null;
  }
}

/**
 * Pretty-print decoded RPC context, dropping null/empty fields and the
 * redundant `__code` (already in the summary). Returns one line per field.
 */
function formatErrorContext(ctx: Record<string, string>): string {
  const lines: string[] = [];
  for (const [key, value] of Object.entries(ctx)) {
    if (value === "null" || value === "" || key === "__code") continue;
    lines.push(`${key}: ${value}`);
  }
  return lines.length > 0 ? lines.join("\n") : "No additional context.";
}

/**
 * Map a thrown error message to a `{ summary, details }` pair safe to
 * pass straight into `<ErrorBanner>`.
 *
 * Strategy:
 *   1. Try to decode the base64 context blob the `@solana/errors` library
 *      embeds in `-32xxx` messages. That gives `logs`, `unitsConsumed`,
 *      etc. directly.
 *   2. Use those fields to refine the summary:
 *      - empty logs + 0 units → simulation didn't run → blockhash hint
 *      - non-empty logs → last log line is usually the failure reason
 *   3. Fall back to keyword matching for errors that don't carry a blob.
 *
 * `details` is always a readable string — never raw base64.
 */
export function explainBroadcastError(raw: string): BroadcastErrorReport {
  const text = raw.trim();
  const ctx = decodeSolanaErrorContext(text);
  const decodedDetails = ctx ? formatErrorContext(ctx) : text;

  // -32002 — "Transaction simulation failed". The decoded `logs` field
  // tells us whether simulation actually ran or bailed at blockhash check.
  if (/-32002/.test(text)) {
    const logs = safeParseJson(ctx?.logs);
    const units = ctx?.unitsConsumed
      ? Number(ctx.unitsConsumed.replace(/n$/, ""))
      : 0;

    if (Array.isArray(logs) && logs.length === 0 && units === 0) {
      return {
        summary:
          "Blockhash likely expired — the signed transaction sat too long between scan and broadcast. Please retry.",
        details: decodedDetails,
      };
    }
    if (Array.isArray(logs) && logs.length > 0) {
      const lastLog = String(logs[logs.length - 1]).trim();
      return {
        summary: `Preflight failed: ${lastLog}`,
        details: decodedDetails,
      };
    }
    return {
      summary: "The RPC rejected the transaction at preflight.",
      details: decodedDetails,
    };
  }

  if (/Blockhash not found|block height exceeded/i.test(text)) {
    return {
      summary:
        "Blockhash expired. The signed transaction sat too long between scan and broadcast — please retry.",
      details: decodedDetails,
    };
  }
  if (/insufficient (lamports|funds)/i.test(text)) {
    return {
      summary: "Insufficient balance for this transaction.",
      details: decodedDetails,
    };
  }
  if (/-32005/.test(text) || /node is behind/i.test(text)) {
    return {
      summary: "RPC node is lagging — try again in a moment.",
      details: decodedDetails,
    };
  }

  // Generic fallback — first line of the raw message as summary, decoded
  // (or full) details underneath.
  const firstLine = text.split("\n", 1)[0]?.trim() ?? text;
  const truncated =
    firstLine.length > 140 ? `${firstLine.slice(0, 140)}…` : firstLine;
  return { summary: truncated, details: decodedDetails };
}
