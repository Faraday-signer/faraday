import { DeviceSection } from "@/components/landing/device-section";
import { Hero } from "@/components/landing/hero";
import { HowItWorksScroll } from "@/components/landing/how-it-works-scroll";
import { MeasurementGrid } from "@/components/landing/measurement-grid";
import { SecurityModel } from "@/components/landing/security-model";
import { SiteFooter } from "@/components/landing/site-footer";
import { SiteNav } from "@/components/landing/site-nav";
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
