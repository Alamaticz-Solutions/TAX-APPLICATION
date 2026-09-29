import assert from "node:assert/strict";
import fs from "node:fs";
import path from "node:path";
import test from "node:test";
import { createElement } from "react";
import { renderToStaticMarkup } from "react-dom/server";
import { fileURLToPath } from "node:url";
import {
  RelationshipAtlas,
  RelationshipAtlasTable,
  RelationshipExplorer,
  relationshipAtlasValidationIssues
} from "../dist/relationship-atlas.js";

const groups = [
  { id: "inputs", label: "Inputs" },
  { id: "outcomes", label: "Outcomes" }
];
const nodes = [
  { id: "evidence", label: "Evidence", group: "inputs" },
  { id: "measure", label: "Measure", group: "outcomes" }
];
const edges = [{ source: "evidence", target: "measure", kind: "supports" }];

test("relationship atlas and table expose the same authored relationships", () => {
  const mapHtml = renderToStaticMarkup(createElement(RelationshipAtlas, {
    ariaLabel: "Relationship map",
    groups,
    nodes,
    edges,
    selectedId: "evidence",
    style: { maxWidth: 900 }
  }));
  const tableHtml = renderToStaticMarkup(createElement(RelationshipAtlasTable, {
    caption: "Relationship table",
    groups,
    nodes,
    edges,
    selectedId: "evidence",
    onSelectNode() {}
  }));

  assert.match(mapHtml, /aria-label="Relationship map"/);
  assert.match(mapHtml, /aria-pressed="true"/);
  assert.match(mapHtml, /max-width:900px/);
  assert.match(tableHtml, /<caption>Relationship table<\/caption>/);
  assert.match(tableHtml, /→ Measure \(supports\)/);
  for (const node of nodes) {
    assert.match(mapHtml, new RegExp(`>${node.label}<`));
    assert.match(tableHtml, new RegExp(`>${node.label}<`));
  }
});

test("invalid group and edge references are explicit and omitted from the map", () => {
  const invalidNodes = [...nodes, { id: "orphan", label: "Orphan", group: "missing" }];
  const invalidEdges = [...edges, { source: "unknown", target: "measure" }];
  const issues = relationshipAtlasValidationIssues(groups, invalidNodes, invalidEdges);
  assert.deepEqual(issues.map((issue) => issue.kind), ["unknown-group", "unknown-source"]);

  const html = renderToStaticMarkup(createElement(RelationshipAtlas, {
    ariaLabel: "Invalid relationship map",
    groups,
    nodes: invalidNodes,
    edges: invalidEdges
  }));
  assert.match(html, /data-validation-issues="2"/);
  assert.match(html, /role="status"/);
  assert.doesNotMatch(html, />Orphan</);
});

test("relationship explorer owns map-table presentation without owning semantics", () => {
  const html = renderToStaticMarkup(createElement(RelationshipExplorer, {
    ariaLabel: "Relationship explorer",
    tableCaption: "Relationship table",
    groups,
    nodes,
    edges,
    selectedId: "measure",
    onSelectNode() {}
  }));
  assert.match(html, /class="pds-atlas-explorer"/);
  assert.match(html, /aria-label="Relationship explorer presentation"/);
  assert.match(html, /data-view="map"/);
  assert.match(html, /aria-pressed="true"/);
});

test("relationship explorer bounds long accessible tables with explicit pagination", () => {
  const longNodes = Array.from({ length: 12 }, (_, index) => ({
    id: `node-${index + 1}`,
    label: `Node ${index + 1}`,
    group: "inputs"
  }));
  const html = renderToStaticMarkup(createElement(RelationshipExplorer, {
    ariaLabel: "Long relationship explorer",
    tableCaption: "Long relationship table",
    groups,
    nodes: longNodes,
    edges: [],
    view: "table",
    tablePageSize: 10
  }));

  assert.match(html, /Rows 1–10 of 12/);
  assert.match(html, /Page 1 of 2/);
  assert.match(html, />Node 10</);
  assert.doesNotMatch(html, />Node 11</);
  assert.match(html, />Previous</);
  assert.match(html, />Next</);
});

test("relationship atlas retains keyboard navigation and reduced-motion treatment", () => {
  const packageRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
  const source = fs.readFileSync(path.join(packageRoot, "src/relationship-atlas.tsx"), "utf8");
  const styles = fs.readFileSync(path.join(packageRoot, "src/styles.css"), "utf8");
  assert.match(source, /ArrowDown/);
  assert.match(source, /ArrowUp/);
  assert.match(source, /ArrowLeft/);
  assert.match(source, /ArrowRight/);
  assert.match(source, /event\.key === "Escape"/);
  assert.match(styles, /\.pds-atlas__node:focus-visible/);
  assert.match(styles, /prefers-reduced-motion: reduce/);
});
