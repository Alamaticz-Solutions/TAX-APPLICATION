import {
  forwardRef,
  type CSSProperties,
  useId,
  type HTMLAttributes,
  type PointerEvent as ReactPointerEvent,
  type ReactNode,
  type Ref
} from "react";
import { Button } from "./primitives";
import { EmptyState } from "./surfaces";
import {
  composeClassNames,
  type PdsDataGridColumn,
  type PdsDensity
} from "./types";

export type DataGridToolbarProps = {
  ariaLabel: string;
  search?: ReactNode;
  filters?: ReactNode;
  summary?: ReactNode;
  actions?: ReactNode;
  className?: string;
  density?: PdsDensity;
  isActive?: boolean;
};

export function DataGridToolbar({
  ariaLabel,
  search,
  filters,
  summary,
  actions,
  className,
  density = "compact",
  isActive = false
}: DataGridToolbarProps) {
  return (
    <div
      className={composeClassNames("pds-data-grid-toolbar", className)}
      data-density={density}
      data-active={isActive || undefined}
      role="toolbar"
      aria-label={ariaLabel}
    >
      <div className="pds-data-grid-toolbar__primary">
        {search ? <div className="pds-data-grid-toolbar__search">{search}</div> : null}
        {filters ? <div className="pds-data-grid-toolbar__filters">{filters}</div> : null}
      </div>
      {summary || actions ? (
        <div className="pds-data-grid-toolbar__secondary">
          {summary ? <div className="pds-data-grid-toolbar__summary">{summary}</div> : null}
          {actions ? <div className="pds-data-grid-toolbar__actions">{actions}</div> : null}
        </div>
      ) : null}
    </div>
  );
}

export type DataGridFilterTriggerState = "idle" | "active" | "dirty";

export type DataGridFilterTriggerProps = {
  label?: ReactNode;
  icon?: ReactNode;
  activeCount?: number;
  state?: DataGridFilterTriggerState;
  stateLabel?: ReactNode;
  as?: "button" | "summary";
  expanded?: boolean;
  controls?: string;
  className?: string;
  disabled?: boolean;
  triggerRef?: Ref<HTMLButtonElement>;
  onClick?: () => void;
};

export function DataGridFilterTrigger({
  label = "Filters",
  icon,
  activeCount = 0,
  state,
  stateLabel,
  as = "button",
  expanded,
  controls,
  className,
  disabled = false,
  triggerRef,
  onClick
}: DataGridFilterTriggerProps) {
  const resolvedState: DataGridFilterTriggerState = state ?? (activeCount > 0 ? "active" : "idle");
  const isActive = resolvedState !== "idle" || activeCount > 0;
  const content = (
    <>
      {icon ? <span className="pds-data-grid-filter-trigger__icon" aria-hidden="true">{icon}</span> : null}
      <span className="pds-data-grid-filter-trigger__label">{label}</span>
      {activeCount > 0 ? (
        <span className="pds-data-grid-filter-trigger__count" aria-label={`${activeCount} active filters`}>
          {activeCount}
        </span>
      ) : null}
      {stateLabel ? <span className="pds-data-grid-filter-trigger__state">{stateLabel}</span> : null}
    </>
  );

  if (as === "summary") {
    return (
      <summary
        className={composeClassNames("pds-data-grid-filter-trigger", className)}
        data-active={isActive || undefined}
        data-state={resolvedState}
      >
        {content}
      </summary>
    );
  }

  return (
    <button
      type="button"
      ref={triggerRef}
      className={composeClassNames("pds-data-grid-filter-trigger", className)}
      data-active={isActive || undefined}
      data-state={resolvedState}
      aria-expanded={expanded}
      aria-controls={controls}
      disabled={disabled}
      onClick={onClick}
    >
      {content}
    </button>
  );
}

export type DataGridFilterPanelProps = {
  title: ReactNode;
  summary?: ReactNode;
  controls?: ReactNode;
  actions?: ReactNode;
  children?: ReactNode;
  ariaLabel?: string;
  className?: string;
};

