import type { CSSProperties } from "react";
import { useEffect, useMemo, useRef, useState } from "react";

import { BrowserQRCodeReader } from "@zxing/browser";
import { QRCodeSVG } from "qrcode.react";

import { AnimatedQr } from "@/components/animated-qr";
import {
  CameraBlockedPanel,
  CameraRequestPrompt,
} from "@/components/camera-permission-prompts";
import { RiskProceedButton } from "@/components/risk-proceed-button";
import { RiskReportView } from "@/components/risk-report-view";
import { WhatWillHappen } from "@/components/what-will-happen";
import {
  categorizeCameraError,
  getCameraPermissionState,
} from "@/lib/camera-permission";
import { FaradayLogo } from "@/lib/brand";
import { sendRuntimeMessage } from "@/lib/runtime";
import { FARADAY_SIG_PREFIX, spliceFaradaySignature } from "@/lib/solana";
import { colors, fontFamily, font, radius, space } from "@/lib/theme";
import { type TxRiskReport } from "@/lib/tx-risk";
import type { GetSignSessionResult } from "@/lib/types";
import { encodeTxForQr } from "@/lib/ur-encode";

type Step = "risk" | "display" | "scan";
type ScanState = "starting" | "scanning" | "success" | "error";

const LOG_PREFIX = "[Faraday][sign]";

function debug(message: string, meta?: unknown): void {
  if (meta === undefined) {
    console.debug(`${LOG_PREFIX} ${message}`);
    return;
  }
  console.debug(`${LOG_PREFIX} ${message}`, meta);
}

function warn(message: string, meta?: unknown): void {
  if (meta === undefined) {
    console.warn(`${LOG_PREFIX} ${message}`);
    return;
  }
  console.warn(`${LOG_PREFIX} ${message}`, meta);
}

type BarcodeDetectorLike = {
  detect: (source: ImageBitmapSource) => Promise<Array<{ rawValue?: string }>>;
};

type BarcodeDetectorCtorLike = new (options?: {
  formats?: string[];
}) => BarcodeDetectorLike;

const ZXING_OPTIONS = {
  delayBetweenScanAttempts: 60,
  delayBetweenScanSuccess: 120,
  tryPlayVideoTimeout: 2500
} as const;

const FAST_VIDEO_CONSTRAINTS: MediaStreamConstraints = {
  audio: false,
  video: {
    facingMode: { ideal: "environment" },
    width: { ideal: 640, max: 1280 },
    height: { ideal: 480, max: 720 },
    frameRate: { ideal: 30, max: 60 }
  }
};

function getBarcodeDetectorCtor(): BarcodeDetectorCtorLike | null {
  const candidate = (globalThis as { BarcodeDetector?: unknown }).BarcodeDetector;
  if (typeof candidate === "function") {
    return candidate as BarcodeDetectorCtorLike;
  }
  return null;
}

function hostFromOrigin(origin: string): string {
  try {
    return new URL(origin).host;
  } catch {
    return origin;
  }
}

/**
 * Sentinel origin used by the sidepanel's own Send flow. Mirrored from
 * background.ts (`EXTENSION_ORIGIN`). Kept inline rather than imported
 * because background isn't a runtime module the popup loads.
 */
const SIDEPANEL_ORIGIN = "ext:sidepanel";

function isSidepanelOrigin(origin: string): boolean {
  return origin === SIDEPANEL_ORIGIN;
}

function shortAddress(address: string): string {
  if (address.length <= 14) {
    return address;
  }
  return `${address.slice(0, 6)}…${address.slice(-6)}`;
}

function getSessionId(): string | null {
  const params = new URLSearchParams(window.location.search);
  const value = params.get("session");
  return value ? value.trim() : null;
}

const shellStyle: CSSProperties = {
  minHeight: "100vh",
  margin: 0,
  padding: 0,
  background: colors.bg,
  color: colors.text,
  fontFamily: fontFamily.ui,
  display: "flex",
  flexDirection: "column"
};

const headerStyle: CSSProperties = {
  display: "flex",
  alignItems: "center",
  justifyContent: "space-between",
  padding: `${space.md}px ${space.lg}px`,
  borderBottom: `1px solid ${colors.border}`
};

