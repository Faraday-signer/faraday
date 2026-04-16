import type { CSSProperties } from "react";
import { useEffect, useMemo, useRef, useState } from "react";

import { BrowserQRCodeReader } from "@zxing/browser";
import { QRCodeSVG } from "qrcode.react";

import { sendRuntimeMessage } from "../../src/lib/runtime";
import type { GetSignSessionResult } from "../../src/lib/types";

type ScanState = "idle" | "starting" | "scanning" | "success" | "error";

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
  tryPlayVideoTimeout: 2500,
} as const;

const FAST_VIDEO_CONSTRAINTS: MediaStreamConstraints = {
  audio: false,
  video: {
    facingMode: { ideal: "environment" },
    width: { ideal: 640, max: 1280 },
    height: { ideal: 480, max: 720 },
    frameRate: { ideal: 30, max: 60 },
  },
};

function getBarcodeDetectorCtor(): BarcodeDetectorCtorLike | null {
  const candidate = (globalThis as { BarcodeDetector?: unknown }).BarcodeDetector;
  if (typeof candidate === "function") {
    return candidate as BarcodeDetectorCtorLike;
  }
  return null;
}

const panelStyle: CSSProperties = {
  border: "1px solid #334155",
  borderRadius: 10,
  padding: 12,
  background: "#0f172a"
};

function shortAddress(address: string): string {
  if (address.length <= 14) {
    return address;
  }
  return `${address.slice(0, 6)}...${address.slice(-6)}`;
}

function getSessionId(): string | null {
  const params = new URLSearchParams(window.location.search);
  const value = params.get("session");
  return value ? value.trim() : null;
}

