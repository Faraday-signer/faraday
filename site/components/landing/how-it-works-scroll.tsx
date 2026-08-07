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
  { n: "01", title: "Build", short: "The extension builds the unsigned tx" },
  { n: "02", title: "Cross the gap", short: "QR out, camera in. Light only" },
  { n: "03", title: "Review & sign", short: "The device decodes and you approve" },
  { n: "04", title: "Return & submit", short: "Signed QR back, extension submits" },
];

/** Device pose per phase: idle three-quarter, spun so the back camera faces
 * the extension's QR, face-on for review, face-on showing the signed QR. */
const POSES = [-0.55, 2.35, 0.05, 0.05];

/* The real extension's theme tokens (extension/src/lib/theme.ts) */
const EXT = {
  bg: "#001721",
  panel: "#002536",
  border: "#0A2836",
  borderStrong: "#114358",
  text: "#E7E7E7",
  muted: "#8C9CA8",
  dim: "#5E7180",
  accent: "#1AF8FF",
  success: "#4ADE80",
};

function ExtField({
  label,
  value,
  trailing,
}: {
  label: string;
  value: string;
  trailing?: string;
}) {
  return (
    <div
      className="rounded-[10px] px-3 py-2"
      style={{ background: EXT.panel, border: `1px solid ${EXT.border}` }}
    >
      <p
        className="font-display text-[9px] uppercase tracking-[0.18em]"
        style={{ color: EXT.dim }}
      >
        {label}
      </p>
      <div className="flex items-baseline justify-between gap-2">
        <p className="truncate text-[12px]" style={{ color: EXT.text }}>
          {value}
        </p>
        {trailing && (
          <span
            className="font-display text-[9px] uppercase tracking-[0.14em]"
            style={{ color: EXT.accent }}
          >
            {trailing}
          </span>
        )}
      </div>
    </div>
  );
}

/** Faithful mini recreation of the real extension side panel (PanelShell:
 * 52px header with eyebrow + title, dark navy shell, panel-card fields,
 * branded white-surface QR frame, success green from the real theme). */
function ExtensionPanel({ phase }: { phase: number }) {
  const headers: [string, string][] = [
    ["Send", "Send SOL"],
    ["Sign transaction", "Sign on Faraday"],
    ["Sign transaction", "Sign on Faraday"],
    ["Send", "Broadcasted"],
  ];
  const [eyebrow, title] = headers[phase];
  return (
    <div
      className="w-60 overflow-hidden rounded-[14px] sm:w-64"
      style={{ background: EXT.bg, border: `1px solid ${EXT.borderStrong}` }}
    >
      {/* PanelShell header: back chevron · eyebrow + title · gear */}
      <div
        className="flex min-h-[44px] items-center justify-between px-3"
        style={{ borderBottom: `1px solid ${EXT.border}` }}
      >
        <span className="text-[13px]" style={{ color: EXT.dim }}>
          ‹
        </span>
        <div className="flex flex-col items-center leading-tight">
          <span
            className="font-display text-[8px] uppercase tracking-[0.22em]"
            style={{ color: EXT.dim }}
          >
            {eyebrow}
          </span>
          <span className="text-[12px] font-medium" style={{ color: EXT.text }}>
            {title}
          </span>
        </div>
        <span className="text-[11px]" style={{ color: EXT.dim }}>
          ⚙
        </span>
      </div>

      <div className="relative h-64">
        {/* P0: compose the send */}
        <div
          className={`absolute inset-0 flex flex-col gap-2 p-3 transition-opacity duration-500 ${
            phase === 0 ? "opacity-100" : "pointer-events-none opacity-0"
          }`}
        >
          <ExtField label="Token" value="SOL" trailing="12.4 SOL" />
          <ExtField label="Amount" value="1.5" trailing="Max" />
          <ExtField label="Recipient" value="9WzDXwBbmkg8ZTbNMqUxvQ…" />
          <div
            className="mt-auto rounded-[10px] py-2 text-center font-display text-[10px] uppercase tracking-[0.16em]"
            style={{ background: EXT.accent, color: EXT.bg }}
          >
            Review send
          </div>
        </div>

        {/* P1 + P2: the branded sign QR, then waiting for the signature */}
        <div
          className={`absolute inset-0 flex flex-col items-center gap-2 p-3 transition-opacity duration-500 ${
            phase === 1 || phase === 2 ? "opacity-100" : "pointer-events-none opacity-0"
          }`}
        >
          <div
            className="rounded-[14px] p-2"
            style={{ border: `1.5px solid ${EXT.accent}` }}
          >
            <p
              className="pb-1 text-center font-display text-[8px] uppercase tracking-[0.2em]"
              style={{ color: EXT.accent }}
            >
              Sign transaction
            </p>
            {/* real frames keep modules black-on-white for scan reliability */}
            <div className="rounded-[8px] bg-white p-2">
              <QrGlyph
                className={`h-32 w-32 text-black transition-opacity duration-500 ${
                  phase === 2 ? "opacity-30" : "opacity-100"
                }`}
              />
            </div>
          </div>
          <p
            className="text-center text-[10px] leading-snug"
            style={{ color: EXT.muted }}
          >
            {phase === 2 ? (
              <>
                <span
                  className="mr-1.5 inline-block h-1.5 w-1.5 rounded-full align-middle motion-safe:animate-pulse"
                  style={{ background: EXT.accent }}
                />
                Waiting for signature…
              </>
            ) : (
              "Hold your Faraday up to the QR."
            )}
          </p>
        </div>

        {/* P3: broadcasted */}
        <div
          className={`absolute inset-0 flex flex-col items-center justify-center gap-3 p-3 transition-opacity duration-500 ${
            phase === 3 ? "opacity-100" : "pointer-events-none opacity-0"
          }`}
        >
          <span
            className="flex h-12 w-12 items-center justify-center rounded-full text-xl"
            style={{ background: "rgba(74,222,128,0.14)", color: EXT.success }}
          >
            ✓
          </span>
          <p className="text-[12px]" style={{ color: EXT.text }}>
            Signature scanned back
          </p>
          <span
            className="rounded-full px-2.5 py-0.5 font-display text-[9px] uppercase tracking-[0.16em]"
            style={{
              border: `1px solid rgba(74,222,128,0.4)`,
              color: EXT.success,
            }}
          >
            Broadcast · finalized
          </span>
        </div>
      </div>
    </div>
  );
}

