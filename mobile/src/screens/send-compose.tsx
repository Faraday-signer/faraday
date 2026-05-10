import * as Clipboard from "expo-clipboard";
import { useMemo, useState } from "react";
import { Pressable, ScrollView, StyleSheet, Text, TextInput, View } from "react-native";
import type { NativeStackScreenProps } from "@react-navigation/native-stack";

import { ScreenShell } from "../components/screen-shell";
import { useAppState } from "../lib/app-state";
import { formatTokenAmount, shortMint } from "../lib/token-format";
import {
  FEE_RESERVE_LAMPORTS,
  validateAmount,
  validateRecipientFormat,
  useRecipientCheck
} from "../lib/send-validation";
import { LAMPORTS_PER_SOL } from "../lib/sol-transfer";
import { colors, font, letterSpacing, radius, space } from "../lib/theme";
import { useTokens } from "../lib/use-tokens";
import { useWallet } from "../lib/use-wallet";
import type { Token } from "../lib/tokens";
import type { RootStackParamList } from "../navigation/root";

type Props = NativeStackScreenProps<RootStackParamList, "SendCompose">;

const SOL_KIND = "SOL" as const;
type SelectedTokenKey = typeof SOL_KIND | string;

export function SendComposeScreen({ navigation }: Props) {
  const { pairedPubkey } = useAppState();
  const wallet = useWallet();
  const { tokens } = useTokens(pairedPubkey);

  const [selected, setSelected] = useState<SelectedTokenKey>(SOL_KIND);
  const [amount, setAmount] = useState("");
  const [recipient, setRecipient] = useState("");

  const selectedToken: Token | null =
    selected === SOL_KIND ? null : tokens.find((t) => t.mint === selected) ?? null;

  const decimals = selectedToken ? selectedToken.decimals : 9;
  const symbol = selectedToken ? selectedToken.symbol || shortMint(selectedToken.mint) : "SOL";
  const balanceRaw =
    selected === SOL_KIND ? wallet.solLamports : selectedToken?.amountRaw ?? null;
  const feeReserve = selected === SOL_KIND ? FEE_RESERVE_LAMPORTS : 0n;

  const amountCheck = validateAmount({
    amountStr: amount,
    balanceRaw,
    decimals,
    symbol,
    feeReserve
  });
  const recipientFormat = validateRecipientFormat(recipient);
  const intendedRaw = amountCheck.kind === "ok" || amountCheck.kind === "warn" ? amountCheck.raw : null;
  const recipientCheck = useRecipientCheck(recipient, pairedPubkey, intendedRaw);

  const balanceUiText = useMemo(() => {
    if (balanceRaw === null) return "—";
    const num = Number(balanceRaw) / 10 ** decimals;
    return formatTokenAmount(num, decimals);
  }, [balanceRaw, decimals]);

  const canProceed =
    pairedPubkey !== null &&
    (amountCheck.kind === "ok" || amountCheck.kind === "warn") &&
    recipientFormat.kind === "ok" &&
    recipientCheck.kind !== "error" &&
    recipientCheck.kind !== "loading";

  const onPasteRecipient = async () => {
    const text = await Clipboard.getStringAsync();
    if (text) setRecipient(text.trim());
  };

  const onMax = () => {
    if (balanceRaw === null) return;
    const sendable = balanceRaw > feeReserve ? balanceRaw - feeReserve : 0n;
    if (sendable <= 0n) {
      setAmount("0");
      return;
    }
    if (selected === SOL_KIND) {
      const sol = Number(sendable) / Number(LAMPORTS_PER_SOL);
      setAmount(sol.toString());
    } else {
      const ui = Number(sendable) / 10 ** decimals;
      setAmount(ui.toString());
    }
  };

  const proceed = () => {
    if (!pairedPubkey || !canProceed) return;
    navigation.navigate("SendReview", {
      tokenKind: selected === SOL_KIND ? "sol" : "spl",
      mint: selected === SOL_KIND ? null : (selected as string),
      programId: selectedToken ? selectedToken.programId : null,
      decimals,
      symbol,
      recipient: recipient.trim(),
      amountStr: amount.trim()
    });
  };

  return (
    <ScreenShell eyebrow="Send" title={`Send ${symbol}`}>
      {/* Token picker */}
      <View style={styles.card}>
        <Text style={styles.label}>Token</Text>
        <ScrollView horizontal showsHorizontalScrollIndicator={false} contentContainerStyle={styles.tokenRow}>
          <TokenChip
            label="SOL"
            balance={
              wallet.solLamports !== null
                ? formatTokenAmount(Number(wallet.solLamports) / Number(LAMPORTS_PER_SOL), 9)
                : "—"
            }
            selected={selected === SOL_KIND}
            onPress={() => setSelected(SOL_KIND)}
          />
          {tokens.map((t) => (
            <TokenChip
              key={t.mint}
              label={t.symbol || shortMint(t.mint)}
              balance={formatTokenAmount(t.amountUi, t.decimals)}
              selected={selected === t.mint}
              onPress={() => setSelected(t.mint)}
            />
          ))}
        </ScrollView>
      </View>

      {/* Recipient */}
      <View style={styles.card}>
        <Text style={styles.label}>Recipient</Text>
        <TextInput
          style={styles.input}
          value={recipient}
          onChangeText={setRecipient}
          autoCapitalize="none"
          autoCorrect={false}
          spellCheck={false}
          placeholder="Solana address"
          placeholderTextColor={colors.textDim}
          multiline
        />
        <Pressable onPress={onPasteRecipient} style={styles.ghostButton}>
          <Text style={styles.ghostButtonLabel}>Paste from clipboard</Text>
        </Pressable>
        {recipientFormat.kind === "error" && recipient.length > 0 ? (
          <Text style={styles.errorText}>{recipientFormat.message}</Text>
        ) : null}
        {recipientCheck.kind === "warn" ? (
          <Text style={styles.warnText}>{recipientCheck.message}</Text>
        ) : null}
        {recipientCheck.kind === "error" ? (
          <Text style={styles.errorText}>{recipientCheck.message}</Text>
        ) : null}
        {recipientCheck.kind === "ok" && recipientCheck.note ? (
          <Text style={styles.muteText}>{recipientCheck.note}</Text>
        ) : null}
        {recipientCheck.kind === "loading" ? (
          <Text style={styles.muteText}>Checking recipient…</Text>
        ) : null}
      </View>

      {/* Amount */}
      <View style={styles.card}>
        <View style={styles.amountHead}>
          <Text style={styles.label}>Amount</Text>
          <Pressable onPress={onMax}>
            <Text style={styles.maxLabel}>Max ({balanceUiText} {symbol})</Text>
          </Pressable>
        </View>
        <View style={styles.amountRow}>
          <TextInput
            style={[styles.input, styles.amountInput]}
            value={amount}
            onChangeText={setAmount}
            keyboardType="decimal-pad"
            placeholder="0.0"
            placeholderTextColor={colors.textDim}
          />
          <Text style={styles.amountUnit}>{symbol}</Text>
        </View>
        {amountCheck.kind === "error" && amount.length > 0 ? (
          <Text style={styles.errorText}>{amountCheck.message}</Text>
        ) : null}
        {amountCheck.kind === "warn" ? (
          <Text style={styles.warnText}>{amountCheck.message}</Text>
        ) : null}
      </View>

      <Pressable
        onPress={proceed}
        disabled={!canProceed}
        style={({ pressed }) => [
          styles.primary,
          pressed && styles.primaryPressed,
          !canProceed && styles.disabled
        ]}
      >
        <Text style={styles.primaryLabel}>Review</Text>
      </Pressable>
    </ScreenShell>
  );
}

