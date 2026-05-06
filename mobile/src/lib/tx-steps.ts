//! Derive a plain-English bullet list of "what this transaction will do"
//! from a `TxRiskReport`. The risk view shows these above the warnings so
//! the user has context for the alarms instead of just trusting them.
//!
//! Strategy: drive everything off the balance changes the simulation
//! produced. We don't try to do program-specific decoding (Jupiter swap,
//! Orca route, etc.) here — that's a deeper feature for later. Net
//! balance deltas are enough to give the user a clear story for the
//! common cases (transfer, swap, mint).

import type { TxRiskReport } from "./tx-risk";

/** Format a token amount for display. Mirrors the risk-display formatter. */
function formatTokenAmount(amount: number): string {
  const abs = Math.abs(amount);
  if (abs < 0.000001) return "<0.000001";
  if (abs >= 1000) return abs.toLocaleString("en-US", { maximumFractionDigits: 2 });
  return abs.toFixed(abs < 0.01 ? 6 : abs < 1 ? 4 : 2);
}

/** Format a SOL amount. Tighter than the token formatter. */
function formatSolAmount(amount: number): string {
  const abs = Math.abs(amount);
  if (abs < 0.000001) return "<0.000001";
  if (abs >= 1) return abs.toFixed(4);
  return abs.toFixed(6);
}

function shortMintInline(mint: string): string {
  return `${mint.slice(0, 4)}…${mint.slice(-4)}`;
}

/**
 * Convert the report into a list of plain-English bullets, ordered:
 *   1. Inflows (what you receive — usually the main intent)
 *   2. Outflows (what you pay)
 *   3. SOL net change (when meaningful, i.e. > the typical fee dust)
 *
 * Returns at least one bullet — falls back to a generic "executes on-chain"
 * line when the simulation produced no balance deltas at all (e.g.
 * read-only-style transactions, account creates, etc.).
 */
export function deriveTxSteps(report: TxRiskReport): string[] {
  const steps: string[] = [];

  // Inflows first — these are the result the user is usually looking for.
  for (const c of report.tokenChanges) {
    if (c.amount <= 0) continue;
    const sym = c.symbol || shortMintInline(c.mint);
    steps.push(`Receive ~${formatTokenAmount(c.amount)} ${sym}`);
  }

  // Outflows — what you're giving up.
  for (const c of report.tokenChanges) {
    if (c.amount >= 0) continue;
    const sym = c.symbol || shortMintInline(c.mint);
    steps.push(`Send ~${formatTokenAmount(Math.abs(c.amount))} ${sym}`);
  }

  // SOL movement. Skip changes < 0.001 SOL since those are usually fee
  // dust the user doesn't need to think about. The fee shows up as a
  // tiny SOL outflow — surfacing it as a step would be noise.
  if (report.solChangeSol !== null && Math.abs(report.solChangeSol) >= 0.001) {
    const verb = report.solChangeSol > 0 ? "Receive" : "Send";
    steps.push(`${verb} ~${formatSolAmount(report.solChangeSol)} SOL`);
  }

  if (steps.length === 0) {
    steps.push(
      report.simulationFailed
        ? "Simulation didn't complete — we can't tell what this transaction would do."
        : "This transaction executes on-chain but causes no balance changes for your wallet.",
    );
  }

  return steps;
}
