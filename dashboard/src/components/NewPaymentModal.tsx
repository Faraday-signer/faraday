import { useEffect, useMemo, useRef, useState } from "react";
import { LAMPORTS_PER_SOL, PublicKey, VersionedTransaction } from "@solana/web3.js";
import { connection } from "../lib/squads";
import { planCreatePayment } from "../lib/createPayment";
import {
  type DiscoveredWallet,
  type WalletAccount,
  signTransactionWithWallet,
} from "../lib/wallet";

interface Props {
  multisigAccountId: string;
  multisigLabel?: string;
  approvalsRequired: number;
  approverCount: number;
  wallet: DiscoveredWallet;
  account: WalletAccount;
  onClose: () => void;
}

type Phase =
  | { kind: "form" }
  | { kind: "submitting"; step: string }
  | { kind: "broadcast"; signature: string; status: "pending" | "confirmed" | "expired"; autoApproved: boolean }
  | { kind: "error"; message: string };

interface Row { id: number; address: string; amount: string }

interface ParsedRow {
  id: number;
  recipient: PublicKey | null;
  lamports: number;
  addressInvalid: boolean;
  amountInvalid: boolean;
}

function parseRow(r: Row): ParsedRow {
  const trimmedAddr = r.address.trim();
  let recipient: PublicKey | null = null;
  if (trimmedAddr.length > 0) {
    try { recipient = new PublicKey(trimmedAddr); } catch { /* invalid */ }
  }
  const trimmedAmt = r.amount.trim();
  const n = parseFloat(trimmedAmt);
  const lamports = isFinite(n) && n > 0 ? Math.round(n * LAMPORTS_PER_SOL) : 0;
  return {
    id: r.id,
    recipient,
    lamports,
    addressInvalid: trimmedAddr.length > 0 && recipient === null,
    amountInvalid: trimmedAmt.length > 0 && lamports === 0,
  };
}

const SOLSCAN_TX = (sig: string) => `https://solscan.io/tx/${sig}`;

