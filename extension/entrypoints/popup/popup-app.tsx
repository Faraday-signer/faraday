import type { CSSProperties } from "react";
import { useEffect, useMemo, useState } from "react";

import { isValidSolanaAddress } from "../../src/lib/solana";
import { sendRuntimeMessage } from "../../src/lib/runtime";
import type { ExtensionState } from "../../src/lib/types";

function shortAddress(address: string): string {
  if (address.length <= 14) {
    return address;
  }
  return `${address.slice(0, 8)}...${address.slice(-6)}`;
}

const cardStyle: CSSProperties = {
  border: "1px solid #1f2937",
  borderRadius: 10,
  padding: 12,
  background: "#0b1220"
};

export function PopupApp() {
  const [state, setState] = useState<ExtensionState | null>(null);
  const [inputPubkey, setInputPubkey] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [saving, setSaving] = useState(false);

  const pubkeyLooksValid = useMemo(() => isValidSolanaAddress(inputPubkey.trim()), [inputPubkey]);

  async function refreshState() {
    const response = await sendRuntimeMessage<ExtensionState>({ type: "faraday:get-state" });
    if (!response.ok) {
      setError(response.error);
      return;
    }

    setState(response.data);
    setError(null);
  }

  useEffect(() => {
    void refreshState();
  }, []);

  async function pairPubkey() {
    const trimmed = inputPubkey.trim();
    if (!isValidSolanaAddress(trimmed)) {
      setError("Please enter a valid Solana pubkey.");
      return;
    }

    setSaving(true);
    const response = await sendRuntimeMessage<ExtensionState>({
      type: "faraday:set-paired-pubkey",
      pubkey: trimmed
    });
    setSaving(false);

    if (!response.ok) {
      setError(response.error);
      return;
    }

    setState(response.data);
    setInputPubkey("");
    setError(null);
  }

  async function clearPairing() {
    setSaving(true);
    const response = await sendRuntimeMessage<ExtensionState>({ type: "faraday:clear-paired-pubkey" });
    setSaving(false);

    if (!response.ok) {
      setError(response.error);
      return;
    }

    setState(response.data);
    setError(null);
  }

  async function revokeOrigin(origin: string) {
    const response = await sendRuntimeMessage<ExtensionState>({
      type: "faraday:revoke-origin",
      origin
    });
    if (!response.ok) {
      setError(response.error);
      return;
    }

    setState(response.data);
    setError(null);
  }

  async function clearOrigins() {
    const response = await sendRuntimeMessage<ExtensionState>({
      type: "faraday:clear-approved-origins"
    });
    if (!response.ok) {
      setError(response.error);
      return;
    }

    setState(response.data);
    setError(null);
  }

  return (
    <main
      style={{
        width: 360,
        minHeight: 480,
        margin: 0,
        padding: 14,
        fontFamily: "ui-sans-serif, system-ui, -apple-system, Segoe UI, sans-serif",
        background: "#020617",
        color: "#e2e8f0"
      }}
    >
      <header style={{ marginBottom: 14 }}>
        <h1 style={{ margin: 0, fontSize: 20, letterSpacing: 0.4 }}>Faraday</h1>
        <p style={{ margin: "6px 0 0", fontSize: 13, color: "#94a3b8" }}>
          Watch-only browser relay for the air-gapped signer.
        </p>
      </header>

      <section style={{ ...cardStyle, marginBottom: 12 }}>
        <h2 style={{ margin: "0 0 8px", fontSize: 14 }}>Paired Account</h2>
        <p style={{ margin: "0 0 8px", fontSize: 12, color: "#94a3b8" }}>
          Paste the Solana pubkey shown by your Faraday device.
        </p>

        {state?.pairedPubkey ? (
          <div style={{ marginBottom: 10, fontSize: 13 }}>
            <div style={{ color: "#22d3ee", fontFamily: "ui-monospace, SFMono-Regular, Menlo, monospace" }}>
              {shortAddress(state.pairedPubkey)}
            </div>
            <div style={{ marginTop: 8, display: "flex", gap: 8 }}>
              <button onClick={clearPairing} disabled={saving} style={{ padding: "6px 10px", cursor: "pointer" }}>
                Remove Pairing
              </button>
            </div>
          </div>
        ) : (
          <div style={{ marginBottom: 10, fontSize: 12, color: "#fda4af" }}>No paired account yet.</div>
        )}

        <input
          value={inputPubkey}
          onChange={(event) => setInputPubkey(event.target.value)}
          placeholder="Paste Solana pubkey"
          spellCheck={false}
          style={{
            width: "100%",
            boxSizing: "border-box",
            padding: "8px 10px",
            borderRadius: 8,
            border: "1px solid #334155",
            background: "#0f172a",
            color: "#e2e8f0",
            fontFamily: "ui-monospace, SFMono-Regular, Menlo, monospace",
            fontSize: 12
          }}
        />

        <div style={{ marginTop: 8, display: "flex", gap: 8, alignItems: "center" }}>
          <button
            onClick={pairPubkey}
            disabled={saving || inputPubkey.trim().length === 0 || !pubkeyLooksValid}
            style={{ padding: "6px 10px", cursor: "pointer" }}
          >
            Pair Pubkey
          </button>
          {!pubkeyLooksValid && inputPubkey.trim().length > 0 ? (
            <span style={{ fontSize: 11, color: "#fda4af" }}>Invalid format</span>
          ) : null}
        </div>
      </section>

      <section style={cardStyle}>
        <h2 style={{ margin: "0 0 8px", fontSize: 14 }}>Approved Sites</h2>

        {state?.approvedOrigins?.length ? (
          <div style={{ display: "grid", gap: 8 }}>
            {state.approvedOrigins.map((origin) => (
              <div
                key={origin}
                style={{
                  border: "1px solid #273449",
                  borderRadius: 8,
                  padding: "8px 10px",
                  display: "flex",
                  alignItems: "center",
                  justifyContent: "space-between",
                  gap: 8
                }}
              >
                <span style={{ fontSize: 12, color: "#cbd5e1", overflow: "hidden", textOverflow: "ellipsis" }}>
                  {origin}
                </span>
                <button onClick={() => revokeOrigin(origin)} style={{ padding: "4px 8px", cursor: "pointer" }}>
                  Revoke
                </button>
              </div>
            ))}

            <button onClick={clearOrigins} style={{ marginTop: 4, padding: "6px 10px", cursor: "pointer" }}>
              Clear All Site Approvals
            </button>
          </div>
        ) : (
          <p style={{ margin: 0, fontSize: 12, color: "#94a3b8" }}>
            No sites approved yet. Access requests appear when a dapp calls connect.
          </p>
        )}
      </section>

      {error ? (
        <div style={{ marginTop: 12, color: "#fda4af", fontSize: 12 }} role="alert">
          {error}
        </div>
      ) : null}
    </main>
  );
}
