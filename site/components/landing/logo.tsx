import { cn } from "@/lib/utils";

import { FaradayWordmark } from "./faraday-wordmark";

interface LogoProps {
  className?: string;
  /** Pixel height of the wordmark. Default 32px. */
  height?: number;
}

export function Logo({ className, height = 32 }: LogoProps) {
  return (
    <FaradayWordmark
      height={height}
      className={cn("text-brand", className)}
      style={{
        filter: "drop-shadow(1px 1px 0 var(--muted-foreground))",
      }}
    />
  );
}
