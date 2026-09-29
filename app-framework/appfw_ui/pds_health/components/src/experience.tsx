import {
  useEffect,
  useId,
  useMemo,
  useRef,
  useState,
  type CSSProperties,
  type Dispatch,
  type HTMLAttributes,
  type KeyboardEvent,
  type ReactNode,
  type RefObject,
  type SetStateAction
} from "react";
import { Button } from "./primitives";
import { composeClassNames } from "./types";

function useModalFocus(
  open: boolean,
  enabled: boolean,
  panelRef: RefObject<HTMLDivElement | null>,
  setOpen: Dispatch<SetStateAction<boolean>>
) {
  useEffect(() => {
    if (!open || !enabled || !panelRef.current) return undefined;
    const panel = panelRef.current;
    const previous = document.activeElement instanceof HTMLElement ? document.activeElement : null;
    const focusableSelector = [
      "button:not([disabled])",
      "input:not([disabled])",
      "select:not([disabled])",
      "textarea:not([disabled])",
      "a[href]",
      "[tabindex]:not([tabindex='-1'])"
    ].join(",");
    const focusables = () => Array.from(panel.querySelectorAll<HTMLElement>(focusableSelector));
    const frame = requestAnimationFrame(() => {
      const selected = panel.querySelector<HTMLElement>("[data-selected='true']:not([disabled])");
      (selected ?? focusables()[0] ?? panel).focus();
    });

    function handleKeyDown(event: globalThis.KeyboardEvent) {
      if (event.key === "Escape") {
        event.preventDefault();
        setOpen(false);
        return;
      }
      if (event.key !== "Tab") return;
      const controls = focusables();
      if (!controls.length) {
        event.preventDefault();
        panel.focus();
        return;
      }
      const first = controls[0];
      const last = controls[controls.length - 1];
      if (event.shiftKey && document.activeElement === first) {
        event.preventDefault();
        last.focus();
      } else if (!event.shiftKey && document.activeElement === last) {
        event.preventDefault();
        first.focus();
      }
    }

    document.addEventListener("keydown", handleKeyDown);
    return () => {
      cancelAnimationFrame(frame);
      document.removeEventListener("keydown", handleKeyDown);
      previous?.focus();
    };
  }, [enabled, open, panelRef, setOpen]);
}

function parseIsoDate(value: string | undefined): Date {
  const match = value?.match(/^(\d{4})-(\d{2})-(\d{2})$/);
  if (!match) return new Date();
  return new Date(Number(match[1]), Number(match[2]) - 1, Number(match[3]));
}

function isoDate(value: Date): string {
  const year = value.getFullYear();
  const month = String(value.getMonth() + 1).padStart(2, "0");
  const day = String(value.getDate()).padStart(2, "0");
  return `${year}-${month}-${day}`;
}

function displayDate(value: string | undefined): string {
  if (!value) return "Select date";
  return new Intl.DateTimeFormat(undefined, {
    weekday: "short",
    month: "short",
    day: "numeric",
    year: "numeric"
  }).format(parseIsoDate(value));
}

function monthLabel(value: Date): string {
  return new Intl.DateTimeFormat(undefined, { month: "long", year: "numeric" }).format(value);
}

function calendarDates(activeMonth: Date): Date[] {
  const first = new Date(activeMonth.getFullYear(), activeMonth.getMonth(), 1);
  const start = new Date(first);
  start.setDate(first.getDate() - first.getDay());
  return Array.from({ length: 42 }, (_, index) => {
    const date = new Date(start);
    date.setDate(start.getDate() + index);
    return date;
  });
}

export type DatePickerProps = Omit<HTMLAttributes<HTMLDivElement>, "defaultValue" | "onChange"> & {
  label: string;
  value?: string;
  defaultValue?: string;
  onValueChange?: (value: string) => void;
  variant?: "docked" | "modal" | "modal-input";
  defaultOpen?: boolean;
  min?: string;
  max?: string;
  name?: string;
  disabled?: boolean;
};

