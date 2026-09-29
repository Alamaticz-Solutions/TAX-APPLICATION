import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import { dirname, resolve } from "node:path";
import test from "node:test";
import { fileURLToPath } from "node:url";
import { createElement } from "react";
import { renderToStaticMarkup } from "react-dom/server";
import {
  ConnectedFabric
} from "../dist/connected-fabric.js";

const componentRoot = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const source = await readFile(
  resolve(componentRoot, "src/connected-fabric.tsx"),
  "utf8"
);

test("ConnectedFabric exposes the reviewed appearance and selector boundary", () => {
  assert.match(source, /export type ConnectedFabricProps = \{/);
  for (const prop of [
    "theme: AppearanceColorMode",
    "visualTheme: AppearanceVisualTheme",
    "contentRootSelector?: string",
    "panelSelector?: string",
    "scrollRootSelector?: string"
  ]) {
    assert.match(source, new RegExp(prop.replace(/[?:]/g, "\\$&")));
  }
});

test("ConnectedFabric remains decorative and advertises the protected runtime contract", () => {
  const html = renderToStaticMarkup(
    createElement(ConnectedFabric, {
      theme: "light",
      visualTheme: "apple-like"
    })
  );

  assert.match(html, /class="pds-connected-fabric catalog__connected-fabric"/);
  assert.match(html, /aria-hidden="true"/);
  assert.match(html, /data-pds-visual-language="connected-fabric"/);
  assert.match(html, /data-semantic-role="decorative"/);
  assert.match(html, /data-edge-policy="shared-nodes-exclusive-edges"/);
  assert.match(html, /data-edge-conflict-detected="false"/);
});
