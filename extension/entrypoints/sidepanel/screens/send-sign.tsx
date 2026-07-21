import { useEffect, useRef, useState, type CSSProperties } from "react";

import { ErrorBanner } from "@/components/error-banner";
import { LinkButton, PanelShell, PrimaryButton } from "@/components/panel-shell";
import { explainBroadcastError } from "@/lib/broadcast-errors";
import { sendRuntimeMessage } from "@/lib/runtime";
import { useNavigation, useRouteOf } from "@/lib/router";
import { broadcastSignedTx, buildSolTransfer, explorerTxUrl } from "@/lib/sol-transfer";
import { waitForNonceAccountReady } from "@/lib/nonce";
import { setNonceAccount } from "@/lib/storage";
import { recordRecipient } from "@/lib/recipient-history";
import { colors, fontFamily, font, letterSpacing, space } from "@/lib/theme";
import type { CreateSignSessionResult, GetSignResult } from "@/lib/types";
import { useWallet } from "@/lib/use-wallet";

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

const draftHeroStyle: CSSProperties = {
  display: "flex",
  flexDirection: "column",
  alignItems: "center",
  gap: 6,
  paddingTop: space.sm
};

const draftAmountStyle: CSSProperties = {
  fontFamily: fontFamily.display,
  fontSize: font.hero,
  letterSpacing: letterSpacing.tight,
  color: colors.text,
  lineHeight: 1
};

const draftSymbolStyle: CSSProperties = {
  fontFamily: fontFamily.display,
  fontSize: font.xl,
  color: colors.accent,
  letterSpacing: letterSpacing.loose,
  marginLeft: space.xs
};

const draftRecipientStyle: CSSProperties = {
  fontFamily: fontFamily.mono,
  fontSize: font.sm,
  color: colors.textMuted,
  display: "inline-flex",
  alignItems: "center",
  gap: 6
};

const draftArrowStyle: CSSProperties = {
  color: colors.textDim,
  fontFamily: fontFamily.display
};

const linkStyle: CSSProperties = {
  color: colors.accent,
  wordBreak: "break-all",
  fontFamily: fontFamily.mono,
  fontSize: font.xs,
};

