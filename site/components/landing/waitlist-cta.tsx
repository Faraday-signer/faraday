import { SectionLabel } from "./primitives";
import { WaitlistForm } from "./waitlist-form";

export function WaitlistCta() {
  return (
    <section id="waitlist" className="border-b border-border bg-background">
      <div className="mx-auto max-w-6xl px-5 py-16 sm:px-8 sm:py-24">
        <div className="max-w-xl">
          <SectionLabel index="07">Early access</SectionLabel>
          <h2 className="mt-5 font-display text-3xl leading-tight tracking-tight text-foreground sm:text-4xl">
            Be first to the early kits
          </h2>
          <p className="mt-5 text-base leading-relaxed text-foreground/70">
            Leave an email and we&apos;ll reach out when devices ship. No
            newsletter, no spam — just a heads-up when there&apos;s something to
            hold.
          </p>
          <div className="mt-8">
            <WaitlistForm />
          </div>
        </div>
      </div>
    </section>
  );
}
