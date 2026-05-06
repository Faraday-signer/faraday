const FALLBACK_RPC_URL = "https://api.mainnet-beta.solana.com";

function pickRpcUrl(): string {
  const fromEnv = process.env.EXPO_PUBLIC_RPC_URL?.trim() ?? "";
  return fromEnv.length > 0 ? fromEnv : FALLBACK_RPC_URL;
}

export const RPC_URL = pickRpcUrl();
export const IS_PUBLIC_RPC = RPC_URL === FALLBACK_RPC_URL;
export const CLUSTER_ID = "mainnet-beta" as const;
export const CLUSTER_LABEL = "Mainnet" as const;

export function redactRpcUrl(url: string): string {
  try {
    const u = new URL(url);
    if (u.searchParams.has("api-key")) {
      u.searchParams.set("api-key", "•••");
    }
    return u.toString();
  } catch {
    return url;
  }
}
