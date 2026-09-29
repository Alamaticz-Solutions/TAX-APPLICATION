import type {
  HTMLAttributes,
  InputHTMLAttributes,
  ReactNode,
  SelectHTMLAttributes,
  TextareaHTMLAttributes
} from "react";
import { useId, useLayoutEffect, useRef, useState } from "react";
import {
  Button as AriaButton,
  ComboBox as AriaComboBox,
  Group as AriaGroup,
  Input as AriaInput,
  ListBox as AriaListBox,
  ListBoxItem as AriaListBoxItem,
  Popover as AriaPopover,
  TextField as AriaTextField
} from "react-aria-components";
import { composeClassNames, describedBy, type PdsOption } from "./types";

type FieldChromeProps = {
  id: string;
  label: string;
  hint?: ReactNode;
  error?: ReactNode;
  required?: boolean;
  children: ReactNode;
  className?: string;
};

function generatedFieldId(id: string | undefined, name: string | undefined, label: string) {
  return id ?? name ?? label.toLowerCase().replace(/[^a-z0-9]+/g, "-");
}

function fieldDescriptionIds(fieldId: string, hint: ReactNode | undefined, error: ReactNode | undefined) {
  return describedBy(
    Boolean(hint) && `${fieldId}-hint`,
    Boolean(error) && `${fieldId}-error`
  );
}

function hasFieldValue(value: unknown): boolean {
  if (Array.isArray(value)) return value.length > 0;
  return value !== undefined && value !== null && String(value).length > 0;
}

function parseCssTime(value: string): number {
  const parsed = Number.parseFloat(value);
  if (!Number.isFinite(parsed)) return 0;
  return value.trim().endsWith("s") && !value.trim().endsWith("ms")
    ? parsed * 1000
    : parsed;
}

function useFloatingLabelMotion(floating: boolean) {
  const restingRef = useRef<HTMLSpanElement>(null);
  const floatingRef = useRef<HTMLSpanElement>(null);
  const previousFloating = useRef(floating);
  const labelAnimation = useRef<Animation | null>(null);
  const restingAnimation = useRef<Animation | null>(null);

  useLayoutEffect(() => {
    const wasFloating = previousFloating.current;
    previousFloating.current = floating;
    if (wasFloating === floating) return;

    labelAnimation.current?.cancel();
    restingAnimation.current?.cancel();

    const floatingLabel = floatingRef.current;
    const restingLabel = restingRef.current;
    if (!floatingLabel || !restingLabel) return;
    if (globalThis.matchMedia?.("(prefers-reduced-motion: reduce)").matches) return;

    const floatingText = floatingLabel.querySelector<HTMLElement>("[data-label-text]");
    const restingText = restingLabel.querySelector<HTMLElement>("[data-label-text]");
    const floatingRect = floatingLabel.getBoundingClientRect();
    const restingRect = restingLabel.getBoundingClientRect();
    const floatingWidth = floatingText?.scrollWidth ?? floatingLabel.scrollWidth;
    const restingWidth = restingText?.scrollWidth ?? restingLabel.scrollWidth;
    if (!floatingWidth || !restingWidth || !floatingRect.height || !restingRect.height) return;

    const scale = restingWidth / floatingWidth;
    const xDelta = restingRect.x - floatingRect.x;
    const yDelta =
      restingRect.y -
      floatingRect.y +
      Math.round((restingRect.height - floatingRect.height * scale) / 2);
    const restingTransform = `translateX(${xDelta}px) translateY(${yDelta}px) scale(${scale})`;
    const floatingTransform = "translateX(0) translateY(0) scale(1)";
    const styles = globalThis.getComputedStyle(floatingLabel);
    const duration = parseCssTime(
      styles.getPropertyValue("--pds-motion-duration-field-label")
    );
    const easing = styles
      .getPropertyValue("--pds-motion-easing-field-label")
      .trim();
    if (!duration || !easing || typeof floatingLabel.animate !== "function") return;

    labelAnimation.current = floatingLabel.animate(
      floating
        ? [
            { opacity: 1, transform: restingTransform },
            { opacity: 1, transform: floatingTransform }
          ]
        : [
            { opacity: 1, transform: floatingTransform },
            { opacity: 1, transform: restingTransform }
          ],
      { duration, easing }
    );

    if (!floating) {
      restingAnimation.current = restingLabel.animate(
        [{ opacity: 0 }, { opacity: 0 }],
        { duration, easing }
      );
    }

    return () => {
      labelAnimation.current?.cancel();
      restingAnimation.current?.cancel();
    };
  }, [floating]);

  return { restingRef, floatingRef };
}

