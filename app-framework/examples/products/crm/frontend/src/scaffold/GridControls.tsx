import { type Dispatch, type RefObject, type SetStateAction, useEffect, useId, useRef, useState } from "react";
import type { AppfwUiFieldContract } from "../generated/appfw-ui-contract";
import type { PdsDensity } from "@appfw/pds-health-components";
import {
  Badge,
  DataGridColumnChooser,
  DataGridColumnChooserTrigger,
  DataGridControlPopover,
  DataGridDensityControl,
  DataGridFilterEmpty,
  DataGridFilterPanel,
  DataGridFilterRule,
  DataGridFilterTrigger,
  LookupSelect,
  SegmentedControl
} from "../components/ui";
import {
  createFilterRule,
  defaultOperatorForField,
  fieldTypeLabel,
  filterInputType,
  isBooleanField,
  isLookupFilterField,
  isLookupSelectorOperator,
  isQueryFilterField,
  operatorOptionsForField,
  selectedLookupValues
} from "./entityScaffoldModel";
import type { FilterJoin, FilterOperator, FilterRuleDraft, QueryFormState, SortDirection } from "./types";
import { useLookupOptions } from "./useLookupOptions";

type GridControlsProps = {
  columnFields: AppfwUiFieldContract[];
  queryFields: AppfwUiFieldContract[];
  visibleColumns: string[];
  setVisibleColumns: (columns: string[]) => void;
  resetColumns: () => void;
  filterJoin: FilterJoin;
  filterRules: FilterRuleDraft[];
  setFilterJoin: (join: FilterJoin) => void;
  setFilterRules: Dispatch<SetStateAction<FilterRuleDraft[]>>;
  sortField: string;
  sortDirection: SortDirection;
  setSortField: (fieldName: string) => void;
  setSortDirection: (direction: SortDirection) => void;
  density: PdsDensity;
  setDensity: (density: PdsDensity) => void;
  searchTerm: string;
  setSearchTerm: (value: string) => void;
  queryError: string | null;
  queryFormState: QueryFormState;
  canApplyQuery: boolean;
  canClearQuery: boolean;
  hasAppliedQuery: boolean;
  applyQuery: () => void;
  clearQuery: () => void;
  createBlocker: string | null;
  onCreateNew: () => void;
};

type NativePopoverElement = HTMLDivElement & {
  hidePopover?: () => void;
  showPopover?: () => void;
};

type PopoverPlacement = {
  maxWidth: number;
  minWidth: number;
  align: "left" | "right";
};

const queryBuilderPlacement: PopoverPlacement = { maxWidth: 640, minWidth: 300, align: "left" };
const columnsPlacement: PopoverPlacement = { maxWidth: 420, minWidth: 300, align: "right" };
const filterJoinOptions = [
  { value: "and", label: "AND" },
  { value: "or", label: "OR" }
] as const;

function togglePopover(
  ref: RefObject<NativePopoverElement | null>,
  triggerRef: RefObject<HTMLElement | null>,
  placement: PopoverPlacement,
  setOpen: Dispatch<SetStateAction<boolean>>,
  beforeHide?: () => void
) {
  const element = ref.current;
  if (!element?.showPopover || !element.hidePopover) {
    setOpen((current) => !current);
    return;
  }
  if (element.matches(":popover-open")) {
    beforeHide?.();
    element.hidePopover();
  } else {
    placePopover(element, triggerRef.current, placement);
    element.showPopover();
  }
}

function hidePopover(
  ref: RefObject<NativePopoverElement | null>,
  setOpen: Dispatch<SetStateAction<boolean>>,
  beforeHide?: () => void
) {
  const element = ref.current;
  if (element?.hidePopover && element.matches(":popover-open")) {
    beforeHide?.();
    element.hidePopover();
    return;
  }
  setOpen(false);
}

