import assert from "node:assert/strict";
import fs from "node:fs";
import path from "node:path";
import test from "node:test";
import { createElement } from "react";
import { renderToStaticMarkup } from "react-dom/server";
import { fileURLToPath } from "node:url";
import { NarrativeWorkspace } from "../dist/narrative-workspace.js";

test("narrative workspace owns one ordered document-scrolling landmark set", () => {
  const html = renderToStaticMarkup(
    createElement(NarrativeWorkspace, {
      brand: createElement("strong", null, "Knowledge application"),
      primaryNavigation: createElement("button", { type: "button", "aria-pressed": true }, "Story"),
      primaryNavigationLabel: "Knowledge modes",
      utilities: createElement("button", { type: "button" }, "Ask"),
      contextBand: createElement("label", null, "Role", createElement("select", null, createElement("option", null, "All readers"))),
      contextBandLabel: "Reader context",
      footer: createElement("span", null, "Release identity"),
      mainId: "knowledge-main",
      children: createElement("article", null, "Narrative content")
    })
  );

  assert.match(html, /class="pds-narrative-workspace"/);
  assert.match(html, /data-has-utilities="true"/);
  assert.match(html, /data-has-context="true"/);
  assert.match(html, /data-has-footer="true"/);
  assert.match(html, /href="#knowledge-main"/);
  assert.match(html, /<header class="pds-narrative-workspace__sticky-header">/);
  assert.match(html, /<nav class="pds-narrative-workspace__navigation" aria-label="Knowledge modes">/);
  assert.match(html, /<section class="pds-narrative-workspace__context" aria-label="Reader context">/);
  assert.match(html, /<main id="knowledge-main" class="pds-narrative-workspace__main" tabindex="-1">/);
  assert.match(html, /<footer class="pds-narrative-workspace__footer">/);

  assert.equal((html.match(/<header/g) ?? []).length, 1);
  assert.equal((html.match(/<nav/g) ?? []).length, 1);
  assert.equal((html.match(/<main/g) ?? []).length, 1);
  assert.ok(html.indexOf("__skip-link") < html.indexOf("__sticky-header"));
  assert.ok(html.indexOf("__sticky-header") < html.indexOf("__context"));
  assert.ok(html.indexOf("__context") < html.indexOf("__main"));
});

test("narrative workspace keeps optional regions optional", () => {
  const html = renderToStaticMarkup(
    createElement(NarrativeWorkspace, {
      brand: "Knowledge application",
      primaryNavigation: createElement("a", { href: "#overview", "aria-current": "page" }, "Overview"),
      children: createElement("p", null, "Content")
    })
  );

  assert.match(html, /aria-label="Primary navigation"/);
  assert.doesNotMatch(html, /pds-narrative-workspace__utilities/);
  assert.doesNotMatch(html, /pds-narrative-workspace__context/);
  assert.doesNotMatch(html, /pds-narrative-workspace__footer/);
});

test("narrative workspace styles preserve document scroll and visible adaptive chrome", () => {
  const packageRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
  const styles = fs.readFileSync(path.join(packageRoot, "src/styles.css"), "utf8");

  assert.match(styles, /\.pds-narrative-workspace\s*\{[\s\S]*overflow:\s*visible/);
  assert.match(styles, /\.pds-narrative-workspace__sticky-header\s*\{[\s\S]*position:\s*sticky/);
  assert.match(styles, /env\(safe-area-inset-top/);
  assert.match(styles, /--pds-narrative-sticky-stack-size/);
  assert.match(styles, /--pds-narrative-sticky-offset/);
  assert.match(styles, /\.pds-narrative-workspace__main\s*\{[\s\S]*overflow:\s*visible/);
  assert.match(styles, /@media \(max-width:\s*900px\)/);
  assert.match(styles, /@media \(max-width:\s*720px\)/);
  assert.match(styles, /@media \(pointer:\s*coarse\)/);
  assert.match(styles, /min-block-size:\s*44px/);
  assert.match(styles, /@media \(forced-colors:\s*active\)/);
  assert.match(styles, /@media print/);
  assert.doesNotMatch(styles, /\.pds-narrative-workspace[\s\S]{0,500}height:\s*100dvh/);
});
