import { CameraView, useCameraPermissions } from "expo-camera";
import * as Clipboard from "expo-clipboard";
import { useCallback, useMemo, useRef, useState } from "react";
import { ActivityIndicator, Pressable, StyleSheet, Text, View } from "react-native";
import type { NativeStackScreenProps } from "@react-navigation/native-stack";

import { ErrorBanner } from "../components/error-banner";
import { QrDisplay } from "../components/qr-display";
import { ScreenShell } from "../components/screen-shell";
import { useAppState } from "../lib/app-state";
import { explainBroadcastError } from "../lib/broadcast-errors";
import { recordRecipient } from "../lib/recipient-history";
import {
  FARADAY_SIG_PREFIX,
  spliceFaradaySignature,
  validateSignedTransactionMatch
} from "../lib/solana";
import {
  broadcastSignedTx,
  buildSolTransfer,
  explorerTxUrl
} from "../lib/sol-transfer";
import { colors, font, letterSpacing, radius, space } from "../lib/theme";
import { encodeTxForQr } from "../lib/ur-encode";
import type { RootStackParamList } from "../navigation/root";

type Props = NativeStackScreenProps<RootStackParamList, "SendSign">;

type Phase =
  | { kind: "display" }
  | { kind: "scan" }
  | { kind: "broadcasting" }
  | { kind: "done"; signature: string }
  | { kind: "error"; message: string };

export function SendSignScreen({ navigation, route }: Props) {
  const { pairedPubkey } = useAppState();
  const [permission, requestPermission] = useCameraPermissions();
  const [phase, setPhase] = useState<Phase>({ kind: "display" });
  const [txBase64, setTxBase64] = useState(route.params.txBase64);
  const handlingRef = useRef(false);

  const qrPayload = useMemo(() => encodeTxForQr(txBase64), [txBase64]);

  const completeSignedPayload = useCallback(
    async (raw: string): Promise<{ ok: true; signedTxBase64: string } | { ok: false; error: string }> => {
      if (!pairedPubkey) return { ok: false, error: "No paired wallet." };

      const trimmed = raw.trim();
      let candidate = trimmed;
      if (trimmed.startsWith(FARADAY_SIG_PREFIX)) {
        try {
          candidate = spliceFaradaySignature(txBase64, trimmed, pairedPubkey);
        } catch (e) {
          return { ok: false, error: e instanceof Error ? e.message : String(e) };
        }
      }

      try {
        validateSignedTransactionMatch(txBase64, candidate, pairedPubkey);
      } catch (e) {
        return { ok: false, error: e instanceof Error ? e.message : String(e) };
      }

      return { ok: true, signedTxBase64: candidate };
    },
    [pairedPubkey, txBase64]
  );

  const broadcast = useCallback(
    async (signedTxBase64: string) => {
      setPhase({ kind: "broadcasting" });
      try {
        const { signature } = await broadcastSignedTx(signedTxBase64);
        void recordRecipient(route.params.recipient).catch(() => {});
        setPhase({ kind: "done", signature });
      } catch (e) {
        setPhase({ kind: "error", message: e instanceof Error ? e.message : String(e) });
      }
    },
    [route.params.recipient]
  );

  const onScanned = useCallback(
    async ({ data }: { data: string }) => {
      if (handlingRef.current) return;
      handlingRef.current = true;
      const result = await completeSignedPayload(data);
      if (!result.ok) {
        handlingRef.current = false;
        setPhase({ kind: "error", message: result.error });
        return;
      }
      await broadcast(result.signedTxBase64);
    },
    [completeSignedPayload, broadcast]
  );

  const retryWithFreshBlockhash = useCallback(async () => {
    if (!pairedPubkey) return;
    setPhase({ kind: "display" });
    handlingRef.current = false;
    try {
      const { txBase64: next } = await buildSolTransfer({
        from: pairedPubkey,
        to: route.params.recipient,
        amountSol: route.params.amountStr
      });
      setTxBase64(next);
    } catch (e) {
      setPhase({ kind: "error", message: e instanceof Error ? e.message : String(e) });
    }
  }, [pairedPubkey, route.params.amountStr, route.params.recipient]);

  if (phase.kind === "done") {
    return <DoneView signature={phase.signature} onClose={() => navigation.popToTop()} />;
  }

  if (phase.kind === "error") {
    const report = explainBroadcastError(phase.message);
    return (
      <ScreenShell eyebrow="Send" title="Sign">
        <ErrorBanner
          title="Signing did not complete"
          message={report.summary}
          details={report.details === report.summary ? undefined : report.details}
          onRetry={retryWithFreshBlockhash}
          onDismiss={() => setPhase({ kind: "display" })}
        />
        <Pressable onPress={() => navigation.popToTop()} style={styles.secondary}>
          <Text style={styles.secondaryLabel}>Cancel</Text>
        </Pressable>
      </ScreenShell>
    );
  }

  if (phase.kind === "broadcasting") {
    return (
      <ScreenShell eyebrow="Send" title="Broadcasting">
        <View style={styles.center}>
          <ActivityIndicator color={colors.accent} />
          <Text style={styles.body}>Submitting to Solana…</Text>
        </View>
      </ScreenShell>
    );
  }

  if (phase.kind === "scan") {
    if (!permission?.granted) {
      return (
        <ScreenShell eyebrow="Send" title="Scan signed">
          <View style={styles.card}>
            <Text style={styles.body}>Faraday needs the camera to scan the signed QR.</Text>
            <Pressable
              onPress={() => {
                void requestPermission();
              }}
              style={({ pressed }) => [styles.primary, pressed && styles.primaryPressed]}
            >
              <Text style={styles.primaryLabel}>Grant camera access</Text>
            </Pressable>
          </View>
          <Pressable onPress={() => setPhase({ kind: "display" })} style={styles.secondary}>
            <Text style={styles.secondaryLabel}>Back to QR</Text>
          </Pressable>
        </ScreenShell>
      );
    }

    return (
      <ScreenShell eyebrow="Send" title="Scan signed">
        <View style={styles.cameraBox}>
          <CameraView
            style={styles.camera}
            facing="back"
            barcodeScannerSettings={{ barcodeTypes: ["qr"] }}
            onBarcodeScanned={(scan) => {
              void onScanned({ data: scan.data });
            }}
          />
        </View>
        <Text style={styles.body}>Hold your Faraday device up to the camera.</Text>
        <Pressable onPress={() => setPhase({ kind: "display" })} style={styles.secondary}>
          <Text style={styles.secondaryLabel}>Back to QR</Text>
        </Pressable>
      </ScreenShell>
    );
  }

  // display
  return (
    <ScreenShell eyebrow="Send" title="Sign on Faraday">
      <View style={styles.heroSummary}>
        <Text style={styles.summaryAmount}>
          {route.params.amountStr} <Text style={styles.summaryUnit}>{route.params.symbol}</Text>
        </Text>
        <Text style={styles.summaryRecipient}>
          → {route.params.recipient.slice(0, 4)}…{route.params.recipient.slice(-4)}
        </Text>
      </View>

      <QrDisplay payload={qrPayload} caption="Scan this QR with your Faraday device." />

      <View style={{ gap: space.sm }}>
        <Pressable
          onPress={() => setPhase({ kind: "scan" })}
          style={({ pressed }) => [styles.primary, pressed && styles.primaryPressed]}
        >
          <Text style={styles.primaryLabel}>I've scanned → Scan signed back</Text>
        </Pressable>
        <Pressable onPress={() => navigation.goBack()} style={styles.secondary}>
          <Text style={styles.secondaryLabel}>Cancel</Text>
        </Pressable>
      </View>
    </ScreenShell>
  );
}

