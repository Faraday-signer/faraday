import "./src/lib/polyfills";

import { useFonts } from "expo-font";
import { StatusBar } from "expo-status-bar";
import { View } from "react-native";
import { SafeAreaProvider } from "react-native-safe-area-context";

import { AppStateProvider } from "./src/lib/app-state";
import { colors } from "./src/lib/theme";
import { RootNavigator } from "./src/navigation/root";

export default function App() {
  const [fontsLoaded] = useFonts({
    "DepartureMono-Regular": require("./assets/fonts/DepartureMono-Regular.otf")
  });

  if (!fontsLoaded) {
    return <View style={{ flex: 1, backgroundColor: colors.bg }} />;
  }

  return (
    <SafeAreaProvider>
      <AppStateProvider>
        <RootNavigator />
        <StatusBar style="light" />
      </AppStateProvider>
    </SafeAreaProvider>
  );
}
