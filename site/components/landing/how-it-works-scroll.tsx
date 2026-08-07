"use client";

import { useEffect, useRef, useState } from "react";
import dynamic from "next/dynamic";

import { QrGlyph } from "./qr-glyph";
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
  {
    n: "01",
    title: "Build",
    body: "Your wallet or a dapp builds an unsigned transaction in the browser, exactly as it would with any wallet.",
  },
  {
    n: "02",
    title: "Cross the gap",
    body: "The Faraday extension renders the transaction as a QR code. The device's camera reads it. Nothing electrical connects the two. The only channel is light.",
  },
  {
    n: "03",
    title: "Review on-device",
    body: "Faraday decodes the raw bytes offline, with no RPC or lookups, and shows exactly what you're about to sign in plain terms. You approve with the buttons.",
  },
  {
    n: "04",
    title: "Sign & return",
    body: "The device signs in RAM and displays the signature as a QR. The browser scans it back and submits to Solana. The private key never crossed.",
  },
];

/** Device pose per phase: intro three-quarter, spun around so the back camera
 * faces the browser panel, face-on for the review, and a soft angle for the
 * return leg. */
const POSES = [-0.55, 2.35, 0.05, -0.9];

/**
 * Option 2 — the scroll story. The section is four viewports tall; a pinned
 * panel advances the four phases as you scroll, steering the real 3D device
 * between poses. No wheel hijacking: native scroll position simply maps to a
 * phase, so speed and direction always belong to the visitor.
 */
export function HowItWorksScroll() {
  const sectionRef = useRef<HTMLElement>(null);
  const [phase, setPhase] = useState(0);

  useEffect(() => {
    let raf = 0;
    const onScroll = () => {
      cancelAnimationFrame(raf);
      raf = requestAnimationFrame(() => {
        const el = sectionRef.current;
        if (!el) return;
        const rect = el.getBoundingClientRect();
        const scrollable = rect.height - window.innerHeight;
        if (scrollable <= 0) return;
        const progress = Math.min(1, Math.max(0, -rect.top / scrollable));
        setPhase(Math.min(3, Math.floor(progress * 4)));
      });
    };
    onScroll();
    window.addEventListener("scroll", onScroll, { passive: true });
    return () => {
      cancelAnimationFrame(raf);
      window.removeEventListener("scroll", onScroll);
    };
  }, []);

  return (
    <section
      ref={sectionRef}
      id="how-it-works-scroll"
      className="relative h-[400svh] border-b border-border bg-background"
    >
      <div className="sticky top-0 flex h-svh flex-col overflow-hidden">
        <div className="mx-auto w-full max-w-6xl px-5 pt-14 sm:px-8">
          <SectionLabel index="01">How it works</SectionLabel>
          <h2 className="mt-4 max-w-2xl font-display text-3xl leading-tight tracking-tight text-foreground sm:text-4xl">
            One round trip across the air gap
          </h2>
        </div>

        <div className="mx-auto grid w-full max-w-6xl flex-1 grid-cols-1 items-center gap-8 px-5 sm:px-8 lg:grid-cols-[1fr_auto]">
          {/* left: the step list, active one expanded */}
          <ol className="max-w-xl">
            {STEPS.map((step, i) => (
              <li
                key={step.n}
                className={`border-l-2 py-3 pl-5 transition-colors duration-300 ${
                  phase === i ? "border-brand" : "border-border"
                }`}
                aria-current={phase === i ? "step" : undefined}
              >
                <div className="flex items-baseline gap-3">
                  <span
                    className={`font-mono text-xs transition-colors duration-300 ${
                      phase === i ? "text-brand" : "text-foreground/40"
                    }`}
                  >
                    {step.n}
                  </span>
                  <h3
                    className={`font-display text-lg tracking-tight transition-colors duration-300 sm:text-xl ${
                      phase === i ? "text-foreground" : "text-foreground/50"
                    }`}
                  >
                    {step.title}
                  </h3>
                </div>
                <div
                  className={`grid transition-[grid-template-rows,opacity] duration-500 ease-out ${
                    phase === i
                      ? "grid-rows-[1fr] opacity-100"
                      : "grid-rows-[0fr] opacity-0"
                  }`}
                >
                  <p className="overflow-hidden text-[15px] leading-relaxed text-foreground/75">
                    {step.body}
                  </p>
                </div>
              </li>
            ))}
          </ol>

          {/* right: browser chip + the real device, steered per phase */}
          <div className="relative mx-auto flex flex-col items-center">
            <div
              aria-hidden
              className={`absolute -left-28 top-1/2 hidden -translate-y-1/2 flex-col items-center gap-2 transition-opacity duration-500 lg:flex ${
                phase === 1 || phase === 3 ? "opacity-100" : "opacity-0"
              }`}
            >
              <QrGlyph
                className={`h-16 w-16 ${phase === 3 ? "text-brand" : "text-foreground/70"}`}
              />
              <span className="font-mono text-[10px] uppercase tracking-[0.16em] text-muted-foreground">
                {phase === 3 ? "signed qr" : "tx qr"}
              </span>
            </div>
            <Device3D device="esp32" holdYaw={POSES[phase]} />
            <p className="mt-2 font-mono text-[11px] uppercase tracking-[0.2em] text-muted-foreground">
              {phase === 0 && "unsigned tx incoming"}
              {phase === 1 && "camera reads the qr · light only"}
              {phase === 2 && "review on the device screen"}
              {phase === 3 && "signature returns as a qr"}
            </p>
          </div>
        </div>

        {/* scroll progress */}
        <div className="mx-auto mb-8 w-full max-w-6xl px-5 sm:px-8">
          <div className="h-px w-full bg-border">
            <div
              className="h-px bg-brand transition-[width] duration-300 ease-out"
              style={{ width: `${((phase + 1) / 4) * 100}%` }}
            />
          </div>
        </div>
      </div>
    </section>
  );
}
