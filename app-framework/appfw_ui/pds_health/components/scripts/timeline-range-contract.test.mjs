import assert from "node:assert/strict";
import fs from "node:fs";
import path from "node:path";
import test from "node:test";
import { fileURLToPath } from "node:url";

const packageRoot = path.resolve(
  path.dirname(fileURLToPath(import.meta.url)),
  ".."
);
const source = fs.readFileSync(
  path.join(packageRoot, "src/timeline.tsx"),
  "utf8"
);
const styles = fs.readFileSync(
  path.join(packageRoot, "src/styles.css"),
  "utf8"
);

test("timeline range exposes one movable and resizable selection", () => {
  assert.match(source, /export function TimelineRangeSelector/);
  assert.match(source, /"move" \| "resize-start" \| "resize-end"/);
  assert.match(source, /onSelectionChange: \(selection: TimelineRangeSelection\)/);
  assert.match(source, /normalizeSnapWidths/);
});

test("timeline range supports pointer and keyboard operation", () => {
  assert.match(source, /setPointerCapture/);
  assert.match(source, /role="slider"/);
  assert.match(source, /ArrowLeft/);
  assert.match(source, /ArrowRight/);
  assert.match(source, /PageDown/);
  assert.match(source, /PageUp/);
  assert.match(source, /aria-valuetext/);
});

test("timeline range has visible focus and reduced-motion treatment", () => {
  assert.match(styles, /\.pds-timeline-range__window:focus-visible/);
  assert.match(styles, /\.pds-timeline-range__handle:focus-visible/);
  assert.match(styles, /prefers-reduced-motion: reduce/);
  assert.match(styles, /\.pds-timeline-range__selection/);
});
