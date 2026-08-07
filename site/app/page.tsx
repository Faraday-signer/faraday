import { DemoSection } from "@/components/landing/demo-section";
import { DeviceSection } from "@/components/landing/device-section";
import { FlashTeaser } from "@/components/landing/flash-teaser";
import { Hero } from "@/components/landing/hero";
import { HowItWorks } from "@/components/landing/how-it-works";
import { MeasurementGrid } from "@/components/landing/measurement-grid";
import { SecurityModel } from "@/components/landing/security-model";
import { SiteFooter } from "@/components/landing/site-footer";
import { SiteNav } from "@/components/landing/site-nav";
import { SuiteSection } from "@/components/landing/suite-section";
import { WaitlistCta } from "@/components/landing/waitlist-cta";

export default function Home() {
  return (
    <div className="relative min-h-svh bg-background text-foreground">
      {/* viewport-fixed measurement grid; transparent sections reveal it,
          sections that want a clean slate opt out with their own bg */}
      <div aria-hidden className="fixed inset-0 z-0 opacity-70">
        <MeasurementGrid />
      </div>
      <div className="relative z-10">
        <SiteNav />
        <main>
          <Hero />
          <HowItWorks />
          <SecurityModel />
          <DeviceSection />
          <SuiteSection />
          <DemoSection />
          <FlashTeaser />
          <WaitlistCta />
        </main>
        <SiteFooter />
      </div>
    </div>
  );
}
