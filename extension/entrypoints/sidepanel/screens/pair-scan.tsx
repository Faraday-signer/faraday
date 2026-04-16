import { useEffect, useRef, useState, type CSSProperties } from "react";
import { BrowserQRCodeReader } from "@zxing/browser";

import { LinkButton, PanelShell, PrimaryButton } from "../../../src/components/panel-shell";
import { useNavigation } from "../../../src/lib/router";
import { sendRuntimeMessage } from "../../../src/lib/runtime";
import { isValidSolanaAddress } from "../../../src/lib/solana";
import type { ExtensionState } from "../../../src/lib/types";
import { colors, flowColors, fontFamily, font, radius, space } from "../../../src/lib/theme";

type BarcodeDetectorLike = {
  detect: (source: ImageBitmapSource) => Promise<Array<{ rawValue?: string }>>;
};
type BarcodeDetectorCtorLike = new (options?: { formats?: string[] }) => BarcodeDetectorLike;

function getNativeDetectorCtor(): BarcodeDetectorCtorLike | null {
  const candidate = (globalThis as { BarcodeDetector?: unknown }).BarcodeDetector;
  return typeof candidate === "function" ? (candidate as BarcodeDetectorCtorLike) : null;
}

const FAST_VIDEO: MediaStreamConstraints = {
  audio: false,
  video: {
    facingMode: { ideal: "environment" },
    width: { ideal: 640, max: 1280 },
    height: { ideal: 480, max: 720 },
    frameRate: { ideal: 30 }
  }
};

