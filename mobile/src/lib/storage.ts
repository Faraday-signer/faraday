import AsyncStorage from "@react-native-async-storage/async-storage";

import type { AppState } from "./types";

const STATE_KEY = "faraday:state:v1";

const DEFAULT_STATE: AppState = {
  pairedPubkey: null,
  approvedOrigins: []
};

async function readState(): Promise<AppState> {
  const raw = await AsyncStorage.getItem(STATE_KEY);
  if (!raw) return DEFAULT_STATE;
  try {
    const parsed = JSON.parse(raw) as Partial<AppState>;
    return {
      pairedPubkey: parsed.pairedPubkey ?? null,
      approvedOrigins: Array.isArray(parsed.approvedOrigins)
        ? [...new Set(parsed.approvedOrigins)]
        : []
    };
  } catch {
    return DEFAULT_STATE;
  }
}

async function writeState(next: AppState): Promise<AppState> {
  const normalized: AppState = {
    pairedPubkey: next.pairedPubkey,
    approvedOrigins: [...new Set(next.approvedOrigins)]
  };
  await AsyncStorage.setItem(STATE_KEY, JSON.stringify(normalized));
  return normalized;
}

export async function getAppState(): Promise<AppState> {
  return readState();
}

export async function setPairedPubkey(pubkey: string): Promise<AppState> {
  const current = await readState();
  return writeState({ ...current, pairedPubkey: pubkey.trim() });
}

export async function clearPairedPubkey(): Promise<AppState> {
  const current = await readState();
  return writeState({ ...current, pairedPubkey: null });
}

export async function approveOrigin(origin: string): Promise<AppState> {
  const current = await readState();
  return writeState({
    ...current,
    approvedOrigins: [...current.approvedOrigins, origin]
  });
}

export async function revokeOrigin(origin: string): Promise<AppState> {
  const current = await readState();
  return writeState({
    ...current,
    approvedOrigins: current.approvedOrigins.filter((o) => o !== origin)
  });
}

export async function clearApprovedOrigins(): Promise<AppState> {
  const current = await readState();
  return writeState({ ...current, approvedOrigins: [] });
}