export function DatePicker({
  label,
  value,
  defaultValue,
  onValueChange,
  variant = "docked",
  defaultOpen = false,
  min,
  max,
  name,
  disabled = false,
  className,
  ...props
}: DatePickerProps) {
  const id = useId();
  const [internalValue, setInternalValue] = useState(defaultValue ?? "");
  const selectedValue = value ?? internalValue;
  const [open, setOpen] = useState(defaultOpen);
  const panelRef = useRef<HTMLDivElement>(null);
  const [activeMonth, setActiveMonth] = useState(() => {
    const initial = parseIsoDate(value ?? defaultValue);
    return new Date(initial.getFullYear(), initial.getMonth(), 1);
  });
  const dates = useMemo(() => calendarDates(activeMonth), [activeMonth]);
  const modal = variant !== "docked";
  useModalFocus(open, modal, panelRef, setOpen);

  function updateValue(nextValue: string) {
    if (value === undefined) setInternalValue(nextValue);
    onValueChange?.(nextValue);
  }

  const picker = (
    <div
      ref={panelRef}
      className="pds-date-picker__panel"
      data-variant={variant}
      role={modal ? "dialog" : "group"}
      aria-modal={modal || undefined}
      aria-label={`${label} calendar`}
      tabIndex={modal ? -1 : undefined}
    >
      <header className="pds-date-picker__selection">
        <span>Select date</span>
        <strong>{displayDate(selectedValue)}</strong>
      </header>
      {variant === "modal-input" ? (
        <label className="pds-date-picker__manual-field">
          <span>Date</span>
          <input
            type="date"
            value={selectedValue}
            min={min}
            max={max}
            onChange={(event) => updateValue(event.currentTarget.value)}
          />
        </label>
      ) : (
        <>
          <div className="pds-date-picker__month-bar">
            <strong>{monthLabel(activeMonth)}</strong>
            <span className="pds-date-picker__month-actions">
              <button
                type="button"
                aria-label="Previous month"
                onClick={() => setActiveMonth(new Date(activeMonth.getFullYear(), activeMonth.getMonth() - 1, 1))}
              >
                <span aria-hidden="true">&lt;</span>
              </button>
              <button
                type="button"
                aria-label="Next month"
                onClick={() => setActiveMonth(new Date(activeMonth.getFullYear(), activeMonth.getMonth() + 1, 1))}
              >
                <span aria-hidden="true">&gt;</span>
              </button>
            </span>
          </div>
          <div className="pds-date-picker__weekdays" aria-hidden="true">
            {[
              "S",
              "M",
              "T",
              "W",
              "T",
              "F",
              "S"
            ].map((day, index) => <span key={`${day}-${index}`}>{day}</span>)}
          </div>
          <div className="pds-date-picker__grid">
            {dates.map((date) => {
              const dateValue = isoDate(date);
              const outsideMonth = date.getMonth() !== activeMonth.getMonth();
              const unavailable = Boolean((min && dateValue < min) || (max && dateValue > max));
              return (
                <button
                  key={dateValue}
                  type="button"
                  data-outside-month={outsideMonth || undefined}
                  data-selected={dateValue === selectedValue || undefined}
                  disabled={unavailable}
                  aria-pressed={dateValue === selectedValue}
                  aria-label={displayDate(dateValue)}
                  onClick={() => updateValue(dateValue)}
                >
                  {date.getDate()}
                </button>
              );
            })}
          </div>
        </>
      )}
      <footer className="pds-date-picker__actions">
        <Button variant="text" size="sm" onClick={() => setOpen(false)}>Cancel</Button>
        <Button variant="text" size="sm" onClick={() => setOpen(false)}>OK</Button>
      </footer>
    </div>
  );

  return (
    <div {...props} className={composeClassNames("pds-date-picker", className)} data-variant={variant}>
      {name ? <input type="hidden" name={name} value={selectedValue} /> : null}
      <button
        id={`${id}-trigger`}
        className="pds-picker-trigger"
        type="button"
        aria-haspopup="dialog"
        aria-expanded={open}
        aria-controls={`${id}-panel`}
        disabled={disabled}
        onClick={() => setOpen((current) => !current)}
      >
        <span>{label}</span>
        <strong>{selectedValue ? displayDate(selectedValue) : "MM/DD/YYYY"}</strong>
        <span className="pds-picker-trigger__icon" aria-hidden="true" />
      </button>
      {open ? (
        modal ? (
          <div
            className="pds-picker-backdrop"
            id={`${id}-panel`}
            onMouseDown={(event) => {
              if (event.target === event.currentTarget) setOpen(false);
            }}
          >
            {picker}
          </div>
        ) : <div id={`${id}-panel`}>{picker}</div>
      ) : null}
    </div>
  );
}

