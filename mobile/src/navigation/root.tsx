import type { ComponentProps } from "react";
import { Text } from "react-native";
import { createBottomTabNavigator } from "@react-navigation/bottom-tabs";
import { DarkTheme, NavigationContainer, type Theme } from "@react-navigation/native";

import { WalletScreen } from "../screens/wallet";
import { SettingsNavigator } from "./settings";
import { colors, letterSpacing } from "../lib/theme";

export type RootTabParamList = {
  Wallet: undefined;
  Settings: undefined;
};

const Tab = createBottomTabNavigator<RootTabParamList>();

const navTheme: Theme = {
  ...DarkTheme,
  colors: {
    ...DarkTheme.colors,
    background: colors.bg,
    card: colors.bg,
    border: colors.border,
    primary: colors.accent,
    text: colors.text
  }
};

type TabIconLabelProps = ComponentProps<typeof Text>;

function tabLabel(label: string) {
  return ({ focused, ...rest }: { focused: boolean } & TabIconLabelProps) => (
    <Text
      {...rest}
      style={{
        color: focused ? colors.accent : colors.textMuted,
        fontSize: 11,
        letterSpacing: letterSpacing.eyebrow,
        textTransform: "uppercase"
      }}
    >
      {label}
    </Text>
  );
}

export function RootNavigator() {
  return (
    <NavigationContainer theme={navTheme}>
      <Tab.Navigator
        screenOptions={{
          headerShown: false,
          tabBarStyle: {
            backgroundColor: colors.bg,
            borderTopColor: colors.border
          },
          tabBarShowLabel: true
        }}
      >
        <Tab.Screen
          name="Wallet"
          component={WalletScreen}
          options={{ tabBarLabel: tabLabel("Wallet") }}
        />
        <Tab.Screen
          name="Settings"
          component={SettingsNavigator}
          options={{ tabBarLabel: tabLabel("Settings") }}
        />
      </Tab.Navigator>
    </NavigationContainer>
  );
}
