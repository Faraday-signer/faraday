import { QrGlyph } from "./qr-glyph";
import { SectionLabel } from "./primitives";

/**
 * Home for the 90-second Ika-approver demo clip. Until the video is dropped in,
 * this renders a framed instrument-style placeholder — not a broken player.
 * Replace the inner block with the embed when the clip is ready.
 */
export function DemoSection() {
  return (
    <section id="demo" className="border-b border-border bg-background">
      <div className="mx-auto max-w-6xl px-5 py-16 sm:px-8 sm:py-24">
        <SectionLabel index="05">Demo</SectionLabel>
        <h2 className="mt-5 max-w-2xl font-display text-3xl leading-tight tracking-tight text-foreground sm:text-4xl">
          Watch it clear-sign a multisig transaction
        </h2>
        <p className="mt-5 max-w-xl text-base leading-relaxed text-foreground/70">
          A 90-second run of Faraday acting as an approver on an Ika
          cross-chain multisig — the device decoding the proposal and showing
          exactly what it will authorize before a signature exists.
        </p>

        <div className="mt-10 flex aspect-video w-full flex-col items-center justify-center gap-4 rounded-sm border border-border bg-muted/30">
          <QrGlyph className="h-16 w-16 text-border" />
          <p className="font-mono text-[11px] uppercase tracking-[0.18em] text-muted-foreground">
            Clip landing soon
          </p>
        </div>
      </div>
    </section>
  );
}
