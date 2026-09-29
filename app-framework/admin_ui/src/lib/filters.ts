import type {
  AdvancedFilterGroup,
  AdvancedFilterNode,
  AdvancedFilterOperator,
  AdvancedFilterRule,
  AdvancedFilterState,
  DataType,
  EntityType,
  FilterCapabilities,
  FilterOperatorCapability,
  FilterValueShape,
  PropertyType,
  RecordValue
} from "../types";
import { queryProps } from "./entityModel";

const NUMERIC_TYPES = new Set<DataType>(["Int8", "Int16", "Int32", "Int64", "Float32", "Float64"]);
const ARRAY_TYPES = new Set<DataType>([
  "UuidArray",
  "ObjectIdArray",
  "StringArray",
  "Int8Array",
  "Int16Array",
  "Int32Array",
  "Int64Array",
  "EnumArray"
]);

const FALLBACK_STRING_OPERATORS: FilterOperatorCapability[] = [
  operator("_contains", "contains", "scalar"),
  operator("_not_contains", "does not contain", "scalar"),
  operator("_starts", "starts with", "scalar"),
  operator("_ends", "ends with", "scalar"),
  operator("_eq", "equals", "scalar"),
  operator("_ne", "does not equal", "scalar"),
  operator("_in", "is one of", "list"),
  operator("_not_in", "is not one of", "list")
];

const FALLBACK_ENUM_OPERATORS: FilterOperatorCapability[] = [
  operator("_eq", "equals", "scalar"),
  operator("_ne", "does not equal", "scalar"),
  operator("_in", "is one of", "list"),
  operator("_not_in", "is not one of", "list")
];

const FALLBACK_ID_OPERATORS: FilterOperatorCapability[] = [
  operator("_eq", "equals", "scalar"),
  operator("_ne", "does not equal", "scalar"),
  operator("_in", "is one of", "list"),
  operator("_not_in", "is not one of", "list")
];

const FALLBACK_BOOLEAN_OPERATORS: FilterOperatorCapability[] = [
  operator("_eq", "is", "scalar"),
  operator("_ne", "is not", "scalar")
];

const FALLBACK_COMPARISON_OPERATORS: FilterOperatorCapability[] = [
  operator("_eq", "equals", "scalar"),
  operator("_ne", "does not equal", "scalar"),
  operator("_gt", "greater than", "scalar"),
  operator("_gte", "greater than or equal", "scalar"),
  operator("_lt", "less than", "scalar"),
  operator("_lte", "less than or equal", "scalar"),
  operator("_in", "is one of", "list"),
  operator("_not_in", "is not one of", "list")
];

const FALLBACK_TEMPORAL_OPERATORS: FilterOperatorCapability[] = [
  operator("_eq", "equals", "scalar"),
  operator("_ne", "does not equal", "scalar"),
  operator("_gt", "after", "scalar"),
  operator("_gte", "on or after", "scalar"),
  operator("_lt", "before", "scalar"),
  operator("_lte", "on or before", "scalar"),
  operator("_before", "before period", "period"),
  operator("_during", "during period", "period"),
  operator("_after", "after period", "period")
];

const FALLBACK_ARRAY_OPERATORS: FilterOperatorCapability[] = [
  operator("_contains", "contains all", "scalar_or_list"),
  operator("_not_contains", "does not contain all", "scalar_or_list"),
  operator("_overlaps", "overlaps", "list"),
  operator("_not_overlaps", "does not overlap", "list"),
  operator("_contained_by", "is contained by", "list"),
  operator("_not_contained_by", "is not contained by", "list")
];

export const EMPTY_ADVANCED_FILTER: AdvancedFilterState = {
  kind: "group",
  id: "root",
  join: "and",
  children: []
};

export function filterableProps(entity: EntityType, capabilities?: FilterCapabilities | null) {
  const props = queryProps(entity);
  if (capabilities) {
    const dataTypes = new Set(
      capabilities.data_types
        .filter((capability) => capability.operators.some((operator) => operator.supported))
        .map((capability) => capability.data_type)
    );
    return props.filter((prop) => dataTypes.has(prop.data_type));
  }
  return props.filter((prop) => fallbackOperatorCapabilitiesForProp(prop).length > 0);
}

export function operatorCapabilitiesForProp(
  capabilities: FilterCapabilities | null | undefined,
  prop: PropertyType
): FilterOperatorCapability[] {
  if (capabilities) {
    return capabilities.data_types.find((capability) => capability.data_type === prop.data_type)?.operators ?? [];
  }
  return fallbackOperatorCapabilitiesForProp(prop);
}

export function defaultOperatorForProp(
  prop: PropertyType,
  capabilities?: FilterCapabilities | null
): AdvancedFilterOperator {
  const operators = supportedOperatorCapabilitiesForProp(capabilities, prop);
  return operators.find((item) => item.op === "_contains")?.op ?? operators[0]?.op ?? "_eq";
}

