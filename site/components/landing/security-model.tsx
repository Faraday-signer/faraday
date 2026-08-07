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

/* Scene windows over the pinned scroll progress */
const SCENES = [
  { start: 0.02, end: 0.3, label: "01 · Birth" },
  { start: 0.3, end: 0.55, label: "02 · Hold" },
  { start: 0.55, end: 0.8, label: "03 · Read" },
  { start: 0.8, end: 1.0, label: "04 · Proof" },
];

const COPY = [
  {
    head: "Your key is born from light.",
    sub: "A photo's noise, mixed with the hardware RNG. Two sources, never one.",
  },
  {
    head: "It lives in memory. Nothing can reach it.",
    sub: "No flash storage. No radios on the board. Power off and it is gone.",
  },
  {
    head: "It signs nothing you haven't read.",
    sub: "Every transaction is decoded from raw bytes, offline, and shown before you approve.",
  },
  {
    head: "And you can prove every word of this.",
    sub: "The whole stack is open source. Build it and check the hash yourself.",
  },
];

const clamp01 = (v: number) => Math.min(1, Math.max(0, v));
/** Local progress of p inside [a, b]. */
const seg = (p: number, a: number, b: number) => clamp01((p - a) / (b - a));
/** Smoothstep ease. */
const ease = (t: number) => t * t * (3 - 2 * t);
/** Bell curve: 0→1→0 across t∈[0,1]. */
const bell = (t: number) => ease(1 - Math.abs(t * 2 - 1));

/** Live noise field that collapses into the key. `collapse` 0 = full field,
 * 1 = fully condensed into the center point. */
function NoiseField({ collapse }: { collapse: number }) {
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const collapseRef = useRef(collapse);
  collapseRef.current = collapse;

  useEffect(() => {
    const cnv = canvasRef.current;
    if (!cnv) return;
    const ctx = cnv.getContext("2d");
    if (!ctx) return;
    const { width: w, height: h } = cnv;
    const img = ctx.createImageData(w, h);
    const cx = w / 2;
    const cy = h / 2;
    const maxR = Math.hypot(cx, cy);
    let raf = 0;
    const draw = () => {
      const c = collapseRef.current;
      const r = maxR * (1 - c * 0.96);
      const d = img.data;
      for (let y = 0; y < h; y++) {
        for (let x = 0; x < w; x++) {
          const i = (y * w + x) * 4;
          const dist = Math.hypot(x - cx, y - cy);
          if (dist > r) {
            d[i + 3] = 0;
            continue;
          }
          const v = Math.random() * 255;
          // grain brightens as it condenses
          const boost = 0.55 + c * 0.45;
          d[i] = v * 0.55 * boost;
          d[i + 1] = v * 0.8 * boost;
          d[i + 2] = v * 0.85 * boost;
          d[i + 3] = 255;
        }
      }
      ctx.putImageData(img, 0, 0);
      raf = requestAnimationFrame(() => setTimeout(draw, 90) as unknown as number);
    };
    draw();
    return () => cancelAnimationFrame(raf);
  }, []);

  return (
    <canvas
      ref={canvasRef}
      width={160}
      height={120}
      className="h-[240px] w-[320px]"
      style={{ imageRendering: "pixelated" }}
    />
  );
}

/** The protagonist: a fixed glowing cyan point at stage center. */
function KeyDot({ visible }: { visible: number }) {
  return (
    <div
      aria-hidden
      className="absolute left-1/2 top-[42%] -translate-x-1/2 -translate-y-1/2 rounded-full"
      style={{
        width: 10,
        height: 10,
        background: FW.accent,
        opacity: visible,
        boxShadow: `0 0 12px 2px ${FW.accent}88, 0 0 42px 10px ${FW.accent}33`,
      }}
    />
  );
}

const SEED_WORDS = ["orbit", "canyon", "ridge", "velvet"];
const BYTES = "01 00 03 05 8a f3 42 9c d4 07 b1 e6 2f 55 aa 03 c8 91";

/** The cinematic stage: everything positioned around the fixed key point,
 * scrubbed by the pinned scroll progress. */
