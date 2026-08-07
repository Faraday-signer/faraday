"use client";

import { useState } from "react";
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

        <div className="mx-auto mt-10 flex w-full max-w-6xl flex-col items-center justify-center gap-6 px-5 sm:px-8 lg:flex-row lg:gap-4">
          <ExtensionPanel phase={phase} />

          {/* the air gap: a dimension-line track physically connecting the
              two machines, with a QR chip that crosses it on the QR legs */}
          <div
            aria-hidden
            className="relative hidden h-24 flex-1 lg:-ml-1 lg:-mr-16 lg:block"
          >
            {/* dashed track + end ticks, drawing-annotation style */}
            <div className="absolute inset-x-0 top-1/2 border-t border-dashed border-brand/50" />
            <div className="absolute left-0 top-1/2 h-4 w-px -translate-y-1/2 bg-brand/60" />
            <div className="absolute right-0 top-1/2 h-4 w-px -translate-y-1/2 bg-brand/60" />

            {/* label riding the line */}
            <div className="absolute inset-x-0 top-1/2 flex -translate-y-[calc(100%+8px)] flex-col items-center">
              <span className="pixel-shadow-sm whitespace-nowrap font-mono text-sm uppercase tracking-[0.24em] text-brand">
                Air gap
              </span>
            </div>
            <div className="absolute inset-x-0 top-1/2 flex translate-y-2 flex-col items-center">
              <span className="whitespace-nowrap font-mono text-[10px] uppercase tracking-[0.16em] text-muted-foreground">
                QR only · light
              </span>
            </div>

          </div>

          {/* stacked (mobile): short vertical track */}
          <div aria-hidden className="flex items-center gap-3 lg:hidden">
            <span className="h-10 border-l border-dashed border-brand/50" />
            <span className="pixel-shadow-sm font-mono text-xs uppercase tracking-[0.2em] text-brand">
              Air gap
            </span>
            <span className="h-10 border-l border-dashed border-brand/50" />
          </div>

          <div className="flex flex-col items-center">
            <Device3D
              device="esp32"
              holdYaw={POSES[phase]}
              screen={phase === 0 ? "menu" : phase === 3 ? "signed" : "review"}
            />
            <p className="mt-1 font-mono text-[10px] uppercase tracking-[0.2em] text-muted-foreground">
              {phase === 0 && "offline · idle"}
              {phase === 1 && "camera reads the qr"}
              {phase === 2 && "review on the screen"}
              {phase === 3 && "signed qr on the screen"}
            </p>
          </div>
        </div>

        {/* the controls: four unmistakably clickable step cards */}
        <div className="mx-auto mt-10 w-full max-w-6xl px-5 sm:px-8">
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
