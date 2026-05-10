import { useEffect, useRef, useState, type CSSProperties } from "react";
import { BrowserQRCodeReader } from "@zxing/browser";

import { CameraBlockedPanel, CameraRequestPrompt } from "@/components/camera-permission-prompts";
import { ErrorBanner } from "@/components/error-banner";
import { LinkButton, PanelShell, PrimaryButton } from "@/components/panel-shell";
import { categorizeCameraError, getCameraPermissionState } from "@/lib/camera-permission";
import { useNavigation } from "@/lib/router";
import { sendRuntimeMessage } from "@/lib/runtime";
import { isValidSolanaAddress } from "@/lib/solana";
import type { ExtensionState } from "@/lib/types";
import { colors, flowColors, fontFamily, font, radius, space } from "@/lib/theme";

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

type ScanResult =
  | { kind: "pair"; pubkey: string; source: "faraday-pair" | "solana-uri" | "bare" }
  | { kind: "wrong-mode"; hint: string }
  | { kind: "invalid" };

/**
 * Parse a QR payload as a pairing input. Supports three formats in order of
 * preference:
 *
 *   1. `faraday:pair:<address>` — preferred tagged envelope. Device firmware
 *      ships this when the user opens the Pair screen.
 *   2. `solana:<address>[?params]` — ecosystem-standard receive URI. Accepted
 *      as a fallback while the device still emits this format.
 *   3. Bare base58 address — last-resort accepted for pasting; discouraged
 *      via QR because any 32-byte value can match it.
 *
 * When a recognised-but-wrong payload is detected (e.g. a signed-tx QR with
 * the `faraday:sig:` prefix), returns `wrong-mode` with a hint pointing the
 * user at the right device screen.
 */
