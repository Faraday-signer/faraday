import type { CSSProperties } from "react";

import { FaradayHeroMark } from "@/lib/brand";
import { PanelShell } from "@/components/panel-shell";
import { colors, fontFamily, font, letterSpacing, space } from "@/lib/theme";

const wrapStyle: CSSProperties = {
  display: "flex",
  flexDirection: "column",
  alignItems: "center",
  padding: space.lg,
  gap: space.sm,
  textAlign: "center"
};

const wordmarkStyle: CSSProperties = {
  fontFamily: fontFamily.display,
  fontSize: font.display,
  letterSpacing: letterSpacing.wider,
  textTransform: "uppercase",
  color: colors.text
};

const versionStyle: CSSProperties = {
  fontFamily: fontFamily.mono,
  fontSize: font.xs,
  color: colors.textMuted
};

const copyStyle: CSSProperties = {
  fontSize: font.sm,
  color: colors.textMuted,
  maxWidth: 280,
  lineHeight: 1.5,
  marginTop: space.sm
};

const linkStyle: CSSProperties = {
  color: colors.accent,
  fontSize: font.xs,
  textDecoration: "underline",
  textUnderlineOffset: 3
};

export function SettingsAboutScreen() {
  return (
    <PanelShell eyebrow="Settings" title="About">
      <div style={wrapStyle}>
        <FaradayHeroMark height={72} />
        <div style={wordmarkStyle}>FARADAY</div>
        <div style={versionStyle}>v0.1.0</div>

        <p style={copyStyle}>
          Air-gapped Solana signing. Your keys live on the device. This extension is a watch-only
          companion that relays signing requests via QR codes.
        </p>

        <div style={{ display: "flex", gap: space.md, marginTop: space.sm }}>
          <a href="https://faraday.to" target="_blank" rel="noreferrer" style={linkStyle}>
            faraday.to
          </a>
          <a href="https://github.com/Faraday-signer/faraday" target="_blank" rel="noreferrer" style={linkStyle}>
            GitHub
          </a>
        </div>
      </div>
    </PanelShell>
  );
}
