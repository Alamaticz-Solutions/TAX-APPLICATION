#!/usr/bin/env node
/**
 * PDS token generator (W1a of the experience dimension plan).
 *
 * `appfw_ui/pds_health/tokens/tokens.dtcg.json` is the canonical design-token
 * source. This script deterministically renders the two consumable artifacts:
 *
 *   - appfw_ui/pds_health/tokens/pdsTokens.css
 *   - appfw_ui/pds_health/tokens/pdsTokens.ts
 *
 * Modes:
 *   node scripts/generate-pds-tokens.mjs             # regenerate both files
 *   node scripts/generate-pds-tokens.mjs --check     # fail (exit 1) on drift
 *   node scripts/generate-pds-tokens.mjs --bootstrap-from-files
 *       # review-only migration escape hatch: re-derive tokens.dtcg.json
 *       # from an inspected CSS/TS state. The JSON remains canonical; this
 *       # inversion is not the normal authoring workflow.
 *
 * The transitional W1a JSON stores ordered token declarations in `pds.root`
 * and `pds.materialLike` extension records, plus `pds.render` metadata for
 * emission order, attached comments, and line-wrap facts. It does not yet use
 * DTCG $type/$value token groups. Non-token CSS (header comment, @font-face
 * blocks, theme-mode blocks) and curated TS exports remain structured
 * passthrough sections pending later W1 slices.
 */
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const repoRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const tokensRoot = path.join(repoRoot, "appfw_ui/pds_health/tokens");
const jsonPath = path.join(tokensRoot, "tokens.dtcg.json");
const cssPath = path.join(tokensRoot, "pdsTokens.css");
const tsPath = path.join(tokensRoot, "pdsTokens.ts");

const args = new Set(process.argv.slice(2));
const checkMode = args.has("--check");
const bootstrapMode = args.has("--bootstrap-from-files");
const jsonMode = args.has("--json");

