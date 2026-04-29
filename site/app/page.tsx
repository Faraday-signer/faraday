import { FinderPattern } from "@/components/landing/finder-pattern";
import { Logo } from "@/components/landing/logo";
import { MeasurementGrid } from "@/components/landing/measurement-grid";
import { WaitlistForm } from "@/components/landing/waitlist-form";

const SPEC_CELLS = [
  "No internet",
  "No flash storage",
  "No closed firmware",
  "QR-only I/O",
  "Keys in RAM only",
  "Anyone can audit",
];

export default function Home() {
  return (
    <div className="relative min-h-svh overflow-hidden bg-background text-foreground">
      <MeasurementGrid />

      <FinderPattern
        className="absolute right-6 top-6 h-12 w-12 text-neutral-900 sm:right-10 sm:top-10 sm:h-20 sm:w-20"
      />

      <div className="relative mx-auto flex min-h-svh max-w-3xl flex-col px-6 py-6 sm:px-10 sm:py-10">
        <header className="mb-6 sm:mb-10">
          <Logo />
        </header>

        <main className="flex-1 pb-8">
          <p className="mb-5 font-mono text-[11px] uppercase tracking-[0.18em] text-muted-foreground sm:text-xs">
            Air-gapped <span className="mx-1.5 text-foreground/30">·</span>
            Memory-resident keys <span className="mx-1.5 text-foreground/30">·</span>
            Open-source
          </p>

          <h1 className="font-display font-bold tracking-tight text-foreground">
            <span className="block text-3xl leading-tight sm:text-4xl">
              Sign Solana Transactions
            </span>
            <span
              className="block text-3xl leading-none text-brand sm:text-5xl"
              style={{ filter: "drop-shadow(1px 1px 0 var(--muted-foreground))" }}
            >
              without trusting
            </span>
            <span className="mt-1 block text-3xl leading-none sm:text-4xl">
              your computer
            </span>
          </h1>

          <p className="mt-4 max-w-xl text-sm leading-relaxed text-foreground/80 sm:text-base">
            Faraday is a pocket-sized hardware signer. Keys never touch a
            network. Transactions cross the air gap via QR. No Wi-Fi,
            Bluetooth, NFC, or USB.
          </p>

          <ul className="mt-6 grid grid-cols-2 gap-x-6 gap-y-2 font-mono text-[11px] uppercase tracking-[0.14em] text-foreground/70 sm:grid-cols-3 sm:text-xs">
            {SPEC_CELLS.map((cell) => (
              <li key={cell} className="flex items-center gap-2">
                <span aria-hidden className="text-foreground/40">—</span>
                {cell}
              </li>
            ))}
          </ul>

          <div className="mt-12 max-w-md sm:mt-16">
            <p className="mb-3 font-mono text-[11px] uppercase tracking-[0.14em] text-foreground/70 sm:text-xs">
              Early access to the first kits.
            </p>
            <WaitlistForm />
          </div>
        </main>

        <footer className="mt-auto flex items-center justify-between pt-4 font-mono text-[11px] uppercase tracking-[0.18em] text-muted-foreground">
          <p>
            Faraday <span className="mx-1.5 text-foreground/30">·</span> 2026
            <span className="mx-1.5 text-foreground/30">·</span> Offline by design
          </p>
          <a
            href="https://x.com/faradaysigner"
            target="_blank"
            rel="noopener noreferrer"
            aria-label="Faraday on X"
            className="text-foreground/70 transition-colors hover:text-foreground"
          >
            <svg
              viewBox="0 0 24 24"
              width="14"
              height="14"
              fill="currentColor"
              aria-hidden="true"
            >
              <path d="M18.244 2.25h3.308l-7.227 8.26 8.502 11.24H16.17l-5.214-6.817L4.99 21.75H1.68l7.73-8.835L1.254 2.25H8.08l4.713 6.231zm-1.161 17.52h1.833L7.084 4.126H5.117z" />
            </svg>
          </a>
        </footer>
      </div>
    </div>
  );
}
