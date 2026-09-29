import { useEffect, useState, type ReactNode } from 'react';
import { ComboboxField } from '@appfw/pds-health-components/forms';
import type { PdsOption } from '@appfw/pds-health-components/types';

// ---------------------------------------------------------------------------
// SingleSelectField: the product's one single-choice field: PDS's own `ComboboxField`, used
// as-is with no product CSS. (Same wrapper as the Project Governance frontend.)
//
// Known PDS 0.12.0 defect, reported for the framework and deliberately NOT patched here (no
// product overrides of PDS classes): `SelectField`, in browsers with `appearance: base-select`,
// has its picker forced to `display: grid` and renders permanently open. That is why this
// wrapper exists and `SelectField` is not used.
// ---------------------------------------------------------------------------

function slugifyFieldId(label: string): string {
  return label.toLowerCase().replace(/[^a-z0-9]+/g, '-').replace(/(^-|-$)/g, '');
}

export function SingleSelectField({
  id,
  label,
  value,
  onValueChange,
  options,
  placeholder,
  hint,
  error,
  required,
  disabled
}: {
  id?: string;
  label: string;
  value: string;
  onValueChange: (value: string) => void;
  options: readonly PdsOption[];
  placeholder?: string;
  hint?: ReactNode;
  error?: ReactNode;
  required?: boolean;
  disabled?: boolean;
}) {
  const fieldId = id ?? slugifyFieldId(label);
  const selectedLabel = options.find((option) => option.value === value)?.label ?? '';
  const [inputValue, setInputValue] = useState(selectedLabel);

  // Keep the visible text in sync when `value` changes from outside (e.g. a form reset or an
  // applied suggestion) without fighting the user's own typing.
  useEffect(() => {
    setInputValue(selectedLabel);
  }, [selectedLabel]);

  return (
    <ComboboxField
      id={fieldId}
      label={label}
      options={options}
      value={value || null}
      onValueChange={(next) => onValueChange(next ?? '')}
      inputValue={inputValue}
      onInputValueChange={setInputValue}
      placeholder={placeholder}
      hint={hint}
      error={error}
      required={required}
      disabled={disabled}
    />
  );
}
