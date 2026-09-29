import { DataGridShell, FeedbackState, type PdsDataGridColumn, type PdsDensity } from "@appfw/pds-health-components";
export {
  Badge,
  Button,
  CommandPalette,
  ConfirmDialog,
  DataGridColumnChooser,
  DataGridColumnChooserTrigger,
  DataGridControlPopover,
  DataGridDensityControl,
  DataGridFilterEmpty,
  DataGridFilterPanel,
  DataGridFilterRule,
  DataGridFilterTrigger,
  DataGridLoadingPreview,
  DataGridPagination,
  DataGridToolbar,
  DateField,
  DateTimeField,
  Dialog,
  Drawer,
  EmptyState,
  ErrorState,
  Field,
  FieldMetadata,
  FileUpload,
  FeedbackState,
  FormLoadingPreview,
  FormLayout,
  ForbiddenState,
  ChartLegend,
  ChartShell,
  IconButton,
  InlineAlert,
  InputGroup,
  KpiTile,
  LoadingState,
  LookupSelect,
  MenuButton,
  MetricTrend,
  MultiSelect,
  PageHeader,
  Popover,
  PopoverTrigger,
  SegmentedControl,
  Skeleton,
  Surface,
  SwitchField,
  TextField,
  TimeField,
  Tooltip,
  ToastRegion,
  ValidationSummary
} from "@appfw/pds-health-components";
export type { CommandPaletteItem, ToastItem } from "@appfw/pds-health-components";

export function StateView({
  kind,
  title,
  detail
}: {
  kind: "loading" | "empty" | "error" | "denied";
  title: string;
  detail?: string;
}) {
  return (
    <FeedbackState
      kind={kind}
      title={kind === "loading" ? "Loading..." : title}
      detail={detail}
    />
  );
}

export function DataTable({
  columns,
  rows,
  rowKey,
  selectedRowKey,
  columnLabels,
  columnWidths,
  density = "compact",
  onColumnResize,
  onRowSelect
}: {
  columns: readonly string[];
  rows: Record<string, unknown>[];
  rowKey?: string;
  selectedRowKey?: string | null;
  columnLabels?: Record<string, string>;
  columnWidths?: Record<string, number>;
  density?: PdsDensity;
  onColumnResize?: (column: string, width: number) => void;
  onRowSelect?: (row: Record<string, unknown>) => void;
}) {
  const gridColumns: PdsDataGridColumn<Record<string, unknown>>[] = columns.map((column) => ({
    key: column,
    header: columnLabels?.[column] ?? humanizeColumn(column),
    width: columnWidths?.[column] ?? 180,
    render: (row) => formatCell(row[column])
  }));

  return (
    <DataGridShell
      ariaLabel="Records"
      columns={gridColumns}
      rows={rows}
      rowKey={(rowKey ?? columns[0] ?? "id") as keyof Record<string, unknown> & string}
      density={density}
      selectedRowKey={selectedRowKey}
      emptyTitle="No rows match the current query."
      onColumnResize={onColumnResize}
      onRowSelect={onRowSelect}
    />
  );
}

function formatCell(value: unknown): string {
  if (value === null || value === undefined) return "—";
  if (typeof value === "boolean") return value ? "Yes" : "No";
  if (typeof value === "number") return formatNumber(value);
  if (typeof value === "string") return maybeFormatDate(value);
  if (Array.isArray(value)) return `${value.length} items`;
  if (isRecord(value)) {
    for (const key of ["name", "subject", "company", "email", "id"]) {
      const candidate = value[key];
      if (typeof candidate === "string" && candidate.trim()) return candidate;
    }
  }
  if (typeof value === "object") return JSON.stringify(value);
  return String(value);
}

function humanizeColumn(value: string) {
  return value
    .replace(/_/g, " ")
    .replace(/([a-z0-9])([A-Z])/g, "$1 $2")
    .replace(/\b\w/g, (letter) => letter.toUpperCase());
}

function maybeFormatDate(value: string) {
  if (!/^\d{4}-\d{2}-\d{2}/.test(value)) return value;
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) return value;
  return new Intl.DateTimeFormat(undefined, {
    month: "short",
    day: "numeric",
    year: "numeric"
  }).format(date);
}

function formatNumber(value: number) {
  if (!Number.isFinite(value)) return String(value);
  return new Intl.NumberFormat(undefined, {
    maximumFractionDigits: Math.abs(value) >= 100 ? 0 : 2
  }).format(value);
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}
