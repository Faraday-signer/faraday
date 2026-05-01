//! Shared UI for the camera-permission states across the sidepanel
//! pair-scan and the popup signed-QR scan.
//!
//! Two surfaces:
//!   - `<CameraRequestPrompt>` — pre-camera-start state. Renders the
//!     scan area's instruction copy + a clear "Allow camera" button so
//!     Chrome's permission prompt fires on a deliberate click instead
//!     of mid-render.
//!   - `<CameraBlockedPanel>` — recovery state for when permission has
//!     been previously denied. Tells the user what to do, opens the
//!     extension's settings page, and offers a Retry button for after
//!     they re-enable in Chrome.

import type { CSSProperties, ReactNode } from "react";

import { openExtensionSettings } from "../lib/camera-permission";
import { colors, fontFamily, font, letterSpacing, radius, space } from "../lib/theme";

const cardStyle: CSSProperties = {
  padding: space.lg,
  borderRadius: radius.lg,
  background: colors.panel,
  border: `1px solid ${colors.borderStrong}`,
  display: "flex",
  flexDirection: "column",
  alignItems: "center",
  gap: space.sm,
  textAlign: "center",
};

const iconCircleStyle = (color: string): CSSProperties => ({
  width: 56,
  height: 56,
  borderRadius: radius.pill,
  display: "inline-flex",
  alignItems: "center",
  justifyContent: "center",
  background: `${color}1f`,
  border: `1px solid ${color}`,
  color,
  fontFamily: fontFamily.display,
  fontSize: 24,
});

const titleStyle: CSSProperties = {
  fontFamily: fontFamily.display,
  fontSize: font.lg,
  letterSpacing: letterSpacing.loose,
  color: colors.text,
  margin: 0,
};

const bodyStyle: CSSProperties = {
  fontFamily: fontFamily.ui,
  fontSize: font.sm,
  color: colors.textMuted,
  lineHeight: 1.5,
  maxWidth: 320,
  margin: 0,
};

const stepListStyle: CSSProperties = {
  fontFamily: fontFamily.mono,
  fontSize: font.xs,
  color: colors.textMuted,
  lineHeight: 1.6,
  textAlign: "left",
  margin: 0,
  paddingLeft: space.lg,
};

const primaryButtonStyle: CSSProperties = {
  background: colors.accent,
  color: colors.bg,
  border: "none",
  borderRadius: radius.md,
  padding: `${space.sm}px ${space.lg}px`,
  fontFamily: fontFamily.display,
  fontSize: font.md,
  letterSpacing: letterSpacing.loose,
  textTransform: "uppercase",
  cursor: "pointer",
  width: "100%",
  maxWidth: 280,
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
  textUnderlineOffset: 3,
};

interface CameraRequestPromptProps {
  /** Per-surface action copy, e.g. "scan your Faraday device." */
  intent: ReactNode;
  /** Fired when the user clicks "Allow camera." */
  onAllow: () => void;
  /** Optional text override for the primary button. Defaults to "Allow camera". */
  buttonLabel?: string;
}

export function CameraRequestPrompt({
  intent,
  onAllow,
  buttonLabel = "Allow camera",
}: CameraRequestPromptProps) {
  return (
    <div style={cardStyle}>
      <span aria-hidden style={iconCircleStyle(colors.accent)}>
        ⌖
      </span>
      <h2 style={titleStyle}>Camera access</h2>
      <p style={bodyStyle}>
        Faraday needs your camera to {intent}
      </p>
      <p style={{ ...bodyStyle, color: colors.textDim, fontSize: font.xs }}>
        We never record or send video — frames stay in this tab and are read
        only for QR codes.
      </p>
      <button type="button" onClick={onAllow} style={primaryButtonStyle}>
        {buttonLabel}
      </button>
    </div>
  );
}

interface CameraBlockedPanelProps {
  /** Original error message — shown in small print for debug. */
  detail?: string;
  /** Fired when the user clicks Retry (after re-enabling in Chrome). */
  onRetry: () => void;
}

export function CameraBlockedPanel({ detail, onRetry }: CameraBlockedPanelProps) {
  async function openSettings() {
    const opened = await openExtensionSettings();
    if (!opened) {
      // Couldn't open chrome:// from this surface (some popup contexts
      // block it). Surface the URL so the user can paste it manually.
      try {
        await navigator.clipboard.writeText(
          `chrome://extensions/?id=${chrome.runtime.id}`,
        );
      } catch {
        // best-effort
      }
    }
  }

  return (
    <div style={{ ...cardStyle, borderColor: colors.warning }}>
      <span aria-hidden style={iconCircleStyle(colors.warning)}>
        ✕
      </span>
      <h2 style={{ ...titleStyle, color: colors.warning }}>Camera blocked</h2>
      <p style={bodyStyle}>
        Chrome blocked camera access for Faraday. Re-enable it to scan QR codes.
      </p>
      <ol style={stepListStyle}>
        <li>Click the button below — it opens Faraday's settings.</li>
        <li>Find "Site access" → "Camera".</li>
        <li>Set it to "Allow", then come back and retry.</li>
      </ol>
      <button type="button" onClick={openSettings} style={primaryButtonStyle}>
        Open Chrome settings
      </button>
      <button type="button" onClick={onRetry} style={secondaryLinkStyle}>
        I've allowed it — retry
      </button>
      {detail ? (
        <p style={{ ...bodyStyle, color: colors.textDim, fontSize: font.xs, fontFamily: fontFamily.mono }}>
          {detail}
        </p>
      ) : null}
    </div>
  );
}
