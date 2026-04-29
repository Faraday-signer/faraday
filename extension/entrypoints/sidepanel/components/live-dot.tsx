import type { CSSProperties } from "react";

import type { LiveConnectionState } from "@/lib/use-live-balance";
import { colors } from "@/lib/theme";

export function LiveDot({ state }: { state: LiveConnectionState }) {
  const { color, title, pulse } = (() => {
    switch (state) {
      case "live":
        return { color: colors.success, title: "Live", pulse: true };
      case "connecting":
        return { color: colors.accent, title: "Connecting", pulse: true };
      case "reconnecting":
        return { color: colors.warning, title: "Reconnecting", pulse: true };
      case "failed":
        return { color: colors.error, title: "Live connection unavailable", pulse: false };
      default:
        return { color: colors.textDim, title: "Idle", pulse: false };
    }
  })();

  const style: CSSProperties = {
    display: "inline-block",
    width: 6,
    height: 6,
    borderRadius: "50%",
    background: color,
    boxShadow: pulse ? `0 0 0 2px ${color}33` : "none",
    animation: pulse ? "faraday-pulse 1.6s ease-in-out infinite" : "none",
  };

  return <span aria-label={title} title={title} style={style} />;
}
