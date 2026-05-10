import { useCallback } from "react";
import useSWR from "swr";

import { address as toAddress } from "@solana/kit";

import { solanaRpc } from "./sol-client";
import { useAppState } from "./app-state";
import { useLiveBalance, type LiveConnectionState } from "./use-live-balance";

const BALANCE_REFRESH_MS = 120_000;
const LAMPORTS_PER_SOL = 1_000_000_000n;

export interface WalletSnapshot {
  pairedPubkey: string | null;
  solLamports: bigint | null;
  solUiAmount: number | null;
  balanceLoading: boolean;
  balanceError: string | null;
  refreshBalance: () => void;
  liveState: LiveConnectionState;
  loading: boolean;
}

function describeError(error: unknown): string {
  if (!error) return "Unknown error.";
  if (error instanceof Error) return error.message;
  if (typeof error === "string") return error;
  try {
    return JSON.stringify(error);
  } catch {
    return String(error);
  }
}

async function fetchLamports(pubkey: string): Promise<bigint> {
  let addr;
  try {
    addr = toAddress(pubkey);
  } catch (error) {
    throw new Error(`Invalid Solana address stored: ${describeError(error)}`);
  }

  try {
    const result = await solanaRpc.getBalance(addr).send();
    return result.value;
  } catch (error) {
    throw new Error(`RPC balance fetch failed: ${describeError(error)}`);
  }
}

export function useWallet(): WalletSnapshot {
  const { pairedPubkey, loading } = useAppState();

  const { data: lamports, error: balanceError, isValidating, mutate } = useSWR(
    pairedPubkey ? ["balance", pairedPubkey] : null,
    async ([, addr]: readonly [string, string]) => fetchLamports(addr),
    {
      refreshInterval: BALANCE_REFRESH_MS,
      revalidateOnFocus: true,
      shouldRetryOnError: true,
      errorRetryCount: 3,
      errorRetryInterval: 4_000
    }
  );

  const refreshBalance = useCallback(() => {
    void mutate();
  }, [mutate]);

  const liveState = useLiveBalance(pairedPubkey, refreshBalance);

  const solLamports = lamports ?? null;
  const solUiAmount =
    solLamports !== null ? Number(solLamports) / Number(LAMPORTS_PER_SOL) : null;

  return {
    pairedPubkey,
    solLamports,
    solUiAmount,
    balanceLoading: Boolean(pairedPubkey && isValidating),
    balanceError: balanceError ? describeError(balanceError) : null,
    refreshBalance,
    liveState,
    loading
  };
}