function placePopover(
  popover: NativePopoverElement | null,
  trigger: HTMLElement | null,
  { maxWidth, minWidth, align }: PopoverPlacement
) {
  if (!popover || !trigger) return;
  const triggerRect = trigger.getBoundingClientRect();
  const gutter = window.innerWidth < 560 ? 28 : 20;
  const availableWidth = Math.max(0, window.innerWidth - gutter * 2);
  const narrowWidth = Math.max(Math.min(minWidth, availableWidth), availableWidth - gutter);
  const width = Math.min(availableWidth < 560 ? narrowWidth : maxWidth, availableWidth);
  const preferredLeft = align === "right" ? triggerRect.right - width : triggerRect.left;
  const left = Math.max(gutter, Math.min(preferredLeft, window.innerWidth - gutter - width));
  const top = Math.max(gutter, triggerRect.bottom + 8);

  popover.style.position = "fixed";
  popover.style.inset = "auto";
  popover.style.left = `${Math.round(left)}px`;
  popover.style.top = `${Math.round(top)}px`;
  popover.style.width = `${Math.round(width)}px`;
  popover.style.maxHeight = `calc(100vh - ${Math.round(top + gutter)}px)`;
}

export function GridControls({
  columnFields,
  queryFields,
  visibleColumns,
  setVisibleColumns,
  resetColumns,
  filterJoin,
  filterRules,
  setFilterJoin,
  setFilterRules,
  sortField,
  sortDirection,
  setSortField,
  setSortDirection,
  density,
  setDensity,
  searchTerm,
  setSearchTerm,
  queryError,
  queryFormState,
  canApplyQuery,
  canClearQuery,
  hasAppliedQuery,
  applyQuery,
  clearQuery,
  createBlocker,
  onCreateNew
}: GridControlsProps) {
  const [columnSearch, setColumnSearch] = useState("");
  const [queryBuilderOpen, setQueryBuilderOpen] = useState(false);
  const [columnsOpen, setColumnsOpen] = useState(false);
  const queryBuilderId = useId();
  const columnsId = useId();
  const queryBuilderRef = useRef<NativePopoverElement | null>(null);
  const columnsRef = useRef<NativePopoverElement | null>(null);
  const queryBuilderTriggerRef = useRef<HTMLButtonElement | null>(null);
  const columnsTriggerRef = useRef<HTMLButtonElement | null>(null);
  const allowQueryBuilderCloseRef = useRef(false);
  const queryBuilderHasUnappliedEditsRef = useRef(false);
  const filterableFields = queryFields.filter((field) => isQueryFilterField(field) && operatorOptionsForField(field).length > 0);
  const sortableFields = columnFields.filter((field) => field.ui.sortable);
  const filteredFields = columnFields.filter((field) => {
    const needle = columnSearch.trim().toLowerCase();
    if (!needle) return true;
    return [field.label, field.name, field.dataType].some((value) => value.toLowerCase().includes(needle));
  });
  const columnChooserGroups = [{
    id: "fields",
    title: "Fields",
    options: filteredFields.map((field) => ({
      id: field.name,
      label: field.label,
      detail: <code>{field.name}</code>,
      meta: fieldTypeLabel(field)
    }))
  }];
  const activeQueryCount =
    Number(Boolean(searchTerm.trim())) +
    filterRules.filter((rule) => rule.value.trim()).length +
    Number(Boolean(sortField && sortDirection));
  const queryBuilderHasUnappliedEdits = queryFormState === "dirty";
  queryBuilderHasUnappliedEditsRef.current = queryBuilderHasUnappliedEdits;

  function allowQueryBuilderClose() {
    allowQueryBuilderCloseRef.current = true;
  }

  useEffect(() => {
    const queryBuilder = queryBuilderRef.current;
    const columns = columnsRef.current;
    const syncQueryBuilder = () => {
      const open = Boolean(queryBuilder?.matches(":popover-open"));
      if (open) {
        placePopover(queryBuilder, queryBuilderTriggerRef.current, queryBuilderPlacement);
        setQueryBuilderOpen(true);
        return;
      }
      if (allowQueryBuilderCloseRef.current) {
        allowQueryBuilderCloseRef.current = false;
        setQueryBuilderOpen(false);
        return;
      }
      if (queryBuilderHasUnappliedEditsRef.current && !allowQueryBuilderCloseRef.current) {
        window.queueMicrotask(() => {
          if (!queryBuilder || queryBuilder.matches(":popover-open")) return;
          placePopover(queryBuilder, queryBuilderTriggerRef.current, queryBuilderPlacement);
          queryBuilder.showPopover?.();
          setQueryBuilderOpen(true);
        });
        return;
      }
      setQueryBuilderOpen(open);
    };
    const syncColumns = () => {
      const open = Boolean(columns?.matches(":popover-open"));
      if (open) placePopover(columns, columnsTriggerRef.current, columnsPlacement);
      setColumnsOpen(open);
    };
    const repositionOpenPopovers = () => {
      if (queryBuilder?.matches(":popover-open")) {
        placePopover(queryBuilder, queryBuilderTriggerRef.current, queryBuilderPlacement);
      }
      if (columns?.matches(":popover-open")) {
        placePopover(columns, columnsTriggerRef.current, columnsPlacement);
      }
    };
    const closeCleanQueryBuilderOnOutsidePointer = (event: PointerEvent) => {
      if (!queryBuilder?.matches(":popover-open")) return;
      if (!(event.target instanceof Node)) return;
      if (queryBuilder.contains(event.target) || queryBuilderTriggerRef.current?.contains(event.target)) return;
      if (queryBuilderHasUnappliedEditsRef.current) return;
      allowQueryBuilderCloseRef.current = true;
      queryBuilder.hidePopover?.();
      setQueryBuilderOpen(false);
    };

    queryBuilder?.addEventListener("toggle", syncQueryBuilder);
    columns?.addEventListener("toggle", syncColumns);
    document.addEventListener("pointerdown", closeCleanQueryBuilderOnOutsidePointer, true);
    window.addEventListener("resize", repositionOpenPopovers);
    window.addEventListener("scroll", repositionOpenPopovers, true);
    return () => {
      queryBuilder?.removeEventListener("toggle", syncQueryBuilder);
      columns?.removeEventListener("toggle", syncColumns);
      document.removeEventListener("pointerdown", closeCleanQueryBuilderOnOutsidePointer, true);
      window.removeEventListener("resize", repositionOpenPopovers);
      window.removeEventListener("scroll", repositionOpenPopovers, true);
    };
  }, []);

  function addRule() {
    const rule = createFilterRule(filterableFields);
    if (!rule) return;
    setFilterRules((current) => [...current, rule]);
  }

  function updateRule(ruleId: string, patch: Partial<FilterRuleDraft>) {
    setFilterRules((current) =>
      current.map((rule) => (rule.id === ruleId ? { ...rule, ...patch } : rule))
    );
  }

  function selectRuleField(rule: FilterRuleDraft, fieldName: string) {
    const field = filterableFields.find((candidate) => candidate.name === fieldName);
    if (!field) return;
    updateRule(rule.id, {
      fieldName,
      operator: defaultOperatorForField(field),
      value: ""
    });
  }

  return (
    <div className="crm-grid-controls">
      <div className="crm-grid-control-row">
        <label className="crm-grid-search">
          <input
            aria-label="Search records"
            type="search"
            value={searchTerm}
            placeholder="Server-side search"
            onChange={(event) => setSearchTerm(event.target.value)}
          />
        </label>

        <div className="crm-query-builder">
          <DataGridFilterTrigger
            triggerRef={queryBuilderTriggerRef}
            className="crm-query-builder-trigger"
            label="Query builder"
            activeCount={activeQueryCount}
            state={queryBuilderHasUnappliedEdits ? "dirty" : activeQueryCount > 0 ? "active" : "idle"}
            stateLabel={queryBuilderHasUnappliedEdits ? "Modified" : undefined}
            expanded={queryBuilderOpen}
            controls={queryBuilderId}
            onClick={() => togglePopover(queryBuilderRef, queryBuilderTriggerRef, queryBuilderPlacement, setQueryBuilderOpen, allowQueryBuilderClose)}
          />
          <DataGridControlPopover
            id={queryBuilderId}
            ref={queryBuilderRef}
            ariaLabel="Query builder"
            className="crm-query-builder-body"
            size="lg"
            nativePopover="auto"
          >
            <DataGridFilterPanel
              title="Advanced filter"
              summary={`${filterableFields.length} filterable fields`}
              controls={(
                <SegmentedControl
                  ariaLabel="Filter join mode"
                  value={filterJoin}
                  options={filterJoinOptions}
                  onValueChange={setFilterJoin}
                />
              )}
              actions={(
                <>
                  <button
                    type="button"
                    className="crm-button primary compact"
                    disabled={!canApplyQuery}
                    onClick={() => {
                      applyQuery();
                      hidePopover(queryBuilderRef, setQueryBuilderOpen, allowQueryBuilderClose);
                    }}
                  >
                    Apply query
                  </button>
                  <button
                    type="button"
                    className="crm-button secondary compact"
                    onClick={clearQuery}
                    disabled={!canClearQuery}
                  >
                    Clear
                  </button>
                  <button
                    type="button"
                    className="crm-button secondary compact"
                    onClick={() => hidePopover(queryBuilderRef, setQueryBuilderOpen, allowQueryBuilderClose)}
                  >
                    Close
                  </button>
                  <QueryFormStateBadge state={queryFormState} />
                  {hasAppliedQuery ? <Badge tone="accent">Server filter active</Badge> : null}
                </>
              )}
            >
              {filterRules.map((rule) => {
                const field = filterableFields.find((candidate) => candidate.name === rule.fieldName) ?? filterableFields[0];
                const operators = field ? operatorOptionsForField(field) : [];
                return (
                  <DataGridFilterRule key={rule.id}>
                    <label>
                      <span>Field</span>
                      <select value={rule.fieldName} onChange={(event) => selectRuleField(rule, event.target.value)}>
                        {filterableFields.map((candidate) => (
                          <option key={candidate.name} value={candidate.name}>
                            {candidate.label} ({candidate.name})
                          </option>
                        ))}
                      </select>
                    </label>
                    <label>
                      <span>Operator</span>
                      <select
                        value={rule.operator}
                        onChange={(event) => updateRule(rule.id, { operator: event.target.value as FilterOperator })}
                      >
                        {operators.map((operator) => (
                          <option key={operator.value} value={operator.value}>
                            {operator.label}
                          </option>
                        ))}
                      </select>
                    </label>
                    {field ? (
                      <FilterValueControl
                        field={field}
                        rule={rule}
                        operator={operators.find((operator) => operator.value === rule.operator)}
                        onValueChange={(value) => updateRule(rule.id, { value })}
                      />
                    ) : null}
                    <button
                      type="button"
                      className="crm-button secondary compact"
                      onClick={() => setFilterRules((current) => current.filter((candidate) => candidate.id !== rule.id))}
                    >
                      Remove
                    </button>
                  </DataGridFilterRule>
                );
              })}
              {filterRules.length === 0 ? (
                <DataGridFilterEmpty>No rules yet. Add a field rule to query the server.</DataGridFilterEmpty>
              ) : null}

              <button type="button" className="crm-button secondary compact" disabled={!filterableFields.length} onClick={addRule}>
                Add filter
              </button>

              <div className="crm-sort-builder">
              <label>
                <span>Sort field</span>
                <select
                  value={sortField}
                  onChange={(event) => {
                    setSortField(event.target.value);
                    setSortDirection(event.target.value ? sortDirection || "ASC" : "");
                  }}
                >
                  <option value="">No sort</option>
                  {sortableFields.map((field) => (
                    <option key={field.name} value={field.name}>
                      {field.label} ({field.name})
                    </option>
                  ))}
                </select>
              </label>
              <label>
                <span>Direction</span>
                <select
                  value={sortDirection}
                  disabled={!sortField}
                  onChange={(event) => setSortDirection(event.target.value as SortDirection)}
                >
                  <option value="">None</option>
                  <option value="ASC">Ascending</option>
                  <option value="DESC">Descending</option>
                </select>
              </label>
              </div>

              {queryError ? <div className="crm-field-errors">{queryError}</div> : null}
            </DataGridFilterPanel>
          </DataGridControlPopover>
        </div>

        <div className="crm-columns-menu">
          <DataGridColumnChooserTrigger
            triggerRef={columnsTriggerRef}
            className="crm-columns-trigger"
            selectedCount={visibleColumns.length}
            totalCount={columnFields.length}
            expanded={columnsOpen}
            controls={columnsId}
            onClick={() => togglePopover(columnsRef, columnsTriggerRef, columnsPlacement, setColumnsOpen)}
          />
          <DataGridControlPopover
            id={columnsId}
            ref={columnsRef}
            ariaLabel="Column chooser"
            className="crm-columns-popover"
            size="sm"
            nativePopover="auto"
          >
            <DataGridColumnChooser
              title="Column chooser"
              summary={`${visibleColumns.length} of ${columnFields.length} selected`}
              groups={columnChooserGroups}
              selectedIds={visibleColumns}
              searchValue={columnSearch}
              searchPlaceholder="Caption, prop, or type"
              emptyMessage="No fields match this search."
              onSearchChange={setColumnSearch}
              onToggle={(columnId, checked) => {
                if (checked) {
                  setVisibleColumns([...visibleColumns, columnId]);
                  return;
                }
                setVisibleColumns(visibleColumns.filter((column) => column !== columnId));
              }}
              actions={(
                <>
                  <button type="button" className="crm-button secondary compact" onClick={() => setVisibleColumns(columnFields.map((field) => field.name))}>
                    All
                  </button>
                  <button type="button" className="crm-button secondary compact" onClick={resetColumns}>
                    Reset
                  </button>
                </>
              )}
            />
          </DataGridControlPopover>
        </div>

        <DataGridDensityControl
          value={density}
          onChange={setDensity}
        />

        <button
          type="button"
          className="crm-button primary compact"
          disabled={Boolean(createBlocker)}
          title={createBlocker ?? undefined}
          onClick={onCreateNew}
        >
          New
        </button>
      </div>
    </div>
  );
}

