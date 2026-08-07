"use client";

import { useEffect, useRef, useState } from "react";

import { QrGlyph } from "./qr-glyph";
import { SectionLabel } from "./primitives";

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

const PHASE_MS = 3200;

function PanelShell({
  status,
  statusTone,
  label,
  children,
}: {
  status: string;
  statusTone: "online" | "offline";
  label: string;
  children: React.ReactNode;
}) {
  return (
    <div className="relative flex flex-1 flex-col items-center gap-4 rounded-sm border border-border bg-muted/30 p-6 text-center">
      <div className="flex items-center gap-2 font-mono text-[11px] uppercase tracking-[0.18em] text-muted-foreground">
        <span
          aria-hidden
          className={`h-1.5 w-1.5 rounded-full ${
            statusTone === "offline" ? "bg-brand" : "bg-muted-foreground"
          }`}
        />
        {status}
      </div>
      <div className="relative h-32 w-full">{children}</div>
      <p className="font-mono text-[11px] uppercase tracking-[0.16em] text-foreground/80">
        {label}
      </p>
    </div>
  );
}

/** Crossfading stack cell: children rendered on top of each other, the active
 * one visible. Keeps panel height stable (no layout shift between phases). */
function Slide({ active, children }: { active: boolean; children: React.ReactNode }) {
  return (
    <div
      className={`absolute inset-0 flex flex-col items-center justify-center gap-2 transition-opacity duration-500 ${
        active ? "opacity-100" : "pointer-events-none opacity-0"
      }`}
    >
      {children}
    </div>
  );
}

/** Mini firmware review screen, echoing the real device UI. */
function MiniReview() {
  return (
    <div className="w-40 rounded-[3px] bg-[#001721] p-2.5 text-left font-mono text-[9px] leading-relaxed">
      <p className="text-brand">APPROVE SEND</p>
      <div className="my-1 h-px bg-[#114358]" />
      <p className="text-[#8C9CA8]">
        FROM <span className="text-brand">(you)</span>
      </p>
      <p className="truncate text-[#E7E7E7]">7xKXtg2CW87d97TXJSDpbD</p>
      <p className="mt-0.5 text-[#8C9CA8]">TO</p>
      <p className="truncate text-[#E7E7E7]">9WzDXwBbmkg8ZTbNMqUxvQ</p>
      <div className="mt-1 flex items-baseline justify-between">
        <span className="text-[#8C9CA8]">AMOUNT</span>
        <span className="text-[#E7E7E7]">
          1.5 <span className="text-brand">SOL</span>
        </span>
      </div>
    </div>
  );
}

/**
 * Option 3 — the live round trip. An auto-advancing loop plays the four
 * phases; hovering or focusing the visual pauses it (WCAG 2.2.2), clicking a
 * step takes manual control, and reduced-motion visitors get the loop
 * advancing by crossfade only (the traveling chip never moves for them).
 */
