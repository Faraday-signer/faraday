"use client";

import { useState } from "react";
import dynamic from "next/dynamic";

import { SectionLabel } from "./primitives";

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

const STEPS = [
  { n: "01", title: "Build", short: "The extension builds the unsigned tx" },
  { n: "02", title: "Cross the gap", short: "QR out, camera in. Light only" },
  { n: "03", title: "Review & sign", short: "The device decodes and you approve" },
  { n: "04", title: "Return & submit", short: "Signed QR back, extension submits" },
];

/** Device pose per phase: idle three-quarter, spun so the back camera faces
 * the extension's QR, face-on for review, face-on showing the signed QR. */
const POSES = [-0.55, 2.35, 0.05, 0.05];

/**
 * The round-trip story, fully reader-paced: four clearly clickable step
 * cards drive the scene between the extension and the 3D device. No
 * auto-play, no pinning — the section is a normal block and nothing moves
 * until the visitor asks it to.
 */
export function HowItWorksScroll() {
  const [phase, setPhase] = useState(0);

  return (
    <section id="how-it-works" className="border-b border-border bg-background">
      <div className="flex flex-col py-14 sm:py-16">
        <div className="mx-auto w-full max-w-6xl px-5 sm:px-8">
          <SectionLabel index="01">How it works</SectionLabel>
          <h2 className="mt-2 max-w-2xl font-display text-2xl leading-tight tracking-tight text-foreground sm:text-3xl">
            One round trip across the air gap
          </h2>
        </div>

        <div className="mx-auto mt-14 flex w-full max-w-6xl flex-col items-center px-5 sm:px-8">
          <Device3D
            device="esp32"
            holdYaw={POSES[phase]}
            screen={phase === 0 ? "menu" : phase === 3 ? "signed" : "review"}
          />
          <p className="mt-4 font-mono text-[10px] uppercase tracking-[0.2em] text-muted-foreground">
            {phase === 0 && "offline · idle"}
            {phase === 1 && "camera reads the extension's qr"}
            {phase === 2 && "review on the screen"}
            {phase === 3 && "signed qr · scan it back"}
          </p>
        </div>

        {/* the controls: four unmistakably clickable step cards */}
        <div className="mx-auto mt-14 w-full max-w-6xl px-5 sm:px-8">
          <ol className="grid grid-cols-2 gap-2 sm:grid-cols-4 sm:gap-3">
            {STEPS.map((step, i) => (
              <li key={step.n} aria-current={phase === i ? "step" : undefined}>
                <button
                  type="button"
                  onClick={() => setPhase(i)}
                  aria-pressed={phase === i}
                  className={`group h-full w-full cursor-pointer rounded-sm border p-3 text-left transition-[border-color,background-color,transform] duration-200 active:scale-[0.98] focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2 focus-visible:ring-offset-background sm:p-4 ${
                    phase === i
                      ? "border-brand/60 bg-brand/[0.06]"
                      : "border-border bg-muted/40 hover:border-foreground/25 hover:bg-foreground/[0.04]"
                  }`}
                >
                  <span
                    className={`inline-flex h-7 w-7 items-center justify-center rounded-sm font-mono text-xs transition-colors duration-200 ${
                      phase === i
                        ? "bg-primary text-brand"
                        : "bg-foreground/10 text-foreground/60 group-hover:bg-foreground/15"
                    }`}
                  >
                    {step.n}
                  </span>
                  <span
                    className={`mt-2.5 block font-display text-sm tracking-tight sm:text-base ${
                      phase === i ? "text-foreground" : "text-foreground/70"
                    }`}
                  >
                    {step.title}
                  </span>
                  <span className="mt-1 hidden text-xs leading-relaxed text-foreground/55 sm:block">
                    {step.short}
                  </span>
                </button>
              </li>
            ))}
          </ol>
        </div>
      </div>
    </section>
  );
}
