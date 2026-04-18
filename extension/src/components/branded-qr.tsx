import type { CSSProperties, ReactNode } from "react";
import { QRCodeSVG } from "qrcode.react";

import { flowColors, fontFamily, font, letterSpacing, radius, space } from "../lib/theme";
import type { FlowKind } from "../lib/theme";

interface BrandedQRProps {
  /** Flow context — drives frame color and default label. */
  flow: FlowKind;
  /** Payload encoded into the QR. Pure black-on-white modules regardless of flow. */
  value: string;
  /** Pixel size of the QR matrix itself (not including the card padding). Default 280. */
  size?: number;
  /** Override the flow's default label. */
  label?: string;
  /** Optional small leading icon in the header strip. */
  icon?: ReactNode;
  /** Secondary line under the QR. Address, amount, note, etc. */
  caption?: ReactNode;
  /** QR error correction level. Higher level = denser code but more robust. */
  errorLevel?: "L" | "M" | "Q" | "H";
}

/**
 * Branded QR card.
 *
 * The card itself (frame, header strip, caption) carries the brand. The QR
 * matrix is always pure black on pure white with a full quiet zone so any
 * camera — including the Pi's OV5647 — can read it reliably.
 *
 * Do not tint the modules. Do not tint the background. Do not place a logo
 * inside the QR unless we formally bump error correction to H and measure
 * scan reliability on-device first.
 */
export function BrandedQR({
  flow,
  value,
  size = 280,
  label,
  icon,
  caption,
  errorLevel = "M"
}: BrandedQRProps) {
  const flowColor = flowColors[flow];
  const displayLabel = label ?? flowColor.label;

  const cardStyle: CSSProperties = {
    display: "inline-block",
    background: flowColor.primary,
    borderRadius: radius.xl,
    padding: 3,
    boxShadow: `0 0 0 1px ${flowColor.primary}, 0 24px 48px rgba(0, 0, 0, 0.45)`
  };

  const innerStyle: CSSProperties = {
    background: "#FFFFFF",
    borderRadius: radius.lg,
    overflow: "hidden",
    display: "flex",
    flexDirection: "column"
  };

  const headerStyle: CSSProperties = {
    background: flowColor.primary,
    color: flowColor.onPrimary,
    padding: `${space.xs}px ${space.sm}px`,
    display: "flex",
    alignItems: "center",
    gap: space.xs,
    fontFamily: fontFamily.display,
    fontSize: font.xs,
    letterSpacing: letterSpacing.wider,
    textTransform: "uppercase"
  };

  const qrAreaStyle: CSSProperties = {
    padding: space.md,
    display: "grid",
    placeItems: "center",
    background: "#FFFFFF"
  };

  const captionStyle: CSSProperties = {
    padding: `${space.xs}px ${space.sm}px ${space.sm}px`,
    background: "#FFFFFF",
    borderTop: "1px solid rgba(0, 0, 0, 0.06)",
    color: "#001721",
    fontFamily: fontFamily.mono,
    fontSize: font.xs,
    letterSpacing: letterSpacing.normal,
    textAlign: "center"
  };

  return (
    <div style={cardStyle}>
      <div style={innerStyle}>
        <div style={headerStyle}>
          {icon ? <span style={{ display: "inline-flex" }}>{icon}</span> : null}
          <span>{displayLabel}</span>
        </div>
        <div style={qrAreaStyle}>
          <QRCodeSVG
            value={value}
            size={size}
            level={errorLevel}
            includeMargin={false}
            bgColor="#FFFFFF"
            fgColor="#000000"
          />
        </div>
        {caption ? <div style={captionStyle}>{caption}</div> : null}
      </div>
    </div>
  );
}