type Phase =
  | "opening"
  | "awaiting-scan"
  | "broadcasting"
  | "provisioning"
  | "done"
  | "error";

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

  const wallet = useWallet();

  const [phase, setPhase] = useState<Phase>("opening");
  const [error, setError] = useState<string | null>(null);
  const [signature, setSignature] = useState<string | null>(null);
  const pollTimerRef = useRef<number | null>(null);
  // Tracks which sessionIds we've already opened, so a `nav.replace` from the
  // retry path (which produces a NEW sessionId) re-fires the auto-open effect
  // without React 19 strict-mode causing a double-fire on first mount.
  const openedSessionsRef = useRef<Set<string>>(new Set());

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

  // Auto-open the popup whenever the route's sessionId changes. The Set ref
  // guarantees one open per sessionId — strict-mode re-mounts and route
  // re-renders can't trigger duplicate windows.
  useEffect(() => {
    if (!route) return;
    const id = route.sessionId;
    if (openedSessionsRef.current.has(id)) return;
    openedSessionsRef.current.add(id);
    void openAndPoll(id);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [route?.sessionId]);

  if (!route) return null;
  const { draft, sessionId } = route;
  const provision = route.provision;

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
      // Provisioning session: this was the one-time nonce-account creation
      // tx. Persist the account, wait for it to confirm, then build + sign the
      // actual transfer in a fresh session.
      if (provision) {
        await continueAfterProvision(provision.nonceAccountAddress);
        return;
      }
      setSignature(sig);
      setPhase("done");
      // Record the recipient so the lookalike-destination detector can flag
      // future near-duplicates of this address. Best-effort — never fail
      // the success state because storage misbehaved.
      void recordRecipient(draft.recipient).catch(() => {});
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
      setPhase("error");
    }
  }

  /**
   * After the nonce-account creation tx broadcasts, persist the account, wait
   * for it to become readable on-chain, then build the durable-nonce transfer
   * and hand off to a fresh signing session (replacing this route so the
   * auto-open effect drives the transfer sign).
   */
  async function continueAfterProvision(nonceAccountAddress: string) {
    if (!wallet.pairedPubkey) {
      setError("No paired wallet to finish the transfer with.");
      setPhase("error");
      return;
    }
    setPhase("provisioning");
    try {
      await setNonceAccount(wallet.pairedPubkey, nonceAccountAddress);
      await waitForNonceAccountReady(nonceAccountAddress);

      const { txBase64 } = await buildSolTransfer({
        from: wallet.pairedPubkey,
        to: draft.recipient,
        amountSol: draft.amountUi,
      });
      const res = await sendRuntimeMessage<CreateSignSessionResult>({
        type: "faraday:ext-create-sign-session",
        txBase64,
      });
      if (!res.ok) {
        setError(res.error);
        setPhase("error");
        return;
      }
      nav.replace({
        name: "send-sign",
        draft,
        txBase64,
        sessionId: res.data.sessionId,
      });
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

  /**
   * Rebuild the unsigned tx with a fresh blockhash, register a new sign
   * session, and replace the current route so the auto-open effect drives
   * the rest of the flow with the new session.
   *
   * Why we can't just re-broadcast: the original broadcast failure here is
   * usually a stale blockhash (-32002 with empty logs). The signed bytes
   * already on hand still reference the expired blockhash, so re-sending
   * them changes nothing — the device has to sign a fresh tx.
   */
  async function retry() {
    if (!wallet.pairedPubkey) {
      setError("No paired wallet to rebuild with.");
      setPhase("error");
      return;
    }

    setError(null);
    setSignature(null);
    setPhase("opening");
    stopPolling();

    try {
      const { txBase64: newTxBase64 } = await buildSolTransfer({
        from: wallet.pairedPubkey,
        to: draft.recipient,
        amountSol: draft.amountUi,
      });
      const res = await sendRuntimeMessage<CreateSignSessionResult>({
        type: "faraday:ext-create-sign-session",
        txBase64: newTxBase64,
      });
      if (!res.ok) {
        setError(res.error);
        setPhase("error");
        return;
      }
      // Swap the current route with one carrying the fresh sessionId. The
      // auto-open effect picks it up and runs the popup + poll cycle.
      nav.replace({
        name: "send-sign",
        draft,
        txBase64: newTxBase64,
        sessionId: res.data.sessionId,
      });
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
      setPhase("error");
    }
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

  const errorReport = error ? explainBroadcastError(error) : null;
  const errorBanner = errorReport ? (
    <ErrorBanner
      title="Signing did not complete"
      message={errorReport.summary}
      details={errorReport.details === errorReport.summary ? undefined : errorReport.details}
      onRetry={phase === "error" ? retry : undefined}
      onDismiss={() => setError(null)}
    />
  ) : null;

  return (
    <PanelShell eyebrow="Sign transaction" title="Sign on Faraday" banner={errorBanner}>
      <div style={wrapStyle}>
        <div style={draftHeroStyle}>
          <div style={{ display: "flex", alignItems: "baseline" }}>
            <span style={draftAmountStyle}>{draft.amountUi}</span>
            <span style={draftSymbolStyle}>{draft.symbol}</span>
          </div>
          <div style={draftRecipientStyle}>
            <span style={draftArrowStyle}>→</span>
            <span>{draft.recipient.slice(0, 4)}…{draft.recipient.slice(-4)}</span>
          </div>
        </div>

        <p style={helpStyle}>
          {phase === "opening"
            ? "Opening signing window…"
            : phase === "awaiting-scan"
              ? provision
                ? "First send from this wallet: sign the one-time nonce-account setup on your Faraday. Your transfer signs right after."
                : "Hold your Faraday up to the QR in the popup. The popup will scan the signed response back automatically."
              : phase === "broadcasting"
                ? "Broadcasting to Solana…"
                : phase === "provisioning"
                  ? "Setting up your nonce account so signatures can't expire during the QR relay…"
                  : phase === "error"
                    ? "Signing did not complete."
                    : ""}
        </p>

        <p style={metaStyle}>
          {phase === "awaiting-scan" ? "Waiting for signature…" : null}
          {phase === "broadcasting" ? "Sending transaction…" : null}
          {phase === "provisioning" ? "Waiting for the nonce account to confirm…" : null}
        </p>

        <LinkButton onClick={cancel}>Cancel</LinkButton>
      </div>
    </PanelShell>
  );
}
