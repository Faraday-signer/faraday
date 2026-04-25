import { useEffect, useMemo, useState } from "react";
import { LAMPORTS_PER_SOL, PublicKey, VersionedTransaction } from "@solana/web3.js";
import { connection } from "../lib/squads";
import { planFundVault } from "../lib/fundVault";
import {
  type DiscoveredWallet,
  type WalletAccount,
  signTransactionWithWallet,
} from "../lib/wallet";
import { shortId } from "../lib/format";

interface Props {
  vaultAddress: string;
  shortfallLamports?: number;  // pre-fill amount when underfunded
  wallet: DiscoveredWallet;
  account: WalletAccount;
  onClose: () => void;
  onFunded: () => void;
}

type Phase =
  | { kind: "form" }
  | { kind: "submitting"; step: string }
  | { kind: "broadcast"; signature: string; status: "pending" | "confirmed" | "expired" }
  | { kind: "error"; message: string };

const SOLSCAN_TX = (sig: string) => `https://solscan.io/tx/${sig}`;

export function FundVaultModal({
  vaultAddress,
  shortfallLamports = 0,
  wallet,
  account,
  onClose,
  onFunded,
}: Props) {
  const from = useMemo(() => new PublicKey(account.address), [account.address]);
  const vault = useMemo(() => new PublicKey(vaultAddress), [vaultAddress]);

  const initial = shortfallLamports > 0
    ? (shortfallLamports / LAMPORTS_PER_SOL).toFixed(4)
    : "";
  const [amountSol, setAmountSol] = useState<string>(initial);
  const [sourceBalance, setSourceBalance] = useState<number | null>(null);
  const [phase, setPhase] = useState<Phase>({ kind: "form" });

  const lamports = useMemo(() => {
    const n = parseFloat(amountSol);
    if (!isFinite(n) || n <= 0) return 0;
    return Math.round(n * LAMPORTS_PER_SOL);
  }, [amountSol]);

  // Best-effort: show the user's current SOL so they know how much they can spare.
  useEffect(() => {
    let cancelled = false;
    connection.getBalance(from).then((b) => { if (!cancelled) setSourceBalance(b); }).catch(() => {});
    return () => { cancelled = true; };
  }, [from]);

  const insufficient = sourceBalance !== null && lamports > sourceBalance;
  const ready = lamports > 0 && !insufficient;

  async function submit() {
    if (!ready) return;
    setPhase({ kind: "submitting", step: "Building transaction…" });
    try {
      const plan = await planFundVault({ from, vault, lamports });

      setPhase({ kind: "submitting", step: "Waiting for wallet signature…" });
      const signed = await signTransactionWithWallet(wallet, account, plan.tx.serialize());

      setPhase({ kind: "submitting", step: "Sending to the network…" });
      const tx = VersionedTransaction.deserialize(signed);
      const signature = await connection.sendRawTransaction(tx.serialize(), { skipPreflight: true });

      setPhase({ kind: "broadcast", signature, status: "pending" });
      onFunded();

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

  return (
    <Backdrop onClose={onClose}>
      <div
        className="w-[480px] max-w-[92vw] rounded-lg border shadow-2xl"
        style={{ background: "var(--color-elevated)", borderColor: "var(--color-border-strong)" }}
        onClick={(e) => e.stopPropagation()}
      >
        <div className="px-6 py-5 border-b flex items-center justify-between" style={{ borderColor: "var(--color-border)" }}>
          <div>
            <h2 className="text-base font-semibold" style={{ color: "var(--color-fg)" }}>Fund company account</h2>
            <p className="text-xs mt-0.5" style={{ color: "var(--color-muted)" }}>
              Top up the vault so pending payments can execute.
            </p>
          </div>
          <button onClick={onClose} className="text-sm px-2 py-1 rounded hover:bg-white/5" style={{ color: "var(--color-muted)" }}>✕</button>
        </div>

        {phase.kind === "form" && (
          <div className="px-6 py-5 space-y-4">
            <Row label="From" right={
              sourceBalance !== null
                ? <span className="font-mono">{(sourceBalance / LAMPORTS_PER_SOL).toFixed(4)} SOL</span>
                : null
            }>
              <span className="font-mono text-xs" style={{ color: "var(--color-fg)" }}>
                {shortId(account.address, 6, 6)} · you
              </span>
            </Row>

            <Row label="To">
              <span className="font-mono text-xs" style={{ color: "var(--color-fg)" }}>
                {shortId(vaultAddress, 6, 6)} · vault
              </span>
            </Row>

            <div>
              <label className="text-xs font-medium block mb-1.5" style={{ color: "var(--color-fg)" }}>Amount</label>
              <div className="flex items-center gap-2">
                <input
                  value={amountSol}
                  onChange={(e) => setAmountSol(e.target.value)}
                  placeholder="0.00"
                  inputMode="decimal"
                  autoFocus
                  className="flex-1 px-3 py-2 rounded-md text-sm font-mono outline-none"
                  style={{
                    background: "var(--color-bg)",
                    border: `1px solid ${insufficient ? "var(--color-danger)" : "var(--color-border-strong)"}`,
                    color: "var(--color-fg)",
                  }}
                />
                <span className="text-sm" style={{ color: "var(--color-muted)" }}>SOL</span>
              </div>
              {insufficient && (
                <p className="text-xs mt-1" style={{ color: "var(--color-danger)" }}>
                  Your wallet doesn't hold that much.
                </p>
              )}
            </div>

            <div className="flex justify-end gap-2 pt-1">
              <button onClick={onClose} className="text-sm px-4 py-2 rounded-md hover:bg-white/5" style={{ color: "var(--color-muted)" }}>Cancel</button>
              <button
                onClick={submit}
                disabled={!ready}
                className="text-sm font-medium px-4 py-2 rounded-md disabled:opacity-50"
                style={{ background: "var(--color-accent)", color: "#001721" }}
              >
                Send
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
              style={{ background: "rgba(248,81,73,0.08)", color: "var(--color-danger)", border: `1px solid var(--color-danger)` }}
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
          <div className="px-6 py-6 text-center">
            <div className="mx-auto w-10 h-10 rounded-full flex items-center justify-center mb-3" style={{ background: "var(--color-accent-soft)", color: "var(--color-accent)" }}>
              <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.5" strokeLinecap="round" strokeLinejoin="round">
                <path d="M20 6L9 17l-5-5" />
              </svg>
            </div>
            <h3 className="text-base font-medium mb-1" style={{ color: "var(--color-fg)" }}>Funds sent</h3>
            <p className="text-xs mb-4" style={{ color: phase.status === "confirmed" ? "var(--color-success)" : phase.status === "expired" ? "var(--color-warning)" : "var(--color-muted)" }}>
              {phase.status === "pending" ? "Confirming on-chain…" : phase.status === "confirmed" ? "Confirmed" : "Network couldn't confirm in time"}
            </p>
            <a
              href={SOLSCAN_TX(phase.signature)}
              target="_blank"
              rel="noreferrer"
              className="text-xs font-mono break-all hover:underline"
              style={{ color: "var(--color-accent)" }}
            >
              {phase.signature.slice(0, 24)}…
            </a>
            <div className="flex justify-end mt-5">
              <button onClick={onClose} className="text-sm font-medium px-4 py-2 rounded-md" style={{ background: "var(--color-accent)", color: "#001721" }}>Done</button>
            </div>
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

function Row({ label, right, children }: { label: string; right?: React.ReactNode; children: React.ReactNode }) {
  return (
    <div
      className="px-3 py-2.5 rounded-md flex items-center justify-between gap-2"
      style={{ background: "var(--color-bg)", border: `1px solid var(--color-border)` }}
    >
      <div>
        <div className="text-[10px] uppercase tracking-wider mb-0.5" style={{ color: "var(--color-dim)" }}>{label}</div>
        {children}
      </div>
      {right && <div className="text-xs" style={{ color: "var(--color-muted)" }}>{right}</div>}
    </div>
  );
}
