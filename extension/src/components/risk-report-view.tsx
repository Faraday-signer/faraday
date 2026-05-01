//! Shared renderer for `TxRiskReport`. Used by the sidepanel Send-review
//! screen and the dapp signing popup so both surfaces show identical
//! risk UI.
//!
//! Composition (top to bottom):
//!   1. Level banner — full-width, accent-coloured, level + plain-English
//!      summary.
//!   2. Balance changes table — pre/post token deltas.
//!   3. Warning rows — sorted critical-first; each row has a severity
//!      sigil, severity-coloured title, and a description tinted to match.
//!
//! Visual notes (vs. the pre-extraction inline versions):
//!   - Warning row backgrounds went from 5% to 12% (warning) / 18%
//!     (critical) so a glance separates the two severities.
//!   - Critical rows pick up a 4px coloured left strip to read as
//!     "stop-sign" without growing borders.
//!   - Description text is tinted to ~85% of severity colour rather than
//!     the previous flat muted gray, so the *reason* stays connected to
//!     the *severity* visually.
//!   - Severity sigil (⚠ / !) renders inside a small filled chip in the
//!     severity colour for fast triage.

import { useState, type CSSProperties } from "react";

import { colors, fontFamily, font, letterSpacing, radius, space } from "../lib/theme";
import {
  balanceChanges,
  categoryLabel,
  formatBalanceAmount,
  riskLevelColor,
  riskLevelLabel,
  severityColor,
  severitySigil,
  sortWarningsBySeverity,
} from "../lib/risk-display";
import type { TxRiskReport, TxRiskWarning } from "../lib/tx-risk";

interface RiskReportViewProps {
  report: TxRiskReport;
  /**
   * Optional className escape hatch for the wrapping flex column. Both
   * surfaces use their own gap / max-width, so the component doesn't
   * impose those.
   */
  style?: CSSProperties;
}

const wrapStyle: CSSProperties = {
  display: "flex",
  flexDirection: "column",
  gap: space.xs,
  width: "100%",
};

/* ------------------------------------------------------------------------- */
/* Level banner                                                              */
/* ------------------------------------------------------------------------- */

function LevelBanner({ report }: { report: TxRiskReport }) {
  const color = riskLevelColor(report.level);
  return (
    <div
      style={{
        padding: `${space.xs}px ${space.sm}px`,
        borderRadius: radius.md,
        background: `${color}1f`,
        border: `1px solid ${color}`,
        display: "flex",
        alignItems: "center",
        gap: space.xs,
      }}
    >
      <span
        style={{
          fontFamily: fontFamily.display,
          fontSize: font.xs,
          letterSpacing: letterSpacing.eyebrow,
          textTransform: "uppercase",
          color,
        }}
      >
        {report.level}
      </span>
      <span
        style={{
          fontFamily: fontFamily.ui,
          fontSize: font.sm,
          color: colors.text,
          opacity: 0.9,
        }}
      >
        {riskLevelLabel(report.level, report.warnings)}
      </span>
    </div>
  );
}

function CategoryChip({ warning }: { warning: TxRiskWarning }) {
  const color = severityColor(warning.severity);
  return (
    <span
      style={{
        display: "inline-flex",
        alignItems: "center",
        padding: `1px ${space.xs}px`,
        borderRadius: radius.pill,
        background: `${color}33`,
        color,
        fontFamily: fontFamily.display,
        fontSize: 9,
        letterSpacing: letterSpacing.eyebrow,
        textTransform: "uppercase",
        flexShrink: 0,
      }}
    >
      {categoryLabel(warning.category)}
    </span>
  );
}

/* ------------------------------------------------------------------------- */
/* Balance changes                                                           */
/* ------------------------------------------------------------------------- */

function BalanceChangesTable({ report }: { report: TxRiskReport }) {
  const rows = balanceChanges(report);
  if (rows.length === 0) return null;

  return (
    <div
      style={{
        padding: space.sm,
        borderRadius: radius.md,
        background: colors.panel,
        border: `1px solid ${colors.border}`,
        display: "flex",
        flexDirection: "column",
        gap: space.xxs,
      }}
    >
      <span
        style={{
          fontFamily: fontFamily.display,
          fontSize: font.xs,
          letterSpacing: letterSpacing.eyebrow,
          textTransform: "uppercase",
          color: colors.textMuted,
        }}
      >
        Balance changes
      </span>
      {rows.map((c) => (
        <div
          key={c.mint}
          style={{ display: "flex", justifyContent: "space-between", alignItems: "baseline" }}
        >
          <span style={{ fontFamily: fontFamily.mono, fontSize: font.xs, color: colors.textMuted }}>
            {c.symbol}
          </span>
          <span
            style={{
              fontFamily: fontFamily.mono,
              fontSize: font.sm,
              color: c.amount >= 0 ? colors.success : colors.error,
            }}
          >
            {formatBalanceAmount(c.amount)}
          </span>
        </div>
      ))}
    </div>
  );
}

