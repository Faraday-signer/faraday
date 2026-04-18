import { useMemo, useState, type CSSProperties } from "react";

import { ErrorBanner } from "../../../src/components/error-banner";
import { LinkButton, PanelShell, PrimaryButton } from "../../../src/components/panel-shell";
import { useNavigation } from "../../../src/lib/router";
import { sendRuntimeMessage } from "../../../src/lib/runtime";
import { isValidSolanaAddress } from "../../../src/lib/solana";
import type { ExtensionState } from "../../../src/lib/types";
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
  outline: "none",
  letterSpacing: 0.2
};

const helperStyle = (isError: boolean): CSSProperties => ({
  fontSize: font.xs,
  color: isError ? colors.error : colors.textDim,
  minHeight: 16
});

export function PairPasteScreen() {
  const nav = useNavigation();
  const [value, setValue] = useState("");
  const [mutationError, setMutationError] = useState<string | null>(null);
  const [saving, setSaving] = useState(false);

  const trimmed = value.trim();
  const looksValid = useMemo(() => isValidSolanaAddress(trimmed), [trimmed]);
  const shouldHint = value.length > 0 && !looksValid;

  async function confirm() {
    if (!looksValid) return;
    setSaving(true);
    setMutationError(null);
    const r = await sendRuntimeMessage<ExtensionState>({
      type: "faraday:set-paired-pubkey",
      pubkey: trimmed
    });
    setSaving(false);
    if (r.ok) {
      nav.reset({ name: "home" });
    } else {
      setMutationError(r.error);
    }
  }

  return (
    <PanelShell eyebrow="Pair Device" title="Paste address">
      <div style={wrapStyle}>
        {mutationError ? (
          <ErrorBanner
            title="Pairing failed"
            message={mutationError}
            onRetry={confirm}
            retrying={saving}
            onDismiss={() => setMutationError(null)}
          />
        ) : null}

        <label style={{ display: "flex", flexDirection: "column", gap: space.xs }}>
          <span style={labelStyle}>Solana public key</span>
          <input
            autoFocus
            spellCheck={false}
            placeholder="Paste your Faraday device address"
            value={value}
            onChange={(event) => setValue(event.target.value)}
            style={inputStyle}
          />
          <span style={helperStyle(shouldHint)}>
            {shouldHint ? "Not a valid Solana address." : "Base58 address from your device."}
          </span>
        </label>

        <div style={{ display: "flex", justifyContent: "center" }}>
          <PrimaryButton onClick={confirm} disabled={!looksValid || saving}>
            {saving ? "Pairing…" : "Pair this address"}
          </PrimaryButton>
        </div>

        <div style={{ display: "flex", justifyContent: "center", marginTop: space.xs }}>
          <LinkButton onClick={() => nav.replace({ name: "pair-scan" })}>Scan QR instead</LinkButton>
        </div>
      </div>
    </PanelShell>
  );
}
