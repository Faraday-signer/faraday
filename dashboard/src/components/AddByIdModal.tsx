import { useState } from "react";
import { PublicKey } from "@solana/web3.js";
import { connection, fetchAccountById, reportMultisig } from "../lib/squads";
import { saveAccount } from "../lib/recentAccounts";
import * as multisig from "@sqds/multisig";

interface Props {
  onClose: () => void;
  onAdded: (accountId: string) => void;
}

export function AddByIdModal({ onClose, onAdded }: Props) {
  const [id, setId] = useState("");
  const [busy, setBusy] = useState(false);
  const [err, setErr] = useState<string | null>(null);

  async function submit() {
    setErr(null);
    let pda: PublicKey;
    try {
      pda = new PublicKey(id.trim());
    } catch {
      setErr("That doesn't look like a valid Solana address.");
      return;
    }
    setBusy(true);
    try {
      const found = await fetchAccountById(pda);
      if (!found) {
        setErr("No Squads multisig at this address (or fetch failed).");
        return;
      }
      saveAccount({ accountId: found.accountId, signature: "", createdAt: Date.now() });
      // Tell the indexer about this multisig so its other approvers can
      // also discover it automatically on connect.
      try {
        const ms = await multisig.accounts.Multisig.fromAccountAddress(connection, pda);
        void reportMultisig(
          found.accountId,
          ms.members.map((m) => m.key.toBase58()),
        );
      } catch {
        // local save still works even if reporting fails
      }
      onAdded(found.accountId);
    } catch (e) {
      setErr(e instanceof Error ? e.message : String(e));
    } finally {
      setBusy(false);
    }
  }

  return (
    <div
      className="fixed inset-0 z-50 flex items-center justify-center"
      style={{ background: "rgba(0,0,0,0.55)", backdropFilter: "blur(4px)" }}
      onClick={onClose}
    >
      <div
        className="w-[480px] max-w-[92vw] rounded-lg border shadow-2xl"
        style={{ background: "var(--color-elevated)", borderColor: "var(--color-border-strong)" }}
        onClick={(e) => e.stopPropagation()}
      >
        <div className="px-6 py-5 border-b flex items-center justify-between" style={{ borderColor: "var(--color-border)" }}>
          <div>
            <h2 className="text-base font-semibold" style={{ color: "var(--color-fg)" }}>Add account by ID</h2>
            <p className="text-xs mt-0.5" style={{ color: "var(--color-muted)" }}>
              Paste a Squads multisig address to view it here.
            </p>
          </div>
          <button onClick={onClose} className="text-sm px-2 py-1 rounded hover:bg-white/5" style={{ color: "var(--color-muted)" }}>✕</button>
        </div>

        <div className="px-6 py-5 space-y-4">
          <input
            value={id}
            onChange={(e) => setId(e.target.value)}
            placeholder="Multisig account ID"
            autoFocus
            onKeyDown={(e) => { if (e.key === "Enter" && !busy) submit(); }}
            className="w-full px-3 py-2 rounded-md text-sm font-mono outline-none"
            style={{
              background: "var(--color-bg)",
              border: `1px solid var(--color-border-strong)`,
              color: "var(--color-fg)",
            }}
          />
          {err && (
            <div
              className="px-3 py-2 rounded-md text-sm"
              style={{ background: "rgba(248,81,73,0.08)", color: "var(--color-danger)", border: `1px solid var(--color-danger)` }}
            >
              {err}
            </div>
          )}
          <div className="flex justify-end gap-2 pt-1">
            <button onClick={onClose} className="text-sm px-4 py-2 rounded-md hover:bg-white/5" style={{ color: "var(--color-muted)" }}>Cancel</button>
            <button
              onClick={submit}
              disabled={busy || id.trim().length === 0}
              className="text-sm font-medium px-4 py-2 rounded-md disabled:opacity-50"
              style={{ background: "var(--color-accent)", color: "#001721" }}
            >
              {busy ? "Looking up…" : "Add"}
            </button>
          </div>
        </div>
      </div>
    </div>
  );
}