const closeButtonStyle: CSSProperties = {
  background: "transparent",
  border: `1px solid ${colors.border}`,
  color: colors.textMuted,
  padding: `${space.xxs}px ${space.xs}px`,
  borderRadius: radius.sm,
  cursor: "pointer",
  fontFamily: fontFamily.ui,
  fontSize: font.xs,
  letterSpacing: 0.6
};

const contentStyle: CSSProperties = {
  flex: 1,
  display: "flex",
  flexDirection: "column",
  alignItems: "center",
  justifyContent: "flex-start",
  padding: `${space.xl}px ${space.lg}px`,
  gap: space.lg
};

const titleStyle: CSSProperties = {
  fontSize: font.xxl,
  fontWeight: 600,
  margin: 0,
  letterSpacing: 0.2
};

const subtitleStyle: CSSProperties = {
  margin: 0,
  fontSize: font.md,
  color: colors.textMuted
};

const accentBadgeStyle: CSSProperties = {
  display: "inline-flex",
  alignItems: "center",
  gap: space.xs,
  padding: `${space.xxs}px ${space.xs}px`,
  borderRadius: radius.sm,
  background: colors.accentSoft,
  color: colors.accent,
  fontFamily: fontFamily.mono,
  fontSize: font.xs,
  letterSpacing: 0.4
};

const qrCardStyle: CSSProperties = {
  background: colors.qrSurface,
  padding: space.md,
  borderRadius: radius.lg,
  width: "100%",
  boxSizing: "border-box",
  display: "grid",
  placeItems: "center",
  boxShadow: `0 0 0 1px ${colors.borderStrong}, 0 20px 40px rgba(0, 0, 0, 0.4)`
};

const qrSvgStyle: CSSProperties = {
  width: "100%",
  height: "auto",
  display: "block"
};

const primaryButtonStyle: CSSProperties = {
  background: colors.accent,
  color: colors.bg,
  border: "none",
  borderRadius: radius.md,
  padding: `${space.sm}px ${space.lg}px`,
  fontFamily: fontFamily.ui,
  fontSize: font.lg,
  fontWeight: 600,
  cursor: "pointer",
  letterSpacing: 0.3,
  minWidth: 260
};

const secondaryLinkStyle: CSSProperties = {
  background: "transparent",
  border: "none",
  color: colors.textMuted,
  padding: `${space.xs}px ${space.sm}px`,
  fontFamily: fontFamily.ui,
  fontSize: font.sm,
  cursor: "pointer",
  textDecoration: "underline dotted",
  textUnderlineOffset: 3
};

const videoFrameStyle: CSSProperties = {
  position: "relative",
  width: "100%",
  maxWidth: 420,
  aspectRatio: "1 / 1",
  borderRadius: radius.lg,
  overflow: "hidden",
  background: "#000",
  border: `1px solid ${colors.borderStrong}`
};

const videoStyle: CSSProperties = {
  width: "100%",
  height: "100%",
  objectFit: "cover"
};

const cornerBaseStyle: CSSProperties = {
  position: "absolute",
  width: 28,
  height: 28,
  borderColor: colors.accent,
  borderStyle: "solid",
  borderWidth: 0
};

function cornerStyle(pos: "tl" | "tr" | "bl" | "br"): CSSProperties {
  const base = { ...cornerBaseStyle };
  const offset = 10;
  switch (pos) {
    case "tl":
      return { ...base, top: offset, left: offset, borderTopWidth: 3, borderLeftWidth: 3, borderTopLeftRadius: 4 };
    case "tr":
      return { ...base, top: offset, right: offset, borderTopWidth: 3, borderRightWidth: 3, borderTopRightRadius: 4 };
    case "bl":
      return { ...base, bottom: offset, left: offset, borderBottomWidth: 3, borderLeftWidth: 3, borderBottomLeftRadius: 4 };
    case "br":
      return { ...base, bottom: offset, right: offset, borderBottomWidth: 3, borderRightWidth: 3, borderBottomRightRadius: 4 };
  }
}

function Shell({ children, onCancel }: { children: React.ReactNode; onCancel: () => void }) {
  return (
    <main style={shellStyle}>
      <header style={headerStyle}>
        <FaradayLogo height={22} title="Faraday" />
        <button type="button" onClick={onCancel} style={closeButtonStyle}>
          CANCEL
        </button>
      </header>
      <div style={contentStyle}>{children}</div>
    </main>
  );
}

