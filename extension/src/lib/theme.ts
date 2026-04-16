export const colors = {
  bg: "#001721",
  panel: "#002536",
  panelHi: "#003246",
  border: "#0A2836",
  borderStrong: "#114358",
  text: "#E7E7E7",
  textMuted: "#8C9CA8",
  textDim: "#5E7180",
  accent: "#1AF8FF",
  accentSoft: "rgba(26, 248, 255, 0.12)",
  accentStrong: "#00DFE6",
  success: "#4ADE80",
  warning: "#FFB454",
  error: "#FF6B6B",
  qrSurface: "#FFFFFF",
  qrModule: "#000000"
} as const;

/**
 * Per-flow accent colors for branded QR frames. Frame + header carry the
 * brand; QR modules themselves stay pure black-on-white for scan reliability.
 */
export const flowColors = {
  sign: {
    primary: "#1AF8FF",
    soft: "rgba(26, 248, 255, 0.14)",
    onPrimary: "#001721",
    label: "SIGN TRANSACTION"
  },
  pair: {
    primary: "#FF6BE1",
    soft: "rgba(255, 107, 225, 0.14)",
    onPrimary: "#001721",
    label: "PAIR DEVICE"
  },
  receive: {
    primary: "#4ADE80",
    soft: "rgba(74, 222, 128, 0.14)",
    onPrimary: "#001721",
    label: "RECEIVE"
  },
  signed: {
    primary: "#E7E7E7",
    soft: "rgba(231, 231, 231, 0.1)",
    onPrimary: "#001721",
    label: "SIGNED"
  }
} as const;

export type FlowKind = keyof typeof flowColors;

export const radius = {
  sm: 6,
  md: 10,
  lg: 14,
  xl: 20,
  pill: 999
} as const;

export const space = {
  xxs: 4,
  xs: 8,
  sm: 12,
  md: 16,
  lg: 20,
  xl: 28,
  xxl: 40,
  xxxl: 56
} as const;

export const fontFamily = {
  ui: "ui-sans-serif, system-ui, -apple-system, 'Segoe UI', sans-serif",
  mono: "ui-monospace, SFMono-Regular, Menlo, 'Roboto Mono', monospace",
  display: "'Departure Mono', ui-monospace, SFMono-Regular, Menlo, monospace"
} as const;

export const font = {
  xs: 11,
  sm: 12,
  md: 13,
  lg: 15,
  xl: 18,
  xxl: 22,
  display: 26,
  hero: 36
} as const;

export const letterSpacing = {
  tight: -0.2,
  normal: 0,
  loose: 0.3,
  wider: 0.8,
  eyebrow: 1.6
} as const;

export const zIndex = {
  base: 1,
  overlay: 10,
  modal: 100,
  toast: 1000
} as const;
