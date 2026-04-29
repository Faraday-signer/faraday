import { useState, type CSSProperties } from "react";

import { PanelShell } from "@/components/panel-shell";
import { useRouteOf } from "@/lib/router";
import { useWallet } from "@/lib/use-wallet";
import { useTokens } from "@/lib/use-tokens";
import {
  colors,
  fontFamily,
  font,
  letterSpacing,
  radius,
  space,
} from "@/lib/theme";

const wrapStyle: CSSProperties = {
  display: "flex",
  flexDirection: "column",
  padding: space.md,
  gap: space.md,
};

const heroStyle: CSSProperties = {
  display: "flex",
  flexDirection: "column",
  alignItems: "center",
  gap: 4,
  padding: `${space.lg}px 0 ${space.md}px`,
};

const heroAmountStyle: CSSProperties = {
  fontFamily: fontFamily.display,
  fontSize: font.hero,
  letterSpacing: letterSpacing.tight,
  color: colors.text,
  lineHeight: 1,
};

const heroSymbolStyle: CSSProperties = {
  fontFamily: fontFamily.display,
  fontSize: font.xl,
  color: colors.accent,
  letterSpacing: letterSpacing.loose,
  marginLeft: space.xs,
};

const heroUsdStyle: CSSProperties = {
  fontFamily: fontFamily.mono,
  fontSize: font.sm,
  color: colors.textMuted,
  marginTop: space.xs,
};

const cardStyle: CSSProperties = {
  padding: space.md,
  borderRadius: radius.md,
  background: colors.panel,
  border: `1px solid ${colors.border}`,
  display: "flex",
  flexDirection: "column",
  gap: space.xs,
};

const labelStyle: CSSProperties = {
  fontFamily: fontFamily.display,
  fontSize: font.xs,
  letterSpacing: letterSpacing.eyebrow,
  textTransform: "uppercase",
  color: colors.textMuted,
};

const valueStyle: CSSProperties = {
  fontFamily: fontFamily.mono,
  fontSize: font.sm,
  color: colors.text,
  wordBreak: "break-all",
};

const verifiedPillStyle: CSSProperties = {
  display: "inline-flex",
  alignItems: "center",
  gap: 4,
  padding: `2px ${space.xs}px`,
  borderRadius: radius.pill,
  background: colors.accentSoft,
  color: colors.accent,
  fontFamily: fontFamily.display,
  fontSize: 9,
  letterSpacing: letterSpacing.eyebrow,
  border: `1px solid ${colors.accent}`,
  textTransform: "uppercase",
};

const sendButtonStyle: CSSProperties = {
  background: colors.panel,
  color: colors.textMuted,
  border: `1px solid ${colors.border}`,
  borderRadius: radius.md,
  padding: `${space.sm}px ${space.md}px`,
  fontFamily: fontFamily.display,
  fontSize: font.md,
  letterSpacing: letterSpacing.loose,
  cursor: "not-allowed",
  textTransform: "uppercase",
  display: "flex",
  alignItems: "center",
  justifyContent: "center",
  gap: space.xs,
  opacity: 0.6,
};

const linkStyle: CSSProperties = {
  fontFamily: fontFamily.display,
  fontSize: font.sm,
  letterSpacing: letterSpacing.loose,
  color: colors.accent,
  textTransform: "uppercase",
  textDecoration: "none",
  textAlign: "center",
};

const footnoteStyle: CSSProperties = {
  fontFamily: fontFamily.ui,
  fontSize: font.xs,
  color: colors.textDim,
  lineHeight: 1.5,
  textAlign: "center",
};

function shortMint(mint: string): string {
  if (mint.length <= 14) return mint;
  return `${mint.slice(0, 6)}…${mint.slice(-6)}`;
}

function formatUsd(value: number): string {
  if (value < 0.01) return "<$0.01";
  return `$${value.toLocaleString("en-US", { maximumFractionDigits: 2 })}`;
}

