import assert from "node:assert/strict";
import fs from "node:fs";
import path from "node:path";
import test from "node:test";
import { createElement } from "react";
import { renderToStaticMarkup } from "react-dom/server";
import { fileURLToPath } from "node:url";
import { ExplorationWorkspace } from "../dist/exploration-workspace.js";

test("exploration workspace composes ordered, labelled system regions", () => {
  const html = renderToStaticMarkup(
    createElement(ExplorationWorkspace, {
      title: "Strategy system",
      description: "Explore governed objects without losing source context.",
      headerActions: createElement("button", { type: "button" }, "Share"),
      controls: createElement("label", null, "Search", createElement("input", { type: "search" })),
      controlsLabel: "Strategy controls",
      navigator: createElement("ul", null, createElement("li", null, "Directives")),
      navigatorLabel: "Strategy objects",
      focusRegion: createElement("div", null, "Relationship map"),
      focusRegionLabel: "Strategy focus",
      inspector: createElement("article", null, "Selected directive"),
      inspectorLabel: "Directive details",
      evidenceRail: createElement("p", null, "Exact source proof"),
      evidenceRailLabel: "Authority and evidence"
    })
  );

  assert.match(html, /class="pds-exploration-workspace"/);
  assert.match(html, /data-presentation="bounded"/);
  assert.match(html, /data-has-controls="true"/);
  assert.match(html, /data-has-navigator="true"/);
  assert.match(html, /data-has-inspector="true"/);
  assert.match(html, /data-has-evidence="true"/);
  assert.match(html, /aria-labelledby="[^"]+-title"/);
  assert.match(html, /aria-describedby="[^"]+-description"/);
  assert.match(html, /class="pds-exploration-workspace__controls" aria-label="Strategy controls"/);
  assert.match(html, /<nav class="pds-exploration-workspace__navigator" aria-label="Strategy objects" tabindex="0">/);
  assert.match(html, /class="pds-exploration-workspace__focus" aria-label="Strategy focus" tabindex="0"/);
  assert.match(html, /class="pds-exploration-workspace__inspector" aria-label="Directive details" tabindex="0"/);
  assert.match(html, /class="pds-exploration-workspace__evidence" aria-label="Authority and evidence" tabindex="0"/);

  const navigatorPosition = html.indexOf("pds-exploration-workspace__navigator");
  const focusPosition = html.indexOf("pds-exploration-workspace__focus");
  const inspectorPosition = html.indexOf("pds-exploration-workspace__inspector");
  const evidencePosition = html.indexOf("pds-exploration-workspace__evidence");
  assert.ok(navigatorPosition < focusPosition);
  assert.ok(focusPosition < inspectorPosition);
  assert.ok(inspectorPosition < evidencePosition);
});

test("exploration workspace keeps a named focus region without optional rails", () => {
  const html = renderToStaticMarkup(
    createElement(ExplorationWorkspace, {
      focusRegion: createElement("p", null, "Focused content"),
      presentation: "flow"
    })
  );

  assert.match(html, /aria-label="Exploration workspace"/);
  assert.match(html, /data-presentation="flow"/);
  assert.match(html, /aria-label="Exploration focus"/);
  assert.doesNotMatch(html, /pds-exploration-workspace__header/);
  assert.doesNotMatch(html, /pds-exploration-workspace__controls/);
  assert.doesNotMatch(html, /pds-exploration-workspace__navigator/);
  assert.doesNotMatch(html, /pds-exploration-workspace__inspector/);
  assert.doesNotMatch(html, /pds-exploration-workspace__evidence/);
});

test("exploration workspace styles bound desktop regions and reflow narrow views", () => {
  const packageRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
  const styles = fs.readFileSync(path.join(packageRoot, "src/styles.css"), "utf8");

  assert.match(styles, /\.pds-exploration-workspace\[data-presentation="bounded"\]/);
  assert.match(styles, /block-size:\s*clamp\(/);
  assert.match(styles, /\.pds-exploration-workspace__body/);
  assert.match(styles, /grid-template-columns:[\s\S]*minmax\(0, 1fr\)/);
  assert.match(styles, /@container \(max-width: 900px\)/);
  assert.match(styles, /\.pds-exploration-workspace \{[^}]*container-type: inline-size/s);
  assert.match(styles, /\.pds-exploration-workspace__navigator[\s\S]*overscroll-behavior: contain/);
  assert.match(styles, /\.pds-exploration-workspace[\s\S]*:focus-visible/);
});
