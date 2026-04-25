import { useEffect, useMemo, useState } from "react";
import { LAMPORTS_PER_SOL, PublicKey, VersionedTransaction } from "@solana/web3.js";
import { connection, reportMultisig } from "../lib/squads";
import {
  estimateCreateAccountCost,
  planCreateAccount,
} from "../lib/createAccount";
import {
  type DiscoveredWallet,
  type WalletAccount,
  signTransactionWithWallet,
} from "../lib/wallet";
import { saveAccount } from "../lib/recentAccounts";

interface Props {
  wallet: DiscoveredWallet;
  account: WalletAccount;
  onClose: () => void;
  onCreated: (accountId: string) => void;
}

type Phase =
  | { kind: "form" }
  | { kind: "submitting"; step: string }
  | { kind: "broadcast"; accountId: string; signature: string; status: "pending" | "confirmed" | "expired" }
  | { kind: "error"; message: string };

const SOLSCAN_TX = (sig: string) => `https://solscan.io/tx/${sig}`;
const SOLSCAN_ACCT = (id: string) => `https://solscan.io/account/${id}`;

export function CreateAccountModal({ wallet, account, onClose, onCreated }: Props) {
  const creator = useMemo(() => new PublicKey(account.address), [account.address]);
  const [extraApprovers, setExtraApprovers] = useState<string>("");
  const [includeSelf, setIncludeSelf] = useState(true);
  const [threshold, setThreshold] = useState(1);
  const [memo, setMemo] = useState("");
  const [phase, setPhase] = useState<Phase>({ kind: "form" });
  const [costLamports, setCostLamports] = useState<number | null>(null);

  const approvers = useMemo(() => {
    const extras = extraApprovers
      .split(/[\s,]+/)
      .map((s) => s.trim())
      .filter(Boolean);
    return includeSelf ? [creator.toBase58(), ...extras] : extras;
  }, [extraApprovers, creator, includeSelf]);

  const approverPubkeys = useMemo(() => {
    return approvers.map((a) => {
      try { return new PublicKey(a); } catch { return null; }
    });
  }, [approvers]);

  const invalidIndex = approverPubkeys.findIndex((k) => k === null);
  const approverCount = approvers.length;
  const validApprovers = invalidIndex === -1;

  // Refresh cost estimate when approver count changes.
  useEffect(() => {
    let cancelled = false;
    estimateCreateAccountCost(approverCount)
      .then((c) => { if (!cancelled) setCostLamports(c); })
      .catch(() => { if (!cancelled) setCostLamports(null); });
    return () => { cancelled = true; };
  }, [approverCount]);

  // Clamp threshold whenever approver count changes.
  useEffect(() => {
    if (threshold > approverCount) setThreshold(approverCount);
    if (threshold < 1) setThreshold(1);
  }, [approverCount, threshold]);

  async function submit() {
    if (!validApprovers) return;
    setPhase({ kind: "submitting", step: "Building transaction…" });
    try {
      const plan = await planCreateAccount({
        creator,
        approvers: approverPubkeys.filter((k): k is PublicKey => k !== null),
        approvalsRequired: threshold,
        memo: memo.trim() || undefined,
      });

      setPhase({ kind: "submitting", step: "Waiting for wallet signature…" });
      const signedBytes = await signTransactionWithWallet(
        wallet,
        account,
        plan.tx.serialize(),
      );

      setPhase({ kind: "submitting", step: "Sending to the network…" });
      const signedTx = VersionedTransaction.deserialize(signedBytes);
      // skipPreflight: the airgapped flow can outlast the blockhash window;
      // the network still validates the blockhash itself, so we get a clean
      // accept/expire signal without the RPC pre-rejecting.
      const signature = await connection.sendRawTransaction(signedTx.serialize(), {
        skipPreflight: true,
      });

      // Persist immediately so the account shows up in the dashboard even if
      // confirmation polling flakes or the user closes the modal early.
      const accountIdStr = plan.accountId.toBase58();
      const trimmedMemo = memo.trim();
      const labelOrUndef = trimmedMemo.length > 0 ? trimmedMemo : undefined;
      saveAccount({
        accountId: accountIdStr,
        signature,
        createdAt: Date.now(),
        label: labelOrUndef,
      });
      // Push to the central indexer so other approvers see this multisig
      // on their dashboards without waiting on a webhook round-trip.
      void reportMultisig(
        accountIdStr,
        approverPubkeys.filter((k): k is PublicKey => k !== null).map((k) => k.toBase58()),
        { signature, label: labelOrUndef },
      );
      onCreated(accountIdStr);

      setPhase({ kind: "broadcast", accountId: accountIdStr, signature, status: "pending" });

      // Poll for confirmation against the TX's own lastValidBlockHeight (not
      // a fresh one — that's what made the previous flow hang silently).
      connection
        .confirmTransaction(
          {
            signature,
            blockhash: plan.blockhash,
            lastValidBlockHeight: plan.lastValidBlockHeight,
          },
          "confirmed",
        )
        .then((res) => {
          setPhase((prev) =>
            prev.kind === "broadcast"
              ? { ...prev, status: res.value.err ? "expired" : "confirmed" }
              : prev,
          );
        })
        .catch(() => {
          setPhase((prev) =>
            prev.kind === "broadcast" ? { ...prev, status: "expired" } : prev,
          );
        });
    } catch (e) {
      setPhase({ kind: "error", message: e instanceof Error ? e.message : String(e) });
    }
  }

  return (
    <Backdrop onClose={onClose}>
      <div
        className="w-[520px] max-w-[92vw] rounded-lg border shadow-2xl"
        style={{ background: "var(--color-elevated)", borderColor: "var(--color-border-strong)" }}
        onClick={(e) => e.stopPropagation()}
      >
        <div className="px-6 py-5 border-b flex items-center justify-between" style={{ borderColor: "var(--color-border)" }}>
          <div>
            <h2 className="text-base font-semibold" style={{ color: "var(--color-fg)" }}>New company account</h2>
            <p className="text-xs mt-0.5" style={{ color: "var(--color-muted)" }}>
              Created on-chain — no service fee.
            </p>
          </div>
          <button
            onClick={onClose}
            className="text-sm px-2 py-1 rounded hover:bg-white/5"
            style={{ color: "var(--color-muted)" }}
          >
            ✕
          </button>
        </div>

        {phase.kind === "form" && (
          <div className="px-6 py-5 space-y-4">
            <Field label="Approvers" hint="One wallet ID per line. The signer (you, paying the fee) doesn't have to be an approver.">
              <label
                className="flex items-center gap-2 text-xs font-mono px-3 py-2 rounded mb-2 cursor-pointer"
                style={{
                  background: includeSelf ? "rgba(26, 248, 255, 0.06)" : "var(--color-bg)",
                  color: includeSelf ? "var(--color-accent)" : "var(--color-muted)",
                  border: `1px solid ${includeSelf ? "transparent" : "var(--color-border)"}`,
                }}
              >
                <input
                  type="checkbox"
                  checked={includeSelf}
                  onChange={(e) => setIncludeSelf(e.target.checked)}
                  className="accent-[color:var(--color-accent)]"
                />
                <span>Include my wallet · {account.address}</span>
              </label>
              <textarea
                value={extraApprovers}
                onChange={(e) => setExtraApprovers(e.target.value)}
                placeholder="Paste additional approver wallet IDs, one per line"
                rows={3}
                className="w-full px-3 py-2 rounded-md text-sm font-mono outline-none focus:ring-1"
                style={{
                  background: "var(--color-bg)",
                  border: `1px solid var(--color-border-strong)`,
                  color: "var(--color-fg)",
                }}
              />
              {!validApprovers && (
                <p className="text-xs mt-1" style={{ color: "var(--color-danger)" }}>
                  Wallet ID #{invalidIndex + 1} is invalid.
                </p>
              )}
            </Field>

            <Field label="Approvals required" hint={`${threshold} of ${approverCount} approvers must sign before a payment goes out.`}>
              <div className="flex items-center gap-2">
                <input
                  type="number"
                  min={1}
                  max={approverCount}
                  value={threshold}
                  onChange={(e) => setThreshold(Number(e.target.value))}
                  className="w-20 px-3 py-2 rounded-md text-sm font-mono outline-none"
                  style={{
                    background: "var(--color-bg)",
                    border: `1px solid var(--color-border-strong)`,
                    color: "var(--color-fg)",
                  }}
                />
                <span className="text-sm" style={{ color: "var(--color-muted)" }}>of {approverCount}</span>
              </div>
            </Field>

            <Field label="Label (optional)">
              <input
                value={memo}
                onChange={(e) => setMemo(e.target.value)}
                placeholder="e.g. Operations · Payroll"
                maxLength={64}
                className="w-full px-3 py-2 rounded-md text-sm outline-none"
                style={{
                  background: "var(--color-bg)",
                  border: `1px solid var(--color-border-strong)`,
                  color: "var(--color-fg)",
                }}
              />
            </Field>

            <CostRow lamports={costLamports} />

            <div className="flex gap-2 justify-end pt-1">
              <button
                onClick={onClose}
                className="text-sm px-4 py-2 rounded-md hover:bg-white/5"
                style={{ color: "var(--color-muted)" }}
              >
                Cancel
              </button>
              <button
                onClick={submit}
                disabled={!validApprovers || approverCount < 1}
                className="text-sm font-medium px-4 py-2 rounded-md disabled:opacity-50"
                style={{ background: "var(--color-accent)", color: "#001721" }}
              >
                Create account
              </button>
            </div>
          </div>
        )}

        {phase.kind === "submitting" && (
          <div className="px-6 py-12 text-center">
            <div className="animate-pulse text-sm" style={{ color: "var(--color-muted)" }}>{phase.step}</div>
          </div>
        )}

        {phase.kind === "error" && (
          <div className="px-6 py-5">
            <div
              className="px-3 py-2 rounded-md text-sm"
              style={{
                background: "rgba(248, 81, 73, 0.08)",
                color: "var(--color-danger)",
                border: `1px solid var(--color-danger)`,
              }}
            >
              {phase.message}
            </div>
            <div className="flex gap-2 justify-end mt-4">
              <button onClick={() => setPhase({ kind: "form" })} className="text-sm px-4 py-2 rounded-md" style={{ color: "var(--color-fg)", border: `1px solid var(--color-border-strong)` }}>
                Try again
              </button>
            </div>
          </div>
        )}

        {phase.kind === "broadcast" && <BroadcastView phase={phase} onClose={onClose} />}
      </div>
    </Backdrop>
  );
}

