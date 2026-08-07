import Link from "next/link";

import { Logo } from "./logo";

const SECTION_LINKS = [
  { href: "#how-it-works", label: "How it works" },
  { href: "#security", label: "Security" },
  { href: "#device", label: "Device" },
  { href: "#suite", label: "The suite" },
];

const GITHUB_URL = "https://github.com/Faraday-signer/faraday";

export function SiteNav() {
  return (
    <header className="sticky top-0 z-50 border-b border-border/70 bg-background/80 backdrop-blur-md">
      <nav className="mx-auto flex h-14 max-w-6xl items-center justify-between px-5 sm:px-8">
        <Link
          href="/"
          aria-label="Faraday home"
          className="inline-flex items-center transition-opacity hover:opacity-80"
        >
          <Logo height={20} />
        </Link>

        <div className="hidden items-center gap-7 md:flex">
          {SECTION_LINKS.map((link) => (
            <a
              key={link.href}
              href={link.href}
              className="font-mono text-[11px] uppercase tracking-[0.16em] text-muted-foreground transition-colors hover:text-foreground"
            >
              {link.label}
            </a>
          ))}
          {/* Reserved slot for the web flasher (FA-13); ships as /flash. */}
          <Link
            href="/flash"
            className="group inline-flex items-center gap-1.5 font-mono text-[11px] uppercase tracking-[0.16em] text-muted-foreground transition-colors hover:text-foreground"
          >
            Flash device
            <span className="rounded-sm border border-brand/40 px-1 py-px text-[11px] leading-none tracking-[0.1em] text-brand">
              soon
            </span>
          </Link>
        </div>

        <div className="flex items-center gap-4">
          <a
            href={GITHUB_URL}
            target="_blank"
            rel="noopener noreferrer"
            aria-label="Faraday on GitHub"
            className="text-muted-foreground transition-colors hover:text-foreground"
          >
            <svg viewBox="0 0 16 16" width="18" height="18" fill="currentColor" aria-hidden>
              <path d="M8 0C3.58 0 0 3.58 0 8c0 3.54 2.29 6.53 5.47 7.59.4.07.55-.17.55-.38 0-.19-.01-.82-.01-1.49-2.01.37-2.53-.49-2.69-.94-.09-.23-.48-.94-.82-1.13-.28-.15-.68-.52-.01-.53.63-.01 1.08.58 1.23.82.72 1.21 1.87.87 2.33.66.07-.52.28-.87.51-1.07-1.78-.2-3.64-.89-3.64-3.95 0-.87.31-1.59.82-2.15-.08-.2-.36-1.02.08-2.12 0 0 .67-.21 2.2.82.64-.18 1.32-.27 2-.27.68 0 1.36.09 2 .27 1.53-1.04 2.2-.82 2.2-.82.44 1.1.16 1.92.08 2.12.51.56.82 1.27.82 2.15 0 3.07-1.87 3.75-3.65 3.95.29.25.54.73.54 1.48 0 1.07-.01 1.93-.01 2.2 0 .21.15.46.55.38A8.01 8.01 0 0 0 16 8c0-4.42-3.58-8-8-8Z" />
            </svg>
          </a>
          <a
            href="#waitlist"
            className="hidden rounded-sm bg-primary px-3.5 py-1.5 font-mono text-[11px] uppercase tracking-[0.14em] text-brand transition-colors hover:bg-primary/90 sm:inline-block"
          >
            Early access
          </a>
        </div>
      </nav>
    </header>
  );
}
