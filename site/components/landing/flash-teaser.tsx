import Link from "next/link";

import { SectionLabel } from "./primitives";

/**
 * Landing teaser for the browser flasher (FA-13). Links to the live /flash
 * page, which flashes ESP32-S3 firmware over WebSerial.
 */
export function FlashTeaser() {
  return (
    <section id="flash" className="border-b border-border">
      <div className="mx-auto max-w-6xl px-5 py-16 sm:px-8 sm:py-24">
        <div className="flex flex-col items-start justify-between gap-8 rounded-sm border border-border bg-muted/30 p-8 sm:p-12 lg:flex-row lg:items-center">
          <div className="max-w-2xl">
            <SectionLabel index="06">Flash your device</SectionLabel>
            <h2 className="mt-5 font-display text-2xl leading-tight tracking-tight text-foreground sm:text-3xl">
              Install Faraday firmware from your browser
            </h2>
            <p className="mt-4 text-base leading-relaxed text-foreground/70">
              A web flasher for the ESP32-S3 build. Plug the board in, click
              once, and the browser writes the firmware over WebSerial. No
              toolchain, no command line. Pi Zero devices keep using the
              SD-card image.
            </p>
          </div>
          <div className="flex flex-col items-start gap-4">
            <Link
              href="/flash"
              className="inline-flex h-11 items-center justify-center rounded-sm border border-foreground/20 px-5 font-mono text-xs uppercase tracking-[0.14em] text-foreground transition-colors hover:bg-foreground/5"
            >
              Flash your device →
            </Link>
          </div>
        </div>
      </div>
    </section>
  );
}
