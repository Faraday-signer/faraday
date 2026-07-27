/**
 * Privacy policy page — served at https://faraday.to/privacy and referenced
 * from the Chrome Web Store listing.
 *
 * SOURCE OF TRUTH: extension/PRIVACY_POLICY.md. This page mirrors that file's
 * content by hand. When the policy changes there, update this file to match —
 * including the "Last updated" date.
 */

import type { Metadata } from "next";
import Link from "next/link";
import { Logo } from "@/components/landing/logo";
import { MeasurementGrid } from "@/components/landing/measurement-grid";

export const metadata: Metadata = {
  title: "Privacy Policy — Faraday",
  description:
    "Faraday collects no personal data. What the browser extension stores, what it sends, and where.",
  robots: {
    index: true,
    follow: true,
  },
};

const LAST_UPDATED = "July 27, 2026";

type Block = { kind: "p"; text: string } | { kind: "ul"; items: string[] };

const SECTIONS: { title: string; blocks: Block[] }[] = [
  {
    title: "Summary",
    blocks: [
      {
        kind: "p",
        text: "Faraday does not collect, transmit, or share any personal data. All data stays on your device.",
      },
    ],
  },
  {
    title: "What data the extension stores",
    blocks: [
      {
        kind: "p",
        text: "The extension stores the following data locally in your browser using Chrome's storage API:",
      },
      {
        kind: "ul",
        items: [
          "Your Solana public key (a watch-only address used to display balances and connect to dapps)",
          "A list of dapp origins you have approved for wallet connection",
          "Recent recipient addresses, used to warn you about mistyped destinations",
          "Display preferences, such as whether to show unverified tokens",
          "Pending signing sessions, held in session storage only — they expire after five minutes and are discarded when you close the browser",
        ],
      },
      {
        kind: "p",
        text: "This data never leaves your browser. It is not sent to Faraday, to any server, or to any third party.",
      },
    ],
  },
  {
    title: "What data the extension does not store",
    blocks: [
      {
        kind: "ul",
        items: [
          "Private keys or seed phrases — signing happens exclusively on the air-gapped Faraday device",
          "Browsing history, page content, or URLs",
          "Personal information such as name, email, or location",
          "Analytics, telemetry, or usage metrics",
        ],
      },
    ],
  },
  {
    title: "Network requests",
    blocks: [
      {
        kind: "p",
        text: "The extension makes the following network requests:",
      },
      {
        kind: "ul",
        items: [
          "Solana RPC nodes — to fetch account balances, simulate transactions, and broadcast signed transactions",
          "Jupiter APIs (lite-api.jup.ag, tokens.jup.ag) — to resolve token symbols, USD prices, and the verified-token list used to flag airdrop spam",
          "Token logo images — loaded from whatever URL a token's own metadata points to, which may be any third-party host",
        ],
      },
      {
        kind: "p",
        text: "No data is sent to servers owned or operated by Faraday. No cookies, identifiers, or tracking parameters are included in any request.",
      },
    ],
  },
  {
    title: "Camera access",
    blocks: [
      {
        kind: "p",
        text: "The extension requests access to your camera for exactly one purpose: scanning QR codes displayed by the Faraday device (pairing your public address, and reading signed transactions back from the device). Camera access is requested through the browser's standard permission prompt the first time a scanner opens, and only on the extension's own pages — never on websites you visit.",
      },
      {
        kind: "ul",
        items: [
          "Video frames are processed locally in your browser to detect QR codes and are discarded immediately; nothing is recorded, stored, or transmitted",
          "The microphone is never requested",
          "You can revoke camera access at any time from the browser's extension settings; the extension offers paste-based alternatives where possible",
        ],
      },
    ],
  },
  {
    title: "Third-party services",
    blocks: [
      {
        kind: "p",
        text: "The extension does not integrate any analytics, advertising, or tracking services.",
      },
    ],
  },
  {
    title: "Data sharing",
    blocks: [
      {
        kind: "p",
        text: "Faraday does not sell, rent, or share any user data with third parties. There is no user data to share.",
      },
    ],
  },
  {
    title: "Data retention",
    blocks: [
      {
        kind: "p",
        text: "Stored data persists in Chrome's local storage until you remove it by unpairing your wallet or uninstalling the extension. Pending signing sessions live in session storage and are gone when you close the browser. No data is retained elsewhere.",
      },
    ],
  },
  {
    title: "Children's privacy",
    blocks: [
      {
        kind: "p",
        text: "The extension does not knowingly collect any information from anyone, including children under 13.",
      },
    ],
  },
  {
    title: "Changes to this policy",
    blocks: [
      {
        kind: "p",
        text: "If this policy is updated, the revised version will be posted at the same URL with an updated date. Since we collect no data, meaningful changes are unlikely.",
      },
    ],
  },
];

export default function PrivacyPolicy() {
  return (
    <div className="relative min-h-svh overflow-hidden bg-background text-foreground">
      <MeasurementGrid />

      <div className="relative mx-auto flex min-h-svh max-w-3xl flex-col px-6 py-6 sm:px-10 sm:py-10">
        <header className="mb-6 sm:mb-10">
          <Link href="/" className="inline-block transition-opacity hover:opacity-70">
            <Logo />
          </Link>
        </header>

        <main className="flex-1 pb-8">
          <p className="mb-5 font-mono text-[11px] uppercase tracking-[0.18em] text-muted-foreground sm:text-xs">
            Browser Extension{" "}
            <span className="mx-1.5 text-foreground/30">·</span> Last updated{" "}
            {LAST_UPDATED}
          </p>

          <h1 className="font-display text-3xl font-bold leading-tight tracking-tight sm:text-4xl">
            Privacy Policy
          </h1>

          <div className="mt-10 flex flex-col gap-8 sm:mt-12">
            {SECTIONS.map((section) => (
              <section key={section.title}>
                <h2 className="mb-3 font-mono text-[11px] uppercase tracking-[0.14em] text-foreground/70 sm:text-xs">
                  {section.title}
                </h2>
                <div className="flex flex-col gap-4">
                  {section.blocks.map((block, i) =>
                    block.kind === "p" ? (
                      <p
                        key={i}
                        className="text-sm leading-relaxed text-foreground/80 sm:text-base"
                      >
                        {block.text}
                      </p>
                    ) : (
                      <ul
                        key={i}
                        className="flex flex-col gap-2 text-sm leading-relaxed text-foreground/80 sm:text-base"
                      >
                        {block.items.map((item, j) => (
                          <li key={j} className="flex gap-2">
                            <span
                              aria-hidden
                              className="text-foreground/40"
                            >
                              —
                            </span>
                            <span>{item}</span>
                          </li>
                        ))}
                      </ul>
                    )
                  )}
                </div>
              </section>
            ))}

            <section>
              <h2 className="mb-3 font-mono text-[11px] uppercase tracking-[0.14em] text-foreground/70 sm:text-xs">
                Contact
              </h2>
              <p className="text-sm leading-relaxed text-foreground/80 sm:text-base">
                If you have questions about this privacy policy, contact us at{" "}
                <a
                  href="mailto:javilois@gmail.com"
                  className="text-brand transition-colors hover:text-brand/80"
                >
                  javilois@gmail.com
                </a>
                .
              </p>
            </section>
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
