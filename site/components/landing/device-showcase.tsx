"use client";

import { useState } from "react";
import { QrGlyph } from "./qr-glyph";

type Device = "pi" | "esp32";

/** Three firmware screens the device cycles through, ambient (9s loop, CSS-only). */
function ScreenCycle({ compact = false }: { compact?: boolean }) {
  const text = compact ? "text-[7px]" : "text-[8px]";
  return (
    <div className="screen-cycle relative h-full w-full overflow-hidden bg-background">
      {/* 1 — menu (mirrors the real UI) */}
      <div className="screen-frame absolute inset-0 flex flex-col p-2" style={{ animationDelay: "0s" }}>
        <p className={`font-mono ${text} uppercase tracking-widest text-brand-ink`}>▚ Faraday</p>
        <div className="mt-1.5 border border-brand/60 bg-brand/15 px-1.5 py-1">
          <p className={`font-mono ${text} uppercase text-brand-ink`}>Create · new wallet</p>
        </div>
        <p className={`mt-1.5 px-1.5 font-mono ${text} uppercase text-muted-foreground`}>Load · existing</p>
        <p className={`mt-1 px-1.5 font-mono ${text} uppercase text-muted-foreground`}>About</p>
      </div>
      {/* 2 — transaction review */}
      <div className="screen-frame absolute inset-0 flex flex-col p-2" style={{ animationDelay: "3s" }}>
        <p className={`font-mono ${text} uppercase tracking-widest text-brand-ink`}>Review</p>
        <p className={`mt-1.5 font-mono ${text} uppercase text-foreground`}>Send 1.20 SOL</p>
        <p className={`mt-1 font-mono ${text} text-muted-foreground`}>to 7xKX…9fWq</p>
        <p className={`mt-auto font-mono ${text} uppercase text-brand-ink`}>✓ Sign</p>
      </div>
      {/* 3 — signed QR */}
      <div className="screen-frame absolute inset-0 flex items-center justify-center p-2" style={{ animationDelay: "6s" }}>
        <QrGlyph className="h-4/5 w-auto text-brand-ink" title="Signed transaction QR" />
      </div>
    </div>
  );
}

function PiDevice() {
  return (
    <div className="relative h-52 w-64 rounded-md border border-brand/50 bg-muted/20 shadow-[0_0_40px_-18px_var(--brand)]">
      {/* square 240×240 screen */}
      <div className="absolute left-1/2 top-1/2 h-36 w-36 -translate-x-1/2 -translate-y-1/2 border border-brand/40">
        <ScreenCycle />
      </div>
      {/* camera dot */}
      <div className="absolute right-4 top-4 h-3 w-3 rounded-full border border-brand/60" />
      <span className="absolute bottom-2 left-3 font-mono text-[9px] uppercase tracking-[0.18em] text-muted-foreground">
        pi zero · 240×240 · no radio footprint
      </span>
    </div>
  );
}

function Esp32Device() {
  return (
    <div className="relative h-64 w-48 rounded-md border border-brand/50 bg-muted/20 shadow-[0_0_40px_-18px_var(--brand)]">
      {/* portrait touch screen with softkey bar, like the real UI */}
      <div className="absolute left-1/2 top-6 h-44 w-36 -translate-x-1/2 border border-brand/40">
        <ScreenCycle compact />
      </div>
      <div className="absolute bottom-6 left-1/2 flex w-36 -translate-x-1/2 justify-between border border-brand/30 px-4 py-1">
        <span className="font-mono text-[9px] text-muted-foreground">↩</span>
        <span className="font-mono text-[9px] text-brand-ink">✓</span>
      </div>
      {/* embossed wordmark, like the printed case */}
      <span className="absolute -left-1 top-1/2 -translate-y-1/2 -rotate-90 font-mono text-[8px] uppercase tracking-[0.3em] text-foreground/25">
        ▚Faraday
      </span>
      <span className="absolute bottom-1 left-3 font-mono text-[9px] uppercase tracking-[0.18em] text-muted-foreground">
        esp32-s3 · touch · onboard cam
      </span>
    </div>
  );
}

export function DeviceShowcase() {
  const [device, setDevice] = useState<Device>("esp32");

  return (
    <div className="flex flex-col items-center gap-8">
      <div className="device-float motion-reduce:animate-none">
        {device === "pi" ? <PiDevice /> : <Esp32Device />}
      </div>

      <div className="inline-flex rounded-sm border border-border p-1" role="tablist" aria-label="Device">
        {(
          [
            ["esp32", "ESP32-S3"],
            ["pi", "Raspberry Pi"],
          ] as const
        ).map(([value, label]) => (
          <button
            key={value}
            role="tab"
            aria-selected={device === value}
            onClick={() => setDevice(value)}
            className={`rounded-[1px] px-4 py-1.5 font-mono text-[11px] uppercase tracking-[0.16em] transition-colors ${
              device === value ? "bg-brand/10 text-brand-ink" : "text-muted-foreground hover:text-foreground"
            }`}
          >
            {label}
          </button>
        ))}
      </div>

      <p className="font-mono text-[10px] uppercase tracking-[0.2em] text-muted-foreground">
        two devices · same firmware · running today
      </p>
    </div>
  );
}
