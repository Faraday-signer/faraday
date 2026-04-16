import { createSolanaRpc } from "@solana/kit";

/**
 * Default mainnet RPC endpoint. Can be overridden in Settings → Network
 * once that flow lands. Public RPC is rate-limited but fine for one user
 * reading their own balance.
 */
export const DEFAULT_RPC_URL = "https://api.mainnet-beta.solana.com";

/**
 * Single SolanaRpc instance shared across the side panel. Pure `@solana/kit`
 * primitives — no `@solana/client` or `@solana/react-hooks`. Caching and
 * polling are layered on top with SWR in `useWallet`.
 *
 * Methods are called as `solanaRpc.getBalance(address).send()`.
 */
export const solanaRpc = createSolanaRpc(DEFAULT_RPC_URL);

export const CLUSTER_LABEL = "MAINNET";
export const CLUSTER_ID = "mainnet-beta" as const;
