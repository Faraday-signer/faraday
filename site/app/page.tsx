import type { Metadata } from "next";

import { DeviceSection } from "@/components/landing/device-section";
import { Hero } from "@/components/landing/hero";
import { HowItWorksScroll } from "@/components/landing/how-it-works-scroll";
import { MeasurementGrid } from "@/components/landing/measurement-grid";
import { SecurityModel } from "@/components/landing/security-model";
import { SiteFooter } from "@/components/landing/site-footer";
import { SiteNav } from "@/components/landing/site-nav";
import { WaitlistCta } from "@/components/landing/waitlist-cta";

export const metadata: Metadata = {
  alternates: { canonical: "/" },
};

/* Structured data for search engines and AI answers: who publishes the site
 * and what the product is. Rendered as JSON-LD per the Next.js guide. */
const JSON_LD = {
  "@context": "https://schema.org",
  "@graph": [
    {
      "@type": "Organization",
      "@id": "https://faraday.to/#organization",
      name: "Faraday",
      url: "https://faraday.to",
      logo: "https://faraday.to/icon.svg",
      sameAs: [
        "https://github.com/Faraday-signer/faraday",
        "https://x.com/faradaysigner",
      ],
    },
    {
      "@type": "SoftwareApplication",
      name: "Faraday",
      url: "https://faraday.to",
      applicationCategory: "SecurityApplication",
      operatingSystem: "Raspberry Pi Zero, ESP32-S3",
      description:
        "Open-source, air-gapped hardware signer for Solana. Keys are generated on a device with no antennas and never leave it; transactions cross the gap as QR codes.",
      isAccessibleForFree: true,
      publisher: { "@id": "https://faraday.to/#organization" },
    },
  ],
};

export default function Home() {
  return (
    <div className="relative min-h-svh bg-background text-foreground">
      <script
        type="application/ld+json"
        dangerouslySetInnerHTML={{
          __html: JSON.stringify(JSON_LD).replace(/</g, "\\u003c"),
        }}
      />
      {/* viewport-fixed measurement grid; transparent sections reveal it,
          sections that want a clean slate opt out with their own bg */}
      <div aria-hidden className="fixed inset-0 z-0 opacity-70">
        <MeasurementGrid />
      </div>
      <div className="relative z-10">
        <SiteNav />
        <main>
          <Hero />
          <SecurityModel />
          <HowItWorksScroll />
          <DeviceSection />
          {/* SuiteSection, DemoSection, and FlashTeaser are hidden until
              their content is real; the components remain in the tree. */}
          <WaitlistCta />
        </main>
        <SiteFooter />
      </div>
    </div>
  );
}
