//! Proceed button toned by risk level. SAFE keeps the wallet's standard
//! cyan-accent primary; WARNING swaps to amber filled; DANGER goes to a
//! red filled "stop sign" so the visual weight matches "Sign anyway —
//! I accept the risk."
//!
//! Used by both the sidepanel Send-review and the dapp signing popup
//! after the user has reviewed a `RiskReportView`.

import type { CSSProperties, ReactNode } from "react";

import { riskLevelColor, riskProceedLabel } from "../lib/risk-display";
import { colors, fontFamily, font, letterSpacing, radius, space } from "../lib/theme";
import type { TxRiskLevel } from "../lib/tx-risk";

interface RiskProceedButtonProps {
  level: TxRiskLevel;
  onClick: () => void;
  disabled?: boolean;
  /** Override the level-derived label (e.g. "Preparing…" while busy). */
  label?: ReactNode;
  /** Inline style escape hatch (margin tweaks, max-width, etc.). */
  style?: CSSProperties;
}

const baseStyle: CSSProperties = {
  border: "none",
  borderRadius: radius.md,
  padding: `${space.sm}px ${space.lg}px`,
  fontFamily: fontFamily.display,
  fontSize: font.lg,
  fontWeight: 600,
  letterSpacing: letterSpacing.loose,
  cursor: "pointer",
  textTransform: "uppercase",
  width: "100%",
  maxWidth: 280,
};

function styleFor(level: TxRiskLevel, disabled: boolean): CSSProperties {
  const accent = riskLevelColor(level);
  if (level === "SAFE") {
    return {
      ...baseStyle,
      background: colors.accent,
      color: colors.bg,
      cursor: disabled ? "not-allowed" : "pointer",
      opacity: disabled ? 0.5 : 1,
    };
  }
  // WARNING + DANGER: filled in the level colour with the dark panel bg
  // for text. DANGER's red bg is unmistakable.
  return {
    ...baseStyle,
    background: accent,
    color: colors.bg,
    cursor: disabled ? "not-allowed" : "pointer",
    opacity: disabled ? 0.5 : 1,
    boxShadow: level === "DANGER" ? `0 0 0 2px ${accent}33` : "none",
  };
}

export function RiskProceedButton({
  level,
  onClick,
  disabled = false,
  label,
  style,
}: RiskProceedButtonProps) {
  return (
    <button
      type="button"
      onClick={onClick}
      disabled={disabled}
      style={{ ...styleFor(level, disabled), ...style }}
    >
      {label ?? riskProceedLabel(level)}
    </button>
  );
}