type FloatingFieldLabelProps = {
  floating: boolean;
  label: string;
  required?: boolean;
};

function FloatingFieldLabel({ floating, label, required }: FloatingFieldLabelProps) {
  const { restingRef, floatingRef } = useFloatingLabelMotion(floating);
  const labelText = (
    <span data-label-text>
      {label}
      {required ? <span aria-hidden="true"> *</span> : null}
    </span>
  );

  return (
    <span className="pds-text-field__label-wrapper" aria-hidden="true">
      <span
        className="pds-text-field__label pds-text-field__label--resting"
        ref={restingRef}
      >
        {labelText}
      </span>
      <span
        className="pds-text-field__label pds-text-field__label--floating"
        ref={floatingRef}
      >
        {labelText}
      </span>
    </span>
  );
}

export type FieldProps = FieldChromeProps;

export function Field({
  id,
  label,
  hint,
  error,
  required = false,
  children,
  className
}: FieldProps) {
  return (
    <div className={composeClassNames("pds-field", className)} data-invalid={Boolean(error) || undefined}>
      <label className="pds-field__label" htmlFor={id}>
        {label}
        {required ? <span aria-hidden="true"> *</span> : null}
      </label>
      {children}
      {hint ? (
        <div className="pds-field__hint" id={`${id}-hint`}>
          {hint}
        </div>
      ) : null}
      {error ? (
        <div className="pds-field__error" id={`${id}-error`} role="alert">
          {error}
        </div>
      ) : null}
    </div>
  );
}

export type FieldMetadataProps = {
  tags?: readonly ReactNode[];
  children?: ReactNode;
  className?: string;
};

export function FieldMetadata({
  tags,
  children,
  className
}: FieldMetadataProps) {
  if (!children && !tags?.length) return null;

  return (
    <span className={composeClassNames("pds-field-metadata", className)}>
      {tags?.length ? (
        <span className="pds-field-metadata__tags">
          {tags.map((tag, index) => (
            <em key={typeof tag === "string" ? tag : index}>{tag}</em>
          ))}
        </span>
      ) : null}
      {children ? <span className="pds-field-metadata__text">{children}</span> : null}
    </span>
  );
}

export type FormLoadingPreviewProps = HTMLAttributes<HTMLDivElement> & {
  fieldCount?: number;
  actionCount?: number;
  label?: string;
};

export function FormLoadingPreview({
  fieldCount = 6,
  actionCount = 2,
  label = "Loading form",
  className,
  ...props
}: FormLoadingPreviewProps) {
  const safeFieldCount = Math.max(2, Math.min(fieldCount, 12));
  const safeActionCount = Math.max(1, Math.min(actionCount, 4));
  const fields = Array.from({ length: safeFieldCount });
  const actions = Array.from({ length: safeActionCount });

  return (
    <div
      {...props}
      className={composeClassNames("pds-form-loading-preview", className)}
      role="status"
      aria-label={label}
      aria-busy="true"
    >
      <div className="pds-form-loading-preview__status" aria-hidden="true">
        <span className="pds-form-loading-preview__pill" />
        <span className="pds-form-loading-preview__pill" data-size="short" />
      </div>
      <div className="pds-form-loading-preview__fields" aria-hidden="true">
        {fields.map((_, index) => (
          <div className="pds-form-loading-preview__field" key={index}>
            <span className="pds-form-loading-preview__line" data-shape="label" />
            <span className="pds-form-loading-preview__control" />
            <span className="pds-form-loading-preview__line" data-shape="hint" />
          </div>
        ))}
      </div>
      <div className="pds-form-loading-preview__actions" aria-hidden="true">
        {actions.map((_, index) => (
          <span
            className="pds-form-loading-preview__action"
            data-size={index === 0 ? "short" : undefined}
            key={index}
          />
        ))}
      </div>
    </div>
  );
}

export type TextFieldProps = InputHTMLAttributes<HTMLInputElement> & {
  label: string;
  hint?: ReactNode;
  error?: ReactNode;
  variant?: "filled" | "outlined";
  leadingIcon?: ReactNode;
  trailingIcon?: ReactNode;
};