export function DataGridFilterPanel({
  title,
  summary,
  controls,
  actions,
  children,
  ariaLabel,
  className
}: DataGridFilterPanelProps) {
  const panelLabel = ariaLabel ?? (typeof title === "string" ? title : undefined);

  return (
    <section className={composeClassNames("pds-data-grid-filter-panel", className)} aria-label={panelLabel}>
      <header className="pds-data-grid-filter-panel__header">
        <div className="pds-data-grid-filter-panel__copy">
          <strong className="pds-data-grid-filter-panel__title">{title}</strong>
          {summary ? <span className="pds-data-grid-filter-panel__summary">{summary}</span> : null}
        </div>
        {controls ? <div className="pds-data-grid-filter-panel__controls">{controls}</div> : null}
      </header>
      {children ? <div className="pds-data-grid-filter-panel__body">{children}</div> : null}
      {actions ? <div className="pds-data-grid-filter-panel__actions">{actions}</div> : null}
    </section>
  );
}

export type DataGridFilterGroupProps = {
  controls?: ReactNode;
  actions?: ReactNode;
  children?: ReactNode;
  isRoot?: boolean;
  className?: string;
};

export function DataGridFilterGroup({
  controls,
  actions,
  children,
  isRoot = false,
  className
}: DataGridFilterGroupProps) {
  return (
    <section className={composeClassNames("pds-data-grid-filter-group", className)} data-root={isRoot || undefined}>
      {controls || actions ? (
        <div className="pds-data-grid-filter-group__header">
          {controls ? <div className="pds-data-grid-filter-group__controls">{controls}</div> : null}
          {actions ? <div className="pds-data-grid-filter-group__actions">{actions}</div> : null}
        </div>
      ) : null}
      {children ? <div className="pds-data-grid-filter-group__rules">{children}</div> : null}
    </section>
  );
}

export type DataGridFilterRuleProps = {
  children: ReactNode;
  className?: string;
};

export function DataGridFilterRule({
  children,
  className
}: DataGridFilterRuleProps) {
  return (
    <div className={composeClassNames("pds-data-grid-filter-rule", className)}>
      {children}
    </div>
  );
}

export type DataGridFilterEmptyProps = {
  icon?: ReactNode;
  children: ReactNode;
  className?: string;
};

export function DataGridFilterEmpty({
  icon,
  children,
  className
}: DataGridFilterEmptyProps) {
  return (
    <div className={composeClassNames("pds-data-grid-filter-empty", className)} role="status">
      {icon ? <span className="pds-data-grid-filter-empty__icon" aria-hidden="true">{icon}</span> : null}
      <span>{children}</span>
    </div>
  );
}

export type DataGridColumnChooserTriggerState = "default" | "custom";

export type DataGridColumnChooserTriggerProps = {
  label?: ReactNode;
  icon?: ReactNode;
  selectedCount?: number;
  totalCount?: number;
  state?: DataGridColumnChooserTriggerState;
  as?: "button" | "summary";
  expanded?: boolean;
  controls?: string;
  className?: string;
  disabled?: boolean;
  triggerRef?: Ref<HTMLButtonElement>;
  onClick?: () => void;
};