export function HowItWorksLoop() {
  const [phase, setPhase] = useState(0);
  const [manual, setManual] = useState(false);
  const paused = useRef(false);

  useEffect(() => {
    if (manual) return;
    const id = setInterval(() => {
      if (!paused.current) setPhase((p) => (p + 1) % 4);
    }, PHASE_MS);
    return () => clearInterval(id);
  }, [manual]);

  return (
    <section id="how-it-works" className="border-b border-border bg-background">
      <div className="mx-auto max-w-6xl px-5 py-16 sm:px-8 sm:py-24">
        <SectionLabel index="01">How it works</SectionLabel>
        <h2 className="mt-5 max-w-2xl font-display text-3xl leading-tight tracking-tight text-foreground sm:text-4xl">
          One round trip across the air gap
        </h2>

        {/* the animated round trip */}
        <div
          aria-hidden
          className="mt-12 flex flex-col items-stretch gap-3 md:flex-row md:items-center"
          onMouseEnter={() => (paused.current = true)}
          onMouseLeave={() => (paused.current = false)}
        >
          <PanelShell status="Online" statusTone="online" label="Browser + extension">
            <Slide active={phase === 0}>
              <div className="w-40 space-y-1.5 text-left">
                <div className="h-2 w-full animate-pulse rounded-[1px] bg-foreground/15" />
                <div className="h-2 w-3/4 animate-pulse rounded-[1px] bg-foreground/15" />
                <div className="h-2 w-5/6 animate-pulse rounded-[1px] bg-foreground/15" />
                <p className="pt-1 font-mono text-[10px] uppercase tracking-[0.16em] text-muted-foreground">
                  Building tx · 206 B
                </p>
              </div>
            </Slide>
            <Slide active={phase === 1}>
              <QrGlyph className="h-24 w-24 text-foreground/85" />
              <p className="font-mono text-[10px] uppercase tracking-[0.16em] text-muted-foreground">
                Emitting QR
              </p>
            </Slide>
            <Slide active={phase === 2}>
              <QrGlyph className="h-24 w-24 text-foreground/25" />
              <p className="font-mono text-[10px] uppercase tracking-[0.16em] text-muted-foreground">
                Waiting for device
              </p>
            </Slide>
            <Slide active={phase === 3}>
              <span className="font-display text-3xl text-brand">✓</span>
              <p className="font-mono text-[10px] uppercase tracking-[0.16em] text-muted-foreground">
                Signature scanned · submitted
              </p>
            </Slide>
          </PanelShell>

          {/* the gap, with the traveling QR chip */}
          <div className="relative flex flex-row items-center justify-center gap-2 md:w-40 md:flex-col">
            <div className="h-px flex-1 bg-gradient-to-r from-transparent via-border to-transparent md:h-16 md:w-px md:flex-none md:bg-gradient-to-b" />
            <div className="flex flex-col items-center gap-1 whitespace-nowrap px-1">
              <span className="pixel-shadow-sm font-mono text-[11px] uppercase tracking-[0.2em] text-brand">
                Air gap
              </span>
              <span className="font-mono text-[11px] uppercase tracking-[0.14em] text-muted-foreground">
                QR only
              </span>
            </div>
            <div className="h-px flex-1 bg-gradient-to-r from-transparent via-border to-transparent md:h-16 md:w-px md:flex-none md:bg-gradient-to-b" />

            {/* chip crossing: right during phase 1, back-left during phase 3 */}
            {phase === 1 && (
              <QrGlyph className="absolute left-0 top-1/2 hidden h-7 w-7 -translate-y-1/2 text-foreground/80 motion-safe:animate-[gap-cross_2.8s_linear_forwards] motion-reduce:hidden md:block" />
            )}
            {phase === 3 && (
              <QrGlyph className="absolute right-0 top-1/2 hidden h-7 w-7 -translate-y-1/2 text-brand motion-safe:animate-[gap-return_2.8s_linear_forwards] motion-reduce:hidden md:block" />
            )}
          </div>

          <PanelShell
            status="Offline · no antennas"
            statusTone="offline"
            label="Faraday signer"
          >
            <Slide active={phase === 0}>
              <div className="flex h-24 w-40 items-center justify-center rounded-[3px] bg-[#001721]">
                <p className="font-mono text-[10px] uppercase tracking-[0.2em] text-[#5E7180]">
                  Ready
                </p>
              </div>
            </Slide>
            <Slide active={phase === 1}>
              <div className="relative flex h-24 w-40 items-center justify-center rounded-[3px] bg-[#001721]">
                <span className="absolute h-10 w-10 rounded-full border border-brand/40 motion-safe:animate-ping" />
                <span className="h-2 w-2 rounded-full bg-brand" />
              </div>
              <p className="font-mono text-[10px] uppercase tracking-[0.16em] text-muted-foreground">
                Camera scanning
              </p>
            </Slide>
            <Slide active={phase === 2}>
              <MiniReview />
            </Slide>
            <Slide active={phase === 3}>
              <QrGlyph className="h-24 w-24 text-brand" />
              <p className="font-mono text-[10px] uppercase tracking-[0.16em] text-muted-foreground">
                Signed QR on screen
              </p>
            </Slide>
          </PanelShell>
        </div>

        {/* step cards, lit in sync; clicking takes manual control */}
        <ol className="mt-12 grid grid-cols-1 gap-px overflow-hidden rounded-sm border border-border bg-border sm:grid-cols-2 lg:grid-cols-4">
          {STEPS.map((step, i) => (
            <li key={step.n} className="relative bg-muted">
              {phase === i && !manual && (
                <span
                  aria-hidden
                  key={phase}
                  className="absolute left-0 top-0 h-0.5 w-full origin-left bg-brand motion-safe:animate-[step-progress_3.2s_linear_forwards]"
                />
              )}
              {phase === i && manual && (
                <span aria-hidden className="absolute left-0 top-0 h-0.5 w-full bg-brand" />
              )}
              <button
                type="button"
                aria-current={phase === i ? "step" : undefined}
                onClick={() => {
                  setManual(true);
                  setPhase(i);
                }}
                className={`flex h-full w-full cursor-pointer flex-col gap-4 p-6 text-left transition-opacity duration-300 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-inset sm:p-7 ${
                  phase === i ? "opacity-100" : "opacity-55 hover:opacity-80"
                }`}
              >
                <span
                  className={`inline-flex h-9 w-9 items-center justify-center rounded-sm font-mono text-sm transition-colors duration-300 ${
                    phase === i
                      ? "bg-primary text-brand"
                      : "bg-foreground/10 text-foreground/60"
                  }`}
                >
                  {step.n}
                </span>
                <h3 className="font-display text-xl tracking-tight text-foreground">
                  {step.title}
                </h3>
                <p className="text-[15px] leading-relaxed text-foreground/75">{step.body}</p>
              </button>
            </li>
          ))}
        </ol>
        {manual && (
          <button
            type="button"
            onClick={() => setManual(false)}
            className="mt-4 cursor-pointer font-mono text-[11px] uppercase tracking-[0.16em] text-muted-foreground underline decoration-dotted underline-offset-4 transition-colors hover:text-brand"
          >
            resume auto-play
          </button>
        )}
      </div>
    </section>
  );
}
