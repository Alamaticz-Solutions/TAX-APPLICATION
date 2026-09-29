import { crmUiContract, type AppfwUiEntityContract, type AppfwUiFieldContract, type AppfwUiOperationContract } from "../generated/appfw-ui-contract";
import { AppfwClientError, type AppfwEntityListData, type AppfwOperationError, type AppfwRecord } from "../lib/appfwClient";
import { routeIdField } from "./routeIdentity";
import type { FilterJoin, FilterOperator, FilterOperatorOption, FilterRuleDraft, LookupOption, RelationshipSelectorMode, SortDirection } from "./types";

export const DELETE_CONFIRMATION_DISABLED_REASON = "Generated delete actions require product confirmation UX before enabling.";

export function scaffoldFields(entity: AppfwUiEntityContract, surface: "list" | "detail" | "edit") {
  const fieldsByName = new Map(entity.fields.map((field) => [field.name, field]));
  const names = entity.scaffold[surface].fields.length
    ? entity.scaffold[surface].fields
    : entity.fields.filter((field) => field.ui[surface]).map((field) => field.name);
  return names
    .map((name) => fieldsByName.get(name))
    .filter((field): field is AppfwUiFieldContract => Boolean(field));
}

export function formFields(entity: AppfwUiEntityContract) {
  const baseFields = scaffoldFields(entity, "edit");
  const names = new Set(baseFields.map((field) => field.name));
  const lookupFields = entity.fields.filter(
    (field) =>
      isLookupFilterField(field) &&
      !field.readOnly &&
      !field.isConcurrencyControl &&
      /^[_A-Za-z][_0-9A-Za-z]*$/.test(field.name) &&
      !names.has(field.name)
  );
  return [...baseFields, ...lookupFields];
}

export function formSelectionFields(entity: AppfwUiEntityContract) {
  const selected = new Set<string>();
  addSelectionField(selected, entity.primaryKey);
  addSelectionField(selected, routeIdField(entity));
  addSelectionField(selected, entity.captionField);
  for (const field of [...scaffoldFields(entity, "detail"), ...formFields(entity)]) {
    if (isFlatRecordSelectionField(field)) addSelectionField(selected, field.name);
  }
  for (const operation of entity.operations) {
    if (operation.optimisticConcurrencyField) addSelectionField(selected, operation.optimisticConcurrencyField);
  }
  return Array.from(selected);
}

export function gridFields(entity: AppfwUiEntityContract) {
  return entity.fields.filter(
    (field) =>
      field.kind === "scalar" &&
      field.ui.list &&
      field.name !== entity.primaryKey &&
      !field.isKey &&
      !field.isConcurrencyControl &&
      field.name &&
      /^[_A-Za-z][_0-9A-Za-z]*$/.test(field.name)
  );
}

export function gridSelection(entity: AppfwUiEntityContract, visibleColumns: readonly string[]) {
  const fieldsByName = new Map(entity.fields.map((field) => [field.name, field]));
  const selected = new Set<string>();
  addSelectionField(selected, entity.primaryKey);
  addSelectionField(selected, routeIdField(entity));
  for (const column of visibleColumns) {
    const field = fieldsByName.get(column);
    if (!field) continue;
    if (isFlatRecordSelectionField(field)) addSelectionField(selected, field.name);
    const lookupForeignKey = lookupForeignKeyForGridColumn(field);
    if (lookupForeignKey) addSelectionField(selected, lookupForeignKey);
  }
  return Array.from(selected);
}

export function queryBuilderFields(entity: AppfwUiEntityContract) {
  return entity.fields.filter(
    (field) =>
      isQueryFilterField(field) &&
      !field.isConcurrencyControl &&
      field.name &&
      /^[_A-Za-z][_0-9A-Za-z]*$/.test(field.name)
  );
}

