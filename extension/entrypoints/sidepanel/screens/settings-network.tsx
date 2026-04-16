import type { CSSProperties } from "react";

import { PanelShell } from "../../../src/components/panel-shell";
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
  color: colors.text
};

export function SettingsNetworkScreen() {
  return (
    <PanelShell eyebrow="Settings" title="Network">
      <div style={wrapStyle}>
        <div style={cardStyle}>
          <span style={labelStyle}>Cluster</span>
          <span style={valueStyle}>mainnet-beta</span>
        </div>
        <div style={cardStyle}>
          <span style={labelStyle}>RPC endpoint</span>
          <span style={valueStyle}>https://api.mainnet-beta.solana.com</span>
        </div>
        <p style={{ fontSize: font.xs, color: colors.textDim, lineHeight: 1.5 }}>
          Network switching and custom RPC endpoints land in the data layer PR.
        </p>
      </div>
    </PanelShell>
  );
}
