import { cn } from "@/lib/utils";

interface FaradayMarkProps {
  className?: string;
  size?: number;
}

/**
 * The 4-square Faraday brand mark. Two big squares on the diagonal,
 * two smaller squares on the off-diagonal. Path traced from
 * faraday-icon-final-final.svg.
 */
export function FaradayMark({ className, size }: FaradayMarkProps) {
  return (
    <svg
      width={size}
      height={size}
      viewBox="0 0 103 103"
      fill="currentColor"
      shapeRendering="crispEdges"
      className={cn(className)}
      aria-hidden="true"
    >
      <rect width="51.5" height="51.5" />
      <rect x="51.5" y="51.5" width="51.5" height="51.5" />
      <rect x="77.7358" width="25.2642" height="25.2642" />
      <rect y="77.7358" width="25.2642" height="25.2642" />
    </svg>
  );
}
