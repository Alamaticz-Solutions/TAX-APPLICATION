import {
  ClientSideRowModelModule,
  CellStyleModule,
  DateFilterModule,
  iconSetMaterial,
  NumberFilterModule,
  PaginationModule,
  QuickFilterModule,
  RenderApiModule,
  RowSelectionModule,
  TextFilterModule,
  TooltipModule,
  themeQuartz,
  type ColDef,
  type GridApi,
  type GridReadyEvent,
  type SelectionChangedEvent
} from "ag-grid-community";
import { AgGridProvider, AgGridReact } from "ag-grid-react";
import {
  useEffect,
  useMemo,
  useRef,
  useSyncExternalStore,
  type CSSProperties,
  type KeyboardEvent as ReactKeyboardEvent,
  type MouseEvent as ReactMouseEvent
} from "react";
import { composeClassNames, type PdsDataGridColumn, type PdsDensity } from "./types";

export type PdsDataGridVisualTheme = "apple-like" | "material-like";
export type PdsDataGridSelectionMode = "none" | "single" | "multiple";
export type PdsDataGridColumnType = "text" | "number" | "date" | "boolean";

export type PdsAdvancedDataGridColumn<Row extends Record<string, unknown>> =
  PdsDataGridColumn<Row> & {
    description?: string;
    type?: PdsDataGridColumnType;
    sortable?: boolean;
    filterable?: boolean;
    resizable?: boolean;
    minWidth?: number;
    maxWidth?: number;
    flex?: number;
  };

export type DataGridProps<Row extends Record<string, unknown>> = {
  ariaLabel: string;
  columns: readonly PdsAdvancedDataGridColumn<Row>[];
  rows: readonly Row[];
  rowKey: keyof Row & string;
  className?: string;
  density?: PdsDensity;
  visualTheme?: PdsDataGridVisualTheme;
  height?: number | string;
  isLoading?: boolean;
  emptyLabel?: string;
  quickFilterText?: string;
  selectionMode?: PdsDataGridSelectionMode;
  pagination?: boolean;
  pageSize?: number;
  pageSizeOptions?: readonly number[];
  onRowActivate?: (row: Row) => void;
  onSelectionChange?: (rows: readonly Row[]) => void;
};

const communityModules = [
  ClientSideRowModelModule,
  CellStyleModule,
  TextFilterModule,
  TooltipModule,
  NumberFilterModule,
  DateFilterModule,
  QuickFilterModule,
  RowSelectionModule,
  PaginationModule,
  RenderApiModule
];

const interactiveCellTargetSelector =
  "button, input:not([type='hidden']), select, textarea, label, a[href], area[href], "
  + "audio[controls], video[controls], summary, iframe, "
  + "[contenteditable]:not([contenteditable='false']), "
  + "[role='button'], [role='checkbox'], [role='combobox'], [role='link'], "
  + "[role='listbox'], [role='menu'], [role='menuitem'], [role='option'], "
  + "[role='radio'], [role='searchbox'], [role='slider'], [role='spinbutton'], "
  + "[role='switch'], [role='tab'], [role='treeitem'], "
  + "[tabindex]:not([tabindex='-1']):not([role='gridcell']):not([role='row']):not([role='grid'])";

function isInteractiveCellTarget(target: Element): boolean {
  return target.closest(interactiveCellTargetSelector) !== null;
}

const sharedThemeParams = {
  accentColor: "var(--pds-color-brand-blue)",
  backgroundColor: "var(--pds-color-surface-panel)",
  borderColor: "var(--pds-color-border-default)",
  browserColorScheme: "inherit",
  cellHorizontalPaddingScale: 1,
  columnBorder: false,
  dataFontSize: "var(--pds-font-size-md)",
  fontFamily: "var(--pds-font-family-sans)",
  foregroundColor: "var(--pds-color-text-default)",
  headerBackgroundColor: "var(--pds-color-surface-panel-soft)",
  headerFontSize: "var(--pds-font-size-sm)",
  headerFontWeight: 600,
  headerTextColor: "var(--pds-color-text-muted)",
  oddRowBackgroundColor: "color-mix(in srgb, var(--pds-color-surface-panel-soft) 24%, transparent)",
  rowBorder: { color: "var(--pds-color-border-default)", width: 1 },
  rowHoverColor: "var(--pds-color-state-accent-soft)",
  selectedRowBackgroundColor: "var(--pds-color-state-accent-soft)",
  wrapperBorder: { color: "var(--pds-color-border-default)", width: 1 }
} as const;

