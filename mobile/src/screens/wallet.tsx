import { StyleSheet, Text, View } from "react-native";

import { FaradayMark } from "../lib/brand";
import { colors, font, letterSpacing, radius, space } from "../lib/theme";
import { ScreenShell } from "../components/screen-shell";

export function WalletScreen() {
  return (
    <ScreenShell eyebrow="Faraday" title="Wallet">
      <View style={styles.empty}>
        <FaradayMark size={56} />
        <Text style={styles.emptyTitle}>No device paired</Text>
        <Text style={styles.emptyBody}>
          Pair your Faraday signer to view balances and send tokens.
        </Text>
        <View style={styles.disabledButton}>
          <Text style={styles.disabledButtonLabel}>Pair (coming soon)</Text>
        </View>
      </View>
    </ScreenShell>
  );
}

const styles = StyleSheet.create({
  empty: {
    alignItems: "center",
    gap: space.md,
    padding: space.lg,
    borderRadius: radius.lg,
    borderWidth: 1,
    borderColor: colors.border,
    backgroundColor: colors.panel
  },
  emptyTitle: {
    color: colors.text,
    fontSize: font.lg,
    fontWeight: "600"
  },
  emptyBody: {
    color: colors.textMuted,
    fontSize: font.sm,
    textAlign: "center",
    lineHeight: 20
  },
  disabledButton: {
    paddingVertical: space.sm,
    paddingHorizontal: space.lg,
    borderRadius: radius.pill,
    borderWidth: 1,
    borderColor: colors.borderStrong,
    opacity: 0.5
  },
  disabledButtonLabel: {
    color: colors.textMuted,
    fontSize: font.sm,
    letterSpacing: letterSpacing.loose
  }
});
