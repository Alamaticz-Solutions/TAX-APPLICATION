import type { AppfwUiEntityContract, AppfwUiFieldContract } from "../generated/appfw-ui-contract";
import type { PdsDensity } from "@appfw/pds-health-components";
import type { FilterJoin, FilterOperator, FilterRuleDraft, SortDirection } from "./types";

const storagePrefix = "crm-grid-preferences-v1";
const filterOperators: ReadonlySet<string> = new Set([
  "_contains",
  "_not_contains",
  "_starts",
  "_ends",
  "_eq",
  "_ne",
  "_gt",
  "_gte",
  "_lt",
  "_lte",
  "_in",
  "_not_in"
]);

export type GridPreferences = {
  visibleColumnNames: string[];
  columnWidths: Record<string, number>;
  filterJoin: FilterJoin;
  filterRules: FilterRuleDraft[];
  sortField: string;
  sortDirection: SortDirection;
  appliedFilterJoin: FilterJoin;
  appliedFilterRules: FilterRuleDraft[];
  appliedSortField: string;
  appliedSortDirection: SortDirection;
  searchTerm: string;
  density: PdsDensity;
};

type StoredGridPreferences = Partial<GridPreferences>;

export function defaultGridPreferences(
  defaultColumns: string[],
  defaultDensity: PdsDensity = "compact"
): GridPreferences {
  return {
    visibleColumnNames: defaultColumns,
    columnWidths: {},
    filterJoin: "and",
    filterRules: [],
    sortField: "",
    sortDirection: "",
    appliedFilterJoin: "and",
    appliedFilterRules: [],
    appliedSortField: "",
    appliedSortDirection: "",
    searchTerm: "",
    density: defaultDensity
  };
}

export function readGridPreferences(
  entity: AppfwUiEntityContract,
  defaultColumns: string[],
  columnFields: AppfwUiFieldContract[],
  queryFields: AppfwUiFieldContract[],
  defaultDensity: PdsDensity = "compact"
): GridPreferences {
  const defaults = defaultGridPreferences(defaultColumns, defaultDensity);
  const raw = readJson<StoredGridPreferences>(storageKey(entity));
  if (!raw) return defaults;

  const columnNames = new Set(columnFields.map((field) => field.name));
  const queryFieldNames = new Set(queryFields.map((field) => field.name));
  const visibleColumnNames = sanitizeStringList(raw.visibleColumnNames, columnNames);
  const filterJoin = raw.filterJoin === "or" ? "or" : "and";
  const filterRules = sanitizeFilterRules(raw.filterRules, queryFieldNames);
  const sortField = typeof raw.sortField === "string" && columnNames.has(raw.sortField) ? raw.sortField : "";
  const hasAppliedSnapshot =
    Object.prototype.hasOwnProperty.call(raw, "appliedFilterRules") ||
    Object.prototype.hasOwnProperty.call(raw, "appliedSortField");
  const appliedFilterJoin = raw.appliedFilterJoin === "or" ? "or" : "and";
  const appliedFilterRules = hasAppliedSnapshot ? sanitizeFilterRules(raw.appliedFilterRules, queryFieldNames) : filterRules;
  const appliedSortField =
    typeof raw.appliedSortField === "string" && columnNames.has(raw.appliedSortField)
      ? raw.appliedSortField
      : hasAppliedSnapshot
        ? ""
        : sortField;

  return {
    visibleColumnNames: visibleColumnNames.length ? visibleColumnNames : defaults.visibleColumnNames,
    columnWidths: sanitizeColumnWidths(raw.columnWidths, columnNames),
    filterJoin,
    filterRules,
    sortField,
    sortDirection: sortField ? sanitizeSortDirection(raw.sortDirection) : "",
    appliedFilterJoin: hasAppliedSnapshot ? appliedFilterJoin : filterJoin,
    appliedFilterRules,
    appliedSortField,
    appliedSortDirection: appliedSortField
      ? sanitizeSortDirection(hasAppliedSnapshot ? raw.appliedSortDirection : raw.sortDirection)
      : "",
    searchTerm: typeof raw.searchTerm === "string" ? raw.searchTerm : "",
    density: sanitizeDensity(raw.density, defaults.density)
  };
}

export function writeGridPreferences(entity: AppfwUiEntityContract, preferences: GridPreferences) {
  writeJson(storageKey(entity), preferences);
}

function storageKey(entity: AppfwUiEntityContract) {
  return `${storagePrefix}:${entity.schemaName}.${entity.typeName}`;
}

function sanitizeStringList(value: unknown, allowed: ReadonlySet<string>) {
  if (!Array.isArray(value)) return [];
  return value.filter((item): item is string => typeof item === "string" && allowed.has(item));
}

function sanitizeColumnWidths(value: unknown, allowed: ReadonlySet<string>) {
  if (!value || typeof value !== "object") return {};
  return Object.fromEntries(
    Object.entries(value)
      .filter(([fieldName, width]) => allowed.has(fieldName) && typeof width === "number" && Number.isFinite(width))
      .map(([fieldName, width]) => [fieldName, Math.max(96, Math.min(560, Math.round(width as number)))])
  );
}

function sanitizeFilterRules(value: unknown, allowedFields: ReadonlySet<string>): FilterRuleDraft[] {
  if (!Array.isArray(value)) return [];
  return value
    .map((rule, index): FilterRuleDraft | null => {
      if (!rule || typeof rule !== "object") return null;
      const candidate = rule as Partial<FilterRuleDraft>;
      if (typeof candidate.fieldName !== "string" || !allowedFields.has(candidate.fieldName)) return null;
      if (typeof candidate.operator !== "string" || !filterOperators.has(candidate.operator)) return null;
      return {
        id: typeof candidate.id === "string" && candidate.id ? candidate.id : `cached-filter-${index}`,
        fieldName: candidate.fieldName,
        operator: candidate.operator as FilterOperator,
        value: typeof candidate.value === "string" ? candidate.value : ""
      };
    })
    .filter((rule): rule is FilterRuleDraft => Boolean(rule));
}

function sanitizeSortDirection(value: unknown): SortDirection {
  return value === "ASC" || value === "DESC" ? value : "";
}

function sanitizeDensity(value: unknown, fallback: PdsDensity): PdsDensity {
  return value === "compact" || value === "comfortable" ? value : fallback;
}

function readJson<T>(key: string): T | null {
  try {
    const raw = globalThis.localStorage?.getItem(key);
    return raw ? JSON.parse(raw) as T : null;
  } catch (_error) {
    return null;
  }
}

function writeJson(key: string, value: unknown) {
  try {
    globalThis.localStorage?.setItem(key, JSON.stringify(value));
  } catch (_error) {
    // Locked-down browsers simply lose persistence; the in-memory React state
    // still keeps the grid usable for the current session.
  }
}
