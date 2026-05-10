import { Pressable, StyleSheet, Text, View } from "react-native";

import { ScreenShell } from "../../components/screen-shell";
import { CLUSTER_LABEL, IS_PUBLIC_RPC, RPC_URL, redactRpcUrl } from "../../lib/sol-client";
import { colors, font, letterSpacing, radius, space } from "../../lib/theme";
import { useTokenSettings } from "../../lib/use-tokens";

export function SettingsNetworkScreen() {
  const display = redactRpcUrl(RPC_URL);
  const { settings, setShowUnverified } = useTokenSettings();

  return (
    <ScreenShell eyebrow="Settings" title="Network">
      <View style={styles.card}>
        <Text style={styles.label}>Cluster</Text>
        <Text style={styles.value}>{CLUSTER_LABEL.toLowerCase()}-beta</Text>
      </View>

      <View style={styles.card}>
        <Text style={styles.label}>RPC endpoint</Text>
        <Text style={styles.value}>{display}</Text>
        <Text style={styles.foot}>Configured via EXPO_PUBLIC_RPC_URL at build time.</Text>
      </View>

      {IS_PUBLIC_RPC ? (
        <View style={styles.warn}>
          <Text style={styles.warnText}>
            Public Solana RPC. Rate-limited — expect 429s under any real load. Set
            EXPO_PUBLIC_RPC_URL in mobile/.env and rebuild to use Helius / QuickNode / your own
            endpoint.
          </Text>
        </View>
      ) : null}

      <View style={styles.card}>
        <View style={styles.toggleRow}>
          <Text style={styles.label}>Show unverified tokens</Text>
          <Pressable
            accessibilityRole="switch"
            accessibilityState={{ checked: settings.showUnverified }}
            onPress={() => {
              void setShowUnverified(!settings.showUnverified);
            }}
            style={[styles.switch, settings.showUnverified && styles.switchOn]}
          >
            <View style={[styles.knob, settings.showUnverified && styles.knobOn]} />
          </Pressable>
        </View>
        <Text style={styles.foot}>
          Hidden tokens use the Jupiter verified-tag list. Unverified mints are often airdrop spam
          — keep this off unless you're expecting a token that hasn't been picked up yet.
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
  foot: {
    color: colors.textDim,
    fontSize: font.xs,
    marginTop: 2,
    lineHeight: 18
  },
  warn: {
    padding: space.sm,
    borderRadius: radius.md,
    borderWidth: 1,
    borderColor: colors.warning,
    backgroundColor: "rgba(255, 180, 84, 0.12)"
  },
  warnText: {
    color: colors.warning,
    fontSize: font.xs,
    lineHeight: 18
  },
  toggleRow: {
    flexDirection: "row",
    alignItems: "center",
    justifyContent: "space-between"
  },
  switch: {
    width: 36,
    height: 20,
    borderRadius: radius.pill,
    backgroundColor: colors.borderStrong,
    justifyContent: "center"
  },
  switchOn: {
    backgroundColor: colors.accent
  },
  knob: {
    width: 16,
    height: 16,
    borderRadius: radius.pill,
    backgroundColor: colors.bg,
    marginLeft: 2
  },
  knobOn: {
    marginLeft: 18
  }
});