const appleLikeTheme = themeQuartz.withParams({
  ...sharedThemeParams,
  borderRadius: "var(--pds-radius-sm)",
  spacing: 6,
  wrapperBorderRadius: "var(--pds-radius-md)"
});

const materialLikeTheme = themeQuartz
  .withPart(iconSetMaterial)
  .withParams({
    ...sharedThemeParams,
    accentColor: "var(--pds-m3-color-primary)",
    borderRadius: "var(--pds-m3-shape-extra-small)",
    headerBackgroundColor: "var(--pds-m3-color-surface-container)",
    headerTextColor: "var(--pds-m3-color-on-surface-variant)",
    rowHoverColor: "color-mix(in srgb, var(--pds-m3-color-on-surface) var(--pds-m3-state-hover-opacity), transparent)",
    selectedRowBackgroundColor: "var(--pds-m3-color-primary-container)",
    spacing: 8,
    wrapperBorderRadius: "var(--pds-m3-shape-medium)"
  });

export function DataGrid<Row extends Record<string, unknown>>({
  ariaLabel,
  columns,
  rows,
  rowKey,
  className,
  density = "compact",
  visualTheme,
  height = 480,
  isLoading = false,
  emptyLabel = "No records found",
  quickFilterText,
  selectionMode = "none",
  pagination = false,
  pageSize = 25,
  pageSizeOptions = [10, 25, 50, 100],
  onRowActivate,
  onSelectionChange
}: DataGridProps<Row>) {
  const observedVisualTheme = useDocumentVisualTheme();
  const resolvedVisualTheme = visualTheme ?? observedVisualTheme;
  const gridRef = useRef<AgGridReact<Row>>(null);
  const gridApiRef = useRef<GridApi<Row> | null>(null);
  const theme = resolvedVisualTheme === "material-like" ? materialLikeTheme : appleLikeTheme;
  const rowHeight = density === "compact" ? 40 : 48;
  const headerHeight = density === "compact" ? 42 : 48;
  const wrapperStyle = { height } as CSSProperties;

  const columnDefs = useMemo<ColDef<Row>[]>(
    () => columns.map(toAgColumnDef),
    [columns]
  );
  const rowData = useMemo(() => [...rows], [rows]);

  useEffect(() => {
    gridApiRef.current?.setGridAriaProperty("label", ariaLabel);
  }, [ariaLabel]);

  const rowSelection = selectionMode === "none"
    ? undefined
    : selectionMode === "single"
      ? {
          mode: "singleRow" as const,
          checkboxes: false,
          enableClickSelection: true
        }
      : {
          mode: "multiRow" as const,
          checkboxes: true,
          headerCheckbox: true,
          enableClickSelection: true,
          enableSelectionWithoutKeys: true
        };

  function handleGridReady(event: GridReadyEvent<Row>) {
    gridApiRef.current = event.api;
    event.api.setGridAriaProperty("label", ariaLabel);
  }

  function handleClickCapture(event: ReactMouseEvent<HTMLElement>) {
    if (
      event.detail === 0
      || !(event.target instanceof Element)
      || isInteractiveCellTarget(event.target)
    ) return;
    const rowId = event.target.closest("[role='row'][row-id]")?.getAttribute("row-id");
    const row = rows.find((candidate) => String(candidate[rowKey]) === rowId);
    if (row) onRowActivate?.(row);
  }

  function handleKeyDownCapture(event: ReactKeyboardEvent<HTMLElement>) {
    if (event.key !== "Enter" || !(event.target instanceof Element)) return;
    if (isInteractiveCellTarget(event.target)) return;
    const targetCell = event.target.closest(".ag-cell[role='gridcell'], .ag-cell");
    if (!targetCell || !event.currentTarget.contains(targetCell)) return;
    const rowId = targetCell.closest("[role='row'][row-id]")?.getAttribute("row-id");
    const row = rows.find((candidate) => String(candidate[rowKey]) === rowId);
    if (row) onRowActivate?.(row);
  }

  function handleSelectionChanged(event: SelectionChangedEvent<Row>) {
    onSelectionChange?.(event.api.getSelectedRows());
  }

  return (
    <section
      className={composeClassNames("pds-data-grid-engine", className)}
      data-density={density}
      data-visual-theme={resolvedVisualTheme}
      role="region"
      aria-label={ariaLabel}
      aria-busy={isLoading || undefined}
      onClickCapture={onRowActivate ? handleClickCapture : undefined}
      onKeyDownCapture={onRowActivate ? handleKeyDownCapture : undefined}
      style={wrapperStyle}
    >
      <AgGridProvider modules={communityModules}>
        <AgGridReact<Row>
          ref={gridRef}
          theme={theme}
          rowData={rowData}
          columnDefs={columnDefs}
          defaultColDef={{
            filter: true,
            minWidth: 112,
            resizable: true,
            sortable: true
          }}
          getRowId={(params) => String(params.data[rowKey])}
          rowHeight={rowHeight}
          headerHeight={headerHeight}
          loading={isLoading}
          overlayNoRowsTemplate={`<span class="pds-data-grid-engine__empty">${escapeHtml(emptyLabel)}</span>`}
          quickFilterText={quickFilterText}
          rowSelection={rowSelection}
          pagination={pagination}
          paginationPageSize={pageSize}
          paginationPageSizeSelector={pagination ? [...pageSizeOptions] : false}
          suppressCellFocus={false}
          enableCellTextSelection
          ensureDomOrder
          animateRows={false}
          onGridReady={handleGridReady}
          onSelectionChanged={onSelectionChange ? handleSelectionChanged : undefined}
        />
      </AgGridProvider>
    </section>
  );
}

