import type { CSSProperties } from "react";

import { ErrorBanner } from "@/components/error-banner";
import { useNavigation } from "@/lib/router";
import type { Token } from "@/lib/tokens";
import {
  formatPricePerToken,
  formatTokenAmount,
  formatTokenUsd,
  shortMint,
} from "@/lib/token-format";
import { useTokens } from "@/lib/use-tokens";
import { colors, fontFamily, font, letterSpacing, radius, space } from "@/lib/theme";

const tokensSectionStyle: CSSProperties = {
  margin: `${space.lg}px ${space.md}px 0`,
  display: "flex",
  flexDirection: "column",
  gap: space.xs,
};

const tokensHeaderStyle: CSSProperties = {
  display: "flex",
  justifyContent: "space-between",
  alignItems: "baseline",
  fontFamily: fontFamily.display,
  fontSize: font.xs,
  letterSpacing: letterSpacing.eyebrow,
  textTransform: "uppercase",
  color: colors.textMuted,
  padding: `0 ${space.xs}px ${space.xxs}px`,
};

const tokensListStyle: CSSProperties = {
  display: "flex",
  flexDirection: "column",
  background: colors.panel,
  border: `1px solid ${colors.border}`,
  borderRadius: radius.md,
  overflow: "hidden",
};

const tokenRowStyle: CSSProperties = {
  display: "flex",
  alignItems: "center",
  gap: space.sm,
  padding: `${space.sm}px ${space.md}px`,
  background: "transparent",
  border: "none",
  borderBottom: `1px solid ${colors.border}`,
  color: colors.text,
  cursor: "pointer",
  textAlign: "left",
};

const tokenLogoStyle: CSSProperties = {
  width: 28,
  height: 28,
  borderRadius: radius.pill,
  background: colors.bg,
  border: `1px solid ${colors.border}`,
  display: "inline-flex",
  alignItems: "center",
  justifyContent: "center",
  flexShrink: 0,
  overflow: "hidden",
  fontFamily: fontFamily.display,
  fontSize: 10,
  color: colors.textMuted,
};

const tokenSymbolStyle: CSSProperties = {
  fontFamily: fontFamily.display,
  fontSize: font.md,
  letterSpacing: letterSpacing.loose,
  color: colors.text,
  display: "inline-flex",
  alignItems: "center",
  gap: 6,
};

const tokenVerifiedBadgeStyle: CSSProperties = {
  fontFamily: fontFamily.display,
  fontSize: 9,
  letterSpacing: letterSpacing.eyebrow,
  color: colors.accent,
  textTransform: "uppercase",
};

const tokenSubLabelStyle: CSSProperties = {
  fontFamily: fontFamily.mono,
  fontSize: font.xs,
  color: colors.textDim,
};

const tokenAmountStyle: CSSProperties = {
  fontFamily: fontFamily.mono,
  fontSize: font.sm,
  color: colors.text,
  textAlign: "right",
};

const tokenUsdStyle: CSSProperties = {
  fontFamily: fontFamily.mono,
  fontSize: font.xs,
  color: colors.textMuted,
  textAlign: "right",
};

const tokensFootnoteStyle: CSSProperties = {
  fontFamily: fontFamily.ui,
  fontSize: font.xs,
  color: colors.textDim,
  padding: `${space.xs}px ${space.xs}px 0`,
  textAlign: "center",
};

const tokensEmptyStyle: CSSProperties = {
  padding: space.md,
  color: colors.textMuted,
  fontSize: font.sm,
  textAlign: "center",
  lineHeight: 1.5,
};

interface TokensSectionProps {
  pairedPubkey: string | null;
}

export function TokensSection({ pairedPubkey }: TokensSectionProps) {
  const { tokens, hiddenUnverifiedCount, loading, error } = useTokens(pairedPubkey);

  if (!pairedPubkey) return null;

  return (
    <div style={tokensSectionStyle}>
      <div style={tokensHeaderStyle}>
        <span>Tokens</span>
        {tokens.length > 0 ? <span>{tokens.length}</span> : null}
      </div>

      {error ? (
        <ErrorBanner title="Token list unavailable" message={error} />
      ) : loading && tokens.length === 0 ? (
        <div style={{ ...tokensListStyle, padding: space.md }}>
          <span style={{ ...tokensEmptyStyle, padding: 0 }}>Loading…</span>
        </div>
      ) : tokens.length === 0 ? (
        <div style={tokensListStyle}>
          <div style={tokensEmptyStyle}>
            {hiddenUnverifiedCount > 0
              ? `No verified tokens. ${hiddenUnverifiedCount} unverified hidden.`
              : "No tokens in this wallet."}
          </div>
        </div>
      ) : (
        <div style={tokensListStyle}>
          {tokens.map((token, idx) => (
            <TokenRow key={token.mint} token={token} isLast={idx === tokens.length - 1} />
          ))}
        </div>
      )}

      {hiddenUnverifiedCount > 0 ? (
        <p style={tokensFootnoteStyle}>
          {hiddenUnverifiedCount} unverified hidden — toggle in Settings → Network.
        </p>
      ) : null}
    </div>
  );
}

function TokenRow({ token, isLast }: { token: Token; isLast: boolean }) {
  const nav = useNavigation();
  const symbol = token.symbol || shortMint(token.mint);
  const rowStyle: CSSProperties = isLast
    ? { ...tokenRowStyle, borderBottom: "none" }
    : tokenRowStyle;

  return (
    <button
      type="button"
      style={rowStyle}
      onClick={() => nav.push({ name: "token-detail", mint: token.mint })}
    >
      <span style={tokenLogoStyle}>
        {token.logoUrl ? (
          <img
            src={token.logoUrl}
            alt=""
            width={28}
            height={28}
            style={{ width: "100%", height: "100%", objectFit: "cover" }}
          />
        ) : (
          symbol.slice(0, 2).toUpperCase()
        )}
      </span>

      <span style={{ display: "flex", flexDirection: "column", gap: 2, flex: 1, minWidth: 0 }}>
        <span style={tokenSymbolStyle}>
          <span style={{ overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>
            {symbol}
          </span>
          {token.verified ? <span style={tokenVerifiedBadgeStyle}>✓</span> : null}
        </span>
        <span style={tokenSubLabelStyle}>
          {token.pricePerToken !== null
            ? formatPricePerToken(token.pricePerToken)
            : shortMint(token.mint)}
        </span>
      </span>

      <span style={{ display: "flex", flexDirection: "column", gap: 2 }}>
        <span style={tokenAmountStyle}>
          {formatTokenAmount(token.amountUi, token.decimals)}
          {token.symbol ? <span style={{ color: colors.textMuted, marginLeft: 4 }}>{token.symbol}</span> : null}
        </span>
        {token.usdValue !== null ? <span style={tokenUsdStyle}>{formatTokenUsd(token.usdValue)}</span> : null}
      </span>
    </button>
  );
}
