import { useState, type CSSProperties } from "react";

import { FaradayLogo } from "@/lib/brand";
import { useNavigation } from "@/lib/router";
import { formatSol, useWallet } from "@/lib/use-wallet";
import { useSolPrice, useTokens } from "@/lib/use-tokens";
import type { Token } from "@/lib/tokens";
import type { LiveConnectionState } from "@/lib/use-live-balance";
import { ErrorBanner } from "@/components/error-banner";
import { PanelShell } from "@/components/panel-shell";
import { CLUSTER_LABEL } from "@/lib/sol-client";
import { colors, fontFamily, font, letterSpacing, radius, space } from "@/lib/theme";

function shortAddress(address: string): string {
  if (address.length <= 14) return address;
  return `${address.slice(0, 4)}…${address.slice(-4)}`;
}

const networkPillStyle: CSSProperties = {
  display: "inline-flex",
  alignItems: "center",
  padding: `2px ${space.xs}px`,
  borderRadius: radius.pill,
  background: colors.accentSoft,
  color: colors.accent,
  fontFamily: fontFamily.display,
  fontSize: 9,
  letterSpacing: letterSpacing.eyebrow,
  border: `1px solid ${colors.accent}`,
  marginLeft: space.xs
};

const gearStyle: CSSProperties = {
  background: "transparent",
  border: "none",
  color: colors.textMuted,
  padding: space.xxs,
  borderRadius: radius.sm,
  cursor: "pointer",
  display: "inline-flex",
  alignItems: "center"
};

const pairedPillStyle: CSSProperties = {
  display: "inline-flex",
  alignItems: "center",
  gap: space.xs,
  padding: `${space.xxs}px ${space.sm}px`,
  borderRadius: radius.pill,
  background: colors.accentSoft,
  color: colors.accent,
  fontFamily: fontFamily.mono,
  fontSize: font.xs,
  letterSpacing: letterSpacing.loose,
  border: `1px solid ${colors.accent}`,
  cursor: "pointer"
};

const balanceWrapStyle: CSSProperties = {
  display: "flex",
  flexDirection: "column",
  alignItems: "center",
  padding: `${space.xl}px ${space.md}px`,
  gap: 2
};

const heroNumberStyle: CSSProperties = {
  fontFamily: fontFamily.display,
  fontSize: font.hero,
  letterSpacing: letterSpacing.tight,
  color: colors.text,
  lineHeight: 1
};

const heroUnitStyle: CSSProperties = {
  fontFamily: fontFamily.display,
  fontSize: font.xl,
  color: colors.accent,
  letterSpacing: letterSpacing.loose,
  marginLeft: space.xs
};

const heroMetaStyle: CSSProperties = {
  fontFamily: fontFamily.mono,
  fontSize: font.xs,
  color: colors.textMuted,
  marginTop: space.xs,
  letterSpacing: letterSpacing.normal
};

const actionRowStyle: CSSProperties = {
  display: "grid",
  gridTemplateColumns: "1fr 1fr",
  gap: space.sm,
  padding: `0 ${space.md}px`
};

const actionButtonStyle: CSSProperties = {
  background: colors.panel,
  color: colors.text,
  border: `1px solid ${colors.border}`,
  borderRadius: radius.md,
  padding: `${space.sm}px ${space.md}px`,
  fontFamily: fontFamily.display,
  fontSize: font.md,
  letterSpacing: letterSpacing.loose,
  cursor: "pointer",
  textTransform: "uppercase",
  display: "flex",
  alignItems: "center",
  justifyContent: "center",
  gap: space.xs
};

const tokensSectionStyle: CSSProperties = {
  margin: `${space.lg}px ${space.md}px 0`,
  display: "flex",
  flexDirection: "column",
  gap: space.xs
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
  padding: `0 ${space.xs}px ${space.xxs}px`
};

const tokensListStyle: CSSProperties = {
  display: "flex",
  flexDirection: "column",
  background: colors.panel,
  border: `1px solid ${colors.border}`,
  borderRadius: radius.md,
  overflow: "hidden"
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
  textAlign: "left"
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
  color: colors.textMuted
};

const tokenSymbolStyle: CSSProperties = {
  fontFamily: fontFamily.display,
  fontSize: font.md,
  letterSpacing: letterSpacing.loose,
  color: colors.text,
  display: "inline-flex",
  alignItems: "center",
  gap: 6
};

