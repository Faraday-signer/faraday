import { useState, type CSSProperties } from "react";

import { ErrorBanner } from "../../../src/components/error-banner";
import { PanelShell } from "../../../src/components/panel-shell";
import { useNavigation } from "../../../src/lib/router";
import { sendRuntimeMessage } from "../../../src/lib/runtime";
import type { ExtensionState } from "../../../src/lib/types";
import { colors, fontFamily, font, letterSpacing, radius, space } from "../../../src/lib/theme";

const wrapStyle: CSSProperties = {
  flex: 1,
  display: "flex",
  flexDirection: "column",
  justifyContent: "space-between",
  padding: space.md,
  gap: space.md
};

const listStyle: CSSProperties = {
  display: "flex",
  flexDirection: "column",
  gap: space.xxs
};

const rowStyle: CSSProperties = {
  display: "flex",
  alignItems: "center",
  justifyContent: "space-between",
  padding: `${space.sm}px ${space.md}px`,
  borderRadius: radius.md,
  background: colors.panel,
  border: `1px solid ${colors.border}`,
  color: colors.text,
  fontFamily: fontFamily.ui,
  fontSize: font.md,
  cursor: "pointer",
  width: "100%",
  textAlign: "left"
};

const rowLabelStyle: CSSProperties = {
  fontFamily: fontFamily.display,
  letterSpacing: letterSpacing.loose,
  textTransform: "uppercase",
  fontSize: font.sm
};

const chevronStyle: CSSProperties = {
  color: colors.textMuted,
  fontFamily: fontFamily.display
};

const disconnectBlockStyle: CSSProperties = {
  display: "flex",
  flexDirection: "column",
  gap: space.xs,
  paddingTop: space.md,
  borderTop: `1px solid ${colors.border}`
};

const disconnectButtonStyle: CSSProperties = {
  width: "100%",
  background: "transparent",
  color: colors.error,
  border: `1px solid ${colors.error}`,
  borderRadius: radius.md,
  padding: `${space.sm}px ${space.md}px`,
  fontFamily: fontFamily.display,
  fontSize: font.md,
  letterSpacing: letterSpacing.loose,
  textTransform: "uppercase",
  cursor: "pointer"
};

const disconnectHelpStyle: CSSProperties = {
  fontSize: font.xs,
  color: colors.textDim,
  textAlign: "center",
  lineHeight: 1.4
};

const confirmHelpStyle: CSSProperties = {
  ...disconnectHelpStyle,
  color: colors.warning
};

const SECTIONS = [
  { name: "settings-device", label: "Paired device" },
  { name: "settings-origins", label: "Approved sites" },
  { name: "settings-network", label: "Network" },
  { name: "settings-about", label: "About" }
] as const;

export function SettingsScreen() {
  const nav = useNavigation();
  const [confirming, setConfirming] = useState(false);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  async function disconnect() {
    setBusy(true);
    setError(null);
    const r = await sendRuntimeMessage<ExtensionState>({ type: "faraday:clear-paired-pubkey" });
    setBusy(false);
    if (!r.ok) {
      setError(r.error);
      return;
    }
    nav.reset({ name: "onboarding" });
  }

  return (
    <PanelShell eyebrow="Settings" title="Settings">
      <div style={wrapStyle}>
        <div style={listStyle}>
          {SECTIONS.map((s) => (
            <button
              key={s.name}
              type="button"
              style={rowStyle}
              onClick={() => nav.push({ name: s.name })}
            >
              <span style={rowLabelStyle}>{s.label}</span>
              <span style={chevronStyle}>→</span>
            </button>
          ))}
        </div>

        <div style={disconnectBlockStyle}>
          {confirming ? (
            <>
              <p style={confirmHelpStyle}>
                Remove pairing? Your device and keys are untouched — this just forgets the public key
                in this browser.
              </p>
              <div style={{ display: "flex", gap: space.xs }}>
                <button
                  type="button"
                  style={{
                    ...disconnectButtonStyle,
                    background: colors.error,
                    color: colors.bg,
                    flex: 1
                  }}
                  onClick={disconnect}
                  disabled={busy}
                >
                  {busy ? "Disconnecting…" : "Confirm disconnect"}
                </button>
                <button
                  type="button"
                  style={{
                    ...disconnectButtonStyle,
                    borderColor: colors.borderStrong,
                    color: colors.textMuted,
                    flex: 1
                  }}
                  onClick={() => setConfirming(false)}
                  disabled={busy}
                >
                  Cancel
                </button>
              </div>
            </>
          ) : (
            <>
              <button
                type="button"
                style={disconnectButtonStyle}
                onClick={() => setConfirming(true)}
              >
                ⊘ Disconnect
              </button>
              <p style={disconnectHelpStyle}>Removes pairing from this browser.</p>
            </>
          )}
          {error ? (
            <ErrorBanner
              title="Disconnect failed"
              message={error}
              onRetry={disconnect}
              retrying={busy}
              onDismiss={() => setError(null)}
            />
          ) : null}
        </div>
      </div>
    </PanelShell>
  );
}
