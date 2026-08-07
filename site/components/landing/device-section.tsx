"use client";

import { useState } from "react";
import { SectionLabel, Tag } from "./primitives";
import { Segmented } from "./segmented";

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
  { k: "Radios", v: "Never linked, CI-audited" },
  { k: "Status", v: "In progress" },
];

export function DeviceSection() {
  const [mode, setMode] = useState<DeviceMode>("build");

  const specs = mode === "build" ? PI_SPECS : ESP32_SPECS;

  return (
    <section id="device" className="border-b border-border bg-background">
      <div className="mx-auto max-w-6xl px-5 py-16 sm:px-8 sm:py-24">
        <div className="grid grid-cols-1 items-center gap-12 lg:grid-cols-[1fr_minmax(0,24rem)] lg:gap-20">
          <div>
            <SectionLabel index="03">The device</SectionLabel>
            <h2 className="mt-5 font-display text-3xl leading-tight tracking-tight text-foreground sm:text-4xl">
              Two devices. One firmware.
            </h2>

            <div className="mt-6">
              <Segmented
                label="Device path"
                value={mode}
                onChange={setMode}
                options={[
                  ["build", "Raspberry Pi Zero"],
                  ["buy", "ESP32-S3"],
                ] as const}
              />
            </div>

            {mode === "build" ? (
              <p key="copy-build" className="mt-6 max-w-xl text-base leading-relaxed text-foreground/70 motion-safe:animate-[content-swap_300ms_ease-out]">
                An open board with no network hardware, a screen, and a camera.
                Every part of it is auditable. The Pi Zero 1.3 is recommended
                precisely because the radio chip isn&apos;t physically there,
                so there is nothing to misconfigure or trust to
                &ldquo;off.&rdquo;
              </p>
            ) : (
              <p key="copy-buy" className="mt-6 max-w-xl text-base leading-relaxed text-foreground/70 motion-safe:animate-[content-swap_300ms_ease-out]">
                One board, no assembly: an ESP32-S3 with a touch screen and
                camera, in a 3D-printed Faraday case. This chip does have
                radio silicon, so the firmware never links the drivers, and
                the build pipeline fails if they so much as appear in the
                binary.
              </p>
            )}

            <div key={`tags-${mode}`} className="mt-8 flex flex-wrap gap-2 motion-safe:animate-[content-swap_300ms_ease-out]">
              {mode === "build" ? (
                <Tag tone="brand">Shipping today · Pi Zero</Tag>
              ) : (
                <Tag tone="brand">In progress · ESP32-S3</Tag>
              )}
              <Tag>Same firmware · same air gap</Tag>
            </div>

          </div>

          <dl key={`dl-${mode}`} className="w-full divide-y divide-border border-t border-border motion-safe:animate-[content-swap_300ms_ease-out]">
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
