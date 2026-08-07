import { AirGapVisual } from "./air-gap-visual";
import { SectionLabel } from "./primitives";

const STEPS = [
  {
    n: "01",
    title: "Build",
    body: "Your wallet or a dapp builds an unsigned transaction in the browser, exactly as it would with any wallet.",
  },
  {
    n: "02",
    title: "Cross the gap",
    body: "The Faraday extension renders the transaction as a QR code. The device's camera reads it. Nothing electrical connects the two — the only channel is light.",
  },
  {
    n: "03",
    title: "Review on-device",
    body: "Faraday decodes the raw bytes offline — no RPC, no lookups — and shows exactly what you're about to sign in plain terms. You approve with the buttons.",
  },
  {
    n: "04",
    title: "Sign & return",
    body: "The device signs in RAM and displays the signature as a QR. The browser scans it back and submits to Solana. The private key never crossed.",
  },
];

export function HowItWorks() {
  return (
    <section id="how-it-works" className="border-b border-border bg-background">
      <div className="mx-auto max-w-6xl px-5 py-16 sm:px-8 sm:py-24">
        <SectionLabel index="01">How it works</SectionLabel>
        <h2 className="mt-5 max-w-2xl font-display text-3xl leading-tight tracking-tight text-foreground sm:text-4xl">
          One round trip across the air gap
        </h2>

        <AirGapVisual className="mt-12" />

        <ol className="mt-12 grid grid-cols-1 gap-px overflow-hidden rounded-sm border border-border bg-border sm:grid-cols-2 lg:grid-cols-4">
          {STEPS.map((step) => (
            <li
              key={step.n}
              className="flex flex-col gap-4 bg-muted p-6 sm:p-7"
            >
              <span className="inline-flex h-9 w-9 items-center justify-center rounded-sm bg-primary font-mono text-sm text-brand">
                {step.n}
              </span>
              <h3 className="font-display text-xl tracking-tight text-foreground">
                {step.title}
              </h3>
              <p className="text-[15px] leading-relaxed text-foreground/75">
                {step.body}
              </p>
            </li>
          ))}
        </ol>
      </div>
    </section>
  );
}
