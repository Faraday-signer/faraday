import Svg, { Path } from "react-native-svg";

import { colors } from "./theme";

interface BrandProps {
  size?: number;
  color?: string;
}

const MARK_PATH =
  "M0 0H51.5V51.5H0V0ZM51.5 51.5H103V103H51.5V51.5ZM77.7358 0H103V25.2642H77.7358V0ZM0 77.7358H25.2642V103H0V77.7358Z";

export function FaradayMark({ size = 28, color = colors.accent }: BrandProps) {
  return (
    <Svg width={size} height={size} viewBox="0 0 103 103">
      <Path d={MARK_PATH} fill={color} />
    </Svg>
  );
}
