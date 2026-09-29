import type { HTMLAttributes } from "react";
import { composeClassNames } from "./types";

// Presentational, zero-dependency SVG chart renderers. The design system owns
// the chart *chrome and rendering primitives*; products still own the data,
// calculations, thresholds, and interpretation. These are deliberately simple
// (no charting engine): they render a provided series and stay token-styled and
// accessible. For richer interactivity, compose them inside `ChartShell`.
export type ChartTone = "accent" | "success" | "warning" | "danger" | "neutral";

export type ChartDatum = {
  label: string;
  value: number;
  tone?: ChartTone;
};

function maxOf(data: readonly { value: number }[]): number {
  return data.reduce((max, point) => Math.max(max, Number.isFinite(point.value) ? point.value : 0), 0);
}

function sumOf(data: readonly { value: number }[]): number {
  return data.reduce((total, point) => total + (Number.isFinite(point.value) ? point.value : 0), 0);
}

function clampPercent(value: number): number {
  return Math.max(0, Math.min(100, value));
}

const defaultFormat = (value: number): string => new Intl.NumberFormat().format(value);

// --- BarChart -------------------------------------------------------------
export type BarChartProps = Omit<HTMLAttributes<HTMLDivElement>, "title"> & {
  data: readonly ChartDatum[];
  ariaLabel: string;
  maxValue?: number;
  formatValue?: (value: number) => string;
};

export function BarChart({
  data,
  ariaLabel,
  maxValue,
  formatValue = defaultFormat,
  className,
  ...props
}: BarChartProps) {
  const max = maxValue ?? Math.max(maxOf(data), 1);
  return (
    <div
      {...props}
      className={composeClassNames("pds-bar-chart", className)}
      role="img"
      aria-label={ariaLabel}
    >
      <div className="pds-bar-chart__plot" aria-hidden="true">
        {data.map((point, index) => (
          <div className="pds-bar-chart__col" key={`${point.label}-${index}`}>
            <span className="pds-bar-chart__value">{formatValue(point.value)}</span>
            <span
              className="pds-bar-chart__bar"
              data-tone={point.tone ?? "accent"}
              style={{ height: `${clampPercent((point.value / max) * 100)}%` }}
            />
            <span className="pds-bar-chart__label">{point.label}</span>
          </div>
        ))}
      </div>
    </div>
  );
}

// --- LineChart / AreaChart ------------------------------------------------
export type LineChartProps = Omit<HTMLAttributes<HTMLDivElement>, "title"> & {
  data: readonly ChartDatum[];
  ariaLabel: string;
  maxValue?: number;
  tone?: ChartTone;
};

function plotGeometry(data: readonly ChartDatum[], max: number) {
  const count = data.length;
  return data.map((point, index) => {
    const x = count <= 1 ? 0 : (index / (count - 1)) * 100;
    const y = 100 - clampPercent((point.value / max) * 100);
    return { x, y, point };
  });
}

export function LineChart({
  data,
  ariaLabel,
  maxValue,
  tone = "accent",
  className,
  ...props
}: LineChartProps) {
  const max = maxValue ?? Math.max(maxOf(data), 1);
  const geometry = plotGeometry(data, max);
  const line = geometry.map((node) => `${node.x.toFixed(2)},${node.y.toFixed(2)}`).join(" ");
  return (
    <div
      {...props}
      className={composeClassNames("pds-line-chart", className)}
      data-tone={tone}
      role="img"
      aria-label={ariaLabel}
    >
      <svg className="pds-line-chart__svg" viewBox="0 0 100 100" preserveAspectRatio="none" aria-hidden="true">
        <polyline className="pds-line-chart__line" points={line} vectorEffect="non-scaling-stroke" />
        {geometry.map((node, index) => (
          <circle
            className="pds-line-chart__dot"
            key={index}
            cx={node.x}
            cy={node.y}
            r="1.4"
            vectorEffect="non-scaling-stroke"
          />
        ))}
      </svg>
    </div>
  );
}

export type AreaChartProps = LineChartProps;

export function AreaChart({
  data,
  ariaLabel,
  maxValue,
  tone = "accent",
  className,
  ...props
}: AreaChartProps) {
  const max = maxValue ?? Math.max(maxOf(data), 1);
  const geometry = plotGeometry(data, max);
  const line = geometry.map((node) => `${node.x.toFixed(2)},${node.y.toFixed(2)}`).join(" ");
  const area = geometry.length
    ? `0,100 ${line} ${geometry[geometry.length - 1].x.toFixed(2)},100`
    : "";
  return (
    <div
      {...props}
      className={composeClassNames("pds-area-chart", className)}
      data-tone={tone}
      role="img"
      aria-label={ariaLabel}
    >
      <svg className="pds-area-chart__svg" viewBox="0 0 100 100" preserveAspectRatio="none" aria-hidden="true">
        <polygon className="pds-area-chart__fill" points={area} />
        <polyline className="pds-area-chart__line" points={line} vectorEffect="non-scaling-stroke" />
      </svg>
    </div>
  );
}

// --- DonutChart -----------------------------------------------------------
export type DonutChartProps = Omit<HTMLAttributes<HTMLDivElement>, "title"> & {
  data: readonly ChartDatum[];
  ariaLabel: string;
  centerLabel?: string;
  centerValue?: string;
};

export function DonutChart({
  data,
  ariaLabel,
  centerLabel,
  centerValue,
  className,
  ...props
}: DonutChartProps) {
  const total = Math.max(sumOf(data), 1);
  const radius = 40;
  const circumference = 2 * Math.PI * radius;
  let offset = 0;
  const segments = data.map((point, index) => {
    const fraction = point.value / total;
    const dash = fraction * circumference;
    const segment = {
      key: `${point.label}-${index}`,
      tone: point.tone ?? "accent",
      dashArray: `${dash.toFixed(2)} ${(circumference - dash).toFixed(2)}`,
      dashOffset: (-offset).toFixed(2)
    };
    offset += dash;
    return segment;
  });
  return (
    <div
      {...props}
      className={composeClassNames("pds-donut-chart", className)}
      role="img"
      aria-label={ariaLabel}
    >
      <svg className="pds-donut-chart__svg" viewBox="0 0 100 100" aria-hidden="true">
        <circle className="pds-donut-chart__track" cx="50" cy="50" r={radius} />
        {segments.map((segment) => (
          <circle
            className="pds-donut-chart__segment"
            key={segment.key}
            cx="50"
            cy="50"
            r={radius}
            data-tone={segment.tone}
            strokeDasharray={segment.dashArray}
            strokeDashoffset={segment.dashOffset}
          />
        ))}
      </svg>
      {centerLabel || centerValue ? (
        <span className="pds-donut-chart__center" aria-hidden="true">
          {centerValue ? <strong>{centerValue}</strong> : null}
          {centerLabel ? <span>{centerLabel}</span> : null}
        </span>
      ) : null}
    </div>
  );
}
