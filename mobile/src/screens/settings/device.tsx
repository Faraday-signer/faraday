import { Alert, Pressable, StyleSheet, Text, View } from "react-native";

import { ScreenShell } from "../../components/screen-shell";
import { useAppState } from "../../lib/app-state";
import { colors, font, letterSpacing, radius, space } from "../../lib/theme";

export function SettingsDeviceScreen() {
  const { pairedPubkey, clearPairedPubkey } = useAppState();

  const onUnpair = () => {
    Alert.alert(
      "Unpair device?",
      "The wallet will return to the empty state. Your Faraday signer is unaffected.",
      [
        { text: "Cancel", style: "cancel" },
        {
          text: "Unpair",
          style: "destructive",
          onPress: () => {
            void clearPairedPubkey();
          }
        }
      ]
    );
  };

  return (
    <ScreenShell eyebrow="Settings" title="Device">
      <View style={styles.card}>
        <Text style={styles.label}>Paired pubkey</Text>
        <Text style={styles.value}>{pairedPubkey ?? "— not paired —"}</Text>
      </View>

      {pairedPubkey ? (
        <Pressable
          onPress={onUnpair}
          style={({ pressed }) => [styles.danger, pressed && styles.dangerPressed]}
        >
          <Text style={styles.dangerLabel}>Unpair device</Text>
        </Pressable>
      ) : null}
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
  danger: {
    paddingVertical: space.sm,
    paddingHorizontal: space.lg,
    borderRadius: radius.pill,
    borderWidth: 1,
    borderColor: colors.error,
    alignItems: "center"
  },
  dangerPressed: {
    backgroundColor: "rgba(255, 107, 107, 0.12)"
  },
  dangerLabel: {
    color: colors.error,
    fontSize: font.sm,
    fontWeight: "600",
    letterSpacing: letterSpacing.loose
  }
});
