import { cn } from "@/lib/utils";

import { QrGlyph } from "./qr-glyph";

function Node({
  label,
  status,
  statusTone,
  children,
  className,
}: {
  label: string;
  status: string;
  statusTone: "online" | "offline";
  children: React.ReactNode;
  className?: string;
}) {
  return (
    <div
      className={cn(
        "relative flex flex-1 flex-col items-center gap-4 rounded-sm border border-border bg-muted/30 p-6 text-center",
        className
      )}
    >
      <div className="flex items-center gap-2 font-mono text-[10px] uppercase tracking-[0.18em] text-muted-foreground">
        <span
          aria-hidden
          className={cn(
            "h-1.5 w-1.5 rounded-full",
            statusTone === "offline" ? "bg-brand" : "bg-muted-foreground"
          )}
        />
        {status}
      </div>
      {children}
      <p className="font-mono text-[11px] uppercase tracking-[0.16em] text-foreground/80">
        {label}
      </p>
    </div>
  );
}

/**
 * The core Faraday picture: an online machine and the offline signer, with an
 * air gap between them that only QR codes cross. Unsigned transactions go one
 * way, signatures come back — the private key never crosses.
 */
export function AirGapVisual({ className }: { className?: string }) {
  return (
    <div
      className={cn(
        "flex flex-col items-stretch gap-3 md:flex-row md:items-center",
        className
      )}
    >
      <Node label="Browser + extension" status="Online" statusTone="online">
        <QrGlyph className="h-24 w-24 text-foreground/85" title="Unsigned transaction QR" />
      </Node>

      {/* The gap. Horizontal on desktop, vertical on stacked mobile. */}
      <div
        aria-hidden
        className="flex flex-row items-center justify-center gap-2 md:w-40 md:flex-col"
      >
        <div className="h-px flex-1 bg-gradient-to-r from-transparent via-border to-transparent md:h-16 md:w-px md:flex-none md:bg-gradient-to-b" />
        <div className="flex flex-col items-center gap-1 whitespace-nowrap px-1">
          <span className="font-mono text-[9px] uppercase tracking-[0.2em] text-brand-ink">
            Air gap
          </span>
          <span className="font-mono text-[9px] uppercase tracking-[0.14em] text-muted-foreground">
            QR only
          </span>
        </div>
        <div className="h-px flex-1 bg-gradient-to-r from-transparent via-border to-transparent md:h-16 md:w-px md:flex-none md:bg-gradient-to-b" />
      </div>

      <Node label="Faraday signer" status="Offline · no antennas" statusTone="offline">
        <QrGlyph className="h-24 w-24 text-brand-ink" title="Signed transaction QR" />
      </Node>
    </div>
  );
}
