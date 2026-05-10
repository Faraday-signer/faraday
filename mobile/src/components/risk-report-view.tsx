import { useState } from "react";
import { Pressable, StyleSheet, Text, View } from "react-native";

import { colors, font, letterSpacing, radius, space } from "../lib/theme";
import {
  balanceChanges,
  categoryLabel,
  formatBalanceAmount,
  riskLevelColor,
  riskLevelLabel,
  severityColor,
  severitySigil,
  sortWarningsBySeverity
} from "../lib/risk-display";
import type { TxRiskReport, TxRiskWarning } from "../lib/tx-risk";

interface Props {
  report: TxRiskReport;
}

export function RiskReportView({ report }: Props) {
  const sorted = sortWarningsBySeverity(report.warnings);
  const banner = riskLevelLabel(report.level, report.warnings);
  const bannerColor = riskLevelColor(report.level);
  const changes = balanceChanges(report);

  return (
    <View style={styles.wrap}>
      <View style={[styles.banner, { borderColor: bannerColor }]}>
        <Text style={[styles.bannerText, { color: bannerColor }]}>{banner}</Text>
      </View>

      {changes.length > 0 ? (
        <View style={styles.card}>
          <Text style={styles.label}>Balance changes</Text>
          {changes.map((c) => (
            <View key={`${c.symbol}-${c.mint}`} style={styles.changeRow}>
              <Text style={styles.changeSymbol}>{c.symbol}</Text>
              <Text
                style={[
                  styles.changeAmount,
                  { color: c.amount >= 0 ? colors.success : colors.error }
                ]}
              >
                {formatBalanceAmount(c.amount)}
              </Text>
            </View>
          ))}
        </View>
      ) : null}

      {sorted.length > 0 ? (
        <View style={{ gap: space.sm }}>
          {sorted.map((w, idx) => (
            <WarningRow key={idx} warning={w} />
          ))}
        </View>
      ) : null}
    </View>
  );
}

function WarningRow({ warning }: { warning: TxRiskWarning }) {
  const [showDetails, setShowDetails] = useState(false);
  const accent = severityColor(warning.severity);
  const sigil = severitySigil(warning.severity);

  return (
    <View style={[styles.warningCard, { borderColor: accent }]}>
      <View style={styles.warningHead}>
        <Text style={[styles.warningSigil, { color: accent }]}>{sigil}</Text>
        <Text style={styles.warningTitle}>{warning.title}</Text>
        <View style={[styles.warningChip, { borderColor: accent }]}>
          <Text style={[styles.warningChipText, { color: accent }]}>
            {categoryLabel(warning.category)}
          </Text>
        </View>
      </View>
      <Text style={styles.warningDescription}>{warning.description}</Text>
      <Text style={styles.warningExplanation}>{warning.explanation}</Text>
      {warning.details ? (
        <Pressable onPress={() => setShowDetails((v) => !v)}>
          <Text style={[styles.toggle, { color: accent }]}>
            {showDetails ? "Hide details" : "Show details"}
          </Text>
        </Pressable>
      ) : null}
      {showDetails && warning.details ? (
        <View style={styles.detailsBox}>
          <Text style={styles.detailsText}>{warning.details}</Text>
        </View>
      ) : null}
    </View>
  );
}

const styles = StyleSheet.create({
  wrap: {
    gap: space.sm
  },
  banner: {
    padding: space.sm,
    borderRadius: radius.md,
    borderWidth: 1,
    backgroundColor: colors.panel,
    alignItems: "center"
  },
  bannerText: {
    fontSize: font.sm,
    fontWeight: "600",
    letterSpacing: letterSpacing.loose
  },
  card: {
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
  changeRow: {
    flexDirection: "row",
    justifyContent: "space-between",
    paddingVertical: 4
  },
  changeSymbol: {
    color: colors.text,
    fontSize: font.sm,
    fontWeight: "600"
  },
  changeAmount: {
    fontSize: font.sm,
    fontFamily: "monospace"
  },
  warningCard: {
    padding: space.md,
    borderRadius: radius.md,
    borderWidth: 1,
    backgroundColor: colors.panel,
    gap: space.xs
  },
  warningHead: {
    flexDirection: "row",
    alignItems: "center",
    gap: space.xs
  },
  warningSigil: {
    fontSize: font.lg,
    fontWeight: "600"
  },
  warningTitle: {
    color: colors.text,
    fontSize: font.sm,
    fontWeight: "600",
    flex: 1
  },
  warningChip: {
    paddingHorizontal: space.xs,
    paddingVertical: 2,
    borderRadius: radius.pill,
    borderWidth: 1
  },
  warningChipText: {
    fontSize: 10,
    letterSpacing: letterSpacing.eyebrow,
    textTransform: "uppercase"
  },
  warningDescription: {
    color: colors.text,
    fontSize: font.sm,
    lineHeight: 20
  },
  warningExplanation: {
    color: colors.textMuted,
    fontSize: font.xs,
    lineHeight: 18
  },
  toggle: {
    fontSize: font.xs,
    letterSpacing: letterSpacing.loose,
    textTransform: "uppercase"
  },
  detailsBox: {
    backgroundColor: colors.bg,
    padding: space.sm,
    borderRadius: radius.sm,
    maxHeight: 200
  },
  detailsText: {
    color: colors.textMuted,
    fontFamily: "monospace",
    fontSize: font.xs,
    lineHeight: 16
  }
});
