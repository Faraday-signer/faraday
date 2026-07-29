interface QrGlyphProps {
  className?: string;
  /** Grid size in modules. Default 21 (QR version-1 dimensions). */
  modules?: number;
  title?: string;
}

/**
 * A stylized, deterministic QR-like glyph — three finder patterns, timing
 * rows, and a fixed data field. It is decorative (not a scannable code), so the
 * module field is generated from a constant seed: identical on server and
 * client, no hydration mismatch, no `Math.random`.
 */
function isFinder(r: number, c: number, n: number): boolean {
  const inBox = (br: number, bc: number) =>
    r >= br && r < br + 7 && c >= bc && c < bc + 7;
  const boxes = [
    [0, 0],
    [0, n - 7],
    [n - 7, 0],
  ];
  for (const [br, bc] of boxes) {
    if (inBox(br, bc)) {
      const lr = r - br;
      const lc = c - bc;
      const ring = lr === 0 || lr === 6 || lc === 0 || lc === 6;
      const core = lr >= 2 && lr <= 4 && lc >= 2 && lc <= 4;
      return ring || core;
    }
  }
  return false;
}

function isFinderZone(r: number, c: number, n: number): boolean {
  return (
    (r < 8 && c < 8) || (r < 8 && c >= n - 8) || (r >= n - 8 && c < 8)
  );
}

export function QrGlyph({ className, modules = 21, title }: QrGlyphProps) {
  const n = modules;
  const cells: { r: number; c: number }[] = [];

  for (let r = 0; r < n; r++) {
    for (let c = 0; c < n; c++) {
      if (isFinderZone(r, c, n)) {
        if (isFinder(r, c, n)) cells.push({ r, c });
        continue;
      }
      // Timing patterns on row/col 6.
      if (r === 6 || c === 6) {
        if ((r + c) % 2 === 0) cells.push({ r, c });
        continue;
      }
      // Deterministic data field from a fixed integer hash.
      const h = (r * 73856093) ^ (c * 19349663) ^ ((r + c) * 83492791);
      if ((h & 7) > 3) cells.push({ r, c });
    }
  }

  return (
    <svg
      viewBox={`0 0 ${n} ${n}`}
      className={className}
      shapeRendering="crispEdges"
      fill="currentColor"
      role={title ? "img" : "presentation"}
      aria-label={title}
      aria-hidden={title ? undefined : true}
    >
      {cells.map(({ r, c }) => (
        <rect key={`${r}-${c}`} x={c} y={r} width={1} height={1} />
      ))}
    </svg>
  );
}
