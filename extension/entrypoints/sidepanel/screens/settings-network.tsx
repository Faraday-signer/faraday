import type { CSSProperties } from "react";

import { PanelShell } from "../../../src/components/panel-shell";
import { CLUSTER_LABEL, IS_PUBLIC_RPC, RPC_URL, redactRpcUrl } from "../../../src/lib/sol-client";
import { useTokenSettings } from "../../../src/lib/use-tokens";
import { colors, fontFamily, font, letterSpacing, radius, space } from "../../../src/lib/theme";

const wrapStyle: CSSProperties = {
  display: "flex",
  flexDirection: "column",
  padding: space.md,
  gap: space.md
};

const cardStyle: CSSProperties = {
  padding: space.md,
  borderRadius: radius.md,
  background: colors.panel,
  border: `1px solid ${colors.border}`,
  display: "flex",
  flexDirection: "column",
  gap: space.xs
};

const labelStyle: CSSProperties = {
  fontFamily: fontFamily.display,
  fontSize: font.xs,
  letterSpacing: letterSpacing.eyebrow,
  textTransform: "uppercase",
  color: colors.textMuted
};

const valueStyle: CSSProperties = {
  fontFamily: fontFamily.mono,
  fontSize: font.sm,
  color: colors.text,
  wordBreak: "break-all"
};

const noticeStyle: CSSProperties = {
  padding: space.sm,
  borderRadius: radius.md,
  background: "rgba(255, 180, 84, 0.12)",
  border: `1px solid ${colors.warning}`,
  color: colors.warning,
  fontFamily: fontFamily.ui,
  fontSize: font.xs,
  lineHeight: 1.5
};

const footnoteStyle: CSSProperties = {
  fontSize: font.xs,
  color: colors.textDim,
  lineHeight: 1.5
};

const toggleRowStyle: CSSProperties = {
  display: "flex",
  alignItems: "center",
  justifyContent: "space-between",
  gap: space.md
};

const toggleSwitchStyle = (on: boolean): CSSProperties => ({
  appearance: "none",
  WebkitAppearance: "none",
  width: 36,
  height: 20,
  borderRadius: radius.pill,
  background: on ? colors.accent : colors.borderStrong,
  border: "none",
  cursor: "pointer",
  position: "relative",
  transition: "background 120ms ease",
  flexShrink: 0
});

const toggleKnobStyle = (on: boolean): CSSProperties => ({
  position: "absolute",
  top: 2,
  left: on ? 18 : 2,
  width: 16,
  height: 16,
  borderRadius: radius.pill,
  background: colors.bg,
  transition: "left 120ms ease",
  pointerEvents: "none"
});

export function SettingsNetworkScreen() {
  const displayUrl = redactRpcUrl(RPC_URL);
  const { settings, setShowUnverified } = useTokenSettings();

  return (
    <PanelShell eyebrow="Settings" title="Network">
      <div style={wrapStyle}>
        <div style={cardStyle}>
          <span style={labelStyle}>Cluster</span>
          <span style={valueStyle}>{CLUSTER_LABEL.toLowerCase()}-beta</span>
        </div>

        <div style={cardStyle}>
          <span style={labelStyle}>RPC endpoint</span>
          <span style={valueStyle}>{displayUrl}</span>
          <span style={{ ...footnoteStyle, marginTop: 2 }}>
            Configured via <code style={{ fontFamily: fontFamily.mono }}>VITE_RPC_URL</code>
            {" "}at build time.
          </span>
        </div>

        {IS_PUBLIC_RPC ? (
          <div style={noticeStyle}>
            <strong>Public Solana RPC.</strong> Rate-limited — expect 429s under any real load.
            Set <code style={{ fontFamily: fontFamily.mono }}>VITE_RPC_URL</code> in{" "}
            <code style={{ fontFamily: fontFamily.mono }}>extension/.env</code> and rebuild to use
            Helius / QuickNode / your own endpoint.
          </div>
        ) : null}

        <div style={cardStyle}>
          <div style={toggleRowStyle}>
            <span style={labelStyle}>Show unverified tokens</span>
            <button
              type="button"
              role="switch"
              aria-checked={settings.showUnverified}
              aria-label="Show unverified tokens"
              onClick={() => void setShowUnverified(!settings.showUnverified)}
              style={toggleSwitchStyle(settings.showUnverified)}
            >
              <span style={toggleKnobStyle(settings.showUnverified)} />
            </button>
          </div>
          <span style={{ ...footnoteStyle, marginTop: 2 }}>
            Hidden tokens use the Jupiter verified-tag list. Unverified mints are
            often airdrop spam — keep this off unless you're expecting a token
            that hasn't been picked up yet.
          </span>
        </div>

        <p style={footnoteStyle}>
          Runtime cluster switching and a user-editable RPC override land with the full data-layer
          pass.
        </p>
      </div>
    </PanelShell>
  );
}
