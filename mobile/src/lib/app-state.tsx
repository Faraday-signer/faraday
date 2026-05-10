import { createContext, useCallback, useContext, useEffect, useState, type ReactNode } from "react";

import {
  approveOrigin as storeApproveOrigin,
  clearApprovedOrigins as storeClearApprovedOrigins,
  clearPairedPubkey as storeClearPairedPubkey,
  getAppState,
  revokeOrigin as storeRevokeOrigin,
  setPairedPubkey as storeSetPairedPubkey
} from "./storage";
import type { AppState } from "./types";

interface AppStateContextValue extends AppState {
  loading: boolean;
  setPairedPubkey: (pubkey: string) => Promise<void>;
  clearPairedPubkey: () => Promise<void>;
  approveOrigin: (origin: string) => Promise<void>;
  revokeOrigin: (origin: string) => Promise<void>;
  clearApprovedOrigins: () => Promise<void>;
  refresh: () => Promise<void>;
}

const DEFAULT: AppState = { pairedPubkey: null, approvedOrigins: [] };

const AppStateContext = createContext<AppStateContextValue | null>(null);

export function AppStateProvider({ children }: { children: ReactNode }) {
  const [state, setState] = useState<AppState>(DEFAULT);
  const [loading, setLoading] = useState(true);

  const refresh = useCallback(async () => {
    const next = await getAppState();
    setState(next);
  }, []);

  useEffect(() => {
    void (async () => {
      await refresh();
      setLoading(false);
    })();
  }, [refresh]);

  const setPairedPubkey = useCallback(async (pubkey: string) => {
    const next = await storeSetPairedPubkey(pubkey);
    setState(next);
  }, []);

  const clearPairedPubkey = useCallback(async () => {
    const next = await storeClearPairedPubkey();
    setState(next);
  }, []);

  const approveOrigin = useCallback(async (origin: string) => {
    const next = await storeApproveOrigin(origin);
    setState(next);
  }, []);

  const revokeOrigin = useCallback(async (origin: string) => {
    const next = await storeRevokeOrigin(origin);
    setState(next);
  }, []);

  const clearApprovedOrigins = useCallback(async () => {
    const next = await storeClearApprovedOrigins();
    setState(next);
  }, []);

  return (
    <AppStateContext.Provider
      value={{
        ...state,
        loading,
        setPairedPubkey,
        clearPairedPubkey,
        approveOrigin,
        revokeOrigin,
        clearApprovedOrigins,
        refresh
      }}
    >
      {children}
    </AppStateContext.Provider>
  );
}

export function useAppState(): AppStateContextValue {
  const ctx = useContext(AppStateContext);
  if (!ctx) throw new Error("useAppState used outside AppStateProvider");
  return ctx;
}