export function TextField({
  id,
  label,
  hint,
  error,
  variant = "outlined",
  leadingIcon,
  trailingIcon,
  className,
  required,
  onFocus,
  onBlur,
  "aria-label": ariaLabel,
  ...props
}: TextFieldProps) {
  const fieldId = generatedFieldId(id, props.name, label);
  const [internalHasValue, setInternalHasValue] = useState(() => hasFieldValue(props.defaultValue));
  const [focused, setFocused] = useState(false);
  const hasValue = props.value !== undefined ? hasFieldValue(props.value) : internalHasValue;
  const floating = focused || hasValue;
  return (
    <AriaTextField
      className="pds-field pds-text-field-field"
      aria-label={ariaLabel ?? label}
      isDisabled={props.disabled}
      isInvalid={Boolean(error)}
      isRequired={required}
      data-invalid={Boolean(error) || undefined}
    >
      <label
        className="pds-text-field"
        data-variant={variant}
        data-disabled={props.disabled || undefined}
        data-has-value={hasValue || undefined}
        data-floating={floating || undefined}
        htmlFor={fieldId}
      >
        {leadingIcon ? (
          <span className="pds-text-field__icon pds-text-field__icon--leading" aria-hidden="true">
            {leadingIcon}
          </span>
        ) : null}
        <AriaInput
          {...props}
          id={fieldId}
          required={required}
          aria-label={ariaLabel ?? label}
          aria-invalid={Boolean(error) || undefined}
          aria-describedby={fieldDescriptionIds(fieldId, hint, error)}
          className={composeClassNames("pds-input pds-text-field__input", className)}
          onChange={(event) => {
            setInternalHasValue(hasFieldValue(event.currentTarget.value));
            props.onChange?.(event);
          }}
          onFocus={(event) => {
            setFocused(true);
            onFocus?.(event);
          }}
          onBlur={(event) => {
            setFocused(false);
            onBlur?.(event);
          }}
        />
        <FloatingFieldLabel floating={floating} label={label} required={required} />
        {trailingIcon ? (
          <span className="pds-text-field__icon pds-text-field__icon--trailing" aria-hidden="true">
            {trailingIcon}
          </span>
        ) : null}
      </label>
      {hint ? <div className="pds-field__hint" id={`${fieldId}-hint`}>{hint}</div> : null}
      {error ? <div className="pds-field__error" id={`${fieldId}-error`} role="alert">{error}</div> : null}
    </AriaTextField>
  );
}

type NativeInputFieldProps = Omit<InputHTMLAttributes<HTMLInputElement>, "type"> & {
  label: string;
  hint?: ReactNode;
  error?: ReactNode;
  inputType: NonNullable<InputHTMLAttributes<HTMLInputElement>["type"]>;
  controlClassName: string;
  variant?: "filled" | "outlined";
};

function NativeInputField({
  id,
  label,
  hint,
  error,
  inputType,
  controlClassName,
  variant = "outlined",
  className,
  required,
  onFocus,
  onBlur,
  "aria-label": ariaLabel,
  ...props
}: NativeInputFieldProps) {
  const fieldId = generatedFieldId(id, props.name, label);
  const [internalHasValue, setInternalHasValue] = useState(() => hasFieldValue(props.defaultValue));
  const [focused, setFocused] = useState(false);
  const hasValue = props.value !== undefined ? hasFieldValue(props.value) : internalHasValue;
  const alwaysFloat = inputType === "date" || inputType === "time" || inputType === "datetime-local";
  const floating = focused || hasValue || alwaysFloat;
  return (
    <AriaTextField
      className="pds-field pds-text-field-field"
      aria-label={ariaLabel ?? label}
      isDisabled={props.disabled}
      isInvalid={Boolean(error)}
      isRequired={required}
      data-invalid={Boolean(error) || undefined}
    >
      <label
        className="pds-text-field pds-native-field"
        data-variant={variant}
        data-disabled={props.disabled || undefined}
        data-has-value={hasValue || alwaysFloat || undefined}
        data-floating={floating || undefined}
        htmlFor={fieldId}
      >
        <AriaInput
          {...props}
          id={fieldId}
          type={inputType}
          required={required}
          aria-label={ariaLabel ?? label}
          aria-invalid={Boolean(error) || undefined}
          aria-describedby={fieldDescriptionIds(fieldId, hint, error)}
          className={composeClassNames("pds-input pds-text-field__input", controlClassName, className)}
          onChange={(event) => {
            setInternalHasValue(hasFieldValue(event.currentTarget.value));
            props.onChange?.(event);
          }}
          onFocus={(event) => {
            setFocused(true);
            onFocus?.(event);
          }}
          onBlur={(event) => {
            setFocused(false);
            onBlur?.(event);
          }}
        />
        <FloatingFieldLabel floating={floating} label={label} required={required} />
      </label>
      {hint ? <div className="pds-field__hint" id={`${fieldId}-hint`}>{hint}</div> : null}
      {error ? <div className="pds-field__error" id={`${fieldId}-error`} role="alert">{error}</div> : null}
    </AriaTextField>
  );
}

