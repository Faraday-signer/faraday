import { createNativeStackNavigator } from "@react-navigation/native-stack";

import { SettingsAboutScreen } from "../screens/settings/about";
import { SettingsHomeScreen } from "../screens/settings";
import { SettingsNetworkScreen } from "../screens/settings/network";
import { colors } from "../lib/theme";

export type SettingsStackParamList = {
  SettingsHome: undefined;
  Network: undefined;
  About: undefined;
};

const Stack = createNativeStackNavigator<SettingsStackParamList>();

export function SettingsNavigator() {
  return (
    <Stack.Navigator
      screenOptions={{
        headerStyle: { backgroundColor: colors.bg },
        headerTintColor: colors.text,
        headerTitleStyle: { color: colors.text },
        contentStyle: { backgroundColor: colors.bg }
      }}
    >
      <Stack.Screen
        name="SettingsHome"
        component={SettingsHomeScreen}
        options={{ headerShown: false }}
      />
      <Stack.Screen name="Network" component={SettingsNetworkScreen} options={{ title: "" }} />
      <Stack.Screen name="About" component={SettingsAboutScreen} options={{ title: "" }} />
    </Stack.Navigator>
  );
}
