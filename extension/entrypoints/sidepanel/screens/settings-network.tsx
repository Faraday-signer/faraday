import type { CSSProperties } from "react";

import { PanelShell } from "../../../src/components/panel-shell";
import { CLUSTER_LABEL, IS_PUBLIC_RPC, RPC_URL, redactRpcUrl } from "../../../src/lib/sol-client";
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

export function SettingsNetworkScreen() {
  const displayUrl = redactRpcUrl(RPC_URL);

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

        <p style={footnoteStyle}>
          Runtime cluster switching and a user-editable RPC override land with the full data-layer
          pass.
        </p>
      </div>
    </PanelShell>
  );
}
