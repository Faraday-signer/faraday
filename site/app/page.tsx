import { DemoSection } from "@/components/landing/demo-section";
import { DeviceSection } from "@/components/landing/device-section";
import { FlashTeaser } from "@/components/landing/flash-teaser";
import { Hero } from "@/components/landing/hero";
import { HowItWorks } from "@/components/landing/how-it-works";
import { SecurityModel } from "@/components/landing/security-model";
import { SiteFooter } from "@/components/landing/site-footer";
import { SiteNav } from "@/components/landing/site-nav";
import { SuiteSection } from "@/components/landing/suite-section";
import { WaitlistCta } from "@/components/landing/waitlist-cta";

export default function Home() {
  return (
    <div className="min-h-svh bg-background text-foreground">
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
  );
}
