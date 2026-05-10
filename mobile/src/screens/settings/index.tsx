import { Pressable, StyleSheet, Text, View } from "react-native";
import type { NativeStackScreenProps } from "@react-navigation/native-stack";

import { ScreenShell } from "../../components/screen-shell";
import { colors, font, letterSpacing, radius, space } from "../../lib/theme";
import type { SettingsStackParamList } from "../../navigation/settings";

type Props = NativeStackScreenProps<SettingsStackParamList, "SettingsHome">;

interface Row {
  key: keyof SettingsStackParamList;
  label: string;
  hint: string;
}

const ROWS: Row[] = [
  { key: "Device", label: "Device", hint: "Paired pubkey and unpair" },
  { key: "Network", label: "Network", hint: "RPC endpoint and tokens" },
  { key: "About", label: "About", hint: "Version and source" }
];

export function SettingsHomeScreen({ navigation }: Props) {
  return (
    <ScreenShell eyebrow="Settings" title="Settings">
      <View style={styles.list}>
        {ROWS.map((row) => (
          <Pressable
            key={row.key}
            style={({ pressed }) => [styles.row, pressed && styles.rowPressed]}
            onPress={() => navigation.navigate(row.key as never)}
          >
            <Text style={styles.rowLabel}>{row.label}</Text>
            <Text style={styles.rowHint}>{row.hint}</Text>
          </Pressable>
        ))}
      </View>
    </ScreenShell>
  );
}

const styles = StyleSheet.create({
  list: {
    borderRadius: radius.lg,
    borderWidth: 1,
    borderColor: colors.border,
    backgroundColor: colors.panel,
    overflow: "hidden"
  },
  row: {
    paddingHorizontal: space.md,
    paddingVertical: space.md,
    borderBottomWidth: 1,
    borderBottomColor: colors.border,
    gap: 2
  },
  rowPressed: {
    backgroundColor: colors.panelHi
  },
  rowLabel: {
    color: colors.text,
    fontSize: font.md,
    letterSpacing: letterSpacing.loose
  },
  rowHint: {
    color: colors.textDim,
    fontSize: font.xs
  }
});
