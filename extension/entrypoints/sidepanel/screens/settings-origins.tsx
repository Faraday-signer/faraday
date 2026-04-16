import { useEffect, useState, type CSSProperties } from "react";

import { LinkButton, PanelShell, SecondaryButton } from "../../../src/components/panel-shell";
import { sendRuntimeMessage } from "../../../src/lib/runtime";
import type { ExtensionState } from "../../../src/lib/types";
import { colors, fontFamily, font, letterSpacing, radius, space } from "../../../src/lib/theme";

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

  async function refresh() {
    const r = await sendRuntimeMessage<ExtensionState>({ type: "faraday:get-state" });
    if (r.ok) setOrigins(r.data.approvedOrigins);
  }

  useEffect(() => {
    void refresh();
  }, []);

  async function revoke(origin: string) {
    const r = await sendRuntimeMessage<ExtensionState>({ type: "faraday:revoke-origin", origin });
    if (r.ok) setOrigins(r.data.approvedOrigins);
  }

  async function clearAll() {
    const r = await sendRuntimeMessage<ExtensionState>({ type: "faraday:clear-approved-origins" });
    if (r.ok) setOrigins(r.data.approvedOrigins);
  }

  return (
    <PanelShell eyebrow="Settings" title="Approved sites">
      <div style={wrapStyle}>
        {origins.length === 0 ? (
          <div style={emptyStyle}>
            No sites approved yet. Dapps ask for access the first time they call connect.
          </div>
        ) : (
          <>
            {origins.map((origin) => (
              <div key={origin} style={rowStyle}>
                <span style={originStyle}>{origin}</span>
                <LinkButton
                  onClick={() => revoke(origin)}
                  style={{ color: colors.error, textDecoration: "none", fontFamily: fontFamily.display, letterSpacing: letterSpacing.loose, textTransform: "uppercase", fontSize: font.xs }}
                >
                  Revoke
                </LinkButton>
              </div>
            ))}

            <div style={{ display: "flex", justifyContent: "center", marginTop: space.sm }}>
              <SecondaryButton onClick={clearAll}>Clear all</SecondaryButton>
            </div>
          </>
        )}
      </div>
    </PanelShell>
  );
}
