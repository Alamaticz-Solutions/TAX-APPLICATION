import { useEffect, useMemo, useRef, useState } from "react";
import { Columns3, RotateCcw, Save, Search, X } from "lucide-react";
import {
  Button,
  DataGridColumnChooser,
  DataGridColumnChooserTrigger,
  DataGridControlPopover,
  DataGridColumnResizeHandle,
  DataGridDensityControl,
  DataGridPagination,
  DataGridSortButton,
  DataGridToolbar,
  EmptyState,
  IconButton,
  type PdsDensity
} from "@appfw/pds-health-components";
import { AdvancedFilterBuilder } from "./AdvancedFilterBuilder";
import type { ActiveScope, AdvancedFilterState, EntityType, FilterCapabilities, GridColumn, GridSort, LookupOption, RecordValue } from "../types";
import { displayColumnValue } from "../lib/entityModel";

type RecordsPanelProps = {
  entity: EntityType | null;
  filterCapabilities: FilterCapabilities | null;
  allColumns: GridColumn[];
  tableColumns: GridColumn[];
  visibleColumnIds: string[];
  page: number;
  pageSize: number;
  pageCount: number | null;
  totalRows: number | null;
  activeScope: ActiveScope | null;
  isLoading: boolean;
  records: RecordValue[];
  recordSearch: string;
  recordSearchTerm: string;
  advancedFilter: AdvancedFilterState;
  lookupOptions: Record<string, LookupOption[]>;
  sort: GridSort | null;
  onSearchChange: (value: string) => void;
  onAdvancedFilterApply: (filter: AdvancedFilterState) => void;
  onAdvancedFilterClear: () => void;
  onSortChange: (sort: GridSort | null) => void;
  onVisibleColumnsChange: (ids: string[]) => void;
  onSaveView: () => void;
  onResetView: () => void;
  onFirstPage: () => void;
  onPreviousPage: () => void;
  onNextPage: () => void;
  onClearScope: () => void;
  onOpenRecord: (record: RecordValue) => void;
  canGoPrevious: boolean;
  canGoNext: boolean;
};

