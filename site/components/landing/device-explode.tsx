"use client";

import { useEffect, useRef, useState } from "react";
import { QrGlyph } from "./qr-glyph";

export type DeviceMode = "build" | "buy";

/** Scroll progress of a tall section: 0 when its top hits the viewport top, 1 when its bottom leaves. */
export function useScrollProgress(ref: React.RefObject<HTMLElement | null>) {
  const [progress, setProgress] = useState(0);
  const [reduced, setReduced] = useState(false);

  useEffect(() => {
    const mq = window.matchMedia("(prefers-reduced-motion: reduce)");
    setReduced(mq.matches);
    const onChange = (e: MediaQueryListEvent) => setReduced(e.matches);
    mq.addEventListener("change", onChange);
    return () => mq.removeEventListener("change", onChange);
  }, []);

  useEffect(() => {
    let raf = 0;
    const update = () => {
      const el = ref.current;
      if (!el) return;
      const rect = el.getBoundingClientRect();
      const total = rect.height - window.innerHeight;
      const p = total > 0 ? Math.min(1, Math.max(0, -rect.top / total)) : 1;
      setProgress(p);
    };
    const onScroll = () => {
      cancelAnimationFrame(raf);
      raf = requestAnimationFrame(update);
    };
    update();
    window.addEventListener("scroll", onScroll, { passive: true });
    window.addEventListener("resize", onScroll);
    return () => {
      cancelAnimationFrame(raf);
      window.removeEventListener("scroll", onScroll);
      window.removeEventListener("resize", onScroll);
    };
  }, [ref]);

  return { progress: reduced ? 1 : progress, reduced };
}

/* ——— Line-art placeholder plates (v0 of the final cyan technical drawings) ——— */

const PLATE_BASE =
  "absolute left-1/2 top-1/2 w-56 -translate-x-1/2 -translate-y-1/2 rounded-sm border border-brand/50 bg-background/85 shadow-[0_0_24px_-12px_var(--brand)] transition-none sm:w-64";

function CaseFront() {
  return (
    <div className={`${PLATE_BASE} h-36 sm:h-40`}>
      <div className="absolute inset-4 rounded-[1px] border border-brand/30" />
      <PlateTag>case · front</PlateTag>
    </div>
  );
}

function DisplayPlate({ lit = 0 }: { lit?: number }) {
  return (
    <div className={`${PLATE_BASE} h-36 sm:h-40`}>
      <div className="absolute left-1/2 top-1/2 h-24 w-24 -translate-x-1/2 -translate-y-1/2 border border-brand/40">
        <div
          className="h-full w-full p-2 transition-opacity duration-300"
          style={{ opacity: lit }}
          aria-hidden
        >
          <QrGlyph className="h-full w-full text-brand" />
        </div>
      </div>
      <PlateTag>display · st7789 · 240×240</PlateTag>
    </div>
  );
}

function BoardPlate({ label = "pi zero 1.3 · no radio footprint" }: { label?: string }) {
  return (
    <div className={`${PLATE_BASE} h-36 sm:h-40`}>
      <div className="absolute left-4 top-4 flex gap-[3px]">
        {Array.from({ length: 20 }, (_, i) => (
          <span key={i} className="h-[3px] w-[3px] bg-brand/60" />
        ))}
      </div>
      <div className="absolute bottom-4 right-4 h-6 w-10 border border-brand/40" />
      <PlateTag>{label}</PlateTag>
    </div>
  );
}

function CameraPlate() {
  return (
    <div className={`${PLATE_BASE} h-36 sm:h-40`}>
      <div className="absolute left-1/2 top-1/2 h-10 w-10 -translate-x-1/2 -translate-y-1/2 rounded-full border border-brand/60" />
      <div className="absolute left-1/2 top-1/2 h-4 w-4 -translate-x-1/2 -translate-y-1/2 rounded-full border border-brand/40" />
      <PlateTag>camera · ov5647</PlateTag>
    </div>
  );
}

function CaseBack() {
  return (
    <div className={`${PLATE_BASE} h-36 sm:h-40`}>
      <PlateTag>case · back</PlateTag>
    </div>
  );
}

function PlateTag({ children }: { children: React.ReactNode }) {
  return (
    <span className="absolute bottom-2 left-3 font-mono text-[9px] uppercase tracking-[0.18em] text-muted-foreground">
      {children}
    </span>
  );
}

/* ——— The pinned stage ——— */

const PI_PLATES = [CaseBack, CameraPlate, BoardPlate, DisplayPlate, CaseFront];

export function DeviceStage({ mode, progress }: { mode: DeviceMode; progress: number }) {
  // 0 → fully exploded · 0.7 → assembled · 1.0 → screen lit
  const assemble = Math.min(1, progress / 0.7);
  const explode = 1 - assemble;
  const lit = Math.max(0, (progress - 0.75) / 0.25);

  if (mode === "buy") {
    return (
      <div className="relative flex h-full min-h-[24rem] items-center justify-center">
        <div className="absolute left-1/2 top-8 -translate-x-1/2 rounded-sm border border-border px-3 py-1.5 font-mono text-[10px] uppercase tracking-[0.2em] text-muted-foreground">
          browser · web flasher
        </div>
        {/* firmware beam */}
        <div
          className="absolute left-1/2 top-16 w-px -translate-x-1/2 border-l border-dashed border-brand/70"
          style={{ height: `${Math.round(assemble * 120)}px` }}
          aria-hidden
        />
        <div className="relative mt-24">
          <div className="relative h-44 w-64 rounded-sm border border-brand/50 bg-background/85 shadow-[0_0_24px_-12px_var(--brand)] sm:h-48 sm:w-72">
            <div className="absolute left-1/2 top-1/2 h-28 w-28 -translate-x-1/2 -translate-y-1/2 border border-brand/40">
              <div className="h-full w-full p-2" style={{ opacity: lit }} aria-hidden>
                <QrGlyph className="h-full w-full text-brand" />
              </div>
            </div>
            <PlateTag>esp32-s3 · one board · touch lcd · camera</PlateTag>
          </div>
          <p className="mt-4 text-center font-mono text-[10px] uppercase tracking-[0.2em] text-brand">
            {progress >= 0.98
              ? "flashed · ready to sign"
              : `flashing firmware · ${Math.round(assemble * 100)}%`}
          </p>
        </div>
      </div>
    );
  }

  return (
    <div className="relative h-full min-h-[24rem] [perspective:1400px]">
      <div className="absolute inset-0 [transform-style:preserve-3d] [transform:rotateX(18deg)]">
        {PI_PLATES.map((Plate, i) => {
          const offset = (i - (PI_PLATES.length - 1) / 2) * (30 + explode * 96);
          return (
            <div
              key={i}
              className="absolute inset-0"
              style={{ transform: `translateY(${-offset}px)`, zIndex: i }}
            >
              {Plate === DisplayPlate ? <DisplayPlate lit={lit} /> : <Plate />}
            </div>
          );
        })}
      </div>
      <p
        className="absolute bottom-0 left-1/2 -translate-x-1/2 whitespace-nowrap font-mono text-[10px] uppercase tracking-[0.2em] text-brand transition-opacity duration-300"
        style={{ opacity: lit }}
      >
        assembled · no radios · ready to sign
      </p>
      <p
        className="absolute bottom-0 left-1/2 -translate-x-1/2 whitespace-nowrap font-mono text-[10px] uppercase tracking-[0.2em] text-muted-foreground transition-opacity duration-300"
        style={{ opacity: Math.max(0, 1 - assemble * 1.6) }}
      >
        five parts · ~$35 · scroll to assemble
      </p>
    </div>
  );
}
