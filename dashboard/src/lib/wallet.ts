import { getWallets } from "@wallet-standard/app";

export const CHAIN = "solana:mainnet";

export interface WalletAccount {
  readonly address: string;
  readonly publicKey: Uint8Array | { readonly [n: number]: number; readonly length: number };
}

interface ConnectFeature {
  connect: (input?: { silent?: boolean }) => Promise<{ accounts: readonly WalletAccount[] }>;
}

interface DisconnectFeature {
  disconnect?: () => Promise<void>;
}

interface SignTransactionFeature {
  signTransaction: (...inputs: Array<{
    account: WalletAccount;
    chain: string;
    transaction: Uint8Array;
  }>) => Promise<Array<{ signedTransaction: Uint8Array }>>;
}

export interface DiscoveredWallet {
  readonly name: string;
  readonly icon?: string;
  readonly chains: readonly string[];
  readonly accounts: readonly WalletAccount[];
  readonly features: {
    readonly "standard:connect"?: ConnectFeature;
    readonly "standard:disconnect"?: DisconnectFeature;
    readonly "solana:signTransaction"?: SignTransactionFeature;
  };
}

export async function signTransactionWithWallet(
  w: DiscoveredWallet,
  account: WalletAccount,
  txBytes: Uint8Array,
): Promise<Uint8Array> {
  const feature = w.features["solana:signTransaction"];
  if (!feature) throw new Error(`${w.name} can't sign transactions`);
  const [{ signedTransaction }] = await feature.signTransaction({
    account,
    chain: CHAIN,
    transaction: txBytes,
  });
  return signedTransaction;
}

export function discoverSolanaWallets(): { wallets: DiscoveredWallet[]; subscribe: (fn: (wallets: DiscoveredWallet[]) => void) => () => void } {
  const api = getWallets();
  const filter = (): DiscoveredWallet[] =>
    api.get().filter(isSolanaWallet);
  const subscribe = (fn: (wallets: DiscoveredWallet[]) => void) => {
    const off1 = api.on("register", () => fn(filter()));
    const off2 = api.on("unregister", () => fn(filter()));
    return () => { off1(); off2(); };
  };
  return { wallets: filter(), subscribe };
}

export async function connectWallet(w: DiscoveredWallet): Promise<WalletAccount> {
  if (!w.chains.includes(CHAIN)) {
    throw new Error(`${w.name} doesn't advertise ${CHAIN}`);
  }
  const feature = w.features["standard:connect"];
  if (!feature) throw new Error(`${w.name} can't connect`);
  const { accounts } = await feature.connect();
  const acct = accounts[0];
  if (!acct) throw new Error("no account returned");
  return acct;
}

export async function disconnectWallet(w: DiscoveredWallet): Promise<void> {
  const feature = w.features["standard:disconnect"];
  if (feature?.disconnect) await feature.disconnect();
}

function isSolanaWallet(w: unknown): w is DiscoveredWallet {
  if (!w || typeof w !== "object") return false;
  const x = w as { chains?: readonly string[]; features?: Record<string, unknown> };
  return Array.isArray(x.chains) && x.chains.some((c) => c.startsWith("solana:"))
    && !!x.features?.["standard:connect"];
}