function BroadcastView({
  phase,
  onClose,
}: {
  phase: { kind: "broadcast"; accountId: string; signature: string; status: "pending" | "confirmed" | "expired" };
  onClose: () => void;
}) {
  const statusLabel =
    phase.status === "pending" ? "Confirming on-chain…"
    : phase.status === "confirmed" ? "Confirmed"
    : "Network couldn't confirm in time";
  const statusColor =
    phase.status === "confirmed" ? "var(--color-success)"
    : phase.status === "expired" ? "var(--color-warning)"
    : "var(--color-muted)";

  return (
    <div className="px-6 py-6">
      <div className="text-center mb-5">
        <div
          className="mx-auto w-10 h-10 rounded-full flex items-center justify-center mb-3"
          style={{ background: "var(--color-accent-soft)", color: "var(--color-accent)" }}
        >
          <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.5" strokeLinecap="round" strokeLinejoin="round">
            <path d="M20 6L9 17l-5-5" />
          </svg>
        </div>
        <h3 className="text-base font-medium" style={{ color: "var(--color-fg)" }}>
          Account submitted
        </h3>
        <p className="text-xs mt-1" style={{ color: statusColor }}>
          {phase.status === "pending" && (
            <span className="inline-block w-2 h-2 rounded-full animate-pulse mr-1.5 align-middle" style={{ background: statusColor }} />
          )}
          {statusLabel}
        </p>
      </div>

      <div className="space-y-2 mb-5">
        <Row label="Account">
          <a
            href={SOLSCAN_ACCT(phase.accountId)}
            target="_blank"
            rel="noreferrer"
            className="font-mono text-xs break-all hover:underline"
            style={{ color: "var(--color-accent)" }}
          >
            {phase.accountId}
          </a>
        </Row>
        <Row label="Transaction">
          <a
            href={SOLSCAN_TX(phase.signature)}
            target="_blank"
            rel="noreferrer"
            className="font-mono text-xs break-all hover:underline"
            style={{ color: "var(--color-accent)" }}
          >
            {phase.signature.slice(0, 24)}…
          </a>
        </Row>
      </div>

      {phase.status === "expired" && (
        <p className="text-xs mb-4" style={{ color: "var(--color-muted)" }}>
          The blockhash window expired before confirmation. The tx may still have landed —
          check Solscan above.
        </p>
      )}

      <div className="flex justify-end">
        <button
          onClick={onClose}
          className="text-sm font-medium px-4 py-2 rounded-md"
          style={{ background: "var(--color-accent)", color: "#001721" }}
        >
          Done
        </button>
      </div>
    </div>
  );
}

