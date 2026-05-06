import "./src/lib/polyfills";

import { StatusBar } from "expo-status-bar";
import { SafeAreaProvider } from "react-native-safe-area-context";

import { AppStateProvider } from "./src/lib/app-state";
import { RootNavigator } from "./src/navigation/root";

export default function App() {
  return (
    <SafeAreaProvider>
      <AppStateProvider>
        <RootNavigator />
        <StatusBar style="light" />
      </AppStateProvider>
    </SafeAreaProvider>
  );
}
