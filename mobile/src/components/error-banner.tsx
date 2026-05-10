import { useState, type ReactNode } from "react";
import { Pressable, StyleSheet, Text, View } from "react-native";

import { colors, font, letterSpacing, radius, space } from "../lib/theme";

export type ErrorTone = "error" | "warning";

interface Props {
  title?: ReactNode;
  message: ReactNode;
  details?: string;
  onRetry?: () => void;
  retrying?: boolean;
  onDismiss?: () => void;
  tone?: ErrorTone;
}

const toneMap: Record<ErrorTone, { fg: string; bg: string; border: string }> = {
  error: { fg: colors.error, bg: "rgba(255, 107, 107, 0.12)", border: colors.error },
  warning: { fg: colors.warning, bg: "rgba(255, 180, 84, 0.12)", border: colors.warning }
};

export function ErrorBanner({
  title,
  message,
  details,
  onRetry,
  retrying,
  onDismiss,
  tone = "error"
}: Props) {
  const [open, setOpen] = useState(false);
  const t = toneMap[tone];

  return (
    <View style={[styles.box, { backgroundColor: t.bg, borderColor: t.border }]}>
      <View style={styles.headRow}>
        <View style={{ flex: 1 }}>
          {title ? <Text style={[styles.title, { color: t.fg }]}>{title}</Text> : null}
          <Text style={[styles.message, { color: t.fg }]}>{message}</Text>
        </View>
        {onDismiss ? (
          <Pressable onPress={onDismiss} hitSlop={10}>
            <Text style={[styles.dismiss, { color: t.fg }]}>×</Text>
          </Pressable>
        ) : null}
      </View>
      {details ? (
        <Pressable onPress={() => setOpen((v) => !v)}>
          <Text style={[styles.toggle, { color: t.fg }]}>
            {open ? "Hide details" : "Show details"}
          </Text>
        </Pressable>
      ) : null}
      {open && details ? (
        <View style={styles.detailsBox}>
          <Text style={styles.detailsText}>{details}</Text>
        </View>
      ) : null}
      {onRetry ? (
        <Pressable
          onPress={onRetry}
          disabled={!!retrying}
          style={({ pressed }) => [
            styles.retry,
            { borderColor: t.fg },
            pressed && styles.retryPressed,
            retrying && styles.disabled
          ]}
        >
          <Text style={[styles.retryLabel, { color: t.fg }]}>
            {retrying ? "Retrying…" : "Retry"}
          </Text>
        </Pressable>
      ) : null}
    </View>
  );
}

const styles = StyleSheet.create({
  box: {
    padding: space.sm,
    borderRadius: radius.md,
    borderWidth: 1,
    gap: space.xs
  },
  headRow: {
    flexDirection: "row",
    alignItems: "flex-start",
    gap: space.sm
  },
  title: {
    fontSize: font.sm,
    fontWeight: "600",
    letterSpacing: letterSpacing.loose,
    textTransform: "uppercase"
  },
  message: {
    fontSize: font.sm,
    lineHeight: 20
  },
  dismiss: {
    fontSize: font.xl,
    lineHeight: font.xl
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
  },
  retry: {
    paddingVertical: space.xs,
    paddingHorizontal: space.sm,
    borderRadius: radius.pill,
    borderWidth: 1,
    alignSelf: "flex-start"
  },
  retryPressed: {
    opacity: 0.7
  },
  retryLabel: {
    fontSize: font.xs,
    fontWeight: "600",
    letterSpacing: letterSpacing.loose,
    textTransform: "uppercase"
  },
  disabled: {
    opacity: 0.6
  }
});