export function NewPaymentModal({
  multisigAccountId,
  multisigLabel,
  approvalsRequired,
  approverCount,
  wallet,
  account,
  onClose,
}: Props) {
  const creator = useMemo(() => new PublicKey(account.address), [account.address]);
  const multisigPda = useMemo(() => new PublicKey(multisigAccountId), [multisigAccountId]);

  const idCounter = useRef(0);
  const newRow = (): Row => ({ id: ++idCounter.current, address: "", amount: "" });
  const [rows, setRows] = useState<Row[]>(() => [newRow()]);
  const [memo, setMemo] = useState("");
  const [phase, setPhase] = useState<Phase>({ kind: "form" });

  const parsedRows = useMemo(
    () => rows.map((r) => parseRow(r)),
    [rows],
  );
  const validTransfers: Array<{ recipient: PublicKey; lamports: number }> = parsedRows
    .filter((p) => p.recipient !== null && p.lamports > 0)
    .map((p) => ({ recipient: p.recipient as PublicKey, lamports: p.lamports }));
  const totalLamports = validTransfers.reduce((sum, t) => sum + t.lamports, 0);
  const ready = validTransfers.length > 0 &&
    parsedRows.every((p) => !p.addressInvalid && !p.amountInvalid);

  function updateRow(id: number, patch: Partial<Row>) {
    setRows((rs) => rs.map((r) => (r.id === id ? { ...r, ...patch } : r)));
  }
  function addRow() {
    setRows((rs) => [...rs, newRow()]);
  }
  function removeRow(id: number) {
    setRows((rs) => (rs.length === 1 ? rs : rs.filter((r) => r.id !== id)));
  }

  async function submit() {
    if (!ready) return;
    setPhase({ kind: "submitting", step: "Building transaction…" });
    try {
      const plan = await planCreatePayment({
        multisigPda,
        creator,
        transfers: validTransfers,
        memo: memo.trim() || undefined,
      });

      setPhase({ kind: "submitting", step: "Waiting for wallet signature…" });
      const signedBytes = await signTransactionWithWallet(wallet, account, plan.tx.serialize());

      setPhase({ kind: "submitting", step: "Sending to the network…" });
      const signedTx = VersionedTransaction.deserialize(signedBytes);
      const signature = await connection.sendRawTransaction(signedTx.serialize(), {
        skipPreflight: true,
      });

      setPhase({
        kind: "broadcast",
        signature,
        status: "pending",
        autoApproved: plan.autoApproved,
      });

      connection
        .confirmTransaction(
          { signature, blockhash: plan.blockhash, lastValidBlockHeight: plan.lastValidBlockHeight },
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
          setPhase((prev) => (prev.kind === "broadcast" ? { ...prev, status: "expired" } : prev));
        });
    } catch (e) {
      setPhase({ kind: "error", message: e instanceof Error ? e.message : String(e) });
    }
  }

  useEffect(() => {
    function onKey(e: KeyboardEvent) { if (e.key === "Escape") onClose(); }
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [onClose]);

  return (
    <Backdrop onClose={onClose}>
      <div
        className="w-[520px] max-w-[92vw] rounded-lg border shadow-2xl"
        style={{ background: "var(--color-elevated)", borderColor: "var(--color-border-strong)" }}
        onClick={(e) => e.stopPropagation()}
      >
        <div className="px-6 py-5 border-b flex items-start justify-between gap-3" style={{ borderColor: "var(--color-border)" }}>
          <div className="min-w-0">
            <h2 className="text-base font-semibold" style={{ color: "var(--color-fg)" }}>New payment</h2>
            <p className="text-xs mt-0.5 truncate" style={{ color: "var(--color-muted)" }}>
              From <span style={{ color: "var(--color-fg)" }}>{multisigLabel ?? "Company account"}</span> ·
              {" "}{approvalsRequired}-of-{approverCount} required
            </p>
          </div>
          <button onClick={onClose} className="text-sm px-2 py-1 rounded hover:bg-white/5 shrink-0" style={{ color: "var(--color-muted)" }}>✕</button>
        </div>

        {phase.kind === "form" && (
          <div className="px-6 py-5 space-y-4">
            <div>
              <label className="text-xs font-medium block mb-1.5" style={{ color: "var(--color-fg)" }}>
                Recipients
              </label>
              <div className="space-y-2">
                {rows.map((row, i) => {
                  const parsed = parsedRows[i];
                  return (
                    <div key={row.id} className="flex items-start gap-2">
                      <div className="flex-1 min-w-0">
                        <input
                          value={row.address}
                          onChange={(e) => updateRow(row.id, { address: e.target.value })}
                          placeholder="Wallet address"
                          spellCheck={false}
                          autoFocus={i === 0 && rows.length === 1}
                          className="w-full px-3 py-2 rounded-md text-sm font-mono outline-none"
                          style={{
                            background: "var(--color-bg)",
                            border: `1px solid ${parsed.addressInvalid ? "var(--color-danger)" : "var(--color-border-strong)"}`,
                            color: "var(--color-fg)",
                          }}
                        />
                        {parsed.addressInvalid && (
                          <p className="text-xs mt-1" style={{ color: "var(--color-danger)" }}>Not a valid Solana address.</p>
                        )}
                      </div>
                      <div className="w-32 shrink-0">
                        <div className="flex items-center gap-1.5">
                          <input
                            value={row.amount}
                            onChange={(e) => updateRow(row.id, { amount: e.target.value })}
                            placeholder="0.00"
                            inputMode="decimal"
                            className="flex-1 min-w-0 px-2 py-2 rounded-md text-sm font-mono text-right outline-none"
                            style={{
                              background: "var(--color-bg)",
                              border: `1px solid ${parsed.amountInvalid ? "var(--color-danger)" : "var(--color-border-strong)"}`,
                              color: "var(--color-fg)",
                            }}
                          />
                          <span className="text-xs" style={{ color: "var(--color-muted)" }}>SOL</span>
                        </div>
                      </div>
                      <button
                        onClick={() => removeRow(row.id)}
                        disabled={rows.length === 1}
                        title="Remove recipient"
                        className="w-8 h-9 rounded-md flex items-center justify-center text-sm hover:bg-white/5 disabled:opacity-30 disabled:cursor-not-allowed shrink-0"
                        style={{ color: "var(--color-muted)" }}
                      >
                        ✕
                      </button>
                    </div>
                  );
                })}
              </div>
              <button
                onClick={addRow}
                className="text-xs mt-2.5 px-3 py-1.5 rounded-md border"
                style={{ borderColor: "var(--color-border-strong)", color: "var(--color-muted)" }}
              >
                + Add recipient
              </button>
            </div>

            <div
              className="flex items-baseline justify-between px-3 py-2.5 rounded-md"
              style={{ background: "var(--color-bg)", border: `1px solid var(--color-border)` }}
            >
              <div>
                <div className="text-[11px] uppercase tracking-wider" style={{ color: "var(--color-dim)" }}>
                  Total · {validTransfers.length} {validTransfers.length === 1 ? "recipient" : "recipients"}
                </div>
                <div className="text-xs mt-0.5" style={{ color: "var(--color-muted)" }}>
                  Vault must hold at least this much for the batch to execute.
                </div>
              </div>
              <div className="font-mono text-sm" style={{ color: "var(--color-fg)" }}>
                {(totalLamports / LAMPORTS_PER_SOL).toFixed(4)} SOL
              </div>
            </div>

            <Field label="Memo (optional)" hint="Shows up on the approver's screen so they know what they're approving.">
              <input
                value={memo}
                onChange={(e) => setMemo(e.target.value)}
                placeholder="e.g. April payroll"
                maxLength={120}
                className="w-full px-3 py-2 rounded-md text-sm outline-none"
                style={{
                  background: "var(--color-bg)",
                  border: `1px solid var(--color-border-strong)`,
                  color: "var(--color-fg)",
                }}
              />
            </Field>

            <div className="flex justify-end gap-2 pt-1">
              <button onClick={onClose} className="text-sm px-4 py-2 rounded-md hover:bg-white/5" style={{ color: "var(--color-muted)" }}>Cancel</button>
              <button
                onClick={submit}
                disabled={!ready}
                className="text-sm font-medium px-4 py-2 rounded-md disabled:opacity-50"
                style={{ background: "var(--color-accent)", color: "#001721" }}
              >
                Submit for approval
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
            <div className="flex justify-end mt-4">
              <button onClick={() => setPhase({ kind: "form" })} className="text-sm px-4 py-2 rounded-md" style={{ color: "var(--color-fg)", border: `1px solid var(--color-border-strong)` }}>
                Try again
              </button>
            </div>
          </div>
        )}

        {phase.kind === "broadcast" && (
          <BroadcastView phase={phase} approvalsRequired={approvalsRequired} approverCount={approverCount} onClose={onClose} />
        )}
      </div>
    </Backdrop>
  );
}

