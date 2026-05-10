import type { ReactNode } from "react";
import { ScrollView, StyleSheet, Text, View } from "react-native";
import { SafeAreaView } from "react-native-safe-area-context";

import { colors, font, fontFamily, letterSpacing, space } from "../lib/theme";

interface ScreenShellProps {
  eyebrow?: string;
  title: string;
  children: ReactNode;
  scroll?: boolean;
}

export function ScreenShell({ eyebrow, title, children, scroll = true }: ScreenShellProps) {
  const body = scroll ? (
    <ScrollView
      style={styles.scroll}
      contentContainerStyle={styles.scrollContent}
      showsVerticalScrollIndicator={false}
    >
      {children}
    </ScrollView>
  ) : (
    <View style={styles.scrollContent}>{children}</View>
  );

  return (
    <SafeAreaView style={styles.root} edges={["top", "left", "right"]}>
      <View style={styles.header}>
        {eyebrow ? <Text style={styles.eyebrow}>{eyebrow}</Text> : null}
        <Text style={styles.title}>{title}</Text>
      </View>
      {body}
    </SafeAreaView>
  );
}

const styles = StyleSheet.create({
  root: {
    flex: 1,
    backgroundColor: colors.bg
  },
  header: {
    paddingHorizontal: space.md,
    paddingTop: space.md,
    paddingBottom: space.sm,
    gap: 2,
    borderBottomWidth: 1,
    borderBottomColor: colors.border
  },
  eyebrow: {
    color: colors.textMuted,
    fontSize: font.xs,
    fontFamily: fontFamily.display,
    letterSpacing: letterSpacing.eyebrow,
    textTransform: "uppercase"
  },
  title: {
    color: colors.text,
    fontSize: font.xxl,
    fontFamily: fontFamily.display
  },
  scroll: {
    flex: 1
  },
  scrollContent: {
    padding: space.md,
    gap: space.md
  }
});