function parseTime(value: string | undefined): { hour: number; minute: number } {
  const match = value?.match(/^(\d{1,2}):(\d{2})$/);
  if (!match) return { hour: 9, minute: 30 };
  return {
    hour: Math.max(0, Math.min(23, Number(match[1]))),
    minute: Math.max(0, Math.min(59, Number(match[2])))
  };
}

function timeValue(hour: number, minute: number): string {
  return `${String(hour).padStart(2, "0")}:${String(minute).padStart(2, "0")}`;
}

function displayTime(value: string): string {
  const parsed = parseTime(value);
  const date = new Date(2026, 0, 1, parsed.hour, parsed.minute);
  return new Intl.DateTimeFormat(undefined, { hour: "numeric", minute: "2-digit" }).format(date);
}

export type TimePickerProps = Omit<HTMLAttributes<HTMLDivElement>, "defaultValue" | "onChange"> & {
  label: string;
  value?: string;
  defaultValue?: string;
  onValueChange?: (value: string) => void;
  variant?: "dial" | "input";
  defaultOpen?: boolean;
  name?: string;
  disabled?: boolean;
};

export function TimePicker({
  label,
  value,
  defaultValue = "09:30",
  onValueChange,
  variant = "dial",
  defaultOpen = false,
  name,
  disabled = false,
  className,
  ...props
}: TimePickerProps) {
  const id = useId();
  const [internalValue, setInternalValue] = useState(defaultValue);
  const selectedValue = value ?? internalValue;
  const selected = parseTime(selectedValue);
  const [open, setOpen] = useState(defaultOpen);
  const panelRef = useRef<HTMLDivElement>(null);
  const [activePart, setActivePart] = useState<"hour" | "minute">("hour");
  const period = selected.hour >= 12 ? "PM" : "AM";
  const displayHour = selected.hour % 12 || 12;
  useModalFocus(open, true, panelRef, setOpen);

  function update(nextHour: number, nextMinute: number) {
    const nextValue = timeValue(nextHour, nextMinute);
    if (value === undefined) setInternalValue(nextValue);
    onValueChange?.(nextValue);
  }

  function updatePeriod(nextPeriod: "AM" | "PM") {
    const baseHour = selected.hour % 12;
    update(baseHour + (nextPeriod === "PM" ? 12 : 0), selected.minute);
  }

  const dialValues = activePart === "hour"
    ? Array.from({ length: 12 }, (_, index) => index + 1)
    : Array.from({ length: 12 }, (_, index) => index * 5);
  const dialAngle = activePart === "hour" ? (displayHour % 12) * 30 : selected.minute * 6;

  return (
    <div {...props} className={composeClassNames("pds-time-picker", className)} data-variant={variant}>
      {name ? <input type="hidden" name={name} value={selectedValue} /> : null}
      <button
        className="pds-picker-trigger"
        type="button"
        aria-haspopup="dialog"
        aria-expanded={open}
        aria-controls={`${id}-panel`}
        disabled={disabled}
        onClick={() => setOpen((current) => !current)}
      >
        <span>{label}</span>
        <strong>{displayTime(selectedValue)}</strong>
        <span className="pds-picker-trigger__icon pds-picker-trigger__icon--time" aria-hidden="true" />
      </button>
      {open ? (
        <div
          className="pds-picker-backdrop"
          id={`${id}-panel`}
          onMouseDown={(event) => {
            if (event.target === event.currentTarget) setOpen(false);
          }}
        >
          <div
            ref={panelRef}
            className="pds-time-picker__panel"
            role="dialog"
            aria-modal="true"
            aria-label={`${label} time picker`}
            tabIndex={-1}
          >
            <span className="pds-time-picker__eyebrow">Select time</span>
            <div className="pds-time-picker__display">
              <button
                type="button"
                data-active={activePart === "hour" || undefined}
                onClick={() => setActivePart("hour")}
                aria-label="Select hour"
              >
                {String(displayHour).padStart(2, "0")}
              </button>
              <span aria-hidden="true">:</span>
              <button
                type="button"
                data-active={activePart === "minute" || undefined}
                onClick={() => setActivePart("minute")}
                aria-label="Select minute"
              >
                {String(selected.minute).padStart(2, "0")}
              </button>
              <span className="pds-time-picker__period" role="group" aria-label="Period">
                {(["AM", "PM"] as const).map((item) => (
                  <button
                    type="button"
                    key={item}
                    data-selected={period === item || undefined}
                    aria-pressed={period === item}
                    onClick={() => updatePeriod(item)}
                  >
                    {item}
                  </button>
                ))}
              </span>
            </div>
            {variant === "dial" ? (
              <div
                className="pds-time-picker__dial"
                aria-label={`Select ${activePart}`}
                style={{ "--pds-time-dial-angle": `${dialAngle}deg` } as CSSProperties}
              >
                <span className="pds-time-picker__dial-center" aria-hidden="true" />
                {dialValues.map((item, index) => {
                  const angle = (index / 12) * Math.PI * 2 - Math.PI / 2;
                  const left = 50 + Math.cos(angle) * 39;
                  const top = 50 + Math.sin(angle) * 39;
                  const itemSelected = activePart === "hour"
                    ? item === displayHour
                    : item === selected.minute;
                  return (
                    <button
                      type="button"
                      key={item}
                      style={{ left: `${left}%`, top: `${top}%` }}
                      data-selected={itemSelected || undefined}
                      aria-pressed={itemSelected}
                      onClick={() => {
                        if (activePart === "hour") {
                          update((item % 12) + (period === "PM" ? 12 : 0), selected.minute);
                          setActivePart("minute");
                        } else {
                          update(selected.hour, item);
                        }
                      }}
                    >
                      {String(item).padStart(2, "0")}
                    </button>
                  );
                })}
              </div>
            ) : (
              <div className="pds-time-picker__input-grid">
                <label>
                  <span>Hour</span>
                  <input
                    type="number"
                    min="1"
                    max="12"
                    value={displayHour}
                    onChange={(event) => {
                      const next = Math.max(1, Math.min(12, Number(event.currentTarget.value)));
                      update((next % 12) + (period === "PM" ? 12 : 0), selected.minute);
                    }}
                  />
                </label>
                <span aria-hidden="true">:</span>
                <label>
                  <span>Minute</span>
                  <input
                    type="number"
                    min="0"
                    max="59"
                    value={selected.minute}
                    onChange={(event) => update(selected.hour, Math.max(0, Math.min(59, Number(event.currentTarget.value))))}
                  />
                </label>
              </div>
            )}
            <footer className="pds-time-picker__actions">
              <Button variant="text" size="sm" onClick={() => setOpen(false)}>Cancel</Button>
              <Button variant="text" size="sm" onClick={() => setOpen(false)}>OK</Button>
            </footer>
          </div>
        </div>
      ) : null}
    </div>
  );
}