export function RecordsPanel({
  entity,
  filterCapabilities,
  allColumns,
  tableColumns,
  visibleColumnIds,
  page,
  pageSize,
  pageCount,
  totalRows,
  activeScope,
  isLoading,
  records,
  recordSearch,
  recordSearchTerm,
  advancedFilter,
  lookupOptions,
  sort,
  onSearchChange,
  onAdvancedFilterApply,
  onAdvancedFilterClear,
  onSortChange,
  onVisibleColumnsChange,
  onSaveView,
  onResetView,
  onFirstPage,
  onPreviousPage,
  onNextPage,
  onClearScope,
  onOpenRecord,
  canGoPrevious,
  canGoNext
}: RecordsPanelProps) {
  const [columnWidths, setColumnWidths] = useState<Record<string, number>>({});
  const [columnSearch, setColumnSearch] = useState("");
  const [gridDensity, setGridDensity] = useState<PdsDensity>("compact");
  const columnsMenuRef = useRef<HTMLDetailsElement | null>(null);
  const columns = useMemo(
    () =>
      tableColumns.map((column) => ({
        column,
        width: columnWidths[column.id] ?? defaultColumnWidth(column)
      })),
    [tableColumns, columnWidths]
  );
  const totalColumnWidth = columns.reduce((total, column) => total + column.width, 0);
  const filteredColumns = useMemo(() => {
    const needle = columnSearch.trim().toLowerCase();
    if (!needle) return allColumns;
    return allColumns.filter((column) => (
      column.label.toLowerCase().includes(needle) ||
      column.pathLabel.toLowerCase().includes(needle) ||
      column.data_type.toLowerCase().includes(needle)
    ));
  }, [allColumns, columnSearch]);
  const nativeColumns = filteredColumns.filter((column) => column.kind === "native");
  const relatedColumns = filteredColumns.filter((column) => column.kind === "related");
  const selectedColumnCount = allColumns.filter((column) => visibleColumnIds.includes(column.id)).length;
  const columnChooserGroups = [
    {
      id: "native",
      title: "Model Fields",
      options: nativeColumns.map((column) => ({
        id: column.id,
        label: column.label,
        detail: column.pathLabel,
        meta: column.data_type
      }))
    },
    {
      id: "related",
      title: "Related Display Fields",
      options: relatedColumns.map((column) => ({
        id: column.id,
        label: column.label,
        detail: column.pathLabel,
        meta: column.data_type
      }))
    }
  ];
  const startRow = records.length ? (page - 1) * pageSize + 1 : 0;
  const endRow = records.length ? startRow + records.length - 1 : 0;

  useEffect(() => {
    const visibleColumnIds = new Set(tableColumns.map((column) => column.id));
    setColumnWidths((current) => {
      const next = Object.fromEntries(Object.entries(current).filter(([propId]) => visibleColumnIds.has(propId)));
      return Object.keys(next).length === Object.keys(current).length ? current : next;
    });
  }, [tableColumns]);

  useEffect(() => {
    function handleDocumentPointerDown(event: PointerEvent) {
      const menu = columnsMenuRef.current;
      if (!menu?.open) return;
      if (event.target instanceof Node && menu.contains(event.target)) return;
      menu.open = false;
    }

    function handleDocumentKeyDown(event: KeyboardEvent) {
      if (event.key === "Escape" && columnsMenuRef.current?.open) {
        columnsMenuRef.current.open = false;
      }
    }

    document.addEventListener("pointerdown", handleDocumentPointerDown);
    document.addEventListener("keydown", handleDocumentKeyDown);
    return () => {
      document.removeEventListener("pointerdown", handleDocumentPointerDown);
      document.removeEventListener("keydown", handleDocumentKeyDown);
    };
  }, []);

  function resizeColumn(propId: string, width: number) {
    setColumnWidths((current) => (
      current[propId] === width ? current : { ...current, [propId]: width }
    ));
  }

  function resetColumnWidth(propId: string) {
    setColumnWidths((current) => {
      const { [propId]: _removed, ...next } = current;
      return next;
    });
  }

  function toggleSort(column: GridColumn) {
    if (!column.sortable) return;
    if (sort?.propId !== column.id) {
      onSortChange({ propId: column.id, propName: column.prop.name, direction: "asc" });
      return;
    }
    if (sort.direction === "asc") {
      onSortChange({ propId: column.id, propName: column.prop.name, direction: "desc" });
      return;
    }
    onSortChange(null);
  }

  function toggleColumn(propId: string, checked: boolean) {
    const next = checked
      ? [...visibleColumnIds, propId]
      : visibleColumnIds.filter((id) => id !== propId);
    onVisibleColumnsChange(next);
  }

  return (
    <article className="table-panel">
      <DataGridToolbar
        ariaLabel="Record grid controls"
        className="admin-records-toolbar"
        density={gridDensity}
        summary={(
          <>
            <strong>{entity?.caption_n ?? "Records"}</strong>
            <span>
              Page {page}
              {pageCount ? ` of ${pageCount}` : ""}
            </span>
          </>
        )}
        filters={(
          <>
          <AdvancedFilterBuilder
            entity={entity}
            filterCapabilities={filterCapabilities}
            value={advancedFilter}
            lookupOptions={lookupOptions}
            onApply={onAdvancedFilterApply}
            onClear={onAdvancedFilterClear}
          />
          <details className="columns-menu" ref={columnsMenuRef}>
            <DataGridColumnChooserTrigger
              as="summary"
              icon={<Columns3 size={15} />}
              selectedCount={selectedColumnCount}
              totalCount={allColumns.length}
            />
            <DataGridControlPopover className="columns-popover" ariaLabel="Column chooser" size="sm">
              <DataGridColumnChooser
                title="Column chooser"
                summary={`${selectedColumnCount} of ${allColumns.length} selected`}
                groups={columnChooserGroups}
                selectedIds={visibleColumnIds}
                searchValue={columnSearch}
                searchPlaceholder="Find fields"
                emptyMessage="No fields match this search."
                onSearchChange={setColumnSearch}
                onToggle={toggleColumn}
                actions={(
                  <>
                    <Button size="sm" variant="secondary" onClick={onSaveView}>
                      <Save size={14} />
                      Save view
                    </Button>
                    <Button size="sm" variant="secondary" onClick={onResetView}>
                      <RotateCcw size={14} />
                      Reset
                    </Button>
                  </>
                )}
              />
            </DataGridControlPopover>
          </details>
          </>
        )}
        search={(
          <label className="admin-record-search">
            <Search size={15} />
            <input
              value={recordSearch}
              onChange={(event) => onSearchChange(event.target.value)}
              placeholder="Search records"
              type="search"
            />
          </label>
        )}
        actions={(
          <DataGridDensityControl
            value={gridDensity}
            onChange={setGridDensity}
          />
        )}
      />

      <div className="table-wrap" data-density={gridDensity}>
        {activeScope && entity && activeScope.entityId === entity.id && (
          <div className="scope-bar">
            <span>{activeScope.label}</span>
            <IconButton
              ariaLabel="Clear relationship scope"
              icon={<X size={16} />}
              onClick={onClearScope}
              size="sm"
              tooltip="Clear relationship scope"
              variant="secondary"
            />
          </div>
        )}
        {isLoading ? (
          <EmptyState title="Loading records" detail="Fetching the current page." />
        ) : records.length === 0 ? (
          <EmptyState title={recordSearchTerm ? "No rows match search" : "No rows loaded"} />
        ) : (
          <table className="records-table" style={{ minWidth: `${totalColumnWidth}px` }}>
            <colgroup>
              {columns.map(({ column, width }) => (
                <col key={column.id} style={{ width: `${width}px` }} />
              ))}
            </colgroup>
            <thead>
              <tr>
                {columns.map(({ column, width }) => (
                  <th key={column.id} aria-sort={sortAria(column, sort)}>
                    <div className="th-inner">
                      <DataGridSortButton
                        ariaLabel={sortLabel(column, sort)}
                        label={column.label}
                        direction={sort?.propId === column.id ? sort.direction : null}
                        sortable={column.sortable}
                        onSort={() => toggleSort(column)}
                        title={sortLabel(column, sort)}
                      />
                      <DataGridColumnResizeHandle
                        ariaLabel={`Resize ${column.label}`}
                        width={width}
                        onResize={(nextWidth) => resizeColumn(column.id, nextWidth)}
                        onReset={() => resetColumnWidth(column.id)}
                        title="Drag to resize"
                      />
                    </div>
                  </th>
                ))}
              </tr>
            </thead>
            <tbody>
              {records.map((record, index) => (
                <tr key={`${record.id ?? index}`} onClick={() => onOpenRecord(record)}>
                  {columns.map(({ column }) => (
                    <td key={column.id} title={displayColumnValue(column, record, lookupOptions)}>
                      {displayColumnValue(column, record, lookupOptions)}
                    </td>
                  ))}
                </tr>
              ))}
            </tbody>
          </table>
        )}
      </div>
      <DataGridPagination
        ariaLabel="Record grid pagination"
        pageSize={pageSize}
        pageIndex={page}
        pageCount={pageCount}
        startRow={startRow}
        endRow={endRow}
        totalRows={totalRows}
        canPrevious={canGoPrevious}
        canNext={canGoNext}
        onFirstPage={onFirstPage}
        onPreviousPage={onPreviousPage}
        onNextPage={onNextPage}
      />
    </article>
  );
}

function defaultColumnWidth(column: GridColumn) {
  if (column.kind === "related") return 190;
  if (column.prop.is_key) return 240;
  if (column.data_type === "String") return 180;
  if (column.data_type === "DateTime") return 170;
  if (column.data_type === "Date" || column.data_type === "Time") return 140;
  if (column.data_type === "Boolean") return 112;
  if (column.data_type.startsWith("Int") || column.data_type.startsWith("Float")) return 128;
  return 160;
}

function sortLabel(column: GridColumn, sort: GridSort | null) {
  const name = column.label;
  if (!column.sortable) return `${name} is a related display column`;
  if (sort?.propId !== column.id) return `Sort ${name} ascending`;
  return sort.direction === "asc" ? `Sort ${name} descending` : `Clear ${name} sort`;
}

function sortAria(column: GridColumn, sort: GridSort | null): "none" | "ascending" | "descending" | undefined {
  if (!column.sortable) return undefined;
  if (sort?.propId !== column.id) return "none";
  return sort.direction === "asc" ? "ascending" : "descending";
}
