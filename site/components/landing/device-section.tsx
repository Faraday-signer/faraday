"use client";

import { useState } from "react";
import { SectionLabel, Tag } from "./primitives";

type DeviceMode = "build" | "buy";

const PI_SPECS = [
  { k: "Computer", v: "Raspberry Pi Zero 1.3" },
  { k: "Display", v: "Waveshare 1.3\" LCD · 240×240" },
  { k: "Camera", v: "Pi Camera v1.3 (OV5647)" },
  { k: "I/O channel", v: "Camera in · screen out" },
  { k: "Radios", v: "None on the board" },
  { k: "Approx. cost", v: "~$35" },
];

const ESP32_SPECS = [
  { k: "Board", v: "ESP32-S3 touch LCD" },
  { k: "Display", v: "2\" touch · portrait" },
  { k: "Camera", v: "Onboard" },
  { k: "I/O channel", v: "Camera in · screen out" },
  { k: "Radios", v: "Never linked — CI-audited" },
  { k: "Status", v: "In progress" },
];

export function DeviceSection() {
  const [mode, setMode] = useState<DeviceMode>("build");

  const specs = mode === "build" ? PI_SPECS : ESP32_SPECS;

  return (
    <section id="device" className="border-b border-border">
      <div className="mx-auto max-w-6xl px-5 py-16 sm:px-8 sm:py-24">
        <div className="max-w-2xl">
            <SectionLabel index="03">The device</SectionLabel>
            <h2 className="mt-5 font-display text-3xl leading-tight tracking-tight text-foreground sm:text-4xl">
              Two devices. One firmware.
            </h2>

            <div
              className="mt-6 inline-flex rounded-sm border border-border p-1"
              role="tablist"
              aria-label="Device path"
            >
              {(
                [
                  ["build", "Raspberry Pi Zero"],
                  ["buy", "ESP32-S3"],
                ] as const
              ).map(([value, label]) => (
                <button
                  key={value}
                  role="tab"
                  aria-selected={mode === value}
                  onClick={() => setMode(value)}
                  className={`rounded-[1px] px-4 py-2 font-mono text-[11px] uppercase tracking-[0.16em] transition-colors ${
                    mode === value
                      ? "bg-brand/10 text-brand-ink"
                      : "text-muted-foreground hover:text-foreground"
                  }`}
                >
                  {label}
                </button>
              ))}
            </div>

            {mode === "build" ? (
              <p className="mt-6 max-w-xl text-base leading-relaxed text-foreground/70">
                No secure element you have to take on faith — just an open board
                with no network hardware, a screen, and a camera. The Pi Zero 1.3
                is recommended precisely because the radio chip isn&apos;t
                physically there, so there is nothing to misconfigure or trust
                to &ldquo;off.&rdquo;
              </p>
            ) : (
              <p className="mt-6 max-w-xl text-base leading-relaxed text-foreground/70">
                One board, no assembly: an ESP32-S3 with a touch screen and
                camera, in a 3D-printed Faraday case. You&apos;ll flash the
                firmware straight from this site when the web flasher ships.
                This chip does have radio silicon — so the firmware never links
                the drivers, and the build pipeline fails if they so much as
                appear in the binary.
              </p>
            )}

            <div className="mt-8 flex flex-wrap gap-2">
              {mode === "build" ? (
                <Tag tone="brand">Shipping today · Pi Zero</Tag>
              ) : (
                <Tag tone="brand">In progress · flash from the browser</Tag>
              )}
              <Tag>Same firmware · same air gap</Tag>
            </div>

            <dl className="mt-8 max-w-xl divide-y divide-border border-t border-border">
              {specs.map((spec) => (
                <div
                  key={spec.k}
                  className="flex items-baseline justify-between gap-4 py-2.5"
                >
                  <dt className="font-mono text-[11px] uppercase tracking-[0.14em] text-muted-foreground">
                    {spec.k}
                  </dt>
                  <dd className="text-right font-mono text-sm text-foreground">
                    {spec.v}
                  </dd>
                </div>
              ))}
            </dl>
        </div>
      </div>
    </section>
  );
}
