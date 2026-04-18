import { useState, type CSSProperties } from "react";

import { BrandedQR } from "../../../src/components/branded-qr";
import { PanelShell, LinkButton } from "../../../src/components/panel-shell";
import { useWallet } from "../../../src/lib/use-wallet";
import { colors, fontFamily, font, letterSpacing, radius, space } from "../../../src/lib/theme";

const wrapStyle: CSSProperties = {
  display: "flex",
  flexDirection: "column",
  alignItems: "center",
  padding: `${space.md}px ${space.md}px ${space.lg}px`,
  gap: space.md
};

const helpStyle: CSSProperties = {
  fontSize: font.sm,
  color: colors.textMuted,
  textAlign: "center",
  maxWidth: 280,
  lineHeight: 1.5
};

const addressRowStyle: CSSProperties = {
  display: "flex",
  alignItems: "center",
  gap: space.xs,
  padding: `${space.xs}px ${space.sm}px`,
  borderRadius: radius.md,
  background: colors.panel,
  border: `1px solid ${colors.border}`
};

const addressMonoStyle: CSSProperties = {
  fontFamily: fontFamily.mono,
  fontSize: font.sm,
  color: colors.text,
  letterSpacing: letterSpacing.normal,
  wordBreak: "break-all",
  flex: 1
};

const copyButtonStyle: CSSProperties = {
  background: "transparent",
  border: "none",
  color: colors.accent,
  cursor: "pointer",
  padding: `${space.xxs}px ${space.xs}px`,
  fontFamily: fontFamily.display,
  fontSize: font.xs,
  letterSpacing: letterSpacing.wider,
  textTransform: "uppercase"
};

export function ReceiveScreen() {
  const { pairedPubkey } = useWallet();
  const [copied, setCopied] = useState(false);

  if (!pairedPubkey) {
    return (
      <PanelShell eyebrow="Receive" title="No wallet">
        <div style={wrapStyle}>
          <p style={helpStyle}>Pair your Faraday device first.</p>
        </div>
      </PanelShell>
    );
  }

  const uri = `solana:${pairedPubkey}`;

  async function copyAddress() {
    try {
      await navigator.clipboard.writeText(pairedPubkey!);
      setCopied(true);
      window.setTimeout(() => setCopied(false), 1400);
    } catch {
      // no-op
    }
  }

  return (
    <PanelShell eyebrow="Receive" title="Your Address">
      <div style={wrapStyle}>
        <BrandedQR
          flow="receive"
          value={uri}
          size={260}
          icon={
            <svg width="12" height="12" viewBox="0 0 12 12" fill="none" aria-hidden>
              <path d="M2 2H6V4H4V6H2V2ZM8 2H10V4H8V2ZM6 4H8V6H6V4ZM4 6H6V8H4V6ZM8 6H10V8H8V6ZM2 8H4V10H2V8ZM6 8H8V10H6V8Z" fill="currentColor" />
            </svg>
          }
        />

        <div style={addressRowStyle}>
          <span style={addressMonoStyle}>{pairedPubkey}</span>
          <button type="button" style={copyButtonStyle} onClick={copyAddress}>
            {copied ? "✓ Copied" : "⎘ Copy"}
          </button>
        </div>

        <p style={helpStyle}>
          Send SOL and SPL tokens to this address on Solana mainnet. This address is watch-only in Faraday —
          signing happens on your device.
        </p>

        <LinkButton onClick={copyAddress}>Copy plain address</LinkButton>
      </div>
    </PanelShell>
  );
}
