/**
 * Local cache of accounts created from this dashboard. Mainnet public RPCs
 * (and even Helius) block `getProgramAccounts` on Squads, so without this
 * cache a freshly created account would have nowhere to surface in the UI.
 *
 * The cache is intentionally a write-only convenience — the source of truth
 * is on-chain. We re-fetch each saved account's state via `getAccountInfo`
 * on dashboard load so threshold/members can't drift from reality.
 */

const KEY = "faraday.dashboard.recentAccounts";
const MAX_ENTRIES = 50;

export interface SavedAccount {
  readonly accountId: string;     // Multisig PDA
  readonly signature: string;     // create-tx signature, useful for Solscan
  readonly createdAt: number;     // Date.now()
  readonly label?: string;        // user-entered memo at create time
}

export function readAccounts(): SavedAccount[] {
  try {
    const raw = localStorage.getItem(KEY);
    if (!raw) return [];
    const parsed = JSON.parse(raw);
    return Array.isArray(parsed) ? parsed.filter(isValid) : [];
  } catch {
    return [];
  }
}

export function saveAccount(entry: SavedAccount): void {
  const all = readAccounts();
  const prior = all.find((a) => a.accountId === entry.accountId);
  // Preserve a previously-recorded label when discovery re-saves without one,
  // so the user's memo never gets overwritten by a label-less re-discovery.
  const merged: SavedAccount = { ...entry, label: entry.label ?? prior?.label };
  const others = all.filter((a) => a.accountId !== entry.accountId);
  const next = [merged, ...others].slice(0, MAX_ENTRIES);
  try {
    localStorage.setItem(KEY, JSON.stringify(next));
  } catch {
    // Quota exceeded or storage disabled — degrade silently. The account
    // is still on-chain; only the local convenience list is missing it.
  }
}

export function removeAccount(accountId: string): void {
  const next = readAccounts().filter((a) => a.accountId !== accountId);
  try {
    localStorage.setItem(KEY, JSON.stringify(next));
  } catch {
    // ignored
  }
}

export function getLabel(accountId: string): string | undefined {
  return readAccounts().find((a) => a.accountId === accountId)?.label;
}

function isValid(a: unknown): a is SavedAccount {
  return (
    typeof a === "object" &&
    a !== null &&
    typeof (a as SavedAccount).accountId === "string" &&
    typeof (a as SavedAccount).signature === "string" &&
    typeof (a as SavedAccount).createdAt === "number"
  );
}
