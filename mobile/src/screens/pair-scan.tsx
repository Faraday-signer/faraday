import { CameraView, useCameraPermissions } from "expo-camera";
import { useCallback, useRef, useState } from "react";
import { Pressable, StyleSheet, Text, View } from "react-native";
import type { NativeStackScreenProps } from "@react-navigation/native-stack";

import { ScreenShell } from "../components/screen-shell";
import { useAppState } from "../lib/app-state";
import { parsePairInput } from "../lib/pair-parser";
import { colors, font, letterSpacing, radius, space } from "../lib/theme";
import type { RootStackParamList } from "../navigation/root";

type Props = NativeStackScreenProps<RootStackParamList, "PairScan">;

export function PairScanScreen({ navigation }: Props) {
  const [permission, requestPermission] = useCameraPermissions();
  const { setPairedPubkey } = useAppState();
  const [error, setError] = useState<string | null>(null);
  const handlingRef = useRef(false);

  const onBarcodeScanned = useCallback(
    async ({ data }: { data: string }) => {
      if (handlingRef.current) return;
      const result = parsePairInput(data);
      if (result.kind === "invalid") {
        setError("Scanned QR is not a Faraday pair QR or a Solana address.");
        return;
      }
      if (result.kind === "wrong-mode") {
        setError(result.hint);
        return;
      }
      handlingRef.current = true;
      try {
        await setPairedPubkey(result.pubkey);
        navigation.popToTop();
      } catch (e) {
        handlingRef.current = false;
        setError(e instanceof Error ? e.message : String(e));
      }
    },
    [navigation, setPairedPubkey]
  );

  if (!permission) {
    return (
      <ScreenShell eyebrow="Pair" title="Scan device">
        <Text style={styles.muted}>Checking camera permission…</Text>
      </ScreenShell>
    );
  }

  if (!permission.granted) {
    return (
      <ScreenShell eyebrow="Pair" title="Scan device">
        <View style={styles.card}>
          <Text style={styles.body}>Faraday needs camera access to scan the device pubkey QR.</Text>
          <Pressable
            style={({ pressed }) => [styles.primary, pressed && styles.primaryPressed]}
            onPress={() => {
              void requestPermission();
            }}
          >
            <Text style={styles.primaryLabel}>Grant camera access</Text>
          </Pressable>
        </View>
        <Pressable onPress={() => navigation.navigate("PairPaste")}>
          <Text style={styles.link}>Paste address instead</Text>
        </Pressable>
      </ScreenShell>
    );
  }

  return (
    <ScreenShell eyebrow="Pair" title="Scan device">
      <View style={styles.cameraBox}>
        <CameraView
          style={styles.camera}
          facing="back"
          barcodeScannerSettings={{ barcodeTypes: ["qr"] }}
          onBarcodeScanned={(scan) => {
            void onBarcodeScanned({ data: scan.data });
          }}
        />
      </View>
      {error ? (
        <Pressable onPress={() => setError(null)} style={styles.errorBox}>
          <Text style={styles.errorText}>{error}</Text>
          <Text style={styles.errorHint}>Tap to dismiss.</Text>
        </Pressable>
      ) : (
        <Text style={styles.muted}>Align the device pubkey QR inside the frame.</Text>
      )}
      <Pressable
        onPress={() => navigation.navigate("PairPaste")}
        style={({ pressed }) => [styles.secondary, pressed && styles.secondaryPressed]}
      >
        <Text style={styles.secondaryLabel}>Paste address instead</Text>
      </Pressable>
    </ScreenShell>
  );
}

const styles = StyleSheet.create({
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
  card: {
    padding: space.md,
    borderRadius: radius.lg,
    borderWidth: 1,
    borderColor: colors.border,
    backgroundColor: colors.panel,
    gap: space.sm
  },
  body: {
    color: colors.text,
    fontSize: font.sm,
    lineHeight: 20
  },
  muted: {
    color: colors.textMuted,
    fontSize: font.sm,
    textAlign: "center"
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
  secondaryPressed: {
    backgroundColor: colors.panelHi
  },
  secondaryLabel: {
    color: colors.text,
    fontSize: font.sm,
    letterSpacing: letterSpacing.loose
  },
  link: {
    color: colors.accent,
    fontSize: font.sm,
    textAlign: "center"
  },
  errorBox: {
    padding: space.sm,
    borderRadius: radius.md,
    borderWidth: 1,
    borderColor: colors.error,
    backgroundColor: "rgba(255, 107, 107, 0.12)"
  },
  errorText: {
    color: colors.error,
    fontSize: font.sm
  },
  errorHint: {
    color: colors.textDim,
    fontSize: font.xs,
    marginTop: 2
  }
});
