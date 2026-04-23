import { useEffect, useRef, useState, type CSSProperties } from "react";

import { BrandedQR } from "../../../src/components/branded-qr";
import { LinkButton, PanelShell, PrimaryButton } from "../../../src/components/panel-shell";
import { sendRuntimeMessage } from "../../../src/lib/runtime";
import { useNavigation, useRouteOf } from "../../../src/lib/router";
import { broadcastSignedTx, explorerTxUrl } from "../../../src/lib/sol-transfer";
import { colors, fontFamily, font, letterSpacing, space } from "../../../src/lib/theme";
import type { GetSignResult } from "../../../src/lib/types";

const wrapStyle: CSSProperties = {
  display: "flex",
  flexDirection: "column",
  alignItems: "center",
  padding: space.md,
  gap: space.md
};

const helpStyle: CSSProperties = {
  fontSize: font.sm,
  color: colors.textMuted,
  textAlign: "center",
  maxWidth: 300,
  lineHeight: 1.5
};

const metaStyle: CSSProperties = {
  fontFamily: fontFamily.mono,
  fontSize: font.xs,
  color: colors.textMuted,
  letterSpacing: letterSpacing.normal,
  textAlign: "center"
};

const errorStyle: CSSProperties = {
  ...metaStyle,
  color: colors.error,
};

const linkStyle: CSSProperties = {
  color: colors.accent,
  wordBreak: "break-all",
  fontFamily: fontFamily.mono,
  fontSize: font.xs,
};

type Phase = "idle" | "awaiting-scan" | "broadcasting" | "done" | "error";

const POLL_INTERVAL_MS = 500;

/**
 * Extension-originated Send flow. Shows the unsigned tx as a QR for the
 * Faraday to scan, then on "Scan signed" opens the same sign window the
 * dapp path uses. Once the session completes (sign window scanned the
 * signed QR and validated it), this screen broadcasts via RPC and shows
 * the signature + explorer link.
 */
export function SendSignScreen() {
  const nav = useNavigation();
  const route = useRouteOf("send-sign");

  const [phase, setPhase] = useState<Phase>("idle");
  const [error, setError] = useState<string | null>(null);
  const [signature, setSignature] = useState<string | null>(null);
  const pollTimerRef = useRef<number | null>(null);

  useEffect(() => {
    return () => {
      if (pollTimerRef.current !== null) {
        window.clearInterval(pollTimerRef.current);
        pollTimerRef.current = null;
      }
    };
  }, []);

  if (!route) return null;
  const { draft, txBase64, sessionId } = route;

  async function openSignWindowAndPoll() {
    setError(null);
    const open = await sendRuntimeMessage<{ opened: boolean }>({
      type: "faraday:ext-open-sign-window",
      sessionId,
    });
    if (!open.ok) {
      setError(open.error);
      setPhase("error");
      return;
    }
    setPhase("awaiting-scan");
    pollTimerRef.current = window.setInterval(async () => {
      const result = await sendRuntimeMessage<GetSignResult>({
        type: "faraday:get-sign-result",
        sessionId,
      });
      if (!result.ok) return;
      if (result.data.status === "completed" && result.data.signedTxBase64) {
        stopPolling();
        await broadcast(result.data.signedTxBase64);
      } else if (result.data.status === "failed" || result.data.status === "canceled") {
        stopPolling();
        setError(result.data.error || `Session ${result.data.status}.`);
        setPhase("error");
      }
    }, POLL_INTERVAL_MS);
  }

  function stopPolling() {
    if (pollTimerRef.current !== null) {
      window.clearInterval(pollTimerRef.current);
      pollTimerRef.current = null;
    }
  }

  async function broadcast(signedTxBase64: string) {
    setPhase("broadcasting");
    try {
      const { signature: sig } = await broadcastSignedTx(signedTxBase64);
      setSignature(sig);
      setPhase("done");
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
      setPhase("error");
    }
  }

  async function cancel() {
    stopPolling();
    await sendRuntimeMessage({
      type: "faraday:cancel-sign-session",
      sessionId,
      reason: "User canceled from sidepanel.",
    });
    nav.back();
  }

  // Terminal success state.
  if (phase === "done" && signature) {
    return (
      <PanelShell eyebrow="Send" title="Broadcasted">
        <div style={wrapStyle}>
          <p style={{ ...helpStyle, color: colors.text }}>
            Transaction submitted. It may take a few seconds to confirm.
          </p>
          <a href={explorerTxUrl(signature)} target="_blank" rel="noreferrer" style={linkStyle}>
            {signature}
          </a>
          <PrimaryButton onClick={() => nav.push({ name: "home" })}>Done</PrimaryButton>
        </div>
      </PanelShell>
    );
  }

  return (
    <PanelShell eyebrow="Sign transaction" title="Scan on Faraday">
      <div style={wrapStyle}>
        <BrandedQR
          flow="sign"
          value={txBase64}
          size={320}
          caption={
            <span>
              {draft.amountUi} {draft.symbol} → {draft.recipient.slice(0, 4)}…{draft.recipient.slice(-4)}
            </span>
          }
        />

        <p style={helpStyle}>
          Hold your Faraday up to this QR, review the details on the device, and approve.
        </p>

        {phase === "awaiting-scan" ? (
          <p style={metaStyle}>Waiting for the signed QR scan-back window…</p>
        ) : phase === "broadcasting" ? (
          <p style={metaStyle}>Broadcasting to Solana…</p>
        ) : (
          <p style={metaStyle}>After approval, scan the signed response back here.</p>
        )}

        {error && <p style={errorStyle}>{error}</p>}

        {phase === "idle" || phase === "error" ? (
          <PrimaryButton onClick={openSignWindowAndPoll}>
            {phase === "error" ? "Retry signed scan" : "I've scanned → Scan signed"}
          </PrimaryButton>
        ) : null}

        <LinkButton onClick={cancel}>Cancel</LinkButton>
      </div>
    </PanelShell>
  );
}