export function activeFilterRules(
  entity: EntityType | null,
  filter: AdvancedFilterState,
  capabilities?: FilterCapabilities | null
) {
  if (!entity) return [];
  const propsById = new Map(filterableProps(entity, capabilities).map((prop) => [prop.id, prop]));
  return collectRules(filter).filter((rule) => {
    const prop = propsById.get(rule.propId);
    const operatorCapability = prop ? capabilityForRule(capabilities, prop, rule.operator) : null;
    return Boolean(prop && operatorCapability && coerceRuleValue(prop, rule, operatorCapability) !== undefined);
  });
}

export function advancedFilterToJson(
  entity: EntityType | null,
  filter: AdvancedFilterState,
  capabilities?: FilterCapabilities | null
) {
  if (!entity) return null;
  const propsById = new Map(filterableProps(entity, capabilities).map((prop) => [prop.id, prop]));
  return groupToJson(filter, propsById, capabilities);
}

export function normalizeAdvancedFilter(
  entity: EntityType | null,
  filter: AdvancedFilterState | null | undefined,
  capabilities?: FilterCapabilities | null
): AdvancedFilterState {
  if (!entity || !filter) return EMPTY_ADVANCED_FILTER;
  const props = filterableProps(entity, capabilities);
  const propsById = new Map(props.map((prop) => [prop.id, prop]));
  const legacy = filter as unknown as LegacyAdvancedFilter;
  if (Array.isArray(legacy.rules)) {
    return {
      kind: "group",
      id: "root",
      join: legacy.join === "or" ? "or" : "and",
      children: legacy.rules
        .map((rule) => normalizeRule(rule, propsById, capabilities))
        .filter((rule): rule is AdvancedFilterRule => Boolean(rule))
    };
  }
  return {
    kind: "group",
    id: filter.id || "root",
    join: filter.join === "or" ? "or" : "and",
    children: normalizeNodes(filter.children, propsById, capabilities)
  };
}

export function recordMatchesFilter(record: RecordValue, filter: Record<string, unknown> | null | undefined): boolean {
  if (!filter || Object.keys(filter).length === 0) return true;
  return Object.entries(filter).every(([key, value]) => {
    if (key === "_and") {
      return Array.isArray(value) && value.every((item) => recordMatchesFilter(record, asFilter(item)));
    }
    if (key === "_or") {
      return Array.isArray(value) && value.some((item) => recordMatchesFilter(record, asFilter(item)));
    }
    if (value && typeof value === "object" && !Array.isArray(value)) {
      return Object.entries(value as Record<string, unknown>).every(([operator, expected]) =>
        matchOperator(record[key], operator as AdvancedFilterOperator, expected)
      );
    }
    return record[key] === value;
  });
}

function operator(
  op: AdvancedFilterOperator,
  label: string,
  valueShape: FilterValueShape
): FilterOperatorCapability {
  return { op, label, value_shape: valueShape, supported: true };
}

function fallbackOperatorCapabilitiesForProp(prop: PropertyType): FilterOperatorCapability[] {
  if (prop.data_type === "Boolean") return FALLBACK_BOOLEAN_OPERATORS;
  if (prop.data_type === "String") return FALLBACK_STRING_OPERATORS;
  if (prop.data_type === "Uuid" || prop.data_type === "ObjectId") return FALLBACK_ID_OPERATORS;
  if (prop.data_type === "Enum") return FALLBACK_ENUM_OPERATORS;
  if (prop.data_type === "Date" || prop.data_type === "DateTime") return FALLBACK_TEMPORAL_OPERATORS;
  if (prop.data_type === "Time") return FALLBACK_TEMPORAL_OPERATORS.filter((item) => item.value_shape !== "period");
  if (NUMERIC_TYPES.has(prop.data_type)) return FALLBACK_COMPARISON_OPERATORS;
  if (ARRAY_TYPES.has(prop.data_type)) return FALLBACK_ARRAY_OPERATORS;
  return [];
}

function capabilityForRule(
  capabilities: FilterCapabilities | null | undefined,
  prop: PropertyType,
  operator: AdvancedFilterOperator
) {
  return supportedOperatorCapabilitiesForProp(capabilities, prop).find((item) => item.op === operator) ?? null;
}

function supportedOperatorCapabilitiesForProp(
  capabilities: FilterCapabilities | null | undefined,
  prop: PropertyType
) {
  return operatorCapabilitiesForProp(capabilities, prop).filter((item) => item.supported);
}

function asFilter(value: unknown) {
  return value && typeof value === "object" && !Array.isArray(value) ? (value as Record<string, unknown>) : null;
}

