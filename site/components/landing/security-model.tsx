"use client";

import { useEffect, useRef, useState } from "react";

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

const CHAPTERS = [
  {
    n: "01",
    title: "Born",
    short: "From randomness you can see",
    body: "A seed starts as light. Point the camera at anything: every frame is hashed and mixed with the hardware RNG, so the seed is never weaker than either source. Prefer your own luck? Flip coins and enter them raw. If good randomness is unavailable, the device refuses to continue.",
  },
  {
    n: "02",
    title: "Lives",
    short: "In RAM, behind no radios",
    body: "Keys live in RAM and nowhere else. There is no flash storage to extract and no radio to reach: the board has no Wi-Fi, Bluetooth, NFC, or cellular silicon. Power off and the keys are gone.",
  },
  {
    n: "03",
    title: "Speaks",
    short: "Only after you read the bytes",
    body: "The key never signs blind. The device decodes each transaction from its raw bytes, offline, and shows what it means in plain terms before you approve. Anything it cannot decode is flagged, never guessed.",
  },
  {
    n: "04",
    title: "Verifiable",
    short: "By you, not by faith",
    body: "The whole stack is open source: firmware, OS image recipe, extension, mobile app. Read it, build it yourself, and check that the binary you run is the one you compiled. No vendor firmware to trust.",
  },
];

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
      width={72}
      height={54}
      className="h-[54px] w-[72px] rounded-[3px]"
      style={{ imageRendering: "pixelated" }}
    />
  );
}

function PipeChip({ children, accent }: { children: React.ReactNode; accent?: boolean }) {
  return (
    <span
      className="whitespace-nowrap rounded-[4px] px-2 py-1 font-mono text-[10px] uppercase tracking-[0.12em]"
      style={{
        border: `1px solid ${accent ? FW.accent : FW.border}`,
        color: accent ? FW.accent : FW.muted,
        background: FW.panel,
      }}
    >
      {children}
    </span>
  );
}

function PipeArrow() {
  return (
    <span aria-hidden className="font-mono text-sm" style={{ color: FW.dim }}>
      →
    </span>
  );
}

/** 01 · Born: photo noise → SHA-256 → ⊕ hardware RNG → words. */
function BornExhibit() {
  return (
    <div className="flex flex-col items-center gap-4">
      <div className="flex flex-wrap items-center justify-center gap-2.5">
        <div className="flex flex-col items-center gap-1">
          <NoiseFrame />
          <span className="font-mono text-[9px] uppercase tracking-[0.16em]" style={{ color: FW.dim }}>
            any photo
          </span>
        </div>
        <PipeArrow />
        <PipeChip>SHA-256</PipeChip>
        <PipeArrow />
        <PipeChip>⊕ hardware RNG</PipeChip>
        <PipeArrow />
        <div className="flex flex-col items-center gap-1">
          <div className="flex gap-1.5">
            {["orbit", "canyon", "ridge"].map((w, i) => (
              <span
                key={w}
                className="rounded-[4px] px-2 py-1 font-mono text-[11px]"
                style={{ background: FW.panel, border: `1px solid ${FW.border}`, color: FW.text }}
              >
                <span style={{ color: FW.dim }}>{i + 1} </span>
                {w}
              </span>
            ))}
            <span className="self-center font-mono text-[11px]" style={{ color: FW.dim }}>
              … 24
            </span>
          </div>
          <span className="font-mono text-[9px] uppercase tracking-[0.16em]" style={{ color: FW.dim }}>
            your seed
          </span>
        </div>
      </div>
      <p className="font-mono text-[10px] uppercase tracking-[0.14em]" style={{ color: FW.muted }}>
        two sources, never one · weak randomness = device refuses
      </p>
    </div>
  );
}

