import { useState } from "react";
import type { AppfwUiEntityContract, AppfwUiFieldContract } from "../generated/appfw-ui-contract";
import { DataTable, DateField, DateTimeField, Field, FieldMetadata, LookupSelect, SwitchField, TextField, TimeField } from "../components/ui";
import type { AppfwRecord } from "../lib/appfwClient";
import {
  coerceCurrencyInput,
  coerceInput,
  fieldInputId,
  formatCurrencyValue,
  humanizeFieldName,
  inputType,
  isCurrencyField,
  isFullEntitySelectorField,
  isLookupFilterField,
  isRecordValue,
  lookupTargetEntity,
  rawCurrencyValue,
  relationshipClearDisabledReason,
  relationshipSelectorMode,
  relationshipSelectedKeys
} from "./entityScaffoldModel";
import { recordRouteRef } from "./routeIdentity";
import type { RelationshipLookupState } from "./types";
import { useLookupOptions } from "./useLookupOptions";

type RecordFormFieldProps = {
  ownerEntity: AppfwUiEntityContract;
  field: AppfwUiFieldContract;
  value: unknown;
  errors?: string[];
  onChange: (value: unknown) => void;
  onNavigateToRecord: (entity: AppfwUiEntityContract, recordKey: string, routeRef?: string) => void;
};

export function RecordFormField({
  ownerEntity,
  field,
  value,
  errors,
  onChange,
  onNavigateToRecord
}: RecordFormFieldProps) {
  const inputId = fieldInputId(field.name);
  const hintId = `${inputId}-hint`;
  const errorId = `${inputId}-error`;
  const hint = fieldHint(field);
  const error = fieldErrorText(errors);
  const describedBy = fieldDescribedBy(field, hintId, errors, errorId);
  const selectorMode = relationshipSelectorMode(field);
  const targetEntity = selectorMode ? lookupTargetEntity(field) : undefined;
  const selectedKeys = Array.from(relationshipSelectedKeys(value, targetEntity));
  const lookup = useLookupOptions(field, Boolean(selectorMode), selectorMode ?? "lookup", selectedKeys);

  if (field.ui.formControl === "toggle") {
    return (
      <SwitchField
        id={inputId}
        label={field.label}
        checked={Boolean(value)}
        required={field.required}
        detail={hint}
        error={error}
        onCheckedChange={(checked) => onChange(checked)}
      />
    );
  }

  if (isLookupFilterField(field) && selectorMode === "lookup") {
    return (
      <Field id={inputId} label={field.label} required={field.required} hint={hint} error={error}>
        <LookupSelect
          id={inputId}
          value={value === null || value === undefined ? "" : String(value)}
          options={lookup.options}
          required={field.required}
          aria-invalid={Boolean(errors?.length)}
          aria-describedby={describedBy}
          className={fieldControlClass(field, "select")}
          placeholder={`Select ${field.label.toLowerCase()}`}
          loading={lookup.status === "loading"}
          loadingMessage="Loading lookup values..."
          error={lookup.status === "error" ? lookup.error : undefined}
          onValueChange={(nextValue) => onChange(Array.isArray(nextValue) ? nextValue[0] ?? null : nextValue || null)}
        />
      </Field>
    );
  }

  if ((isLookupFilterField(field) && selectorMode === "entity-list") || isFullEntitySelectorField(field)) {
    return (
      <RelationshipEntitySelector
        ownerEntity={ownerEntity}
        field={field}
        value={value}
        lookup={lookup}
        errors={errors}
        onChange={onChange}
        onNavigateToRecord={onNavigateToRecord}
      />
    );
  }

  if (isCurrencyField(field)) {
    return (
      <CurrencyField
        field={field}
        value={value}
        errors={errors}
        inputId={inputId}
        onChange={onChange}
      />
    );
  }

  if (isDateOnlyField(field)) {
    return (
      <DateField
        id={inputId}
        label={field.label}
        value={temporalInputValue(field, value)}
        required={field.required}
        hint={hint}
        error={error}
        onChange={(event) => onChange(coerceInput(field, event.target.value))}
        className={fieldControlClass(field)}
      />
    );
  }

  if (isTimeOnlyField(field)) {
    return (
      <TimeField
        id={inputId}
        label={field.label}
        value={temporalInputValue(field, value)}
        required={field.required}
        hint={hint}
        error={error}
        onChange={(event) => onChange(coerceInput(field, event.target.value))}
        className={fieldControlClass(field)}
      />
    );
  }

  if (isDateTimeInputField(field)) {
    return (
      <DateTimeField
        id={inputId}
        label={field.label}
        value={temporalInputValue(field, value)}
        required={field.required}
        hint={hint}
        error={error}
        onChange={(event) => onChange(coerceInput(field, event.target.value))}
        className={fieldControlClass(field)}
      />
    );
  }

  return (
    <TextField
      id={inputId}
      label={field.label}
      type={inputType(field)}
      value={value === null || value === undefined ? "" : String(value)}
      required={field.required}
      hint={hint}
      error={error}
      onChange={(event) => onChange(coerceInput(field, event.target.value))}
      className={fieldControlClass(field)}
    />
  );
}

