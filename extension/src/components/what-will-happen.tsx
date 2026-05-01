//! "What Will Happen" — plain-English bullets above the risk view.
//!
//! Inspired by SolDecode's drawer pattern. Putting this above the risk
//! warnings changes the UX from "trust the alarm" to "understand what
//! this tx does, then decide if the warnings worry you."
//!
//! Renders nothing when there are no steps to show (extremely rare —
//! `deriveTxSteps` always emits at least one fallback line).

import type { CSSProperties } from "react";

import { colors, fontFamily, font, letterSpacing, radius, space } from "../lib/theme";
import { deriveTxSteps } from "../lib/tx-steps";
import type { TxRiskReport } from "../lib/tx-risk";

interface WhatWillHappenProps {
  report: TxRiskReport;
  style?: CSSProperties;
}

const wrapStyle: CSSProperties = {
  padding: space.sm,
  borderRadius: radius.md,
  background: colors.panel,
  border: `1px solid ${colors.borderStrong}`,
  display: "flex",
  flexDirection: "column",
  gap: space.xxs,
};

const eyebrowStyle: CSSProperties = {
  fontFamily: fontFamily.display,
  fontSize: font.xs,
  letterSpacing: letterSpacing.eyebrow,
  textTransform: "uppercase",
  color: colors.accent,
};

const listStyle: CSSProperties = {
  margin: 0,
  paddingLeft: space.lg,
  display: "flex",
  flexDirection: "column",
  gap: 2,
};

const stepStyle: CSSProperties = {
  fontFamily: fontFamily.ui,
  fontSize: font.sm,
  color: colors.text,
  lineHeight: 1.4,
};

export function WhatWillHappen({ report, style }: WhatWillHappenProps) {
  const steps = deriveTxSteps(report);
  if (steps.length === 0) return null;

  return (
    <div style={{ ...wrapStyle, ...style }}>
      <span style={eyebrowStyle}>What will happen</span>
      <ul style={listStyle}>
        {steps.map((step, i) => (
          <li key={i} style={stepStyle}>
            {step}
          </li>
        ))}
      </ul>
    </div>
  );
}