export type DateFieldProps = Omit<InputHTMLAttributes<HTMLInputElement>, "type"> & {
  label: string;
  hint?: ReactNode;
  error?: ReactNode;
  variant?: "filled" | "outlined";
};

export function DateField(props: DateFieldProps) {
  return <NativeInputField {...props} inputType="date" controlClassName="pds-date-input" />;
}

export type TimeFieldProps = Omit<InputHTMLAttributes<HTMLInputElement>, "type"> & {
  label: string;
  hint?: ReactNode;
  error?: ReactNode;
  variant?: "filled" | "outlined";
};

export function TimeField(props: TimeFieldProps) {
  return <NativeInputField {...props} inputType="time" controlClassName="pds-time-input" />;
}

export type DateTimeFieldProps = Omit<InputHTMLAttributes<HTMLInputElement>, "type"> & {
  label: string;
  hint?: ReactNode;
  error?: ReactNode;
  variant?: "filled" | "outlined";
};

export function DateTimeField(props: DateTimeFieldProps) {
  return <NativeInputField {...props} inputType="datetime-local" controlClassName="pds-datetime-input" />;
}

export type TextAreaProps = TextareaHTMLAttributes<HTMLTextAreaElement> & {
  label: string;
  hint?: ReactNode;
  error?: ReactNode;
};

export function TextArea({
  id,
  label,
  hint,
  error,
  className,
  required,
  ...props
}: TextAreaProps) {
  const fieldId = generatedFieldId(id, props.name, label);
  return (
    <Field id={fieldId} label={label} hint={hint} error={error} required={required}>
      <textarea
        {...props}
        id={fieldId}
        required={required}
        aria-invalid={Boolean(error) || undefined}
        aria-describedby={fieldDescriptionIds(fieldId, hint, error)}
        className={composeClassNames("pds-input pds-textarea", className)}
      />
    </Field>
  );
}

export type SelectFieldProps = SelectHTMLAttributes<HTMLSelectElement> & {
  label: string;
  hint?: ReactNode;
  error?: ReactNode;
  options: readonly PdsOption[];
  placeholder?: string;
  variant?: "filled" | "outlined";
};

export function SelectField({
  id,
  label,
  hint,
  error,
  options,
  placeholder,
  variant = "outlined",
  className,
  required,
  onFocus,
  onBlur,
  "aria-label": ariaLabel,
  ...props
}: SelectFieldProps) {
  const fieldId = generatedFieldId(id, props.name, label);
  const [internalHasValue, setInternalHasValue] = useState(() => hasFieldValue(props.defaultValue));
  const [focused, setFocused] = useState(false);
  const hasValue = props.value !== undefined ? hasFieldValue(props.value) : internalHasValue;
  const floating = focused || hasValue || Boolean(placeholder);
  return (
    <div className="pds-field pds-text-field-field" data-invalid={Boolean(error) || undefined}>
      <label
        className="pds-text-field pds-select-field"
        data-variant={variant}
        data-disabled={props.disabled || undefined}
        data-has-value={hasValue || Boolean(placeholder) || undefined}
        data-floating={floating || undefined}
        htmlFor={fieldId}
      >
        <select
          {...props}
          id={fieldId}
          required={required}
          aria-label={ariaLabel ?? label}
          aria-invalid={Boolean(error) || undefined}
          aria-describedby={fieldDescriptionIds(fieldId, hint, error)}
          className={composeClassNames("pds-input pds-select pds-text-field__input", className)}
          onChange={(event) => {
            setInternalHasValue(hasFieldValue(event.currentTarget.value));
            props.onChange?.(event);
          }}
          onFocus={(event) => {
            setFocused(true);
            onFocus?.(event);
          }}
          onBlur={(event) => {
            setFocused(false);
            onBlur?.(event);
          }}
        >
          {placeholder ? <option value="">{placeholder}</option> : null}
          {options.map((option) => (
            <option key={option.value} value={option.value} disabled={option.disabled}>
              {option.label}
            </option>
          ))}
        </select>
        <FloatingFieldLabel floating={floating} label={label} required={required} />
        <span className="pds-select-field__indicator" aria-hidden="true" />
      </label>
      {hint ? <div className="pds-field__hint" id={`${fieldId}-hint`}>{hint}</div> : null}
      {error ? <div className="pds-field__error" id={`${fieldId}-error`} role="alert">{error}</div> : null}
    </div>
  );
}