const riskScrollStyle: CSSProperties = {
  display: "flex",
  flexDirection: "column",
  gap: space.sm,
  width: "100%",
  maxWidth: 420,
  overflowY: "auto",
  maxHeight: "calc(100vh - 120px)",
  paddingBottom: space.md,
};


function RiskScreen({
  session,
  report,
  onProceed,
  onCancel,
}: {
  session: GetSignSessionResult;
  report: TxRiskReport;
  onProceed: () => void;
  onCancel: () => void;
}) {
  return (
    <div style={riskScrollStyle}>
      <div style={{ textAlign: "center" }}>
        <h1 style={titleStyle}>Risk Check</h1>
        <p style={subtitleStyle}>
          from <strong style={{ color: colors.text }}>{hostFromOrigin(session.origin)}</strong>
        </p>
      </div>

      <WhatWillHappen report={report} />
      <RiskReportView report={report} />

      <div style={{ display: "flex", flexDirection: "column", alignItems: "center", gap: space.sm, marginTop: space.xs }}>
        <RiskProceedButton
          level={report.level}
          onClick={onProceed}
          label={report.level === "SAFE" ? "Proceed to QR" : undefined}
        />
        <button type="button" onClick={onCancel} style={secondaryLinkStyle}>
          Cancel
        </button>
      </div>
    </div>
  );
}

function DisplayScreen({
  session,
  onAdvance,
  onCancel
}: {
  session: GetSignSessionResult;
  onAdvance: () => void;
  onCancel: () => void;
}) {
  const isMessage = session.kind === "message";
  const qrValue = isMessage ? session.messageQrBase64 : session.txBase64;
  const qrPayload = useMemo(() => {
    if (!qrValue) {
      return null;
    }
    if (isMessage) {
      return { kind: "static" as const, value: qrValue };
    }
    return encodeTxForQr(qrValue);
  }, [isMessage, qrValue]);
  const [animatedIndex, setAnimatedIndex] = useState(0);

  useEffect(() => {
    setAnimatedIndex(0);
    if (!qrPayload || qrPayload.kind !== "animated") {
      return;
    }
    const timer = window.setInterval(() => {
      setAnimatedIndex((prev) => (prev + 1) % qrPayload.frames.length);
    }, qrPayload.intervalMs);
    return () => {
      window.clearInterval(timer);
    };
  }, [qrPayload]);

  if (!qrValue) {
    return (
      <>
        <h1 style={titleStyle}>Signing unavailable</h1>
        <p style={{ ...subtitleStyle, color: colors.error, textAlign: "center" }}>
          Missing payload for this signing session.
        </p>
        <button type="button" onClick={onCancel} style={secondaryLinkStyle}>
          Cancel
        </button>
      </>
    );
  }

  return (
    <>
      <div style={{ textAlign: "center", display: "flex", flexDirection: "column", alignItems: "center", gap: space.xs }}>
        <h1 style={titleStyle}>{isMessage ? "Sign Message" : "Sign Transaction"}</h1>
        <p style={subtitleStyle}>
          {isSidepanelOrigin(session.origin) ? (
            <>from <strong style={{ color: colors.text }}>Faraday wallet</strong></>
          ) : (
            <>from <strong style={{ color: colors.text }}>{hostFromOrigin(session.origin)}</strong></>
          )}
        </p>
        <span style={accentBadgeStyle}>{shortAddress(session.expectedPubkey)}</span>
      </div>

      <div style={qrCardStyle}>
        {qrPayload && qrPayload.kind === "animated" ? (
          <AnimatedQr
            frames={qrPayload.frames}
            size={480}
            intervalMs={qrPayload.intervalMs}
            level="M"
            bgColor={colors.qrSurface}
            fgColor={colors.qrModule}
            svgStyle={qrSvgStyle}
            showCounter={false}
          />
        ) : (
          <QRCodeSVG
            value={qrValue}
            size={320}
            level="M"
            includeMargin={false}
            bgColor={colors.qrSurface}
            fgColor={colors.qrModule}
            style={qrSvgStyle}
          />
        )}
      </div>

      {qrPayload && qrPayload.kind === "animated" ? (
        <p style={{ ...subtitleStyle, fontFamily: fontFamily.mono, fontSize: font.sm }}>
          Animated UR frame {animatedIndex + 1}/{qrPayload.frames.length}
        </p>
      ) : null}

      <p style={{ ...subtitleStyle, textAlign: "center" }}>
        Scan this QR with your Faraday device.
      </p>

      <div style={{ display: "flex", flexDirection: "column", alignItems: "center", gap: space.sm }}>
        <button type="button" onClick={onAdvance} style={primaryButtonStyle}>
          I&apos;ve scanned → Scan signed
        </button>
        <button type="button" onClick={onCancel} style={secondaryLinkStyle}>
          Cancel
        </button>
      </div>
    </>
  );
}