export function SignApp() {
  const sessionId = useMemo(() => getSessionId(), []);

  const [session, setSession] = useState<GetSignSessionResult | null>(null);
  const [scanState, setScanState] = useState<ScanState>("idle");
  const [statusText, setStatusText] = useState("Waiting to start camera scan.");
  const [fatalError, setFatalError] = useState<string | null>(null);

  const videoRef = useRef<HTMLVideoElement | null>(null);
  const readerRef = useRef<BrowserQRCodeReader | null>(null);
  const controlsRef = useRef<{ stop: () => void } | null>(null);
  const streamRef = useRef<MediaStream | null>(null);
  const animationRef = useRef<number | null>(null);
  const decodeLockedRef = useRef(false);

  function stopActiveScanner() {
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

  useEffect(() => {
    if (!sessionId) {
      setFatalError("Missing sign session id.");
      return;
    }

    let canceled = false;
    void (async () => {
      const response = await sendRuntimeMessage<GetSignSessionResult>({
        type: "faraday:get-sign-session",
        sessionId
      });

      if (canceled) {
        return;
      }

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
        expectedPubkey: response.data.expectedPubkey
      });
      setSession(response.data);
    })();

    return () => {
      canceled = true;
    };
  }, [sessionId]);

  useEffect(() => {
    return () => {
      stopActiveScanner();
      readerRef.current = null;
    };
  }, []);

  async function completeSession(signedTxBase64: string): Promise<boolean> {
    if (!sessionId) {
      return false;
    }

    const response = await sendRuntimeMessage({
      type: "faraday:complete-sign-session",
      sessionId,
      signedTxBase64
    });

    if (!response.ok) {
      warn("Failed to complete sign session", { sessionId, error: response.error });
      setScanState("error");
      setStatusText(response.error);
      return false;
    }

    debug("Completed sign session", { sessionId });
    setScanState("success");
    setStatusText("Signed transaction captured. Returning to dapp...");

    window.setTimeout(() => {
      window.close();
    }, 300);

    return true;
  }

  async function cancelSession(reason = "Signing canceled by user.") {
    if (!sessionId) {
      return;
    }

    stopActiveScanner();

    await sendRuntimeMessage({
      type: "faraday:cancel-sign-session",
      sessionId,
      reason
    });
    debug("Canceled sign session", { sessionId, reason });
    window.close();
  }

  async function handleDecodedPayload(rawValue: string, scanner: "native" | "zxing") {
    if (decodeLockedRef.current) {
      return;
    }

    decodeLockedRef.current = true;
    stopActiveScanner();
    setStatusText("QR decoded. Verifying signed payload...");
    debug("Signed QR decoded", { sessionId, scanner, size: rawValue.length });

    const ok = await completeSession(rawValue);
    if (!ok) {
      decodeLockedRef.current = false;
    }
  }

  async function startNativeBarcodeDetector() {
    if (!videoRef.current) {
      throw new Error("Video element not ready.");
    }

    const BarcodeDetectorImpl = getBarcodeDetectorCtor();
    if (!BarcodeDetectorImpl) {
      throw new Error("BarcodeDetector is unavailable.");
    }

    const stream = await navigator.mediaDevices.getUserMedia(FAST_VIDEO_CONSTRAINTS);
    streamRef.current = stream;
    videoRef.current.srcObject = stream;
    await videoRef.current.play();

    const detector = new BarcodeDetectorImpl({ formats: ["qr_code"] });
    setScanState("scanning");
    setStatusText("Scanning for signed QR... (native detector)");
    debug("Native BarcodeDetector started", { sessionId });

    const detectLoop = async () => {
      if (!videoRef.current || decodeLockedRef.current) {
        return;
      }

      try {
        const results = await detector.detect(videoRef.current);
        const first = results[0];
        if (first?.rawValue) {
          await handleDecodedPayload(first.rawValue, "native");
          return;
        }
      } catch {
        // keep looping
      }

      animationRef.current = window.requestAnimationFrame(() => {
        void detectLoop();
      });
    };

    animationRef.current = window.requestAnimationFrame(() => {
      void detectLoop();
    });
  }

  async function startZxingScanner() {
    if (!videoRef.current) {
      throw new Error("Video element not ready.");
    }

    const reader = new BrowserQRCodeReader(undefined, ZXING_OPTIONS);
    readerRef.current = reader;

    const controls = await reader.decodeFromConstraints(
      FAST_VIDEO_CONSTRAINTS,
      videoRef.current,
      (result) => {
        if (result) {
          void handleDecodedPayload(result.getText(), "zxing");
        }
      }
    );

    controlsRef.current = controls;
    setScanState("scanning");
    setStatusText("Scanning for signed QR...");
    debug("ZXing scanner started", { sessionId, options: ZXING_OPTIONS });
  }

  async function startScanning() {
    if (!videoRef.current) {
      return;
    }

    setScanState("starting");
    setStatusText("Requesting camera access...");
    decodeLockedRef.current = false;

    stopActiveScanner();

    try {
      if (getBarcodeDetectorCtor()) {
        await startNativeBarcodeDetector();
      } else {
        await startZxingScanner();
      }
    } catch (error) {
      const msg = error instanceof Error ? error.message : "Failed to start camera.";
      warn("Failed starting camera scanner", { sessionId, error: msg });
      setScanState("error");
      setStatusText(msg);
    }
  }

  if (fatalError) {
    return (
      <main style={{ padding: 16, fontFamily: "ui-sans-serif, system-ui", color: "#e2e8f0", background: "#020617", minHeight: "100vh" }}>
        <h1 style={{ fontSize: 20, margin: "0 0 12px" }}>Faraday</h1>
        <p style={{ color: "#fda4af" }}>{fatalError}</p>
      </main>
    );
  }

  if (!session) {
    return (
      <main style={{ padding: 16, fontFamily: "ui-sans-serif, system-ui", color: "#e2e8f0", background: "#020617", minHeight: "100vh" }}>
        Loading signing session...
      </main>
    );
  }

  return (
    <main
      style={{
        minHeight: "100vh",
        margin: 0,
        padding: 16,
        background: "#020617",
        color: "#e2e8f0",
        fontFamily: "ui-sans-serif, system-ui, -apple-system, Segoe UI, sans-serif"
      }}
    >
      <header style={{ marginBottom: 12 }}>
        <h1 style={{ margin: 0, fontSize: 20 }}>Sign with Faraday</h1>
        <p style={{ margin: "6px 0 0", fontSize: 13, color: "#94a3b8" }}>
          Pair: {shortAddress(session.expectedPubkey)}
        </p>
      </header>

      <section style={{ ...panelStyle, marginBottom: 12 }}>
        <h2 style={{ margin: "0 0 8px", fontSize: 14 }}>1) Scan this unsigned transaction on Pi</h2>
        <div style={{ display: "grid", placeItems: "center", background: "#fff", borderRadius: 8, padding: 10 }}>
          <QRCodeSVG
            value={session.txBase64}
            size={260}
            level="M"
            includeMargin
            bgColor="#ffffff"
            fgColor="#000000"
          />
        </div>
      </section>

      <section style={{ ...panelStyle, marginBottom: 12 }}>
        <h2 style={{ margin: "0 0 8px", fontSize: 14 }}>2) Scan signed QR from Pi</h2>
        <video
          ref={videoRef}
          autoPlay
          muted
          playsInline
          style={{ width: "100%", minHeight: 220, background: "#000", borderRadius: 8 }}
        />

        <div style={{ marginTop: 10, display: "flex", gap: 8, flexWrap: "wrap" }}>
          <button
            onClick={startScanning}
            disabled={scanState === "starting" || scanState === "scanning" || scanState === "success"}
            style={{ padding: "7px 12px", cursor: "pointer" }}
          >
            {scanState === "scanning" ? "Scanning..." : "Start Camera Scan"}
          </button>
          <button onClick={() => cancelSession()} style={{ padding: "7px 12px", cursor: "pointer" }}>
            Cancel
          </button>
        </div>

        <p style={{ margin: "10px 0 0", fontSize: 12, color: scanState === "error" ? "#fda4af" : "#94a3b8" }}>
          {statusText}
        </p>
      </section>
    </main>
  );
}
