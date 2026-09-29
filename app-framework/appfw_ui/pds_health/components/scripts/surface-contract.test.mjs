import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import { dirname, resolve } from "node:path";
import test from "node:test";
import { fileURLToPath } from "node:url";

const componentRoot = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const source = await readFile(
  resolve(componentRoot, "src/surfaces.tsx"),
  "utf8"
);
const styles = await readFile(resolve(componentRoot, "src/styles.css"), "utf8");

test("surface overflow is clipped by default and explicitly opt-in", () => {
  assert.match(source, /overflow\?: "clip" \| "visible"/);
  assert.match(source, /overflow = "clip"/);
  assert.match(source, /data-overflow=\{overflow\}/);
  assert.match(
    styles,
    /\.pds-surface\s*\{[\s\S]*?overflow:\s*hidden;/
  );
  assert.match(
    styles,
    /\.pds-surface\[data-overflow="visible"\]\s*\{[\s\S]*?overflow:\s*visible;/
  );
});