function TokenChip({
  label,
  balance,
  selected,
  onPress
}: {
  label: string;
  balance: string;
  selected: boolean;
  onPress: () => void;
}) {
  return (
    <Pressable
      onPress={onPress}
      style={[styles.chip, selected && styles.chipSelected]}
    >
      <Text style={[styles.chipLabel, selected && styles.chipLabelSelected]}>{label}</Text>
      <Text style={styles.chipBalance}>{balance}</Text>
    </Pressable>
  );
}

const styles = StyleSheet.create({
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
    textTransform: "uppercase"
  },
  tokenRow: {
    gap: space.xs,
    paddingVertical: 2
  },
  chip: {
    paddingHorizontal: space.sm,
    paddingVertical: space.xs,
    borderRadius: radius.md,
    borderWidth: 1,
    borderColor: colors.border,
    backgroundColor: colors.bg,
    minWidth: 80,
    gap: 2
  },
  chipSelected: {
    borderColor: colors.accent,
    backgroundColor: colors.accentSoft
  },
  chipLabel: {
    color: colors.text,
    fontSize: font.sm,
    fontWeight: "600"
  },
  chipLabelSelected: {
    color: colors.accent
  },
  chipBalance: {
    color: colors.textMuted,
    fontSize: font.xs,
    fontFamily: "monospace"
  },
  input: {
    color: colors.text,
    fontSize: font.sm,
    fontFamily: "monospace",
    padding: space.sm,
    borderRadius: radius.md,
    borderWidth: 1,
    borderColor: colors.borderStrong,
    backgroundColor: colors.bg,
    minHeight: 44,
    textAlignVertical: "top"
  },
  ghostButton: {
    paddingVertical: 4,
    alignSelf: "flex-start"
  },
  ghostButtonLabel: {
    color: colors.accent,
    fontSize: font.xs,
    letterSpacing: letterSpacing.loose
  },
  amountHead: {
    flexDirection: "row",
    justifyContent: "space-between",
    alignItems: "center"
  },
  maxLabel: {
    color: colors.accent,
    fontSize: font.xs,
    fontFamily: "monospace"
  },
  amountRow: {
    flexDirection: "row",
    alignItems: "center",
    gap: space.xs
  },
  amountInput: {
    flex: 1,
    minHeight: 44,
    fontSize: font.lg
  },
  amountUnit: {
    color: colors.accent,
    fontSize: font.lg,
    letterSpacing: letterSpacing.loose
  },
  errorText: {
    color: colors.error,
    fontSize: font.xs
  },
  warnText: {
    color: colors.warning,
    fontSize: font.xs
  },
  muteText: {
    color: colors.textMuted,
    fontSize: font.xs
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
    opacity: 0.5
  }
});