export type ListProps = HTMLAttributes<HTMLUListElement> & {
  variant?: "standard" | "segmented";
};

export function List({ variant = "standard", className, ...props }: ListProps) {
  return <ul {...props} className={composeClassNames("pds-list", className)} data-variant={variant} />;
}

export type ListItemProps = HTMLAttributes<HTMLLIElement> & {
  headline: ReactNode;
  supportingText?: ReactNode;
  overline?: ReactNode;
  leading?: ReactNode;
  trailing?: ReactNode;
  action?: ReactNode;
  selected?: boolean;
};

export function ListItem({
  headline,
  supportingText,
  overline,
  leading,
  trailing,
  action,
  selected = false,
  className,
  ...props
}: ListItemProps) {
  return (
    <li {...props} className={composeClassNames("pds-list-item", className)} data-selected={selected || undefined}>
      {leading ? <span className="pds-list-item__leading" aria-hidden="true">{leading}</span> : null}
      <span className="pds-list-item__copy">
        {overline ? <span className="pds-list-item__overline">{overline}</span> : null}
        <strong>{headline}</strong>
        {supportingText ? <span>{supportingText}</span> : null}
      </span>
      {trailing ? <span className="pds-list-item__trailing">{trailing}</span> : null}
      {action ? <span className="pds-list-item__action">{action}</span> : null}
    </li>
  );
}

