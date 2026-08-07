"use client";

import { useState } from "react";
import dynamic from "next/dynamic";
import type { Device3DName } from "./device-3d";

const Device3D = dynamic(() => import("./device-3d").then((m) => m.Device3D), {
  ssr: false,
  loading: () => (
    <div className="flex h-72 w-72 items-center justify-center sm:h-80 sm:w-80">
      <span className="font-mono text-[11px] uppercase tracking-[0.2em] text-muted-foreground">
        loading device…
      </span>
    </div>
  ),
});

export function DeviceShowcase() {
  const [device, setDevice] = useState<Device3DName>("esp32");
  const [open, setOpen] = useState(false);

  return (
    <div className="flex flex-col items-center gap-6">
      <Device3D device={device} open={open} />

      <div
        className="relative inline-grid grid-cols-2 rounded-sm border border-border p-1"
        role="group"
        aria-label="Choose device"
      >
        {/* sliding active pill */}
        <span
          aria-hidden
          className={`absolute inset-y-1 left-1 w-[calc(50%-4px)] rounded-[1px] bg-primary transition-transform duration-300 ease-out ${
            device === "pi" ? "translate-x-full" : ""
          }`}
        />
        {(
          [
            ["esp32", "ESP32-S3"],
            ["pi", "Raspberry Pi"],
          ] as const
        ).map(([value, label]) => (
          <button
            key={value}
            aria-pressed={device === value}
            onClick={() => setDevice(value)}
            className={`relative z-10 cursor-pointer rounded-[1px] px-4 py-1.5 font-mono text-[11px] uppercase tracking-[0.16em] transition-[color,transform] duration-300 active:scale-[0.96] focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2 focus-visible:ring-offset-background ${
              device === value
                ? "text-brand"
                : "text-muted-foreground hover:bg-foreground/[0.04] hover:text-foreground"
            }`}
          >
            {label}
          </button>
        ))}
      </div>

      <button
        aria-pressed={open}
        onClick={() => setOpen((v) => !v)}
        className="group cursor-pointer font-mono text-[11px] uppercase tracking-[0.16em] text-muted-foreground underline decoration-dotted underline-offset-4 transition-colors hover:text-brand-ink hover:decoration-solid focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2 focus-visible:ring-offset-background"
      >
        {open ? "close it up" : "look inside"}
        <span className="ml-1.5 inline-block transition-transform duration-300 group-hover:translate-y-0.5">
          {open ? "↑" : "↓"}
        </span>
      </button>

      <p className="font-mono text-[11px] uppercase tracking-[0.2em] text-muted-foreground">
        two devices · same firmware
      </p>
    </div>
  );
}