export function relatedCollectionFields(entity: AppfwUiEntityContract) {
  return entity.fields.filter(
    (field) =>
      field.kind === "relationship" &&
      field.dataType === "NavToMany" &&
      field.relationship?.kind === "nav_to_many" &&
      Boolean(field.relationship.fieldName) &&
      Boolean(field.relationshipTarget || field.relationship.typeName)
  );
}

export function isQueryFilterField(field: AppfwUiFieldContract) {
  return (field.kind === "scalar" && field.ui.filterable) || isLookupFilterField(field);
}

export function createFilterRule(fields: AppfwUiFieldContract[]): FilterRuleDraft | null {
  const field = fields[0];
  if (!field) return null;
  return {
    id: createFilterId(),
    fieldName: field.name,
    operator: defaultOperatorForField(field),
    value: defaultFilterValue(field)
  };
}

export function filterRulesToJson(
  fields: AppfwUiFieldContract[],
  rules: FilterRuleDraft[],
  join: FilterJoin
): Record<string, unknown> | undefined {
  const fieldsByName = new Map(fields.map((field) => [field.name, field]));
  const clauses = rules
    .map((rule): Record<string, unknown> | null => {
      const field = fieldsByName.get(rule.fieldName);
      if (!field) {
        throw new Error(`Unknown filter field: ${rule.fieldName}`);
      }
      const operator = operatorOptionsForField(field).find((candidate) => candidate.value === rule.operator);
      if (!operator) {
        throw new Error(`${field.label} does not support ${rule.operator}.`);
      }
      const value = coerceFilterValue(field, rule.value, operator);
      if (value === undefined) return null;
      return { [field.name]: { [rule.operator]: value } };
    })
    .filter((clause): clause is Record<string, unknown> => clause !== null);

  if (!clauses.length) return undefined;
  if (clauses.length === 1) return clauses[0];
  return { [join === "or" ? "_or" : "_and"]: clauses };
}

export function sortInput(fieldName: string, direction: SortDirection) {
  if (!fieldName || !direction) return undefined;
  return { [fieldName]: direction };
}

export function operatorOptionsForField(field: AppfwUiFieldContract): FilterOperatorOption[] {
  if (isLookupFilterField(field)) {
    return [
      { value: "_eq", label: "equals" },
      { value: "_ne", label: "does not equal" },
      { value: "_in", label: "is one of", list: true },
      { value: "_not_in", label: "is not one of", list: true }
    ];
  }
  if (isBooleanField(field)) {
    return [
      { value: "_eq", label: "is" },
      { value: "_ne", label: "is not" }
    ];
  }
  if (isNumericField(field)) {
    return [
      { value: "_eq", label: "equals" },
      { value: "_ne", label: "does not equal" },
      { value: "_gt", label: "greater than" },
      { value: "_gte", label: "greater than or equal" },
      { value: "_lt", label: "less than" },
      { value: "_lte", label: "less than or equal" },
      { value: "_in", label: "is one of", list: true },
      { value: "_not_in", label: "is not one of", list: true }
    ];
  }
  if (isTemporalField(field)) {
    return [
      { value: "_eq", label: "equals" },
      { value: "_ne", label: "does not equal" },
      { value: "_gt", label: "after" },
      { value: "_gte", label: "on or after" },
      { value: "_lt", label: "before" },
      { value: "_lte", label: "on or before" }
    ];
  }
  if (isArrayField(field)) {
    return [
      { value: "_contains", label: "contains all", list: true },
      { value: "_not_contains", label: "does not contain all", list: true }
    ];
  }
  if (isIdentifierField(field) || field.enumTypeName) {
    return [
      { value: "_eq", label: "equals" },
      { value: "_ne", label: "does not equal" },
      { value: "_in", label: "is one of", list: true },
      { value: "_not_in", label: "is not one of", list: true }
    ];
  }
  return [
    { value: "_contains", label: "contains" },
    { value: "_not_contains", label: "does not contain" },
    { value: "_starts", label: "starts with" },
    { value: "_ends", label: "ends with" },
    { value: "_eq", label: "equals" },
    { value: "_ne", label: "does not equal" },
    { value: "_in", label: "is one of", list: true },
    { value: "_not_in", label: "is not one of", list: true }
  ];
}

