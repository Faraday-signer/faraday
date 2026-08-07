"use client";

import { useEffect, useRef } from "react";

import { SectionLabel } from "./primitives";

/* Firmware/extension palette, same as the other dark exhibits */
const FW = {
  bg: "#001721",
  panel: "#002536",
  border: "#114358",
  text: "#E7E7E7",
  muted: "#8C9CA8",
  dim: "#5E7180",
  accent: "#1AF8FF",
  danger: "#FF6B6B",
};

/** Live sensor-noise grain, the raw material of a camera-born seed. Renders a
 * single static frame under prefers-reduced-motion. */
function NoiseFrame() {
  const canvasRef = useRef<HTMLCanvasElement>(null);
  useEffect(() => {
    const cnv = canvasRef.current;
    if (!cnv) return;
    const ctx = cnv.getContext("2d");
    if (!ctx) return;
    const { width: w, height: h } = cnv;
    const img = ctx.createImageData(w, h);
    const draw = () => {
      const d = img.data;
      for (let i = 0; i < d.length; i += 4) {
        const v = Math.random() * 255;
        d[i] = v * 0.55;
        d[i + 1] = v * 0.8;
        d[i + 2] = v * 0.85;
        d[i + 3] = 255;
      }
      ctx.putImageData(img, 0, 0);
    };
    draw();
    if (window.matchMedia("(prefers-reduced-motion: reduce)").matches) return;
    const id = setInterval(draw, 120);
    return () => clearInterval(id);
  }, []);
  return (
    <canvas
      ref={canvasRef}
      width={96}
      height={72}
      className="h-[81px] w-[108px] rounded-[4px]"
      style={{ imageRendering: "pixelated" }}
    />
  );
}

function PipeChip({ children }: { children: React.ReactNode }) {
  return (
    <span
      className="whitespace-nowrap rounded-[5px] px-2.5 py-1.5 font-mono text-[11px] uppercase tracking-[0.12em]"
      style={{
        border: `1px solid ${FW.border}`,
        color: FW.muted,
        background: FW.panel,
      }}
    >
      {children}
    </span>
  );
}

function PipeArrow() {
  return (
    <span aria-hidden className="font-mono" style={{ color: FW.dim }}>
      →
    </span>
  );
}

/** 01: photo noise → SHA-256 → ⊕ hardware RNG → words. */
function BornExhibit() {
  return (
    <div className="flex flex-col items-center gap-5">
      <div className="flex flex-wrap items-center justify-center gap-3">
        <div className="flex flex-col items-center gap-1.5">
          <NoiseFrame />
          <span className="font-mono text-[10px] uppercase tracking-[0.16em]" style={{ color: FW.dim }}>
            any photo
          </span>
        </div>
        <PipeArrow />
        <PipeChip>SHA-256</PipeChip>
        <PipeArrow />
        <PipeChip>⊕ hardware RNG</PipeChip>
      </div>
      <div className="flex flex-col items-center gap-1.5">
        <div className="flex flex-wrap justify-center gap-2">
          {["orbit", "canyon", "ridge", "velvet"].map((w, i) => (
            <span
              key={w}
              className="rounded-[5px] px-2.5 py-1.5 font-mono text-[13px]"
              style={{ background: FW.panel, border: `1px solid ${FW.border}`, color: FW.text }}
            >
              <span style={{ color: FW.dim }}>{i + 1} </span>
              {w}
            </span>
          ))}
          <span className="self-center font-mono text-[13px]" style={{ color: FW.dim }}>
            … 24
          </span>
        </div>
        <span className="font-mono text-[10px] uppercase tracking-[0.16em]" style={{ color: FW.dim }}>
          your seed · never weaker than either source
        </span>
      </div>
      <p className="font-mono text-[10px] uppercase tracking-[0.14em]" style={{ color: FW.muted }}>
        or flip coins and enter them raw · weak randomness = device refuses
      </p>
    </div>
  );
}

/** 02: RAM only, radios struck out. */
function HoldExhibit() {
  return (
    <div className="flex flex-col items-center gap-5">
      <div
        className="rounded-[6px] px-6 py-4 text-center"
        style={{ border: `1px solid ${FW.accent}`, background: FW.panel }}
      >
        <p className="font-mono text-base tracking-[0.14em]" style={{ color: FW.accent }}>
          KEYS · RAM ONLY
        </p>
        <p className="mt-1 font-mono text-[10px] uppercase tracking-[0.14em]" style={{ color: FW.muted }}>
          no flash · gone at power-off
        </p>
      </div>
      <div className="flex flex-wrap items-center justify-center gap-2.5">
        {["Wi-Fi", "Bluetooth", "NFC", "Cellular"].map((r) => (
          <span
            key={r}
            className="relative rounded-[5px] px-3 py-1.5 font-mono text-[11px] uppercase tracking-[0.12em]"
            style={{ border: `1px solid ${FW.border}`, color: FW.dim, background: FW.bg }}
          >
            {r}
            <span
              aria-hidden
              className="absolute left-1 right-1 top-1/2 h-px -rotate-6"
              style={{ background: FW.danger }}
            />
          </span>
        ))}
      </div>
      <p className="font-mono text-[10px] uppercase tracking-[0.14em]" style={{ color: FW.muted }}>
        not disabled · not present on the board
      </p>
    </div>
  );
}

