import type { ExtensionState } from "./types";

const STATE_KEY = "faraday:state:v1";

const DEFAULT_STATE: ExtensionState = {
  pairedPubkey: null,
  approvedOrigins: [],
  nonceAccounts: {}
};

function storageGet<T>(key: string, fallback: T): Promise<T> {
  return new Promise((resolve) => {
    chrome.storage.local.get([key], (items) => {
      const value = items[key] as T | undefined;
      resolve(value ?? fallback);
    });
  });
}

function storageSet<T>(key: string, value: T): Promise<void> {
  return new Promise((resolve) => {
    chrome.storage.local.set({ [key]: value }, () => resolve());
  });
}

export async function getExtensionState(): Promise<ExtensionState> {
  const state = await storageGet<ExtensionState>(STATE_KEY, DEFAULT_STATE);

  return {
    pairedPubkey: state.pairedPubkey ?? null,
    approvedOrigins: Array.isArray(state.approvedOrigins) ? [...new Set(state.approvedOrigins)] : [],
    nonceAccounts:
      state.nonceAccounts && typeof state.nonceAccounts === "object"
        ? { ...state.nonceAccounts }
        : {}
  };
}

async function setExtensionState(next: ExtensionState): Promise<ExtensionState> {
  const normalized: ExtensionState = {
    pairedPubkey: next.pairedPubkey,
    approvedOrigins: [...new Set(next.approvedOrigins)],
    nonceAccounts: { ...next.nonceAccounts }
  };
  await storageSet(STATE_KEY, normalized);
  return normalized;
}

/** Stored durable-nonce account for a wallet, or null if none provisioned. */
export async function getNonceAccount(walletPubkey: string): Promise<string | null> {
  const current = await getExtensionState();
  return current.nonceAccounts[walletPubkey] ?? null;
}

/** Record the durable-nonce account provisioned for a wallet. */
export async function setNonceAccount(
  walletPubkey: string,
  nonceAccount: string
): Promise<ExtensionState> {
  const current = await getExtensionState();
  return setExtensionState({
    ...current,
    nonceAccounts: { ...current.nonceAccounts, [walletPubkey]: nonceAccount }
  });
}

export async function setPairedPubkey(pubkey: string): Promise<ExtensionState> {
  const current = await getExtensionState();
  return setExtensionState({
    ...current,
    pairedPubkey: pubkey.trim()
  });
}

export async function clearPairedPubkey(): Promise<ExtensionState> {
  const current = await getExtensionState();
  return setExtensionState({
    ...current,
    pairedPubkey: null
  });
}

export async function approveOrigin(origin: string): Promise<ExtensionState> {
  const current = await getExtensionState();
  return setExtensionState({
    ...current,
    approvedOrigins: [...current.approvedOrigins, origin]
  });
}

export async function revokeOrigin(origin: string): Promise<ExtensionState> {
  const current = await getExtensionState();
  return setExtensionState({
    ...current,
    approvedOrigins: current.approvedOrigins.filter((item: string) => item !== origin)
  });
}

export async function clearApprovedOrigins(): Promise<ExtensionState> {
  const current = await getExtensionState();
  return setExtensionState({
    ...current,
    approvedOrigins: []
  });
}