export function DataGridColumnChooserTrigger({
  label = "Columns",
  icon,
  selectedCount = 0,
  totalCount,
  state,
  as = "button",
  expanded,
  controls,
  className,
  disabled = false,
  triggerRef,
  onClick
}: DataGridColumnChooserTriggerProps) {
  const resolvedState: DataGridColumnChooserTriggerState =
    state ?? (totalCount != null && selectedCount !== totalCount ? "custom" : "default");
  const countLabel = totalCount != null ? `${selectedCount}/${totalCount}` : String(selectedCount);
  const countAriaLabel =
    totalCount != null
      ? `${selectedCount} of ${totalCount} columns selected`
      : `${selectedCount} columns selected`;
  const content = (
    <>
      {icon ? <span className="pds-data-grid-column-trigger__icon" aria-hidden="true">{icon}</span> : null}
      <span className="pds-data-grid-column-trigger__label">{label}</span>
      <span className="pds-data-grid-column-trigger__count" aria-label={countAriaLabel}>
        {countLabel}
      </span>
    </>
  );

  if (as === "summary") {
    return (
      <summary
        className={composeClassNames("pds-data-grid-column-trigger", className)}
        data-active={resolvedState === "custom" || undefined}
        data-state={resolvedState}
      >
        {content}
      </summary>
    );
  }

  return (
    <button
      type="button"
      ref={triggerRef}
      className={composeClassNames("pds-data-grid-column-trigger", className)}
      data-active={resolvedState === "custom" || undefined}
      data-state={resolvedState}
      aria-expanded={expanded}
      aria-controls={controls}
      disabled={disabled}
      onClick={onClick}
    >
      {content}
    </button>
  );
}

export type DataGridControlPopoverSize = "sm" | "md" | "lg";

export type DataGridControlPopoverProps = HTMLAttributes<HTMLDivElement> & {
  ariaLabel: string;
  size?: DataGridControlPopoverSize;
  nativePopover?: "auto" | "manual";
};

export const DataGridControlPopover = forwardRef<HTMLDivElement, DataGridControlPopoverProps>(
  function DataGridControlPopover({
    ariaLabel,
    size = "md",
    nativePopover,
    className,
    children,
    role = "dialog",
    ...props
  }, ref) {
    const popoverProps = nativePopover ? ({ popover: nativePopover } as Record<string, string>) : {};

    return (
      <div
        {...props}
        {...popoverProps}
        ref={ref}
        className={composeClassNames("pds-data-grid-control-popover", className)}
        data-size={size}
        role={role}
        aria-label={ariaLabel}
      >
        {children}
      </div>
    );
  }
);

export type DataGridDensityControlProps = {
  value: PdsDensity;
  onChange: (density: PdsDensity) => void;
  label?: ReactNode;
  compactLabel?: ReactNode;
  comfortableLabel?: ReactNode;
  compactDetail?: ReactNode;
  comfortableDetail?: ReactNode;
  className?: string;
  disabled?: boolean;
};

export function DataGridDensityControl({
  value,
  onChange,
  label = "Density",
  compactLabel = "Compact",
  comfortableLabel = "Comfortable",
  compactDetail,
  comfortableDetail,
  className,
  disabled = false
}: DataGridDensityControlProps) {
  const name = useId();
  const options: Array<{
    value: PdsDensity;
    label: ReactNode;
    detail?: ReactNode;
  }> = [
    { value: "compact", label: compactLabel, detail: compactDetail },
    { value: "comfortable", label: comfortableLabel, detail: comfortableDetail }
  ];

  return (
    <fieldset
      className={composeClassNames("pds-data-grid-density-control", className)}
      disabled={disabled}
    >
      <legend className="pds-data-grid-density-control__legend">{label}</legend>
      <div className="pds-data-grid-density-control__options">
        {options.map((option) => (
          <label
            className="pds-data-grid-density-control__option"
            data-selected={value === option.value || undefined}
            key={option.value}
            title={typeof option.label === "string" ? `${option.label} density` : undefined}
          >
            <input
              type="radio"
              name={name}
              value={option.value}
              checked={value === option.value}
              onChange={() => onChange(option.value)}
            />
            <span className="pds-data-grid-density-control__icon" data-density={option.value} aria-hidden="true">
              <span />
              <span />
              <span />
              <span />
            </span>
            <span className="pds-data-grid-density-control__sr-label">
              <span>{option.label}</span>
              {option.detail ? <span>{option.detail}</span> : null}
            </span>
          </label>
        ))}
      </div>
    </fieldset>
  );
}

export type DataGridSortDirection = "asc" | "desc";

export type DataGridColumnResizeHandleProps = {
  ariaLabel: string;
  width?: string | number;
  minWidth?: number;
  maxWidth?: number;
  defaultWidth?: number;
  title?: string;
  className?: string;
  onResize: (width: number) => void;
  onReset?: () => void;
};