function Stage({ p }: { p: number }) {
  const t1 = seg(p, SCENES[0].start, SCENES[0].end);
  const t2 = seg(p, SCENES[1].start, SCENES[1].end);
  const t3 = seg(p, SCENES[2].start, SCENES[2].end);
  const t4 = seg(p, SCENES[3].start, SCENES[3].end);

  const sceneIdx = p < SCENES[1].start ? 0 : p < SCENES[2].start ? 1 : p < SCENES[3].start ? 2 : 3;
  // fade each scene's props in/out at its window edges
  const vis1 = t1 < 1 ? 1 : 1 - ease(seg(t2, 0, 0.25));
  const vis2 = bell(t2) > 0 && t2 > 0 ? Math.min(ease(seg(t2, 0, 0.2)), 1 - ease(seg(t3, 0, 0.2))) : 0;
  const vis3 = t3 > 0 ? Math.min(ease(seg(t3, 0, 0.2)), 1 - ease(seg(t4, 0, 0.2))) : 0;
  const vis4 = ease(seg(t4, 0, 0.25));

  const collapse = ease(seg(t1, 0.1, 0.62));
  const dotVisible = ease(seg(t1, 0.45, 0.62));
  const wordsIn = seg(t1, 0.66, 1);

  const hash = "3f9c…a1d2 = published release";
  const hashShown = Math.round(hash.length * ease(seg(t4, 0.15, 0.8)));

  return (
    <div
      className="relative h-full w-full overflow-hidden rounded-[12px]"
      style={{ background: FW.bg, border: `1px solid ${FW.border}` }}
    >
      {/* scene label */}
      <p
        className="absolute left-5 top-4 font-mono text-xs uppercase tracking-[0.2em] sm:left-7 sm:top-6"
        style={{ color: FW.dim }}
      >
        {SCENES[sceneIdx].label}
      </p>

      {/* SCENE 1 · the noise field collapsing into the key */}
      <div
        className="absolute left-1/2 top-[42%] -translate-x-1/2 -translate-y-1/2"
        style={{ opacity: vis1 }}
      >
        <NoiseField collapse={collapse} />
      </div>
      {/* seed words unfolding under the key */}
      <div
        className="absolute left-1/2 top-[54%] flex -translate-x-1/2 gap-2.5"
        style={{ opacity: vis1 }}
      >
        {SEED_WORDS.map((w, i) => {
          const wi = ease(seg(wordsIn, i * 0.18, i * 0.18 + 0.3));
          return (
            <span
              key={w}
              className="rounded-[6px] px-3 py-2 font-mono text-sm"
              style={{
                background: FW.panel,
                border: `1px solid ${FW.border}`,
                color: FW.text,
                opacity: wi,
                transform: `translateY(${(1 - wi) * 10}px)`,
              }}
            >
              <span style={{ color: FW.dim }}>{i + 1} </span>
              {w}
            </span>
          );
        })}
        <span
          className="self-center font-mono text-sm"
          style={{ color: FW.dim, opacity: ease(seg(wordsIn, 0.75, 1)) }}
        >
          … 24
        </span>
      </div>

      {/* SCENE 2 · the vault draws around the key; radios dissolve */}
      <div
        aria-hidden
        className="absolute left-1/2 top-[42%] rounded-[10px]"
        style={{
          width: 150,
          height: 96,
          border: `1.5px solid ${FW.accent}`,
          opacity: vis2,
          transform: `translate(-50%, -50%) scale(${0.85 + 0.15 * ease(t2)})`,
        }}
      />
      <p
        className="absolute left-1/2 top-[42%] -translate-x-1/2 translate-y-[30px] font-mono text-xs uppercase tracking-[0.2em]"
        style={{ color: FW.accent, opacity: vis2 }}
      >
        RAM
      </p>
      {[
        ["Wi-Fi", -1, -0.5],
        ["Bluetooth", 1, -0.65],
        ["NFC", -1, 0.55],
        ["Cellular", 1, 0.5],
      ].map(([label, dirX, dirY]) => {
        const lt = ease(t2);
        const travel = 1 - lt * 0.55;
        return (
          <span
            key={label as string}
            className="absolute left-1/2 top-[42%] whitespace-nowrap font-mono text-xs uppercase tracking-[0.14em]"
            style={{
              color: FW.muted,
              opacity: vis2 * bell(lt * 0.85),
              transform: `translate(calc(-50% + ${(dirX as number) * travel * 260}px), calc(-50% + ${(dirY as number) * travel * 150}px))`,
            }}
          >
            {label}
          </span>
        );
      })}

      {/* SCENE 3 · bytes pass; the review card assembles beside the key */}
      <p
        aria-hidden
        className="absolute left-1/2 top-[30%] w-[560px] overflow-hidden whitespace-nowrap text-center font-mono text-xs"
        style={{
          color: FW.dim,
          opacity: vis3 * 0.9,
          transform: `translateX(calc(-50% + ${(1 - t3) * 120 - 60}px))`,
        }}
      >
        {BYTES} {BYTES}
      </p>
      <div
        className="absolute left-1/2 top-[45%] w-64 rounded-[8px] p-4 text-left font-mono text-xs leading-relaxed"
        style={{
          background: FW.panel,
          border: `1px solid ${FW.border}`,
          opacity: vis3,
          transform: `translateX(-50%) translateY(${(1 - ease(t3)) * 16}px)`,
        }}
      >
        <p style={{ color: FW.accent }}>APPROVE SEND</p>
        <div className="my-2 h-px" style={{ background: FW.border }} />
        <div className="flex items-baseline justify-between">
          <span style={{ color: FW.muted }}>TO</span>
          <span style={{ color: FW.text }}>9WzD…tAWWM</span>
        </div>
        <div className="mt-1.5 flex items-baseline justify-between">
          <span style={{ color: FW.muted }}>AMOUNT</span>
          <span style={{ color: FW.text }}>
            1.5 <span style={{ color: FW.accent }}>SOL</span>
          </span>
        </div>
      </div>

      {/* SCENE 4 · the proof types itself out */}
      <p
        className="absolute left-1/2 top-[46%] -translate-x-1/2 whitespace-nowrap font-mono text-sm sm:text-base"
        style={{ color: FW.text, opacity: vis4 }}
      >
        <span style={{ color: FW.dim }}>$ </span>
        {hash.slice(0, hashShown)}
        <span
          className="ml-1.5"
          style={{
            color: FW.accent,
            opacity: hashShown === hash.length ? 1 : 0,
          }}
        >
          ✓
        </span>
      </p>

      {/* the key itself: never moves, never leaves */}
      <KeyDot visible={Math.max(dotVisible, sceneIdx > 0 ? 1 : 0)} />

      {/* copy block */}
      <div className="absolute inset-x-0 bottom-16 flex flex-col items-center gap-2 px-6 text-center">
        {COPY.map((c, i) => {
          const active = sceneIdx === i;
          return (
            <div
              key={i}
              className={`col-start-1 row-start-1 transition-opacity duration-300 ${
                active ? "opacity-100" : "hidden opacity-0"
              }`}
            >
              <p
                className="font-display text-xl tracking-tight sm:text-2xl"
                style={{ color: FW.text }}
              >
                {c.head}
              </p>
              <p className="mx-auto mt-2 max-w-md text-sm leading-relaxed" style={{ color: FW.muted }}>
                {c.sub}
              </p>
            </div>
          );
        })}
      </div>

      {/* progress hairline */}
      <div className="absolute inset-x-0 bottom-0 h-px" style={{ background: FW.border }}>
        <div
          className="h-px"
          style={{ background: FW.accent, width: `${p * 100}%` }}
        />
      </div>
    </div>
  );
}

