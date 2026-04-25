import { useEffect, useState } from "react";
import { PublicKey } from "@solana/web3.js";
import {
  CHAIN,
  type DiscoveredWallet,
  type WalletAccount,
  connectWallet,
  disconnectWallet,
  discoverSolanaWallets,
} from "./lib/wallet";
import {
  type CompanyAccount,
  fetchAccountById,
  fetchIndexedMultisigs,
  findKnownMultisigs,
  reportMultisig,
} from "./lib/squads";
import { readAccounts as readSavedAccounts, saveAccount } from "./lib/recentAccounts";
import * as multisig from "@sqds/multisig";
import { connection } from "./lib/squads";
import { Sidebar, type Section } from "./components/Sidebar";
import { Topbar } from "./components/Topbar";
import { AccountCard } from "./components/AccountCard";
import { EmptyState } from "./components/EmptyState";
import { CreateAccountModal } from "./components/CreateAccountModal";
import { AddByIdModal } from "./components/AddByIdModal";
import { PaymentsPage } from "./components/PaymentsPage";

export default function App() {
  const [wallets, setWallets] = useState<DiscoveredWallet[]>([]);
  const [active, setActive] = useState<DiscoveredWallet | null>(null);
  const [account, setAccount] = useState<WalletAccount | null>(null);
  const pubkey = account?.address ?? null;
  const [section, setSection] = useState<Section>("accounts");

  const [accounts, setAccounts] = useState<CompanyAccount[] | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [showCreate, setShowCreate] = useState(false);
  const [showAddById, setShowAddById] = useState(false);

  useEffect(() => {
    const { wallets, subscribe } = discoverSolanaWallets();
    setWallets(wallets);
    return subscribe(setWallets);
  }, []);

  async function onConnect(w: DiscoveredWallet) {
    setError(null);
    try {
      const acct = await connectWallet(w);
      setActive(w);
      setAccount(acct);
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    }
  }

  async function onDisconnect() {
    if (active) {
      try { await disconnectWallet(active); } catch { /* ignore */ }
    }
    setActive(null);
    setAccount(null);
    setAccounts(null);
  }

  async function refreshAccounts() {
    if (!pubkey) return;
    setLoading(true);
    setError(null);
    try {
      const me = new PublicKey(pubkey);

      // Three discovery sources, in order from cheapest to most expensive:
      //   1. Cloudflare indexer (the proper member -> multisigs reverse
      //      index — catches passive approvers who haven't signed yet)
      //   2. Tx-history walk (catches creators + active interactors;
      //      doubles as a backfill source when the indexer is empty)
      //   3. Local cache (hydrates instantly, gets refreshed by 1+2)
      const [indexed, viaTxHistory] = await Promise.all([
        fetchIndexedMultisigs(me).catch(() => []),
        findKnownMultisigs(me).catch(() => []),
      ]);

      // Merge by accountId, preferring entries with a label.
      const merged = new Map<string, { signature: string; label?: string }>();
      for (const m of [...indexed, ...viaTxHistory]) {
        const prior = merged.get(m.accountId);
        merged.set(m.accountId, {
          signature: m.signature || prior?.signature || "",
          label: m.label ?? prior?.label,
        });
      }
      for (const [accountId, info] of merged) {
        saveAccount({ accountId, signature: info.signature, label: info.label, createdAt: Date.now() });
      }

      // Backfill the indexer with anything tx-history found (so the next
      // load is a single GET instead of a 50-tx walk).
      for (const m of viaTxHistory) {
        if (indexed.some((i) => i.accountId === m.accountId)) continue;
        try {
          const ms = await multisig.accounts.Multisig.fromAccountAddress(
            connection,
            new PublicKey(m.accountId),
          );
          void reportMultisig(
            m.accountId,
            ms.members.map((mem) => mem.key.toBase58()),
            { signature: m.signature, label: m.label },
          );
        } catch {
          // skip — already persisted locally regardless
        }
      }

      // Hydrate everything in localStorage with live on-chain state.
      const cached = readSavedAccounts();
      const rows = await Promise.all(
        cached.map(async (saved): Promise<CompanyAccount | null> => {
          const live = await fetchAccountById(new PublicKey(saved.accountId));
          return live ? { ...live, label: saved.label } : null;
        }),
      );
      setAccounts(rows.filter((r): r is CompanyAccount => r !== null));
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    } finally {
      setLoading(false);
    }
  }

  // Auto-load accounts the first time the user connects.
  useEffect(() => {
    if (pubkey && accounts === null && !loading) refreshAccounts();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [pubkey]);

  return (
    <div className="min-h-screen flex" style={{ background: "var(--color-bg)" }}>
      <Sidebar current={section} onChange={setSection} />

      <div className="flex-1 flex flex-col min-w-0">
        <Topbar
          wallets={wallets}
          pubkey={pubkey}
          walletName={active?.name ?? null}
          onConnect={onConnect}
          onDisconnect={onDisconnect}
        />

        <main className="flex-1 px-8 py-8 overflow-y-auto">
          <div className="max-w-6xl mx-auto">
            <div className="flex items-end justify-between mb-6">
              <div>
                <h1 className="text-2xl font-semibold tracking-tight" style={{ color: "var(--color-fg)" }}>
                  {section === "payments" ? "Payments" : "Accounts"}
                </h1>
                <p className="text-sm mt-1" style={{ color: "var(--color-muted)" }}>
                  {section === "payments"
                    ? "Every payment across your company accounts. Approve, execute, or review history."
                    : "Company accounts you're an approver on. Approvals are signed offline on your Faraday device."}
                </p>
              </div>
              {pubkey && section === "accounts" && (
                <div className="flex items-center gap-2">
                  <button
                    onClick={refreshAccounts}
                    disabled={loading}
                    className="text-xs px-3 py-1.5 rounded-md border disabled:opacity-50"
                    style={{ borderColor: "var(--color-border-strong)", color: "var(--color-muted)" }}
                  >
                    {loading ? "Refreshing…" : "Refresh"}
                  </button>
                  <button
                    onClick={() => setShowAddById(true)}
                    className="text-xs px-3 py-1.5 rounded-md border"
                    style={{ borderColor: "var(--color-border-strong)", color: "var(--color-fg)" }}
                  >
                    Add by ID
                  </button>
                  <button
                    onClick={() => setShowCreate(true)}
                    className="text-xs font-medium px-3 py-1.5 rounded-md"
                    style={{ background: "var(--color-accent)", color: "#001721" }}
                  >
                    + New account
                  </button>
                </div>
              )}
            </div>

            {error && (
              <div
                className="mb-4 px-4 py-3 rounded-md border text-sm"
                style={{ borderColor: "var(--color-danger)", color: "var(--color-danger)", background: "rgba(248, 81, 73, 0.06)" }}
              >
                {error}
              </div>
            )}

            {section === "payments" ? (
              !pubkey ? (
                <EmptyState
                  icon={<IconLock />}
                  title="Sign in to view payments"
                  description="Connect a Solana wallet to see payments across the company accounts you're a member of."
                />
              ) : (
                <PaymentsPage
                  accounts={accounts}
                  currentWallet={pubkey}
                  wallet={active}
                  walletAccount={account}
                />
              )
            ) : (
              <Body
                connected={!!pubkey}
                loading={loading}
                accounts={accounts}
                chain={CHAIN}
                currentWallet={pubkey}
                wallet={active}
                walletAccount={account}
                onCreate={() => setShowCreate(true)}
                onAddById={() => setShowAddById(true)}
              />
            )}
          </div>
        </main>
      </div>

      {showCreate && active && account && (
        <CreateAccountModal
          wallet={active}
          account={account}
          onClose={() => setShowCreate(false)}
          onCreated={() => { refreshAccounts(); }}
        />
      )}

      {showAddById && (
        <AddByIdModal
          onClose={() => setShowAddById(false)}
          onAdded={() => { setShowAddById(false); refreshAccounts(); }}
        />
      )}
    </div>
  );
}

function Body({
  connected,
  loading,
  accounts,
  currentWallet,
  wallet,
  walletAccount,
  onCreate,
  onAddById,
}: {
  connected: boolean;
  loading: boolean;
  accounts: CompanyAccount[] | null;
  chain: string;
  currentWallet: string | null;
  wallet: DiscoveredWallet | null;
  walletAccount: WalletAccount | null;
  onCreate: () => void;
  onAddById: () => void;
}) {
  if (!connected) {
    return (
      <EmptyState
        icon={<IconLock />}
        title="Sign in to view accounts"
        description="Connect a Solana wallet to see the company accounts where you're listed as an approver."
      />
    );
  }
  if (loading && accounts === null) {
    return <SkeletonGrid />;
  }
  if (accounts && accounts.length === 0) {
    return (
      <EmptyState
        icon={<IconWallet />}
        title="No accounts found"
        description="We didn't find any company accounts created from this wallet. If someone added you as an approver to one they created, paste the account ID to bring it in."
        cta={
          <div className="flex gap-2 justify-center">
            <button
              onClick={onAddById}
              className="text-sm px-4 py-2 rounded-md border"
              style={{ borderColor: "var(--color-border-strong)", color: "var(--color-fg)" }}
            >
              Add by ID
            </button>
            <button
              onClick={onCreate}
              className="text-sm font-medium px-4 py-2 rounded-md"
              style={{ background: "var(--color-accent)", color: "#001721" }}
            >
              + New account
            </button>
          </div>
        }
      />
    );
  }
  if (!accounts) return null;
  return (
    <div className="grid grid-cols-1 md:grid-cols-2 xl:grid-cols-3 gap-4">
      {accounts.map((a) => (
        <AccountCard
          key={a.accountId}
          account={a}
          currentWallet={currentWallet}
          wallet={wallet}
          walletAccount={walletAccount}
        />
      ))}
    </div>
  );
}

function SkeletonGrid() {
  return (
    <div className="grid grid-cols-1 md:grid-cols-2 xl:grid-cols-3 gap-4">
      {[0, 1, 2].map((i) => (
        <div
          key={i}
          className="rounded-lg border p-5 animate-pulse h-[200px]"
          style={{ background: "var(--color-surface)", borderColor: "var(--color-border)" }}
        />
      ))}
    </div>
  );
}

function IconLock() {
  return (
    <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
      <rect x="4" y="11" width="16" height="10" rx="2" />
      <path d="M8 11V7a4 4 0 018 0v4" />
    </svg>
  );
}
function IconWallet() {
  return (
    <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
      <rect x="3" y="6" width="18" height="13" rx="2" />
      <path d="M3 10h18 M16 14h2" />
    </svg>
  );
}