export function DataGridColumnResizeHandle({
  ariaLabel,
  width,
  minWidth = 96,
  maxWidth = 560,
  defaultWidth = 180,
  title = "Drag to resize column",
  className,
  onResize,
  onReset
}: DataGridColumnResizeHandleProps) {
  return (
    <button
      type="button"
      className={composeClassNames("pds-data-grid__resizer", className)}
      aria-label={ariaLabel}
      title={onReset ? `${title}. Double-click to reset.` : title}
      onPointerDown={(event) =>
        startColumnResize(event, width, onResize, { minWidth, maxWidth, defaultWidth })
      }
      onDoubleClick={(event) => {
        if (!onReset) return;
        event.preventDefault();
        event.stopPropagation();
        onReset();
      }}
    />
  );
}

export type DataGridSortButtonProps = {
  label: ReactNode;
  ariaLabel: string;
  direction?: DataGridSortDirection | null;
  sortable?: boolean;
  title?: string;
  className?: string;
  onSort?: () => void;
};

export function DataGridSortButton({
  label,
  ariaLabel,
  direction = null,
  sortable = true,
  title,
  className,
  onSort
}: DataGridSortButtonProps) {
  const content = (
    <>
      <span className="pds-data-grid-sort-button__label">{label}</span>
      {direction ? (
        <span className="pds-data-grid-sort-button__indicator" aria-hidden="true">
          {direction.toUpperCase()}
        </span>
      ) : null}
    </>
  );

  if (!sortable) {
    return (
      <span
        className={composeClassNames("pds-data-grid-sort-button", className)}
        data-sortable="false"
        title={title}
      >
        {content}
      </span>
    );
  }

  return (
    <button
      type="button"
      className={composeClassNames("pds-data-grid-sort-button", className)}
      data-active={Boolean(direction) || undefined}
      data-direction={direction ?? undefined}
      aria-label={ariaLabel}
      title={title}
      onClick={onSort}
    >
      {content}
    </button>
  );
}

export type DataGridColumnChooserOption = {
  id: string;
  label: ReactNode;
  detail?: ReactNode;
  meta?: ReactNode;
  disabled?: boolean;
};

export type DataGridColumnChooserGroup = {
  id: string;
  title: ReactNode;
  options: readonly DataGridColumnChooserOption[];
};

export type DataGridColumnChooserProps = {
  groups: readonly DataGridColumnChooserGroup[];
  selectedIds: readonly string[];
  onToggle: (id: string, checked: boolean) => void;
  title?: ReactNode;
  summary?: ReactNode;
  searchLabel?: string;
  searchPlaceholder?: string;
  searchValue?: string;
  onSearchChange?: (value: string) => void;
  actions?: ReactNode;
  emptyMessage?: ReactNode;
  minSelected?: number;
  className?: string;
};

