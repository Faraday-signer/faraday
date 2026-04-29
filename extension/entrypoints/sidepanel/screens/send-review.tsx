import { useState, type CSSProperties } from "react";

import { LinkButton, PanelShell, PrimaryButton } from "../../../src/components/panel-shell";
import { sendRuntimeMessage } from "../../../src/lib/runtime";
import { useNavigation, useRouteOf } from "../../../src/lib/router";
import { buildSolTransfer } from "../../../src/lib/sol-transfer";
import { RPC_URL } from "../../../src/lib/sol-client";
import { colors, fontFamily, font, letterSpacing, radius, space } from "../../../src/lib/theme";
import { analyzeTxRisk, type TxRiskReport, type TxRiskWarning } from "../../../src/lib/tx-risk";
import type { CreateSignSessionResult } from "../../../src/lib/types";
import { useWallet } from "../../../src/lib/use-wallet";

function shortAddress(address: string): string {
  if (address.length <= 12) return address;
  return `${address.slice(0, 4)}…${address.slice(-4)}`;
}

const wrapStyle: CSSProperties = {
  display: "flex",
  flexDirection: "column",
  padding: space.md,
  gap: space.md
};

const rowStyle: CSSProperties = {
  padding: space.md,
  borderRadius: radius.md,
  background: colors.panel,
  border: `1px solid ${colors.border}`,
  display: "flex",
  flexDirection: "column",
  gap: space.xxs
};

const labelStyle: CSSProperties = {
  fontFamily: fontFamily.display,
  fontSize: font.xs,
  letterSpacing: letterSpacing.eyebrow,
  textTransform: "uppercase",
  color: colors.textMuted
};

const primaryValueStyle: CSSProperties = {
  fontFamily: fontFamily.display,
  fontSize: font.xxl,
  letterSpacing: letterSpacing.tight,
  color: colors.text
};

const secondaryValueStyle: CSSProperties = {
  fontFamily: fontFamily.mono,
  fontSize: font.sm,
  color: colors.textMuted
};

type Phase = "idle" | "analyzing" | "risk" | "confirming";

function riskLevelColor(report: TxRiskReport): string {
  if (report.level === "SAFE") return colors.success;
  if (report.level === "WARNING") return colors.warning;
  return colors.error;
}

function proceedLabel(report: TxRiskReport): string {
  if (report.level === "SAFE") return "Confirm with Faraday";
  if (report.level === "WARNING") return "Proceed with caution";
  return "Sign anyway — I accept the risk";
}

function formatAmount(amount: number): string {
  const abs = Math.abs(amount);
  const sign = amount >= 0 ? "+" : "-";
  if (abs < 0.000001) return `${sign}0`;
  if (abs >= 1000) return `${sign}${abs.toLocaleString("en-US", { maximumFractionDigits: 2 })}`;
  return `${sign}${abs.toFixed(abs < 0.01 ? 6 : abs < 1 ? 4 : 2)}`;
}

function BalanceChanges({ report }: { report: TxRiskReport }) {
  const allChanges = report.tokenChanges.slice();

  if (allChanges.length === 0 && report.solChangeSol === null) return null;

  // Show SOL if not already in tokenChanges and simulation succeeded
  const hasSolInChanges = allChanges.some((c) => c.symbol === "SOL");
  if (!hasSolInChanges && report.solChangeSol !== null && !report.simulationFailed) {
    allChanges.push({ mint: "SOL", symbol: "SOL", amount: report.solChangeSol });
  }

  if (allChanges.length === 0) return null;

  return (
    <div style={{
      padding: space.sm,
      borderRadius: radius.md,
      background: colors.panel,
      border: `1px solid ${colors.border}`,
      display: "flex",
      flexDirection: "column",
      gap: space.xxs,
    }}>
      <span style={{ fontFamily: fontFamily.display, fontSize: font.xs, letterSpacing: letterSpacing.eyebrow, textTransform: "uppercase", color: colors.textMuted }}>
        Balance changes
      </span>
      {allChanges.map((c) => (
        <div key={c.mint} style={{ display: "flex", justifyContent: "space-between", alignItems: "baseline" }}>
          <span style={{ fontFamily: fontFamily.mono, fontSize: font.xs, color: colors.textMuted }}>{c.symbol}</span>
          <span style={{ fontFamily: fontFamily.mono, fontSize: font.sm, color: c.amount >= 0 ? colors.success : colors.error }}>
            {formatAmount(c.amount)}
          </span>
        </div>
      ))}
    </div>
  );
}

function RiskBanner({ report }: { report: TxRiskReport }) {
  const color = riskLevelColor(report);

  const levelLabel =
    report.level === "SAFE" ? "Transaction looks safe" :
    report.level === "WARNING" ? "Review warnings before signing" :
    "Potential fraud detected";

  return (
    <div style={{ display: "flex", flexDirection: "column", gap: space.xs }}>
      <div style={{
        padding: space.sm,
        borderRadius: radius.md,
        background: `${color}14`,
        border: `1px solid ${color}`,
      }}>
        <span style={{
          fontFamily: fontFamily.display,
          fontSize: font.xs,
          letterSpacing: letterSpacing.eyebrow,
          textTransform: "uppercase",
          color,
        }}>
          {report.level} — {levelLabel}
        </span>
      </div>

      <BalanceChanges report={report} />

      {report.warnings.map((w, i) => (
        <WarningRow key={i} warning={w} />
      ))}
    </div>
  );
}

