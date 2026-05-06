import { createSolanaRpc, createSolanaRpcSubscriptions } from "@solana/kit";

const FALLBACK_RPC_URL = "https://api.mainnet-beta.solana.com";

function pickRpcUrl(): string {
  const fromEnv = process.env.EXPO_PUBLIC_RPC_URL?.trim() ?? "";
  return fromEnv.length > 0 ? fromEnv : FALLBACK_RPC_URL;
}

export const RPC_URL = pickRpcUrl();
export const IS_PUBLIC_RPC = RPC_URL === FALLBACK_RPC_URL;
export const CLUSTER_ID = "mainnet-beta" as const;
export const CLUSTER_LABEL = "MAINNET";

export function redactRpcUrl(url: string): string {
  try {
    const parsed = new URL(url);
    const key = parsed.searchParams.get("api-key");
    if (key && key.length > 12) {
      parsed.searchParams.set("api-key", `${key.slice(0, 4)}…${key.slice(-4)}`);
    } else if (key) {
      parsed.searchParams.set("api-key", "***");
    }
    return parsed.toString();
  } catch {
    return url;
  }
}

export const solanaRpc = createSolanaRpc(RPC_URL);

export const WSS_URL = RPC_URL.replace(/^http(s)?:/, (_, s) => (s ? "wss:" : "ws:"));

export const solanaRpcSubscriptions = createSolanaRpcSubscriptions(WSS_URL);