export function DataGridColumnChooser({
  groups,
  selectedIds,
  onToggle,
  title = "Column chooser",
  summary,
  searchLabel = "Find columns",
  searchPlaceholder = "Caption, field, or type",
  searchValue,
  onSearchChange,
  actions,
  emptyMessage = "No columns match this search.",
  minSelected = 1,
  className
}: DataGridColumnChooserProps) {
  const selected = new Set(selectedIds);
  const visibleGroups = groups.filter((group) => group.options.length > 0);
  const optionCount = groups.reduce((count, group) => count + group.options.length, 0);

  return (
    <div className={composeClassNames("pds-data-grid-column-chooser", className)}>
      <div className="pds-data-grid-column-chooser__header">
        <div>
          <strong>{title}</strong>
          <span>{summary ?? `${selectedIds.length} of ${optionCount} selected`}</span>
        </div>
      </div>
      {onSearchChange ? (
        <label className="pds-data-grid-column-chooser__search">
          <span>{searchLabel}</span>
          <input
            type="search"
            value={searchValue ?? ""}
            placeholder={searchPlaceholder}
            onChange={(event) => onSearchChange(event.target.value)}
          />
        </label>
      ) : null}
      {actions ? <div className="pds-data-grid-column-chooser__actions">{actions}</div> : null}
      <div className="pds-data-grid-column-chooser__options">
        {visibleGroups.map((group) => (
          <fieldset className="pds-data-grid-column-chooser__group" key={group.id}>
            <legend className="pds-data-grid-column-chooser__group-title">{group.title}</legend>
            {group.options.map((option) => {
              const checked = selected.has(option.id);
              const disabled = option.disabled || (checked && selectedIds.length <= minSelected);
              return (
                <label className="pds-data-grid-column-chooser__option" key={option.id}>
                  <input
                    type="checkbox"
                    checked={checked}
                    disabled={disabled}
                    onChange={(event) => onToggle(option.id, event.target.checked)}
                  />
                  <span>
                    <b>{option.label}</b>
                    {option.detail || option.meta ? (
                      <small>
                        {option.detail ? <span>{option.detail}</span> : null}
                        {option.meta ? <em>{option.meta}</em> : null}
                      </small>
                    ) : null}
                  </span>
                </label>
              );
            })}
          </fieldset>
        ))}
        {visibleGroups.length === 0 ? (
          <div className="pds-data-grid-column-chooser__empty" role="status">{emptyMessage}</div>
        ) : null}
      </div>
    </div>
  );
}

export type DataGridLoadingPreviewProps = HTMLAttributes<HTMLDivElement> & {
  columnCount?: number;
  rowCount?: number;
  label?: string;
  density?: PdsDensity;
};

export function DataGridLoadingPreview({
  columnCount = 5,
  rowCount = 7,
  label = "Loading records",
  density = "compact",
  className,
  style,
  ...props
}: DataGridLoadingPreviewProps) {
  const safeColumnCount = Math.max(1, Math.min(columnCount, 8));
  const safeRowCount = Math.max(1, Math.min(rowCount, 12));
  const previewStyle = {
    "--pds-data-grid-loading-columns": safeColumnCount,
    ...style
  } as CSSProperties;
  const columns = Array.from({ length: safeColumnCount });
  const rows = Array.from({ length: safeRowCount });

  return (
    <div
      {...props}
      className={composeClassNames("pds-data-grid-loading-preview", className)}
      data-density={density}
      style={previewStyle}
      role="status"
      aria-label={label}
      aria-busy="true"
    >
      <div className="pds-data-grid-loading-preview__header" aria-hidden="true">
        {columns.map((_, index) => (
          <span className="pds-data-grid-loading-preview__heading" key={index} />
        ))}
      </div>
      <div className="pds-data-grid-loading-preview__rows" aria-hidden="true">
        {rows.map((_, rowIndex) => (
          <div className="pds-data-grid-loading-preview__row" key={rowIndex}>
            {columns.map((__, columnIndex) => (
              <span
                className="pds-data-grid-loading-preview__cell"
                data-length={(rowIndex + columnIndex) % 3}
                key={columnIndex}
              />
            ))}
          </div>
        ))}
      </div>
    </div>
  );
}

export type DataGridShellProps<Row extends Record<string, unknown>> = {
  columns: readonly PdsDataGridColumn<Row>[];
  rows: readonly Row[];
  rowKey: keyof Row & string;
  ariaLabel: string;
  className?: string;
  density?: PdsDensity;
  isLoading?: boolean;
  selectedRowKey?: string | null;
  emptyTitle?: string;
  emptyDetail?: ReactNode;
  onColumnResize?: (columnKey: keyof Row & string, width: number) => void;
  onRowSelect?: (row: Row) => void;
};