function DoneView({ signature, onClose }: { signature: string; onClose: () => void }) {
  const url = explorerTxUrl(signature);
  const onCopy = async () => {
    await Clipboard.setStringAsync(signature);
  };
  return (
    <ScreenShell eyebrow="Send" title="Broadcasted">
      <Text style={styles.body}>Transaction submitted. Confirmation may take a few seconds.</Text>
      <View style={styles.card}>
        <Text style={styles.label}>Signature</Text>
        <Text style={styles.signature} numberOfLines={2}>
          {signature}
        </Text>
        <Pressable
          onPress={() => {
            void onCopy();
          }}
          style={styles.ghost}
        >
          <Text style={styles.ghostLabel}>Copy signature</Text>
        </Pressable>
        <Text style={styles.linkHint}>{url}</Text>
      </View>
      <Pressable onPress={onClose} style={({ pressed }) => [styles.primary, pressed && styles.primaryPressed]}>
        <Text style={styles.primaryLabel}>Done</Text>
      </Pressable>
    </ScreenShell>
  );
}

const styles = StyleSheet.create({
  center: {
    alignItems: "center",
    paddingVertical: space.xl,
    gap: space.sm
  },
  body: {
    color: colors.text,
    fontSize: font.sm,
    textAlign: "center",
    lineHeight: 20
  },
  cameraBox: {
    aspectRatio: 1,
    borderRadius: radius.lg,
    overflow: "hidden",
    borderWidth: 1,
    borderColor: colors.borderStrong,
    backgroundColor: "#000"
  },
  camera: {
    flex: 1
  },
  heroSummary: {
    alignItems: "center",
    gap: 4,
    paddingTop: space.sm
  },
  summaryAmount: {
    color: colors.text,
    fontSize: font.xxl,
    fontWeight: "600"
  },
  summaryUnit: {
    color: colors.accent,
    fontSize: font.lg,
    letterSpacing: letterSpacing.loose
  },
  summaryRecipient: {
    color: colors.textMuted,
    fontSize: font.sm,
    fontFamily: "monospace"
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
    textTransform: "uppercase"
  },
  signature: {
    color: colors.text,
    fontSize: font.sm,
    fontFamily: "monospace"
  },
  ghost: {
    paddingVertical: 4,
    alignSelf: "flex-start"
  },
  ghostLabel: {
    color: colors.accent,
    fontSize: font.xs,
    letterSpacing: letterSpacing.loose
  },
  linkHint: {
    color: colors.textDim,
    fontSize: font.xs,
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
