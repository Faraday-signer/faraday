import * as Clipboard from "expo-clipboard";
import { useCallback, useState } from "react";
import { Pressable, StyleSheet, Text, TextInput, View } from "react-native";
import type { NativeStackScreenProps } from "@react-navigation/native-stack";

import { address as toAddress } from "@solana/kit";

import { ScreenShell } from "../components/screen-shell";
import { useAppState } from "../lib/app-state";
import { colors, font, letterSpacing, radius, space } from "../lib/theme";
import type { RootStackParamList } from "../navigation/root";

type Props = NativeStackScreenProps<RootStackParamList, "PairPaste">;

export function PairPasteScreen({ navigation }: Props) {
  const { setPairedPubkey } = useAppState();
  const [value, setValue] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  const onPaste = useCallback(async () => {
    try {
      const text = await Clipboard.getStringAsync();
      if (text) setValue(text.trim());
    } catch {
      // ignore
    }
  }, []);

  const onSubmit = useCallback(async () => {
    setError(null);
    const trimmed = value.trim();
    if (!trimmed) {
      setError("Address is empty.");
      return;
    }
    try {
      toAddress(trimmed);
    } catch {
      setError("Not a valid Solana address.");
      return;
    }
    setBusy(true);
    try {
      await setPairedPubkey(trimmed);
      navigation.popToTop();
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
      setBusy(false);
    }
  }, [navigation, setPairedPubkey, value]);

  return (
    <ScreenShell eyebrow="Pair" title="Paste address">
      <View style={styles.card}>
        <Text style={styles.label}>Faraday pubkey</Text>
        <TextInput
          value={value}
          onChangeText={setValue}
          placeholder="Solana address"
          placeholderTextColor={colors.textDim}
          autoCapitalize="none"
          autoCorrect={false}
          spellCheck={false}
          style={styles.input}
          multiline
        />
        <Pressable onPress={onPaste} style={({ pressed }) => [styles.ghost, pressed && styles.ghostPressed]}>
          <Text style={styles.ghostLabel}>Paste from clipboard</Text>
        </Pressable>
      </View>

      {error ? <Text style={styles.error}>{error}</Text> : null}

      <Pressable
        onPress={() => {
          void onSubmit();
        }}
        disabled={busy}
        style={({ pressed }) => [
          styles.primary,
          (busy || pressed) && styles.primaryPressed,
          busy && styles.disabled
        ]}
      >
        <Text style={styles.primaryLabel}>{busy ? "Saving…" : "Pair"}</Text>
      </Pressable>
    </ScreenShell>
  );
}

const styles = StyleSheet.create({
  card: {
    padding: space.md,
    borderRadius: radius.lg,
    borderWidth: 1,
    borderColor: colors.border,
    backgroundColor: colors.panel,
    gap: space.sm
  },
  label: {
    color: colors.textMuted,
    fontSize: font.xs,
    letterSpacing: letterSpacing.eyebrow,
    textTransform: "uppercase"
  },
  input: {
    color: colors.text,
    fontSize: font.sm,
    fontFamily: "monospace",
    minHeight: 64,
    padding: space.sm,
    borderRadius: radius.md,
    borderWidth: 1,
    borderColor: colors.borderStrong,
    backgroundColor: colors.bg,
    textAlignVertical: "top"
  },
  ghost: {
    paddingVertical: space.xs,
    alignItems: "center"
  },
  ghostPressed: {
    opacity: 0.6
  },
  ghostLabel: {
    color: colors.accent,
    fontSize: font.sm,
    letterSpacing: letterSpacing.loose
  },
  primary: {
    paddingVertical: space.sm,
    paddingHorizontal: space.lg,
    borderRadius: radius.pill,
    backgroundColor: colors.accent,
    alignItems: "center"
  },
  primaryPressed: {
    backgroundColor: colors.accentStrong
  },
  primaryLabel: {
    color: colors.bg,
    fontSize: font.sm,
    fontWeight: "600",
    letterSpacing: letterSpacing.loose
  },
  disabled: {
    opacity: 0.6
  },
  error: {
    color: colors.error,
    fontSize: font.sm,
    paddingHorizontal: space.xxs
  }
});
