import type { ReactNode } from "react";

export function PanelHeader({
  kicker,
  title,
  action
}: {
  kicker: string;
  title: string;
  action?: ReactNode;
}) {
  return (
    <div className="crm-panel-header">
      <div>
        <div className="crm-kicker">{kicker}</div>
        <h2>{title}</h2>
      </div>
      {action ? <div className="crm-panel-action">{action}</div> : null}
    </div>
  );
}

export function formatCurrency(value: number) {
  return new Intl.NumberFormat(undefined, {
    style: "currency",
    currency: "USD",
    notation: Math.abs(value) >= 1_000_000 ? "compact" : "standard",
    maximumFractionDigits: Math.abs(value) >= 1_000_000 ? 1 : 0
  }).format(value);
}

export function formatNumber(value: number) {
  return new Intl.NumberFormat(undefined, {
    notation: Math.abs(value) >= 10_000 ? "compact" : "standard",
    maximumFractionDigits: 1
  }).format(value);
}

export function formatPercent(value: number) {
  return new Intl.NumberFormat(undefined, {
    style: "percent",
    maximumFractionDigits: 0
  }).format(Number.isFinite(value) ? value : 0);
}

export function formatTime(value: string) {
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) return "just now";
  return new Intl.DateTimeFormat(undefined, {
    hour: "numeric",
    minute: "2-digit"
  }).format(date);
}
