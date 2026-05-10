import { useEffect, useState } from "react";
import { ActivityIndicator, Pressable, StyleSheet, Text, View } from "react-native";
import type { NativeStackScreenProps } from "@react-navigation/native-stack";

import { address as toAddress } from "@solana/kit";
import { findAssociatedTokenPda } from "@solana-program/token";

import { ErrorBanner } from "../components/error-banner";
import { RiskReportView } from "../components/risk-report-view";
import { ScreenShell } from "../components/screen-shell";
import { WhatWillHappen } from "../components/what-will-happen";
import { useAppState } from "../lib/app-state";
import { getRecipientHistory } from "../lib/recipient-history";
import { riskProceedLabel } from "../lib/risk-display";
import { RPC_URL } from "../lib/sol-client";
import { buildSolTransfer } from "../lib/sol-transfer";
import { buildSplTransfer } from "../lib/spl-transfer";
import { colors, font, letterSpacing, radius, space } from "../lib/theme";
import { analyzeTxRisk, type TxRiskReport } from "../lib/tx-risk";
import type { RootStackParamList } from "../navigation/root";

type Props = NativeStackScreenProps<RootStackParamList, "SendReview">;

const TOKEN_PROGRAM_ID = "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA";
const TOKEN_2022_PROGRAM_ID = "TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb";

type Phase =
  | { kind: "building" }
  | { kind: "analyzing"; txBase64: string }
  | { kind: "ready"; txBase64: string; report: TxRiskReport }
  | { kind: "error"; message: string };

export function SendReviewScreen({ navigation, route }: Props) {
  const { pairedPubkey } = useAppState();
  const params = route.params;
  const [phase, setPhase] = useState<Phase>({ kind: "building" });

  useEffect(() => {
    if (!pairedPubkey) {
      setPhase({ kind: "error", message: "No paired wallet." });
      return;
    }

    let cancelled = false;

    void (async () => {
      try {
        const txBase64 = await build(pairedPubkey, params);
        if (cancelled) return;
        setPhase({ kind: "analyzing", txBase64 });

        const history = await getRecipientHistory().catch(() => []);
        const report = await analyzeTxRisk(txBase64, RPC_URL, pairedPubkey, {
          recipientHistory: history
        });
        if (cancelled) return;
        setPhase({ kind: "ready", txBase64, report });
      } catch (err) {
        if (cancelled) return;
        setPhase({
          kind: "error",
          message: err instanceof Error ? err.message : String(err)
        });
      }
    })();

    return () => {
      cancelled = true;
    };
  }, [pairedPubkey, params]);

  if (phase.kind === "error") {
    return (
      <ScreenShell eyebrow="Send" title="Review">
        <ErrorBanner
          title="Could not prepare transaction"
          message={phase.message}
          onRetry={() => navigation.goBack()}
        />
      </ScreenShell>
    );
  }

  if (phase.kind === "building" || phase.kind === "analyzing") {
    return (
      <ScreenShell eyebrow="Send" title="Review">
        <View style={styles.loading}>
          <ActivityIndicator color={colors.accent} />
          <Text style={styles.loadingText}>
            {phase.kind === "building" ? "Building transaction…" : "Analyzing risk…"}
          </Text>
        </View>
      </ScreenShell>
    );
  }

  // ready
  return (
    <ScreenShell eyebrow="Send" title="Review">
      <View style={styles.summary}>
        <Text style={styles.summaryAmount}>
          {params.amountStr} <Text style={styles.summaryUnit}>{params.symbol}</Text>
        </Text>
        <Text style={styles.summaryArrow}>↓</Text>
        <Text style={styles.summaryRecipient}>
          {params.recipient.slice(0, 4)}…{params.recipient.slice(-4)}
        </Text>
      </View>

      <WhatWillHappen report={phase.report} />
      <RiskReportView report={phase.report} />

      <View style={{ gap: space.sm }}>
        <Pressable
          onPress={() =>
            navigation.navigate("SendSign", {
              txBase64: phase.txBase64,
              recipient: params.recipient,
              amountStr: params.amountStr,
              symbol: params.symbol
            })
          }
          style={({ pressed }) => [styles.primary, pressed && styles.primaryPressed]}
        >
          <Text style={styles.primaryLabel}>{riskProceedLabel(phase.report.level)}</Text>
        </Pressable>
        <Pressable onPress={() => navigation.goBack()} style={styles.secondary}>
          <Text style={styles.secondaryLabel}>Back</Text>
        </Pressable>
      </View>
    </ScreenShell>
  );
}

async function build(
  pairedPubkey: string,
  params: RootStackParamList["SendReview"]
): Promise<string> {
  if (params.tokenKind === "sol") {
    const { txBase64 } = await buildSolTransfer({
      from: pairedPubkey,
      to: params.recipient,
      amountSol: params.amountStr
    });
    return txBase64;
  }

  if (!params.mint || !params.programId) {
    throw new Error("Missing token metadata for SPL transfer.");
  }

  const tokenProgram =
    params.programId === "spl-token-2022" ? TOKEN_2022_PROGRAM_ID : TOKEN_PROGRAM_ID;

  const [sourceAta] = await findAssociatedTokenPda({
    owner: toAddress(pairedPubkey),
    tokenProgram: toAddress(tokenProgram),
    mint: toAddress(params.mint)
  });

  // Convert UI amount to atoms.
  const decimals = params.decimals;
  const trimmed = params.amountStr.trim();
  const [whole, frac = ""] = trimmed.split(".");
  const factor = 10n ** BigInt(decimals);
  const padded = decimals > 0 ? (frac + "0".repeat(decimals)).slice(0, decimals) : "";
  const amountRaw = BigInt(whole) * factor + (decimals > 0 ? BigInt(padded) : 0n);

  const { txBase64 } = await buildSplTransfer({
    from: pairedPubkey,
    to: params.recipient,
    mint: params.mint,
    decimals,
    amountRaw,
    sourceAta: String(sourceAta)
  });
  return txBase64;
}

const styles = StyleSheet.create({
  loading: {
    alignItems: "center",
    paddingVertical: space.xl,
    gap: space.sm
  },
  loadingText: {
    color: colors.textMuted,
    fontSize: font.sm
  },
  summary: {
    alignItems: "center",
    paddingVertical: space.md,
    gap: 4
  },
  summaryAmount: {
    color: colors.text,
    fontSize: font.hero,
    fontWeight: "600"
  },
  summaryUnit: {
    color: colors.accent,
    fontSize: font.xl,
    letterSpacing: letterSpacing.loose
  },
  summaryArrow: {
    color: colors.textDim,
    fontSize: font.lg
  },
  summaryRecipient: {
    color: colors.textMuted,
    fontSize: font.sm,
    fontFamily: "monospace"
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
  secondary: {
    paddingVertical: space.sm,
    paddingHorizontal: space.lg,
    borderRadius: radius.pill,
    borderWidth: 1,
    borderColor: colors.borderStrong,
    alignItems: "center"
  },
  secondaryLabel: {
    color: colors.text,
    fontSize: font.sm,
    letterSpacing: letterSpacing.loose
  }
});
