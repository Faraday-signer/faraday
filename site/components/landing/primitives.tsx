import { cn } from "@/lib/utils";

/**
 * Technical section kicker: `[ 02 · SECURITY MODEL ]`. The index and label
 * evoke an instrument readout and orient the reader within the page.
 */
export function SectionLabel({
  index,
  children,
  className,
}: {
  index: string;
  children: React.ReactNode;
  className?: string;
}) {
  return (
    <p
      className={cn(
        "flex items-center gap-2 font-mono text-[11px] uppercase tracking-[0.22em] text-brand sm:text-xs",
        className
      )}
    >
      <span aria-hidden className="text-foreground/30">
        [
      </span>
      <span className="text-foreground/45">{index}</span>
      <span aria-hidden className="text-foreground/25">
        ·
      </span>
      <span className="pixel-shadow-sm">{children}</span>
      <span aria-hidden className="text-foreground/30">
        ]
      </span>
    </p>
  );
}

/** Small monospace pill used for status / metadata tags. */
export function Tag({
  children,
  tone = "muted",
  className,
}: {
  children: React.ReactNode;
  tone?: "muted" | "brand";
  className?: string;
}) {
  return (
    <span
      className={cn(
        "inline-flex items-center gap-1.5 rounded-sm border px-2 py-0.5 font-mono text-[11px] uppercase tracking-[0.14em] sm:text-[11px]",
        tone === "brand"
          ? "border-brand/40 text-brand"
          : "border-border text-muted-foreground",
        className
      )}
    >
      {children}
    </span>
  );
}
