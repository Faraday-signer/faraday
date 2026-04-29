import type { CSSProperties } from "react";

import { FaradayHeroMark } from "@/lib/brand";
import { useNavigation } from "@/lib/router";
import {
  PanelShell,
  PrimaryButton,
  SecondaryButton
} from "@/components/panel-shell";
import { colors, fontFamily, font, letterSpacing, space } from "@/lib/theme";

const columnStyle: CSSProperties = {
  flex: 1,
  width: "100%",
  display: "flex",
  flexDirection: "column",
  justifyContent: "space-between",
  alignItems: "center",
  padding: `${space.xl}px ${space.lg}px ${space.md}px`,
  gap: space.lg
};

const heroBlockStyle: CSSProperties = {
  display: "flex",
  flexDirection: "column",
  alignItems: "center",
  width: "100%"
};

const wordmarkStyle: CSSProperties = {
  fontFamily: fontFamily.display,
  fontSize: font.display,
  letterSpacing: letterSpacing.wider,
  color: colors.text,
  textTransform: "uppercase",
  marginTop: space.md
};

const taglineStyle: CSSProperties = {
  marginTop: space.xs,
  fontSize: font.sm,
  color: colors.textMuted,
  textAlign: "center",
  maxWidth: 280,
  lineHeight: 1.5
};

const actionsStyle: CSSProperties = {
  marginTop: space.xl,
  display: "flex",
  flexDirection: "column",
  alignItems: "center",
  gap: space.sm,
  width: "100%"
};

const footerLinkStyle: CSSProperties = {
  fontSize: font.xs,
  color: colors.textDim,
  textDecoration: "none"
};

export function OnboardingScreen() {
  const { push } = useNavigation();

  return (
    <PanelShell hideBack>
      <div style={columnStyle}>
        {/* spacer so the hero block visually centers between top and footer */}
        <span />

        <div style={heroBlockStyle}>
          <FaradayHeroMark height={96} title="Faraday" />

          <div style={wordmarkStyle}>FARADAY</div>

          <p style={taglineStyle}>
            Air-gapped Solana signing. Your keys never touch the internet.
          </p>

          <div style={actionsStyle}>
            <PrimaryButton onClick={() => push({ name: "pair-scan" })}>
              Import wallet
            </PrimaryButton>
            <SecondaryButton onClick={() => push({ name: "pair-paste" })}>
              Paste address
            </SecondaryButton>
          </div>
        </div>

        <a
          href="https://faraday.dev"
          target="_blank"
          rel="noreferrer"
          style={footerLinkStyle}
        >
          Don&apos;t have a Faraday? Learn more →
        </a>
      </div>
    </PanelShell>
  );
}
