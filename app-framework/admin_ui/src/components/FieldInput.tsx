import type { DrawerMode, LookupOption, PropertyType } from "../types";
import { acceptsJsonInput, inputType, normalizeInputValue } from "../lib/form";

type FieldInputProps = {
  prop: PropertyType;
  value: unknown;
  mode: DrawerMode;
  options?: LookupOption[];
  isLoadingOptions?: boolean;
  isDisabled?: boolean;
  isReadOnly?: boolean;
  onChange: (value: unknown) => void;
};

export function FieldInput({
  prop,
  value,
  mode,
  options,
  isLoadingOptions = false,
  isDisabled = false,
  isReadOnly = false,
  onChange
}: FieldInputProps) {
  const locked = isReadOnly || prop.is_key || prop.is_concurrency_control || prop.is_read_only;
  const normalizedValue = normalizeInputValue(value, prop.data_type);
  const hasCurrentOption = !normalizedValue || options?.some((option) => option.value === normalizedValue);
  const label = (
    <span>
      <b>{prop.caption || prop.name}</b>
      <small>{prop.data_type}</small>
    </span>
  );

  if (prop.foreign_key && options) {
    return (
      <label>
        {label}
        <select disabled={locked || isDisabled} onChange={(event) => onChange(event.target.value)} value={normalizedValue}>
          <option value="">{isLoadingOptions ? "Loading options" : "No selection"}</option>
          {!hasCurrentOption && <option value={normalizedValue}>{normalizedValue}</option>}
          {options.map((option) => (
            <option key={option.value} value={option.value}>
              {option.label}
            </option>
          ))}
        </select>
      </label>
    );
  }

  if (prop.data_type === "Boolean") {
    return (
      <label className="check-field">
        {label}
        <input
          checked={Boolean(value)}
          disabled={locked || isDisabled}
          onChange={(event) => onChange(event.target.checked)}
          type="checkbox"
        />
      </label>
    );
  }

  if (acceptsJsonInput(prop.data_type)) {
    return (
      <label className="span-2">
        {label}
        <textarea
          disabled={isDisabled}
          readOnly={locked}
          onChange={(event) => onChange(event.target.value)}
          value={typeof value === "string" ? value : value === undefined ? "" : JSON.stringify(value, null, 2)}
        />
      </label>
    );
  }

  return (
    <label className={prop.data_type === "String" && String(value ?? "").length > 80 ? "span-2" : ""}>
      {label}
      <input
        disabled={isDisabled || (locked && mode === "create")}
        readOnly={locked && mode === "edit"}
        onChange={(event) => onChange(event.target.value)}
        type={inputType(prop)}
        value={normalizedValue}
      />
    </label>
  );
}