export type ComboboxFieldProps = {
  id: string;
  name?: string;
  label: string;
  hint?: ReactNode;
  error?: ReactNode;
  options: readonly PdsOption[];
  value: string | null;
  onValueChange: (value: string | null) => void;
  inputValue: string;
  onInputValueChange: (value: string) => void;
  placeholder?: string;
  required?: boolean;
  disabled?: boolean;
  loading?: boolean;
  emptyMessage?: ReactNode;
  className?: string;
};

export function ComboboxField({
  id,
  name,
  label,
  hint,
  error,
  options,
  value,
  onValueChange,
  inputValue,
  onInputValueChange,
  placeholder,
  required = false,
  disabled = false,
  loading = false,
  emptyMessage = "No options available.",
  className
}: ComboboxFieldProps) {
  const status = loading ? "Loading options." : options.length === 0 ? emptyMessage : null;
  const statusId = status ? `${id}-status` : undefined;

  return (
    <AriaComboBox
      className={composeClassNames("pds-field pds-lookup-select", className)}
      name={name}
      selectedKey={value}
      inputValue={inputValue}
      onSelectionChange={(key) => onValueChange(key === null ? null : String(key))}
      onInputChange={onInputValueChange}
      isDisabled={disabled || loading}
      isInvalid={Boolean(error)}
      isRequired={required}
      allowsEmptyCollection
      data-loading={loading || undefined}
      data-empty={!loading && options.length === 0 ? true : undefined}
    >
      <label className="pds-field__label" htmlFor={id}>
        {label}
        {required ? <span aria-hidden="true"> *</span> : null}
      </label>
      <AriaGroup className="pds-input-group" data-invalid={Boolean(error) || undefined}>
        <AriaInput
          id={id}
          className="pds-input"
          placeholder={placeholder}
          aria-describedby={fieldDescriptionIds(id, hint, error) ?? statusId}
          aria-invalid={Boolean(error) || undefined}
        />
        <AriaButton
          className="pds-input-group__action pds-button"
          aria-label={`Show ${label.toLowerCase()} options`}
        >
          <span className="pds-menu-button__indicator" aria-hidden="true" />
        </AriaButton>
      </AriaGroup>
      {hint ? <div className="pds-field__hint" id={`${id}-hint`}>{hint}</div> : null}
      {error ? <div className="pds-field__error" id={`${id}-error`} role="alert">{error}</div> : null}
      {status ? (
        <div className="pds-lookup-select__status" id={statusId} role="status" aria-live="polite">
          {status}
        </div>
      ) : null}
      <AriaPopover className="pds-menu-button__menu" placement="bottom start">
        <AriaListBox
          aria-label={`${label} options`}
          items={loading ? [] : options}
          className="pds-lookup-select"
          renderEmptyState={() => <span className="pds-lookup-select__status">{emptyMessage}</span>}
        >
          {(option) => (
            <AriaListBoxItem
              id={option.value}
              textValue={option.label}
              isDisabled={option.disabled}
              className="pds-menu-button__item"
            >
              <span className="pds-menu-button__item-copy">
                <strong>{option.label}</strong>
                {option.description ? <small>{option.description}</small> : null}
              </span>
            </AriaListBoxItem>
          )}
        </AriaListBox>
      </AriaPopover>
    </AriaComboBox>
  );
}

export type MultiSelectProps = Omit<SelectHTMLAttributes<HTMLSelectElement>, "defaultValue" | "multiple" | "onChange" | "value"> & {
  label: string;
  hint?: ReactNode;
  error?: ReactNode;
  options: readonly PdsOption[];
  value: readonly string[];
  onValueChange: (value: string[]) => void;
  rootClassName?: string;
};

