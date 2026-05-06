import { StyleSheet, Text, View } from "react-native";

import { colors, font, letterSpacing, radius, space } from "../lib/theme";
import { deriveTxSteps } from "../lib/tx-steps";
import type { TxRiskReport } from "../lib/tx-risk";

interface Props {
  report: TxRiskReport;
}

export function WhatWillHappen({ report }: Props) {
  const steps = deriveTxSteps(report);

  return (
    <View style={styles.box}>
      <Text style={styles.label}>What will happen</Text>
      {steps.map((step, idx) => (
        <View key={idx} style={styles.row}>
          <Text style={styles.bullet}>•</Text>
          <Text style={styles.step}>{step}</Text>
        </View>
      ))}
    </View>
  );
}

const styles = StyleSheet.create({
  box: {
    padding: space.md,
    borderRadius: radius.md,
    borderWidth: 1,
    borderColor: colors.border,
    backgroundColor: colors.panel,
    gap: space.xs
  },
  label: {
    color: colors.textMuted,
    fontSize: font.xs,
    letterSpacing: letterSpacing.eyebrow,
    textTransform: "uppercase",
    marginBottom: 2
  },
  row: {
    flexDirection: "row",
    gap: space.xs,
    alignItems: "flex-start"
  },
  bullet: {
    color: colors.accent,
    fontSize: font.sm,
    lineHeight: 20
  },
  step: {
    color: colors.text,
    fontSize: font.sm,
    flex: 1,
    lineHeight: 20
  }
});
