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
  // Stay neutral once we've given up — SWR polling is still serving fresh
  // balances, so a red error dot would be misleading on RN.
  failed: colors.textDim
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