/** 03: the on-device review, nothing signed blind. */
function ReadExhibit() {
  return (
    <div
      className="w-72 rounded-[6px] p-4 text-left font-mono text-xs leading-relaxed"
      style={{ background: FW.panel, border: `1px solid ${FW.border}` }}
    >
      <p style={{ color: FW.accent }}>APPROVE SEND</p>
      <div className="my-2 h-px" style={{ background: FW.border }} />
      <p style={{ color: FW.muted }}>
        FROM <span style={{ color: FW.accent }}>(you)</span>
      </p>
      <p className="truncate" style={{ color: FW.text }}>
        7xKXtg2CW87d97TXJSDpbD
      </p>
      <p className="mt-1.5" style={{ color: FW.muted }}>
        TO
      </p>
      <p className="truncate" style={{ color: FW.text }}>
        9WzDXwBbmkg8ZTbNMqUxvQ
      </p>
      <div className="mt-2 flex items-baseline justify-between">
        <span style={{ color: FW.muted }}>AMOUNT</span>
        <span style={{ color: FW.text }}>
          1.5 <span style={{ color: FW.accent }}>SOL</span>
        </span>
      </div>
    </div>
  );
}

/** 04: build it yourself, compare the hash. */
function ProveExhibit() {
  return (
    <div
      className="w-full max-w-sm rounded-[6px] p-4 text-left font-mono text-xs leading-loose"
      style={{ background: FW.panel, border: `1px solid ${FW.border}` }}
    >
      <p style={{ color: FW.muted }}>
        <span style={{ color: FW.dim }}>$</span> git clone faraday && just build
      </p>
      <p style={{ color: FW.muted }}>
        <span style={{ color: FW.dim }}>$</span> sha256 faraday.img
      </p>
      <p style={{ color: FW.text }}>
        3f9c…a1d2 <span style={{ color: FW.accent }}>= published release ✓</span>
      </p>
    </div>
  );
}

const CHAPTERS = [
  {
    n: "01",
    title: "You birth it",
    stakes: "Most keys trust one hidden randomness source.",
    proof: "create flow · open rust",
    Exhibit: BornExhibit,
  },
  {
    n: "02",
    title: "You hold it",
    stakes: "A stored key is a stealable key.",
    proof: "nothing on the board to reach",
    Exhibit: HoldExhibit,
  },
  {
    n: "03",
    title: "You read it",
    stakes: "Blind signatures are how devices betray their owners.",
    proof: "decoded offline · raw bytes",
    Exhibit: ReadExhibit,
  },
  {
    n: "04",
    title: "You prove it",
    stakes: "“Trust us” is not a security model.",
    proof: "sha256 = published release",
    Exhibit: ProveExhibit,
  },
];

/**
 * The life of a key, told statically: four chapters with identical anatomy
 * (verb title, one stakes line, one exhibit, one proof chip), threaded by a
 * single cyan line that IS the key — born in the noise, ending at the proof.
 */
export function SecurityModel() {
  return (
    <section id="security" className="border-b border-border">
      <div className="mx-auto max-w-6xl px-5 py-16 sm:px-8 sm:py-24">
        <SectionLabel index="02">Security model</SectionLabel>
        <h2 className="mt-5 max-w-2xl font-display text-3xl leading-tight tracking-tight text-foreground sm:text-4xl">
          The life of a key
        </h2>
        <p className="mt-4 max-w-2xl text-base leading-relaxed text-foreground/75">
          Four moments decide whether a key is ever really yours.
        </p>

        <div className="relative mt-12">
          {/* the thread: the key itself, born in chapter 01, proven in 04 */}
          <div
            aria-hidden
            className="absolute bottom-24 top-20 hidden w-px bg-brand/50 lg:left-[280px] lg:block"
          />

          <ol className="relative flex flex-col gap-12 lg:gap-16">
            {CHAPTERS.map(({ n, title, stakes, proof, Exhibit }) => (
              <li
                key={n}
                className="grid grid-cols-1 items-center gap-5 lg:grid-cols-[560px_1fr] lg:gap-12"
              >
                <div className="lg:order-last">
                  <h3 className="font-display text-xl tracking-tight text-foreground sm:text-2xl">
                    <span className="mr-3 font-mono text-sm text-foreground/40">{n}</span>
                    {title}
                  </h3>
                  <p className="mt-2 text-[15px] leading-relaxed text-foreground/70">{stakes}</p>
                  <p className="mt-3 inline-flex items-center gap-2 rounded-sm border border-brand/40 px-2.5 py-1 font-mono text-[10px] uppercase tracking-[0.14em] text-brand">
                    <span aria-hidden className="text-brand/60">▫</span>
                    {proof}
                  </p>
                </div>
                <div
                  className="relative flex min-h-52 items-center justify-center rounded-[10px] p-6 sm:p-8"
                  style={{ background: FW.bg, border: `1px solid ${FW.border}` }}
                >
                  <Exhibit />
                </div>
              </li>
            ))}
          </ol>

          {/* the thread ends; what remains */}
          <div className="mt-10 flex flex-col items-center gap-3 lg:items-start lg:pl-[276px]">
            <span aria-hidden className="hidden h-2 w-2 rounded-full bg-brand lg:-ml-[3.5px] lg:block" />
            <p className="max-w-md text-center font-display text-lg tracking-tight text-foreground lg:-ml-40 lg:text-left">
              What's left is a key that answers only to you.
            </p>
          </div>
        </div>
      </div>
    </section>
  );
}