function QueryFormStateBadge({ state }: { state: QueryFormState }) {
  if (state === "dirty") return <Badge tone="accent">Modified</Badge>;
  if (state === "clean") return <Badge tone="success">Applied</Badge>;
  return <Badge>Empty</Badge>;
}

function FilterValueControl({
  field,
  rule,
  operator,
  onValueChange
}: {
  field: AppfwUiFieldContract;
  rule: FilterRuleDraft;
  operator?: { value: FilterOperator; label: string; list?: boolean };
  onValueChange: (value: string) => void;
}) {
  const isListValue = operator?.list ?? (rule.operator === "_in" || rule.operator === "_not_in");
  const useLookupSelector = Boolean(operator && isLookupFilterField(field) && isLookupSelectorOperator(operator));
  const lookup = useLookupOptions(field, useLookupSelector);

  if (useLookupSelector) {
    const selectedValues = selectedLookupValues(rule.value);
    return (
      <label className="crm-lookup-filter-control">
        <span>{isListValue ? "Values" : "Value"}</span>
        <LookupSelect
          multiple={isListValue}
          size={isListValue ? Math.min(6, Math.max(3, lookup.options.length || 3)) : undefined}
          value={isListValue ? selectedValues : rule.value}
          options={lookup.options}
          placeholder={`Select ${field.label.toLowerCase()}`}
          loading={lookup.status === "loading"}
          loadingMessage="Loading lookup values..."
          error={lookup.status === "error" ? lookup.error : undefined}
          onValueChange={(nextValue) => {
            if (Array.isArray(nextValue)) {
              onValueChange(nextValue.join(","));
              return;
            }
            onValueChange(nextValue);
          }}
        />
      </label>
    );
  }

  if (isBooleanField(field)) {
    return (
      <label>
        <span>Value</span>
        <select value={rule.value || "true"} onChange={(event) => onValueChange(event.target.value)}>
          <option value="true">True</option>
          <option value="false">False</option>
        </select>
      </label>
    );
  }

  return (
    <label>
      <span>{isListValue ? "Values" : "Value"}</span>
      <input
        type={filterInputType(field)}
        value={rule.value}
        placeholder={isListValue ? "Comma-separated values" : fieldTypeLabel(field)}
        onChange={(event) => onValueChange(event.target.value)}
      />
    </label>
  );
}
