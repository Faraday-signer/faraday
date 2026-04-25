import { useState } from "react";
import { LAMPORTS_PER_SOL, PublicKey, VersionedTransaction } from "@solana/web3.js";
import { connection } from "../lib/squads";
import type { PendingPayment } from "../lib/payments";
import type { DiscoveredWallet, WalletAccount } from "../lib/wallet";
import { signTransactionWithWallet } from "../lib/wallet";
import { planApprove, planExecute, planReject } from "../lib/votePayment";
import { shortId } from "../lib/format";

export interface PaymentRowProps {
  payment: PendingPayment;
  threshold: number;
  multisigAccountId: string;
  /** When set, renders the company-account context above the summary. */
  accountLabel?: string;
  currentWallet: string | null;
  isApprover: boolean;
  wallet: DiscoveredWallet | null;
  walletAccount: WalletAccount | null;
  onMutated: () => void;
}

export function PaymentRow({
  payment,
  threshold,
  multisigAccountId,
  accountLabel,
  currentWallet,
  isApprover,
  wallet,
  walletAccount,
  onMutated,
}: PaymentRowProps) {
  const [busy, setBusy] = useState<null | "approve" | "reject" | "execute">(null);
  const [actionError, setActionError] = useState<string | null>(null);
  const [actionResult, setActionResult] = useState<null | { kind: "approve" | "reject" | "execute"; signature: string }>(null);

  const totalLamports = payment.transfers.reduce((s, t) => s + t.lamports, 0);
  const totalSol = (totalLamports / LAMPORTS_PER_SOL).toFixed(4);
  const approved = payment.approvedBy.length;
  const iApproved = !!currentWallet && payment.approvedBy.includes(currentWallet);
  const iRejected = !!currentWallet && payment.rejectedBy.includes(currentWallet);
  const iVoted = iApproved || iRejected;
  const canVote = isApprover && !iVoted && payment.status === "Active";
  const canExecute = isApprover && payment.status === "Approved";

  const statusStyle = STATUS_STYLES[payment.status];

  async function runAction(kind: "approve" | "reject" | "execute") {
    if (!wallet || !walletAccount) return;
    setBusy(kind);
    setActionError(null);
    try {
      const member = new PublicKey(walletAccount.address);
      const multisigPda = new PublicKey(multisigAccountId);
      const txIndex = BigInt(payment.transactionIndex);

      const plan =
        kind === "approve" ? await planApprove({ multisigPda, transactionIndex: txIndex, member })
        : kind === "reject" ? await planReject({ multisigPda, transactionIndex: txIndex, member })
        : await planExecute({ multisigPda, transactionIndex: txIndex, member });

      const signed = await signTransactionWithWallet(wallet, walletAccount, plan.tx.serialize());
      const tx = VersionedTransaction.deserialize(signed);
      const signature = await connection.sendRawTransaction(tx.serialize(), { skipPreflight: true });
      const conf = await connection.confirmTransaction(
        { signature, blockhash: plan.blockhash, lastValidBlockHeight: plan.lastValidBlockHeight },
        "confirmed",
      );
      if (conf.value.err) {
        setActionError("Transaction landed but the network reported an error. Check Solscan.");
        setActionResult({ kind, signature });
        return;
      }
      setActionResult({ kind, signature });
      setTimeout(() => onMutated(), 1800);
    } catch (e) {
      setActionError(e instanceof Error ? e.message : String(e));
    } finally {
      setBusy(null);
    }
  }

  return (
    <div
      className="rounded-md p-3 space-y-2"
      style={{ background: "var(--color-bg)", border: `1px solid var(--color-border)` }}
    >
      <div className="flex items-start justify-between gap-2">
        <div className="min-w-0">
          {accountLabel && (
            <div className="text-[10px] uppercase tracking-wider mb-1" style={{ color: "var(--color-dim)" }}>
              {accountLabel}
            </div>
          )}
          <div className="text-sm font-medium" style={{ color: "var(--color-fg)" }}>
            {summary(payment, totalSol)}
          </div>
          <div className="text-[11px] mt-0.5" style={{ color: "var(--color-dim)" }}>
            #{payment.transactionIndex}
            {payment.createdAt && payment.createdAt > 0 && ` · ${formatRelative(payment.createdAt)}`}
          </div>
        </div>
        <span
          className="text-[10px] uppercase tracking-wider px-2 py-0.5 rounded-full shrink-0"
          style={{ background: statusStyle.bg, color: statusStyle.fg }}
        >
          {statusStyle.label}
        </span>
      </div>

      {payment.transfers.length > 0 && (
        <div className="space-y-1.5">
          {payment.transfers.map((t, i) => (
            <div key={i} className="flex items-start justify-between gap-3 text-xs font-mono">
              <span className="break-all min-w-0" style={{ color: "var(--color-muted)" }}>
                → {t.recipient}
              </span>
              <span className="shrink-0" style={{ color: "var(--color-fg)" }}>
                {(t.lamports / LAMPORTS_PER_SOL).toFixed(4)} SOL
              </span>
            </div>
          ))}
        </div>
      )}

      <div className="flex items-center justify-between pt-1.5 border-t" style={{ borderColor: "var(--color-border)" }}>
        <div className="flex items-center gap-1.5">
          <ApprovalDots approved={approved} threshold={threshold} />
          <span className="text-xs" style={{ color: "var(--color-muted)" }}>
            {approved} / {threshold} approved
            {iApproved && " · you"}
            {iRejected && " · you rejected"}
          </span>
        </div>
        <a
          href={`https://solscan.io/account/${payment.proposalPda}`}
          target="_blank"
          rel="noreferrer"
          className="text-xs hover:opacity-80"
          style={{ color: "var(--color-accent)" }}
        >
          Solscan ↗
        </a>
      </div>

      {(canVote || canExecute) && actionResult === null && (
        <div className="flex gap-2 pt-1">
          {canVote && (
            <>
              <button
                onClick={() => runAction("approve")}
                disabled={busy !== null}
                className="flex-1 text-xs font-medium px-3 py-1.5 rounded-md disabled:opacity-50"
                style={{ background: "var(--color-accent)", color: "#001721" }}
              >
                {busy === "approve" ? "Approving…" : "Approve"}
              </button>
              <button
                onClick={() => runAction("reject")}
                disabled={busy !== null}
                className="text-xs px-3 py-1.5 rounded-md border disabled:opacity-50"
                style={{ borderColor: "var(--color-border-strong)", color: "var(--color-danger)" }}
              >
                {busy === "reject" ? "Rejecting…" : "Reject"}
              </button>
            </>
          )}
          {canExecute && (
            <button
              onClick={() => runAction("execute")}
              disabled={busy !== null}
              className="flex-1 text-xs font-medium px-3 py-1.5 rounded-md disabled:opacity-50"
              style={{ background: "var(--color-success)", color: "#001721" }}
            >
              {busy === "execute" ? "Executing payment…" : "Execute payment"}
            </button>
          )}
        </div>
      )}

      {actionResult && (
        <div
          className="text-xs px-3 py-2 rounded flex items-center justify-between gap-2"
          style={{ background: "rgba(63, 185, 80, 0.10)", color: "var(--color-success)", border: "1px solid rgba(63, 185, 80, 0.30)" }}
        >
          <div className="flex items-center gap-2">
            <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.5" strokeLinecap="round" strokeLinejoin="round">
              <path d="M20 6L9 17l-5-5" />
            </svg>
            <span>
              {actionResult.kind === "execute" ? "Payment executed — funds sent"
                : actionResult.kind === "approve" ? "Approval recorded"
                : "Rejection recorded"}
            </span>
          </div>
          <a
            href={`https://solscan.io/tx/${actionResult.signature}`}
            target="_blank"
            rel="noreferrer"
            className="hover:underline shrink-0"
          >
            View tx ↗
          </a>
        </div>
      )}

      {actionError && (
        <div
          className="text-xs px-2 py-1.5 rounded"
          style={{ background: "rgba(248,81,73,0.08)", color: "var(--color-danger)" }}
        >
          {actionError}
        </div>
      )}
    </div>
  );
}

