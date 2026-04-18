import { useCallback, useEffect, useState } from "react";
import useSWR from "swr";

import { address as toAddress } from "@solana/kit";

import { sendRuntimeMessage } from "./runtime";
import { solanaRpc } from "./sol-client";
import type { ExtensionState } from "./types";
import { useLiveBalance, type LiveConnectionState } from "./use-live-balance";

/** SWR poll interval — backstop for when the WebSocket subscription is down. */
const BALANCE_REFRESH_MS = 120_000;
const LAMPORTS_PER_SOL = 1_000_000_000n;

export interface WalletSnapshot {
  pairedPubkey: string | null;
  solLamports: bigint | null;
  solUiAmount: number | null;
  balanceLoading: boolean;
  balanceError: string | null;
  /** Force a refresh of the balance. Useful for pull-to-refresh or retry. */
  refreshBalance: () => void;
  /** WebSocket subscription status for live balance pushes. */
  liveState: LiveConnectionState;
  /** True while the initial extension-state fetch is pending. */
  loading: boolean;
  /** Extension-state load error (couldn't reach background, etc.). */
  error: string | null;
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
  // toAddress() throws synchronously if pubkey isn't a valid Solana address.
  // Wrapped so SWR surfaces it as a normal fetch error instead of blowing up
  // the render.
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
  const [pairedPubkey, setPairedPubkey] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let cancelled = false;
    void (async () => {
      const response = await sendRuntimeMessage<ExtensionState>({ type: "faraday:get-state" });
      if (cancelled) return;
      if (response.ok) {
        setPairedPubkey(response.data.pairedPubkey);
      } else {
        setError(response.error);
      }
      setLoading(false);
    })();
    return () => {
      cancelled = true;
    };
  }, []);

  const {
    data: lamports,
    error: balanceError,
    isValidating,
    mutate
  } = useSWR(
    pairedPubkey ? ["balance", pairedPubkey] : null,
    async ([, addr]: readonly [string, string]) => fetchLamports(addr),
    {
      // Slower backstop poll: the WebSocket subscription below is the fast
      // path, and each SWR refetch hits the HTTP endpoint. Keep the safety
      // net but don't burn rate limit.
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

  // Live push notifications via WebSocket. Each on-chain change triggers
  // an SWR revalidation so `lamports` stays current within 1–2 seconds
  // instead of the poll interval.
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
    loading,
    error
  };
}

export function formatSol(amount: number | null): string {
  if (amount === null) return "—";
  if (amount === 0) return "0";
  if (amount < 0.000001) return amount.toExponential(2);
  if (amount < 1) return amount.toFixed(6);
  if (amount < 100) return amount.toFixed(4);
  return amount.toLocaleString("en-US", { maximumFractionDigits: 2 });
}
