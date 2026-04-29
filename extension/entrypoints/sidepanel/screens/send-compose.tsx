import { useMemo, useState, type CSSProperties } from "react";

import { ErrorBanner } from "@/components/error-banner";
import { LinkButton, PanelShell, PrimaryButton } from "@/components/panel-shell";
import { useNavigation } from "@/lib/router";
import { formatSol, useWallet } from "@/lib/use-wallet";
import {
  FEE_RESERVE_LAMPORTS,
  useRecipientCheck,
  validateAmount,
  validateRecipientFormat,
} from "@/lib/send-validation";
import { colors, fontFamily, font, letterSpacing, radius, space } from "@/lib/theme";

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

const inputErrorStyle: CSSProperties = {
  borderColor: colors.error
};

const inputWarnStyle: CSSProperties = {
  borderColor: colors.warning
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

const maxBtnDisabledStyle: CSSProperties = {
  ...maxBtnStyle,
  color: colors.textDim,
  cursor: "not-allowed"
};

const helperStyle = (kind: "neutral" | "warn" | "error" | "info"): CSSProperties => ({
  fontSize: font.xs,
  color:
    kind === "error"
      ? colors.error
      : kind === "warn"
        ? colors.warning
        : kind === "info"
          ? colors.textMuted
          : colors.textDim,
  minHeight: 16,
  lineHeight: 1.4
});

const pairFirstStyle: CSSProperties = {
  padding: space.lg,
  borderRadius: radius.md,
  border: `1px dashed ${colors.border}`,
  background: colors.panel,
  color: colors.textMuted,
  textAlign: "center",
  lineHeight: 1.5,
  fontSize: font.sm
};

const LAMPORTS_PER_SOL = 1_000_000_000n;

export function SendComposeScreen() {
  const nav = useNavigation();
  const wallet = useWallet();
  const [amount, setAmount] = useState("");
  const [recipient, setRecipient] = useState("");

  // All hooks must run unconditionally — bootstrap-state branching happens
  // after the validation hooks below.
  const balanceLamports = wallet.solLamports;

  const amountCheck = useMemo(
    () =>
      validateAmount({
        amountStr: amount,
        balanceRaw: balanceLamports,
        decimals: 9,
        symbol: "SOL",
        feeReserve: FEE_RESERVE_LAMPORTS,
      }),
    [amount, balanceLamports]
  );

  const recipientFormat = useMemo(
    () => validateRecipientFormat(recipient),
    [recipient]
  );

  const intendedLamports =
    amountCheck.kind === "ok" || amountCheck.kind === "warn" ? amountCheck.raw : null;

  const recipientNetwork = useRecipientCheck(
    recipient,
    wallet.pairedPubkey,
    intendedLamports
  );

  // ── Bootstrap states ─────────────────────────────────────────────────────
  // Initial extension-state load (paired pubkey).
  if (wallet.loading) {
    return (
      <PanelShell eyebrow="Send" title="Send SOL">
        <div style={wrapStyle}>
          <p style={{ ...helperStyle("info"), textAlign: "center", padding: space.xl }}>
            Loading wallet…
          </p>
        </div>
      </PanelShell>
    );
  }

  // Case #13 — no paired pubkey. Defensive: bootstrapper normally redirects
  // to onboarding, but if the user navigated here directly we render a
  // pair-first state.
  if (!wallet.pairedPubkey) {
    return (
      <PanelShell eyebrow="Send" title="Send SOL">
        <div style={wrapStyle}>
          <div style={pairFirstStyle}>
            Pair a Faraday device first.
          </div>
          <PrimaryButton onClick={() => nav.reset({ name: "onboarding" })}>
            Go to onboarding
          </PrimaryButton>
        </div>
      </PanelShell>
    );
  }

  // ── Derived state ────────────────────────────────────────────────────────
  const balanceLoaded = balanceLamports !== null;
  const amountOk = amountCheck.kind === "ok" || amountCheck.kind === "warn";
  const recipientOk =
    recipientFormat.kind === "ok" &&
    (recipientNetwork.kind === "ok" || recipientNetwork.kind === "warn");
  const canReview = amountOk && recipientOk && balanceLoaded;

  function applyMax() {
    if (balanceLamports === null) return;
    const spendable =
      balanceLamports > FEE_RESERVE_LAMPORTS
        ? balanceLamports - FEE_RESERVE_LAMPORTS
        : 0n;
    const ui = Number(spendable) / Number(LAMPORTS_PER_SOL);
    setAmount(ui.toFixed(9).replace(/\.?0+$/, "") || "0");
  }

  function advance() {
    if (!canReview || !wallet.pairedPubkey) return;
    nav.push({
      name: "send-review",
      draft: {
        mint: "SOL",
        symbol: "SOL",
        decimals: 9,
        amountUi: amount.trim(),
        recipient: recipient.trim(),
      },
    });
  }

  // ── Inline messages ──────────────────────────────────────────────────────
  // Amount helper — only shows once the user has typed something. Empty
  // input shouldn't read as an error before any interaction.
  let amountHelperKind: "neutral" | "warn" | "error" | "info" = "neutral";
  let amountHelperMsg = "";
  if (amount.length > 0) {
    if (amountCheck.kind === "error") {
      amountHelperKind = "error";
      amountHelperMsg = amountCheck.message;
    } else if (amountCheck.kind === "warn") {
      amountHelperKind = "warn";
      amountHelperMsg = amountCheck.message;
    }
  }

  // Recipient helper — combines format check (always-on) and network check
  // (after format passes). Empty input shows a hint, not an error.
  let recipientHelperKind: "neutral" | "warn" | "error" | "info" = "neutral";
  let recipientHelperMsg = recipient.length === 0
    ? "Paste or scan the recipient's address."
    : "";
  if (recipient.length > 0) {
    if (recipientFormat.kind === "error" && recipientFormat.message) {
      recipientHelperKind = "error";
      recipientHelperMsg = recipientFormat.message;
    } else if (recipientNetwork.kind === "loading") {
      recipientHelperKind = "info";
      recipientHelperMsg = "Checking address…";
    } else if (recipientNetwork.kind === "error") {
      recipientHelperKind = "error";
      recipientHelperMsg = recipientNetwork.message;
    } else if (recipientNetwork.kind === "warn") {
      recipientHelperKind = "warn";
      recipientHelperMsg = recipientNetwork.message;
    } else if (recipientNetwork.kind === "ok" && recipientNetwork.note) {
      recipientHelperKind = "info";
      recipientHelperMsg = recipientNetwork.note;
    }
  }

  const amountFieldStyle: CSSProperties = {
    ...amountInputStyle,
    ...(amountHelperKind === "error" ? inputErrorStyle : {}),
    ...(amountHelperKind === "warn" ? inputWarnStyle : {}),
  };

  const recipientFieldStyle: CSSProperties = {
    ...inputStyle,
    ...(recipientHelperKind === "error" ? inputErrorStyle : {}),
    ...(recipientHelperKind === "warn" ? inputWarnStyle : {}),
  };

  return (
    <PanelShell eyebrow="Send" title="Send SOL">
      <div style={wrapStyle}>
        <div style={cardStyle}>
          <span style={labelStyle}>Token</span>
          <span
            style={{
              fontFamily: fontFamily.display,
              fontSize: font.lg,
              color: colors.text,
              letterSpacing: letterSpacing.loose,
            }}
          >
            SOL
          </span>
          <div style={availableRowStyle}>
            <span>Available</span>
            <span>
              {balanceLoaded
                ? `${formatSol(wallet.solUiAmount)} SOL`
                : "Loading balance…"}
            </span>
          </div>
        </div>

        {wallet.balanceError ? (
          <ErrorBanner
            title="Balance unavailable"
            message={wallet.balanceError}
            onRetry={wallet.refreshBalance}
            retrying={wallet.balanceLoading}
          />
        ) : null}

        <label style={{ display: "flex", flexDirection: "column", gap: space.xs }}>
          <span style={labelStyle}>Amount</span>
          <input
            value={amount}
            onChange={(event) => setAmount(event.target.value)}
            inputMode="decimal"
            placeholder="0.00"
            style={amountFieldStyle}
            disabled={!balanceLoaded}
          />
          <div style={availableRowStyle}>
            <span style={helperStyle(amountHelperKind)}>{amountHelperMsg || " "}</span>
            <button
              type="button"
              onClick={applyMax}
              disabled={!balanceLoaded}
              style={balanceLoaded ? maxBtnStyle : maxBtnDisabledStyle}
            >
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
            style={recipientFieldStyle}
          />
          <span style={helperStyle(recipientHelperKind)}>
            {recipientHelperMsg || " "}
          </span>
        </label>

        <div style={{ display: "flex", flexDirection: "column", alignItems: "center", gap: space.xs, marginTop: space.sm }}>
          <PrimaryButton onClick={advance} disabled={!canReview}>
            Review
          </PrimaryButton>
          <LinkButton onClick={() => nav.back()}>Cancel</LinkButton>
        </div>
      </div>
    </PanelShell>
  );
}