function CurrencyField({
  field,
  value,
  errors,
  inputId,
  onChange
}: {
  field: AppfwUiFieldContract;
  value: unknown;
  errors?: string[];
  inputId: string;
  onChange: (value: unknown) => void;
}) {
  const [focused, setFocused] = useState(false);
  const displayValue = focused ? rawCurrencyValue(value) : formatCurrencyValue(value);

  return (
    <TextField
      id={inputId}
      label={field.label}
      type="text"
      inputMode="decimal"
      value={displayValue}
      required={field.required}
      hint={fieldHint(field)}
      error={fieldErrorText(errors)}
      onFocus={() => setFocused(true)}
      onBlur={() => setFocused(false)}
      onChange={(event) => onChange(coerceCurrencyInput(field, event.target.value))}
      className={fieldControlClass(field, "currency")}
    />
  );
}

function RelationshipEntitySelector({
  ownerEntity,
  field,
  value,
  lookup,
  errors,
  onChange,
  onNavigateToRecord
}: {
  ownerEntity: AppfwUiEntityContract;
  field: AppfwUiFieldContract;
  value: unknown;
  lookup: RelationshipLookupState;
  errors?: string[];
  onChange: (value: unknown) => void;
  onNavigateToRecord: (entity: AppfwUiEntityContract, recordKey: string, routeRef?: string) => void;
}) {
  const targetEntity = lookup.targetEntity;
  const inputId = fieldInputId(field.name);
  const errorId = `${inputId}-error`;
  const isMany = field.dataType === "NavToMany";
  const selectedKeys = relationshipSelectedKeys(value, targetEntity);
  const selectedKey = Array.from(selectedKeys)[0] ?? null;
  const columns = targetEntity ? entitySelectorColumns(targetEntity) : [];
  const selectedRecord = targetEntity ? selectedRelationshipRecord(value, lookup.rows, targetEntity) : null;
  const selectedRows = targetEntity
    ? lookup.rows.filter((row) => selectedKeys.has(String(row[targetEntity.primaryKey] ?? "")))
    : [];
  const clearDisabledReason = relationshipClearDisabledReason(ownerEntity, field);

  return (
    <div className="crm-relationship-field">
      <div className="crm-field-label">
        <span>{field.label}</span>
        {field.required ? <small>Required</small> : null}
      </div>
      {lookup.status === "loading" ? (
        <div className="crm-relationship-empty">Loading {field.label.toLowerCase()}...</div>
      ) : null}
      {lookup.status === "error" ? <div className="crm-field-errors">{lookup.error}</div> : null}
      {!isMany && targetEntity ? (
        <div className="crm-linked-record">
          <div className="crm-linked-record-summary">
            <strong>{linkedRecordTitle(field, targetEntity, selectedRecord, selectedKey)}</strong>
            <span>{linkedRecordDetail(targetEntity, selectedRecord, selectedKey, columns)}</span>
          </div>
          <div className="crm-linked-record-actions">
            {!field.required && !clearDisabledReason && selectedKey ? (
              <button
                type="button"
                className="crm-button secondary compact"
                onClick={() => {
                  onChange(null);
                }}
              >
                Clear
              </button>
            ) : null}
            {selectedKey ? (
              <button
                type="button"
                className="crm-button secondary compact"
                onClick={() => onNavigateToRecord(targetEntity, selectedKey, selectedRecord ? recordRouteRef(targetEntity, selectedRecord) : undefined)}
              >
                Open
              </button>
            ) : null}
          </div>
        </div>
      ) : null}
      {isMany && targetEntity ? (
        selectedRows.length ? (
          <div className="crm-relationship-selector-grid">
            <DataTable
              columns={columns}
              rows={selectedRows}
              rowKey={targetEntity.primaryKey}
              columnLabels={Object.fromEntries(columns.map((column) => [column, humanizeFieldName(column)]))}
              onRowSelect={(row) => onNavigateToRecord(targetEntity, String(row[targetEntity.primaryKey] ?? ""), recordRouteRef(targetEntity, row))}
            />
          </div>
        ) : (
          <div className="crm-relationship-empty">
            Linked {targetEntity.caption.plural.toLowerCase()} appear in the related records section.
          </div>
        )
      ) : null}
      <FieldAnnotation id={`${inputId}-annotation`} field={field} />
      <FieldErrors id={errorId} errors={errors} />
    </div>
  );
}

function selectedRelationshipRecord(
  value: unknown,
  rows: AppfwRecord[],
  targetEntity: AppfwUiEntityContract
) {
  if (isRecordValue(value)) return value;
  const selectedKey = Array.from(relationshipSelectedKeys(value, targetEntity))[0];
  if (!selectedKey) return null;
  return rows.find((row) => String(row[targetEntity.primaryKey] ?? "") === selectedKey) ?? null;
}