export function DataGridShell<Row extends Record<string, unknown>>({
  columns,
  rows,
  rowKey,
  ariaLabel,
  className,
  density = "compact",
  isLoading = false,
  selectedRowKey,
  emptyTitle = "No records found",
  emptyDetail,
  onColumnResize,
  onRowSelect
}: DataGridShellProps<Row>) {
  if (isLoading) {
    return (
      <DataGridLoadingPreview
        className={className}
        columnCount={columns.length}
        density={density}
        label={`Loading ${ariaLabel}`}
      />
    );
  }

  if (rows.length === 0) {
    return (
      <div className={composeClassNames("pds-data-grid", className)} role="region" aria-label={ariaLabel}>
        <EmptyState title={emptyTitle} detail={emptyDetail} />
      </div>
    );
  }

  return (
    <div
      className={composeClassNames("pds-data-grid", className)}
      data-density={density}
      data-row-action={Boolean(onRowSelect) || undefined}
      role="region"
      aria-label={ariaLabel}
      tabIndex={0}
    >
      <table>
        <colgroup>
          {columns.map((column) => (
            <col key={column.key} style={column.width ? { width: column.width } : undefined} />
          ))}
        </colgroup>
        <thead>
          <tr>
            {columns.map((column) => (
              <th key={column.key} data-resizable={Boolean(onColumnResize) || undefined}>
                <div className="pds-data-grid__header-cell">
                  <span className={composeClassNames("pds-data-grid__head", column.align && `is-${column.align}`)}>
                    {column.header}
                  </span>
                  {onColumnResize ? (
                    <DataGridColumnResizeHandle
                      ariaLabel={`Resize ${column.header}`}
                      width={column.width}
                      onResize={(width) => onColumnResize(column.key, width)}
                    />
                  ) : null}
                </div>
              </th>
            ))}
          </tr>
        </thead>
        <tbody>
          {rows.map((row, index) => {
            const key = String(row[rowKey] ?? index);
            const isSelected = selectedRowKey != null && key === selectedRowKey;
            return (
              <tr
                key={key}
                className={isSelected ? "is-selected" : undefined}
                aria-selected={isSelected || undefined}
                tabIndex={onRowSelect ? 0 : undefined}
                onClick={() => onRowSelect?.(row)}
                onKeyDown={(event) => {
                  if (onRowSelect && (event.key === "Enter" || event.key === " ")) {
                    event.preventDefault();
                    onRowSelect(row);
                  }
                }}
              >
                {columns.map((column) => (
                  <td key={column.key} data-align={column.align}>
                    {column.render ? column.render(row) : formatCell(row[column.key])}
                  </td>
                ))}
              </tr>
            );
          })}
        </tbody>
      </table>
    </div>
  );
}

export type DataGridPaginationProps = {
  pageSize: number;
  pageIndex: number;
  startRow: number;
  endRow: number;
  totalRows?: number | null;
  pageCount?: number | null;
  responseMs?: number | null;
  pageSizeOptions?: readonly number[];
  className?: string;
  ariaLabel?: string;
  onPageSizeChange?: (pageSize: number) => void;
  onFirstPage?: () => void;
  onPreviousPage?: () => void;
  onNextPage?: () => void;
  onLastPage?: () => void;
  canPrevious?: boolean;
  canNext?: boolean;
};

