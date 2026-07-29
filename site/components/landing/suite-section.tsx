import { SectionLabel, Tag } from "./primitives";

const GITHUB_URL = "https://github.com/Faraday-signer/faraday";

const PARTS = [
  {
    title: "The signer",
    kind: "hardware/",
    body: "Rust firmware for the air-gapped device — the wallet, the transaction parser, and the QR pipeline. Runs on the Pi and, unchanged, as a desktop simulator.",
    status: <Tag tone="brand">Open source</Tag>,
    action: {
      label: "Read the firmware →",
      href: GITHUB_URL,
      external: true,
    },
  },
  {
    title: "The browser extension",
    kind: "extension/",
    body: "A Wallet Standard companion for Chrome. Dapps connect to it like any wallet; every signing request is relayed to Faraday over QR. Private keys never touch the browser.",
    status: <Tag>Heading to the Chrome Web Store</Tag>,
    // Reserved slot for the store listing (FA-08); swap href in when live.
    action: {
      label: "Chrome Web Store · soon",
      href: null,
      external: false,
    },
  },
  {
    title: "The mobile app",
    kind: "mobile/",
    body: "A watch-only wallet for the Solana Seeker: track balances and send with the same QR relay to your Faraday device. Work in progress toward full extension parity.",
    status: <Tag>Work in progress</Tag>,
    action: {
      label: "Follow along →",
      href: GITHUB_URL,
      external: true,
    },
  },
];

export function SuiteSection() {
  return (
    <section id="suite" className="border-b border-border">
      <div className="mx-auto max-w-6xl px-5 py-16 sm:px-8 sm:py-24">
        <SectionLabel index="04">The suite</SectionLabel>
        <h2 className="mt-5 max-w-2xl font-display text-3xl leading-tight tracking-tight text-foreground sm:text-4xl">
          One device, a companion for every surface
        </h2>

        <div className="mt-12 grid grid-cols-1 gap-px overflow-hidden rounded-sm border border-border bg-border md:grid-cols-3">
          {PARTS.map((part) => (
            <div
              key={part.title}
              className="flex flex-col bg-background p-6 sm:p-8"
            >
              <p className="font-mono text-[11px] uppercase tracking-[0.16em] text-brand">
                {part.kind}
              </p>
              <h3 className="mt-3 font-display text-xl tracking-tight text-foreground">
                {part.title}
              </h3>
              <p className="mt-3 flex-1 text-sm leading-relaxed text-foreground/70">
                {part.body}
              </p>
              <div className="mt-6 flex items-center justify-between gap-3">
                {part.status}
                {part.action.href ? (
                  <a
                    href={part.action.href}
                    target={part.action.external ? "_blank" : undefined}
                    rel={part.action.external ? "noopener noreferrer" : undefined}
                    className="whitespace-nowrap font-mono text-[11px] uppercase tracking-[0.14em] text-foreground transition-colors hover:text-brand"
                  >
                    {part.action.label}
                  </a>
                ) : (
                  <span className="whitespace-nowrap font-mono text-[11px] uppercase tracking-[0.14em] text-muted-foreground/60">
                    {part.action.label}
                  </span>
                )}
              </div>
            </div>
          ))}
        </div>
      </div>
    </section>
  );
}
