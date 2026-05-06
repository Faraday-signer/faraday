import * as Clipboard from "expo-clipboard";
import { useCallback, useState } from "react";
import { Pressable, ScrollView, StyleSheet, Text, View } from "react-native";
import { SafeAreaView } from "react-native-safe-area-context";
import type { NativeStackScreenProps } from "@react-navigation/native-stack";

import { FaradayMark } from "../lib/brand";
import { LiveDot } from "../components/live-dot";
import { TokensSection } from "../components/tokens-section";
import { CLUSTER_LABEL } from "../lib/sol-client";
import { formatSol, formatTokenUsd, shortAddress } from "../lib/token-format";
import { colors, font, letterSpacing, radius, space } from "../lib/theme";
import { useSolPrice } from "../lib/use-tokens";
import { useWallet } from "../lib/use-wallet";
import type { RootStackParamList } from "../navigation/root";

type Props = NativeStackScreenProps<RootStackParamList, "WalletHome">;

export function WalletScreen({ navigation }: Props) {
  const wallet = useWallet();
  const { priceUsd } = useSolPrice();
  const [justCopied, setJustCopied] = useState(false);

  const solUsdValue =
    priceUsd !== null && wallet.solUiAmount !== null ? priceUsd * wallet.solUiAmount : null;

  const copyPubkey = useCallback(async () => {
    if (!wallet.pairedPubkey) return;
    await Clipboard.setStringAsync(wallet.pairedPubkey);
    setJustCopied(true);
    setTimeout(() => setJustCopied(false), 1400);
  }, [wallet.pairedPubkey]);

  if (wallet.loading) {
    return (
      <SafeAreaView style={styles.root} edges={["top", "left", "right"]}>
        <Text style={styles.loading}>Loading…</Text>
      </SafeAreaView>
    );
  }

  if (!wallet.pairedPubkey) {
    return (
      <SafeAreaView style={styles.root} edges={["top", "left", "right"]}>
        <View style={styles.emptyWrap}>
          <FaradayMark size={56} />
          <Text style={styles.emptyTitle}>No device paired</Text>
          <Text style={styles.emptyBody}>
            Pair your Faraday signer to view balances and send tokens.
          </Text>
          <Pressable
            onPress={() => navigation.navigate("PairScan")}
            style={({ pressed }) => [styles.primary, pressed && styles.primaryPressed]}
          >
            <Text style={styles.primaryLabel}>Scan device QR</Text>
          </Pressable>
          <Pressable
            onPress={() => navigation.navigate("PairPaste")}
            style={({ pressed }) => [styles.secondary, pressed && styles.secondaryPressed]}
          >
            <Text style={styles.secondaryLabel}>Paste address</Text>
          </Pressable>
        </View>
      </SafeAreaView>
    );
  }

  return (
    <SafeAreaView style={styles.root} edges={["top", "left", "right"]}>
      <View style={styles.header}>
        <View style={styles.headerLeft}>
          <FaradayMark size={20} />
          <View style={styles.networkPill}>
            <Text style={styles.networkPillLabel}>{CLUSTER_LABEL}</Text>
          </View>
        </View>
      </View>

      <ScrollView contentContainerStyle={styles.scroll} showsVerticalScrollIndicator={false}>
        <View style={styles.pubkeyRow}>
          <Pressable
            onPress={() => {
              void copyPubkey();
            }}
            style={({ pressed }) => [styles.pubkeyPill, pressed && styles.pubkeyPillPressed]}
          >
            <Text style={styles.pubkeyLabel}>{shortAddress(wallet.pairedPubkey)}</Text>
            <Text style={styles.pubkeyIcon}>{justCopied ? "✓" : "⎘"}</Text>
          </Pressable>
        </View>

        <View style={styles.heroWrap}>
          <View style={styles.heroRow}>
            <Text style={styles.heroNumber}>{formatSol(wallet.solUiAmount)}</Text>
            <Text style={styles.heroUnit}>SOL</Text>
          </View>
          {solUsdValue !== null ? (
            <Text style={styles.heroUsd}>{formatTokenUsd(solUsdValue)}</Text>
          ) : null}
          <View style={styles.heroMetaRow}>
            <LiveDot state={wallet.liveState} />
            <Text style={styles.heroMeta}>
              {wallet.balanceLoading
                ? "Fetching…"
                : wallet.solUiAmount === null
                  ? "Balance unavailable"
                  : wallet.liveState === "live"
                    ? `Live on ${CLUSTER_LABEL.toLowerCase()}`
                    : wallet.liveState === "reconnecting"
                      ? "Reconnecting… (polling fallback)"
                      : wallet.liveState === "failed"
                        ? "Live connection down — polling"
                        : `Balance on ${CLUSTER_LABEL.toLowerCase()}`}
            </Text>
          </View>
          {wallet.balanceError ? (
            <Pressable onPress={wallet.refreshBalance}>
              <Text style={styles.heroError}>Could not load balance — tap to retry</Text>
            </Pressable>
          ) : null}
        </View>

        <View style={styles.actionRow}>
          <Pressable
            onPress={() => navigation.navigate("SendCompose")}
            style={({ pressed }) => [styles.actionButton, pressed && styles.actionPressed]}
          >
            <Text style={styles.actionLabel}>↗ Send</Text>
          </Pressable>
          <View style={[styles.actionButton, styles.actionDisabled]}>
            <Text style={styles.actionLabel}>↙ Receive (next)</Text>
          </View>
        </View>

        <TokensSection pairedPubkey={wallet.pairedPubkey} />
      </ScrollView>
    </SafeAreaView>
  );
}

