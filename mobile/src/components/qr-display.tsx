import { useEffect, useState } from "react";
import { StyleSheet, Text, View } from "react-native";
import QRCode from "react-native-qrcode-svg";

import { colors, font, letterSpacing, radius, space } from "../lib/theme";
import type { EncodedQrPayload } from "../lib/ur-encode";

interface Props {
  payload: EncodedQrPayload;
  size?: number;
  caption?: string;
  showCounter?: boolean;
}

/**
 * Renders a static QR or animated UR sequence. UR fragments are uppercased
 * before display because Faraday's camera works best on the canonical
 * ALL-CAPS bytewords ABC encoding.
 */
export function QrDisplay({ payload, size = 320, caption, showCounter = true }: Props) {
  const [index, setIndex] = useState(0);

  useEffect(() => {
    setIndex(0);
    if (payload.kind !== "animated") return;
    const timer = setInterval(() => {
      setIndex((prev) => (prev + 1) % payload.frames.length);
    }, payload.intervalMs);
    return () => clearInterval(timer);
  }, [payload]);

  const value =
    payload.kind === "static" ? payload.value : payload.frames[index].toUpperCase();
  const total = payload.kind === "animated" ? payload.frames.length : 0;

  return (
    <View style={styles.wrap}>
      <View style={styles.card}>
        <QRCode
          value={value}
          size={size}
          ecl="M"
          backgroundColor={colors.qrSurface}
          color={colors.qrModule}
        />
      </View>
      {payload.kind === "animated" && showCounter ? (
        <Text style={styles.counter}>
          UR frame {index + 1}/{total}
        </Text>
      ) : null}
      {caption ? <Text style={styles.caption}>{caption}</Text> : null}
    </View>
  );
}

const styles = StyleSheet.create({
  wrap: {
    alignItems: "center",
    gap: space.xs
  },
  card: {
    padding: space.md,
    borderRadius: radius.lg,
    backgroundColor: colors.qrSurface
  },
  counter: {
    color: colors.textMuted,
    fontSize: font.xs,
    fontFamily: "monospace",
    letterSpacing: letterSpacing.normal
  },
  caption: {
    color: colors.textMuted,
    fontSize: font.sm,
    textAlign: "center"
  }
});