export function defaultOperatorForField(field: AppfwUiFieldContract): FilterOperator {
  return operatorOptionsForField(field)[0]?.value ?? "_eq";
}

export function filterInputType(field: AppfwUiFieldContract) {
  if (isNumericField(field)) return "number";
  if (field.dataType === "Date") return "date";
  if (field.dataType === "DateTime") return "datetime-local";
  if (field.dataType === "Time") return "time";
  return "text";
}

export function fieldTypeLabel(field: AppfwUiFieldContract) {
  if (isForeignKeyRelationshipField(field)) {
    const target = field.relationshipTarget ?? field.relationship?.typeName ?? "record";
    return relationshipSelectorMode(field) === "lookup" ? `Lookup:${target}` : `Entity:${target}`;
  }
  if (isCurrencyField(field)) return `${field.dataType}:currency`;
  if (field.enumTypeName) return `${field.dataType}:${field.enumTypeName}`;
  return field.dataType;
}

export function isForeignKeyRelationshipField(field: AppfwUiFieldContract) {
  return (
    field.kind === "relationship" &&
    field.ui.formControl === "relationship-picker" &&
    field.relationship?.kind === "foreign_key" &&
    Boolean(field.relationshipTarget || field.relationship.typeName) &&
    !field.dataType.startsWith("NavTo")
  );
}

export function isLookupFilterField(field: AppfwUiFieldContract) {
  return isForeignKeyRelationshipField(field);
}

export function isLookupSelectorOperator(operator: FilterOperatorOption) {
  return operator.value === "_eq" || operator.value === "_ne" || operator.value === "_in" || operator.value === "_not_in";
}

export function isFullEntitySelectorField(field: AppfwUiFieldContract) {
  return (
    field.kind === "relationship" &&
    field.ui.formControl === "relationship-picker" &&
    (field.dataType === "NavToOne" || field.dataType === "NavToMany") &&
    Boolean(field.relationshipTarget || field.relationship?.typeName)
  );
}

export function relationshipSelectorMode(field: AppfwUiFieldContract): RelationshipSelectorMode | null {
  if (
    field.kind !== "relationship" ||
    field.ui.formControl !== "relationship-picker" ||
    (!field.relationshipTarget && !field.relationship?.typeName)
  ) {
    return null;
  }
  if (field.dataType === "NavToMany") return "entity-list";
  const targetEntity = lookupTargetEntity(field);
  if (!targetEntity) return null;
  return isReferenceLookupEntity(targetEntity) ? "lookup" : "entity-list";
}

export function relationshipClearDisabledReason(
  ownerEntity: AppfwUiEntityContract,
  field: AppfwUiFieldContract
) {
  if (!isFullEntitySelectorField(field) && !isForeignKeyRelationshipField(field)) return null;
  if (field.required || field.validation.required) return `${field.label} is required.`;

  const relationship = field.relationship;
  const foreignKeyFieldName =
    relationship?.kind === "foreign_key"
      ? field.name
      : relationship?.kind === "nav_to_one"
        ? relationship.fieldName
        : undefined;
  if (!foreignKeyFieldName) return null;

  const targetEntity = lookupTargetEntity(field);
  if (!targetEntity) return null;

  const inverseCollection = relatedCollectionFields(targetEntity).find((candidate) => {
    const candidateTarget = candidate.relationshipTarget ?? candidate.relationship?.typeName;
    const candidateSchema = candidate.relationship?.schemaName ?? ownerEntity.schemaName;
    return (
      candidateTarget === ownerEntity.typeName &&
      candidateSchema === ownerEntity.schemaName &&
      candidate.relationship?.fieldName === foreignKeyFieldName
    );
  });
  if (inverseCollection) {
    return `${field.label} is a parent relationship. Open the parent ${targetEntity.caption.singular.toLowerCase()} to manage linked ${ownerEntity.caption.plural.toLowerCase()}.`;
  }

  const backingField = ownerEntity.fields.find((candidate) => candidate.name === foreignKeyFieldName);
  if (backingField?.required || backingField?.validation.required) {
    return `${field.label} is backed by required field ${humanizeFieldName(foreignKeyFieldName)}.`;
  }

  return null;
}

