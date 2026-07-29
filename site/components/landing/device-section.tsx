import { SectionLabel, Tag } from "./primitives";

const SPECS = [
  { k: "Computer", v: "Raspberry Pi Zero 1.3" },
  { k: "Display", v: "Waveshare 1.3\" LCD · 240×240" },
  { k: "Camera", v: "Pi Camera v1.3 (OV5647)" },
  { k: "I/O channel", v: "Camera in · screen out" },
  { k: "Radios", v: "None" },
  { k: "Approx. cost", v: "~$35" },
];

export function DeviceSection() {
  return (
    <section id="device" className="border-b border-border">
      <div className="mx-auto grid max-w-6xl grid-cols-1 gap-12 px-5 py-16 sm:px-8 sm:py-24 lg:grid-cols-2 lg:gap-16">
        <div>
          <SectionLabel index="03">The device</SectionLabel>
          <h2 className="mt-5 font-display text-3xl leading-tight tracking-tight text-foreground sm:text-4xl">
            Built from parts you can buy and inspect
          </h2>
          <p className="mt-5 max-w-xl text-base leading-relaxed text-foreground/70">
            No secure element you have to take on faith — just an open board
            with no network hardware, a screen, and a camera. The Pi Zero 1.3 is
            recommended precisely because the radio chip isn&apos;t physically
            there, so there is nothing to misconfigure or trust to &ldquo;off.&rdquo;
          </p>

          <div className="mt-8 flex flex-wrap gap-2">
            <Tag tone="brand">Shipping today · Pi Zero</Tag>
            <Tag>In progress · ESP32-S3</Tag>
          </div>

          <p className="mt-6 max-w-xl text-sm leading-relaxed text-foreground/60">
            A smaller, cheaper ESP32-S3 build is in development. No hardware to
            hand? The same firmware runs as a desktop simulator with your
            webcam — try the full signing flow before you build a thing.
          </p>
        </div>

        <div className="rounded-sm border border-border bg-muted/30 p-6 sm:p-8">
          <p className="font-mono text-[11px] uppercase tracking-[0.18em] text-muted-foreground">
            Reference build
          </p>
          <dl className="mt-5 divide-y divide-border">
            {SPECS.map((spec) => (
              <div
                key={spec.k}
                className="flex items-baseline justify-between gap-4 py-3"
              >
                <dt className="font-mono text-[11px] uppercase tracking-[0.14em] text-muted-foreground">
                  {spec.k}
                </dt>
                <dd className="text-right font-mono text-sm text-foreground">
                  {spec.v}
                </dd>
              </div>
            ))}
          </dl>
        </div>
      </div>
    </section>
  );
}
