export interface BroadcastErrorReport {
  summary: string;
  details: string;
}

export function decodeSolanaErrorContext(text: string): Record<string, string> | null {
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

function safeParseJson(value: string | undefined): unknown {
  if (!value) return null;
  try {
    return JSON.parse(value);
  } catch {
    return null;
  }
}

function formatErrorContext(ctx: Record<string, string>): string {
  const lines: string[] = [];
  for (const [key, value] of Object.entries(ctx)) {
    if (value === "null" || value === "" || key === "__code") continue;
    lines.push(`${key}: ${value}`);
  }
  return lines.length > 0 ? lines.join("\n") : "No additional context.";
}

export function explainBroadcastError(raw: string): BroadcastErrorReport {
  const text = raw.trim();
  const ctx = decodeSolanaErrorContext(text);
  const decodedDetails = ctx ? formatErrorContext(ctx) : text;

  if (/-32002/.test(text)) {
    const logs = safeParseJson(ctx?.logs);
    const units = ctx?.unitsConsumed ? Number(ctx.unitsConsumed.replace(/n$/, "")) : 0;

    if (Array.isArray(logs) && logs.length === 0 && units === 0) {
      return {
        summary:
          "Blockhash likely expired — the signed transaction sat too long between scan and broadcast. Please retry.",
        details: decodedDetails
      };
    }
    if (Array.isArray(logs) && logs.length > 0) {
      const lastLog = String(logs[logs.length - 1]).trim();
      return { summary: `Preflight failed: ${lastLog}`, details: decodedDetails };
    }
    return {
      summary: "The RPC rejected the transaction at preflight.",
      details: decodedDetails
    };
  }

  if (/Blockhash not found|block height exceeded/i.test(text)) {
    return {
      summary:
        "Blockhash expired. The signed transaction sat too long between scan and broadcast — please retry.",
      details: decodedDetails
    };
  }
  if (/insufficient (lamports|funds)/i.test(text)) {
    return { summary: "Insufficient balance for this transaction.", details: decodedDetails };
  }
  if (/-32005/.test(text) || /node is behind/i.test(text)) {
    return { summary: "RPC node is lagging — try again in a moment.", details: decodedDetails };
  }

  const firstLine = text.split("\n", 1)[0]?.trim() ?? text;
  const truncated = firstLine.length > 140 ? `${firstLine.slice(0, 140)}…` : firstLine;
  return { summary: truncated, details: decodedDetails };
}
