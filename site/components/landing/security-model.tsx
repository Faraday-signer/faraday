import { SectionLabel } from "./primitives";

/* Firmware/extension palette for the dark artifact cards */
const FW = {
  bg: "#001721",
  panel: "#002536",
  border: "#114358",
  text: "#E7E7E7",
  muted: "#8C9CA8",
  dim: "#5E7180",
  accent: "#1AF8FF",
};

/** Receipt 01: the firmware's camera-entropy screen, artifact-sized. */
function EntropyReceipt() {
  return (
    <div
      className="w-full max-w-xs rounded-[8px] p-4 font-mono text-xs"
      style={{ background: FW.bg, border: `1px solid ${FW.border}` }}
    >
      <p style={{ color: FW.accent }}>CREATE · CAMERA</p>
      <div className="my-2.5 h-px" style={{ background: FW.border }} />
      <p style={{ color: FW.muted }}>GATHERING RANDOMNESS</p>
      <div className="mt-2.5 flex items-center gap-1">
        {Array.from({ length: 8 }).map((_, i) => (
          <span
            key={i}
            className="h-3 flex-1 rounded-[1px]"
            style={{ background: i < 5 ? FW.accent : FW.panel }}
          />
        ))}
      </div>
      <div className="mt-2 flex items-baseline justify-between">
        <span style={{ color: FW.dim }}>frames hashed · ⊕ hw rng</span>
        <span style={{ color: FW.text }}>160/256</span>
      </div>
    </div>
  );
}

/** Chapter 02 as a full-width pattern interrupt: a spec band spanning the
 * section, in the page's light instrument language. */
function SpecBand() {
  const cells: [string, string][] = [
    ["Radios", "none on the board"],
    ["Flash storage", "none"],
    ["Keys", "RAM only"],
    ["At power-off", "gone"],
  ];
  return (
    <dl className="grid grid-cols-2 gap-y-6 border-y border-border py-8 lg:grid-cols-4">
      {cells.map(([k, v], i) => (
        <div
          key={k}
          className={`px-1 ${i > 0 ? "lg:border-l lg:border-border lg:pl-8" : ""}`}
        >
          <dt className="font-mono text-[11px] uppercase tracking-[0.16em] text-muted-foreground">
            {k}
          </dt>
          <dd className="mt-1.5 font-mono text-sm text-foreground">{v}</dd>
        </div>
      ))}
    </dl>
  );
}

/** Receipt 03: the on-device review screen. */
function ReviewReceipt() {
  return (
    <div
      className="w-full max-w-xs rounded-[8px] p-4 font-mono text-xs leading-relaxed"
      style={{ background: FW.bg, border: `1px solid ${FW.border}` }}
    >
      <p style={{ color: FW.accent }}>APPROVE SEND</p>
      <div className="my-2.5 h-px" style={{ background: FW.border }} />
      <div className="flex items-baseline justify-between">
        <span style={{ color: FW.muted }}>TO</span>
        <span style={{ color: FW.text }}>9WzD…tAWWM</span>
      </div>
      <div className="mt-2 flex items-baseline justify-between">
        <span style={{ color: FW.muted }}>AMOUNT</span>
        <span style={{ color: FW.text }}>
          1.5 <span style={{ color: FW.accent }}>SOL</span>
        </span>
      </div>
      <div className="mt-2.5 flex items-center justify-end gap-2">
        <span style={{ color: FW.dim }}>you approve</span>
        <span style={{ color: FW.accent }}>✓</span>
      </div>
    </div>
  );
}

/** Receipt 04: build it yourself, compare the hash. */
function HashReceipt() {
  return (
    <div
      className="w-full max-w-xs rounded-[8px] p-4 font-mono text-xs leading-loose"
      style={{ background: FW.bg, border: `1px solid ${FW.border}` }}
    >
      <p style={{ color: FW.muted }}>
        <span style={{ color: FW.dim }}>$</span> just build && sha256 faraday.img
      </p>
      <p style={{ color: FW.text }}>
        3f9c…a1d2 <span style={{ color: FW.accent }}>= published release ✓</span>
      </p>
    </div>
  );
}

function Claim({
  n,
  claim,
  body,
  lead = false,
}: {
  n: string;
  claim: string;
  body: string;
  lead?: boolean;
}) {
  return (
    <div>
      <p className="font-mono text-xs text-foreground/40">{n}</p>
      <h3
        className={`mt-2 max-w-lg font-display leading-tight tracking-tight text-foreground ${
          lead ? "text-3xl sm:text-4xl" : "text-2xl sm:text-3xl"
        }`}
      >
        {claim}
      </h3>
      <p className="mt-3 max-w-md text-[15px] leading-relaxed text-foreground/70">{body}</p>
    </div>
  );
}

/**
 * The life of a key as claim + receipt, composed against monotony: the eye
 * zigzags (receipt right, then a full-width spec band, then receipt left,
 * then right again), and the lead chapter carries the largest claim. The
 * anatomy stays parallel so comprehension pays nothing for the variety.
 */
export function SecurityModel() {
  return (
    <section id="security" className="border-b border-border bg-background">
      <div className="mx-auto max-w-6xl px-5 py-16 sm:px-8 sm:py-24">
        <SectionLabel index="01">Security model</SectionLabel>
        <h2 className="mt-5 max-w-2xl font-display text-3xl leading-tight tracking-tight text-foreground sm:text-4xl">
          The life of a key
        </h2>
        <p className="mt-4 max-w-2xl text-base leading-relaxed text-foreground/75">
          Four moments decide whether a key is ever really yours.
        </p>

        <div className="mt-16 flex flex-col gap-24 sm:mt-20 sm:gap-32">
          {/* 01 · lead row, receipt right */}
          <div className="grid grid-cols-1 items-center gap-8 lg:grid-cols-[1fr_minmax(0,20rem)] lg:gap-24">
            <Claim
              n="01"
              lead
              claim="Your key is born from light."
              body="Point the camera at anything. Every frame is hashed and mixed with the hardware RNG, so the seed is never weaker than either source. Prefer your own luck? Flip coins and enter them raw."
            />
            <div className="flex lg:justify-end">
              <EntropyReceipt />
            </div>
          </div>

          {/* 02 · full-width pattern interrupt */}
          <div>
            <Claim
              n="02"
              claim="It lives in memory. Nothing can reach it."
              body="No radios. No flash storage. Power off and it is gone."
            />
            <div className="mt-8">
              <SpecBand />
            </div>
          </div>

          {/* 03 · mirrored row, receipt left */}
          <div className="grid grid-cols-1 items-center gap-8 lg:grid-cols-[minmax(0,20rem)_1fr] lg:gap-24">
            <div className="order-last flex lg:order-first">
              <ReviewReceipt />
            </div>
            <Claim
              n="03"
              claim="It signs nothing you haven't read."
              body="Every transaction is decoded offline, from raw bytes, and shown on the device before you approve."
            />
          </div>

          {/* 04 · back to receipt right */}
          <div className="grid grid-cols-1 items-center gap-8 lg:grid-cols-[1fr_minmax(0,20rem)] lg:gap-24">
            <Claim
              n="04"
              claim="And you can prove every word of this."
              body="The whole stack is open source. Build it and check the hash yourself."
            />
            <div className="flex lg:justify-end">
              <HashReceipt />
            </div>
          </div>
        </div>
      </div>
    </section>
  );
}
