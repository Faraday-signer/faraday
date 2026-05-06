import { Image, StyleSheet, Text, View } from "react-native";

import { formatPricePerToken, formatTokenAmount, formatTokenUsd, shortMint } from "../lib/token-format";
import { colors, font, letterSpacing, radius, space } from "../lib/theme";
import { useTokens } from "../lib/use-tokens";
import type { Token } from "../lib/tokens";

interface Props {
  pairedPubkey: string | null;
}

export function TokensSection({ pairedPubkey }: Props) {
  const { tokens, loading, error, hiddenUnverifiedCount } = useTokens(pairedPubkey);

  if (!pairedPubkey) return null;

  return (
    <View style={styles.section}>
      <Text style={styles.heading}>Tokens</Text>

      {loading && tokens.length === 0 ? (
        <Text style={styles.muted}>Loading tokens…</Text>
      ) : null}

      {error ? <Text style={styles.error}>Could not load tokens: {error}</Text> : null}

      {tokens.length === 0 && !loading && !error ? (
        <Text style={styles.muted}>No tokens with a balance.</Text>
      ) : null}

      <View style={styles.list}>
        {tokens.map((t) => (
          <TokenRow key={t.mint} token={t} />
        ))}
      </View>

      {hiddenUnverifiedCount > 0 ? (
        <Text style={styles.hint}>
          {hiddenUnverifiedCount} unverified token{hiddenUnverifiedCount === 1 ? "" : "s"} hidden.
          Toggle in Settings → Network.
        </Text>
      ) : null}
    </View>
  );
}

function TokenRow({ token }: { token: Token }) {
  const symbol = token.symbol || shortMint(token.mint);
  const amount = formatTokenAmount(token.amountUi, token.decimals);
  const usd = token.usdValue !== null ? formatTokenUsd(token.usdValue) : null;
  const price = token.pricePerToken !== null ? formatPricePerToken(token.pricePerToken) : null;

  return (
    <View style={styles.row}>
      <View style={styles.logoBox}>
        {token.logoUrl ? (
          <Image source={{ uri: token.logoUrl }} style={styles.logoImg} />
        ) : (
          <Text style={styles.logoFallback}>{symbol.slice(0, 2).toUpperCase()}</Text>
        )}
      </View>
      <View style={styles.rowMain}>
        <Text style={styles.symbol} numberOfLines={1}>
          {symbol}
        </Text>
        {price ? <Text style={styles.subtle}>{price}</Text> : null}
      </View>
      <View style={styles.rowEnd}>
        <Text style={styles.amount}>{amount}</Text>
        {usd ? <Text style={styles.subtle}>{usd}</Text> : null}
      </View>
    </View>
  );
}

const styles = StyleSheet.create({
  section: {
    gap: space.sm
  },
  heading: {
    color: colors.textMuted,
    fontSize: font.xs,
    letterSpacing: letterSpacing.eyebrow,
    textTransform: "uppercase",
    paddingHorizontal: space.xxs
  },
  list: {
    backgroundColor: colors.panel,
    borderRadius: radius.lg,
    borderWidth: 1,
    borderColor: colors.border,
    overflow: "hidden"
  },
  row: {
    flexDirection: "row",
    alignItems: "center",
    paddingHorizontal: space.md,
    paddingVertical: space.sm,
    gap: space.sm,
    borderBottomWidth: 1,
    borderBottomColor: colors.border
  },
  logoBox: {
    width: 32,
    height: 32,
    borderRadius: 999,
    backgroundColor: colors.panelHi,
    alignItems: "center",
    justifyContent: "center",
    overflow: "hidden"
  },
  logoImg: {
    width: 32,
    height: 32
  },
  logoFallback: {
    color: colors.text,
    fontSize: font.xs,
    fontWeight: "600"
  },
  rowMain: {
    flex: 1,
    gap: 2
  },
  rowEnd: {
    alignItems: "flex-end",
    gap: 2
  },
  symbol: {
    color: colors.text,
    fontSize: font.md,
    fontWeight: "600"
  },
  amount: {
    color: colors.text,
    fontSize: font.md,
    fontFamily: "monospace"
  },
  subtle: {
    color: colors.textMuted,
    fontSize: font.xs
  },
  muted: {
    color: colors.textMuted,
    fontSize: font.sm
  },
  error: {
    color: colors.error,
    fontSize: font.sm
  },
  hint: {
    color: colors.textDim,
    fontSize: font.xs,
    paddingHorizontal: space.xxs
  }
});
