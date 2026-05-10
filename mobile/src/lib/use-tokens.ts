import AsyncStorage from "@react-native-async-storage/async-storage";
import { useCallback, useEffect, useState } from "react";
import useSWR from "swr";

import { fetchJupiterPrices, fetchOwnedTokens, WSOL_MINT, type Token } from "./tokens";

const TOKENS_REFRESH_MS = 60_000;
const SOL_PRICE_REFRESH_MS = 30_000;
const SETTINGS_KEY = "faraday:settings:v1";

export interface TokenSettings {
  showUnverified: boolean;
}

const DEFAULT_SETTINGS: TokenSettings = { showUnverified: false };

async function settingsGet(): Promise<TokenSettings> {
  const raw = await AsyncStorage.getItem(SETTINGS_KEY);
  if (!raw) return DEFAULT_SETTINGS;
  try {
    const parsed = JSON.parse(raw) as Partial<TokenSettings>;
    return { ...DEFAULT_SETTINGS, ...parsed };
  } catch {
    return DEFAULT_SETTINGS;
  }
}

async function settingsSet(next: TokenSettings): Promise<void> {
  await AsyncStorage.setItem(SETTINGS_KEY, JSON.stringify(next));
}

export function useTokenSettings(): {
  settings: TokenSettings;
  loading: boolean;
  setShowUnverified: (next: boolean) => Promise<void>;
} {
  const [settings, setSettings] = useState<TokenSettings>(DEFAULT_SETTINGS);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    let cancelled = false;
    void (async () => {
      const loaded = await settingsGet();
      if (cancelled) return;
      setSettings(loaded);
      setLoading(false);
    })();
    return () => {
      cancelled = true;
    };
  }, []);

  const setShowUnverified = useCallback(async (next: boolean) => {
    setSettings((prev) => ({ ...prev, showUnverified: next }));
    await settingsSet({ ...DEFAULT_SETTINGS, showUnverified: next });
  }, []);

  return { settings, loading, setShowUnverified };
}

export interface TokenListSnapshot {
  tokens: Token[];
  hiddenUnverifiedCount: number;
  loading: boolean;
  error: string | null;
  refresh: () => void;
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

function compareForSort(a: Token, b: Token): number {
  if (a.verified !== b.verified) return a.verified ? -1 : 1;
  const aHasUsd = a.usdValue !== null;
  const bHasUsd = b.usdValue !== null;
  if (aHasUsd !== bHasUsd) return aHasUsd ? -1 : 1;
  if (aHasUsd && bHasUsd) return (b.usdValue ?? 0) - (a.usdValue ?? 0);
  return b.amountUi - a.amountUi;
}

export function useSolPrice(): { priceUsd: number | null } {
  const { data } = useSWR(
    "sol-price",
    async () => {
      const prices = await fetchJupiterPrices([WSOL_MINT]);
      return prices.get(WSOL_MINT) ?? null;
    },
    {
      refreshInterval: SOL_PRICE_REFRESH_MS,
      revalidateOnFocus: true,
      keepPreviousData: true,
      shouldRetryOnError: true,
      errorRetryCount: 2
    }
  );

  return { priceUsd: data ?? null };
}

export function useTokens(pairedPubkey: string | null): TokenListSnapshot {
  const { settings } = useTokenSettings();

  const { data, error, isValidating, mutate } = useSWR(
    pairedPubkey ? ["tokens", pairedPubkey] : null,
    async ([, addr]: readonly [string, string]) => fetchOwnedTokens(addr),
    {
      refreshInterval: TOKENS_REFRESH_MS,
      revalidateOnFocus: true,
      shouldRetryOnError: true,
      errorRetryCount: 2,
      errorRetryInterval: 5_000,
      keepPreviousData: true
    }
  );

  const refresh = useCallback(() => {
    void mutate();
  }, [mutate]);

  const all = data ?? [];
  const verified = all.filter((t) => t.verified);
  const unverified = all.filter((t) => !t.verified);

  const visible = settings.showUnverified ? all : verified;
  const sorted = [...visible].sort(compareForSort);

  return {
    tokens: sorted,
    hiddenUnverifiedCount: settings.showUnverified ? 0 : unverified.length,
    loading: !data && Boolean(pairedPubkey) && isValidating,
    error: error ? describeError(error) : null,
    refresh
  };
}