function groupToJson(
  group: AdvancedFilterGroup,
  propsById: Map<string, PropertyType>,
  capabilities?: FilterCapabilities | null
): Record<string, unknown> | null {
  const clauses = group.children
    .map((child) => {
      if (child.kind === "group") return groupToJson(child, propsById, capabilities);
      const prop = propsById.get(child.propId);
      if (!prop) return null;
      const operatorCapability = capabilityForRule(capabilities, prop, child.operator);
      if (!operatorCapability) return null;
      const value = coerceRuleValue(prop, child, operatorCapability);
      if (value === undefined) return null;
      return { [prop.name]: { [child.operator]: value } } as Record<string, unknown>;
    })
    .filter((clause): clause is Record<string, unknown> => Boolean(clause));

  if (clauses.length === 0) return null;
  if (clauses.length === 1) return clauses[0];
  return { [group.join === "or" ? "_or" : "_and"]: clauses };
}

function collectRules(node: AdvancedFilterNode): AdvancedFilterRule[] {
  if (node.kind === "rule") return [node];
  return node.children.flatMap(collectRules);
}

function normalizeNodes(
  nodes: AdvancedFilterNode[] | undefined,
  propsById: Map<string, PropertyType>,
  capabilities?: FilterCapabilities | null
): AdvancedFilterNode[] {
  if (!Array.isArray(nodes)) return [];
  return nodes
    .map((node) => {
      if (node.kind === "group") {
        return {
          kind: "group",
          id: node.id || createStableId(),
          join: node.join === "or" ? "or" : "and",
          children: normalizeNodes(node.children, propsById, capabilities)
        } satisfies AdvancedFilterGroup;
      }
      if (node.kind === "rule") {
        return normalizeRule(node, propsById, capabilities);
      }
      return null;
    })
    .filter((node): node is AdvancedFilterNode => Boolean(node));
}

function normalizeRule(
  rule: Omit<AdvancedFilterRule, "kind"> & { kind?: "rule" },
  propsById: Map<string, PropertyType>,
  capabilities?: FilterCapabilities | null
) {
  const prop = propsById.get(rule.propId);
  if (!prop) return null;
  const operators = supportedOperatorCapabilitiesForProp(capabilities, prop);
  const operator = operators.some((item) => item.op === rule.operator)
    ? rule.operator
    : defaultOperatorForProp(prop, capabilities);
  return {
    kind: "rule",
    id: rule.id || createStableId(),
    propId: rule.propId,
    operator,
    value: rule.value ?? ""
  } satisfies AdvancedFilterRule;
}

function createStableId() {
  return `filter-${Math.random().toString(36).slice(2)}`;
}

function coerceRuleValue(
  prop: PropertyType,
  rule: AdvancedFilterRule,
  operatorCapability: FilterOperatorCapability
): unknown {
  const raw = rule.value.trim();
  if (!raw) return undefined;
  if (operatorCapability.value_shape === "period") return raw;
  if (operatorCapability.value_shape === "list") return coerceListValue(prop, raw);
  if (operatorCapability.value_shape === "scalar_or_list") {
    if (raw.includes(",") || raw.includes("\n")) return coerceListValue(prop, raw);
    return coerceScalarValue(prop, raw);
  }
  return coerceScalarValue(prop, raw);
}

function coerceListValue(prop: PropertyType, raw: string) {
  const items = raw
    .split(/[\n,]/)
    .map((item) => item.trim())
    .filter(Boolean)
    .map((item) => coerceScalarValue(prop, item))
    .filter((item) => item !== undefined);
  return items.length ? items : undefined;
}

type LegacyAdvancedFilter = {
  join?: "and" | "or";
  rules?: Array<Omit<AdvancedFilterRule, "kind"> & { kind?: "rule" }>;
};

function coerceScalarValue(prop: PropertyType, raw: string): unknown {
  if (prop.data_type === "Boolean") {
    if (raw === "true") return true;
    if (raw === "false") return false;
    return undefined;
  }
  if (NUMERIC_TYPES.has(prop.data_type) || prop.data_type.startsWith("Int")) {
    const numberValue = Number(raw);
    return Number.isFinite(numberValue) ? numberValue : undefined;
  }
  return raw;
}

