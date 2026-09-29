import type { DataType, EntityType, PropertyType, RecordValue } from "../types";
import { isNative } from "./entityModel";

const INTEGER_TYPES = new Set<DataType>(["Int8", "Int16", "Int32", "Int64"]);
const FLOAT_TYPES = new Set<DataType>(["Float32", "Float64"]);
const ARRAY_TYPES = new Set<DataType>([
  "UuidArray",
  "ObjectIdArray",
  "StringArray",
  "Int8Array",
  "Int16Array",
  "Int32Array",
  "Int64Array",
  "EnumArray",
  "JsonArray"
]);

/** Marks an Int64 digit string so GraphQL JSON can emit it as a raw number. */
const INT64_WIRE_PREFIX = "__APPFW_I64__";
const INT64_WIRE_SUFFIX = "__";
const INT64_WIRE_RE = /^__APPFW_I64__(-?\d+)__$/;
const I64_MIN = -9223372036854775808n;
const I64_MAX = 9223372036854775807n;

export function formProps(entity: EntityType, isCreate: boolean, includeReadOnly = false) {
  if (!isCreate && includeReadOnly) return entity.props.filter(isNative);
  const editable = entity.props.filter((prop) => isEditable(prop, isCreate));
  if (isCreate) return editable;
  const locked = entity.props.filter((prop) => isNative(prop) && (prop.is_key || prop.is_concurrency_control));
  return [...locked, ...editable.filter((prop) => !locked.includes(prop))];
}

export function defaultFormValues(entity: EntityType, isCreate: boolean, record: RecordValue = {}, includeReadOnly = false) {
  return formProps(entity, isCreate, includeReadOnly).reduce<RecordValue>((values, prop) => {
    values[prop.name] = !isCreate && prop.is_concurrency_control
      ? record[prop.name] ?? ""
      : record[prop.name] ?? initialValue(prop);
    return values;
  }, {});
}

export function normalizeInput(entity: EntityType, values: RecordValue, isCreate: boolean) {
  const props = formProps(entity, isCreate);
  return props.reduce<RecordValue>((input, prop) => {
    input[prop.name] = parseFieldValue(values[prop.name], prop);
    return input;
  }, {});
}

export function missingConcurrencyProps(entity: EntityType, values: RecordValue) {
  return formProps(entity, false).filter(
    (prop) => prop.is_concurrency_control && isEmpty(values[prop.name])
  );
}

export function inputType(prop: PropertyType) {
  // Int64 must stay exact through the control and submit path. HTML number
  // inputs and JS Number cannot safely represent every i64.
  if (prop.data_type === "Int64") return "text";
  if (INTEGER_TYPES.has(prop.data_type) || FLOAT_TYPES.has(prop.data_type)) return "number";
  if (prop.data_type === "Date") return "date";
  if (prop.data_type === "DateTime") return "datetime-local";
  if (prop.data_type === "Time") return "time";
  return "text";
}

export function normalizeInputValue(value: unknown, dataType: DataType) {
  if (value === null || value === undefined) return "";
  if (dataType === "DateTime" && typeof value === "string") return value.slice(0, 16);
  if (typeof value === "object") return JSON.stringify(value);
  // Form controls should show bare digits, never the GraphQL wire marker.
  if (dataType === "Int64" && typeof value === "string") {
    return unwrapInt64Wire(value) ?? value;
  }
  return String(value);
}

export function acceptsJsonInput(dataType: DataType) {
  return dataType === "Json" || dataType === "JsonArray" || ARRAY_TYPES.has(dataType);
}

/**
 * Serialize a GraphQL request body, emitting Int64 wire markers as raw JSON
 * numbers so digit fidelity survives past Number.MAX_SAFE_INTEGER.
 */
export function stringifyGraphqlRequest(query: string, variables: Record<string, unknown>) {
  const variablesJson = emitInt64WireNumbers(JSON.stringify(variables ?? {}));
  return `{"query":${JSON.stringify(query)},"variables":${variablesJson}}`;
}

/** Pretty-print variables the same way they go on the wire (raw Int64 numbers). */
export function formatGraphqlVariables(variables: unknown) {
  return emitInt64WireNumbers(JSON.stringify(variables ?? {}, null, 2));
}

function isEditable(prop: PropertyType, isCreate: boolean) {
  if (!isNative(prop)) return false;
  if (prop.is_read_only || prop.is_concurrency_control) return false;
  if (prop.computed && prop.computed !== "None") return false;
  if (isCreate && prop.is_key) return false;
  return true;
}

function initialValue(prop: PropertyType) {
  if (prop.data_type === "Boolean") return false;
  if (prop.default_value && typeof prop.default_value === "object" && Object.keys(prop.default_value).length === 0) {
    return "";
  }
  return prop.default_value ?? "";
}

function parseFieldValue(value: unknown, prop: PropertyType) {
  if (isEmpty(value)) return null;
  // Keep Int64 as an exact digit string (wire-marked for GraphQL JSON). Never
  // Number.parseInt / Number — those silently round above MAX_SAFE_INTEGER.
  if (prop.data_type === "Int64") return toInt64Wire(value);
  if (INTEGER_TYPES.has(prop.data_type)) return Number.parseInt(String(value), 10);
  if (FLOAT_TYPES.has(prop.data_type)) return Number.parseFloat(String(value));
  if (prop.data_type === "Boolean") return Boolean(value);
  if (acceptsJsonInput(prop.data_type)) {
    if (typeof value !== "string") return value;
    try {
      return JSON.parse(value);
    } catch {
      return value;
    }
  }
  if (prop.data_type === "DateTime" && typeof value === "string" && value.length === 16) return `${value}:00Z`;
  return value;
}

function toInt64Wire(value: unknown): string {
  const raw = typeof value === "string" ? unwrapInt64Wire(value) ?? value.trim() : String(value).trim();
  if (!/^-?\d+$/.test(raw)) {
    // Leave non-digits unchanged; server validation reports the error.
    return raw;
  }
  try {
    const asBig = BigInt(raw);
    if (asBig < I64_MIN || asBig > I64_MAX) {
      return raw;
    }
  } catch {
    return raw;
  }
  return `${INT64_WIRE_PREFIX}${raw}${INT64_WIRE_SUFFIX}`;
}

function unwrapInt64Wire(value: string) {
  const match = INT64_WIRE_RE.exec(value);
  return match ? match[1] : null;
}

function emitInt64WireNumbers(json: string) {
  return json.replace(/"__APPFW_I64__(-?\d+)__"/g, "$1");
}

function isEmpty(value: unknown) {
  return value === "" || value === null || value === undefined;
}