/** Static fallback (mobile, reduced motion): each scene at its final state,
 * generously sized, stacked. */
function StaticStory() {
  return (
    <div className="flex flex-col gap-6">
      {COPY.map((c, i) => (
        <div
          key={i}
          className="flex min-h-56 flex-col items-center justify-center gap-5 rounded-[12px] p-6 text-center"
          style={{ background: FW.bg, border: `1px solid ${FW.border}` }}
        >
          <p className="font-mono text-xs uppercase tracking-[0.2em]" style={{ color: FW.dim }}>
            {SCENES[i].label}
          </p>
          {i === 0 && (
            <div className="flex flex-wrap items-center justify-center gap-2.5">
              {SEED_WORDS.map((w, j) => (
                <span
                  key={w}
                  className="rounded-[6px] px-3 py-2 font-mono text-sm"
                  style={{ background: FW.panel, border: `1px solid ${FW.border}`, color: FW.text }}
                >
                  <span style={{ color: FW.dim }}>{j + 1} </span>
                  {w}
                </span>
              ))}
            </div>
          )}
          {i === 1 && (
            <div
              className="rounded-[10px] px-8 py-4 font-mono text-sm uppercase tracking-[0.2em]"
              style={{ border: `1.5px solid ${FW.accent}`, color: FW.accent }}
            >
              ● RAM
            </div>
          )}
          {i === 2 && (
            <div
              className="w-60 rounded-[8px] p-4 text-left font-mono text-xs leading-relaxed"
              style={{ background: FW.panel, border: `1px solid ${FW.border}` }}
            >
              <p style={{ color: FW.accent }}>APPROVE SEND</p>
              <div className="my-2 h-px" style={{ background: FW.border }} />
              <div className="flex items-baseline justify-between">
                <span style={{ color: FW.muted }}>TO</span>
                <span style={{ color: FW.text }}>9WzD…tAWWM</span>
              </div>
              <div className="mt-1.5 flex items-baseline justify-between">
                <span style={{ color: FW.muted }}>AMOUNT</span>
                <span style={{ color: FW.text }}>
                  1.5 <span style={{ color: FW.accent }}>SOL</span>
                </span>
              </div>
            </div>
          )}
          {i === 3 && (
            <p className="font-mono text-sm" style={{ color: FW.text }}>
              <span style={{ color: FW.dim }}>$ </span>
              3f9c…a1d2 = published release <span style={{ color: FW.accent }}>✓</span>
            </p>
          )}
          <div>
            <p className="font-display text-lg tracking-tight sm:text-xl" style={{ color: FW.text }}>
              {c.head}
            </p>
            <p className="mx-auto mt-1.5 max-w-md text-sm leading-relaxed" style={{ color: FW.muted }}>
              {c.sub}
            </p>
          </div>
        </div>
      ))}
    </div>
  );
}