const tokenVerifiedBadgeStyle: CSSProperties = {
  fontFamily: fontFamily.display,
  fontSize: 9,
  letterSpacing: letterSpacing.eyebrow,
  color: colors.accent,
  textTransform: "uppercase"
};

const tokenSubLabelStyle: CSSProperties = {
  fontFamily: fontFamily.mono,
  fontSize: font.xs,
  color: colors.textDim
};

const tokenAmountStyle: CSSProperties = {
  fontFamily: fontFamily.mono,
  fontSize: font.sm,
  color: colors.text,
  textAlign: "right"
};

const tokenUsdStyle: CSSProperties = {
  fontFamily: fontFamily.mono,
  fontSize: font.xs,
  color: colors.textMuted,
  textAlign: "right"
};

const tokensFootnoteStyle: CSSProperties = {
  fontFamily: fontFamily.ui,
  fontSize: font.xs,
  color: colors.textDim,
  padding: `${space.xs}px ${space.xs}px 0`,
  textAlign: "center"
};

const tokensEmptyStyle: CSSProperties = {
  padding: space.md,
  color: colors.textMuted,
  fontSize: font.sm,
  textAlign: "center",
  lineHeight: 1.5
};

function GearIcon() {
  return (
    <svg width="18" height="18" viewBox="0 0 18 18" fill="none" xmlns="http://www.w3.org/2000/svg" aria-hidden>
      <path
        d="M6 0H12V2H6ZM2 2H16V4H2ZM2 4H4V14H2ZM14 4H16V14H14ZM0 6H2V12H0ZM16 6H18V12H16ZM8 8H10V10H8ZM2 14H16V16H2ZM6 16H12V18H6Z"
        fill="currentColor"
      />
    </svg>
  );
}

export function HomeScreen() {
  const nav = useNavigation();
  const wallet = useWallet();
  const { priceUsd: solPriceUsd } = useSolPrice();
  const [justCopied, setJustCopied] = useState(false);

  const solUsdValue =
    solPriceUsd !== null && wallet.solUiAmount !== null
      ? solPriceUsd * wallet.solUiAmount
      : null;

  async function copyPubkey() {
    if (!wallet.pairedPubkey) return;
    try {
      await navigator.clipboard.writeText(wallet.pairedPubkey);
      setJustCopied(true);
      window.setTimeout(() => setJustCopied(false), 1400);
    } catch {
      // no-op
    }
  }

  return (
    <PanelShell
      hideBack
      leading={
        <span style={{ display: "inline-flex", alignItems: "center", gap: space.xs }}>
          <FaradayLogo height={18} />
          <span style={networkPillStyle}>{CLUSTER_LABEL}</span>
        </span>
      }
      trailing={
        <button
          type="button"
          aria-label="Settings"
          style={gearStyle}
          onClick={() => nav.push({ name: "settings" })}
        >
          <GearIcon />
        </button>
      }
    >
      <div style={{ padding: `${space.md}px ${space.md}px 0`, display: "flex", justifyContent: "center" }}>
        {wallet.pairedPubkey ? (
          <button
            type="button"
            style={pairedPillStyle}
            onClick={copyPubkey}
            aria-label={justCopied ? "Address copied" : "Copy address"}
          >
            {shortAddress(wallet.pairedPubkey)}
            <span aria-hidden>{justCopied ? "✓" : "⎘"}</span>
          </button>
        ) : null}
      </div>

      <div style={balanceWrapStyle}>
        <div style={{ display: "flex", alignItems: "baseline" }}>
          <span style={heroNumberStyle}>{formatSol(wallet.solUiAmount)}</span>
          <span style={heroUnitStyle}>SOL</span>
        </div>
        {solUsdValue !== null ? (
          <div
            style={{
              fontFamily: fontFamily.mono,
              fontSize: font.xl,
              color: colors.textMuted,
              marginTop: 4
            }}
          >
            {formatTokenUsd(solUsdValue)}
          </div>
        ) : null}
        <div style={{ ...heroMetaStyle, display: "flex", alignItems: "center", gap: 6 }}>
          <LiveDot state={wallet.liveState} />
          <span>
            {wallet.balanceLoading
              ? "Fetching…"
              : wallet.solUiAmount === null
                ? "Balance unavailable"
                : wallet.liveState === "live"
                  ? `Live on ${CLUSTER_LABEL.toLowerCase()}`
                  : wallet.liveState === "reconnecting"
                    ? `Reconnecting… (polling fallback)`
                    : wallet.liveState === "failed"
                      ? `Live connection down — polling`
                      : `Balance on ${CLUSTER_LABEL.toLowerCase()}`}
          </span>
        </div>
      </div>

      {wallet.balanceError ? (
        <div style={{ padding: `0 ${space.md}px` }}>
          <ErrorBanner
            title="Balance unavailable"
            message={wallet.balanceError}
            onRetry={wallet.refreshBalance}
            retrying={wallet.balanceLoading}
          />
        </div>
      ) : null}

      <div style={actionRowStyle}>
        <button
          type="button"
          style={actionButtonStyle}
          onClick={() => nav.push({ name: "send-compose" })}
        >
          ↗ Send
        </button>
        <button
          type="button"
          style={actionButtonStyle}
          onClick={() => nav.push({ name: "receive" })}
        >
          ↙ Receive
        </button>
      </div>

      <TokensSection />
    </PanelShell>
  );
}

