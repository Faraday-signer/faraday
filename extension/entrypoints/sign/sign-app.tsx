import type { CSSProperties } from "react";
import { useEffect, useMemo, useRef, useState } from "react";

import { BrowserQRCodeReader } from "@zxing/browser";
import { QRCodeSVG } from "qrcode.react";

import { AnimatedQr } from "../../src/components/animated-qr";
import { FaradayLogo } from "../../src/lib/brand";
import { sendRuntimeMessage } from "../../src/lib/runtime";
import { FARADAY_SIG_PREFIX, spliceFaradaySignature } from "../../src/lib/solana";
import { colors, fontFamily, font, radius, space } from "../../src/lib/theme";
import { type TxRiskReport, type TxRiskWarning } from "../../src/lib/tx-risk";
import type { GetSignSessionResult } from "../../src/lib/types";
import { encodeTxForQr } from "../../src/lib/ur-encode";

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

const riskBannerStyle = (color: string): CSSProperties => ({
  padding: space.sm,
  borderRadius: radius.md,
  background: `${color}14`,
  border: `1px solid ${color}`,
  display: "flex",
  flexDirection: "column",
  gap: 4,
});

const riskWarningStyle = (color: string): CSSProperties => ({
  padding: space.sm,
  borderRadius: radius.md,
  background: `${color}0d`,
  border: `1px solid ${color}`,
  display: "flex",
  flexDirection: "column",
  gap: 4,
});

function WarningItem({ warning }: { warning: TxRiskWarning }) {
  const color = warning.severity === "critical" ? colors.error : colors.warning;
  return (
    <div style={riskWarningStyle(color)}>
      <span style={{ fontFamily: fontFamily.mono, fontSize: font.xs, color, letterSpacing: 0.6 }}>
        {warning.title}
      </span>
      <span style={{ fontFamily: fontFamily.mono, fontSize: font.xs, color: colors.textMuted, lineHeight: 1.5 }}>
        {warning.description}
      </span>
    </div>
  );
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
  const hasSolInChanges = allChanges.some((c) => c.symbol === "SOL");
  if (!hasSolInChanges && report.solChangeSol !== null && !report.simulationFailed) {
    allChanges.push({ mint: "SOL", symbol: "SOL", amount: report.solChangeSol });
  }
  if (allChanges.length === 0) return null;
  return (
    <div style={{ padding: space.sm, borderRadius: radius.md, background: colors.panel, border: `1px solid ${colors.borderStrong}`, display: "flex", flexDirection: "column", gap: 6 }}>
      <span style={{ fontFamily: fontFamily.mono, fontSize: font.xs, color: colors.textDim, letterSpacing: 0.8, textTransform: "uppercase" }}>
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
  const levelColor =
    report.level === "SAFE" ? colors.success :
    report.level === "WARNING" ? colors.warning :
    colors.error;

  const levelLabel =
    report.level === "SAFE" ? "Transaction looks safe" :
    report.level === "WARNING" ? "Review warnings before signing" :
    "Potential fraud detected";

  const proceedLabel =
    report.level === "SAFE" ? "Proceed to QR" :
    report.level === "WARNING" ? "Proceed with caution" :
    "Sign anyway — I accept the risk";

  return (
    <div style={riskScrollStyle}>
      <div style={{ textAlign: "center" }}>
        <h1 style={titleStyle}>Risk Check</h1>
        <p style={subtitleStyle}>
          from <strong style={{ color: colors.text }}>{hostFromOrigin(session.origin)}</strong>
        </p>
      </div>

      <div style={riskBannerStyle(levelColor)}>
        <span style={{
          fontFamily: fontFamily.mono,
          fontSize: font.xs,
          color: levelColor,
          letterSpacing: 0.8,
          textTransform: "uppercase",
        }}>
          {report.level} — {levelLabel}
        </span>
      </div>

      <BalanceChanges report={report} />

      {report.warnings.map((w, i) => (
        <WarningItem key={i} warning={w} />
      ))}

      <div style={{ display: "flex", flexDirection: "column", alignItems: "center", gap: space.sm, marginTop: space.xs }}>
        <button
          type="button"
          onClick={onProceed}
          style={{
            background: levelColor,
            color: colors.bg,
            border: "none",
            borderRadius: radius.pill,
            padding: `${space.sm}px ${space.xl}px`,
            fontFamily: fontFamily.display,
            fontSize: font.sm,
            letterSpacing: 0.6,
            cursor: "pointer",
            width: "100%",
            maxWidth: 280,
          }}
        >
          {proceedLabel}
        </button>
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
          from <strong style={{ color: colors.text }}>{hostFromOrigin(session.origin)}</strong>
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
        const message = error instanceof Error ? error.message : "Failed to start camera.";
        warn("Failed starting camera scanner", { error: message });
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
  }, []);

  const statusColor = scanState === "error" ? colors.error : colors.textMuted;

  return (
    <>
      <div style={{ textAlign: "center", display: "flex", flexDirection: "column", alignItems: "center", gap: space.xs }}>
        <h1 style={titleStyle}>Scan Signed QR</h1>
        <p style={subtitleStyle}>Hold your Faraday device up to the camera.</p>
      </div>

      <div style={videoFrameStyle}>
        <video ref={videoRef} autoPlay muted playsInline style={videoStyle} />
        <span style={cornerStyle("tl")} aria-hidden />
        <span style={cornerStyle("tr")} aria-hidden />
        <span style={cornerStyle("bl")} aria-hidden />
        <span style={cornerStyle("br")} aria-hidden />
      </div>

      <p style={{ margin: 0, fontSize: font.sm, color: statusColor, textAlign: "center" }}>{statusText}</p>

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
      // Risk report comes pre-computed from background. For sign-message or
      // sessions without a report, go straight to the QR display screen.
      setStep(response.data.kind === "tx" && response.data.riskReport ? "risk" : "display");
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