/**
 * The life of a key as a scroll-scrubbed film: one pinned stage where the key
 * is a fixed glowing point and the world transforms around it. Native scroll
 * drives every frame; mobile and reduced-motion get a static four-panel cut.
 */
export function SecurityModel() {
  const sectionRef = useRef<HTMLElement>(null);
  const [p, setP] = useState(0);
  const [cinematic, setCinematic] = useState<boolean | null>(null);

  useEffect(() => {
    const wide = window.matchMedia("(min-width: 1024px)");
    const reduced = window.matchMedia("(prefers-reduced-motion: reduce)");
    const update = () => setCinematic(wide.matches && !reduced.matches);
    update();
    wide.addEventListener("change", update);
    reduced.addEventListener("change", update);
    return () => {
      wide.removeEventListener("change", update);
      reduced.removeEventListener("change", update);
    };
  }, []);

  useEffect(() => {
    if (!cinematic) return;
    let raf = 0;
    const onScroll = () => {
      cancelAnimationFrame(raf);
      raf = requestAnimationFrame(() => {
        const el = sectionRef.current;
        if (!el) return;
        const rect = el.getBoundingClientRect();
        const scrollable = rect.height - window.innerHeight;
        if (scrollable <= 0) return;
        setP(clamp01(-rect.top / scrollable));
      });
    };
    onScroll();
    window.addEventListener("scroll", onScroll, { passive: true });
    return () => {
      cancelAnimationFrame(raf);
      window.removeEventListener("scroll", onScroll);
    };
  }, [cinematic]);

  return (
    <section
      ref={sectionRef}
      id="security"
      className={`border-b border-border ${cinematic ? "relative h-[400svh]" : ""}`}
    >
      {cinematic ? (
        <div className="sticky top-0 flex h-svh flex-col overflow-hidden pt-16">
          <div className="mx-auto w-full max-w-6xl px-5 pt-4 sm:px-8">
            <SectionLabel index="02">Security model</SectionLabel>
            <h2 className="mt-3 font-display text-2xl leading-tight tracking-tight text-foreground sm:text-3xl">
              The life of a key
            </h2>
          </div>
          <div className="mx-auto mb-6 mt-5 w-full max-w-6xl flex-1 px-5 sm:px-8">
            <Stage p={p} />
          </div>
        </div>
      ) : (
        <div className="mx-auto max-w-6xl px-5 py-16 sm:px-8 sm:py-24">
          <SectionLabel index="02">Security model</SectionLabel>
          <h2 className="mt-5 max-w-2xl font-display text-3xl leading-tight tracking-tight text-foreground sm:text-4xl">
            The life of a key
          </h2>
          <div className="mt-10">
            <StaticStory />
          </div>
        </div>
      )}
    </section>
  );
}
