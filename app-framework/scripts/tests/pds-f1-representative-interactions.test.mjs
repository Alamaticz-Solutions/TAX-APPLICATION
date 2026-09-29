import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import test from "node:test";

const read = (path) => readFileSync(new URL(`../../${path}`, import.meta.url), "utf8");

const forms = read("appfw_ui/pds_health/components/src/forms.tsx");
const primitives = read("appfw_ui/pds_health/components/src/primitives.tsx");
const fixture = read("appfw_ui/pds_health/catalog-app/src/f1/RepresentativeInteractionsFixture.tsx");
const main = read("appfw_ui/pds_health/catalog-app/src/f1/main.tsx");
const html = read("appfw_ui/pds_health/catalog-app/f1-representative-interactions.html");

test("ComboboxField keeps a PDS-owned string contract", () => {
  assert.match(forms, /export type ComboboxFieldProps = \{/);
  assert.match(forms, /value: string \| null;/);
  assert.match(forms, /onValueChange: \(value: string \| null\) => void;/);
  assert.match(forms, /inputValue: string;/);
  assert.match(forms, /onInputValueChange: \(value: string\) => void;/);
  assert.match(forms, /export function ComboboxField\(/);
  assert.doesNotMatch(forms, /export type ComboboxFieldProps[^}]+\b(?:Key|Selection|DateValue|ValidationResult)\b/s);
});

test("representative mechanics use only the pinned substrate internally", () => {
  for (const symbol of ["ComboBox", "Input", "Popover", "ListBox", "ListBoxItem"]) {
    assert.match(forms, new RegExp(`\\b${symbol}\\b`));
  }
  for (const symbol of ["MenuTrigger", "Menu", "MenuItem", "Popover"]) {
    assert.match(primitives, new RegExp(`\\b${symbol}\\b`));
  }
  assert.doesNotMatch(`${forms}\n${primitives}`, /@base-ui|from ["']@?radix/);
});

test("MenuButton keeps an element-neutral root contract and a tone-aware portal", () => {
  assert.match(primitives, /MenuButtonProps = Omit<HTMLAttributes<HTMLElement>, "children">/);
  assert.doesNotMatch(primitives, /HTMLAttributes<HTMLDetailsElement>/);
  assert.doesNotMatch(primitives, /as HTMLAttributes<HTMLDivElement>/);
  assert.match(primitives, /toggleAttribute\("open", open\)/);
  assert.match(primitives, /UNSTABLE_portalContainer=\{rootRef\.current/);
  assert.match(primitives, /data-menu-tone=\{menuTone\}/);
});

test("MenuButton preserves the pre-existing custom-children compatibility path", () => {
  assert.match(primitives, /if \(children\) \{/);
  assert.match(primitives, /<details/);
  assert.match(primitives, /role="menu" aria-label=\{resolvedMenuLabel\}/);
  assert.match(primitives, /\{children\}/);
});

test("date and time stay bounded browser-native controls", () => {
  assert.match(forms, /inputType="date"/);
  assert.match(forms, /inputType="time"/);
  assert.match(forms, /inputType="datetime-local"/);
  assert.doesNotMatch(forms, /Calendar|DatePicker|RangePicker|TimeZone/);
});

test("neutral fixture consumes only public PDS APIs and local state", () => {
  assert.match(fixture, /from "@appfw\/pds-health-components"/);
  assert.doesNotMatch(fixture, /react-aria-components|ServiceNow|CRM|Nexus|tenant|fetch\(|XMLHttpRequest|WebSocket/);
  for (const mechanic of ["TextField", "ComboboxField", "MenuButton", "DateField", "TimeField"]) {
    assert.match(fixture, new RegExp(`\\b${mechanic}\\b`));
  }
  assert.match(fixture, /role="status"/);
  assert.match(main, /@appfw\/pds-health-components\/styles\.css/);
  assert.match(html, /src\/f1\/main\.tsx/);
});
