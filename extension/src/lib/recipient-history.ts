//! Rolling history of addresses the user has sent SOL to. Powers the
//! "lookalike destination" risk detector — a paste/typo attack where the
//! attacker dusts you with a near-duplicate (same first/last 4 chars but
//! a different middle) so a future copy-from-history selects their
//! address instead of the real one.
//!
//! Storage layout: a single key `faraday:recipients:v1` in
//! `chrome.storage.local`, holding up to `MAX_HISTORY` base58 pubkeys
//! ordered most-recent-first. Reads survive worker eviction; the in-flight
//! cost of recording is one debounced write.

const STORAGE_KEY = "faraday:recipients:v1";
const MAX_HISTORY = 50;

function storageGet(): Promise<string[]> {
  return new Promise((resolve) => {
    chrome.storage.local.get([STORAGE_KEY], (items) => {
      const raw = items[STORAGE_KEY];
      if (Array.isArray(raw)) {
        resolve(raw.filter((s): s is string => typeof s === "string"));
      } else {
        resolve([]);
      }
    });
  });
}

function storageSet(list: string[]): Promise<void> {
  return new Promise((resolve) => {
    chrome.storage.local.set({ [STORAGE_KEY]: list }, () => resolve());
  });
}

export async function getRecipientHistory(): Promise<string[]> {
  return storageGet();
}

/**
 * Add `address` to the head of the history. De-duplicates (re-sending to
 * the same address moves it to the front), trims to MAX_HISTORY.
 */
export async function recordRecipient(address: string): Promise<void> {
  const trimmed = address.trim();
  if (trimmed.length === 0) return;
  const current = await storageGet();
  const next = [trimmed, ...current.filter((a) => a !== trimmed)].slice(0, MAX_HISTORY);
  await storageSet(next);
}

/** Wipe the history. Exposed for the Settings → Reset surface. */
export async function clearRecipientHistory(): Promise<void> {
  await storageSet([]);
}
