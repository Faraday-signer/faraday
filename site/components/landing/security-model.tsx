import { FinderPattern } from "./finder-pattern";
import { SectionLabel } from "./primitives";

const PILLARS = [
  {
    title: "No antennas, no remote",
    body: "The signer runs on a Raspberry Pi Zero 1.3 — a board with no Wi-Fi, Bluetooth, NFC, or cellular chip. There is no remote attack surface because there is no radio to reach. The camera and the screen are the only way in or out.",
  },
  {
    title: "Keys live in RAM only",
    body: "Your seed is generated on the device from camera entropy and held in memory while it's powered. It is never written to storage, never logged, never displayed outside the flows built for it. Power off and it's gone.",
  },
  {
    title: "You sign what you see",
    body: "Faraday decodes each transaction from its raw bytes — offline, with no RPC — and shows it in plain terms: Jupiter and Raydium swaps, SPL transfers, staking, Anchor calls, Ika multisig approvals. Anything it can't decode is flagged, never guessed.",
  },
  {
    title: "Open all the way down",
    body: "Firmware, the OS image recipe, the browser extension, the mobile app — the whole stack is open source. Read it, build it yourself, and verify the binary you run is the one you compiled. No vendor firmware to trust.",
  },
];

export function SecurityModel() {
  return (
    <section id="security" className="border-b border-border">
      <div className="mx-auto max-w-6xl px-5 py-16 sm:px-8 sm:py-24">
        <div className="flex items-start justify-between gap-6">
          <div>
            <SectionLabel index="02">Security model</SectionLabel>
            <h2 className="mt-5 max-w-2xl font-display text-3xl leading-tight tracking-tight text-foreground sm:text-4xl">
              The air gap is the product
            </h2>
            <p className="mt-5 max-w-xl text-base leading-relaxed text-foreground/70">
              Hot wallets get drained and closed-source hardware asks you to
              trust a vendor you can&apos;t audit. Faraday removes the trust
              instead of relocating it.
            </p>
          </div>
          <FinderPattern
            size={64}
            className="hidden shrink-0 text-border sm:block"
          />
        </div>

        <div className="mt-12 grid grid-cols-1 gap-px overflow-hidden rounded-sm border border-border bg-border md:grid-cols-2">
          {PILLARS.map((pillar) => (
            <div key={pillar.title} className="bg-background p-6 sm:p-8">
              <h3 className="font-display text-xl tracking-tight text-foreground">
                {pillar.title}
              </h3>
              <p className="mt-3 text-sm leading-relaxed text-foreground/70 sm:text-base">
                {pillar.body}
              </p>
            </div>
          ))}
        </div>
      </div>
    </section>
  );
}
