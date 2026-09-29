import { useCallback, useEffect, useMemo, useRef, useState, type ReactNode } from "react";
import { crmUiContract, type AppfwUiEntityContract } from "../generated/appfw-ui-contract";
import { ArrowLeft, ClipboardList, LayoutDashboard } from "lucide-react";
import type { PdsDensity } from "@appfw/pds-health-components";
import { useNavigate } from "react-router";
import { useAppfwClient, useAuth, useTenant } from "../app/providers";
import { type AppfwEntityListData, type AppfwOperationError, type AppfwRecord } from "../lib/appfwClient";
import { Badge, DataGridLoadingPreview, DataGridPagination, DataTable, StateView } from "../components/ui";
import { EditPanel } from "./EditPanel";
import { GridControls } from "./GridControls";
import {
  readGridPreferences,
  writeGridPreferences
} from "./gridPreferences";
import { useLoadingPreview } from "./useLoadingPreview";
import {
  createNewRecordDraftRef,
  isRecordLocator,
  recordRouteRef,
  resolveRecordRouteRef
} from "./routeIdentity";
import {
  combineFilters,
  filterRulesToJson,
  gridFields,
  gridSelection,
  operationBlocker,
  queryBuilderFields,
  removeDeletedRecord,
  scaffoldFields,
  searchFilter,
  sortByFieldOrder,
  sortInput,
  toOperationError,
  upsertSavedRecord
} from "./entityScaffoldModel";
import type { EntityScaffoldMode, FilterJoin, FilterRuleDraft, QueryFormState, RequestTrace, SortDirection } from "./types";

export type EntityRecordInsight = {
  label: string;
  formLabel?: string;
  render: (recordKey: string) => ReactNode;
};

