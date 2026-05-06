//! Stateless helpers for rendering `TxRiskReport`s consistently across the
//! sidepanel send flow and the dapp signing popup.
//!
//! Both surfaces used to ship near-identical copies of these helpers
//! inline. Keep them here so a tweak to "what colour is DANGER" or "how
//! do we sort warnings" lands in exactly one place.

import { colors } from "./theme";
import type { TxRiskCategory, TxRiskLevel, TxRiskReport, TxRiskWarning } from "./tx-risk";

/** Map a top-level risk verdict to a single accent colour. */
export function riskLevelColor(level: TxRiskLevel): string {
  if (level === "SAFE") return colors.success;
  if (level === "WARNING") return colors.warning;
  return colors.error;
}

/** Map a warning's severity to its accent colour. */
export function severityColor(severity: TxRiskWarning["severity"]): string {
  return severity === "critical" ? colors.error : colors.warning;
}

/**
 * Plain-English banner label that takes the warning categories into
 * account, not just the top-level severity. A "would fail" tx no longer
 * reads as "fraud detected" — it reads as "transaction would fail."
 */
export function riskLevelLabel(level: TxRiskLevel, warnings: TxRiskWarning[] = []): string {
  if (level === "SAFE") return "Transaction looks safe";
  const cats = new Set<TxRiskCategory>(warnings.map((w) => w.category));
  if (cats.has("fraud")) return "Possible fraud — review before signing";
  if (cats.has("failure")) return "Transaction would not succeed — do not sign";
  return "Unusual transaction — review before signing";
}

/** Compact uppercase label for the per-warning category chip. */
export function categoryLabel(category: TxRiskCategory): string {
  if (category === "fraud") return "Fraud";
  if (category === "failure") return "Failure";
  return "Quality";
}

/** Label for the "proceed" button — escalates with severity. */
export function riskProceedLabel(level: TxRiskLevel): string {
  if (level === "SAFE") return "Confirm with Faraday";
  if (level === "WARNING") return "Proceed with caution";
  return "Sign anyway — I accept the risk";
}

/**
 * Single-character sigil shown before the warning title for fast visual
 * triage. Unicode glyphs so we don't ship icon assets just for this.
 */
export function severitySigil(severity: TxRiskWarning["severity"]): string {
  return severity === "critical" ? "⚠" : "!";
}

/**
 * Stable sort: critical warnings first, then non-critical, preserving the
 * original order within each group.
 */
export function sortWarningsBySeverity(warnings: TxRiskWarning[]): TxRiskWarning[] {
  return warnings
    .map((w, i) => ({ w, i }))
    .sort((a, b) => {
      if (a.w.severity === b.w.severity) return a.i - b.i;
      return a.w.severity === "critical" ? -1 : 1;
    })
    .map(({ w }) => w);
}

/**
 * Format a signed balance delta for the balance-changes table. Negative is
 * shown with `-`, positive with `+`. Small fractional amounts widen the
 * decimal places automatically; large amounts use locale grouping.
 */
export function formatBalanceAmount(amount: number): string {
  const abs = Math.abs(amount);
  const sign = amount >= 0 ? "+" : "-";
  if (abs < 0.000001) return `${sign}0`;
  if (abs >= 1000) {
    return `${sign}${abs.toLocaleString("en-US", { maximumFractionDigits: 2 })}`;
  }
  return `${sign}${abs.toFixed(abs < 0.01 ? 6 : abs < 1 ? 4 : 2)}`;
}

/**
 * Append a synthetic SOL row to the token-changes list when simulation
 * succeeded and SOL isn't already tracked. Used by the balance-changes
 * table in `RiskReportView`. Returns a copy — does not mutate.
 */
export function balanceChanges(report: TxRiskReport): { mint: string; symbol: string; amount: number }[] {
  const list = report.tokenChanges.slice();
  const hasSol = list.some((c) => c.symbol === "SOL");
  if (!hasSol && report.solChangeSol !== null && !report.simulationFailed) {
    list.push({ mint: "SOL", symbol: "SOL", amount: report.solChangeSol });
  }
  return list;
}
