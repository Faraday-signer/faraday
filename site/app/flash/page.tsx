import type { Metadata } from "next";
import Link from "next/link";

import { FlashInstaller } from "@/components/landing/flash-installer";
import { SectionLabel } from "@/components/landing/primitives";
import { SiteFooter } from "@/components/landing/site-footer";
import { SiteNav } from "@/components/landing/site-nav";

export const metadata: Metadata = {
  title: "Flash your device · Faraday",
  description:
    "Install Faraday firmware on an ESP32-S3 from your browser over WebSerial. Firmware is served versioned from GitHub releases. Pi Zero devices use the SD-card image instead.",
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

const STEPS = [
  "Plug your ESP32-S3 into a USB port.",
  "Click “Connect device” and pick the serial port from the browser dialog.",
  "If the board isn’t detected, hold BOOT and tap RESET to enter download mode.",
];

export default function FlashPage() {
  return (
    <div className="min-h-svh bg-background text-foreground">
      <SiteNav />
      <main className="mx-auto max-w-4xl px-5 py-16 sm:px-8 sm:py-24">
        <SectionLabel index="00">Flash your device</SectionLabel>
        <h1 className="mt-5 font-display text-4xl leading-tight tracking-tight text-foreground sm:text-5xl">
          Install Faraday firmware<span className="text-brand pixel-shadow"> from your browser</span>
        </h1>
        <p className="mt-6 max-w-2xl text-base leading-relaxed text-foreground/75 sm:text-lg">
          Plug an ESP32-S3 into your computer, click once, and the browser
          writes a verified Faraday firmware image over WebSerial. No
          toolchain, no command line. The image is served versioned from our
          GitHub releases, so you always know exactly what you&apos;re
          installing.
        </p>

        <ol className="mt-8 grid grid-cols-1 gap-3 sm:grid-cols-3">
          {STEPS.map((step, i) => (
            <li
              key={step}
              className="rounded-sm border border-border bg-muted/30 p-4 text-sm leading-relaxed text-foreground/75"
            >
              <span className="font-mono text-[11px] text-brand-ink">
                {String(i + 1).padStart(2, "0")}
              </span>
              <p className="mt-2">{step}</p>
            </li>
          ))}
        </ol>

        <FlashInstaller />

        <dl className="mt-12 grid grid-cols-1 gap-px overflow-hidden rounded-sm border border-border bg-border sm:grid-cols-2">
          {NOTES.map((note) => (
            <div key={note.k} className="bg-background p-5 sm:p-6">
              <dt className="font-mono text-[11px] uppercase tracking-[0.16em] text-brand-ink">
                {note.k}
              </dt>
              <dd className="mt-2 text-sm text-foreground/80">{note.v}</dd>
            </div>
          ))}
        </dl>

        <p className="mt-10 text-sm leading-relaxed text-foreground/60">
          Running a Raspberry Pi Zero? Browser flashing doesn&apos;t apply. The
          Pi boots a Buildroot SD-card image instead.{" "}
          <Link
            href="/#device"
            className="text-brand-ink transition-colors hover:text-brand-ink/80"
          >
            See the device build →
          </Link>
        </p>
      </main>
      <SiteFooter />
    </div>
  );
}