/** 02 · Lives: RAM only, radios struck out. */
function LivesExhibit() {
  return (
    <div className="flex flex-col items-center gap-4">
      <div
        className="rounded-[6px] px-5 py-3 text-center"
        style={{ border: `1px solid ${FW.accent}`, background: FW.panel }}
      >
        <p className="font-mono text-sm tracking-[0.14em]" style={{ color: FW.accent }}>
          KEYS · RAM ONLY
        </p>
        <p className="mt-0.5 font-mono text-[10px] uppercase tracking-[0.14em]" style={{ color: FW.muted }}>
          no flash · gone at power-off
        </p>
      </div>
      <div className="flex flex-wrap items-center justify-center gap-2">
        {["Wi-Fi", "Bluetooth", "NFC", "Cellular"].map((r) => (
          <span
            key={r}
            className="relative rounded-[4px] px-2.5 py-1 font-mono text-[10px] uppercase tracking-[0.12em]"
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

/** 03 · Speaks: the on-device review, nothing signed blind. */
function SpeaksExhibit() {
  return (
    <div className="flex flex-col items-center gap-3">
      <div
        className="w-64 rounded-[6px] p-3 text-left font-mono text-[11px] leading-relaxed"
        style={{ background: FW.panel, border: `1px solid ${FW.border}` }}
      >
        <p style={{ color: FW.accent }}>APPROVE SEND</p>
        <div className="my-1.5 h-px" style={{ background: FW.border }} />
        <p style={{ color: FW.muted }}>
          FROM <span style={{ color: FW.accent }}>(you)</span>
        </p>
        <p className="truncate" style={{ color: FW.text }}>
          7xKXtg2CW87d97TXJSDpbD
        </p>
        <p className="mt-1" style={{ color: FW.muted }}>
          TO
        </p>
        <p className="truncate" style={{ color: FW.text }}>
          9WzDXwBbmkg8ZTbNMqUxvQ
        </p>
        <div className="mt-1.5 flex items-baseline justify-between">
          <span style={{ color: FW.muted }}>AMOUNT</span>
          <span style={{ color: FW.text }}>
            1.5 <span style={{ color: FW.accent }}>SOL</span>
          </span>
        </div>
      </div>
      <p className="font-mono text-[10px] uppercase tracking-[0.14em]" style={{ color: FW.muted }}>
        decoded from raw bytes, offline · unknown programs are flagged
      </p>
    </div>
  );
}

/** 04 · Verifiable: build it yourself, compare the hash. */
function VerifiableExhibit() {
  return (
    <div className="flex flex-col items-center gap-3">
      <div
        className="w-full max-w-sm rounded-[6px] p-3.5 text-left font-mono text-[11px] leading-loose"
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
      <p className="font-mono text-[10px] uppercase tracking-[0.14em]" style={{ color: FW.muted }}>
        firmware · os image · extension · mobile · all open
      </p>
    </div>
  );
}

const EXHIBITS = [BornExhibit, LivesExhibit, SpeaksExhibit, VerifiableExhibit];

export function SecurityModel() {
  const [chapter, setChapter] = useState(0);
  const Exhibit = EXHIBITS[chapter];

  return (
    <section id="security" className="border-b border-border">
      <div className="mx-auto max-w-6xl px-5 py-16 sm:px-8 sm:py-24">
        <SectionLabel index="02">Security model</SectionLabel>
        <h2 className="mt-5 max-w-2xl font-display text-3xl leading-tight tracking-tight text-foreground sm:text-4xl">
          The life of a key
        </h2>
        <p className="mt-4 max-w-2xl text-base leading-relaxed text-foreground/75">
          Security is not a feature you add later. It is decided the moment a
          key is born, and at every moment after. Faraday is built around that
          whole life.
        </p>

        {/* the exhibit for the selected chapter */}
        <div
          className="mt-10 flex min-h-64 items-center justify-center rounded-[10px] p-6 sm:p-8"
          style={{ background: FW.bg, border: `1px solid ${FW.border}` }}
        >
          <Exhibit />
        </div>

        {/* the timeline: stations on a dimension line */}
        <div className="relative mt-8">
          <div
            aria-hidden
            className="absolute left-0 right-0 top-[9px] hidden border-t border-dashed border-brand/40 sm:block"
          />
          <ol className="grid grid-cols-2 gap-x-6 gap-y-6 sm:grid-cols-4">
            {CHAPTERS.map((c, i) => (
              <li key={c.n} aria-current={chapter === i ? "step" : undefined}>
                <button
                  type="button"
                  onClick={() => setChapter(i)}
                  aria-pressed={chapter === i}
                  className="group w-full cursor-pointer text-left focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2 focus-visible:ring-offset-background"
                >
                  {/* station dot on the line */}
                  <span className="relative block h-[19px]">
                    <span
                      className={`absolute left-0 top-1/2 h-3 w-3 -translate-y-1/2 rounded-full border transition-colors duration-200 ${
                        chapter === i
                          ? "border-brand bg-brand"
                          : "border-brand/50 bg-background group-hover:bg-brand/30"
                      }`}
                    />
                  </span>
                  <span
                    className={`mt-2 block font-display text-lg tracking-tight transition-colors duration-200 sm:text-xl ${
                      chapter === i ? "text-foreground" : "text-foreground/50 group-hover:text-foreground/75"
                    }`}
                  >
                    <span className="mr-2 font-mono text-xs text-foreground/40">{c.n}</span>
                    {c.title}
                  </span>
                  <span
                    className={`mt-0.5 block font-mono text-[10px] uppercase tracking-[0.14em] transition-colors duration-200 ${
                      chapter === i ? "text-brand" : "text-foreground/40"
                    }`}
                  >
                    {c.short}
                  </span>
                </button>
              </li>
            ))}
          </ol>
        </div>

        {/* the selected chapter's story */}
        <p className="mt-6 max-w-3xl text-[15px] leading-relaxed text-foreground/75">
          {CHAPTERS[chapter].body}
        </p>
      </div>
    </section>
  );
}