export function MultiSelect({
  id,
  label,
  hint,
  error,
  options,
  value,
  onValueChange,
  className,
  rootClassName,
  required,
  size,
  ...props
}: MultiSelectProps) {
  const fieldId = generatedFieldId(id, props.name, label);
  const selectedValues = value.map(String);
  const visibleRows = size ?? Math.min(Math.max(options.length, 3), 8);

  return (
    <Field id={fieldId} label={label} hint={hint} error={error} required={required} className={rootClassName}>
      <select
        {...props}
        id={fieldId}
        multiple
        required={required}
        size={visibleRows}
        value={selectedValues}
        aria-invalid={Boolean(error) || undefined}
        aria-describedby={fieldDescriptionIds(fieldId, hint, error)}
        className={composeClassNames("pds-input pds-select pds-multi-select", className)}
        onChange={(event) => {
          onValueChange(Array.from(event.currentTarget.selectedOptions).map((option) => option.value));
        }}
      >
        {options.map((option) => (
          <option key={option.value} value={option.value} disabled={option.disabled}>
            {option.label}
          </option>
        ))}
      </select>
    </Field>
  );
}

export type InputGroupProps = HTMLAttributes<HTMLSpanElement> & {
  prefix?: ReactNode;
  suffix?: ReactNode;
  action?: ReactNode;
  invalid?: boolean;
  disabled?: boolean;
  children: ReactNode;
};

export function InputGroup({
  prefix,
  suffix,
  action,
  invalid = false,
  disabled = false,
  children,
  className,
  ...props
}: InputGroupProps) {
  return (
    <span
      {...props}
      className={composeClassNames("pds-input-group", className)}
      data-disabled={disabled || undefined}
      data-invalid={invalid || undefined}
    >
      {prefix ? <span className="pds-input-group__affix pds-input-group__prefix">{prefix}</span> : null}
      <span className="pds-input-group__control">{children}</span>
      {suffix ? <span className="pds-input-group__affix pds-input-group__suffix">{suffix}</span> : null}
      {action ? <span className="pds-input-group__action">{action}</span> : null}
    </span>
  );
}

export type FileUploadProps = Omit<InputHTMLAttributes<HTMLInputElement>, "type"> & {
  label: string;
  hint?: ReactNode;
  error?: ReactNode;
  selectedLabel?: ReactNode;
  action?: ReactNode;
};

export function FileUpload({
  id,
  label,
  hint,
  error,
  selectedLabel,
  action,
  className,
  required,
  ...props
}: FileUploadProps) {
  const fieldId = generatedFieldId(id, props.name, label);

  return (
    <Field id={fieldId} label={label} hint={hint} error={error} required={required} className="pds-field--file-upload">
      <span className="pds-file-upload" data-invalid={Boolean(error) || undefined}>
        <input
          {...props}
          id={fieldId}
          type="file"
          required={required}
          aria-invalid={Boolean(error) || undefined}
          aria-describedby={fieldDescriptionIds(fieldId, hint, error)}
          className={composeClassNames("pds-file-upload__input", className)}
        />
        {selectedLabel ? <span className="pds-file-upload__summary">{selectedLabel}</span> : null}
        {action ? <span className="pds-file-upload__action">{action}</span> : null}
      </span>
    </Field>
  );
}

export type LookupSelectProps = Omit<SelectHTMLAttributes<HTMLSelectElement>, "defaultValue" | "multiple" | "onChange" | "value"> & {
  options: readonly PdsOption[];
  value: string | readonly string[] | null | undefined;
  onValueChange: (value: string | string[]) => void;
  multiple?: boolean;
  placeholder?: string;
  loading?: boolean;
  loadingMessage?: string;
  emptyMessage?: string;
  error?: ReactNode;
  rootClassName?: string;
};