// Generic, model-driven workspace (ADR 0007). This component is deliberately
// an orchestrator: reusable grid/query controls and record form behavior live
// beside it as exemplar scaffold components for downstream agents. Product
// workflows can extend a record view through EntityRecordInsight without
// importing product-specific screens into the generic scaffold.
export function EntityListView({
  entity,
  limit = 25,
  initialMode = "list",
  initialRecordId = null,
  initialDraft,
  recordInsight
}: {
  entity: AppfwUiEntityContract;
  limit?: number;
  initialMode?: EntityScaffoldMode;
  initialRecordId?: string | null;
  initialDraft?: AppfwRecord;
  recordInsight?: EntityRecordInsight;
}) {
  const navigate = useNavigate();
  const client = useAppfwClient();
  const { auth } = useAuth();
  const { tenant } = useTenant();
  const rowKey = entity.primaryKey || entity.captionField;
  const initialDraftKey = useMemo(() => JSON.stringify(initialDraft ?? {}), [initialDraft]);
  const listFields = useMemo(() => scaffoldFields(entity, "list"), [entity]);
  const columnFields = useMemo(() => gridFields(entity), [entity]);
  const queryFields = useMemo(() => queryBuilderFields(entity), [entity]);
  const publicListFields = useMemo(
    () => listFields.filter((field) => field.name !== entity.primaryKey && !field.isKey),
    [entity.primaryKey, listFields]
  );
  const defaultColumns = useMemo(() => {
    const fieldNames = publicListFields.map((field) => field.name);
    return fieldNames.length ? fieldNames : columnFields.slice(0, 1).map((field) => field.name);
  }, [columnFields, publicListFields]);
  const defaultDensity: PdsDensity = crmUiContract.design.density;
  const gridPreferences = useMemo(
    () => readGridPreferences(entity, defaultColumns, columnFields, queryFields, defaultDensity),
    [columnFields, defaultColumns, defaultDensity, entity, queryFields]
  );
  const restoredAppliedFilter = useMemo(
    () => safeFilterRulesToJson(queryFields, gridPreferences.appliedFilterRules, gridPreferences.appliedFilterJoin),
    [gridPreferences.appliedFilterJoin, gridPreferences.appliedFilterRules, queryFields]
  );
  const restoredAppliedSort = useMemo(
    () => sortInput(gridPreferences.appliedSortField, gridPreferences.appliedSortDirection),
    [gridPreferences.appliedSortDirection, gridPreferences.appliedSortField]
  );
  const [mode, setMode] = useState<EntityScaffoldMode>(initialMode);
  const [pageSize, setPageSize] = useState(limit);
  const [skip, setSkip] = useState(0);
  const [visibleColumnNames, setVisibleColumnNames] = useState<string[]>(gridPreferences.visibleColumnNames);
  const [columnWidths, setColumnWidths] = useState<Record<string, number>>(gridPreferences.columnWidths);
  const [gridDensity, setGridDensity] = useState<PdsDensity>(gridPreferences.density);
  const [filterJoin, setFilterJoin] = useState<FilterJoin>(gridPreferences.filterJoin);
  const [filterRules, setFilterRules] = useState<FilterRuleDraft[]>(gridPreferences.filterRules);
  const [sortField, setSortField] = useState(gridPreferences.sortField);
  const [sortDirection, setSortDirection] = useState<SortDirection>(gridPreferences.sortDirection);
  const [appliedFilterJoin, setAppliedFilterJoin] = useState<FilterJoin>(gridPreferences.appliedFilterJoin);
  const [appliedFilterRules, setAppliedFilterRules] = useState<FilterRuleDraft[]>(gridPreferences.appliedFilterRules);
  const [appliedSortField, setAppliedSortField] = useState(gridPreferences.appliedSortField);
  const [appliedSortDirection, setAppliedSortDirection] = useState<SortDirection>(gridPreferences.appliedSortDirection);
  const [searchTerm, setSearchTerm] = useState(gridPreferences.searchTerm);
  const [appliedFilter, setAppliedFilter] = useState<unknown>(restoredAppliedFilter);
  const [appliedSort, setAppliedSort] = useState<unknown>(restoredAppliedSort);
  const [queryError, setQueryError] = useState<string | null>(null);
  const [listData, setListData] = useState<AppfwEntityListData | null>(null);
  const [loadingList, setLoadingList] = useState(true);
  const [selectedKey, setSelectedKey] = useState<string | null>(() =>
    initialRecordId ? resolveRecordRouteRef(entity, initialRecordId) : null
  );
  const [recordInsightOpen, setRecordInsightOpen] = useState(false);
  const [trace, setTrace] = useState<RequestTrace | null>(null);
  const [error, setError] = useState<AppfwOperationError | null>(null);
  const modeRef = useRef(mode);
  const columns = useMemo(
    () => (visibleColumnNames.length ? visibleColumnNames : defaultColumns),
    [defaultColumns, visibleColumnNames]
  );
  const gridSelectionFields = useMemo(() => gridSelection(entity, columns), [columns, entity]);
  const columnLabels = useMemo(
    () => Object.fromEntries(columnFields.map((field) => [field.name, field.label])),
    [columnFields]
  );
  const createOperation = entity.operations.find(
    (operation) =>
      operation.kind === "mutation" &&
      operation.returnsShape === "record" &&
      operation.name.startsWith("create_")
  );
  const createBlocker = operationBlocker(createOperation, auth.authorization, tenant.tenantId, "creating");
  const draftQueryKey = useMemo(
    () => queryFormKey(filterJoin, filterRules, sortField, sortDirection),
    [filterJoin, filterRules, sortDirection, sortField]
  );
  const appliedQueryKey = useMemo(
    () => queryFormKey(appliedFilterJoin, appliedFilterRules, appliedSortField, appliedSortDirection),
    [appliedFilterJoin, appliedFilterRules, appliedSortDirection, appliedSortField]
  );
  const hasDraftQuery = hasQueryDraft(filterRules, sortField, sortDirection);
  const hasAppliedQueryState = hasActiveQuery(appliedFilterRules, appliedSortField, appliedSortDirection);
  const queryFormState: QueryFormState = draftQueryKey !== appliedQueryKey
    ? "dirty"
    : hasAppliedQueryState
      ? "clean"
      : "empty";
  const canApplyQuery = draftQueryKey !== appliedQueryKey;
  const canClearQuery = Boolean(searchTerm.trim()) || hasDraftQuery || hasAppliedQueryState;
  const activeRecordInsight =
    mode === "record" && selectedKey && recordInsightOpen && recordInsight ? recordInsight : null;
  const showInitialListPreview = useLoadingPreview(loadingList && listData === null);
  const isListRefreshing = loadingList && listData !== null;

  useEffect(() => {
    setSkip(0);
    setPageSize(limit);
    setVisibleColumnNames(gridPreferences.visibleColumnNames);
    setColumnWidths(gridPreferences.columnWidths);
    setGridDensity(gridPreferences.density);
    setFilterJoin(gridPreferences.filterJoin);
    setFilterRules(gridPreferences.filterRules);
    setSortField(gridPreferences.sortField);
    setSortDirection(gridPreferences.sortDirection);
    setAppliedFilterJoin(gridPreferences.appliedFilterJoin);
    setAppliedFilterRules(gridPreferences.appliedFilterRules);
    setAppliedSortField(gridPreferences.appliedSortField);
    setAppliedSortDirection(gridPreferences.appliedSortDirection);
    setSearchTerm(gridPreferences.searchTerm);
    setAppliedFilter(restoredAppliedFilter);
    setAppliedSort(restoredAppliedSort);
    setQueryError(null);
    setListData(null);
    setTrace(null);
    setError(null);
    setLoadingList(true);
  }, [entity.typeName, gridPreferences, limit, restoredAppliedFilter, restoredAppliedSort]);

  useEffect(() => {
    writeGridPreferences(entity, {
      visibleColumnNames: columns,
      columnWidths,
      filterJoin,
      filterRules,
      sortField,
      sortDirection,
      appliedFilterJoin,
      appliedFilterRules,
      appliedSortField,
      appliedSortDirection,
      searchTerm,
      density: gridDensity
    });
  }, [
    appliedFilterJoin,
    appliedFilterRules,
    appliedSortDirection,
    appliedSortField,
    columnWidths,
    columns,
    entity,
    filterJoin,
    filterRules,
    gridDensity,
    searchTerm,
    sortDirection,
    sortField
  ]);

  useEffect(() => {
    setSelectedKey(initialRecordId ? resolveRecordRouteRef(entity, initialRecordId) : null);
    setMode(initialMode);
  }, [entity, entity.typeName, initialDraftKey, initialMode, initialRecordId]);

  useEffect(() => {
    setRecordInsightOpen(false);
  }, [entity.typeName, mode, selectedKey]);

  useEffect(() => {
    modeRef.current = mode;
  }, [mode]);

  useEffect(() => {
    let active = true;
    setTrace(null);
    setError(null);
    setLoadingList(true);

    client
      .queryEntityList(entity, {
        limit: pageSize,
        skip,
        selection: gridSelectionFields,
        filter: combineFilters(searchFilter(entity, columns, searchTerm), appliedFilter),
        sort: appliedSort
      })
      .then((result) => {
        if (!active) return;
        setListData(result.data);
        setTrace({
          requestId: result.requestId,
          correlationId: result.correlationId,
          responseMs: result.responseMs
        });
        setSelectedKey((current) => {
          if (modeRef.current !== "list") return current;
          if (current && result.data.rows.some((row) => String(row[rowKey] ?? "") === current)) {
            return current;
          }
          return result.data.rows[0] ? String(result.data.rows[0][rowKey] ?? "") : null;
        });
      })
      .catch((caught: unknown) => {
        if (!active) return;
        setError(toOperationError(caught));
      })
      .finally(() => {
        if (active) setLoadingList(false);
      });

    return () => {
      active = false;
    };
  }, [appliedFilter, appliedSort, client, columns, entity, gridSelectionFields, pageSize, rowKey, searchTerm, skip]);

  const showList = () => {
    setSelectedKey(null);
    setRecordInsightOpen(false);
    setMode("list");
    navigate(`/data/${entity.routeSegment}`);
  };
  const handleRecordLoaded = useCallback(
    (record: AppfwRecord) => {
      const nextKey = String(record[rowKey] ?? "");
      if (nextKey) setSelectedKey(nextKey);
      const nextRouteRef = recordRouteRef(entity, record);
      if (initialRecordId && !isRecordLocator(initialRecordId) && nextRouteRef) {
        navigate(recordPath(entity, nextRouteRef), { replace: true });
      }
    },
    [entity, initialRecordId, navigate, rowKey]
  );

  if (error) {
    if (error.category === "policy_denied" || error.category === "auth") {
      return (
        <StateView
          kind="denied"
          title="Access denied"
          detail={error.message || "The backend policy denied this request."}
        />
      );
    }
    return <StateView kind="error" title={`Could not load ${entity.caption.plural}`} detail={error.message} />;
  }

  if (listData === null || showInitialListPreview) {
    if (!showInitialListPreview) {
      return (
        <div
          className="crm-entity-workspace crm-entity-workspace--pending"
          aria-busy="true"
          aria-label={`Loading ${entity.caption.plural}`}
        />
      );
    }
    return <EntityWorkspaceSkeleton entity={entity} />;
  }

  return (
    <div className={`crm-entity-workspace${isListRefreshing ? " crm-entity-workspace--refreshing" : ""}`} aria-busy={isListRefreshing || undefined}>
      <ScaffoldToolbar
        entity={entity}
        mode={mode}
        trace={trace}
        data={listData}
        onBackToList={showList}
        recordInsightLabel={recordInsight?.label}
        recordInsightFormLabel={recordInsight?.formLabel}
        recordInsightOpen={recordInsightOpen}
        onToggleRecordInsight={
          mode === "record" && selectedKey && recordInsight
            ? () => setRecordInsightOpen((current) => !current)
            : undefined
        }
      />

      <div className="crm-workbench">
        <div className="crm-workbench-main">
          {mode === "list" ? (
            <>
              <GridControls
                columnFields={columnFields}
                queryFields={queryFields}
                visibleColumns={columns}
                setVisibleColumns={(nextColumns) => {
                  setVisibleColumnNames(sortByFieldOrder(columnFields, nextColumns));
                  setSkip(0);
                }}
                resetColumns={() => {
                  setVisibleColumnNames(defaultColumns);
                  setColumnWidths({});
                  setSkip(0);
                }}
                filterJoin={filterJoin}
                filterRules={filterRules}
                setFilterJoin={setFilterJoin}
                setFilterRules={setFilterRules}
                sortField={sortField}
                sortDirection={sortDirection}
                setSortField={setSortField}
                setSortDirection={setSortDirection}
                density={gridDensity}
                setDensity={setGridDensity}
                searchTerm={searchTerm}
                setSearchTerm={(value) => {
                  setSearchTerm(value);
                  setSkip(0);
                }}
                queryError={queryError}
                queryFormState={queryFormState}
                canApplyQuery={canApplyQuery}
                canClearQuery={canClearQuery}
                hasAppliedQuery={Boolean(searchTerm.trim()) || hasAppliedQueryState}
                applyQuery={() => {
                  try {
                    const nextAppliedFilter = filterRulesToJson(queryFields, filterRules, filterJoin);
                    const nextAppliedSort = sortInput(sortField, sortDirection);
                    setAppliedFilter(nextAppliedFilter);
                    setAppliedSort(nextAppliedSort);
                    setAppliedFilterJoin(filterJoin);
                    setAppliedFilterRules(cloneFilterRules(filterRules));
                    setAppliedSortField(sortField);
                    setAppliedSortDirection(sortDirection);
                    setQueryError(null);
                    setSkip(0);
                  } catch (caught) {
                    setQueryError(caught instanceof Error ? caught.message : String(caught));
                  }
                }}
                clearQuery={() => {
                  setFilterJoin("and");
                  setFilterRules([]);
                  setSortField("");
                  setSortDirection("");
                  setSearchTerm("");
                  setAppliedFilterJoin("and");
                  setAppliedFilterRules([]);
                  setAppliedSortField("");
                  setAppliedSortDirection("");
                  setAppliedFilter(undefined);
                  setAppliedSort(undefined);
                  setQueryError(null);
                  setSkip(0);
                }}
                createBlocker={createBlocker}
                onCreateNew={() => {
                  setSelectedKey(null);
                  setMode("new");
                  navigate(newRecordPath(entity));
                }}
              />
              <DataTable
                columns={columns}
                rows={listData.rows}
                rowKey={rowKey}
                selectedRowKey={selectedKey}
                columnLabels={columnLabels}
                columnWidths={columnWidths}
                density={gridDensity}
                onColumnResize={(column, width) =>
                  setColumnWidths((current) => ({
                    ...current,
                    [column]: width
                  }))
                }
                onRowSelect={(row) => {
                  const nextKey = String(row[rowKey] ?? "");
                  if (!nextKey) return;
                  setSelectedKey(nextKey);
                  setMode("record");
                  navigate(recordPath(entity, recordRouteRef(entity, row)));
                }}
              />
              <PaginationSummary
                data={listData}
                trace={trace}
                pageSize={pageSize}
                setPageSize={(nextSize) => {
                  setPageSize(nextSize);
                  setSkip(0);
                }}
                goToFirstPage={() => setSkip(0)}
                goToPreviousPage={() => setSkip((current) => Math.max(0, current - pageSize))}
                goToNextPage={() => setSkip((current) => current + pageSize)}
              />
            </>
          ) : null}

          {mode === "record" || mode === "new" ? (
            <>
              {activeRecordInsight && selectedKey ? (
                <div className="crm-record-insight">
                  {activeRecordInsight.render(selectedKey)}
                </div>
              ) : null}
              <div hidden={Boolean(activeRecordInsight)}>
                <EditPanel
                  entity={entity}
                  recordId={mode === "record" ? selectedKey : null}
                  isNew={mode === "new"}
                  initialValues={mode === "new" ? initialDraft : undefined}
                  onCancel={showList}
                  onNavigateToRecord={(targetEntity, recordKey, routeRef) => {
                    navigate(recordPath(targetEntity, routeRef ?? recordKey));
                  }}
                  onNavigateToNew={(targetEntity, initialValues) => {
                    navigate(newRecordPath(targetEntity, initialValues));
                  }}
                  onLoaded={handleRecordLoaded}
                  onDeleted={(record) => {
                    setListData((current) => removeDeletedRecord(current, rowKey, record));
                    showList();
                  }}
                  onSaved={(record) => {
                    setListData((current) => upsertSavedRecord(current, rowKey, record));
                    const nextKey = record ? String(record[rowKey] ?? selectedKey ?? "") : selectedKey;
                    setSelectedKey(nextKey);
                    if (record && nextKey) {
                      setMode("record");
                      navigate(recordPath(entity, recordRouteRef(entity, record)), { replace: mode === "new" });
                    }
                  }}
                />
              </div>
            </>
          ) : null}
        </div>
      </div>
    </div>
  );
}

