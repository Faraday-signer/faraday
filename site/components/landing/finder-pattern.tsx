interface FinderPatternProps {
  size?: number;
  className?: string;
}

/**
 * QR finder pattern (the corner anchor on every QR code):
 * 7x7 grid — outer ring filled, inner ring empty, center 3x3 filled.
 */
export function FinderPattern({ size = 84, className }: FinderPatternProps) {
  const cell = size / 7;
  return (
    <svg
      width={size}
      height={size}
      viewBox="0 0 7 7"
      shapeRendering="crispEdges"
      className={className}
      aria-hidden="true"
    >
      {/* outer 7x7 frame, 1-cell border */}
      <path
        d="M0 0 H7 V7 H0 Z M1 1 V6 H6 V1 Z"
        fill="currentColor"
        fillRule="evenodd"
      />
      {/* center 3x3 */}
      <rect x="2" y="2" width="3" height="3" fill="currentColor" />
      {/* expose cell size for callers that need to align with grid */}
      <desc>cell={cell}</desc>
    </svg>
  );
}
