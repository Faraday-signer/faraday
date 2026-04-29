import { useEffect, useState, type CSSProperties } from "react";

import { ErrorBanner } from "@/components/error-banner";
import { LinkButton, PanelShell, SecondaryButton } from "@/components/panel-shell";
import { sendRuntimeMessage } from "@/lib/runtime";
import type { ExtensionState } from "@/lib/types";
import { colors, fontFamily, font, letterSpacing, radius, space } from "@/lib/theme";

const wrapStyle: CSSProperties = {
  display: "flex",
  flexDirection: "column",
  padding: space.md,
  gap: space.sm
};

const rowStyle: CSSProperties = {
  display: "flex",
  alignItems: "center",
  justifyContent: "space-between",
  gap: space.sm,
  padding: `${space.xs}px ${space.sm}px`,
  borderRadius: radius.md,
  background: colors.panel,
  border: `1px solid ${colors.border}`
};

const originStyle: CSSProperties = {
  fontFamily: fontFamily.mono,
  fontSize: font.xs,
  color: colors.text,
  overflow: "hidden",
  textOverflow: "ellipsis",
  whiteSpace: "nowrap",
  flex: 1
};

const emptyStyle: CSSProperties = {
  textAlign: "center",
  color: colors.textMuted,
  fontSize: font.sm,
  padding: space.lg,
  border: `1px dashed ${colors.border}`,
  borderRadius: radius.md,
  background: colors.panel
};

export function SettingsOriginsScreen() {
  const [origins, setOrigins] = useState<string[]>([]);
  const [loadError, setLoadError] = useState<string | null>(null);
  const [mutationError, setMutationError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  async function refresh() {
    setLoadError(null);
    const r = await sendRuntimeMessage<ExtensionState>({ type: "faraday:get-state" });
    if (r.ok) {
      setOrigins(r.data.approvedOrigins);
    } else {
      setLoadError(r.error);
    }
  }

  useEffect(() => {
    void refresh();
  }, []);

  async function revoke(origin: string) {
    setBusy(true);
    setMutationError(null);
    const r = await sendRuntimeMessage<ExtensionState>({ type: "faraday:revoke-origin", origin });
    setBusy(false);
    if (r.ok) {
      setOrigins(r.data.approvedOrigins);
    } else {
      setMutationError(`Could not revoke ${origin}: ${r.error}`);
    }
  }

  async function clearAll() {
    setBusy(true);
    setMutationError(null);
    const r = await sendRuntimeMessage<ExtensionState>({ type: "faraday:clear-approved-origins" });
    setBusy(false);
    if (r.ok) {
      setOrigins(r.data.approvedOrigins);
    } else {
      setMutationError(`Could not clear approvals: ${r.error}`);
    }
  }

  const errorBanner =
    loadError || mutationError ? (
      <>
        {loadError ? (
          <ErrorBanner
            title="Load failed"
            message={loadError}
            onRetry={() => void refresh()}
          />
        ) : null}
        {mutationError ? (
          <ErrorBanner
            message={mutationError}
            onDismiss={() => setMutationError(null)}
          />
        ) : null}
      </>
    ) : null;

  return (
    <PanelShell eyebrow="Settings" title="Approved sites" banner={errorBanner}>
      <div style={wrapStyle}>
        {!loadError && origins.length === 0 ? (
          <div style={emptyStyle}>
            No sites approved yet. Dapps ask for access the first time they call connect.
          </div>
        ) : null}

        {origins.length > 0 ? (
          <>
            {origins.map((origin) => (
              <div key={origin} style={rowStyle}>
                <span style={originStyle}>{origin}</span>
                <LinkButton
                  onClick={() => revoke(origin)}
                  disabled={busy}
                  style={{
                    color: colors.error,
                    textDecoration: "none",
                    fontFamily: fontFamily.display,
                    letterSpacing: letterSpacing.loose,
                    textTransform: "uppercase",
                    fontSize: font.xs
                  }}
                >
                  Revoke
                </LinkButton>
              </div>
            ))}

            <div style={{ display: "flex", justifyContent: "center", marginTop: space.sm }}>
              <SecondaryButton onClick={clearAll} disabled={busy}>
                Clear all
              </SecondaryButton>
            </div>
          </>
        ) : null}
      </div>
    </PanelShell>
  );
}
