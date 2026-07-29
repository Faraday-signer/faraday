/**
 * Web flasher page — reserved home for FA-13 (browser-based ESP32-S3 flashing
 * over WebSerial / ESP Web Tools). For now it is a "coming soon" stub so the
 * route and nav slot exist; FA-13 replaces the placeholder block with the real
 * flashing flow (device connect, manifest, progress, unsupported-browser copy).
 */

import type { Metadata } from "next";
import Link from "next/link";

import { QrGlyph } from "@/components/landing/qr-glyph";
import { SectionLabel, Tag } from "@/components/landing/primitives";
import { SiteFooter } from "@/components/landing/site-footer";
import { SiteNav } from "@/components/landing/site-nav";

export const metadata: Metadata = {
  title: "Flash your device — Faraday",
  description:
    "Install Faraday firmware on an ESP32-S3 from your browser over WebSerial. Coming soon. Pi Zero devices use the SD-card image instead.",
};

const NOTES = [
  {
    k: "Target",
    v: "ESP32-S3 boards only",
  },
  {
    k: "Method",
    v: "WebSerial in Chrome or Edge",
  },
  {
    k: "Firmware",
    v: "Versioned, from GitHub releases",
  },
  {
    k: "Pi Zero",
    v: "Uses the SD-card image, not this page",
  },
];

export default function FlashPage() {
  return (
    <div className="min-h-svh bg-background text-foreground">
      <SiteNav />
      <main className="mx-auto max-w-4xl px-5 py-16 sm:px-8 sm:py-24">
        <SectionLabel index="—">Flash your device</SectionLabel>
        <h1 className="mt-5 font-display text-4xl leading-tight tracking-tight text-foreground sm:text-5xl">
          Install Faraday firmware<span className="text-brand pixel-shadow"> from your browser</span>
        </h1>
        <p className="mt-6 max-w-2xl text-base leading-relaxed text-foreground/75 sm:text-lg">
          Plug an ESP32-S3 into your computer, click once, and the browser
          writes a verified Faraday firmware image over WebSerial — no
          toolchain, no command line. The image is served versioned from our
          GitHub releases, so you always know exactly what you&apos;re
          installing.
        </p>

        <div className="mt-8">
          <Tag tone="brand">Coming soon</Tag>
        </div>

        <div className="mt-12 flex aspect-video w-full flex-col items-center justify-center gap-4 rounded-sm border border-border bg-muted/30">
          <QrGlyph className="h-16 w-16 text-border" />
          <p className="font-mono text-[11px] uppercase tracking-[0.18em] text-muted-foreground">
            Flasher landing here
          </p>
        </div>

        <dl className="mt-12 grid grid-cols-1 gap-px overflow-hidden rounded-sm border border-border bg-border sm:grid-cols-2">
          {NOTES.map((note) => (
            <div key={note.k} className="bg-background p-5 sm:p-6">
              <dt className="font-mono text-[11px] uppercase tracking-[0.16em] text-brand">
                {note.k}
              </dt>
              <dd className="mt-2 text-sm text-foreground/80">{note.v}</dd>
            </div>
          ))}
        </dl>

        <p className="mt-10 text-sm leading-relaxed text-foreground/60">
          Running a Raspberry Pi Zero? Browser flashing doesn&apos;t apply — the
          Pi boots a Buildroot SD-card image instead.{" "}
          <Link
            href="/#device"
            className="text-brand transition-colors hover:text-brand/80"
          >
            See the device build →
          </Link>
        </p>
      </main>
      <SiteFooter />
    </div>
  );
}