export type SearchBarItem = {
  id: string;
  label: string;
  detail?: ReactNode;
  leading?: ReactNode;
  trailing?: ReactNode;
  keywords?: readonly string[];
  onSelect?: () => void;
};

export type SearchBarProps = Omit<HTMLAttributes<HTMLDivElement>, "onChange"> & {
  items: readonly SearchBarItem[];
  label?: string;
  placeholder?: string;
  value?: string;
  defaultValue?: string;
  onValueChange?: (value: string) => void;
  defaultOpen?: boolean;
  variant?: "bar" | "app-bar";
  trailingActions?: ReactNode;
};

export function SearchBar({
  items,
  label = "Search",
  placeholder = "Search",
  value,
  defaultValue = "",
  onValueChange,
  defaultOpen = false,
  variant = "bar",
  trailingActions,
  className,
  ...props
}: SearchBarProps) {
  const id = useId();
  const [internalValue, setInternalValue] = useState(defaultValue);
  const query = value ?? internalValue;
  const [open, setOpen] = useState(defaultOpen);
  const [activeIndex, setActiveIndex] = useState(0);
  const results = useMemo(() => {
    const needle = query.trim().toLowerCase();
    if (!needle) return items;
    return items.filter((item) => [item.label, item.detail, ...(item.keywords ?? [])]
      .filter((part): part is string => typeof part === "string")
      .join(" ")
      .toLowerCase()
      .includes(needle));
  }, [items, query]);

  function updateQuery(nextValue: string) {
    if (value === undefined) setInternalValue(nextValue);
    onValueChange?.(nextValue);
    setActiveIndex(0);
    setOpen(true);
  }

  function selectItem(item: SearchBarItem) {
    updateQuery(item.label);
    setOpen(false);
    item.onSelect?.();
  }

  function handleKeyDown(event: KeyboardEvent<HTMLInputElement>) {
    if (event.key === "ArrowDown") {
      event.preventDefault();
      setOpen(true);
      setActiveIndex((current) => Math.min(results.length - 1, current + 1));
    } else if (event.key === "ArrowUp") {
      event.preventDefault();
      setActiveIndex((current) => Math.max(0, current - 1));
    } else if (event.key === "Enter" && open && results[activeIndex]) {
      event.preventDefault();
      selectItem(results[activeIndex]);
    } else if (event.key === "Escape") {
      setOpen(false);
    }
  }

  return (
    <div
      {...props}
      className={composeClassNames("pds-search-bar", className)}
      data-open={open || undefined}
      data-variant={variant}
      onBlur={(event) => {
        if (!event.currentTarget.contains(event.relatedTarget)) setOpen(false);
      }}
    >
      <div className="pds-search-bar__control" role="search">
        <span className="pds-search-bar__icon" aria-hidden="true" />
        <label className="pds-search-bar__field">
          <span className="pds-visually-hidden">{label}</span>
          <input
            type="search"
            value={query}
            placeholder={placeholder}
            role="combobox"
            aria-expanded={open}
            aria-controls={`${id}-results`}
            aria-activedescendant={open && results[activeIndex] ? `${id}-${results[activeIndex].id}` : undefined}
            autoComplete="off"
            onFocus={() => setOpen(true)}
            onChange={(event) => updateQuery(event.currentTarget.value)}
            onKeyDown={handleKeyDown}
          />
        </label>
        {query ? (
          <button type="button" className="pds-search-bar__clear" aria-label="Clear search" onClick={() => updateQuery("")}>
            <span aria-hidden="true">x</span>
          </button>
        ) : null}
        {trailingActions ? <span className="pds-search-bar__actions">{trailingActions}</span> : null}
      </div>
      {open ? (
        <ul className="pds-search-bar__results" id={`${id}-results`} role="listbox" aria-label="Search suggestions">
          {results.length ? results.map((item, index) => (
            <li
              key={item.id}
              role="presentation"
              data-active={index === activeIndex || undefined}
            >
              <button
                id={`${id}-${item.id}`}
                type="button"
                role="option"
                aria-selected={index === activeIndex}
                onClick={() => selectItem(item)}
                onMouseEnter={() => setActiveIndex(index)}
              >
                {item.leading ? <span className="pds-search-bar__result-leading" aria-hidden="true">{item.leading}</span> : null}
                <span className="pds-search-bar__result-copy">
                  <strong>{item.label}</strong>
                  {item.detail ? <span>{item.detail}</span> : null}
                </span>
                {item.trailing ? <span className="pds-search-bar__result-trailing">{item.trailing}</span> : null}
              </button>
            </li>
          )) : <li className="pds-search-bar__empty">No results</li>}
        </ul>
      ) : null}
    </div>
  );
}

