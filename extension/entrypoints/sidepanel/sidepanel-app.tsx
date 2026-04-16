import { useEffect, useState } from "react";

import { sendRuntimeMessage } from "../../src/lib/runtime";
import { NavigationProvider, useNavigation, type Route } from "../../src/lib/router";
import type { ExtensionState } from "../../src/lib/types";

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
  }
}

export function SidePanelApp() {
  const [initial, setInitial] = useState<Route | null>(null);

  useEffect(() => {
    let cancelled = false;
    void (async () => {
      const response = await sendRuntimeMessage<ExtensionState>({ type: "faraday:get-state" });
      if (cancelled) return;
      const paired = response.ok && response.data.pairedPubkey;
      setInitial(paired ? { name: "home" } : { name: "onboarding" });
    })();
    return () => {
      cancelled = true;
    };
  }, []);

  if (!initial) {
    return null;
  }

  return (
    <NavigationProvider initial={initial}>
      <ActiveRoute />
    </NavigationProvider>
  );
}