function shortMintInline(mint: string): string {
  return `${mint.slice(0, 4)}…${mint.slice(-4)}`;
}

function formatTokenAmount(amount: number, decimals: number): string {
  if (amount === 0) return "0";
  if (amount < 0.000001) return amount.toExponential(2);
  const maxFrac = Math.min(decimals, amount < 1 ? 6 : 4);
  return amount.toLocaleString("en-US", { maximumFractionDigits: maxFrac });
}

function formatTokenUsd(value: number): string {
  if (value < 0.01) return "<$0.01";
  return `$${value.toLocaleString("en-US", { maximumFractionDigits: 2 })}`;
}

function formatPricePerToken(price: number): string {
  if (price >= 1) {
    return `$${price.toLocaleString("en-US", { maximumFractionDigits: 2 })}`;
  }
  if (price >= 0.01) {
    return `$${price.toFixed(4)}`;
  }
  if (price > 0) {
    return `$${price.toExponential(2)}`;
  }
  return "—";
}

function TokensSection() {
  const wallet = useWallet();
  const { tokens, hiddenUnverifiedCount, loading, error } = useTokens(
    wallet.pairedPubkey
  );

  if (!wallet.pairedPubkey) return null;

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
            <TokenRow
              key={token.mint}
              token={token}
              isLast={idx === tokens.length - 1}
            />
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
  const symbol = token.symbol || shortMintInline(token.mint);
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
          {token.verified ? (
            <span style={tokenVerifiedBadgeStyle}>✓</span>
          ) : null}
        </span>
        <span style={tokenSubLabelStyle}>
          {token.pricePerToken !== null
            ? formatPricePerToken(token.pricePerToken)
            : shortMintInline(token.mint)}
        </span>
      </span>

      <span style={{ display: "flex", flexDirection: "column", gap: 2 }}>
        <span style={tokenAmountStyle}>
          {formatTokenAmount(token.amountUi, token.decimals)}
          {token.symbol ? (
            <span style={{ color: colors.textMuted, marginLeft: 4 }}>
              {token.symbol}
            </span>
          ) : null}
        </span>
        {token.usdValue !== null ? (
          <span style={tokenUsdStyle}>{formatTokenUsd(token.usdValue)}</span>
        ) : null}
      </span>
    </button>
  );
}

function LiveDot({ state }: { state: LiveConnectionState }) {
  const { color, title, pulse } = (() => {
    switch (state) {
      case "live":
        return { color: colors.success, title: "Live", pulse: true };
      case "connecting":
        return { color: colors.accent, title: "Connecting", pulse: true };
      case "reconnecting":
        return { color: colors.warning, title: "Reconnecting", pulse: true };
      case "failed":
        return { color: colors.error, title: "Live connection unavailable", pulse: false };
      default:
        return { color: colors.textDim, title: "Idle", pulse: false };
    }
  })();

  return (
    <span
      aria-label={title}
      title={title}
      style={{
        display: "inline-block",
        width: 6,
        height: 6,
        borderRadius: "50%",
        background: color,
        boxShadow: pulse ? `0 0 0 2px ${color}33` : "none",
        animation: pulse ? "faraday-pulse 1.6s ease-in-out infinite" : "none"
      }}
    />
  );
}
