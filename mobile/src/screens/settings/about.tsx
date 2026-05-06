import Constants from "expo-constants";
import { StyleSheet, Text, View } from "react-native";

import { ScreenShell } from "../../components/screen-shell";
import { colors, font, letterSpacing, radius, space } from "../../lib/theme";

export function SettingsAboutScreen() {
  const version = Constants.expoConfig?.version ?? "unknown";

  return (
    <ScreenShell eyebrow="Settings" title="About">
      <View style={styles.card}>
        <Text style={styles.label}>App version</Text>
        <Text style={styles.value}>{version}</Text>
      </View>

      <View style={styles.card}>
        <Text style={styles.label}>About Faraday</Text>
        <Text style={styles.body}>
          Faraday is an air-gapped Solana signer. This phone app is a watch-only companion that
          relays signing to your Faraday device over QR. Private keys never touch this phone.
        </Text>
      </View>
    </ScreenShell>
  );
}

const styles = StyleSheet.create({
  card: {
    padding: space.md,
    borderRadius: radius.md,
    borderWidth: 1,
    borderColor: colors.border,
    backgroundColor: colors.panel,
    gap: space.xs
  },
  label: {
    color: colors.textMuted,
    fontSize: font.xs,
    letterSpacing: letterSpacing.eyebrow,
    textTransform: "uppercase"
  },
  value: {
    color: colors.text,
    fontSize: font.sm,
    fontFamily: "monospace"
  },
  body: {
    color: colors.text,
    fontSize: font.sm,
    lineHeight: 20
  }
});
