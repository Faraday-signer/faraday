import { useState, type CSSProperties } from "react";

import { PanelShell, PrimaryButton } from "../../../src/components/panel-shell";
import { useNavigation } from "../../../src/lib/router";
import { formatSol, useWallet } from "../../../src/lib/use-wallet";
import { isValidSolanaAddress } from "../../../src/lib/solana";
import { colors, fontFamily, font, letterSpacing, radius, space } from "../../../src/lib/theme";

const wrapStyle: CSSProperties = {
  display: "flex",
  flexDirection: "column",
  padding: space.md,
  gap: space.md
};

const labelStyle: CSSProperties = {
  fontFamily: fontFamily.display,
  fontSize: font.xs,
  letterSpacing: letterSpacing.eyebrow,
  textTransform: "uppercase",
  color: colors.textMuted
};

const cardStyle: CSSProperties = {
  padding: space.sm,
  borderRadius: radius.md,
  background: colors.panel,
  border: `1px solid ${colors.border}`,
  display: "flex",
  flexDirection: "column",
  gap: space.xs
};

const inputStyle: CSSProperties = {
  width: "100%",
  boxSizing: "border-box",
  padding: `${space.sm}px ${space.md}px`,
  borderRadius: radius.md,
  border: `1px solid ${colors.borderStrong}`,
  background: colors.panel,
  color: colors.text,
  fontFamily: fontFamily.mono,
  fontSize: font.sm,
  outline: "none"
};

const amountInputStyle: CSSProperties = {
  ...inputStyle,
  fontFamily: fontFamily.display,
  fontSize: font.xl,
  letterSpacing: letterSpacing.loose
};

const availableRowStyle: CSSProperties = {
  display: "flex",
  justifyContent: "space-between",
  alignItems: "baseline",
  fontSize: font.xs,
  color: colors.textMuted,
  fontFamily: fontFamily.mono
};

const maxBtnStyle: CSSProperties = {
  background: "transparent",
  border: "none",
  color: colors.accent,
  cursor: "pointer",
  fontFamily: fontFamily.display,
  fontSize: font.xs,
  letterSpacing: letterSpacing.wider,
  textTransform: "uppercase",
  padding: 0
};

const helperStyle = (isError: boolean): CSSProperties => ({
  fontSize: font.xs,
  color: isError ? colors.error : colors.textDim,
  minHeight: 16
});

export function SendComposeScreen() {
  const nav = useNavigation();
  const { solUiAmount, pairedPubkey } = useWallet();
  const availableSol = solUiAmount ?? 0;

  const [amount, setAmount] = useState("");
  const [recipient, setRecipient] = useState("");

  const amountNum = Number(amount);
  const amountValid =
    Number.isFinite(amountNum) && amountNum > 0 && amountNum <= availableSol;
  const recipientValid = isValidSolanaAddress(recipient.trim());
  const canReview = amountValid && recipientValid && pairedPubkey !== null;

  function advance() {
    if (!canReview || !pairedPubkey) return;
    nav.push({
      name: "send-review",
      draft: {
        mint: "SOL",
        symbol: "SOL",
        decimals: 9,
        amountUi: amount,
        recipient: recipient.trim()
      }
    });
  }

  return (
    <PanelShell eyebrow="Send" title="Send SOL">
      <div style={wrapStyle}>
        <div style={cardStyle}>
          <span style={labelStyle}>Token</span>
          <span style={{ fontFamily: fontFamily.display, fontSize: font.lg, color: colors.text, letterSpacing: letterSpacing.loose }}>
            SOL
          </span>
          <div style={availableRowStyle}>
            <span>Available</span>
            <span>{formatSol(solUiAmount)} SOL</span>
          </div>
        </div>

        <label style={{ display: "flex", flexDirection: "column", gap: space.xs }}>
          <span style={labelStyle}>Amount</span>
          <input
            value={amount}
            onChange={(event) => setAmount(event.target.value)}
            inputMode="decimal"
            placeholder="0.00"
            style={amountInputStyle}
          />
          <div style={availableRowStyle}>
            <span />
            <button type="button" onClick={() => setAmount(String(availableSol))} style={maxBtnStyle}>
              Max
            </button>
          </div>
        </label>

        <label style={{ display: "flex", flexDirection: "column", gap: space.xs }}>
          <span style={labelStyle}>To</span>
          <input
            value={recipient}
            onChange={(event) => setRecipient(event.target.value)}
            spellCheck={false}
            placeholder="Solana address"
            style={inputStyle}
          />
          <span style={helperStyle(!!recipient && !recipientValid)}>
            {recipient && !recipientValid ? "Not a valid Solana address." : "Paste or scan the recipient's address."}
          </span>
        </label>

        <div style={{ display: "flex", justifyContent: "center", marginTop: space.sm }}>
          <PrimaryButton onClick={advance} disabled={!canReview}>
            Review
          </PrimaryButton>
        </div>
      </div>
    </PanelShell>
  );
}