function parsePairScan(raw: string): ScanResult {
  const trimmed = raw.trim();
  if (!trimmed) return { kind: "invalid" };
  const lower = trimmed.toLowerCase();

  if (lower.startsWith("faraday:pair:")) {
    const candidate = trimmed.slice("faraday:pair:".length).split(/[?&#]/)[0];
    if (isValidSolanaAddress(candidate)) {
      return { kind: "pair", pubkey: candidate, source: "faraday-pair" };
    }
    return { kind: "invalid" };
  }

  if (lower.startsWith("faraday:sig:") || lower.startsWith("faraday:signed:")) {
    return {
      kind: "wrong-mode",
      hint: "That's a signed-transaction QR. On your Faraday, go to Home → Show Address."
    };
  }

  if (lower.startsWith("faraday:unsigned:") || lower.startsWith("faraday:tx:")) {
    return {
      kind: "wrong-mode",
      hint: "That's a transaction QR, not a wallet QR. On your Faraday, go to Home → Show Address."
    };
  }

  if (lower.startsWith("solana:")) {
    const candidate = trimmed.slice("solana:".length).split(/[?&#]/)[0];
    if (isValidSolanaAddress(candidate)) {
      return { kind: "pair", pubkey: candidate, source: "solana-uri" };
    }
  }

  if (isValidSolanaAddress(trimmed)) {
    return { kind: "pair", pubkey: trimmed, source: "bare" };
  }

  return { kind: "invalid" };
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
  alignItems: "stretch",
  gap: space.md,
  padding: `${space.md}px ${space.md}px ${space.lg}px`
};

const instructionCardStyle: CSSProperties = {
  padding: space.sm,
  borderRadius: radius.md,
  background: flowColors.pair.soft,
  border: `1px solid ${flowColors.pair.primary}`,
  display: "flex",
  flexDirection: "column",
  gap: 2
};

const instructionEyebrowStyle: CSSProperties = {
  fontFamily: fontFamily.display,
  fontSize: 10,
  letterSpacing: 1.6,
  textTransform: "uppercase",
  color: flowColors.pair.primary
};

const instructionBodyStyle: CSSProperties = {
  fontFamily: fontFamily.ui,
  fontSize: font.xs,
  color: colors.text,
  lineHeight: 1.5
};

const instructionStepStyle: CSSProperties = {
  fontFamily: fontFamily.mono,
  fontSize: font.xs,
  color: flowColors.pair.primary
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

/**
 * Camera-access phase machine:
 *   - "prompt"  → user hasn't initiated yet; show the explicit Allow card
 *                so Chrome's permission prompt fires on a clear gesture.
 *   - "running" → camera is active and we're scanning.
 *   - "denied"  → previously blocked; show recovery panel that links to
 *                Chrome's extension settings.
 *
 * The scan effect runs only in "running". On mount we skip straight to
 * "running" when the Permissions API reports `granted`, so re-visiting
 * the screen after a successful pair-and-back doesn't make the user
 * click "Allow" again.
 */
type CameraPhase = "prompt" | "running" | "denied";

export function PairScanScreen() {
  const nav = useNavigation();
  const [status, setStatus] = useState("Point the camera at your device's address QR");
  const [cameraError, setCameraError] = useState<string | null>(null);
  const [mutationError, setMutationError] = useState<string | null>(null);
  const [retryKey, setRetryKey] = useState(0);
  const [detectedPubkey, setDetectedPubkey] = useState<string | null>(null);
  const [saving, setSaving] = useState(false);
  const [cameraPhase, setCameraPhase] = useState<CameraPhase>("prompt");

  // Skip the explicit allow card when permission is already granted —
  // returning to this screen after a successful pair shouldn't make the
  // user re-click the button.
  useEffect(() => {
    let cancelled = false;
    void (async () => {
      const state = await getCameraPermissionState();
      if (cancelled) return;
      if (state === "granted") setCameraPhase("running");
      else if (state === "denied") setCameraPhase("denied");
      // "prompt" / "unknown" → keep the default "prompt" phase
    })();
    return () => { cancelled = true; };
  }, []);

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
    const result = parsePairScan(raw);
    if (result.kind === "pair") {
      lockedRef.current = true;
      stopCamera();
      setDetectedPubkey(result.pubkey);
      setStatus(
        result.source === "faraday-pair"
          ? "Faraday wallet QR recognised."
          : result.source === "solana-uri"
            ? "Solana address QR captured."
            : "Address captured."
      );
      return;
    }
    if (result.kind === "wrong-mode") {
      setStatus(result.hint);
      return;
    }
    setStatus("QR recognised but not a Solana address. Keep trying.");
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

    if (cameraPhase !== "running") return undefined;

    void (async () => {
      setCameraError(null);
      try {
        if (getNativeDetectorCtor()) await startNative();
        else await startZxing();
      } catch (error) {
        if (cancelled) return;
        const { kind, message } = categorizeCameraError(error);
        if (kind === "denied") {
          // Permission was just denied (user clicked "Block") or was
          // already denied at start. Switch to the recovery panel.
          setCameraPhase("denied");
          setCameraError(null);
        } else {
          setCameraError(message);
          setStatus("Camera unavailable.");
        }
      }
    })();

    return () => {
      cancelled = true;
      stopCamera();
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [retryKey, cameraPhase]);

  async function confirmPair() {
    if (!detectedPubkey) return;
    setSaving(true);
    setMutationError(null);
    const r = await sendRuntimeMessage<ExtensionState>({
      type: "faraday:set-paired-pubkey",
      pubkey: detectedPubkey
    });
    setSaving(false);
    if (r.ok) {
      nav.reset({ name: "home" });
    } else {
      setMutationError(r.error);
    }
  }

  function retryCamera() {
    lockedRef.current = false;
    setDetectedPubkey(null);
    setCameraError(null);
    setStatus("Point the camera at your device's address QR");
    setCameraPhase("running");
    setRetryKey((k) => k + 1);
  }

  function allowCamera() {
    setCameraPhase("running");
    setRetryKey((k) => k + 1);
  }

  const errorBanner =
    cameraError || mutationError ? (
      <>
        {cameraError ? (
          <ErrorBanner
            title="Camera unavailable"
            message={cameraError}
            onRetry={retryCamera}
          />
        ) : null}
        {mutationError ? (
          <ErrorBanner
            title="Import failed"
            message={mutationError}
            onRetry={confirmPair}
            retrying={saving}
            onDismiss={() => setMutationError(null)}
          />
        ) : null}
      </>
    ) : null;

  return (
    <PanelShell
      eyebrow="Import wallet"
      title={detectedPubkey ? "Confirm wallet" : "Scan to import"}
      banner={errorBanner}
    >
      <div style={wrapStyle}>
        {!detectedPubkey ? (
          <>
            <div style={instructionCardStyle}>
              <span style={instructionEyebrowStyle}>On your Faraday</span>
              <span style={instructionStepStyle}>Home → Show Address</span>
              <span style={instructionBodyStyle}>
                Point your webcam at the address QR on the device screen.
              </span>
            </div>

            {cameraPhase === "prompt" ? (
              <CameraRequestPrompt
                intent="scan your Faraday device's address QR."
                onAllow={allowCamera}
              />
            ) : cameraPhase === "denied" ? (
              <CameraBlockedPanel onRetry={retryCamera} />
            ) : (
              <>
                <div style={frameStyle}>
                  <video ref={videoRef} autoPlay muted playsInline style={videoStyle} />
                  <span style={corner("tl")} aria-hidden />
                  <span style={corner("tr")} aria-hidden />
                  <span style={corner("bl")} aria-hidden />
                  <span style={corner("br")} aria-hidden />
                </div>
                <p style={statusStyle(false)}>{status}</p>
              </>
            )}

            <LinkButton onClick={() => nav.replace({ name: "pair-paste" })}>Paste address instead</LinkButton>
          </>
        ) : (
          <div style={previewCardStyle}>
            <span style={{ fontFamily: fontFamily.display, fontSize: font.xs, letterSpacing: 1.6, textTransform: "uppercase", color: colors.textMuted }}>
              Use this address?
            </span>
            <span style={previewAddressStyle}>{detectedPubkey}</span>
            <div style={{ display: "flex", flexDirection: "column", alignItems: "center", gap: space.xs, marginTop: space.xs }}>
              <PrimaryButton onClick={confirmPair} disabled={saving}>
                {saving ? "Importing…" : "Import wallet"}
              </PrimaryButton>
              <LinkButton onClick={retryCamera}>Scan another</LinkButton>
            </div>
          </div>
        )}
      </div>
    </PanelShell>
  );
}
