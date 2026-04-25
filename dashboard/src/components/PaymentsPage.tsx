import { useEffect, useMemo, useState } from "react";
import { PublicKey } from "@solana/web3.js";
import type { CompanyAccount } from "../lib/squads";
import { type PendingPayment, fetchAllPayments } from "../lib/payments";
import type { DiscoveredWallet, WalletAccount } from "../lib/wallet";
import { PaymentRow } from "./PaymentRow";
import { EmptyState } from "./EmptyState";

interface Props {
  accounts: CompanyAccount[] | null;
  currentWallet: string | null;
  wallet: DiscoveredWallet | null;
  walletAccount: WalletAccount | null;
}

interface JoinedPayment {
  payment: PendingPayment;
  account: CompanyAccount;
}

type FilterKey = "all" | "active" | "paid" | "rejected";

const FILTERS: { key: FilterKey; label: string; matches: (status: PendingPayment["status"]) => boolean }[] = [
  { key: "all",      label: "All",      matches: () => true },
  { key: "active",   label: "Pending",  matches: (s) => s === "Active" || s === "Approved" || s === "Executing" || s === "Draft" },
  { key: "paid",     label: "Paid",     matches: (s) => s === "Executed" },
  { key: "rejected", label: "Rejected", matches: (s) => s === "Rejected" || s === "Cancelled" },
];

export function PaymentsPage({ accounts, currentWallet, wallet, walletAccount }: Props) {
  const [loading, setLoading] = useState(false);
  const [payments, setPayments] = useState<JoinedPayment[] | null>(null);
  const [filter, setFilter] = useState<FilterKey>("all");
  const [reloadKey, setReloadKey] = useState(0);

  useEffect(() => {
    if (!accounts) { setPayments(null); return; }
    let cancelled = false;
    setLoading(true);
    Promise.all(
      accounts.map(async (acct) => {
        const list = await fetchAllPayments(new PublicKey(acct.accountId)).catch(() => [] as PendingPayment[]);
        return list.map((p) => ({ payment: p, account: acct }));
      }),
    )
      .then((groups) => {
        if (cancelled) return;
        const flat = groups.flat();
        // Newest first across accounts; ties broken by transactionIndex.
        flat.sort((a, b) => {
          const at = a.payment.createdAt ?? 0;
          const bt = b.payment.createdAt ?? 0;
          if (bt !== at) return bt - at;
          return b.payment.transactionIndex - a.payment.transactionIndex;
        });
        setPayments(flat);
      })
      .finally(() => { if (!cancelled) setLoading(false); });
    return () => { cancelled = true; };
  }, [accounts, reloadKey]);

  const counts = useMemo(() => {
    const out: Record<FilterKey, number> = { all: 0, active: 0, paid: 0, rejected: 0 };
    for (const p of payments ?? []) {
      out.all += 1;
      for (const f of FILTERS) if (f.key !== "all" && f.matches(p.payment.status)) out[f.key] += 1;
    }
    return out;
  }, [payments]);

  const filtered = (payments ?? []).filter((p) =>
    FILTERS.find((f) => f.key === filter)!.matches(p.payment.status),
  );

  if (!accounts || accounts.length === 0) {
    return (
      <EmptyState
        icon={<IconWallet />}
        title="No accounts yet"
        description="Once you create or join a company account, payments to and from it will land here."
      />
    );
  }

  return (
    <div>
      <div className="flex items-center gap-1.5 mb-4 flex-wrap">
        {FILTERS.map((f) => {
          const active = f.key === filter;
          return (
            <button
              key={f.key}
              onClick={() => setFilter(f.key)}
              className="text-xs px-3 py-1.5 rounded-full transition-colors"
              style={{
                background: active ? "var(--color-accent-soft)" : "transparent",
                color: active ? "var(--color-accent)" : "var(--color-muted)",
                border: `1px solid ${active ? "transparent" : "var(--color-border-strong)"}`,
              }}
            >
              {f.label}
              {payments && (
                <span className="ml-1.5 opacity-70">{counts[f.key]}</span>
              )}
            </button>
          );
        })}
      </div>

      {loading && payments === null && <SkeletonList />}

      {payments !== null && filtered.length === 0 && (
        <EmptyState
          icon={<IconWallet />}
          title={filter === "all" ? "No payments yet" : "Nothing in this filter"}
          description={
            filter === "all"
              ? "Create a payment from any of your company accounts to get started."
              : `Switch filters or check back as activity moves through.`
          }
        />
      )}

      {filtered.length > 0 && (
        <div className="space-y-3">
          {filtered.map(({ payment, account }) => (
            <PaymentRow
              key={`${account.accountId}:${payment.transactionIndex}`}
              payment={payment}
              threshold={account.approvalsRequired}
              multisigAccountId={account.accountId}
              accountLabel={account.label ?? shortAccountId(account.accountId)}
              currentWallet={currentWallet}
              isApprover={!!currentWallet}  // PaymentsPage only lists accounts the user is in
              wallet={wallet}
              walletAccount={walletAccount}
              onMutated={() => setReloadKey((k) => k + 1)}
            />
          ))}
        </div>
      )}
    </div>
  );
}

function SkeletonList() {
  return (
    <div className="space-y-3">
      {[0, 1, 2].map((i) => (
        <div
          key={i}
          className="rounded-md p-3 h-[120px] animate-pulse"
          style={{ background: "var(--color-bg)", border: `1px solid var(--color-border)` }}
        />
      ))}
    </div>
  );
}

function shortAccountId(id: string): string {
  return id.length > 12 ? `${id.slice(0, 6)}…${id.slice(-4)}` : id;
}

function IconWallet() {
  return (
    <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
      <path d="M4 12h16 M14 6l6 6-6 6" />
    </svg>
  );
}