export function LookupSelect({
  id,
  options,
  value,
  onValueChange,
  multiple = false,
  placeholder = "Select a value",
  loading = false,
  loadingMessage = "Loading lookup values...",
  emptyMessage = "No lookup values available.",
  error,
  disabled,
  className,
  rootClassName,
  size,
  "aria-describedby": ariaDescribedBy,
  "aria-invalid": ariaInvalid,
  ...props
}: LookupSelectProps) {
  const generatedId = useId();
  const controlId = id ?? `pds-lookup-${generatedId}`;
  const selectedValues = Array.isArray(value)
    ? value.map(String)
    : value === "" || value === null || value === undefined
      ? []
      : [String(value)];
  const hasOptions = options.length > 0;
  const selectDisabled = disabled || loading || !hasOptions;
  const placeholderLabel = loading ? loadingMessage : hasOptions ? placeholder : emptyMessage;
  const state = error ? "error" : loading ? "loading" : hasOptions ? "ready" : "empty";
  const statusMessage = error ?? (loading ? loadingMessage : null) ?? (!hasOptions ? emptyMessage : null);
  const statusId = statusMessage ? `${controlId}-status` : undefined;

  return (
    <span
      className={composeClassNames("pds-lookup-select", rootClassName)}
      data-empty={!hasOptions || undefined}
      data-invalid={Boolean(error) || undefined}
      data-loading={loading || undefined}
      data-state={state}
    >
      <select
        {...props}
        id={controlId}
        multiple={multiple}
        size={multiple ? size : undefined}
        value={multiple ? selectedValues : selectedValues[0] ?? ""}
        disabled={selectDisabled}
        aria-busy={loading || undefined}
        aria-invalid={Boolean(error) || ariaInvalid || undefined}
        aria-describedby={describedBy(ariaDescribedBy, statusId)}
        className={composeClassNames("pds-input pds-select", className)}
        onChange={(event) => {
          if (multiple) {
            onValueChange(Array.from(event.currentTarget.selectedOptions).map((option) => option.value));
            return;
          }
          onValueChange(event.currentTarget.value);
        }}
      >
        {!multiple && placeholderLabel ? <option value="">{placeholderLabel}</option> : null}
        {options.map((option) => (
          <option key={option.value} value={option.value} disabled={option.disabled}>
            {option.label}
          </option>
        ))}
      </select>
      {statusMessage ? (
        <small
          className="pds-lookup-select__status"
          id={statusId}
          role={error ? "alert" : "status"}
          aria-live={error ? "assertive" : "polite"}
        >
          {statusMessage}
        </small>
      ) : null}
    </span>
  );
}

export type CheckboxFieldProps = Omit<InputHTMLAttributes<HTMLInputElement>, "type"> & {
  label: string;
  detail?: ReactNode;
};

export function CheckboxField({
  id,
  label,
  detail,
  className,
  ...props
}: CheckboxFieldProps) {
  const fieldId = generatedFieldId(id, props.name, label);
  return (
    <label className={composeClassNames("pds-choice", className)} htmlFor={fieldId}>
      <input {...props} id={fieldId} type="checkbox" />
      <span className="pds-choice__body">
        <span className="pds-choice__label">{label}</span>
        {detail ? <span className="pds-choice__detail">{detail}</span> : null}
      </span>
    </label>
  );
}

export type SwitchFieldProps = {
  id: string;
  label: string;
  checked: boolean;
  onCheckedChange: (checked: boolean) => void;
  detail?: ReactNode;
  error?: ReactNode;
  required?: boolean;
  disabled?: boolean;
  className?: string;
};

export function SwitchField({
  id,
  label,
  checked,
  onCheckedChange,
  detail,
  error,
  required = false,
  disabled = false,
  className
}: SwitchFieldProps) {
  const detailId = detail ? `${id}-detail` : undefined;
  const errorId = error ? `${id}-error` : undefined;

  return (
    <div className={composeClassNames("pds-switch-field", className)} data-invalid={Boolean(error) || undefined}>
      <button
        id={id}
        className="pds-switch"
        type="button"
        role="switch"
        aria-checked={checked}
        aria-labelledby={`${id}-label`}
        aria-describedby={describedBy(detailId, errorId)}
        aria-invalid={Boolean(error) || undefined}
        aria-required={required || undefined}
        disabled={disabled}
        onClick={() => onCheckedChange(!checked)}
      >
        <span className="pds-switch__thumb" aria-hidden="true" />
      </button>
      <span className="pds-switch-field__body">
        <span className="pds-switch-field__label" id={`${id}-label`}>
          {label}
          {required ? <span aria-hidden="true"> *</span> : null}
        </span>
        {detail ? <span className="pds-switch-field__detail" id={detailId}>{detail}</span> : null}
        {error ? (
          <span className="pds-field__error" id={errorId} role="alert">
            {error}
          </span>
        ) : null}
      </span>
    </div>
  );
}