function WarningRow({ warning }: { warning: TxRiskWarning }) {
  const color = warning.severity === "critical" ? colors.error : colors.warning;
  return (
    <div style={{
      padding: space.sm,
      borderRadius: radius.md,
      background: `${color}0d`,
      border: `1px solid ${color}`,
      display: "flex",
      flexDirection: "column",
      gap: space.xxs,
    }}>
      <span style={{ fontFamily: fontFamily.display, fontSize: font.xs, color, letterSpacing: letterSpacing.wider }}>
        {warning.title}
      </span>
      <span style={{ fontFamily: fontFamily.mono, fontSize: font.xs, color: colors.textMuted, lineHeight: 1.5 }}>
        {warning.description}
      </span>
    </div>
  );
}

export function SendReviewScreen() {
  const nav = useNavigation();
  const route = useRouteOf("send-review");
  const { pairedPubkey } = useWallet();
  const [phase, setPhase] = useState<Phase>("idle");
  const [error, setError] = useState<string | null>(null);
  const [riskReport, setRiskReport] = useState<TxRiskReport | null>(null);
  const [pendingTxBase64, setPendingTxBase64] = useState<string | null>(null);

  if (!route) return null;

  const { draft } = route;

  async function analyze() {
    if (!pairedPubkey || phase !== "idle") return;
    setError(null);
    setPhase("analyzing");
    try {
      const { txBase64 } = await buildSolTransfer({
        from: pairedPubkey,
        to: draft.recipient,
        amountSol: draft.amountUi,
      });
      const report = await analyzeTxRisk(txBase64, RPC_URL, pairedPubkey);
      setPendingTxBase64(txBase64);
      setRiskReport(report);
      setPhase("risk");
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
      setPhase("idle");
    }
  }

  async function proceed() {
    if (!pairedPubkey || !pendingTxBase64 || phase !== "risk") return;
    setError(null);
    setPhase("confirming");
    try {
      const res = await sendRuntimeMessage<CreateSignSessionResult>({
        type: "faraday:ext-create-sign-session",
        txBase64: pendingTxBase64,
      });
      if (!res.ok) {
        setError(res.error);
        setPhase("risk");
        return;
      }
      nav.push({
        name: "send-sign",
        draft,
        txBase64: pendingTxBase64,
        sessionId: res.data.sessionId,
      });
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
      setPhase("risk");
    }
  }

  function back() {
    setPhase("idle");
    setRiskReport(null);
    setPendingTxBase64(null);
    setError(null);
  }

  const isRiskPhase = (phase === "risk" || phase === "confirming") && riskReport !== null;

  return (
    <PanelShell eyebrow="Send" title="Review">
      <div style={wrapStyle}>
        <div style={rowStyle}>
          <span style={labelStyle}>You're sending</span>
          <span style={primaryValueStyle}>
            {draft.amountUi} {draft.symbol}
          </span>
        </div>

        <div style={rowStyle}>
          <span style={labelStyle}>To</span>
          <span style={{ fontFamily: fontFamily.mono, fontSize: font.md, color: colors.text, wordBreak: "break-all" }}>
            {draft.recipient}
          </span>
          <span style={secondaryValueStyle}>{shortAddress(draft.recipient)}</span>
        </div>

        <div style={rowStyle}>
          <span style={labelStyle}>Network fee (est.)</span>
          <span style={{ fontFamily: fontFamily.mono, fontSize: font.sm, color: colors.text }}>~0.000005 SOL</span>
        </div>

        {isRiskPhase ? (
          <RiskBanner report={riskReport} />
        ) : (
          <div style={{
            padding: space.sm,
            borderRadius: radius.md,
            background: colors.accentSoft,
            border: `1px solid ${colors.accent}`,
            color: colors.accent,
            fontSize: font.xs,
            lineHeight: 1.5,
            fontFamily: fontFamily.mono
          }}>
            You'll see the unsigned transaction as a QR. Scan it with your Faraday, approve on the device, then
            scan the signed response back here.
          </div>
        )}

        {error && (
          <div style={{
            padding: space.sm,
            borderRadius: radius.md,
            background: "rgba(255, 107, 107, 0.08)",
            border: `1px solid ${colors.error}`,
            color: colors.error,
            fontSize: font.xs,
            fontFamily: fontFamily.mono,
            lineHeight: 1.5,
          }}>
            {error}
          </div>
        )}

        <div style={{ display: "flex", flexDirection: "column", alignItems: "center", gap: space.xs, marginTop: space.sm }}>
          {isRiskPhase ? (
            <>
              <PrimaryButton onClick={proceed} disabled={phase !== "risk" || !pairedPubkey}>
                {phase !== "risk" ? "Preparing…" : proceedLabel(riskReport)}
              </PrimaryButton>
              <LinkButton onClick={back}>Back</LinkButton>
            </>
          ) : (
            <>
              <PrimaryButton onClick={analyze} disabled={phase === "analyzing" || !pairedPubkey}>
                {phase === "analyzing" ? "Analyzing transaction…" : "Confirm with Faraday"}
              </PrimaryButton>
              <LinkButton onClick={() => nav.back()}>Edit transaction</LinkButton>
            </>
          )}
        </div>
      </div>
    </PanelShell>
  );
}
