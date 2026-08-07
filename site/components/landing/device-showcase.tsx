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
        className="inline-flex rounded-sm border border-border p-1"
        role="group"
        aria-label="Choose device"
      >
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
            className={`rounded-[1px] px-4 py-1.5 font-mono text-[11px] uppercase tracking-[0.16em] transition-colors focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2 focus-visible:ring-offset-background ${
              device === value
                ? "bg-primary text-brand"
                : "text-muted-foreground hover:text-foreground"
            }`}
          >
            {label}
          </button>
        ))}
      </div>

      <button
        aria-pressed={open}
        onClick={() => setOpen((v) => !v)}
        className="font-mono text-[11px] uppercase tracking-[0.16em] text-muted-foreground underline decoration-dotted underline-offset-4 transition-colors hover:text-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2 focus-visible:ring-offset-background"
      >
        {open ? "close it up" : "look inside"}
      </button>

      <p className="font-mono text-[11px] uppercase tracking-[0.2em] text-muted-foreground">
        two devices · same firmware
      </p>
    </div>
  );
}
