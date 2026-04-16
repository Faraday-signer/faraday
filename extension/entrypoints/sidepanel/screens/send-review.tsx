import type { CSSProperties } from "react";

import { LinkButton, PanelShell, PrimaryButton } from "../../../src/components/panel-shell";
import { useNavigation, useRouteOf } from "../../../src/lib/router";
import { colors, fontFamily, font, letterSpacing, radius, space } from "../../../src/lib/theme";

function shortAddress(address: string): string {
  if (address.length <= 12) return address;
  return `${address.slice(0, 4)}…${address.slice(-4)}`;
}

const wrapStyle: CSSProperties = {
  display: "flex",
  flexDirection: "column",
  padding: space.md,
  gap: space.md
};

const rowStyle: CSSProperties = {
  padding: space.md,
  borderRadius: radius.md,
  background: colors.panel,
  border: `1px solid ${colors.border}`,
  display: "flex",
  flexDirection: "column",
  gap: space.xxs
};

const labelStyle: CSSProperties = {
  fontFamily: fontFamily.display,
  fontSize: font.xs,
  letterSpacing: letterSpacing.eyebrow,
  textTransform: "uppercase",
  color: colors.textMuted
};

const primaryValueStyle: CSSProperties = {
  fontFamily: fontFamily.display,
  fontSize: font.xxl,
  letterSpacing: letterSpacing.tight,
  color: colors.text
};

const secondaryValueStyle: CSSProperties = {
  fontFamily: fontFamily.mono,
  fontSize: font.sm,
  color: colors.textMuted
};

const noticeStyle: CSSProperties = {
  padding: space.sm,
  borderRadius: radius.md,
  background: colors.accentSoft,
  border: `1px solid ${colors.accent}`,
  color: colors.accent,
  fontSize: font.xs,
  lineHeight: 1.5,
  fontFamily: fontFamily.mono
};

export function SendReviewScreen() {
  const nav = useNavigation();
  const route = useRouteOf("send-review");

  if (!route) return null;

  const { draft } = route;

  function confirm() {
    // Transaction construction + broadcasting wire-up lands in the data PR.
    alert("Transaction build + broadcast arrives in the data layer pass.");
  }

  return (
    <PanelShell eyebrow="Send" title="Review">
      <div style={wrapStyle}>
        <div style={rowStyle}>
          <span style={labelStyle}>You're sending</span>
          <span style={primaryValueStyle}>
            {draft.amountUi} {draft.symbol}
          </span>
        </div>

        <div style={rowStyle}>
          <span style={labelStyle}>To</span>
          <span style={{ fontFamily: fontFamily.mono, fontSize: font.md, color: colors.text, wordBreak: "break-all" }}>
            {draft.recipient}
          </span>
          <span style={secondaryValueStyle}>{shortAddress(draft.recipient)}</span>
        </div>

        <div style={rowStyle}>
          <span style={labelStyle}>Network fee (est.)</span>
          <span style={{ fontFamily: fontFamily.mono, fontSize: font.sm, color: colors.text }}>~0.000005 SOL</span>
        </div>

        <div style={noticeStyle}>
          You'll see the unsigned transaction as a QR. Scan it with your Faraday, approve on the device, then
          scan the signed response back here.
        </div>

        <div style={{ display: "flex", flexDirection: "column", alignItems: "center", gap: space.xs, marginTop: space.sm }}>
          <PrimaryButton onClick={confirm}>Confirm with Faraday</PrimaryButton>
          <LinkButton onClick={() => nav.back()}>Edit transaction</LinkButton>
        </div>
      </div>
    </PanelShell>
  );
}