export function isReferenceLookupEntity(entity: AppfwUiEntityContract) {
  const businessScalarFields = entity.fields.filter(isBusinessScalarField);
  const editableRelationshipFields = entity.fields.filter((field) => field.kind === "relationship" && field.ui.edit);
  const visibleRelationshipFields = entity.fields.filter(
    (field) => field.kind === "relationship" && (field.ui.list || field.ui.detail || field.ui.edit)
  );
  const visibleListFields = entity.scaffold.list.fields.filter((name) => {
    const field = entity.fields.find((candidate) => candidate.name === name);
    return field ? isBusinessScalarField(field) : false;
  });

  return (
    !entity.audited &&
    businessScalarFields.length > 0 &&
    businessScalarFields.length <= 5 &&
    visibleListFields.length <= 6 &&
    editableRelationshipFields.length === 0 &&
    visibleRelationshipFields.length === 0 &&
    businessScalarFields.some((field) => field.name === entity.captionField)
  );
}

export function isCurrencyField(field: AppfwUiFieldContract) {
  return field.ui.format === "currency";
}

export function lookupTargetEntity(field: AppfwUiFieldContract): AppfwUiEntityContract | undefined {
  const targetName = field.relationshipTarget ?? field.relationship?.typeName;
  const schemaName = field.relationship?.schemaName ?? "crm";
  if (!targetName) return undefined;
  return crmUiContract.entities.find(
    (entity) => entity.schemaName === schemaName && entity.typeName === targetName
  );
}

export function lookupSelection(entity: AppfwUiEntityContract, mode: RelationshipSelectorMode = "lookup") {
  const routeField = routeIdField(entity);
  const fields = mode === "entity-list"
    ? [entity.primaryKey, routeField, entity.captionField, ...gridFields(entity).slice(0, 5).map((field) => field.name)]
    : [entity.primaryKey, routeField, entity.captionField];
  return Array.from(new Set(fields.filter((name) => name && isGraphqlName(name))));
}

export function lookupSort(entity: AppfwUiEntityContract) {
  const caption = entity.fields.find((field) => field.name === entity.captionField);
  if (!caption?.ui.sortable) return undefined;
  return { [entity.captionField]: "ASC" };
}

export function lookupOptionsFromRows(entity: AppfwUiEntityContract, rows: AppfwRecord[]): LookupOption[] {
  return rows
    .map((row) => {
      const value = String(row[entity.primaryKey] ?? "");
      if (!value) return null;
      const label = String(row[entity.captionField] ?? value);
      return { value, label };
    })
    .filter((option): option is LookupOption => option !== null);
}

export function selectedLookupValues(value: string) {
  return value
    .split(/[\n,]/)
    .map((candidate) => candidate.trim())
    .filter(Boolean);
}

export function relationshipSelectedKeys(value: unknown, targetEntity: AppfwUiEntityContract | undefined) {
  const keys = new Set<string>();
  if (!targetEntity) return keys;
  const addValue = (candidate: unknown) => {
    if (isRecordValue(candidate)) {
      const key = candidate[targetEntity.primaryKey];
      if (key !== null && key !== undefined) keys.add(String(key));
      return;
    }
    if (typeof candidate === "string" && candidate) keys.add(candidate);
  };
  if (Array.isArray(value)) {
    value.forEach(addValue);
  } else {
    addValue(value);
  }
  return keys;
}

