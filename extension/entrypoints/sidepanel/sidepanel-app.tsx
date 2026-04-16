import type { CSSProperties } from "react";
import { useEffect, useMemo, useState } from "react";

import { FaradayLogo } from "../../src/lib/brand";
import { isValidSolanaAddress } from "../../src/lib/solana";
import { sendRuntimeMessage } from "../../src/lib/runtime";
import { colors, fontFamily, font, radius, space } from "../../src/lib/theme";
import type { ExtensionState } from "../../src/lib/types";

function shortAddress(address: string): string {
  if (address.length <= 14) {
    return address;
  }
  return `${address.slice(0, 8)}…${address.slice(-6)}`;
}

const shellStyle: CSSProperties = {
  width: "100%",
  minHeight: "100vh",
  margin: 0,
  padding: 0,
  fontFamily: fontFamily.ui,
  background: colors.bg,
  color: colors.text,
  display: "flex",
  flexDirection: "column"
};

const headerStyle: CSSProperties = {
  padding: `${space.md}px ${space.md}px ${space.sm}px`,
  borderBottom: `1px solid ${colors.border}`
};

const tagStyle: CSSProperties = {
  margin: `${space.xs}px 0 0`,
  fontSize: font.xs,
  color: colors.textMuted,
  letterSpacing: 0.4
};

const cardStyle: CSSProperties = {
  border: `1px solid ${colors.border}`,
  borderRadius: radius.md,
  padding: space.sm,
  background: colors.panel
};

const sectionTitleStyle: CSSProperties = {
  margin: `0 0 ${space.xs}px`,
  fontSize: font.md,
  fontWeight: 600,
  letterSpacing: 0.3
};

const helpTextStyle: CSSProperties = {
  margin: `0 0 ${space.xs}px`,
  fontSize: font.xs,
  color: colors.textMuted
};

const primaryButtonStyle: CSSProperties = {
  background: colors.accent,
  color: colors.bg,
  border: "none",
  borderRadius: radius.sm,
  padding: `${space.xs}px ${space.sm}px`,
  fontFamily: fontFamily.ui,
  fontSize: font.sm,
  fontWeight: 600,
  cursor: "pointer",
  letterSpacing: 0.3
};

const ghostButtonStyle: CSSProperties = {
  background: "transparent",
  color: colors.text,
  border: `1px solid ${colors.borderStrong}`,
  borderRadius: radius.sm,
  padding: `${space.xs}px ${space.sm}px`,
  fontFamily: fontFamily.ui,
  fontSize: font.sm,
  cursor: "pointer"
};

const inputStyle: CSSProperties = {
  width: "100%",
  boxSizing: "border-box",
  padding: `${space.xs}px ${space.sm}px`,
  borderRadius: radius.sm,
  border: `1px solid ${colors.borderStrong}`,
  background: colors.bg,
  color: colors.text,
  fontFamily: fontFamily.mono,
  fontSize: font.xs,
  outline: "none"
};

const pairedAddressStyle: CSSProperties = {
  color: colors.accent,
  fontFamily: fontFamily.mono,
  fontSize: font.md,
  letterSpacing: 0.4
};

const originRowStyle: CSSProperties = {
  border: `1px solid ${colors.border}`,
  borderRadius: radius.sm,
  padding: `${space.xs}px ${space.sm}px`,
  display: "flex",
  alignItems: "center",
  justifyContent: "space-between",
  gap: space.xs,
  background: colors.bg
};

export function SidePanelApp() {
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
    <main style={shellStyle}>
      <header style={headerStyle}>
        <FaradayLogo height={24} title="Faraday" />
        <p style={tagStyle}>Watch-only browser relay for the air-gapped signer.</p>
      </header>

      <div style={{ padding: space.md, display: "flex", flexDirection: "column", gap: space.sm }}>
        <section style={cardStyle}>
          <h2 style={sectionTitleStyle}>Paired account</h2>
          <p style={helpTextStyle}>Paste the Solana pubkey shown by your Faraday device.</p>

          {state?.pairedPubkey ? (
            <div style={{ marginBottom: space.xs }}>
              <div style={pairedAddressStyle}>{shortAddress(state.pairedPubkey)}</div>
              <div style={{ marginTop: space.xs, display: "flex", gap: space.xs }}>
                <button type="button" onClick={clearPairing} disabled={saving} style={ghostButtonStyle}>
                  Remove pairing
                </button>
              </div>
            </div>
          ) : (
            <div style={{ marginBottom: space.xs, fontSize: font.xs, color: colors.error }}>
              No paired account yet.
            </div>
          )}

          <input
            value={inputPubkey}
            onChange={(event) => setInputPubkey(event.target.value)}
            placeholder="Paste Solana pubkey"
            spellCheck={false}
            style={inputStyle}
          />

          <div style={{ marginTop: space.xs, display: "flex", gap: space.xs, alignItems: "center" }}>
            <button
              type="button"
              onClick={pairPubkey}
              disabled={saving || inputPubkey.trim().length === 0 || !pubkeyLooksValid}
              style={primaryButtonStyle}
            >
              Pair pubkey
            </button>
            {!pubkeyLooksValid && inputPubkey.trim().length > 0 ? (
              <span style={{ fontSize: font.xs, color: colors.error }}>Invalid format</span>
            ) : null}
          </div>
        </section>

        <section style={cardStyle}>
          <h2 style={sectionTitleStyle}>Approved sites</h2>

          {state?.approvedOrigins?.length ? (
            <div style={{ display: "grid", gap: space.xs }}>
              {state.approvedOrigins.map((origin) => (
                <div key={origin} style={originRowStyle}>
                  <span
                    style={{
                      fontSize: font.xs,
                      color: colors.text,
                      overflow: "hidden",
                      textOverflow: "ellipsis"
                    }}
                  >
                    {origin}
                  </span>
                  <button type="button" onClick={() => revokeOrigin(origin)} style={ghostButtonStyle}>
                    Revoke
                  </button>
                </div>
              ))}

              <button
                type="button"
                onClick={clearOrigins}
                style={{ ...ghostButtonStyle, marginTop: space.xxs }}
              >
                Clear all approvals
              </button>
            </div>
          ) : (
            <p style={{ margin: 0, fontSize: font.xs, color: colors.textMuted }}>
              No sites approved yet. Access requests appear when a dapp calls connect.
            </p>
          )}
        </section>

        {error ? (
          <div
            role="alert"
            style={{
              marginTop: space.xxs,
              padding: `${space.xs}px ${space.sm}px`,
              borderRadius: radius.sm,
              background: "rgba(255, 107, 107, 0.12)",
              border: `1px solid ${colors.error}`,
              color: colors.error,
              fontSize: font.xs
            }}
          >
            {error}
          </div>
        ) : null}
      </div>
    </main>
  );
}