function EntityWorkspaceSkeleton({ entity }: { entity: AppfwUiEntityContract }) {
  const previewColumnCount = Math.min(6, Math.max(3, gridFields(entity).length));

  return (
    <div
      className="crm-entity-workspace crm-entity-workspace--loading"
      aria-busy="true"
    >
      <div className="crm-scaffold-toolbar crm-scaffold-toolbar--skeleton">
        <div className="crm-skeleton-title-stack">
          <span className="crm-skeleton-line is-kicker" />
          <span className="crm-skeleton-line is-title" />
        </div>
        <div className="crm-toolbar-controls" aria-hidden="true">
          <span className="crm-skeleton-pill" />
          <span className="crm-skeleton-pill is-short" />
        </div>
      </div>

      <div className="crm-workbench">
        <div className="crm-workbench-main">
          <div className="crm-grid-controls crm-grid-controls--skeleton" aria-hidden="true">
            <div className="crm-grid-control-row">
              <span className="crm-skeleton-input" />
              <span className="crm-skeleton-button" />
              <span className="crm-skeleton-button is-wide" />
              <span className="crm-skeleton-segment" />
              <span className="crm-skeleton-button is-compact" />
            </div>
          </div>

          <DataGridLoadingPreview
            columnCount={previewColumnCount}
            rowCount={7}
            density="compact"
            label={`Loading ${entity.caption.plural}`}
          />

          <div className="crm-pagination crm-pagination--skeleton" aria-hidden="true">
            <span className="crm-skeleton-line is-summary" />
            <div className="crm-toolbar-controls">
              <span className="crm-skeleton-button is-compact" />
              <span className="crm-skeleton-button is-compact" />
              <span className="crm-skeleton-button is-wide" />
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}

function recordPath(entity: AppfwUiEntityContract, recordKey: string) {
  return `/data/${entity.routeSegment}/${encodeURIComponent(recordKey.trim())}`;
}

function newRecordPath(entity: AppfwUiEntityContract, initialValues: AppfwRecord = {}) {
  const draftRef = createNewRecordDraftRef(entity, initialValues);
  return `/data/${entity.routeSegment}/new${draftRef ? `?draft=${encodeURIComponent(draftRef)}` : ""}`;
}

function safeFilterRulesToJson(
  queryFields: ReturnType<typeof queryBuilderFields>,
  filterRules: FilterRuleDraft[],
  filterJoin: FilterJoin
) {
  try {
    return filterRulesToJson(queryFields, filterRules, filterJoin);
  } catch (_error) {
    return undefined;
  }
}

function queryFormKey(
  filterJoin: FilterJoin,
  filterRules: FilterRuleDraft[],
  sortField: string,
  sortDirection: SortDirection
) {
  return JSON.stringify({
    filterJoin,
    filterRules: filterRules.map(({ fieldName, operator, value }) => ({ fieldName, operator, value })),
    sortField,
    sortDirection: sortField ? sortDirection : ""
  });
}

function hasQueryDraft(filterRules: FilterRuleDraft[], sortField: string, sortDirection: SortDirection) {
  return filterRules.length > 0 || Boolean(sortField || sortDirection);
}

function hasActiveQuery(filterRules: FilterRuleDraft[], sortField: string, sortDirection: SortDirection) {
  return filterRules.some((rule) => rule.value.trim()) || Boolean(sortField && sortDirection);
}

function cloneFilterRules(filterRules: FilterRuleDraft[]) {
  return filterRules.map((rule) => ({ ...rule }));
}

function ScaffoldToolbar({
  entity,
  mode,
  trace,
  data,
  onBackToList,
  recordInsightLabel,
  recordInsightFormLabel,
  recordInsightOpen,
  onToggleRecordInsight
}: {
  entity: AppfwUiEntityContract;
  mode: EntityScaffoldMode;
  trace: RequestTrace | null;
  data: AppfwEntityListData;
  onBackToList: () => void;
  recordInsightLabel?: string;
  recordInsightFormLabel?: string;
  recordInsightOpen?: boolean;
  onToggleRecordInsight?: () => void;
}) {
  return (
    <div className="crm-scaffold-toolbar">
      <div>
        <div className="crm-kicker">Entity workspace</div>
        <div className="crm-toolbar-title">{entity.caption.plural}</div>
      </div>
      <div className="crm-toolbar-controls">
        {mode !== "list" ? (
          <button type="button" className="crm-button secondary compact" onClick={onBackToList}>
            <ArrowLeft size={14} />
            Back to list
          </button>
        ) : null}
        {onToggleRecordInsight ? (
          <button
            type="button"
            className={`crm-button secondary compact${recordInsightOpen ? " is-active" : ""}`}
            onClick={onToggleRecordInsight}
            aria-pressed={recordInsightOpen}
          >
            {recordInsightOpen ? <ClipboardList size={14} /> : <LayoutDashboard size={14} />}
            {recordInsightOpen ? recordInsightFormLabel ?? "Record form" : recordInsightLabel ?? "Record dashboard"}
          </button>
        ) : null}
        {mode === "list" ? <Badge>{data.rows.length} shown</Badge> : null}
        {mode === "list" && data.page.queryCount ? <Badge>{data.page.queryCount} total</Badge> : null}
        {entity.audited ? <Badge tone="accent">Audited</Badge> : null}
        {entity.readOnly ? <Badge>Read only</Badge> : null}
        {trace ? <Badge>{Math.round(trace.responseMs)} ms</Badge> : null}
      </div>
    </div>
  );
}

function PaginationSummary({
  data,
  trace,
  pageSize,
  setPageSize,
  goToFirstPage,
  goToPreviousPage,
  goToNextPage
}: {
  data: AppfwEntityListData;
  trace: RequestTrace | null;
  pageSize: number;
  setPageSize: (pageSize: number) => void;
  goToFirstPage: () => void;
  goToPreviousPage: () => void;
  goToNextPage: () => void;
}) {
  const total = data.page.queryCount;
  const start = data.rows.length ? data.page.skip + 1 : 0;
  const end = data.page.skip + data.rows.length;
  const pageIndex = pageSize ? Math.floor(data.page.skip / pageSize) + 1 : data.page.pageIndex + 1;
  const pageCount = total ? Math.max(1, Math.ceil(total / pageSize)) : data.page.pageCount;
  const canPrevious = data.page.skip > 0;
  const canNext = total ? end < total : data.rows.length >= pageSize;

  return (
    <DataGridPagination
      pageSize={pageSize}
      pageIndex={pageIndex}
      pageCount={pageCount}
      startRow={start}
      endRow={end}
      totalRows={total}
      responseMs={trace?.responseMs ?? null}
      canPrevious={canPrevious}
      canNext={canNext}
      onPageSizeChange={setPageSize}
      onFirstPage={goToFirstPage}
      onPreviousPage={goToPreviousPage}
      onNextPage={goToNextPage}
    />
  );
}
