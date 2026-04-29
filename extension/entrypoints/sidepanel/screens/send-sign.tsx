import { useEffect, useRef, useState, type CSSProperties } from "react";

import { ErrorBanner } from "@/components/error-banner";
import { LinkButton, PanelShell, PrimaryButton } from "@/components/panel-shell";
import { explainBroadcastError } from "@/lib/broadcast-errors";
import { sendRuntimeMessage } from "@/lib/runtime";
import { useNavigation, useRouteOf } from "@/lib/router";
import { broadcastSignedTx, explorerTxUrl } from "@/lib/sol-transfer";
import { colors, fontFamily, font, letterSpacing, space } from "@/lib/theme";
import type { GetSignResult } from "@/lib/types";

const wrapStyle: CSSProperties = {
  display: "flex",
  flexDirection: "column",
  alignItems: "center",
  padding: space.md,
  gap: space.md,
  textAlign: "center"
};

const helpStyle: CSSProperties = {
  fontSize: font.sm,
  color: colors.textMuted,
  textAlign: "center",
  maxWidth: 320,
  lineHeight: 1.5
};

const metaStyle: CSSProperties = {
  fontFamily: fontFamily.mono,
  fontSize: font.xs,
  color: colors.textMuted,
  letterSpacing: letterSpacing.normal,
  textAlign: "center"
};

const draftLineStyle: CSSProperties = {
  fontFamily: fontFamily.display,
  fontSize: font.lg,
  color: colors.text,
  letterSpacing: letterSpacing.loose
};

const linkStyle: CSSProperties = {
  color: colors.accent,
  wordBreak: "break-all",
  fontFamily: fontFamily.mono,
  fontSize: font.xs,
};

type Phase = "opening" | "awaiting-scan" | "broadcasting" | "done" | "error";

const POLL_INTERVAL_MS = 500;

/**
 * Extension-originated Send flow — popup variant.
 *
 * The sidepanel itself doesn't render the unsigned QR anymore. The popup at
 * sign.html (the same one the dapp signing path uses) hosts the QR display
 * + scan-back. This screen auto-opens that popup on mount, polls for the
 * session result, and on success broadcasts via RPC and shows the signature.
 *
 * Design choice: a single signing surface for both dapps and sidepanel
 * means one camera path, one set of QR-sizing knobs to tune, and a 480 px
 * QR instead of the sidepanel's cramped 320 px (which the Faraday camera
 * struggles to resolve at typical hand-held distance).
 */
export function SendSignScreen() {
  const nav = useNavigation();
  const route = useRouteOf("send-sign");

  const [phase, setPhase] = useState<Phase>("opening");
  const [error, setError] = useState<string | null>(null);
  const [signature, setSignature] = useState<string | null>(null);
  const pollTimerRef = useRef<number | null>(null);
  const startedRef = useRef(false);

  function stopPolling() {
    if (pollTimerRef.current !== null) {
      window.clearInterval(pollTimerRef.current);
      pollTimerRef.current = null;
    }
  }

  useEffect(() => {
    return () => {
      stopPolling();
    };
  }, []);

  // Auto-open the popup once on mount. `startedRef` guards against the
  // double-fire we'd otherwise get from React 19 strict-mode re-mount.
  useEffect(() => {
    if (!route || startedRef.current) return;
    startedRef.current = true;
    void openAndPoll(route.sessionId);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [route?.sessionId]);

  if (!route) return null;
  const { draft, sessionId } = route;

  async function openAndPoll(id: string) {
    setError(null);
    setPhase("opening");

    const open = await sendRuntimeMessage<{ opened: boolean }>({
      type: "faraday:ext-open-sign-window",
      sessionId: id,
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
        sessionId: id,
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

  async function retry() {
    startedRef.current = true;
    await openAndPoll(sessionId);
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
    <PanelShell eyebrow="Sign transaction" title="Sign on Faraday">
      <div style={wrapStyle}>
        <p style={draftLineStyle}>
          {draft.amountUi} {draft.symbol} → {draft.recipient.slice(0, 4)}…{draft.recipient.slice(-4)}
        </p>

        <p style={helpStyle}>
          {phase === "opening"
            ? "Opening signing window…"
            : phase === "awaiting-scan"
              ? "Hold your Faraday up to the QR in the popup. The popup will scan the signed response back automatically."
              : phase === "broadcasting"
                ? "Broadcasting to Solana…"
                : phase === "error"
                  ? "Signing did not complete."
                  : ""}
        </p>

        <p style={metaStyle}>
          {phase === "awaiting-scan" ? "Waiting for signature…" : null}
          {phase === "broadcasting" ? "Sending transaction…" : null}
        </p>

        {error ? (
          (() => {
            const { summary, details } = explainBroadcastError(error);
            return (
              <ErrorBanner
                title="Signing did not complete"
                message={summary}
                details={details === summary ? undefined : details}
                onRetry={phase === "error" ? retry : undefined}
              />
            );
          })()
        ) : null}

        <LinkButton onClick={cancel}>Cancel</LinkButton>
      </div>
    </PanelShell>
  );
}
