import { createSolanaRpc, createSolanaRpcSubscriptions } from "@solana/kit";

/**
 * Fallback RPC if no override is configured. Public mainnet is free but
 * heavily rate-limited; expect "429 Too Many Requests" under any real load.
 * Override via VITE_RPC_URL (see extension/.env.example).
 */
const FALLBACK_RPC_URL = "https://api.mainnet-beta.solana.com";

function pickRpcUrl(): string {
  const fromEnv =
    typeof import.meta.env.VITE_RPC_URL === "string"
      ? import.meta.env.VITE_RPC_URL.trim()
      : "";
  return fromEnv.length > 0 ? fromEnv : FALLBACK_RPC_URL;
}

export const RPC_URL = pickRpcUrl();

/**
 * Redact any `api-key=<value>` portion of the URL for display so we don't
 * splatter the configured key across the Settings UI or logs. Keeps the
 * host, path, and first/last 4 chars of the key as a breadcrumb.
 */
export function redactRpcUrl(url: string): string {
  try {
    const parsed = new URL(url);
    const key = parsed.searchParams.get("api-key");
    if (key && key.length > 12) {
      const redacted = `${key.slice(0, 4)}…${key.slice(-4)}`;
      parsed.searchParams.set("api-key", redacted);
    } else if (key) {
      parsed.searchParams.set("api-key", "***");
    }
    return parsed.toString();
  } catch {
    return url;
  }
}

/** True when using the public RPC fallback (rate-limited, not for real use). */
export const IS_PUBLIC_RPC = RPC_URL === FALLBACK_RPC_URL;

/**
 * Single SolanaRpc instance shared across the side panel. Pure `@solana/kit`
 * primitives — no `@solana/client` or `@solana/react-hooks`. Caching and
 * polling are layered on top with SWR in `useWallet`.
 *
 * Methods are called as `solanaRpc.getBalance(address).send()`.
 */
export const solanaRpc = createSolanaRpc(RPC_URL);

/**
 * WebSocket subscription endpoint. Derived from RPC_URL by protocol swap —
 * Helius, QuickNode, and the public Solana RPC all follow this convention.
 */
export const WSS_URL = RPC_URL.replace(/^http(s)?:/, (_, s) => (s ? "wss:" : "ws:"));

/**
 * SolanaRpcSubscriptions for live push notifications (balance changes,
 * signature confirmations). Used by `useLiveBalance`. Do not hand this to
 * untrusted code — subscriptions live for as long as the channel is open
 * and each one consumes a slot on the server side.
 */
export const solanaRpcSubscriptions = createSolanaRpcSubscriptions(WSS_URL);

export const CLUSTER_LABEL = "MAINNET";
export const CLUSTER_ID = "mainnet-beta" as const;
