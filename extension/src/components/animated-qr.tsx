import { useEffect, useState, type CSSProperties } from "react";
import { QRCodeSVG } from "qrcode.react";

import { colors, font, fontFamily, letterSpacing, space } from "../lib/theme";

interface AnimatedQrProps {
  frames: string[];
  size?: number;
  intervalMs?: number;
  level?: "L" | "M" | "Q" | "H";
  bgColor?: string;
  fgColor?: string;
  svgStyle?: CSSProperties;
  showCounter?: boolean;
}

const counterStyle: CSSProperties = {
  marginTop: space.xs,
  color: colors.textMuted,
  fontFamily: fontFamily.mono,
  fontSize: font.xs,
  letterSpacing: letterSpacing.normal
};

export function AnimatedQr({
  frames,
  size = 320,
  intervalMs = 450,
  level = "M",
  bgColor = "#FFFFFF",
  fgColor = "#000000",
  svgStyle,
  showCounter = true
}: AnimatedQrProps) {
  const safeFrames = frames.length > 0 ? frames : [""];
  const [index, setIndex] = useState(0);

  useEffect(() => {
    setIndex(0);
  }, [safeFrames.length, safeFrames[0]]);

  useEffect(() => {
    if (safeFrames.length <= 1) {
      return;
    }
    const timer = window.setInterval(() => {
      setIndex((prev) => (prev + 1) % safeFrames.length);
    }, intervalMs);
    return () => {
      window.clearInterval(timer);
    };
  }, [safeFrames.length, intervalMs]);

  return (
    <div style={{ display: "flex", flexDirection: "column", alignItems: "center", width: "100%" }}>
      <QRCodeSVG
        value={safeFrames[index] ?? safeFrames[0]}
        size={size}
        level={level}
        includeMargin={false}
        bgColor={bgColor}
        fgColor={fgColor}
        style={svgStyle}
      />
      {showCounter && safeFrames.length > 1 ? (
        <p style={counterStyle}>
          Frame {index + 1}/{safeFrames.length}
        </p>
      ) : null}
    </div>
  );
}
