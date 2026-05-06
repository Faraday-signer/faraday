import { StyleSheet, View } from "react-native";

import type { LiveConnectionState } from "../lib/use-live-balance";
import { colors } from "../lib/theme";

interface Props {
  state: LiveConnectionState;
}

const COLOR: Record<LiveConnectionState, string> = {
  idle: colors.textDim,
  connecting: colors.warning,
  live: colors.success,
  reconnecting: colors.warning,
  failed: colors.error
};

export function LiveDot({ state }: Props) {
  return <View style={[styles.dot, { backgroundColor: COLOR[state] }]} />;
}

const styles = StyleSheet.create({
  dot: {
    width: 6,
    height: 6,
    borderRadius: 999
  }
});
