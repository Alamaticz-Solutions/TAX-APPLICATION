import { useMemo, useState } from "react";
import {
  Button,
  ComboboxField,
  DateField,
  MenuButton,
  TextField,
  TimeField,
  type PdsOption
} from "@appfw/pds-health-components";

const readyOptions: readonly PdsOption[] = [
  { value: "attention", label: "Needs attention", description: "Requires a timely review" },
  { value: "scheduled", label: "Scheduled", description: "Has a planned review window" },
  { value: "protected", label: "Protected", description: "Unavailable for this fixture", disabled: true }
];

type OptionState = "ready" | "loading" | "empty";

export function RepresentativeInteractionsFixture() {
  const [summary, setSummary] = useState("");
  const [summaryTouched, setSummaryTouched] = useState(true);
  const [optionState, setOptionState] = useState<OptionState>("ready");
  const [optionValue, setOptionValue] = useState<string | null>(null);
  const [optionInput, setOptionInput] = useState("");
  const [date, setDate] = useState("");
  const [time, setTime] = useState("");
  const [lastEvent, setLastEvent] = useState("Fixture ready");
  const options = optionState === "ready" ? readyOptions : [];
  const menuItems = useMemo(() => [
    { id: "review", label: "Review item", selected: true, onSelect: () => setLastEvent("Review selected") },
    { id: "defer", label: "Defer item", description: "Unavailable in this state", disabled: true },
    { id: "remove", label: "Remove from view", tone: "danger" as const, onSelect: () => setLastEvent("Remove selected") }
  ], []);

  const surfaceStyle = {
    width: "min(100% - 32px, 1040px)",
    margin: "0 auto",
    padding: "32px 0 48px",
    display: "grid",
    gap: "24px"
  } as const;
  const panelStyle = {
    display: "grid",
    gap: "20px",
    padding: "24px",
    border: "1px solid var(--pds-color-border-default)",
    borderRadius: "var(--pds-radius-md)",
    background: "var(--pds-color-surface-elevated)",
    boxShadow: "var(--pds-shadow-card)"
  } as const;
  const gridStyle = {
    display: "grid",
    gridTemplateColumns: "repeat(auto-fit, minmax(min(100%, 240px), 1fr))",
    gap: "20px",
    alignItems: "start"
  } as const;

  return (
    <main style={surfaceStyle} aria-labelledby="f1-title">
      <header>
        <p style={{ margin: "0 0 6px", color: "var(--pds-color-text-muted)" }}>Interaction foundation</p>
        <h1 id="f1-title" style={{ margin: 0, fontSize: "clamp(1.6rem, 4vw, 2.4rem)", letterSpacing: 0 }}>
          Representative work controls
        </h1>
      </header>

      <section style={panelStyle} aria-label="Representative fields">
        <div style={gridStyle}>
          <TextField
            id="work-summary"
            label="Work summary"
            hint="Use a short neutral description."
            required
            value={summary}
            error={summaryTouched && !summary ? "Enter a work summary." : undefined}
            onBlur={() => setSummaryTouched(true)}
            onChange={(event) => {
              setSummary(event.currentTarget.value);
              setLastEvent(event.currentTarget.value ? "Summary corrected" : "Summary cleared");
            }}
          />
          <ComboboxField
            id="work-status"
            label="Status"
            hint="Choose one deterministic fixture value."
            options={options}
            value={optionValue}
            onValueChange={(value) => {
              setOptionValue(value);
              setLastEvent(value ? `Status selected: ${value}` : "Status cleared");
            }}
            inputValue={optionInput}
            onInputValueChange={setOptionInput}
            placeholder="Filter statuses"
            loading={optionState === "loading"}
            emptyMessage="No fixture statuses match."
          />
        </div>

        <div aria-label="Combobox fixture state" style={{ display: "flex", flexWrap: "wrap", gap: "8px" }}>
          {(["ready", "loading", "empty"] as const).map((state) => (
            <Button
              key={state}
              size="sm"
              variant={optionState === state ? "primary" : "secondary"}
              aria-pressed={optionState === state}
              onClick={() => {
                setOptionState(state);
                setOptionInput("");
                setOptionValue(null);
                setLastEvent(`Status fixture: ${state}`);
              }}
            >
              {state[0].toUpperCase() + state.slice(1)}
            </Button>
          ))}
        </div>

        <div style={gridStyle}>
          <DateField
            id="review-date"
            label="Review date"
            required
            value={date}
            error={!date ? "Choose a review date." : undefined}
            onChange={(event) => {
              setDate(event.currentTarget.value);
              setLastEvent(event.currentTarget.value ? "Date corrected" : "Date cleared");
            }}
          />
          <TimeField
            id="review-time"
            label="Review time"
            required
            value={time}
            error={!time ? "Choose a review time." : undefined}
            onChange={(event) => {
              setTime(event.currentTarget.value);
              setLastEvent(event.currentTarget.value ? "Time corrected" : "Time cleared");
            }}
          />
        </div>

        <MenuButton
          label="More actions"
          menuLabel="Work item actions"
          menuTone="vibrant"
          items={menuItems}
          data-testid="menu-root"
          data-root-contract="element-neutral"
          onPointerDown={(event) => {
            event.currentTarget.dataset.lastPointerType = event.pointerType;
          }}
        />
        <MenuButton
          label="Legacy custom actions"
          menuLabel="Legacy custom actions"
          items={[
            { id: "legacy-run", label: "Run legacy action", onSelect: () => setLastEvent("Legacy item selected") },
            { id: "legacy-disabled", label: "Unavailable legacy action", disabled: true }
          ]}
          data-testid="legacy-menu-root"
        >
          <button
            className="pds-menu-button__item"
            type="button"
            role="menuitem"
            onClick={() => setLastEvent("Legacy child selected")}
          >
            Legacy child action
          </button>
        </MenuButton>
      </section>

      <output role="status" aria-live="polite" style={panelStyle}>
        <strong>Local event</strong>
        <span data-testid="event-summary">{lastEvent}</span>
      </output>
    </main>
  );
}
