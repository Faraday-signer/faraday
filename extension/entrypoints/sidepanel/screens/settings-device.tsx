import { useEffect, useState, type CSSProperties } from "react";

import { LinkButton, PanelShell } from "@/components/panel-shell";
import { useNavigation } from "@/lib/router";
import { sendRuntimeMessage } from "@/lib/runtime";
import type { ExtensionState } from "@/lib/types";
import { colors, fontFamily, font, letterSpacing, radius, space } from "@/lib/theme";

const wrapStyle: CSSProperties = {
  display: "flex",
  flexDirection: "column",
  padding: space.md,
  gap: space.md
};

const cardStyle: CSSProperties = {
  padding: space.md,
  borderRadius: radius.md,
  background: colors.panel,
  border: `1px solid ${colors.border}`,
  display: "flex",
  flexDirection: "column",
  gap: space.xs
};

const labelStyle: CSSProperties = {
  fontFamily: fontFamily.display,
  fontSize: font.xs,
  letterSpacing: letterSpacing.eyebrow,
  textTransform: "uppercase",
  color: colors.textMuted
};

const valueStyle: CSSProperties = {
  fontFamily: fontFamily.mono,
  fontSize: font.sm,
  color: colors.accent,
  wordBreak: "break-all"
};

const hintStyle: CSSProperties = {
  fontSize: font.xs,
  color: colors.textDim,
  textAlign: "center",
  lineHeight: 1.5,
  marginTop: space.xs
};

export function SettingsDeviceScreen() {
  const nav = useNavigation();
  const [pubkey, setPubkey] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let cancelled = false;
    void (async () => {
      const r = await sendRuntimeMessage<ExtensionState>({ type: "faraday:get-state" });
      if (cancelled) return;
      if (r.ok) {
        setPubkey(r.data.pairedPubkey);
      } else {
        setError(r.error);
      }
      setLoading(false);
    })();
    return () => {
      cancelled = true;
    };
  }, []);

  if (loading) {
    return (
      <PanelShell eyebrow="Settings" title="Connected wallet">
        <div style={wrapStyle}>
          <p style={{ color: colors.textMuted, fontSize: font.sm }}>Loading…</p>
        </div>
      </PanelShell>
    );
  }

  if (error) {
    return (
      <PanelShell eyebrow="Settings" title="Connected wallet">
        <div style={wrapStyle}>
          <div style={{ ...cardStyle, borderColor: colors.error }}>
            <span style={{ color: colors.error, fontSize: font.sm }}>{error}</span>
          </div>
        </div>
      </PanelShell>
    );
  }

  return (
    <PanelShell eyebrow="Settings" title="Connected wallet">
      <div style={wrapStyle}>
        {pubkey ? (
          <>
            <div style={cardStyle}>
              <span style={labelStyle}>Public key</span>
              <span style={valueStyle}>{pubkey}</span>
            </div>
            <p style={hintStyle}>
              To disconnect this wallet, use the Disconnect button on the Settings screen.
            </p>
          </>
        ) : (
          <div style={cardStyle}>
            <span style={{ fontSize: font.sm, color: colors.textMuted }}>No wallet connected yet.</span>
            <LinkButton onClick={() => nav.reset({ name: "onboarding" })}>Import wallet</LinkButton>
          </div>
        )}
      </div>
    </PanelShell>
  );
}