export type ToolbarProps = HTMLAttributes<HTMLDivElement> & {
  ariaLabel: string;
  variant?: "docked" | "floating" | "vibrant";
  orientation?: "horizontal" | "vertical";
  leading?: ReactNode;
  trailing?: ReactNode;
  children: ReactNode;
};

export function Toolbar({
  ariaLabel,
  variant = "docked",
  orientation = "horizontal",
  leading,
  trailing,
  children,
  className,
  onKeyDown,
  ...props
}: ToolbarProps) {
  function moveFocus(event: KeyboardEvent<HTMLDivElement>) {
    onKeyDown?.(event);
    if (event.defaultPrevented) return;
    const previousKey = orientation === "horizontal" ? "ArrowLeft" : "ArrowUp";
    const nextKey = orientation === "horizontal" ? "ArrowRight" : "ArrowDown";
    if (![previousKey, nextKey, "Home", "End"].includes(event.key)) return;
    const controls = Array.from(event.currentTarget.querySelectorAll<HTMLElement>(
      "button:not([disabled]), a[href], input:not([disabled])"
    ));
    if (!controls.length) return;
    event.preventDefault();
    const current = Math.max(0, controls.indexOf(document.activeElement as HTMLElement));
    const next = event.key === "Home"
      ? 0
      : event.key === "End"
        ? controls.length - 1
        : event.key === nextKey
          ? (current + 1) % controls.length
          : (current - 1 + controls.length) % controls.length;
    controls[next]?.focus();
  }

  return (
    <div
      {...props}
      className={composeClassNames("pds-toolbar", className)}
      data-variant={variant}
      data-orientation={orientation}
      role="toolbar"
      aria-label={ariaLabel}
      aria-orientation={orientation}
      onKeyDown={moveFocus}
    >
      {leading ? <span className="pds-toolbar__leading">{leading}</span> : null}
      <span className="pds-toolbar__content">{children}</span>
      {trailing ? <span className="pds-toolbar__trailing">{trailing}</span> : null}
    </div>
  );
}