function formatAmount(amount: number, decimals: number): string {
  if (amount === 0) return "0";
  if (amount < 0.000001) return amount.toExponential(2);
  const maxFrac = Math.min(decimals, amount < 1 ? 6 : 4);
  return amount.toLocaleString("en-US", { maximumFractionDigits: maxFrac });
}

function formatPrice(price: number): string {
  if (price >= 1) {
    return `$${price.toLocaleString("en-US", { maximumFractionDigits: 4 })}`;
  }
  if (price >= 0.01) return `$${price.toFixed(4)}`;
  if (price > 0) return `$${price.toExponential(2)}`;
  return "—";
}

export function TokenDetailScreen() {
  const route = useRouteOf("token-detail");
  const wallet = useWallet();
  const { tokens } = useTokens(wallet.pairedPubkey);
  const [justCopied, setJustCopied] = useState(false);

  const token = tokens.find((t) => t.mint === route?.mint);

  async function copyMint() {
    if (!token) return;
    try {
      await navigator.clipboard.writeText(token.mint);
      setJustCopied(true);
      window.setTimeout(() => setJustCopied(false), 1400);
    } catch {
      // no-op
    }
  }

  if (!token) {
    return (
      <PanelShell eyebrow="Token" title="Not found">
        <div style={wrapStyle}>
          <div style={cardStyle}>
            <span style={labelStyle}>Token unavailable</span>
            <span style={valueStyle}>
              This token is no longer in the wallet, or the list is still
              loading. Go back and try again.
            </span>
          </div>
        </div>
      </PanelShell>
    );
  }

  const symbol = token.symbol || "—";
  const programLabel =
    token.programId === "spl-token-2022" ? "Token-2022" : "SPL Token";

  return (
    <PanelShell
      eyebrow="Token"
      title={
        <span style={{ display: "inline-flex", alignItems: "center", gap: space.xs }}>
          {symbol}
          {token.verified ? <span style={verifiedPillStyle}>✓ verified</span> : null}
        </span>
      }
    >
      <div style={wrapStyle}>
        <div style={heroStyle}>
          <div style={{ display: "flex", alignItems: "baseline" }}>
            <span style={heroAmountStyle}>
              {formatAmount(token.amountUi, token.decimals)}
            </span>
            <span style={heroSymbolStyle}>{symbol}</span>
          </div>
          {token.usdValue !== null ? (
            <div style={heroUsdStyle}>{formatUsd(token.usdValue)}</div>
          ) : null}
        </div>

        {token.name ? (
          <div style={cardStyle}>
            <span style={labelStyle}>Name</span>
            <span style={valueStyle}>{token.name}</span>
          </div>
        ) : null}

        <button
          type="button"
          onClick={copyMint}
          style={{ ...cardStyle, cursor: "pointer", textAlign: "left" as const }}
          aria-label={justCopied ? "Mint copied" : "Copy mint"}
        >
          <span style={labelStyle}>
            Mint {justCopied ? "(copied)" : ""}
          </span>
          <span style={valueStyle}>{shortMint(token.mint)}</span>
        </button>

        {token.pricePerToken !== null ? (
          <div style={cardStyle}>
            <span style={labelStyle}>Price</span>
            <span style={valueStyle}>
              {formatPrice(token.pricePerToken)}
              <span style={{ color: colors.textMuted, marginLeft: 6 }}>
                per {symbol}
              </span>
            </span>
          </div>
        ) : null}

        <div style={cardStyle}>
          <span style={labelStyle}>Decimals</span>
          <span style={valueStyle}>{token.decimals}</span>
        </div>

        <div style={cardStyle}>
          <span style={labelStyle}>Program</span>
          <span style={valueStyle}>{programLabel}</span>
        </div>

        <button type="button" style={sendButtonStyle} disabled>
          ↗ Send {symbol}
        </button>
        <p style={footnoteStyle}>Token sending lands in the next pass.</p>

        <a
          href={`https://solscan.io/token/${token.mint}`}
          target="_blank"
          rel="noreferrer noopener"
          style={linkStyle}
        >
          View on Solscan ↗
        </a>
      </div>
    </PanelShell>
  );
}
