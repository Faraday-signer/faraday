import Link from "next/link";

import { FaradayMark } from "./faraday-mark";

const GITHUB_URL = "https://github.com/Faraday-signer/faraday";

export function SiteFooter() {
  return (
    <footer className="bg-background">
      <div className="mx-auto max-w-6xl px-5 py-14 sm:px-8">
        <div className="flex flex-col gap-10 sm:flex-row sm:justify-between">
          <div className="max-w-xs">
            <FaradayMark size={28} className="text-brand-ink" />
            <p className="mt-4 text-sm leading-relaxed text-foreground/60">
              An open-source, air-gapped signing suite for Solana. Your keys are
              born offline and stay there.
            </p>
          </div>

          <div className="grid grid-cols-2 gap-x-12 gap-y-8 sm:grid-cols-3">
            <FooterCol title="Product">
              <FooterLink href="#how-it-works">How it works</FooterLink>
              <FooterLink href="#security">Security</FooterLink>
              <FooterLink href="#device">Device</FooterLink>
              <FooterLink href="/flash" internal>
                Flash device
              </FooterLink>
            </FooterCol>

            <FooterCol title="Code">
              <FooterLink href={GITHUB_URL} external>
                GitHub
              </FooterLink>
              {/* Reserved slot for the extension store listing (FA-08). */}
              <span className="font-mono text-[11px] uppercase tracking-[0.14em] text-muted-foreground/50">
                Extension · soon
              </span>
            </FooterCol>

            <FooterCol title="More">
              <FooterLink href="/privacy" internal>
                Privacy
              </FooterLink>
              <FooterLink href="https://x.com/faradaysigner" external>
                X / @faradaysigner
              </FooterLink>
            </FooterCol>
          </div>
        </div>

        <div className="mt-12 flex flex-col gap-2 border-t border-border pt-6 font-mono text-[11px] uppercase tracking-[0.18em] text-muted-foreground sm:flex-row sm:items-center sm:justify-between">
          <p>
            Faraday <span className="mx-1.5 text-foreground/30">·</span> 2026
            <span className="mx-1.5 text-foreground/30">·</span> Offline by design
          </p>
          <p className="text-foreground/40">No servers. No accounts. No radios.</p>
        </div>
      </div>
    </footer>
  );
}

function FooterCol({
  title,
  children,
}: {
  title: string;
  children: React.ReactNode;
}) {
  return (
    <div className="flex flex-col gap-3">
      <p className="font-mono text-[10px] uppercase tracking-[0.2em] text-foreground/40">
        {title}
      </p>
      {children}
    </div>
  );
}

function FooterLink({
  href,
  children,
  external,
  internal,
}: {
  href: string;
  children: React.ReactNode;
  external?: boolean;
  internal?: boolean;
}) {
  const className =
    "font-mono text-[11px] uppercase tracking-[0.14em] text-muted-foreground transition-colors hover:text-foreground";
  if (internal) {
    return (
      <Link href={href} className={className}>
        {children}
      </Link>
    );
  }
  return (
    <a
      href={href}
      target={external ? "_blank" : undefined}
      rel={external ? "noopener noreferrer" : undefined}
      className={className}
    >
      {children}
    </a>
  );
}