/* ------------------------------------------------------------------------- */
/* Warning rows                                                              */
/* ------------------------------------------------------------------------- */

function SeverityChip({ severity }: { severity: TxRiskWarning["severity"] }) {
  const color = severityColor(severity);
  return (
    <span
      aria-hidden
      style={{
        display: "inline-flex",
        alignItems: "center",
        justifyContent: "center",
        width: 20,
        height: 20,
        borderRadius: radius.pill,
        background: color,
        color: colors.bg,
        fontFamily: fontFamily.display,
        fontSize: 12,
        fontWeight: 700,
        flexShrink: 0,
        // Visual weight tuning — the sigil glyph reads small inside the chip,
        // so nudge it up by a hair.
        lineHeight: 1,
      }}
    >
      {severitySigil(severity)}
    </span>
  );
}

function WarningRow({ warning }: { warning: TxRiskWarning }) {
  const [showDetails, setShowDetails] = useState(false);
  const color = severityColor(warning.severity);
  // Critical rows get a heavier fill (~18%) and a thicker accent strip on
  // the left so they're unmistakable next to a non-critical (~12%).
  const isCritical = warning.severity === "critical";
  const bg = isCritical ? `${color}2e` : `${color}1f`;
  const accentStripe = isCritical ? `4px solid ${color}` : `2px solid ${color}`;
  const indent = 28; // chip width 20 + gap 8

  const detailsToggleStyle: CSSProperties = {
    background: "transparent",
    border: "none",
    padding: 0,
    color,
    fontFamily: fontFamily.display,
    fontSize: 10,
    letterSpacing: letterSpacing.eyebrow,
    textTransform: "uppercase",
    cursor: "pointer",
    textDecoration: "underline dotted",
    textUnderlineOffset: 3,
    alignSelf: "flex-start",
    marginLeft: indent,
  };

  const detailsBoxStyle: CSSProperties = {
    fontFamily: fontFamily.mono,
    fontSize: font.xs,
    color: `${color}d9`,
    background: "rgba(0, 0, 0, 0.3)",
    border: `1px solid ${color}`,
    borderRadius: radius.sm,
    padding: space.xs,
    maxHeight: 160,
    overflow: "auto",
    wordBreak: "break-all",
    whiteSpace: "pre-wrap",
    marginLeft: indent,
  };

  return (
    <div
      style={{
        padding: `${space.sm}px ${space.sm}px ${space.sm}px ${space.md}px`,
        borderRadius: radius.md,
        background: bg,
        border: `1px solid ${color}`,
        borderLeft: accentStripe,
        display: "flex",
        flexDirection: "column",
        gap: space.xxs,
      }}
    >
      <div style={{ display: "flex", alignItems: "center", gap: space.xs, flexWrap: "wrap" }}>
        <SeverityChip severity={warning.severity} />
        <span
          style={{
            fontFamily: fontFamily.display,
            fontSize: font.sm,
            color,
            letterSpacing: letterSpacing.loose,
            lineHeight: 1.2,
          }}
        >
          {warning.title}
        </span>
        <CategoryChip warning={warning} />
      </div>
      <span
        style={{
          fontFamily: fontFamily.ui,
          fontSize: font.xs,
          // Tint description text in the severity colour at ~85% so the
          // reason text is connected to severity, not flat muted gray.
          color: `${color}d9`,
          lineHeight: 1.5,
          marginLeft: indent,
        }}
      >
        {warning.description}
      </span>
      <div
        style={{
          fontFamily: fontFamily.ui,
          fontSize: font.xs,
          color: colors.textMuted,
          lineHeight: 1.55,
          marginLeft: indent,
          marginTop: 2,
          paddingLeft: space.sm,
          borderLeft: `2px solid ${color}66`,
        }}
      >
        <span
          style={{
            display: "block",
            fontFamily: fontFamily.display,
            fontSize: 9,
            letterSpacing: letterSpacing.eyebrow,
            textTransform: "uppercase",
            color: `${color}b3`,
            marginBottom: 2,
          }}
        >
          Why this matters
        </span>
        {warning.explanation}
      </div>
      {warning.details ? (
        <>
          <button
            type="button"
            style={detailsToggleStyle}
            onClick={() => setShowDetails((prev) => !prev)}
            aria-expanded={showDetails}
          >
            {showDetails ? "Hide details ▴" : "Show details ▾"}
          </button>
          {showDetails ? <div style={detailsBoxStyle}>{warning.details}</div> : null}
        </>
      ) : null}
    </div>
  );
}

/* ------------------------------------------------------------------------- */
/* Public                                                                    */
/* ------------------------------------------------------------------------- */

export function RiskReportView({ report, style }: RiskReportViewProps) {
  const sorted = sortWarningsBySeverity(report.warnings);

  return (
    <div style={{ ...wrapStyle, ...style }}>
      <LevelBanner report={report} />
      <BalanceChangesTable report={report} />
      {sorted.map((w, i) => (
        <WarningRow key={i} warning={w} />
      ))}
    </div>
  );
}
