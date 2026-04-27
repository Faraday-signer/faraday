interface MeasurementGridProps {
  className?: string;
}

export function MeasurementGrid({ className }: MeasurementGridProps) {
  return (
    <div
      aria-hidden="true"
      className={`absolute inset-0 overflow-hidden pointer-events-none text-foreground ${className ?? ""}`}
    >
      <svg
        className="absolute inset-0 h-full w-full"
        xmlns="http://www.w3.org/2000/svg"
        width="100%"
        height="100%"
      >
        <defs>
          <radialGradient id="fadeGradient" cx="50%" cy="50%" r="75%" fx="50%" fy="50%">
            <stop offset="0%" stopColor="white" stopOpacity="1" />
            <stop offset="100%" stopColor="white" stopOpacity="0" />
          </radialGradient>
          <mask id="fadeMask">
            <rect width="100%" height="100%" fill="url(#fadeGradient)" />
          </mask>

          <pattern id="smallGrid" width="20" height="20" patternUnits="userSpaceOnUse">
            <path
              d="M 20 0 L 0 0 0 20"
              fill="none"
              stroke="currentColor"
              strokeWidth="0.5"
              strokeOpacity="0.14"
            />
          </pattern>

          <pattern id="largeGrid" width="100" height="100" patternUnits="userSpaceOnUse">
            <rect width="100" height="100" fill="url(#smallGrid)" />
            <path
              d="M 100 0 L 0 0 0 100"
              fill="none"
              stroke="currentColor"
              strokeWidth="1"
              strokeOpacity="0.22"
            />
          </pattern>
        </defs>

        <g mask="url(#fadeMask)">
          <rect width="100%" height="100%" fill="url(#largeGrid)" />
        </g>

        {/* Top-left axis L mark */}
        <path
          d="M 28 8 L 8 8 L 8 28"
          fill="none"
          stroke="currentColor"
          strokeWidth="1.5"
          strokeOpacity="0.3"
        />

        {/* Top axis ticks + labels (every 100px) */}
        {Array.from({ length: 18 }).map((_, i) => (
          <g key={`top-${i}`}>
            <line
              x1={100 + i * 100}
              y1="0"
              x2={100 + i * 100}
              y2="10"
              stroke="currentColor"
              strokeWidth="1"
              strokeOpacity="0.22"
            />
            <text
              x={100 + i * 100}
              y="24"
              fill="currentColor"
              fillOpacity="0.2"
              fontSize="9"
              fontFamily="Departure Mono, ui-monospace, monospace"
              textAnchor="middle"
            >
              {(i + 1) * 10}
            </text>
          </g>
        ))}

        {/* Left axis ticks + labels */}
        {Array.from({ length: 18 }).map((_, i) => (
          <g key={`left-${i}`}>
            <line
              x1="0"
              y1={100 + i * 100}
              x2="10"
              y2={100 + i * 100}
              stroke="currentColor"
              strokeWidth="1"
              strokeOpacity="0.22"
            />
            <text
              x="14"
              y={104 + i * 100}
              fill="currentColor"
              fillOpacity="0.2"
              fontSize="9"
              fontFamily="Departure Mono, ui-monospace, monospace"
              textAnchor="start"
            >
              {(i + 1) * 10}
            </text>
          </g>
        ))}

        <text
          x="36"
          y="36"
          fill="currentColor"
          fillOpacity="0.2"
          fontSize="9"
          fontFamily="Departure Mono, ui-monospace, monospace"
        >
          mm
        </text>
      </svg>
    </div>
  );
}