export function isRecordValue(value: unknown): value is AppfwRecord {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

export function humanizeFieldName(value: string) {
  return value
    .replace(/_/g, " ")
    .replace(/([a-z0-9])([A-Z])/g, "$1 $2")
    .replace(/\b\w/g, (letter) => letter.toUpperCase());
}

export function rawCurrencyValue(value: unknown) {
  if (value === null || value === undefined || value === "") return "";
  const parsed = typeof value === "number" ? value : Number(String(value).replace(/[$,\s]/g, ""));
  if (!Number.isFinite(parsed)) return String(value);
  return String(parsed);
}

export function formatCurrencyValue(value: unknown) {
  if (value === null || value === undefined || value === "") return "";
  const parsed = typeof value === "number" ? value : Number(String(value).replace(/[$,\s]/g, ""));
  if (!Number.isFinite(parsed)) return String(value);
  return new Intl.NumberFormat(undefined, {
    style: "currency",
    currency: "USD",
    minimumFractionDigits: 2,
    maximumFractionDigits: 2
  }).format(parsed);
}

export function isBooleanField(field: AppfwUiFieldContract) {
  return field.dataType === "Boolean" || field.ui.formControl === "toggle";
}

export function isNumericField(field: AppfwUiFieldContract) {
  return /^(Int|Float|Decimal|Number)/.test(field.dataType);
}

export function isTemporalField(field: AppfwUiFieldContract) {
  return field.dataType === "Date" || field.dataType === "DateTime" || field.dataType === "Time";
}

export function isArrayField(field: AppfwUiFieldContract) {
  return field.dataType.endsWith("Array");
}

export function isIdentifierField(field: AppfwUiFieldContract) {
  return field.dataType === "Uuid" || field.dataType === "ObjectId" || field.dataType === "ID";
}

export function combineFilters(...filters: unknown[]) {
  const active = filters.filter((filter) => filter !== undefined && filter !== null);
  if (!active.length) return undefined;
  if (active.length === 1) return active[0];
  return { _and: active };
}

export function searchFilter(entity: AppfwUiEntityContract, columns: readonly string[], searchTerm: string) {
  const term = searchTerm.trim();
  if (!term) return undefined;
  const columnSet = new Set(columns);
  const searchableFields = entity.fields.filter(
    (field) =>
      columnSet.has(field.name) &&
      field.kind === "scalar" &&
      field.ui.filterable &&
      isTextSearchField(field)
  );
  if (!searchableFields.length) return undefined;
  return {
    _or: searchableFields.map((field) => ({
      [field.name]: { _contains: term }
    }))
  };
}

export function sortByFieldOrder(fields: AppfwUiFieldContract[], columns: string[]) {
  const order = new Map(fields.map((field, index) => [field.name, index]));
  return [...columns].sort((left, right) => (order.get(left) ?? 9999) - (order.get(right) ?? 9999));
}

export function editDraft(fields: AppfwUiFieldContract[], row: AppfwRecord) {
  return Object.fromEntries(fields.map((field) => [field.name, row[field.name] ?? ""]));
}

export function newDraft(fields: AppfwUiFieldContract[]) {
  return Object.fromEntries(
    fields.map((field) => [
      field.name,
      field.ui.formControl === "toggle" ? false : ""
    ])
  );
}

export function draftsEqual(left: AppfwRecord, right: AppfwRecord) {
  return JSON.stringify(left) === JSON.stringify(right);
}

export function validateDraft(fields: AppfwUiFieldContract[], draft: AppfwRecord): Record<string, string[]> {
  const errors: Record<string, string[]> = {};
  for (const field of fields) {
    const value = draft[field.name];
    const messages: string[] = [];
    if (field.required && isBlank(value) && field.ui.formControl !== "toggle") {
      messages.push(`${field.label} is required.`);
    }
    if (!isBlank(value) && field.ui.formControl === "number" && Number.isNaN(Number(value))) {
      messages.push(`${field.label} must be a number.`);
    }
    if (!isBlank(value) && field.ui.format === "email" && !/^[^\s@]+@[^\s@]+\.[^\s@]+$/.test(String(value))) {
      messages.push(`${field.label} must be a valid email address.`);
    }
    if (!isBlank(value) && field.ui.format === "url" && !isValidUrl(String(value))) {
      messages.push(`${field.label} must be a valid URL.`);
    }
    if (messages.length) errors[field.name] = messages;
  }
  return errors;
}

export function fieldInputId(fieldName: string) {
  return `crm-field-${fieldName.replace(/[^a-zA-Z0-9_-]/g, "-")}`;
}

export function mutationInput(
  entity: AppfwUiEntityContract,
  concurrencyField: string | undefined,
  row: AppfwRecord | null,
  draft: AppfwRecord
) {
  const fieldsByName = new Map(entity.fields.map((field) => [field.name, field]));
  const input: AppfwRecord = {};
  for (const [fieldName, value] of Object.entries(draft)) {
    const field = fieldsByName.get(fieldName);
    if (!field || !isMutationInputField(field)) continue;
    input[fieldName] = value;
  }
  if (row && entity.primaryKey) input[entity.primaryKey] = row[entity.primaryKey];
  if (row && concurrencyField) input[concurrencyField] = row[concurrencyField];
  return input;
}

export function deleteInput(entity: AppfwUiEntityContract, concurrencyField: string | undefined, row: AppfwRecord) {
  const input: AppfwRecord = {};
  if (entity.primaryKey) input[entity.primaryKey] = row[entity.primaryKey];
  if (concurrencyField) input[concurrencyField] = row[concurrencyField];
  return input;
}

export function operationBlocker(
  operation: AppfwUiOperationContract | undefined,
  authorization: string | null | undefined,
  tenantId: string | null | undefined,
  action: string,
  options: { allowDeleteConfirmationDisabledReason?: boolean } = {}
) {
  if (!operation) return `No generated ${action} operation exists for this entity.`;
  if (operation.disabledReason) {
    const confirmationDisabledReasonIsAllowed =
      options.allowDeleteConfirmationDisabledReason &&
      operation.disabledReason === DELETE_CONFIRMATION_DISABLED_REASON;
    if (!confirmationDisabledReasonIsAllowed) return operation.disabledReason;
  }
  if (operation.requiresAuth && !authorization) return `Sign in before ${action}.`;
  if (operation.requiresTenant && !tenantId) return `Choose a tenant before ${action}.`;
  return null;
}

export function upsertSavedRecord(current: AppfwEntityListData | null, rowKey: string, record: AppfwRecord | null) {
  if (!current || !record) return current;
  const savedKey = String(record[rowKey] ?? "");
  const exists = current.rows.some((row) => String(row[rowKey] ?? "") === savedKey);
  return {
    ...current,
    rows: exists
      ? current.rows.map((row) => (String(row[rowKey] ?? "") === savedKey ? { ...row, ...record } : row))
      : [record, ...current.rows],
    page: {
      ...current.page,
      queryCount: exists ? current.page.queryCount : current.page.queryCount + 1
    }
  };
}

export function removeDeletedRecord(current: AppfwEntityListData | null, rowKey: string, record: AppfwRecord) {
  if (!current) return current;
  const deletedKey = String(record[rowKey] ?? "");
  return {
    ...current,
    rows: current.rows.filter((row) => String(row[rowKey] ?? "") !== deletedKey),
    page: {
      ...current.page,
      queryCount: Math.max(0, current.page.queryCount - 1)
    }
  };
}

export function inputType(field: AppfwUiFieldContract) {
  if (isCurrencyField(field)) return "text";
  if (field.ui.format === "email") return "email";
  if (field.ui.format === "url") return "url";
  if (field.ui.formControl === "number") return "number";
  if (field.ui.formControl === "date") return "date";
  if (field.ui.formControl === "datetime") return "datetime-local";
  if (field.ui.formControl === "time") return "time";
  return "text";
}

export function coerceInput(field: AppfwUiFieldContract, value: string) {
  if (isCurrencyField(field)) return coerceCurrencyInput(field, value);
  if (!value) return field.required ? "" : null;
  if (field.ui.formControl === "number") {
    const parsed = Number(value);
    return Number.isFinite(parsed) ? parsed : value;
  }
  return value;
}

export function coerceCurrencyInput(field: AppfwUiFieldContract, value: string) {
  const normalized = value.replace(/[$,\s]/g, "");
  if (!normalized) return field.required ? "" : null;
  const parsed = Number(normalized);
  return Number.isFinite(parsed) ? parsed : value;
}

export function toOperationError(caught: unknown): AppfwOperationError {
  if (caught instanceof AppfwClientError) return caught.details;
  return {
    message: caught instanceof Error ? caught.message : String(caught),
    category: "unknown"
  };
}

function addSelectionField(selected: Set<string>, fieldName: string | undefined) {
  if (fieldName && isGraphqlName(fieldName)) selected.add(fieldName);
}

function isFlatRecordSelectionField(field: AppfwUiFieldContract) {
  return field.kind === "scalar" || field.kind === "enum" || isLookupFilterField(field);
}

function lookupForeignKeyForGridColumn(field: AppfwUiFieldContract) {
  if (!isFullEntitySelectorField(field)) return undefined;
  return field.relationship?.fieldName;
}

function isBusinessScalarField(field: AppfwUiFieldContract) {
  return (field.kind === "scalar" || field.kind === "enum") && !field.isKey && !field.isConcurrencyControl;
}

function createFilterId() {
  if (typeof crypto !== "undefined" && typeof crypto.randomUUID === "function") {
    return crypto.randomUUID();
  }
  return `filter-${Math.random().toString(36).slice(2)}`;
}

function defaultFilterValue(field: AppfwUiFieldContract) {
  return isBooleanField(field) ? "true" : "";
}

function coerceFilterValue(field: AppfwUiFieldContract, rawValue: string, operator: FilterOperatorOption): unknown {
  const raw = rawValue.trim();
  if (!raw) return undefined;
  if (operator.list) {
    const values = raw
      .split(/[\n,]/)
      .map((value) => value.trim())
      .filter(Boolean)
      .map((value) => coerceScalarFilterValue(field, value));
    if (!values.length) return undefined;
    return values;
  }
  return coerceScalarFilterValue(field, raw);
}

function coerceScalarFilterValue(field: AppfwUiFieldContract, raw: string) {
  if (isBooleanField(field)) {
    if (raw === "true") return true;
    if (raw === "false") return false;
    throw new Error(`${field.label} must be true or false.`);
  }
  if (isNumericField(field)) {
    const parsed = Number(raw);
    if (Number.isFinite(parsed)) return parsed;
    throw new Error(`${field.label} must be a number.`);
  }
  return raw;
}

function isGraphqlName(value: string) {
  return /^[_A-Za-z][_0-9A-Za-z]*$/.test(value);
}

function isTextSearchField(field: AppfwUiFieldContract) {
  return (
    field.dataType === "String" ||
    field.dataType === "Email" ||
    field.dataType === "Url" ||
    field.ui.format === "email" ||
    field.ui.format === "url"
  );
}

function isBlank(value: unknown) {
  return value === null || value === undefined || (typeof value === "string" && value.trim() === "");
}

function isValidUrl(value: string) {
  try {
    new URL(value);
    return true;
  } catch {
    return false;
  }
}

function isMutationInputField(field: AppfwUiFieldContract) {
  return (field.kind === "scalar" || field.kind === "enum" || isLookupFilterField(field)) && !field.readOnly && !field.isConcurrencyControl;
}
