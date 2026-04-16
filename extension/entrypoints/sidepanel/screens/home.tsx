import { useState, type CSSProperties } from "react";

import { FaradayMark } from "../../../src/lib/brand";
import { useNavigation } from "../../../src/lib/router";
import { formatSol, useWallet } from "../../../src/lib/use-wallet";
import { PanelShell } from "../../../src/components/panel-shell";
import { CLUSTER_LABEL } from "../../../src/lib/sol-client";
import { colors, fontFamily, font, letterSpacing, radius, space } from "../../../src/lib/theme";

function shortAddress(address: string): string {
  if (address.length <= 14) return address;
  return `${address.slice(0, 4)}…${address.slice(-4)}`;
}

const wordmarkStyle: CSSProperties = {
  fontFamily: fontFamily.display,
  fontSize: font.md,
  letterSpacing: letterSpacing.wider,
  color: colors.text,
  textTransform: "uppercase"
};

const networkPillStyle: CSSProperties = {
  display: "inline-flex",
  alignItems: "center",
  padding: `2px ${space.xs}px`,
  borderRadius: radius.pill,
  background: colors.accentSoft,
  color: colors.accent,
  fontFamily: fontFamily.display,
  fontSize: 9,
  letterSpacing: letterSpacing.eyebrow,
  border: `1px solid ${colors.accent}`,
  marginLeft: space.xs
};

const gearStyle: CSSProperties = {
  background: "transparent",
  border: "none",
  color: colors.textMuted,
  padding: space.xxs,
  borderRadius: radius.sm,
  cursor: "pointer",
  display: "inline-flex",
  alignItems: "center"
};

const pairedPillStyle: CSSProperties = {
  display: "inline-flex",
  alignItems: "center",
  gap: space.xs,
  padding: `${space.xxs}px ${space.sm}px`,
  borderRadius: radius.pill,
  background: colors.accentSoft,
  color: colors.accent,
  fontFamily: fontFamily.mono,
  fontSize: font.xs,
  letterSpacing: letterSpacing.loose,
  border: `1px solid ${colors.accent}`,
  cursor: "pointer"
};

const balanceWrapStyle: CSSProperties = {
  display: "flex",
  flexDirection: "column",
  alignItems: "center",
  padding: `${space.xl}px ${space.md}px`,
  gap: 2
};

const heroNumberStyle: CSSProperties = {
  fontFamily: fontFamily.display,
  fontSize: font.hero,
  letterSpacing: letterSpacing.tight,
  color: colors.text,
  lineHeight: 1
};

const heroUnitStyle: CSSProperties = {
  fontFamily: fontFamily.display,
  fontSize: font.xl,
  color: colors.accent,
  letterSpacing: letterSpacing.loose,
  marginLeft: space.xs
};

const heroMetaStyle: CSSProperties = {
  fontFamily: fontFamily.mono,
  fontSize: font.xs,
  color: colors.textMuted,
  marginTop: space.xs,
  letterSpacing: letterSpacing.normal
};

const actionRowStyle: CSSProperties = {
  display: "grid",
  gridTemplateColumns: "1fr 1fr",
  gap: space.sm,
  padding: `0 ${space.md}px`
};

const actionButtonStyle: CSSProperties = {
  background: colors.panel,
  color: colors.text,
  border: `1px solid ${colors.border}`,
  borderRadius: radius.md,
  padding: `${space.sm}px ${space.md}px`,
  fontFamily: fontFamily.display,
  fontSize: font.md,
  letterSpacing: letterSpacing.loose,
  cursor: "pointer",
  textTransform: "uppercase",
  display: "flex",
  alignItems: "center",
  justifyContent: "center",
  gap: space.xs
};

const tokensPlaceholderStyle: CSSProperties = {
  margin: `${space.lg}px ${space.md}px`,
  padding: space.lg,
  borderRadius: radius.md,
  border: `1px dashed ${colors.border}`,
  background: colors.panel,
  color: colors.textMuted,
  fontSize: font.sm,
  textAlign: "center",
  lineHeight: 1.5
};

function GearIcon() {
  return (
    <svg width="18" height="18" viewBox="0 0 18 18" fill="none" xmlns="http://www.w3.org/2000/svg" aria-hidden>
      <path
        d="M6 0H12V2H6ZM2 2H16V4H2ZM2 4H4V14H2ZM14 4H16V14H14ZM0 6H2V12H0ZM16 6H18V12H16ZM8 8H10V10H8ZM2 14H16V16H2ZM6 16H12V18H6Z"
        fill="currentColor"
      />
    </svg>
  );
}

export function HomeScreen() {
  const nav = useNavigation();
  const wallet = useWallet();
  const [justCopied, setJustCopied] = useState(false);

  async function copyPubkey() {
    if (!wallet.pairedPubkey) return;
    try {
      await navigator.clipboard.writeText(wallet.pairedPubkey);
      setJustCopied(true);
      window.setTimeout(() => setJustCopied(false), 1400);
    } catch {
      // no-op
    }
  }

  return (
    <PanelShell
      hideBack
      leading={
        <span style={{ display: "inline-flex", alignItems: "center", gap: space.xs }}>
          <FaradayMark height={18} color={colors.accent} />
          <span style={wordmarkStyle}>FARADAY</span>
          <span style={networkPillStyle}>{CLUSTER_LABEL}</span>
        </span>
      }
      trailing={
        <button
          type="button"
          aria-label="Settings"
          style={gearStyle}
          onClick={() => nav.push({ name: "settings" })}
        >
          <GearIcon />
        </button>
      }
    >
      <div style={{ padding: `${space.md}px ${space.md}px 0`, display: "flex", justifyContent: "center" }}>
        {wallet.pairedPubkey ? (
          <button
            type="button"
            style={pairedPillStyle}
            onClick={copyPubkey}
            aria-label={justCopied ? "Address copied" : "Copy address"}
          >
            {shortAddress(wallet.pairedPubkey)}
            <span aria-hidden>{justCopied ? "✓" : "⎘"}</span>
          </button>
        ) : null}
      </div>

      <div style={balanceWrapStyle}>
        <div style={{ display: "flex", alignItems: "baseline" }}>
          <span style={heroNumberStyle}>{formatSol(wallet.solUiAmount)}</span>
          <span style={heroUnitStyle}>SOL</span>
        </div>
        <div style={heroMetaStyle}>
          {wallet.balanceError
            ? `Error: ${wallet.balanceError}`
            : wallet.balanceLoading
              ? "Fetching…"
              : wallet.solUiAmount === null
                ? "Balance unavailable"
                : `Balance on ${CLUSTER_LABEL.toLowerCase()}`}
        </div>
      </div>

      <div style={actionRowStyle}>
        <button
          type="button"
          style={actionButtonStyle}
          onClick={() => nav.push({ name: "send-compose" })}
        >
          ↗ Send
        </button>
        <button
          type="button"
          style={actionButtonStyle}
          onClick={() => nav.push({ name: "receive" })}
        >
          ↙ Receive
        </button>
      </div>

      <div style={tokensPlaceholderStyle}>
        SPL tokens, activity, and USD values land in the next data-layer pass.
      </div>
    </PanelShell>
  );
}
