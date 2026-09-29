import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import test from "node:test";
import { createElement } from "react";
import { renderToStaticMarkup } from "react-dom/server";
import { DateField, TextField } from "../dist/forms.js";

const styles = await readFile(new URL("../src/styles.css", import.meta.url), "utf8");

test("React Aria field wrappers inherit the visible field label", () => {
  const textHtml = renderToStaticMarkup(createElement(TextField, {
    label: "Search knowledge",
    name: "query"
  }));
  const dateHtml = renderToStaticMarkup(createElement(DateField, {
    label: "Observed date",
    name: "observedAt"
  }));

  assert.match(textHtml, /aria-label="Search knowledge"/);
  assert.match(dateHtml, /aria-label="Observed date"/);
});

test("popover fields reserve clearance for outlined floating labels", () => {
  assert.match(
    styles,
    /\.pds-popover\s*>\s*\.pds-text-field-field\s*\{[^}]*padding-block-start:\s*var\(--pds-space-3\)/s
  );
});