/**
 * The scroll story: a pinned panel three viewports deep. Native scroll maps to
 * four phases acting out the real round trip between the extension and the 3D
 * device (which turns its camera to the QR, reviews, then shows the signed QR
 * on its actual screen). No wheel hijacking, and anchor jumps pass through.
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
      id="how-it-works"
      className="relative h-[300svh] border-b border-border bg-background"
    >
      <div className="sticky top-0 flex h-svh flex-col overflow-hidden pt-16">
        <div className="mx-auto w-full max-w-6xl px-5 pt-6 sm:px-8">
          <SectionLabel index="01">How it works</SectionLabel>
          <h2 className="mt-3 max-w-2xl font-display text-2xl leading-tight tracking-tight text-foreground sm:text-4xl">
            One round trip across the air gap
          </h2>
        </div>

        <div className="mx-auto flex w-full max-w-6xl flex-1 flex-col items-center justify-center gap-6 px-5 sm:px-8 lg:flex-row lg:gap-12">
          <ExtensionPanel phase={phase} />

          {/* the gap between the two: direction arrow per leg */}
          <div
            aria-hidden
            className="flex h-10 flex-col items-center justify-center lg:h-auto lg:w-16"
          >
            <span
              className={`font-mono text-xl text-brand transition-opacity duration-500 ${
                phase === 1 || phase === 3 ? "opacity-100" : "opacity-20"
              }`}
            >
              <span className="hidden lg:inline">{phase === 3 ? "←" : "→"}</span>
              <span className="lg:hidden">{phase === 3 ? "↑" : "↓"}</span>
            </span>
            <span className="mt-1 hidden whitespace-nowrap font-mono text-[9px] uppercase tracking-[0.18em] text-muted-foreground lg:block">
              air gap
            </span>
          </div>

          <div className="flex flex-col items-center">
            <Device3D
              device="esp32"
              holdYaw={POSES[phase]}
              screen={phase === 3 ? "signed" : "review"}
            />
            <p className="mt-1 font-mono text-[10px] uppercase tracking-[0.2em] text-muted-foreground">
              {phase === 0 && "offline · idle"}
              {phase === 1 && "camera reads the qr"}
              {phase === 2 && "review on the screen"}
              {phase === 3 && "signed qr on the screen"}
            </p>
          </div>
        </div>

        {/* compact step rail + progress */}
        <div className="mx-auto mb-6 w-full max-w-6xl px-5 sm:px-8">
          <ol className="grid grid-cols-2 gap-x-6 gap-y-2 sm:grid-cols-4">
            {STEPS.map((step, i) => (
              <li key={step.n} aria-current={phase === i ? "step" : undefined}>
                <div
                  className={`h-0.5 w-full origin-left rounded-full transition-[background-color,transform] duration-500 ${
                    phase >= i ? "scale-x-100 bg-brand" : "scale-x-100 bg-border"
                  }`}
                />
                <p
                  className={`mt-2 font-mono text-[10px] uppercase tracking-[0.14em] transition-colors duration-300 ${
                    phase === i ? "text-foreground" : "text-foreground/40"
                  }`}
                >
                  {step.n} · {step.title}
                </p>
                <p
                  className={`mt-0.5 hidden text-xs leading-relaxed text-foreground/60 transition-opacity duration-300 sm:block ${
                    phase === i ? "opacity-100" : "opacity-0"
                  }`}
                >
                  {step.short}
                </p>
              </li>
            ))}
          </ol>
        </div>
      </div>
    </section>
  );
}