function ScanScreen({
  onDecoded,
  onBack,
  onCancel
}: {
  onDecoded: (rawValue: string) => Promise<boolean>;
  onBack: () => void;
  onCancel: () => void;
}) {
  const [scanState, setScanState] = useState<ScanState>("starting");
  const [statusText, setStatusText] = useState("Requesting camera access…");
  // The user clicked through "I've scanned → Scan signed" to get here,
  // which is itself a recent gesture — so we attempt camera start
  // immediately for the granted/prompt states. The "denied" branch
  // shows the recovery panel instead of trying and failing silently.
  const [cameraPhase, setCameraPhase] = useState<"checking" | "running" | "denied">("checking");
  const [retryKey, setRetryKey] = useState(0);

  useEffect(() => {
    let cancelled = false;
    void (async () => {
      const state = await getCameraPermissionState();
      if (cancelled) return;
      setCameraPhase(state === "denied" ? "denied" : "running");
    })();
    return () => { cancelled = true; };
  }, [retryKey]);

  const videoRef = useRef<HTMLVideoElement | null>(null);
  const readerRef = useRef<BrowserQRCodeReader | null>(null);
  const controlsRef = useRef<{ stop: () => void } | null>(null);
  const streamRef = useRef<MediaStream | null>(null);
  const animationRef = useRef<number | null>(null);
  const decodeLockedRef = useRef(false);

  function stopActiveScanner(): void {
    try {
      controlsRef.current?.stop();
    } catch {
      // no-op
    }
    controlsRef.current = null;

    if (animationRef.current !== null) {
      window.cancelAnimationFrame(animationRef.current);
      animationRef.current = null;
    }

    if (streamRef.current) {
      for (const track of streamRef.current.getTracks()) {
        try {
          track.stop();
        } catch {
          // no-op
        }
      }
      streamRef.current = null;
    }

    if (videoRef.current) {
      try {
        videoRef.current.pause();
      } catch {
        // no-op
      }
      videoRef.current.srcObject = null;
    }
  }

  async function handleDecoded(rawValue: string, scanner: "native" | "zxing"): Promise<void> {
    if (decodeLockedRef.current) {
      return;
    }
    decodeLockedRef.current = true;
    stopActiveScanner();
    setStatusText("QR decoded. Verifying signed payload…");
    debug("Signed QR decoded", { scanner, size: rawValue.length });

    const ok = await onDecoded(rawValue);
    if (ok) {
      setScanState("success");
      setStatusText("Signed transaction captured. Returning to dapp…");
    } else {
      decodeLockedRef.current = false;
      setScanState("error");
    }
  }

  useEffect(() => {
    let cancelled = false;

    async function startNative(): Promise<void> {
      if (!videoRef.current) {
        throw new Error("Video element not ready.");
      }
      const Ctor = getBarcodeDetectorCtor();
      if (!Ctor) {
        throw new Error("BarcodeDetector unavailable.");
      }

      const stream = await navigator.mediaDevices.getUserMedia(FAST_VIDEO_CONSTRAINTS);
      if (cancelled) {
        for (const track of stream.getTracks()) {
          track.stop();
        }
        return;
      }
      streamRef.current = stream;
      videoRef.current.srcObject = stream;
      await videoRef.current.play();

      const detector = new Ctor({ formats: ["qr_code"] });
      setScanState("scanning");
      setStatusText("Point camera at Faraday screen");
      debug("Native BarcodeDetector started");

      const loop = async () => {
        if (!videoRef.current || decodeLockedRef.current || cancelled) {
          return;
        }
        try {
          const results = await detector.detect(videoRef.current);
          const first = results[0];
          if (first?.rawValue) {
            await handleDecoded(first.rawValue, "native");
            return;
          }
        } catch {
          // keep looping
        }
        animationRef.current = window.requestAnimationFrame(() => {
          void loop();
        });
      };
      animationRef.current = window.requestAnimationFrame(() => {
        void loop();
      });
    }

    async function startZxing(): Promise<void> {
      if (!videoRef.current) {
        throw new Error("Video element not ready.");
      }
      const reader = new BrowserQRCodeReader(undefined, ZXING_OPTIONS);
      readerRef.current = reader;

      const controls = await reader.decodeFromConstraints(
        FAST_VIDEO_CONSTRAINTS,
        videoRef.current,
        (result) => {
          if (result && !cancelled) {
            void handleDecoded(result.getText(), "zxing");
          }
        }
      );
      if (cancelled) {
        controls.stop();
        return;
      }
      controlsRef.current = controls;
      setScanState("scanning");
      setStatusText("Point camera at Faraday screen");
      debug("ZXing scanner started");
    }

    if (cameraPhase !== "running") {
      return () => {
        cancelled = true;
      };
    }

    void (async () => {
      try {
        if (getBarcodeDetectorCtor()) {
          await startNative();
        } else {
          await startZxing();
        }
      } catch (error) {
        if (cancelled) {
          return;
        }
        const { kind, message } = categorizeCameraError(error);
        warn("Failed starting camera scanner", { error: message, kind });
        if (kind === "denied") {
          setCameraPhase("denied");
          return;
        }
        setScanState("error");
        setStatusText(message);
      }
    })();

    return () => {
      cancelled = true;
      stopActiveScanner();
      readerRef.current = null;
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [cameraPhase, retryKey]);

  const statusColor = scanState === "error" ? colors.error : colors.textMuted;

  return (
    <>
      <div style={{ textAlign: "center", display: "flex", flexDirection: "column", alignItems: "center", gap: space.xs }}>
        <h1 style={titleStyle}>Scan Signed QR</h1>
        <p style={subtitleStyle}>Hold your Faraday device up to the camera.</p>
      </div>

      {cameraPhase === "denied" ? (
        <CameraBlockedPanel onRetry={() => setRetryKey((k) => k + 1)} />
      ) : cameraPhase === "checking" ? (
        <p style={{ margin: 0, fontSize: font.sm, color: colors.textMuted, textAlign: "center" }}>
          Checking camera access…
        </p>
      ) : (
        <>
          <div style={videoFrameStyle}>
            <video ref={videoRef} autoPlay muted playsInline style={videoStyle} />
            <span style={cornerStyle("tl")} aria-hidden />
            <span style={cornerStyle("tr")} aria-hidden />
            <span style={cornerStyle("bl")} aria-hidden />
            <span style={cornerStyle("br")} aria-hidden />
          </div>
          <p style={{ margin: 0, fontSize: font.sm, color: statusColor, textAlign: "center" }}>{statusText}</p>
        </>
      )}

      <div style={{ display: "flex", flexDirection: "column", alignItems: "center", gap: space.sm }}>
        <button type="button" onClick={onBack} style={secondaryLinkStyle}>
          ← Back to QR
        </button>
        <button type="button" onClick={onCancel} style={secondaryLinkStyle}>
          Cancel
        </button>
      </div>
    </>
  );
}

export function SignApp() {
  const sessionId = useMemo(() => getSessionId(), []);
  const [session, setSession] = useState<GetSignSessionResult | null>(null);
  const [fatalError, setFatalError] = useState<string | null>(null);
  const [step, setStep] = useState<Step>("risk");

  useEffect(() => {
    if (!sessionId) {
      setFatalError("Missing sign session id.");
      return;
    }

    let cancelled = false;
    void (async () => {
      const response = await sendRuntimeMessage<GetSignSessionResult>({
        type: "faraday:get-sign-session",
        sessionId
      });
      if (cancelled) return;
      if (!response.ok) {
        warn("Failed to load sign session", { sessionId, error: response.error });
        setFatalError(response.error);
        return;
      }
      if (response.data.status !== "pending") {
        warn("Loaded non-pending sign session", {
          sessionId,
          status: response.data.status,
          error: response.data.error
        });
        setFatalError(response.data.error || `Session is ${response.data.status}.`);
        return;
      }
      debug("Loaded sign session", {
        sessionId: response.data.sessionId,
        expectedPubkey: response.data.expectedPubkey,
        origin: response.data.origin
      });
      setSession(response.data);
      // Risk report comes pre-computed from background. Show the risk
      // step only when the analyzer flagged the tx as WARNING or DANGER —
      // for SAFE (or sign-message / no-report sessions) skip straight to
      // the QR display so the user doesn't pay an extra click on the
      // common path.
      const report = response.data.riskReport;
      const needsRiskReview =
        response.data.kind === "tx" && !!report && report.level !== "SAFE";
      setStep(needsRiskReview ? "risk" : "display");
    })();

    return () => {
      cancelled = true;
    };
  }, [sessionId]);

  async function completeSession(signedPayload: string): Promise<boolean> {
    if (!sessionId || !session) {
      return false;
    }

    // Tx path may arrive as a `faraday:sig:` envelope (Pi ships only
    // version + pubkey + 64-byte sig). Splice it into the unsigned tx
    // the extension already holds, ed25519-verify against the message,
    // then fall through to the existing signed-tx validation path.
    const trimmedPayload = signedPayload.trim();
    let txPayload = trimmedPayload;
    if (
      session.kind === "tx" &&
      trimmedPayload.startsWith(FARADAY_SIG_PREFIX) &&
      session.txBase64
    ) {
      try {
        txPayload = spliceFaradaySignature(
          session.txBase64,
          trimmedPayload,
          session.expectedPubkey
        );
      } catch (err) {
        warn("Failed to splice signature envelope", {
          sessionId,
          error: err instanceof Error ? err.message : String(err)
        });
        return false;
      }
    }

    const response =
      session.kind === "message"
        ? await sendRuntimeMessage({
            type: "faraday:complete-sign-message-session",
            sessionId,
            signatureHex: signedPayload
          })
        : await sendRuntimeMessage({
            type: "faraday:complete-sign-session",
            sessionId,
            signedTxBase64: txPayload
          });
    if (!response.ok) {
      warn("Failed to complete sign session", { sessionId, error: response.error });
      return false;
    }
    debug("Completed sign session", { sessionId });
    window.setTimeout(() => {
      window.close();
    }, 300);
    return true;
  }

  async function cancelSession(reason = "Signing canceled by user."): Promise<void> {
    if (!sessionId) {
      return;
    }
    await sendRuntimeMessage({
      type: "faraday:cancel-sign-session",
      sessionId,
      reason
    });
    debug("Canceled sign session", { sessionId, reason });
    window.close();
  }

  if (fatalError) {
    return (
      <main style={shellStyle}>
        <header style={headerStyle}>
          <FaradayLogo height={22} title="Faraday" />
        </header>
        <div style={contentStyle}>
          <h1 style={titleStyle}>Signing unavailable</h1>
          <p style={{ ...subtitleStyle, color: colors.error, textAlign: "center" }}>{fatalError}</p>
        </div>
      </main>
    );
  }

  if (!session) {
    return (
      <main style={shellStyle}>
        <header style={headerStyle}>
          <FaradayLogo height={22} title="Faraday" />
        </header>
        <div style={contentStyle}>
          <p style={subtitleStyle}>Loading signing session…</p>
        </div>
      </main>
    );
  }

  return (
    <Shell onCancel={() => void cancelSession()}>
      {step === "risk" && session.riskReport ? (
        <RiskScreen
          session={session}
          report={session.riskReport}
          onProceed={() => setStep("display")}
          onCancel={() => void cancelSession()}
        />
      ) : step === "display" ? (
        <DisplayScreen
          session={session}
          onAdvance={() => setStep("scan")}
          onCancel={() => void cancelSession()}
        />
      ) : (
        <ScanScreen
          onDecoded={(raw) => completeSession(raw)}
          onBack={() => setStep("display")}
          onCancel={() => void cancelSession()}
        />
      )}
    </Shell>
  );
}
