import type { ReactNode } from "react";

export type PdsTone = "neutral" | "accent" | "success" | "danger" | "warning";
export type PdsSize = "sm" | "md" | "lg";
export type PdsDensity = "compact" | "comfortable";

export type PdsOption = {
  value: string;
  label: string;
  description?: string;
  disabled?: boolean;
};

export type PdsDataGridColumn<Row extends Record<string, unknown>> = {
  key: keyof Row & string;
  header: string;
  width?: string | number;
  align?: "start" | "center" | "end";
  render?: (row: Row) => ReactNode;
};

export function composeClassNames(
  ...values: Array<string | false | null | undefined>
): string {
  return values.filter(Boolean).join(" ");
}

export function describedBy(
  ...ids: Array<string | false | null | undefined>
): string | undefined {
  const value = ids.filter(Boolean).join(" ");
  return value.length > 0 ? value : undefined;
}