function matchOperator(actual: unknown, operator: AdvancedFilterOperator, expected: unknown) {
  switch (operator) {
    case "_eq":
      return compare(actual, expected) === 0;
    case "_ne":
      return compare(actual, expected) !== 0;
    case "_contains":
      if (Array.isArray(actual)) return arrayContainsAll(actual, expected);
      return String(actual ?? "").toLowerCase().includes(String(expected ?? "").toLowerCase());
    case "_not_contains":
      if (Array.isArray(actual)) return !arrayContainsAll(actual, expected);
      return !String(actual ?? "").toLowerCase().includes(String(expected ?? "").toLowerCase());
    case "_starts":
      return String(actual ?? "").toLowerCase().startsWith(String(expected ?? "").toLowerCase());
    case "_ends":
      return String(actual ?? "").toLowerCase().endsWith(String(expected ?? "").toLowerCase());
    case "_regex":
      return regexMatches(actual, expected);
    case "_overlaps":
      return arraysOverlap(actual, expected);
    case "_not_overlaps":
      return !arraysOverlap(actual, expected);
    case "_contained_by":
      return arrayContainedBy(actual, expected);
    case "_not_contained_by":
      return !arrayContainedBy(actual, expected);
    case "_lt":
      return compare(actual, expected) < 0;
    case "_lte":
      return compare(actual, expected) <= 0;
    case "_gt":
      return compare(actual, expected) > 0;
    case "_gte":
      return compare(actual, expected) >= 0;
    case "_in":
      return Array.isArray(expected) && expected.some((item) => compare(actual, item) === 0);
    case "_not_in":
      return Array.isArray(expected) && expected.every((item) => compare(actual, item) !== 0);
    case "_before":
      return compareToPeriod(actual, expected, "before");
    case "_during":
      return compareToPeriod(actual, expected, "during");
    case "_after":
      return compareToPeriod(actual, expected, "after");
    default:
      return false;
  }
}

function regexMatches(actual: unknown, expected: unknown) {
  try {
    return new RegExp(String(expected ?? ""), "i").test(String(actual ?? ""));
  } catch {
    return false;
  }
}

function arrayContainsAll(actual: unknown[], expected: unknown) {
  const expectedItems = Array.isArray(expected) ? expected : [expected];
  return expectedItems.every((item) => actual.some((actualItem) => compare(actualItem, item) === 0));
}

function arraysOverlap(actual: unknown, expected: unknown) {
  if (!Array.isArray(actual) || !Array.isArray(expected)) return false;
  return expected.some((item) => actual.some((actualItem) => compare(actualItem, item) === 0));
}

function arrayContainedBy(actual: unknown, expected: unknown) {
  if (!Array.isArray(actual) || !Array.isArray(expected)) return false;
  return actual.every((actualItem) => expected.some((item) => compare(actualItem, item) === 0));
}

function compareToPeriod(actual: unknown, expected: unknown, mode: "before" | "during" | "after") {
  const bounds = periodBounds(String(expected ?? ""));
  if (!bounds) {
    if (mode === "during") return compare(actual, expected) === 0;
    return mode === "before" ? compare(actual, expected) < 0 : compare(actual, expected) > 0;
  }
  const actualTime = Date.parse(String(actual ?? ""));
  if (!Number.isFinite(actualTime)) return false;
  if (mode === "before") return actualTime < bounds.start;
  if (mode === "after") return actualTime >= bounds.end;
  return actualTime >= bounds.start && actualTime < bounds.end;
}

function periodBounds(raw: string) {
  const value = raw.trim();
  if (/^\d{4}$/.test(value)) {
    const year = Number(value);
    return { start: Date.UTC(year, 0, 1), end: Date.UTC(year + 1, 0, 1) };
  }
  if (/^\d{4}-\d{2}$/.test(value)) {
    const [year, month] = value.split("-").map(Number);
    return { start: Date.UTC(year, month - 1, 1), end: Date.UTC(year, month, 1) };
  }
  if (/^\d{4}-\d{2}-\d{2}$/.test(value)) {
    const [year, month, day] = value.split("-").map(Number);
    return { start: Date.UTC(year, month - 1, day), end: Date.UTC(year, month - 1, day + 1) };
  }
  const parsed = Date.parse(value);
  return Number.isFinite(parsed) ? { start: parsed, end: parsed + 1 } : null;
}

function compare(actual: unknown, expected: unknown) {
  if (actual === expected) return 0;
  const actualNumber = typeof actual === "number" ? actual : Number(actual);
  const expectedNumber = typeof expected === "number" ? expected : Number(expected);
  if (Number.isFinite(actualNumber) && Number.isFinite(expectedNumber)) {
    return actualNumber === expectedNumber ? 0 : actualNumber > expectedNumber ? 1 : -1;
  }
  const actualDate = typeof actual === "string" ? Date.parse(actual) : NaN;
  const expectedDate = typeof expected === "string" ? Date.parse(expected) : NaN;
  if (Number.isFinite(actualDate) && Number.isFinite(expectedDate)) {
    return actualDate === expectedDate ? 0 : actualDate > expectedDate ? 1 : -1;
  }
  return String(actual ?? "").localeCompare(String(expected ?? ""), undefined, {
    numeric: true,
    sensitivity: "base"
  });
}
