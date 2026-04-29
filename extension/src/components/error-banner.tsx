import { useState, type CSSProperties, type ReactNode } from "react";

import { colors, fontFamily, font, letterSpacing, radius, space } from "../lib/theme";

export type ErrorTone = "error" | "warning";

interface ErrorBannerProps {
  /** Short identifier, e.g. "Balance unavailable". */
  title?: ReactNode;
  /** Longer human-readable description, typically from a thrown Error. */
  message: ReactNode;
  /**
   * Verbose technical details — full RPC error blob, stack trace, etc.
   * Hidden by default behind a "Show details" toggle so the banner stays
   * readable. The banner truncates this with monospace + scroll so even
   * very long blobs don't blow out the layout.
   */
  details?: ReactNode;
  /** Fires a retry action. Renders a small RETRY button when provided. */
  onRetry?: () => void;
  /** Disabled state for the retry button (e.g. a fetch is already in flight). */
  retrying?: boolean;
  /** Dismisses the banner. Renders an × when provided. */
  onDismiss?: () => void;
  /** Visual tone. Defaults to "error". */
  tone?: ErrorTone;
  /** Escape-hatch style overrides (alignment tweaks, margins). */
  style?: CSSProperties;
}

const toneMap: Record<ErrorTone, { fg: string; bg: string; border: string }> = {
  error: {
    fg: colors.error,
    bg: "rgba(255, 107, 107, 0.12)",
    border: colors.error
  },
  warning: {
    fg: colors.warning,
    bg: "rgba(255, 180, 84, 0.12)",
    border: colors.warning
  }
};

const labelStyle: CSSProperties = {
  fontFamily: fontFamily.display,
  fontSize: 10,
  letterSpacing: letterSpacing.eyebrow,
  textTransform: "uppercase"
};

const messageStyle: CSSProperties = {
  fontFamily: fontFamily.ui,
  fontSize: font.xs,
  lineHeight: 1.5,
  wordBreak: "break-word"
};

const actionsRowStyle: CSSProperties = {
  display: "flex",
  alignItems: "center",
  justifyContent: "flex-end",
  gap: space.xs,
  marginTop: space.xs
};

const buttonBase: CSSProperties = {
  background: "transparent",
  border: "1px solid",
  borderRadius: radius.sm,
  padding: `${space.xxs}px ${space.xs}px`,
  cursor: "pointer",
  fontFamily: fontFamily.display,
  fontSize: font.xs,
  letterSpacing: letterSpacing.loose,
  textTransform: "uppercase"
};

const dismissButtonStyle: CSSProperties = {
  ...buttonBase,
  border: "none",
  color: colors.textMuted,
  padding: `${space.xxs}px ${space.xs}px`
};

/**
 * Reusable error display. Use inline near the action that failed — don't
 * hoist errors into a toast layer unless they're truly global.
 *
 * Examples of good placement:
 *   - Above the Home balance when the RPC balance fetch fails
 *   - Next to the "Revoke" button on a single origin row when revoke fails
 *   - Under the amount/recipient inputs when a send validation fails
 */
export function ErrorBanner({
  title,
  message,
  details,
  onRetry,
  retrying,
  onDismiss,
  tone = "error",
  style
}: ErrorBannerProps) {
  const palette = toneMap[tone];
  const [showDetails, setShowDetails] = useState(false);

  const detailsToggleStyle: CSSProperties = {
    background: "transparent",
    border: "none",
    padding: 0,
    color: palette.fg,
    fontFamily: fontFamily.display,
    fontSize: 10,
    letterSpacing: letterSpacing.eyebrow,
    textTransform: "uppercase",
    cursor: "pointer",
    textDecoration: "underline dotted",
    textUnderlineOffset: 3,
    alignSelf: "flex-start"
  };

  const detailsBoxStyle: CSSProperties = {
    fontFamily: fontFamily.mono,
    fontSize: font.xs,
    color: palette.fg,
    background: "rgba(0, 0, 0, 0.25)",
    border: `1px solid ${palette.border}`,
    borderRadius: radius.sm,
    padding: space.xs,
    maxHeight: 160,
    overflow: "auto",
    wordBreak: "break-all",
    whiteSpace: "pre-wrap"
  };

  return (
    <div
      role="alert"
      aria-live="polite"
      style={{
        width: "100%",
        background: palette.bg,
        border: `1px solid ${palette.border}`,
        borderRadius: radius.md,
        padding: space.sm,
        color: palette.fg,
        display: "flex",
        flexDirection: "column",
        gap: 6,
        ...style
      }}
    >
      <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", gap: space.xs }}>
        {title ? <span style={labelStyle}>{title}</span> : <span />}
        {onDismiss ? (
          <button
            type="button"
            aria-label="Dismiss"
            style={{ ...dismissButtonStyle, color: palette.fg }}
            onClick={onDismiss}
          >
            ×
          </button>
        ) : null}
      </div>

      <div style={messageStyle}>{message}</div>

      {details ? (
        <>
          <button
            type="button"
            style={detailsToggleStyle}
            onClick={() => setShowDetails((prev) => !prev)}
            aria-expanded={showDetails}
          >
            {showDetails ? "Hide details ▴" : "Show details ▾"}
          </button>
          {showDetails ? <div style={detailsBoxStyle}>{details}</div> : null}
        </>
      ) : null}

      {onRetry ? (
        <div style={actionsRowStyle}>
          <button
            type="button"
            style={{ ...buttonBase, borderColor: palette.fg, color: palette.fg }}
            onClick={onRetry}
            disabled={retrying}
          >
            {retrying ? "Retrying…" : "Retry"}
          </button>
        </div>
      ) : null}
    </div>
  );
}