const styles = StyleSheet.create({
  root: {
    flex: 1,
    backgroundColor: colors.bg
  },
  loading: {
    color: colors.textMuted,
    textAlign: "center",
    marginTop: space.xl
  },
  emptyWrap: {
    flex: 1,
    alignItems: "center",
    justifyContent: "center",
    gap: space.md,
    padding: space.lg
  },
  emptyTitle: {
    color: colors.text,
    fontSize: font.xl,
    fontWeight: "600"
  },
  emptyBody: {
    color: colors.textMuted,
    fontSize: font.sm,
    textAlign: "center",
    lineHeight: 20,
    paddingHorizontal: space.md
  },
  primary: {
    paddingVertical: space.sm,
    paddingHorizontal: space.lg,
    borderRadius: radius.pill,
    backgroundColor: colors.accent,
    alignItems: "center",
    minWidth: 200
  },
  primaryPressed: {
    backgroundColor: colors.accentStrong
  },
  primaryLabel: {
    color: colors.bg,
    fontSize: font.sm,
    fontWeight: "600",
    letterSpacing: letterSpacing.loose
  },
  secondary: {
    paddingVertical: space.sm,
    paddingHorizontal: space.lg,
    borderRadius: radius.pill,
    borderWidth: 1,
    borderColor: colors.borderStrong,
    alignItems: "center",
    minWidth: 200
  },
  secondaryPressed: {
    backgroundColor: colors.panelHi
  },
  secondaryLabel: {
    color: colors.text,
    fontSize: font.sm,
    letterSpacing: letterSpacing.loose
  },
  header: {
    paddingHorizontal: space.md,
    paddingVertical: space.sm,
    flexDirection: "row",
    alignItems: "center",
    justifyContent: "space-between",
    borderBottomWidth: 1,
    borderBottomColor: colors.border
  },
  headerLeft: {
    flexDirection: "row",
    alignItems: "center",
    gap: space.xs
  },
  networkPill: {
    paddingHorizontal: space.xs,
    paddingVertical: 2,
    borderRadius: radius.pill,
    backgroundColor: colors.accentSoft,
    borderWidth: 1,
    borderColor: colors.accent
  },
  networkPillLabel: {
    color: colors.accent,
    fontSize: 9,
    letterSpacing: letterSpacing.eyebrow,
    textTransform: "uppercase"
  },
  scroll: {
    padding: space.md,
    gap: space.md
  },
  pubkeyRow: {
    alignItems: "center"
  },
  pubkeyPill: {
    flexDirection: "row",
    alignItems: "center",
    gap: space.xs,
    paddingVertical: space.xxs,
    paddingHorizontal: space.sm,
    borderRadius: radius.pill,
    backgroundColor: colors.accentSoft,
    borderWidth: 1,
    borderColor: colors.accent
  },
  pubkeyPillPressed: {
    backgroundColor: "rgba(26, 248, 255, 0.22)"
  },
  pubkeyLabel: {
    color: colors.accent,
    fontSize: font.xs,
    fontFamily: "monospace",
    letterSpacing: letterSpacing.loose
  },
  pubkeyIcon: {
    color: colors.accent,
    fontSize: font.xs
  },
  heroWrap: {
    alignItems: "center",
    paddingVertical: space.lg,
    gap: 4
  },
  heroRow: {
    flexDirection: "row",
    alignItems: "baseline"
  },
  heroNumber: {
    color: colors.text,
    fontSize: font.hero,
    fontWeight: "600"
  },
  heroUnit: {
    color: colors.accent,
    fontSize: font.xl,
    marginLeft: space.xs,
    letterSpacing: letterSpacing.loose
  },
  heroUsd: {
    color: colors.textMuted,
    fontSize: font.xl,
    fontFamily: "monospace"
  },
  heroMetaRow: {
    flexDirection: "row",
    alignItems: "center",
    gap: 6,
    marginTop: space.xs
  },
  heroMeta: {
    color: colors.textMuted,
    fontSize: font.xs,
    fontFamily: "monospace"
  },
  heroError: {
    color: colors.error,
    fontSize: font.xs,
    marginTop: space.xs
  },
  actionRow: {
    flexDirection: "row",
    gap: space.sm
  },
  actionButton: {
    flex: 1,
    paddingVertical: space.sm,
    borderRadius: radius.md,
    borderWidth: 1,
    borderColor: colors.border,
    backgroundColor: colors.panel,
    alignItems: "center"
  },
  actionPressed: {
    backgroundColor: colors.panelHi
  },
  actionDisabled: {
    opacity: 0.5
  },
  actionLabel: {
    color: colors.text,
    fontSize: font.sm,
    letterSpacing: letterSpacing.loose,
    textTransform: "uppercase"
  }
});
