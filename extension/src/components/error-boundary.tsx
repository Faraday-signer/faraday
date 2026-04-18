import { Component, type ErrorInfo, type ReactNode } from "react";

import { colors, fontFamily, font, letterSpacing, radius, space } from "../lib/theme";
import { FaradayMark } from "../lib/brand";

interface ErrorBoundaryProps {
  children: ReactNode;
  /** Custom fallback renderer. Defaults to a minimal branded error page. */
  fallback?: (state: { error: Error; reset: () => void }) => ReactNode;
}

interface ErrorBoundaryState {
  error: Error | null;
}

/**
 * Catches render errors in the component tree and shows a branded fallback
 * instead of a blank side panel. Errors thrown during event handlers and
 * async work are NOT caught here — surface those with `ErrorBanner` where
 * they originate.
 */
export class ErrorBoundary extends Component<ErrorBoundaryProps, ErrorBoundaryState> {
  state: ErrorBoundaryState = { error: null };

  static getDerivedStateFromError(error: Error): ErrorBoundaryState {
    return { error };
  }

  componentDidCatch(error: Error, info: ErrorInfo): void {
    // Send to the extension console so devs debugging via `chrome://extensions`
    // → service worker can see what blew up. No external reporting.
    console.error("[Faraday][error-boundary]", error, info);
  }

  reset = (): void => {
    this.setState({ error: null });
  };

  render(): ReactNode {
    const { error } = this.state;
    if (!error) {
      return this.props.children;
    }

    if (this.props.fallback) {
      return this.props.fallback({ error, reset: this.reset });
    }

    return (
      <main
        style={{
          minHeight: "100vh",
          background: colors.bg,
          color: colors.text,
          fontFamily: fontFamily.ui,
          padding: space.xl,
          display: "flex",
          flexDirection: "column",
          alignItems: "center",
          gap: space.md,
          textAlign: "center"
        }}
      >
        <FaradayMark height={48} color={colors.error} />
        <h1
          style={{
            fontFamily: fontFamily.display,
            fontSize: font.xxl,
            letterSpacing: letterSpacing.loose,
            textTransform: "uppercase",
            margin: 0,
            color: colors.error
          }}
        >
          Something broke
        </h1>
        <p
          style={{
            fontFamily: fontFamily.ui,
            fontSize: font.sm,
            color: colors.textMuted,
            margin: 0,
            maxWidth: 300,
            lineHeight: 1.5
          }}
        >
          The Faraday side panel hit an unexpected error. Your device and keys are unaffected.
        </p>
        <details
          style={{
            fontFamily: fontFamily.mono,
            fontSize: font.xs,
            color: colors.textMuted,
            background: colors.panel,
            border: `1px solid ${colors.border}`,
            borderRadius: radius.md,
            padding: space.sm,
            maxWidth: 340,
            textAlign: "left",
            width: "100%",
            wordBreak: "break-word"
          }}
        >
          <summary style={{ cursor: "pointer" }}>Error details</summary>
          <pre style={{ margin: `${space.xs}px 0 0`, whiteSpace: "pre-wrap" }}>{error.message}</pre>
        </details>
        <button
          type="button"
          style={{
            background: colors.accent,
            color: colors.bg,
            border: "none",
            borderRadius: radius.md,
            padding: `${space.sm}px ${space.lg}px`,
            fontFamily: fontFamily.display,
            fontSize: font.md,
            letterSpacing: letterSpacing.loose,
            textTransform: "uppercase",
            cursor: "pointer"
          }}
          onClick={this.reset}
        >
          Reload panel
        </button>
      </main>
    );
  }
}
