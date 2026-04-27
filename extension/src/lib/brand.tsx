import type { CSSProperties } from "react";

import { colors } from "./theme";

interface BrandProps {
  height?: number;
  color?: string;
  style?: CSSProperties;
  title?: string;
}

const MARK_PATH =
  "M0 0H51.5V51.5H0V0ZM51.5 51.5H103V103H51.5V51.5ZM77.7358 0H103V25.2642H77.7358V0ZM0 77.7358H25.2642V103H0V77.7358Z";

export function FaradayLogo({ height = 28, color = colors.accent, style, title }: BrandProps) {
  return (
    <svg
      role={title ? "img" : undefined}
      aria-label={title}
      height={height}
      viewBox="0 0 768 129"
      fill="none"
      xmlns="http://www.w3.org/2000/svg"
      style={style}
    >
      {title ? <title>{title}</title> : null}
      <path
        d="M684.436 25.6719H697.321V64.1797H684.436V25.6719ZM735.973 25.6719H748.857V64.1797H735.973V25.6719ZM710.205 89.8516H723.089V115.524H710.205V89.8516ZM710.205 115.524V128.359H684.436V115.524H710.205ZM723.089 64.1797H735.973V89.8516H723.089V64.1797ZM697.321 64.1797H710.205V89.8516H697.321V64.1797Z"
        fill={color}
      />
      <path
        d="M594.197 64.1797H607.081V89.8516H594.197V64.1797ZM607.081 89.8516H632.85V102.688H607.081V89.8516ZM632.85 77.0157H645.734V64.1797H607.081V51.3438H645.734V38.5078H658.618V102.688H645.734V89.8516H632.85V77.0157ZM607.081 25.6719H645.734V38.5078H607.081V25.6719Z"
        fill={color}
      />
      <path
        d="M555.494 0H568.378V102.688H555.494V89.8516H542.61V77.0157H555.494V51.3438H542.61V38.5078H555.494V0ZM542.61 89.8516V102.688H516.842V89.8516H542.61ZM516.842 89.8516H503.958V38.5078H516.842V89.8516ZM542.61 38.5078H516.842V25.6719H542.61V38.5078Z"
        fill={color}
      />
      <path
        d="M413.718 64.1797H426.602V89.8516H413.718V64.1797ZM426.602 89.8516H452.371V102.688H426.602V89.8516ZM452.371 77.0157H465.255V64.1797H426.602V51.3438H465.255V38.5078H478.139V102.688H465.255V89.8516H452.371V77.0157ZM426.602 25.6719H465.255V38.5078H426.602V25.6719Z"
        fill={color}
      />
      <path
        d="M349.247 25.6719V38.5078H362.131V51.3438H349.247V89.8516H375.015V102.688H323.479V89.8516H336.363V38.5078H323.479V25.6719H349.247ZM387.9 25.6719V38.5078H362.131V25.6719H387.9Z"
        fill={color}
      />
      <path
        d="M233.239 64.1797H246.124V89.8516H233.239V64.1797ZM246.124 89.8516H271.892V102.688H246.124V89.8516ZM271.892 77.0157H284.776V64.1797H246.124V51.3438H284.776V38.5078H297.66V102.688H284.776V89.8516H271.892V77.0157ZM246.124 25.6719H284.776V38.5078H246.124V25.6719Z"
        fill={color}
      />
      <path
        d="M143 0H207.421V12.836H155.884V38.5078H194.537V51.3438H155.884V102.688H143V0Z"
        fill={color}
      />
      <path d={MARK_PATH} fill={color} />
    </svg>
  );
}

export function FaradayMark({ height = 20, color = colors.accent, style, title }: BrandProps) {
  return (
    <svg
      role={title ? "img" : undefined}
      aria-label={title}
      height={height}
      width={height}
      viewBox="0 0 103 103"
      fill="none"
      xmlns="http://www.w3.org/2000/svg"
      style={style}
    >
      {title ? <title>{title}</title> : null}
      <path d={MARK_PATH} fill={color} />
    </svg>
  );
}

export function FaradayHeroMark({ height = 72, color = colors.accent, style, title }: BrandProps) {
  return (
    <svg
      role={title ? "img" : undefined}
      aria-label={title}
      height={height}
      width={height}
      viewBox="0 0 103 103"
      fill="none"
      xmlns="http://www.w3.org/2000/svg"
      style={style}
    >
      {title ? <title>{title}</title> : null}
      <path d={MARK_PATH} fill={color} />
    </svg>
  );
}