export function DataGridPagination({
  pageSize,
  pageIndex,
  startRow,
  endRow,
  totalRows = null,
  pageCount = null,
  responseMs = null,
  pageSizeOptions = [10, 25, 50, 100],
  className,
  ariaLabel = "Grid pagination",
  onPageSizeChange,
  onFirstPage,
  onPreviousPage,
  onNextPage,
  onLastPage,
  canPrevious = Boolean(onPreviousPage || onFirstPage),
  canNext = Boolean(onNextPage || onLastPage)
}: DataGridPaginationProps) {
  const pageLabel = `Page ${pageIndex}${pageCount ? ` of ${pageCount}` : ""}`;
  const rangeLabel =
    startRow > 0 || endRow > 0
      ? `${formatNumber(startRow)}-${formatNumber(endRow)}${totalRows ? ` of ${formatNumber(totalRows)}` : ""}`
      : totalRows
        ? `0 of ${formatNumber(totalRows)}`
        : "No rows";

  return (
    <nav
      className={composeClassNames("pds-data-grid-pagination", className)}
      aria-label={ariaLabel}
    >
      <div className="pds-data-grid-pagination__summary" aria-live="polite">
        <strong>{rangeLabel}</strong>
        <span>{pageLabel}</span>
        {responseMs != null ? <span>{Math.round(responseMs)} ms</span> : null}
      </div>
      <div className="pds-data-grid-pagination__controls">
        {onPageSizeChange ? (
          <label className="pds-data-grid-pagination__page-size">
            <span>Rows</span>
            <select value={pageSize} onChange={(event) => onPageSizeChange(Number(event.target.value))}>
              {pageSizeOptions.map((candidate) => (
                <option key={candidate} value={candidate}>
                  {candidate}
                </option>
              ))}
            </select>
          </label>
        ) : null}
        {onFirstPage ? (
          <Button size="sm" variant="secondary" onClick={onFirstPage} disabled={!canPrevious}>
            First
          </Button>
        ) : null}
        {onPreviousPage ? (
          <Button size="sm" variant="secondary" onClick={onPreviousPage} disabled={!canPrevious}>
            Prev
          </Button>
        ) : null}
        {onNextPage ? (
          <Button size="sm" variant="secondary" onClick={onNextPage} disabled={!canNext}>
            Next
          </Button>
        ) : null}
        {onLastPage ? (
          <Button size="sm" variant="secondary" onClick={onLastPage} disabled={!canNext}>
            Last
          </Button>
        ) : null}
      </div>
    </nav>
  );
}

function startColumnResize(
  event: ReactPointerEvent<HTMLButtonElement>,
  width: string | number | undefined,
  onResize: (width: number) => void,
  {
    minWidth,
    maxWidth,
    defaultWidth
  }: {
    minWidth: number;
    maxWidth: number;
    defaultWidth: number;
  }
) {
  event.preventDefault();
  event.stopPropagation();
  const header = event.currentTarget.closest("th");
  const startX = event.clientX;
  const startWidth = pixelWidth(width) ?? header?.getBoundingClientRect().width ?? defaultWidth;
  document.body.classList.add("pds-is-resizing-column");

  const onMove = (moveEvent: PointerEvent) => {
    onResize(clampColumnWidth(startWidth + moveEvent.clientX - startX, minWidth, maxWidth));
  };
  const onUp = () => {
    document.body.classList.remove("pds-is-resizing-column");
    window.removeEventListener("pointermove", onMove);
    window.removeEventListener("pointerup", onUp);
    window.removeEventListener("pointercancel", onUp);
  };

  window.addEventListener("pointermove", onMove);
  window.addEventListener("pointerup", onUp);
  window.addEventListener("pointercancel", onUp);
}

function pixelWidth(width: string | number | undefined): number | null {
  if (typeof width === "number") return width;
  if (typeof width === "string" && /^\d+(\.\d+)?px$/.test(width)) {
    return Number.parseFloat(width);
  }
  return null;
}

function clampColumnWidth(value: number, minWidth: number, maxWidth: number): number {
  return Math.min(Math.max(Math.round(value), minWidth), maxWidth);
}

function formatCell(value: unknown): string {
  if (value === null || value === undefined) return "Not set";
  if (typeof value === "boolean") return value ? "Yes" : "No";
  if (typeof value === "number") return formatNumber(value);
  if (typeof value === "string") return maybeFormatDate(value);
  if (Array.isArray(value)) return `${value.length} items`;
  if (typeof value === "object") return "Object";
  return String(value);
}

function maybeFormatDate(value: string): string {
  if (!/^\d{4}-\d{2}-\d{2}/.test(value)) return value;
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) return value;
  return new Intl.DateTimeFormat(undefined, {
    month: "short",
    day: "numeric",
    year: "numeric"
  }).format(date);
}

function formatNumber(value: number): string {
  if (!Number.isFinite(value)) return String(value);
  return new Intl.NumberFormat(undefined, {
    maximumFractionDigits: Math.abs(value) >= 100 ? 0 : 2
  }).format(value);
}
