import type { CSSProperties } from "react";

import { BrandedQR } from "../../../src/components/branded-qr";
import { LinkButton, PanelShell, PrimaryButton } from "../../../src/components/panel-shell";
import { useNavigation, useRouteOf } from "../../../src/lib/router";
import { colors, fontFamily, font, letterSpacing, space } from "../../../src/lib/theme";

const wrapStyle: CSSProperties = {
  display: "flex",
  flexDirection: "column",
  alignItems: "center",
  padding: space.md,
  gap: space.md
};

const helpStyle: CSSProperties = {
  fontSize: font.sm,
  color: colors.textMuted,
  textAlign: "center",
  maxWidth: 300,
  lineHeight: 1.5
};

const metaStyle: CSSProperties = {
  fontFamily: fontFamily.mono,
  fontSize: font.xs,
  color: colors.textMuted,
  letterSpacing: letterSpacing.normal,
  textAlign: "center"
};

/**
 * Inline sign screen for extension-originated sends. Displays the unsigned
 * tx as a branded QR; the actual scan-back and broadcast step will land once
 * the tx-build wiring is in place (see send-review.confirm TODO).
 */
export function SendSignScreen() {
  const nav = useNavigation();
  const route = useRouteOf("send-sign");

  if (!route) return null;

  const { draft, txBase64 } = route;

  return (
    <PanelShell eyebrow="Sign transaction" title="Scan on Faraday">
      <div style={wrapStyle}>
        <BrandedQR
          flow="sign"
          value={txBase64}
          size={260}
          caption={
            <span>
              {draft.amountUi} {draft.symbol} → {draft.recipient.slice(0, 4)}…{draft.recipient.slice(-4)}
            </span>
          }
        />

        <p style={helpStyle}>Hold your Faraday up to this QR, review the details on the device, and approve.</p>

        <p style={metaStyle}>After approval, scan the signed response back here.</p>

        <PrimaryButton onClick={() => alert("Signed-QR scan + broadcast lands with the data layer.")}>
          I&apos;ve scanned → Scan signed
        </PrimaryButton>

        <LinkButton onClick={() => nav.back()}>Cancel</LinkButton>
      </div>
    </PanelShell>
  );
}