function ApprovalDots({ approved, threshold }: { approved: number; threshold: number }) {
  return (
    <div className="flex gap-0.5">
      {Array.from({ length: threshold }, (_, i) => (
        <span
          key={i}
          className="w-1.5 h-1.5 rounded-full"
          style={{ background: i < approved ? "var(--color-success)" : "var(--color-border-strong)" }}
        />
      ))}
    </div>
  );
}

function summary(p: PendingPayment, totalSol: string): string {
  const n = p.transfers.length;
  if (n === 0 && p.otherInstructions > 0) {
    return `Custom transaction · ${p.otherInstructions} ${p.otherInstructions === 1 ? "instruction" : "instructions"}`;
  }
  if (n === 1) {
    const single = p.transfers[0];
    if (single) return `Pay ${totalSol} SOL to ${shortId(single.recipient, 4, 4)}`;
    return `Pay ${totalSol} SOL`;
  }
  return `Pay ${totalSol} SOL to ${n} recipients`;
}

function formatRelative(unixSeconds: number): string {
  const diff = Math.floor(Date.now() / 1000) - unixSeconds;
  if (diff < 60) return "just now";
  if (diff < 3600) return `${Math.floor(diff / 60)}m ago`;
  if (diff < 86400) return `${Math.floor(diff / 3600)}h ago`;
  return `${Math.floor(diff / 86400)}d ago`;
}

const STATUS_STYLES: Record<string, { bg: string; fg: string; label: string }> = {
  Active:    { bg: "rgba(210, 153, 34, 0.10)", fg: "var(--color-warning)", label: "Pending approval" },
  Approved:  { bg: "rgba(63, 185, 80, 0.10)",  fg: "var(--color-success)", label: "Ready to execute" },
  Executing: { bg: "rgba(63, 185, 80, 0.10)",  fg: "var(--color-success)", label: "Executing" },
  Draft:     { bg: "var(--color-bg)",           fg: "var(--color-muted)",   label: "Draft" },
  Executed:  { bg: "rgba(63, 185, 80, 0.06)",  fg: "var(--color-success)", label: "Paid" },
  Rejected:  { bg: "rgba(248, 81, 73, 0.10)",   fg: "var(--color-danger)",  label: "Rejected" },
  Cancelled: { bg: "var(--color-bg)",           fg: "var(--color-muted)",   label: "Cancelled" },
};