function toAgColumnDef<Row extends Record<string, unknown>>(
  column: PdsAdvancedDataGridColumn<Row>
): ColDef<Row> {
  const width = normalizeWidth(column.width);
  return {
    colId: column.key,
    field: column.key as ColDef<Row>["field"],
    headerName: column.header,
    headerTooltip: column.description,
    cellDataType: column.type,
    sortable: column.sortable ?? true,
    filter: column.filterable ?? true,
    resizable: column.resizable ?? true,
    width,
    minWidth: column.minWidth,
    maxWidth: column.maxWidth,
    flex: column.flex ?? (width == null ? 1 : undefined),
    cellClass: column.align ? `pds-data-grid-engine__cell--${column.align}` : undefined,
    cellRenderer: column.render
      ? (params: { data?: Row }) => params.data ? column.render?.(params.data) : null
      : undefined
  };
}

function normalizeWidth(width: string | number | undefined): number | undefined {
  if (typeof width === "number") return width;
  if (typeof width !== "string") return undefined;
  const match = width.trim().match(/^(\d+(?:\.\d+)?)px$/);
  return match ? Number(match[1]) : undefined;
}

function subscribeToVisualTheme(onChange: () => void): () => void {
  if (typeof document === "undefined" || typeof MutationObserver === "undefined") return () => undefined;
  const observer = new MutationObserver(onChange);
  observer.observe(document.documentElement, {
    attributes: true,
    attributeFilter: ["data-visual-theme"]
  });
  return () => observer.disconnect();
}

function currentVisualTheme(): PdsDataGridVisualTheme {
  if (typeof document === "undefined") return "apple-like";
  return document.documentElement.dataset.visualTheme === "material-like"
    ? "material-like"
    : "apple-like";
}

function useDocumentVisualTheme(): PdsDataGridVisualTheme {
  return useSyncExternalStore(subscribeToVisualTheme, currentVisualTheme, () => "apple-like");
}

function escapeHtml(value: string): string {
  return value
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;")
    .replace(/"/g, "&quot;")
    .replace(/'/g, "&#039;");
}
