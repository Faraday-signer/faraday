import { useState, type CSSProperties } from "react";

import { ErrorBanner } from "@/components/error-banner";
import { LinkButton, PanelShell, PrimaryButton } from "@/components/panel-shell";
import { sendRuntimeMessage } from "@/lib/runtime";
import { useNavigation, useRouteOf } from "@/lib/router";
import { buildSolTransfer } from "@/lib/sol-transfer";
import { colors, fontFamily, font, letterSpacing, radius, space } from "@/lib/theme";
import type { CreateSignSessionResult } from "@/lib/types";
import { useWallet } from "@/lib/use-wallet";

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

const noticeStyle: CSSProperties = {
  padding: space.sm,
  borderRadius: radius.md,
  background: colors.accentSoft,
  border: `1px solid ${colors.accent}`,
  color: colors.accent,
  fontSize: font.xs,
  lineHeight: 1.5,
  fontFamily: fontFamily.mono
};

export function SendReviewScreen() {
  const nav = useNavigation();
  const route = useRouteOf("send-review");
  const { pairedPubkey } = useWallet();
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  if (!route) return null;

  const { draft } = route;

  async function confirm() {
    if (!pairedPubkey || busy) return;
    setError(null);
    setBusy(true);
    try {
      const { txBase64 } = await buildSolTransfer({
        from: pairedPubkey,
        to: draft.recipient,
        amountSol: draft.amountUi,
      });
      const res = await sendRuntimeMessage<CreateSignSessionResult>({
        type: "faraday:ext-create-sign-session",
        txBase64,
      });
      if (!res.ok) {
        setError(res.error);
        return;
      }
      nav.push({
        name: "send-sign",
        draft,
        txBase64,
        sessionId: res.data.sessionId,
      });
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setBusy(false);
    }
  }

  const errorBanner = error ? (
    <ErrorBanner
      title="Could not prepare transaction"
      message={error}
      onRetry={confirm}
      retrying={busy}
      onDismiss={() => setError(null)}
    />
  ) : null;

  return (
    <PanelShell eyebrow="Send" title="Review" banner={errorBanner}>
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

        <div style={noticeStyle}>
          You'll see the unsigned transaction as a QR. Scan it with your Faraday, approve on the device, then
          scan the signed response back here.
        </div>

        <div style={{ display: "flex", flexDirection: "column", alignItems: "center", gap: space.xs, marginTop: space.sm }}>
          <PrimaryButton onClick={confirm} disabled={busy || !pairedPubkey}>
            {busy ? "Preparing…" : "Confirm with Faraday"}
          </PrimaryButton>
          <LinkButton onClick={() => nav.back()}>Edit transaction</LinkButton>
        </div>
      </div>
    </PanelShell>
  );
}
