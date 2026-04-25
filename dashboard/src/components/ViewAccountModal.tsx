import { useEffect, useState } from "react";
import { LAMPORTS_PER_SOL, PublicKey } from "@solana/web3.js";
import { type AccountDetails, fetchAccountDetails } from "../lib/squads";
import { type PendingPayment, fetchPendingPayments } from "../lib/payments";
import type { DiscoveredWallet, WalletAccount } from "../lib/wallet";
import { copy, shortId } from "../lib/format";
import { getLabel } from "../lib/recentAccounts";
import { FundVaultModal } from "./FundVaultModal";
import { PaymentRow } from "./PaymentRow";

interface Props {
  accountId: string;
  currentWallet: string | null;
  wallet: DiscoveredWallet | null;
  walletAccount: WalletAccount | null;
  onClose: () => void;
}

const SOLSCAN_ACCT = (id: string) => `https://solscan.io/account/${id}`;
const SOLSCAN_TX = (sig: string) => `https://solscan.io/tx/${sig}`;

export function ViewAccountModal({
  accountId,
  currentWallet,
  wallet,
  walletAccount,
  onClose,
}: Props) {
  const [details, setDetails] = useState<AccountDetails | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [payments, setPayments] = useState<PendingPayment[] | null>(null);
  const [showFund, setShowFund] = useState(false);
  const [reloadKey, setReloadKey] = useState(0);
  const label = getLabel(accountId);

  useEffect(() => {
    let cancelled = false;
    setLoading(true);
    const pda = new PublicKey(accountId);
    Promise.all([
      fetchAccountDetails(pda),
      fetchPendingPayments(pda).catch(() => [] as PendingPayment[]),
    ])
      .then(([d, p]) => {
        if (cancelled) return;
        if (!d) setError("Couldn't load this account.");
        else { setDetails(d); setPayments(p); }
      })
      .catch((e) => {
        if (!cancelled) setError(e instanceof Error ? e.message : String(e));
      })
      .finally(() => { if (!cancelled) setLoading(false); });
    return () => { cancelled = true; };
  }, [accountId, reloadKey]);

  const totalPendingLamports = (payments ?? []).reduce(
    (sum, p) => sum + p.transfers.reduce((s, t) => s + t.lamports, 0),
    0,
  );
  const shortfallLamports = details
    ? Math.max(0, totalPendingLamports - details.vaultBalanceLamports)
    : 0;
  const underfunded = shortfallLamports > 0;
  const canFund = wallet !== null && walletAccount !== null;

  return (
    <Backdrop onClose={onClose}>
      <div
        className="w-[560px] max-w-[92vw] max-h-[88vh] overflow-y-auto rounded-lg border shadow-2xl"
        style={{ background: "var(--color-elevated)", borderColor: "var(--color-border-strong)" }}
        onClick={(e) => e.stopPropagation()}
      >
        <div
          className="px-6 py-5 border-b flex items-start justify-between gap-4 sticky top-0 z-10"
          style={{ borderColor: "var(--color-border)", background: "var(--color-elevated)" }}
        >
          <div className="min-w-0">
            <div className="text-[11px] uppercase tracking-wider mb-1" style={{ color: "var(--color-dim)" }}>
              Company account
            </div>
            <h2 className="text-base font-semibold truncate" style={{ color: "var(--color-fg)" }}>
              {label ?? shortId(accountId, 6, 6)}
            </h2>
            {label && (
              <div className="font-mono text-xs mt-0.5" style={{ color: "var(--color-muted)" }}>
                {shortId(accountId, 6, 6)}
              </div>
            )}
          </div>
          {details && (
            <span
              className="text-[11px] px-2 py-1 rounded-full shrink-0"
              style={{ background: "var(--color-accent-soft)", color: "var(--color-accent)" }}
            >
              {details.approvalsRequired}-of-{details.approvers.length}
            </span>
          )}
          <button
            onClick={onClose}
            className="text-sm px-2 py-1 rounded hover:bg-white/5 shrink-0"
            style={{ color: "var(--color-muted)" }}
          >
            ✕
          </button>
        </div>

        {loading && (
          <div className="px-6 py-12 text-center">
            <div className="animate-pulse text-sm" style={{ color: "var(--color-muted)" }}>Loading…</div>
          </div>
        )}

        {error && !loading && (
          <div className="px-6 py-5">
            <div
              className="px-3 py-2 rounded-md text-sm"
              style={{
                background: "rgba(248, 81, 73, 0.08)",
                color: "var(--color-danger)",
                border: `1px solid var(--color-danger)`,
              }}
            >
              {error}
            </div>
          </div>
        )}

        {details && !loading && (
          <div className="px-6 py-5 space-y-5">
            <Section title="Account ID">
              <Pubkey value={details.accountId} />
            </Section>

            <Section title="Vault" hint="Where this account's funds are held. Send to this address to fund payroll.">
              <Pubkey value={details.vaultId} />
              <div
                className="mt-3 px-3 py-2.5 rounded-md flex items-center justify-between gap-3"
                style={{
                  background: underfunded ? "rgba(248, 81, 73, 0.06)" : "var(--color-bg)",
                  border: `1px solid ${underfunded ? "var(--color-danger)" : "var(--color-border)"}`,
                }}
              >
                <div>
                  <div className="text-[11px] uppercase tracking-wider" style={{ color: "var(--color-dim)" }}>Balance</div>
                  <div className="font-mono text-sm mt-0.5" style={{ color: underfunded ? "var(--color-danger)" : "var(--color-fg)" }}>
                    {(details.vaultBalanceLamports / LAMPORTS_PER_SOL).toFixed(4)} SOL
                  </div>
                  {underfunded && (
                    <div className="text-xs mt-1" style={{ color: "var(--color-danger)" }}>
                      Short by {(shortfallLamports / LAMPORTS_PER_SOL).toFixed(4)} SOL — pending payments can't execute.
                    </div>
                  )}
                </div>
                {canFund && (
                  <button
                    onClick={() => setShowFund(true)}
                    className="text-xs font-medium px-3 py-1.5 rounded-md shrink-0"
                    style={{
                      background: underfunded ? "var(--color-accent)" : "transparent",
                      color: underfunded ? "#001721" : "var(--color-fg)",
                      border: `1px solid ${underfunded ? "transparent" : "var(--color-border-strong)"}`,
                    }}
                  >
                    Fund
                  </button>
                )}
              </div>
            </Section>

            <Section title={`Approvers · ${details.approvers.length}`}>
              <div className="space-y-1.5">
                {details.approvers.map((m) => (
                  <ApproverRow
                    key={m.key}
                    approver={m}
                    isMe={currentWallet === m.key}
                  />
                ))}
              </div>
            </Section>

            {showFund && wallet && walletAccount && (
              <FundVaultModal
                vaultAddress={details.vaultId}
                shortfallLamports={shortfallLamports}
                wallet={wallet}
                account={walletAccount}
                onClose={() => setShowFund(false)}
                onFunded={() => {
                  setShowFund(false);
                  setReloadKey((k) => k + 1);  // refresh balance + payments
                }}
              />
            )}

            <Section title={`Pending payments · ${payments?.length ?? 0}`}>
              {!payments || payments.length === 0 ? (
                <p className="text-xs" style={{ color: "var(--color-muted)" }}>
                  No payments awaiting approval right now.
                </p>
              ) : (
                <div className="space-y-2">
                  {payments.map((p) => (
                    <PaymentRow
                      key={p.transactionIndex}
                      payment={p}
                      threshold={details.approvalsRequired}
                      multisigAccountId={accountId}
                      currentWallet={currentWallet}
                      isApprover={details.approvers.some((a) => a.key === currentWallet)}
                      wallet={wallet}
                      walletAccount={walletAccount}
                      onMutated={() => setReloadKey((k) => k + 1)}
                    />
                  ))}
                </div>
              )}
            </Section>
          </div>
        )}
      </div>
    </Backdrop>
  );
}