function BroadcastView({
  phase,
  approvalsRequired,
  approverCount,
  onClose,
}: {
  phase: { kind: "broadcast"; signature: string; status: "pending" | "confirmed" | "expired"; autoApproved: boolean };
  approvalsRequired: number;
  approverCount: number;
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

  const myVote = phase.autoApproved ? 1 : 0;
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
        <h3 className="text-base font-medium" style={{ color: "var(--color-fg)" }}>Payment submitted</h3>
        <p className="text-xs mt-1" style={{ color: statusColor }}>
          {phase.status === "pending" && (
            <span className="inline-block w-2 h-2 rounded-full animate-pulse mr-1.5 align-middle" style={{ background: statusColor }} />
          )}
          {statusLabel}
        </p>
      </div>

      <div
        className="px-3 py-2.5 rounded-md mb-3"
        style={{ background: "var(--color-bg)", border: `1px solid var(--color-border)` }}
      >
        <div className="text-[11px] uppercase tracking-wider mb-1" style={{ color: "var(--color-dim)" }}>Approvals</div>
        <div className="text-sm font-mono" style={{ color: "var(--color-fg)" }}>
          {myVote} of {approvalsRequired} <span style={{ color: "var(--color-muted)" }}>({approverCount} approvers total)</span>
        </div>
        <div className="text-xs mt-1" style={{ color: "var(--color-muted)" }}>
          {phase.autoApproved
            ? "Your vote was counted automatically. The payment will execute once the remaining approvals come in."
            : "Waiting for approvers to sign on their Faraday devices."}
        </div>
      </div>

      <div
        className="px-3 py-2.5 rounded-md mb-5"
        style={{ background: "var(--color-bg)", border: `1px solid var(--color-border)` }}
      >
        <div className="text-[11px] uppercase tracking-wider mb-1" style={{ color: "var(--color-dim)" }}>Transaction</div>
        <a
          href={SOLSCAN_TX(phase.signature)}
          target="_blank"
          rel="noreferrer"
          className="font-mono text-xs break-all hover:underline"
          style={{ color: "var(--color-accent)" }}
        >
          {phase.signature.slice(0, 24)}…
        </a>
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

function Field({ label, hint, children }: { label: string; hint?: string; children: React.ReactNode }) {
  return (
    <div>
      <label className="text-xs font-medium block mb-1.5" style={{ color: "var(--color-fg)" }}>{label}</label>
      {children}
      {hint && <p className="text-xs mt-1.5" style={{ color: "var(--color-dim)" }}>{hint}</p>}
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
