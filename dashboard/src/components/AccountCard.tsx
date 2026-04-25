import { useState } from "react";
import type { CompanyAccount } from "../lib/squads";
import type { DiscoveredWallet, WalletAccount } from "../lib/wallet";
import { copy, shortId } from "../lib/format";
import { ViewAccountModal } from "./ViewAccountModal";
import { NewPaymentModal } from "./NewPaymentModal";

export function AccountCard({
  account,
  currentWallet,
  wallet,
  walletAccount,
}: {
  account: CompanyAccount;
  currentWallet: string | null;
  wallet: DiscoveredWallet | null;
  walletAccount: WalletAccount | null;
}) {
  const [copied, setCopied] = useState(false);
  const [showView, setShowView] = useState(false);
  const [showPayment, setShowPayment] = useState(false);
  const canPay = wallet !== null && walletAccount !== null;

  async function onCopy() {
    if (await copy(account.accountId)) {
      setCopied(true);
      setTimeout(() => setCopied(false), 1200);
    }
  }

  return (
    <div
      className="rounded-lg border p-5 transition-colors hover:border-[var(--color-border-strong)]"
      style={{ background: "var(--color-surface)", borderColor: "var(--color-border)" }}
    >
      <div className="flex items-start justify-between mb-4">
        <div className="min-w-0">
          <div className="text-[11px] uppercase tracking-wider mb-1" style={{ color: "var(--color-dim)" }}>
            Company account
          </div>
          {account.label ? (
            <>
              <h3 className="text-base font-semibold truncate" style={{ color: "var(--color-fg)" }}>
                {account.label}
              </h3>
              <button
                onClick={onCopy}
                className="font-mono text-xs mt-0.5 hover:opacity-80 inline-flex items-center gap-1.5"
                style={{ color: "var(--color-muted)" }}
                title="Copy full ID"
              >
                {shortId(account.accountId, 4, 4)}
                <CopyIcon copied={copied} />
              </button>
            </>
          ) : (
            <button
              onClick={onCopy}
              className="font-mono text-sm hover:opacity-80 inline-flex items-center gap-2"
              style={{ color: "var(--color-fg)" }}
              title="Copy full ID"
            >
              {shortId(account.accountId, 6, 6)}
              <CopyIcon copied={copied} />
            </button>
          )}
        </div>

        <span
          className="text-[11px] px-2 py-1 rounded-full shrink-0 ml-2"
          style={{ background: "var(--color-accent-soft)", color: "var(--color-accent)" }}
        >
          {account.approvalsRequired}-of-{account.approverCount}
        </span>
      </div>

      <Stat
        items={[
          { label: "Approvals required", value: String(account.approvalsRequired) },
          { label: "Total approvers", value: String(account.approverCount) },
          { label: "Pending payments", value: "0" },
        ]}
      />

      <div className="flex gap-2 mt-5">
        <button
          onClick={() => setShowPayment(true)}
          disabled={!canPay}
          className="flex-1 text-sm font-medium px-3 py-2 rounded-md disabled:opacity-50 disabled:cursor-not-allowed"
          style={{ background: "var(--color-accent)", color: "#001721" }}
        >
          New payment
        </button>
        <button
          onClick={() => setShowView(true)}
          className="text-sm px-3 py-2 rounded-md border hover:bg-white/[0.03]"
          style={{ borderColor: "var(--color-border-strong)", color: "var(--color-fg)" }}
        >
          View
        </button>
      </div>

      {showView && (
        <ViewAccountModal
          accountId={account.accountId}
          currentWallet={currentWallet}
          wallet={wallet}
          walletAccount={walletAccount}
          onClose={() => setShowView(false)}
        />
      )}

      {showPayment && wallet && walletAccount && (
        <NewPaymentModal
          multisigAccountId={account.accountId}
          multisigLabel={account.label}
          approvalsRequired={account.approvalsRequired}
          approverCount={account.approverCount}
          wallet={wallet}
          account={walletAccount}
          onClose={() => setShowPayment(false)}
        />
      )}
    </div>
  );
}

function Stat({ items }: { items: { label: string; value: string }[] }) {
  return (
    <div
      className="grid grid-cols-3 gap-3 rounded-md border p-3"
      style={{ borderColor: "var(--color-border)", background: "var(--color-bg)" }}
    >
      {items.map((it) => (
        <div key={it.label}>
          <div className="text-[10px] uppercase tracking-wider mb-1" style={{ color: "var(--color-dim)" }}>
            {it.label}
          </div>
          <div className="font-mono text-sm" style={{ color: "var(--color-fg)" }}>
            {it.value}
          </div>
        </div>
      ))}
    </div>
  );
}

function CopyIcon({ copied }: { copied: boolean }) {
  return copied ? (
    <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="var(--color-success)" strokeWidth="2.5" strokeLinecap="round" strokeLinejoin="round">
      <path d="M20 6L9 17l-5-5" />
    </svg>
  ) : (
    <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="var(--color-dim)" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
      <rect x="9" y="9" width="13" height="13" rx="2" />
      <path d="M5 15V5a2 2 0 012-2h10" />
    </svg>
  );
}