function inferType(value) {
  if (/^#[0-9a-fA-F]{3,8}$/.test(value)) return "color";
  if (/^light-dark\(/.test(value) || /^rgba?\(/.test(value)) return "color";
  if (/^linear-gradient\(/.test(value)) return "gradient";
  if (/^cubic-bezier\(/.test(value)) return "cubicBezier";
  if (/^\d+(\.\d+)?ms$/.test(value)) return "duration";
  if (/^-?\d+(\.\d+)?(px|%)$/.test(value)) return "dimension";
  if (/^var\(--[a-z0-9-]+\)$/.test(value)) return "alias";
  if (/^\d+(\.\d+)?$/.test(value)) return "number";
  return "raw";
}

// --- parsing (bootstrap) ----------------------------------------------------

function parseDeclarations(blockText) {
  // Sequential items inside a selector block: multiline comments attach to the
  // next declaration; every declaration in the current files is single-line.
  const items = [];
  const lines = blockText.split("\n");
  let pendingComment = null;
  let pendingBlank = false;
  for (const line of lines) {
    const trimmed = line.trim();
    if (trimmed === "") {
      pendingBlank = true;
      continue;
    }
    if (trimmed.startsWith("/*")) {
      pendingComment = [line];
      if (trimmed.endsWith("*/")) {
        // single-line comment
      } else {
        pendingComment.open = true;
      }
      continue;
    }
    if (pendingComment && pendingComment.open) {
      pendingComment.push(line);
      if (trimmed.endsWith("*/")) pendingComment.open = false;
      continue;
    }
    const match = line.match(/^\s*(--[a-z0-9-]+|color-scheme):\s*(.*);\s*$/i);
    if (!match) {
      throw new Error(`unparsed token line during bootstrap: ${line}`);
    }
    items.push({
      prop: match[1],
      value: match[2],
      comment: pendingComment ? pendingComment.join("\n") : undefined,
      blankBefore: pendingBlank || undefined
    });
    pendingComment = null;
    pendingBlank = false;
  }
  return items;
}

function extractBlock(css, selector) {
  const start = css.indexOf(`${selector} {`);
  if (start === -1) throw new Error(`selector not found: ${selector}`);
  const open = css.indexOf("{", start);
  const close = css.indexOf("\n}", open);
  return {
    inner: css.slice(open + 2, close),
    raw: css.slice(start, close + 2)
  };
}

function parseTsTreeRenderFacts(ts) {
  // Record which pdsTokens tree entries wrap their value onto the next line so
  // regeneration reproduces the file byte-for-byte. Wrap facts are keyed by
  // full dotted path — leaf names collide across groups (surface.panel is
  // unwrapped while gradient.panel wraps).
  const treeStart = ts.indexOf("export const pdsTokens = {");
  const treeEnd = ts.indexOf("} as const satisfies DesignTokenTree;");
  const treeLines = ts.slice(treeStart, treeEnd).split("\n");
  const wrapped = [];
  const keyOrder = [];
  const stack = [];
  for (let i = 0; i < treeLines.length; i += 1) {
    const line = treeLines[i];
    const openMatch = line.match(/^\s*([A-Za-z0-9_"]+): \{\s*$/);
    if (openMatch) {
      stack.push(openMatch[1].replace(/"/g, ""));
      keyOrder.push(stack.join("."));
      continue;
    }
    if (/^\s*\},?\s*$/.test(line)) {
      stack.pop();
      continue;
    }
    const wrapMatch = line.match(/^\s*([A-Za-z0-9_"]+):\s*$/);
    if (wrapMatch) {
      const dotted = [...stack, wrapMatch[1].replace(/"/g, "")].join(".");
      wrapped.push(dotted);
      keyOrder.push(dotted);
      continue;
    }
    const leafMatch = line.match(/^\s*([A-Za-z0-9_"]+): ["']/);
    if (leafMatch) {
      keyOrder.push([...stack, leafMatch[1].replace(/"/g, "")].join("."));
    }
  }
  return { wrapped, keyOrder };
}

function bootstrap() {
  const css = fs.readFileSync(cssPath, "utf8");
  const ts = fs.readFileSync(tsPath, "utf8");

  const headerEnd = css.indexOf("@font-face");
  const headerComment = css.slice(0, headerEnd).trimEnd();

  const fontFaces = [];
  const fontFaceRegex = /@font-face \{[\s\S]*?\n\}/g;
  let fontFaceMatch;
  while ((fontFaceMatch = fontFaceRegex.exec(css)) !== null) {
    fontFaces.push(fontFaceMatch[0]);
  }

  const root = extractBlock(css, ":root");
  const material = extractBlock(css, ':root[data-visual-theme="material-like"]');
  const materialCommentStart = css.indexOf("/*\n * Material-like");
  const materialComment = css
    .slice(materialCommentStart, css.indexOf(':root[data-visual-theme="material-like"]'))
    .trimEnd();
  const lightBlock = extractBlock(css, ':root[data-theme="light"]');
  const darkBlock = extractBlock(css, ':root[data-theme="dark"]');

  const rootItems = parseDeclarations(root.inner);
  const materialItems = parseDeclarations(material.inner);

  const tsFacts = parseTsTreeRenderFacts(ts);

  // Curated TS export sections are carried verbatim; the pdsTokens tree is the
  // generated portion of the TS file.
  const tsHeaderEnd = ts.indexOf("export const pdsTokens = {");
  const tsHeader = ts.slice(0, tsHeaderEnd).trimEnd();
  const treeEndMarker = "} as const satisfies DesignTokenTree;";
  const tsAfterTree = ts
    .slice(ts.indexOf(treeEndMarker) + treeEndMarker.length)
    .replace(/^\n+/, "");

  const doc = {
    $schema: "https://design-tokens.github.io/community-group/format/",
    $description:
      "Canonical PDS design-token source (W1a). Generates pdsTokens.css and pdsTokens.ts via scripts/generate-pds-tokens.mjs; do not hand-edit the generated files. This transitional W1a format stores ordered token declarations in pds.root and pds.materialLike extension records plus render metadata in pds.render; it does not yet use DTCG $type/$value token groups. Later W1 slices may migrate to native DTCG groups while adding scale, density, window-class, and OKLCH tokens.",
    pds: {
      render: {
        cssHeaderComment: headerComment,
        fontFaces,
        materialComment,
        themeModeBlocks: [lightBlock.raw, darkBlock.raw],
        tsHeader,
        tsAfterTree,
        tsWrappedKeys: tsFacts.wrapped,
        tsKeyOrder: tsFacts.keyOrder
      },
      root: rootItems,
      materialLike: materialItems
    }
  };

  fs.writeFileSync(jsonPath, `${JSON.stringify(doc, null, 2)}\n`);
  return doc;
}

// --- rendering ----------------------------------------------------------------

function renderCss(doc) {
  const { render, root, materialLike } = doc.pds;
  const parts = [render.cssHeaderComment];
  for (const face of render.fontFaces) {
    parts.push(face, "");
  }
  parts.push(":root {");
  for (const item of root) {
    if (item.comment) parts.push(item.comment);
    parts.push(`  ${item.prop}: ${item.value};`);
  }
  parts.push("}", "");
  parts.push(render.materialComment);
  parts.push(':root[data-visual-theme="material-like"] {');
  for (const item of materialLike) {
    if (item.blankBefore) parts.push("");
    if (item.comment) parts.push(item.comment);
    parts.push(`  ${item.prop}: ${item.value};`);
  }
  parts.push("}", "");
  parts.push(render.themeModeBlocks[0], "");
  parts.push(render.themeModeBlocks[1]);
  return `${parts.join("\n")}\n`;
}

const TS_GROUP_RULES = [
  { cssPrefix: "--pds-color-", treePath: ["color"] },
  { cssPrefix: "--pds-gradient-", treePath: ["gradient"] },
  { cssPrefix: "--pds-font-", treePath: ["font"] },
  { cssPrefix: "--pds-space-", treePath: ["space"] },
  { cssPrefix: "--pds-radius-", treePath: ["radius"] },
  { cssPrefix: "--pds-shadow-", treePath: ["shadow"] },
  { cssPrefix: "--pds-backdrop-", treePath: ["backdrop"] },
  { cssPrefix: "--pds-motion-", treePath: ["motion"] }
];

// Tree shapes and irregular key mappings observed in the hand-authored file.
const TS_PATH_OVERRIDES = new Map([
  ["--pds-color-spotlight", "color.spotlight"],
  ["--pds-color-spotlight-violet", "color.spotlightViolet"],
  ["--pds-color-spotlight-warm", "color.spotlightWarm"],
  ["--pds-color-backdrop-scrim", "color.backdropScrim"],
  ["--pds-color-intelligence-surface", "color.intelligence.surface"],
  ["--pds-color-intelligence-border", "color.intelligence.border"],
  ["--pds-font-family-sans", "font.familySans"],
  ["--pds-font-family-mono", "font.familyMono"],
  ["--pds-font-family-display", "font.familyDisplay"],
  ["--pds-font-feature-sans", "font.featureSans"],
  ["--pds-font-optical-sizing", "font.opticalSizing"],
  ["--pds-font-variant-metric", "font.variantMetric"],
  ["--pds-motion-distance-enter", "motion.distanceEnter"],
  ["--pds-motion-fluid", "motion.fluid"]
]);

// Pre-existing divergences between the hand-maintained CSS and TS files,
// captured verbatim so the W1a migration is byte-for-byte behavior-preserving.
// Reconcile deliberately in a later W1 slice (tracked in the dimension spec).
const TS_VALUE_DIVERGENCES = new Map([
  ["--pds-color-state-gold", "#ffb020"],
  ["--pds-motion-fluid", "cubic-bezier(0.16, 1, 0.3, 1)"]
]);

const TS_TREE_SKIP = new Set([
  "color-scheme",
  "--pds-font-sans",
  "--pds-font-mono",
  "--pds-font-display",
  "--pds-motion-duration-field-label",
  "--pds-motion-easing-field-label"
]);

function reorderTree(node, keyOrder, pathPrefix) {
  // Serialize following the captured key order of the hand-authored file;
  // paths not present in the order list append after their ordered siblings
  // in insertion order (new tokens land deterministically at the end).
  const ordered = {};
  const childOrder = keyOrder
    .filter((dotted) => {
      if (pathPrefix === "") return !dotted.includes(".");
      return (
        dotted.startsWith(`${pathPrefix}.`) &&
        !dotted.slice(pathPrefix.length + 1).includes(".")
      );
    })
    .map((dotted) => (pathPrefix === "" ? dotted : dotted.slice(pathPrefix.length + 1)));
  for (const key of childOrder) {
    if (key in node) ordered[key] = node[key];
  }
  for (const key of Object.keys(node)) {
    if (!(key in ordered)) ordered[key] = node[key];
  }
  for (const [key, value] of Object.entries(ordered)) {
    if (typeof value === "object" && value !== null) {
      ordered[key] = reorderTree(
        value,
        keyOrder,
        pathPrefix === "" ? key : `${pathPrefix}.${key}`
      );
    }
  }
  return ordered;
}

function kebabSegmentsToCamel(segments) {
  const joined = segments.join("-");
  return joined.replace(/-([a-z0-9])/g, (_, ch) => ch.toUpperCase());
}

function tsPathFor(prop) {
  if (TS_TREE_SKIP.has(prop)) return null;
  const override = TS_PATH_OVERRIDES.get(prop);
  if (override) return override;
  for (const rule of TS_GROUP_RULES) {
    if (!prop.startsWith(rule.cssPrefix)) continue;
    const rest = prop.slice(rule.cssPrefix.length).split("-");
    const group = rule.treePath[0];
    if (group === "color" || group === "motion" || group === "font") {
      const sub = rest[0];
      const leaf = kebabSegmentsToCamel(rest.slice(1));
      if (rest.length === 1) return `${group}.${kebabSegmentsToCamel(rest)}`;
      return `${group}.${sub}.${leaf}`;
    }
    if (group === "space") return `space.${rest.join("")}`;
    return `${group}.${kebabSegmentsToCamel(rest)}`;
  }
  return null;
}

function setDeep(target, dottedPath, value) {
  const segments = dottedPath.split(".");
  let cursor = target;
  for (const segment of segments.slice(0, -1)) {
    cursor[segment] = cursor[segment] ?? {};
    cursor = cursor[segment];
  }
  cursor[segments[segments.length - 1]] = value;
}

function tsQuote(value) {
  return value.includes('"') ? `'${value}'` : `"${value}"`;
}

function tsKey(key) {
  return /^[A-Za-z_][A-Za-z0-9_]*$/.test(key) ? key : `"${key}"`;
}

function renderTsTree(node, indent, wrapped, pathPrefix) {
  const pad = " ".repeat(indent);
  const lines = [];
  const entries = Object.entries(node);
  entries.forEach(([key, value], index) => {
    const comma = index === entries.length - 1 ? "" : ",";
    const dottedPath = pathPrefix ? `${pathPrefix}.${key}` : key;
    if (typeof value === "string") {
      const rendered = tsQuote(value);
      if (wrapped.has(dottedPath)) {
        lines.push(`${pad}${tsKey(key)}:`);
        lines.push(`${pad}  ${rendered}${comma}`);
      } else {
        lines.push(`${pad}${tsKey(key)}: ${rendered}${comma}`);
      }
    } else {
      lines.push(`${pad}${tsKey(key)}: {`);
      lines.push(renderTsTree(value, indent + 2, wrapped, dottedPath));
      lines.push(`${pad}}${comma}`);
    }
  });
  return lines.join("\n");
}

function renderTs(doc) {
  const { render, root } = doc.pds;
  let tree = {};
  for (const item of root) {
    const treePath = tsPathFor(item.prop);
    if (!treePath) continue;
    setDeep(tree, treePath, item.tsValue ?? TS_VALUE_DIVERGENCES.get(item.prop) ?? item.value);
  }
  tree = reorderTree(tree, render.tsKeyOrder ?? [], "");
  const wrapped = new Set(render.tsWrappedKeys);
  const body = renderTsTree(tree, 2, wrapped, "");
  return `${render.tsHeader}\n\nexport const pdsTokens = {\n${body}\n} as const satisfies DesignTokenTree;\n\n${render.tsAfterTree}`;
}

// --- main ---------------------------------------------------------------------

function report(ok, detail) {
  const payload = {
    command: "generate-pds-tokens",
    mode: bootstrapMode ? "bootstrap" : checkMode ? "check" : "generate",
    ok,
    ...detail
  };
  if (jsonMode) {
    console.log(JSON.stringify(payload, null, 2));
  } else if (!ok) {
    console.error(`generate-pds-tokens: ${JSON.stringify(detail)}`);
  }
  if (!ok) process.exit(1);
}

function firstDifference(a, b) {
  const limit = Math.min(a.length, b.length);
  for (let i = 0; i < limit; i += 1) {
    if (a[i] !== b[i]) {
      return { at: i, expected: a.slice(i, i + 80), got: b.slice(i, i + 80) };
    }
  }
  if (a.length !== b.length) {
    return { at: limit, expected: a.slice(limit, limit + 80), got: b.slice(limit, limit + 80) };
  }
  return null;
}

if (bootstrapMode) {
  bootstrap();
  // Prove the canonical artifact on disk round-trips before accepting it.
  const doc = JSON.parse(fs.readFileSync(jsonPath, "utf8"));
  const cssOut = renderCss(doc);
  const tsOut = renderTs(doc);
  const cssActual = fs.readFileSync(cssPath, "utf8");
  const tsActual = fs.readFileSync(tsPath, "utf8");
  report(cssOut === cssActual && tsOut === tsActual, {
    json: path.relative(repoRoot, jsonPath),
    css_roundtrip: cssOut === cssActual,
    ts_roundtrip: tsOut === tsActual,
    css_first_difference: firstDifference(cssActual, cssOut),
    ts_first_difference: firstDifference(tsActual, tsOut),
    root_tokens: doc.pds.root.length,
    material_tokens: doc.pds.materialLike.length
  });
} else {
  const doc = JSON.parse(fs.readFileSync(jsonPath, "utf8"));
  const cssOut = renderCss(doc);
  const tsOut = renderTs(doc);
  if (checkMode) {
    const cssActual = fs.existsSync(cssPath) ? fs.readFileSync(cssPath, "utf8") : "";
    const tsActual = fs.existsSync(tsPath) ? fs.readFileSync(tsPath, "utf8") : "";
    report(cssOut === cssActual && tsOut === tsActual, {
      css_in_sync: cssOut === cssActual,
      ts_in_sync: tsOut === tsActual,
      hint: "run node scripts/generate-pds-tokens.mjs to regenerate from tokens.dtcg.json"
    });
  } else {
    fs.writeFileSync(cssPath, cssOut);
    fs.writeFileSync(tsPath, tsOut);
    report(true, {
      wrote: [path.relative(repoRoot, cssPath), path.relative(repoRoot, tsPath)]
    });
  }
}
