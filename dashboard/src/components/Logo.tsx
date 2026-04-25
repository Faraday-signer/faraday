/**
 * Real Faraday brand assets — pixel-art bitmap exported at 60x10 (mark +
 * "Faraday" wordmark) and the standalone mark cropped to its native 8x8
 * grid. The PNGs use transparency, so we render them on whatever surface
 * the parent has — no background required.
 */

// Files live in /public — Vite serves them from the root, no import needed.
const LOGO_SRC = "/logo.png";
const MARK_SRC = "/mark.png";

export function Logo({ height = 22, className = "" }: { height?: number; className?: string }) {
  // Original aspect is 60:10 = 6:1.
  return (
    <img
      src={LOGO_SRC}
      alt="Faraday"
      style={{ height, width: height * 6, imageRendering: "pixelated" }}
      className={`select-none ${className}`}
      draggable={false}
    />
  );
}

export function Mark({ size = 18, className = "" }: { size?: number; className?: string }) {
  return (
    <img
      src={MARK_SRC}
      alt=""
      style={{ width: size, height: size, imageRendering: "pixelated" }}
      className={`select-none ${className}`}
      draggable={false}
    />
  );
}