function Backdrop({ children, onClose }: { children: React.ReactNode; onClose: () => void }) {
  return (
    <div
      className="fixed inset-0 z-50 flex items-center justify-center"
      style={{ background: "rgba(0,0,0,0.55)", backdropFilter: "blur(4px)" }}
      onClick={onClose}
    >
      {children}
    </div>
  );
}

function Section({ title, hint, children }: { title: string; hint?: string; children: React.ReactNode }) {
  return (
    <div>
      <div className="text-[11px] uppercase tracking-wider mb-2" style={{ color: "var(--color-dim)" }}>
        {title}
      </div>
      {hint && <p className="text-xs mb-2" style={{ color: "var(--color-muted)" }}>{hint}</p>}
      {children}
    </div>
  );
}

function Pubkey({ value }: { value: string }) {
  const [copied, setCopied] = useState(false);
  async function onCopy() {
    if (await copy(value)) {
      setCopied(true);
      setTimeout(() => setCopied(false), 1200);
    }
  }
  return (
    <div
      className="px-3 py-2.5 rounded-md flex items-center justify-between gap-2"
      style={{ background: "var(--color-bg)", border: `1px solid var(--color-border)` }}
    >
      <span className="font-mono text-xs break-all min-w-0" style={{ color: "var(--color-fg)" }}>{value}</span>
      <div className="flex items-center gap-2 shrink-0">
        <button onClick={onCopy} title={copied ? "Copied" : "Copy"} className="text-xs hover:opacity-80" style={{ color: "var(--color-muted)" }}>
          {copied ? "✓" : "Copy"}
        </button>
        <a
          href={SOLSCAN_ACCT(value)}
          target="_blank"
          rel="noreferrer"
          className="text-xs hover:opacity-80"
          style={{ color: "var(--color-accent)" }}
        >
          Solscan ↗
        </a>
      </div>
    </div>
  );
}

function ApproverRow({ approver, isMe }: { approver: { key: string; permissions: number }; isMe: boolean }) {
  return (
    <div
      className="px-3 py-2 rounded-md flex items-start justify-between gap-3"
      style={{
        background: isMe ? "var(--color-accent-soft)" : "var(--color-bg)",
        border: `1px solid ${isMe ? "transparent" : "var(--color-border)"}`,
      }}
    >
      <div className="min-w-0 flex-1">
        <div className="font-mono text-xs break-all" style={{ color: isMe ? "var(--color-accent)" : "var(--color-fg)" }}>
          {approver.key}
        </div>
        {isMe && (
          <span className="text-[10px] uppercase tracking-wider mt-0.5 inline-block" style={{ color: "var(--color-accent)" }}>
            you
          </span>
        )}
      </div>
      <span className="text-[10px] uppercase tracking-wider shrink-0 mt-0.5" style={{ color: "var(--color-dim)" }}>
        {formatPermissions(approver.permissions)}
      </span>
    </div>
  );
}

function formatPermissions(mask: number): string {
  const parts: string[] = [];
  if (mask & 0x01) parts.push("init");
  if (mask & 0x02) parts.push("vote");
  if (mask & 0x04) parts.push("exec");
  return parts.length > 0 ? parts.join("+") : `0x${mask.toString(16)}`;
}


// Solscan tx link helper kept for future use when we surface the create-tx
// signature in this view.
export const _solscanTx = SOLSCAN_TX;
