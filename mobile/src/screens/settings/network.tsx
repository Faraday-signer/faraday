import { StyleSheet, Text, View } from "react-native";

import { ScreenShell } from "../../components/screen-shell";
import { CLUSTER_LABEL, IS_PUBLIC_RPC, RPC_URL, redactRpcUrl } from "../../lib/rpc";
import { colors, font, letterSpacing, radius, space } from "../../lib/theme";

export function SettingsNetworkScreen() {
  const display = redactRpcUrl(RPC_URL);

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
    marginTop: 2
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
  }
});
