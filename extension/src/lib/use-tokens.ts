//! SWR-backed hook for the SPL token list shown on the home screen.
//!
//! Reads the paired pubkey, fetches tokens via `fetchOwnedTokens`, applies
//! the user's "show unverified" preference, and returns a sorted list with
//! the standard SWR loading/error knobs.
//!
//! Sort order (after the unverified filter):
//!   1. Verified tokens with a USD value, by USD value descending.
//!   2. Verified tokens without a USD value, by amountUi descending.
//!   3. Unverified tokens (only when the toggle is on), same scheme.

import { useCallback, useEffect, useState } from "react";
import useSWR from "swr";

import {
  fetchJupiterPrices,
  fetchOwnedTokens,
  WSOL_MINT,
  type Token,
} from "./tokens";

const TOKENS_REFRESH_MS = 60_000;
const SOL_PRICE_REFRESH_MS = 30_000;
const SETTINGS_KEY = "faraday:settings:v1";

export interface TokenSettings {
  /** When true, unverified tokens are shown alongside verified ones. */
  showUnverified: boolean;
}

const DEFAULT_SETTINGS: TokenSettings = {
  showUnverified: false,
};

function settingsGet(): Promise<TokenSettings> {
  return new Promise((resolve) => {
    chrome.storage.local.get([SETTINGS_KEY], (items) => {
      const raw = items[SETTINGS_KEY] as Partial<TokenSettings> | undefined;
      resolve({
        ...DEFAULT_SETTINGS,
        ...(raw ?? {}),
      });
    });
  });
}

function settingsSet(next: TokenSettings): Promise<void> {
  return new Promise((resolve) => {
    chrome.storage.local.set({ [SETTINGS_KEY]: next }, () => resolve());
  });
}

/**
 * Hook returning the user's token preferences. Persists to chrome.storage.local
 * directly — these are sidepanel-only UI preferences, no background round-trip
 * is needed.
 */
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

    function onChanged(
      changes: Record<string, chrome.storage.StorageChange>,
      area: chrome.storage.AreaName
    ) {
      if (area !== "local" || !changes[SETTINGS_KEY]) return;
      const next = changes[SETTINGS_KEY].newValue as
        | Partial<TokenSettings>
        | undefined;
      setSettings({ ...DEFAULT_SETTINGS, ...(next ?? {}) });
    }
    chrome.storage.onChanged.addListener(onChanged);

    return () => {
      cancelled = true;
      chrome.storage.onChanged.removeListener(onChanged);
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
  /** Count of tokens hidden by the unverified filter (for the toggle hint). */
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
  // Verified first, unverified last.
  if (a.verified !== b.verified) return a.verified ? -1 : 1;
  // Then by USD value desc; tokens without a price drop below.
  const aHasUsd = a.usdValue !== null;
  const bHasUsd = b.usdValue !== null;
  if (aHasUsd !== bHasUsd) return aHasUsd ? -1 : 1;
  if (aHasUsd && bHasUsd) return (b.usdValue ?? 0) - (a.usdValue ?? 0);
  return b.amountUi - a.amountUi;
}

/**
 * Hook returning the current SOL/USD price from Jupiter. Refreshes every
 * 30s in the background. Returns null while loading or on failure — the
 * caller should hide the price line rather than show a stale or zero value.
 */
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
      errorRetryCount: 2,
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
      keepPreviousData: true,
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
    refresh,
  };
}
