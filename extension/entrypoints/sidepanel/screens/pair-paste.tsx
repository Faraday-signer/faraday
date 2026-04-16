import { useMemo, useState, type CSSProperties } from "react";

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
  const [error, setError] = useState<string | null>(null);
  const [saving, setSaving] = useState(false);

  const trimmed = value.trim();
  const looksValid = useMemo(() => isValidSolanaAddress(trimmed), [trimmed]);

  async function confirm() {
    if (!looksValid) {
      setError("Not a valid Solana address.");
      return;
    }
    setSaving(true);
    const r = await sendRuntimeMessage<ExtensionState>({
      type: "faraday:set-paired-pubkey",
      pubkey: trimmed
    });
    setSaving(false);
    if (r.ok) {
      nav.reset({ name: "home" });
    } else {
      setError(r.error);
    }
  }

  return (
    <PanelShell eyebrow="Pair Device" title="Paste address">
      <div style={wrapStyle}>
        <label style={{ display: "flex", flexDirection: "column", gap: space.xs }}>
          <span style={labelStyle}>Solana public key</span>
          <input
            autoFocus
            spellCheck={false}
            placeholder="Paste your Faraday device address"
            value={value}
            onChange={(event) => {
              setValue(event.target.value);
              if (error) setError(null);
            }}
            style={inputStyle}
          />
          <span style={helperStyle(!!error)}>
            {error ?? (value.length > 0 && !looksValid ? "Not a valid Solana address." : "Base58 address from your device.")}
          </span>
        </label>

        <div style={{ display: "flex", justifyContent: "center" }}>
          <PrimaryButton onClick={confirm} disabled={!looksValid || saving}>
            Pair this address
          </PrimaryButton>
        </div>

        <div style={{ display: "flex", justifyContent: "center", marginTop: space.xs }}>
          <LinkButton onClick={() => nav.replace({ name: "pair-scan" })}>Scan QR instead</LinkButton>
        </div>
      </div>
    </PanelShell>
  );
}
