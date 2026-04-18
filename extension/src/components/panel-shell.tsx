import type { CSSProperties, ReactNode } from "react";

import { colors, fontFamily, font, letterSpacing, space } from "../lib/theme";
import { useNavigation } from "../lib/router";

interface PanelShellProps {
  /** Short title rendered in the header. Optional. */
  title?: ReactNode;
  /** Eyebrow text above the title (e.g. "PAIR DEVICE"). */
  eyebrow?: ReactNode;
  /** Trailing node in the header (e.g. settings gear). */
  trailing?: ReactNode;
  /** Leading node override. Defaults to a back chevron when `canGoBack`. */
  leading?: ReactNode;
  /** Main content area. */
  children: ReactNode;
  /** When true, hide the default back chevron even if canGoBack is true. */
  hideBack?: boolean;
  /** Fill-down container instead of scrollable. Use for hero / centered screens. */
  center?: boolean;
}

const shellStyle: CSSProperties = {
  minHeight: "100vh",
  display: "flex",
  flexDirection: "column",
  background: colors.bg,
  color: colors.text,
  fontFamily: fontFamily.ui
};

const headerStyle: CSSProperties = {
  display: "flex",
  alignItems: "center",
  justifyContent: "space-between",
  gap: space.sm,
  padding: `${space.sm}px ${space.md}px`,
  borderBottom: `1px solid ${colors.border}`,
  background: colors.bg,
  minHeight: 52
};

const leadingSlotStyle: CSSProperties = {
  display: "inline-flex",
  alignItems: "center",
  minWidth: 32
};

const centerSlotStyle: CSSProperties = {
  flex: 1,
  display: "flex",
  flexDirection: "column",
  alignItems: "center",
  gap: 2,
  lineHeight: 1.1
};

const trailingSlotStyle: CSSProperties = {
  display: "inline-flex",
  alignItems: "center",
  justifyContent: "flex-end",
  minWidth: 32
};

const eyebrowStyle: CSSProperties = {
  fontFamily: fontFamily.display,
  fontSize: 10,
  letterSpacing: letterSpacing.eyebrow,
  textTransform: "uppercase",
  color: colors.accent
};

const titleStyle: CSSProperties = {
  fontFamily: fontFamily.display,
  fontSize: font.lg,
  letterSpacing: letterSpacing.loose,
  color: colors.text
};

const backButtonStyle: CSSProperties = {
  background: "transparent",
  border: "none",
  color: colors.textMuted,
  cursor: "pointer",
  padding: `${space.xxs}px ${space.xs}px`,
  borderRadius: 6,
  fontFamily: fontFamily.display,
  fontSize: font.md,
  letterSpacing: letterSpacing.loose,
  display: "inline-flex",
  alignItems: "center",
  gap: 4
};

export function PanelShell({
  title,
  eyebrow,
  trailing,
  leading,
  children,
  hideBack,
  center
}: PanelShellProps) {
  const nav = useNavigation();
  const showBack = !hideBack && nav.canGoBack && leading === undefined;

  return (
    <main style={shellStyle}>
      <header style={headerStyle}>
        <div style={leadingSlotStyle}>
          {leading !== undefined ? (
            leading
          ) : showBack ? (
            <button type="button" onClick={nav.back} style={backButtonStyle} aria-label="Go back">
              ← BACK
            </button>
          ) : (
            <span />
          )}
        </div>

        <div style={centerSlotStyle}>
          {eyebrow ? <div style={eyebrowStyle}>{eyebrow}</div> : null}
          {title ? <div style={titleStyle}>{title}</div> : null}
        </div>

        <div style={trailingSlotStyle}>{trailing ?? <span />}</div>
      </header>

      <section
        style={{
          flex: 1,
          display: "flex",
          flexDirection: "column",
          padding: center ? space.lg : 0,
          justifyContent: center ? "center" : "flex-start",
          alignItems: center ? "center" : "stretch",
          gap: space.md
        }}
      >
        {children}
      </section>
    </main>
  );
}

const primaryButtonStyle: CSSProperties = {
  background: colors.accent,
  color: colors.bg,
  border: "none",
  borderRadius: 10,
  padding: `${space.sm}px ${space.lg}px`,
  fontFamily: fontFamily.display,
  fontSize: font.lg,
  fontWeight: 400,
  letterSpacing: letterSpacing.loose,
  cursor: "pointer",
  width: "100%",
  maxWidth: 320,
  textTransform: "uppercase"
};

const secondaryButtonStyle: CSSProperties = {
  background: "transparent",
  color: colors.text,
  border: `1px solid ${colors.borderStrong}`,
  borderRadius: 10,
  padding: `${space.sm}px ${space.lg}px`,
  fontFamily: fontFamily.display,
  fontSize: font.md,
  letterSpacing: letterSpacing.loose,
  cursor: "pointer",
  width: "100%",
  maxWidth: 320,
  textTransform: "uppercase"
};

const linkButtonStyle: CSSProperties = {
  background: "transparent",
  border: "none",
  color: colors.textMuted,
  cursor: "pointer",
  padding: `${space.xxs}px ${space.xs}px`,
  fontFamily: fontFamily.ui,
  fontSize: font.sm,
  textDecoration: "underline",
  textUnderlineOffset: 3,
  textDecorationStyle: "dotted"
};

export function PrimaryButton(props: React.ButtonHTMLAttributes<HTMLButtonElement>) {
  const { style, ...rest } = props;
  return <button {...rest} style={{ ...primaryButtonStyle, ...style }} />;
}

export function SecondaryButton(props: React.ButtonHTMLAttributes<HTMLButtonElement>) {
  const { style, ...rest } = props;
  return <button {...rest} style={{ ...secondaryButtonStyle, ...style }} />;
}

export function LinkButton(props: React.ButtonHTMLAttributes<HTMLButtonElement>) {
  const { style, ...rest } = props;
  return <button {...rest} style={{ ...linkButtonStyle, ...style }} />;
}
