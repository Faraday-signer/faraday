import { DeviceShowcase } from "./device-showcase";
import { MeasurementGrid } from "./measurement-grid";

const GITHUB_URL = "https://github.com/Faraday-signer/faraday";

export function Hero() {
  return (
    <section className="relative overflow-hidden border-b border-border">
      <MeasurementGrid className="opacity-70" />

      <div className="relative mx-auto grid max-w-6xl grid-cols-1 items-center gap-12 px-5 pb-16 pt-14 sm:px-8 sm:pb-24 sm:pt-20 lg:grid-cols-[1fr_auto] lg:gap-16">
        <div>
        <p className="font-mono text-[11px] uppercase tracking-[0.2em] text-muted-foreground sm:text-xs">
          Air-gapped
          <span className="mx-2 text-brand">·</span>
          Memory-resident keys
          <span className="mx-2 text-brand">·</span>
          Open-source
        </p>

        <h1 className="mt-6 max-w-3xl font-display font-bold tracking-tight text-foreground">
          <span className="block text-4xl leading-[1.05] sm:text-6xl">
            Sign Solana transactions
          </span>
          <span className="mt-1 block text-4xl leading-[1.05] text-brand pixel-shadow sm:text-6xl">
            without trusting
          </span>
          <span className="mt-1 block text-4xl leading-[1.05] sm:text-6xl">
            your computer
          </span>
        </h1>

        <p className="mt-7 max-w-xl text-base leading-relaxed text-foreground/80 sm:text-lg">
          Faraday is a pocket-sized hardware signer for Solana. Your keys are
          born on a device with no antennas and never leave it. Transactions
          cross the air gap as QR codes — no Wi-Fi, Bluetooth, NFC, USB, or
          Faraday servers anywhere in the loop.
        </p>

        <div className="mt-9 flex flex-col gap-3 sm:flex-row sm:items-center">
          <a
            href="#waitlist"
            className="inline-flex h-12 items-center justify-center rounded-sm bg-primary px-6 font-mono text-xs uppercase tracking-[0.14em] text-primary-foreground transition-colors hover:bg-primary/90"
          >
            Get early access →
          </a>
          <a
            href={GITHUB_URL}
            target="_blank"
            rel="noopener noreferrer"
            className="inline-flex h-12 items-center justify-center rounded-sm border border-foreground/20 px-6 font-mono text-xs uppercase tracking-[0.14em] text-foreground transition-colors hover:bg-foreground/5"
          >
            Read the code
          </a>
        </div>
        </div>

        <DeviceShowcase />
      </div>
    </section>
  );
}