function linkedRecordTitle(
  field: AppfwUiFieldContract,
  entity: AppfwUiEntityContract,
  record: AppfwRecord | null,
  selectedKey: string | null
) {
  if (record) return String(record[entity.captionField] ?? record[entity.primaryKey] ?? `${entity.caption.singular} selected`);
  if (selectedKey) return `${entity.caption.singular} linked`;
  return `No ${field.label.toLowerCase()} linked`;
}

function linkedRecordDetail(
  entity: AppfwUiEntityContract,
  record: AppfwRecord | null,
  selectedKey: string | null,
  columns: readonly string[]
) {
  if (!record) return selectedKey ? selectedKey : "Choose a linked record.";
  const details = columns
    .filter((column) => column !== entity.captionField)
    .map((column) => record[column])
    .filter((value) => value !== null && value !== undefined && value !== "")
    .map((value) => String(value))
    .slice(0, 2);
  return details.length ? details.join(" · ") : String(record[entity.primaryKey] ?? "");
}

function entitySelectorColumns(entity: AppfwUiEntityContract) {
  const ordered = [entity.captionField, ...entity.scaffold.list.fields].filter((name) => name !== entity.primaryKey);
  const valid = ordered.filter((name) => entity.fields.some((field) => field.name === name));
  const columns = Array.from(new Set(valid)).slice(0, 5);
  return columns.length ? columns : [entity.captionField];
}

function fieldHint(field: AppfwUiFieldContract) {
  const tags = fieldValidationTags(field);
  const hint = field.validation.clientHint;
  if (!hint && !tags.length) return undefined;
  return <FieldMetadata tags={tags}>{hint}</FieldMetadata>;
}

function fieldErrorText(errors?: string[]) {
  return errors?.length ? errors.join(", ") : undefined;
}

function FieldAnnotation({ id, field }: { id: string; field: AppfwUiFieldContract }) {
  const tags = fieldValidationTags(field);
  const hint = field.validation.clientHint;
  if (!hint && !tags.length) return null;
  return (
    <div id={id} className="crm-field-annotation">
      {tags.length ? (
        <span className="crm-field-annotation-tags">
          {tags.map((tag) => (
            <em key={tag}>{tag}</em>
          ))}
        </span>
      ) : null}
      {hint ? <span>{hint}</span> : null}
    </div>
  );
}

function FieldErrors({ id, errors }: { id: string; errors?: string[] }) {
  if (!errors?.length) return null;
  return (
    <div id={id} className="crm-field-errors">
      {errors.join(", ")}
    </div>
  );
}

function fieldDescribedBy(
  field: AppfwUiFieldContract,
  annotationId: string,
  errors: string[] | undefined,
  errorId: string
) {
  return [
    field.validation.clientHint || fieldValidationTags(field).length ? annotationId : undefined,
    errors?.length ? errorId : undefined
  ].filter(Boolean).join(" ") || undefined;
}

function fieldControlClass(field: AppfwUiFieldContract, variant?: "select" | "currency") {
  const classes: string[] = [];
  if (variant === "select") classes.push("crm-select-control");
  if (variant === "currency") classes.push("crm-currency-control");
  if (isDateLikeField(field)) classes.push("crm-date-control");
  if (isTimeLikeField(field)) classes.push("crm-time-control");
  return classes.join(" ");
}

function isDateLikeField(field: AppfwUiFieldContract) {
  return field.ui.formControl === "date" || field.ui.formControl === "datetime" || field.dataType === "Date" || field.dataType === "DateTime";
}

function isTimeLikeField(field: AppfwUiFieldContract) {
  return field.ui.formControl === "time" || field.ui.formControl === "datetime" || field.dataType === "Time" || field.dataType === "DateTime";
}

function isDateOnlyField(field: AppfwUiFieldContract) {
  return field.ui.formControl === "date" || (field.dataType === "Date" && field.ui.formControl !== "readonly");
}

function isTimeOnlyField(field: AppfwUiFieldContract) {
  return field.ui.formControl === "time" || (field.dataType === "Time" && field.ui.formControl !== "readonly");
}

function isDateTimeInputField(field: AppfwUiFieldContract) {
  return (
    field.ui.formControl === "datetime" ||
    (
      field.dataType === "DateTime" &&
      field.ui.formControl !== "date" &&
      field.ui.formControl !== "time" &&
      field.ui.formControl !== "readonly"
    )
  );
}

function temporalInputValue(field: AppfwUiFieldContract, value: unknown) {
  if (value === null || value === undefined) return "";
  const text = String(value);
  if (isDateOnlyField(field)) return text.slice(0, 10);
  if (isTimeOnlyField(field)) return text.slice(0, 5);
  if (isDateTimeInputField(field)) return text.replace(/Z$/, "").slice(0, 16);
  return text;
}

function fieldValidationTags(field: AppfwUiFieldContract) {
  return [
    field.readOnly ? "Read-only" : undefined,
    field.isConcurrencyControl ? "Concurrency token" : undefined
  ].filter((tag): tag is string => Boolean(tag));
}