function Row({ label, children }: { label: string; children: React.ReactNode }) {
  return (
    <div
      className="px-3 py-2 rounded-md"
      style={{ background: "var(--color-bg)", border: `1px solid var(--color-border)` }}
    >
      <div className="text-[10px] uppercase tracking-wider mb-1" style={{ color: "var(--color-dim)" }}>
        {label}
      </div>
      {children}
    </div>
  );
}

function Backdrop({ children, onClose }: { children: React.ReactNode; onClose: () => void }) {
  return (
    <div
      className="fixed inset-0 z-50 flex items-center justify-center"
      style={{ background: "rgba(0, 0, 0, 0.55)", backdropFilter: "blur(4px)" }}
      onClick={onClose}
    >
      {children}
    </div>
  );
}

function Field({ label, hint, children }: { label: string; hint?: string; children: React.ReactNode }) {
  return (
    <div>
      <label className="text-xs font-medium block mb-1.5" style={{ color: "var(--color-fg)" }}>{label}</label>
      {children}
      {hint && <p className="text-xs mt-1.5" style={{ color: "var(--color-dim)" }}>{hint}</p>}
    </div>
  );
}

function CostRow({ lamports }: { lamports: number | null }) {
  if (lamports === null) return null;
  const sol = (lamports / LAMPORTS_PER_SOL).toFixed(5);
  return (
    <div
      className="flex items-center justify-between px-3 py-2.5 rounded-md"
      style={{ background: "var(--color-bg)", border: `1px solid var(--color-border)` }}
    >
      <div>
        <div className="text-[11px] uppercase tracking-wider" style={{ color: "var(--color-dim)" }}>One-time cost</div>
        <div className="text-xs mt-0.5" style={{ color: "var(--color-muted)" }}>Rent + Squads program fee</div>
      </div>
      <div className="font-mono text-sm" style={{ color: "var(--color-fg)" }}>{sol} SOL</div>
    </div>
  );
}
