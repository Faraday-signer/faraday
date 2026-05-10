import type { ComponentProps } from "react";
import { Text } from "react-native";
import { createBottomTabNavigator } from "@react-navigation/bottom-tabs";
import { DarkTheme, NavigationContainer, type Theme } from "@react-navigation/native";
import { createNativeStackNavigator } from "@react-navigation/native-stack";

import { PairPasteScreen } from "../screens/pair-paste";
import { PairScanScreen } from "../screens/pair-scan";
import { SendComposeScreen } from "../screens/send-compose";
import { SendReviewScreen } from "../screens/send-review";
import { SendSignScreen } from "../screens/send-sign";
import { SettingsNavigator } from "./settings";
import { WalletScreen } from "../screens/wallet";
import type { TokenProgram } from "../lib/tokens";
import { colors, fontFamily, letterSpacing } from "../lib/theme";

export type RootStackParamList = {
  WalletHome: undefined;
  PairScan: undefined;
  PairPaste: undefined;
  SendCompose: undefined;
  SendReview: {
    tokenKind: "sol" | "spl";
    mint: string | null;
    programId: TokenProgram | null;
    decimals: number;
    symbol: string;
    recipient: string;
    amountStr: string;
  };
  SendSign: {
    txBase64: string;
    recipient: string;
    amountStr: string;
    symbol: string;
  };
};

export type RootTabParamList = {
  Wallet: undefined;
  Settings: undefined;
};

const Stack = createNativeStackNavigator<RootStackParamList>();
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
        fontFamily: fontFamily.display,
        letterSpacing: letterSpacing.eyebrow,
        textTransform: "uppercase"
      }}
    >
      {label}
    </Text>
  );
}

function WalletNavigator() {
  return (
    <Stack.Navigator
      screenOptions={{
        headerStyle: { backgroundColor: colors.bg },
        headerTintColor: colors.text,
        headerTitleStyle: { color: colors.text },
        contentStyle: { backgroundColor: colors.bg }
      }}
    >
      <Stack.Screen name="WalletHome" component={WalletScreen} options={{ headerShown: false }} />
      <Stack.Screen name="PairScan" component={PairScanScreen} options={{ title: "" }} />
      <Stack.Screen name="PairPaste" component={PairPasteScreen} options={{ title: "" }} />
      <Stack.Screen name="SendCompose" component={SendComposeScreen} options={{ title: "" }} />
      <Stack.Screen name="SendReview" component={SendReviewScreen} options={{ title: "" }} />
      <Stack.Screen name="SendSign" component={SendSignScreen} options={{ title: "" }} />
    </Stack.Navigator>
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
          component={WalletNavigator}
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
