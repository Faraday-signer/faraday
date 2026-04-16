import { getWallets } from "@wallet-standard/app";
import {
  Connection,
  LAMPORTS_PER_SOL,
  PublicKey,
  SystemProgram,
  Transaction
} from "@solana/web3.js";

export const RPC_URL = "https://api.devnet.solana.com";
export const CHAIN = "solana:devnet";

export const connection = new Connection(RPC_URL, "confirmed");
export const walletsApi = getWallets();

// Wallet Standard's published types use `ReadonlyUint8Array` and an opaque
// `Wallet` union that's hard to narrow. For a test harness we don't need strict
// typing — we duck-type at runtime and keep the UI layer tolerant.

export interface StandardAccount {
  readonly address: string;
  readonly publicKey: Uint8Array | { readonly [n: number]: number; readonly length: number };
}

interface StandardConnectFeature {
  connect: (input?: { silent?: boolean }) => Promise<{ accounts: readonly StandardAccount[] }>;
}

interface StandardDisconnectFeature {
  disconnect: () => Promise<void>;
}

interface SolanaSignTransactionFeature {
  signTransaction: (...inputs: Array<{
    account: StandardAccount;
    chain: string;
    transaction: Uint8Array;
  }>) => Promise<Array<{ signedTransaction: Uint8Array }>>;
}

export interface SupportedWallet {
  readonly name: string;
  readonly icon?: string;
  readonly chains: readonly string[];
  readonly accounts: readonly StandardAccount[];
  readonly features: {
    readonly "standard:connect"?: StandardConnectFeature;
    readonly "standard:disconnect"?: StandardDisconnectFeature;
    readonly "solana:signTransaction"?: SolanaSignTransactionFeature;
  };
}

function supportsFaradayFlow(wallet: unknown): boolean {
  if (!wallet || typeof wallet !== "object") return false;
  const w = wallet as {
    chains?: readonly string[];
    features?: Record<string, unknown>;
  };
  const chains = Array.isArray(w.chains) ? w.chains : [];
  const hasSolanaChain = chains.some((chain) => chain.startsWith("solana:"));
  const hasConnect = Boolean(w.features?.["standard:connect"]);
  const hasSign = Boolean(w.features?.["solana:signTransaction"]);
  return hasSolanaChain && hasConnect && hasSign;
}

export function detectWallets(): SupportedWallet[] {
  return walletsApi
    .get()
    .filter(supportsFaradayFlow) as unknown as SupportedWallet[];
}

export function shortAddress(address: string): string {
  if (address.length <= 12) return address;
  return `${address.slice(0, 4)}…${address.slice(-4)}`;
}

export async function connectWallet(wallet: SupportedWallet): Promise<StandardAccount> {
  const feature = wallet.features["standard:connect"];
  if (!feature) {
    throw new Error("Wallet missing standard:connect feature.");
  }
  const result = await feature.connect();
  const account = result.accounts?.[0] ?? wallet.accounts?.[0];
  if (!account) {
    throw new Error("Connected wallet did not return any account.");
  }
  return account;
}

export async function disconnectWallet(wallet: SupportedWallet): Promise<void> {
  const feature = wallet.features["standard:disconnect"];
  if (feature?.disconnect) {
    await feature.disconnect();
  }
}

export async function requestAirdrop(address: string): Promise<string> {
  const signature = await connection.requestAirdrop(new PublicKey(address), LAMPORTS_PER_SOL);
  await connection.confirmTransaction(signature, "confirmed");
  return signature;
}

export async function signAndSendTransfer(params: {
  wallet: SupportedWallet;
  account: StandardAccount;
  recipient: string;
  amountSol: number;
}): Promise<{ signature: string }> {
  const { wallet, account, recipient, amountSol } = params;

  if (!Number.isFinite(amountSol) || amountSol <= 0) {
    throw new Error("Amount must be > 0.");
  }

  const fromPubkey = new PublicKey(account.address);
  const toPubkey = new PublicKey(recipient || account.address);
  const lamports = Math.floor(amountSol * LAMPORTS_PER_SOL);
  const latest = await connection.getLatestBlockhash("confirmed");

  const tx = new Transaction({ feePayer: fromPubkey, recentBlockhash: latest.blockhash }).add(
    SystemProgram.transfer({ fromPubkey, toPubkey, lamports })
  );

  const unsignedBytes = tx.serialize({
    requireAllSignatures: false,
    verifySignatures: false
  });

  const signFeature = wallet.features["solana:signTransaction"];
  if (!signFeature) {
    throw new Error("Wallet missing solana:signTransaction feature.");
  }

  const outputs = await signFeature.signTransaction({
    account,
    chain: CHAIN,
    transaction: unsignedBytes
  });

  const signedBytes = outputs?.[0]?.signedTransaction;
  if (!signedBytes) {
    throw new Error("Wallet did not return signed transaction bytes.");
  }

  const signature = await connection.sendRawTransaction(signedBytes, { skipPreflight: false });
  await connection.confirmTransaction(
    {
      signature,
      blockhash: latest.blockhash,
      lastValidBlockHeight: latest.lastValidBlockHeight
    },
    "confirmed"
  );

  return { signature };
}

export function explorerTxUrl(signature: string): string {
  return `https://explorer.solana.com/tx/${signature}?cluster=devnet`;
}
