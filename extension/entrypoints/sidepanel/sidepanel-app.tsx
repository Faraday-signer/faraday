import { useEffect, useState } from "react";

import { ErrorBoundary } from "@/components/error-boundary";
import { sendRuntimeMessage } from "@/lib/runtime";
import { NavigationProvider, useNavigation, type Route } from "@/lib/router";
import type { ExtensionState } from "@/lib/types";

import { HomeScreen } from "./screens/home";
import { OnboardingScreen } from "./screens/onboarding";
import { PairPasteScreen } from "./screens/pair-paste";
import { PairScanScreen } from "./screens/pair-scan";
import { ReceiveScreen } from "./screens/receive";
import { SendComposeScreen } from "./screens/send-compose";
import { SendReviewScreen } from "./screens/send-review";
import { SendSignScreen } from "./screens/send-sign";
import { SettingsScreen } from "./screens/settings";
import { SettingsAboutScreen } from "./screens/settings-about";
import { SettingsDeviceScreen } from "./screens/settings-device";
import { SettingsNetworkScreen } from "./screens/settings-network";
import { SettingsOriginsScreen } from "./screens/settings-origins";
import { TokenDetailScreen } from "./screens/token-detail";

function ActiveRoute() {
  const { current } = useNavigation();
  switch (current.name) {
    case "home":
      return <HomeScreen />;
    case "onboarding":
      return <OnboardingScreen />;
    case "pair-scan":
      return <PairScanScreen />;
    case "pair-paste":
      return <PairPasteScreen />;
    case "receive":
      return <ReceiveScreen />;
    case "send-compose":
      return <SendComposeScreen />;
    case "send-review":
      return <SendReviewScreen />;
    case "send-sign":
      return <SendSignScreen />;
    case "settings":
      return <SettingsScreen />;
    case "settings-device":
      return <SettingsDeviceScreen />;
    case "settings-origins":
      return <SettingsOriginsScreen />;
    case "settings-network":
      return <SettingsNetworkScreen />;
    case "settings-about":
      return <SettingsAboutScreen />;
    case "token-detail":
      return <TokenDetailScreen />;
  }
}

function Bootstrapper() {
  const [initial, setInitial] = useState<Route | null>(null);
  const [bootstrapError, setBootstrapError] = useState<string | null>(null);

  useEffect(() => {
    let cancelled = false;
    void (async () => {
      try {
        const response = await sendRuntimeMessage<ExtensionState>({ type: "faraday:get-state" });
        if (cancelled) return;
        if (!response.ok) {
          setBootstrapError(response.error);
          return;
        }
        const paired = Boolean(response.data.pairedPubkey);
        setInitial(paired ? { name: "home" } : { name: "onboarding" });
      } catch (error) {
        if (cancelled) return;
        const message = error instanceof Error ? error.message : "Failed to start side panel.";
        setBootstrapError(message);
      }
    })();
    return () => {
      cancelled = true;
    };
  }, []);

  if (bootstrapError) {
    // Throw from render so the outer ErrorBoundary paints the fallback UI
    // with the same treatment as any later render crash.
    throw new Error(bootstrapError);
  }

  if (!initial) {
    return null;
  }

  return (
    <NavigationProvider initial={initial}>
      <ActiveRoute />
    </NavigationProvider>
  );
}

export function SidePanelApp() {
  return (
    <ErrorBoundary>
      <Bootstrapper />
    </ErrorBoundary>
  );
}