/** Accept either a raw base58 address or a `solana:<address>[?…]` URI. */
function extractPubkey(raw: string): string | null {
  const trimmed = raw.trim();
  if (!trimmed) return null;
  const lower = trimmed.toLowerCase();
  const candidate = lower.startsWith("solana:")
    ? trimmed.slice(7).split(/[?&#]/)[0]
    : trimmed;
  return isValidSolanaAddress(candidate) ? candidate : null;
}

const frameStyle: CSSProperties = {
  position: "relative",
  width: "100%",
  aspectRatio: "1 / 1",
  borderRadius: radius.xl,
  overflow: "hidden",
  background: "#000",
  border: `3px solid ${flowColors.pair.primary}`,
  boxShadow: `0 0 0 1px ${flowColors.pair.primary}, 0 24px 48px rgba(0, 0, 0, 0.45)`
};

const videoStyle: CSSProperties = {
  width: "100%",
  height: "100%",
  objectFit: "cover"
};

const cornerBase: CSSProperties = {
  position: "absolute",
  width: 28,
  height: 28,
  borderColor: flowColors.pair.primary,
  borderStyle: "solid",
  borderWidth: 0
};
const corner = (pos: "tl" | "tr" | "bl" | "br"): CSSProperties => {
  const offset = 12;
  const base = { ...cornerBase };
  if (pos === "tl") return { ...base, top: offset, left: offset, borderTopWidth: 3, borderLeftWidth: 3, borderTopLeftRadius: 4 };
  if (pos === "tr") return { ...base, top: offset, right: offset, borderTopWidth: 3, borderRightWidth: 3, borderTopRightRadius: 4 };
  if (pos === "bl") return { ...base, bottom: offset, left: offset, borderBottomWidth: 3, borderLeftWidth: 3, borderBottomLeftRadius: 4 };
  return { ...base, bottom: offset, right: offset, borderBottomWidth: 3, borderRightWidth: 3, borderBottomRightRadius: 4 };
};

const wrapStyle: CSSProperties = {
  display: "flex",
  flexDirection: "column",
  alignItems: "center",
  gap: space.md,
  padding: `${space.md}px ${space.md}px ${space.lg}px`
};

const statusStyle = (isError: boolean): CSSProperties => ({
  fontSize: font.sm,
  color: isError ? colors.error : colors.textMuted,
  textAlign: "center",
  minHeight: 18
});

const previewCardStyle: CSSProperties = {
  width: "100%",
  padding: space.md,
  background: colors.panel,
  border: `1px solid ${colors.border}`,
  borderRadius: radius.md,
  display: "flex",
  flexDirection: "column",
  gap: space.xs,
  textAlign: "center"
};

const previewAddressStyle: CSSProperties = {
  fontFamily: fontFamily.mono,
  fontSize: font.sm,
  color: colors.accent,
  wordBreak: "break-all"
};

export function PairScanScreen() {
  const nav = useNavigation();
  const [status, setStatus] = useState("Requesting camera access…");
  const [isError, setIsError] = useState(false);
  const [detectedPubkey, setDetectedPubkey] = useState<string | null>(null);
  const [saving, setSaving] = useState(false);

  const videoRef = useRef<HTMLVideoElement | null>(null);
  const streamRef = useRef<MediaStream | null>(null);
  const controlsRef = useRef<{ stop: () => void } | null>(null);
  const rafRef = useRef<number | null>(null);
  const lockedRef = useRef(false);

  function stopCamera() {
    try {
      controlsRef.current?.stop();
    } catch {}
    controlsRef.current = null;
    if (rafRef.current !== null) {
      window.cancelAnimationFrame(rafRef.current);
      rafRef.current = null;
    }
    if (streamRef.current) {
      for (const t of streamRef.current.getTracks()) {
        try {
          t.stop();
        } catch {}
      }
      streamRef.current = null;
    }
    if (videoRef.current) {
      try {
        videoRef.current.pause();
      } catch {}
      videoRef.current.srcObject = null;
    }
  }

  function handleDecoded(raw: string) {
    if (lockedRef.current) return;
    const pubkey = extractPubkey(raw);
    if (!pubkey) {
      setStatus("QR recognised but not a Solana address. Keep trying.");
      setIsError(true);
      return;
    }
    lockedRef.current = true;
    stopCamera();
    setDetectedPubkey(pubkey);
    setIsError(false);
    setStatus("Address captured.");
  }

  useEffect(() => {
    let cancelled = false;

    async function startNative() {
      const Ctor = getNativeDetectorCtor();
      if (!Ctor || !videoRef.current) throw new Error("native-unavailable");
      const stream = await navigator.mediaDevices.getUserMedia(FAST_VIDEO);
      if (cancelled) {
        for (const t of stream.getTracks()) t.stop();
        return;
      }
      streamRef.current = stream;
      videoRef.current.srcObject = stream;
      await videoRef.current.play();
      const detector = new Ctor({ formats: ["qr_code"] });
      setStatus("Point the camera at your device's address QR");
      const loop = async () => {
        if (!videoRef.current || lockedRef.current || cancelled) return;
        try {
          const results = await detector.detect(videoRef.current);
          if (results[0]?.rawValue) {
            handleDecoded(results[0].rawValue);
            return;
          }
        } catch {}
        rafRef.current = window.requestAnimationFrame(() => void loop());
      };
      rafRef.current = window.requestAnimationFrame(() => void loop());
    }

    async function startZxing() {
      if (!videoRef.current) return;
      const reader = new BrowserQRCodeReader();
      const controls = await reader.decodeFromConstraints(FAST_VIDEO, videoRef.current, (result) => {
        if (result && !cancelled) handleDecoded(result.getText());
      });
      if (cancelled) {
        controls.stop();
        return;
      }
      controlsRef.current = controls;
      setStatus("Point the camera at your device's address QR");
    }

    void (async () => {
      try {
        if (getNativeDetectorCtor()) await startNative();
        else await startZxing();
      } catch (error) {
        if (cancelled) return;
        const msg = error instanceof Error ? error.message : "Failed to start camera.";
        setStatus(msg);
        setIsError(true);
      }
    })();

    return () => {
      cancelled = true;
      stopCamera();
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  async function confirmPair() {
    if (!detectedPubkey) return;
    setSaving(true);
    const r = await sendRuntimeMessage<ExtensionState>({
      type: "faraday:set-paired-pubkey",
      pubkey: detectedPubkey
    });
    setSaving(false);
    if (r.ok) {
      nav.reset({ name: "home" });
    } else {
      setStatus(r.error);
      setIsError(true);
    }
  }

  return (
    <PanelShell eyebrow="Pair Device" title="Scan your Faraday">
      <div style={wrapStyle}>
        <div style={frameStyle}>
          <video ref={videoRef} autoPlay muted playsInline style={videoStyle} />
          <span style={corner("tl")} aria-hidden />
          <span style={corner("tr")} aria-hidden />
          <span style={corner("bl")} aria-hidden />
          <span style={corner("br")} aria-hidden />
        </div>

        <p style={statusStyle(isError)}>{status}</p>

        {detectedPubkey ? (
          <div style={previewCardStyle}>
            <span style={{ fontFamily: fontFamily.display, fontSize: font.xs, letterSpacing: 1.6, textTransform: "uppercase", color: colors.textMuted }}>
              Pair with this address?
            </span>
            <span style={previewAddressStyle}>{detectedPubkey}</span>
            <div style={{ display: "flex", justifyContent: "center", marginTop: space.xs }}>
              <PrimaryButton onClick={confirmPair} disabled={saving}>
                Confirm pair
              </PrimaryButton>
            </div>
          </div>
        ) : null}

        <LinkButton onClick={() => nav.replace({ name: "pair-paste" })}>Paste address instead</LinkButton>
      </div>
    </PanelShell>
  );
}
