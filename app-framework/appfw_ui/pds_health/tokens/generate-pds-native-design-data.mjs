#!/usr/bin/env node

import { createHash } from "node:crypto";
import { mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const root = dirname(fileURLToPath(import.meta.url));
const sourcePath = resolve(root, "tokens.dtcg.json");
const jsonPath = resolve(root, "pdsNativeDesignData.json");
const tsPath = resolve(root, "pdsNativeDesignData.ts");
const nativePackageTsPath = resolve(root, "../native-components/src/generated/pdsNativeDesignData.ts");
const args = new Set(process.argv.slice(2));
const check = args.has("--check");
const json = args.has("--json");

const sourceBytes = readFileSync(sourcePath);
const source = JSON.parse(sourceBytes.toString("utf8"));
const declarations = new Map(source.pds.root.map(({ prop, value }) => [prop, value]));

const colorRoles = {
  action: { light: "--pds-color-brand-blue-deep", dark: "--pds-color-brand-blue-bright" },
  onAction: { light: "--pds-color-text-on-accent", dark: "--pds-color-text-inverse" },
  canvas: "--pds-color-surface-canvas-start",
  surface: "--pds-color-surface-panel",
  subtle: "--pds-color-surface-panel-soft",
  elevated: "--pds-color-surface-elevated",
  intelligence: "--pds-color-intelligence-surface",
  text: "--pds-color-text-default",
  muted: "--pds-color-text-muted",
  inverse: "--pds-color-text-inverse",
  border: "--pds-color-border-default",
  accent: "--pds-color-border-accent",
  focus: "--pds-color-state-focus",
  active: "--pds-color-signal-blue",
  success: "--pds-color-state-success",
  warning: "--pds-color-state-warning",
  danger: "--pds-color-state-danger"
};

function sha256(value) {
  return `sha256:${createHash("sha256").update(value).digest("hex")}`;
}

function splitTopLevel(value) {
  let depth = 0;
  for (let index = 0; index < value.length; index += 1) {
    if (value[index] === "(") depth += 1;
    else if (value[index] === ")") depth -= 1;
    else if (value[index] === "," && depth === 0) {
      return [value.slice(0, index).trim(), value.slice(index + 1).trim()];
    }
  }
  throw new Error(`expected a top-level comma in ${value}`);
}

function dereference(value, seen = new Set()) {
  const alias = value.match(/^var\((--[a-z0-9-]+)\)$/);
  if (!alias) return value;
  if (seen.has(alias[1])) throw new Error(`cyclic PDS token alias: ${alias[1]}`);
  const target = declarations.get(alias[1]);
  if (target === undefined) throw new Error(`unknown PDS token alias: ${alias[1]}`);
  return dereference(target, new Set([...seen, alias[1]]));
}

function themed(value) {
  const resolved = dereference(value);
  if (!resolved.startsWith("light-dark(") || !resolved.endsWith(")")) {
    return { light: resolved, dark: resolved };
  }
  const inner = resolved.slice("light-dark(".length, -1);
  const [light, dark] = splitTopLevel(inner);
  return { light: dereference(light), dark: dereference(dark) };
}

function numberToken(name) {
  const value = dereference(declarations.get(name) ?? "");
  const match = value.match(/^(\d+(?:\.\d+)?)px$/);
  if (!match) throw new Error(`${name} must resolve to a px dimension, got ${value}`);
  return Number(match[1]);
}

function durationToken(name) {
  const value = dereference(declarations.get(name) ?? "");
  const match = value.match(/^(\d+(?:\.\d+)?)ms$/);
  if (!match) throw new Error(`${name} must resolve to an ms duration, got ${value}`);
  return Number(match[1]);
}

function scalarToken(name) {
  const value = dereference(declarations.get(name) ?? "");
  if (!/^\d+(?:\.\d+)?$/.test(value)) {
    throw new Error(`${name} must resolve to a numeric scalar, got ${value}`);
  }
  return Number(value);
}

function tokenForScheme(mapping, scheme) {
  return typeof mapping === "string" ? mapping : mapping[scheme];
}

function resolveRole(mapping, scheme) {
  const token = tokenForScheme(mapping, scheme);
  const value = declarations.get(token);
  if (value === undefined) throw new Error(`missing canonical PDS token: ${token}`);
  return themed(value)[scheme];
}

const lightColors = {};
const darkColors = {};
const sources = {};
for (const [role, mapping] of Object.entries(colorRoles)) {
  lightColors[role] = resolveRole(mapping, "light");
  darkColors[role] = resolveRole(mapping, "dark");
  sources[`color.${role}`] = typeof mapping === "string"
    ? mapping
    : { light: mapping.light, dark: mapping.dark };
}

const spacing = {
  "4": numberToken("--pds-space-1"),
  "8": numberToken("--pds-space-2"),
  "12": numberToken("--pds-space-3"),
  "16": numberToken("--pds-space-4"),
  "20": numberToken("--pds-space-5"),
  "24": numberToken("--pds-space-6"),
  "32": numberToken("--pds-space-8")
};
const shape = {
  small: numberToken("--pds-radius-sm"),
  medium: numberToken("--pds-radius-md"),
  large: numberToken("--pds-radius-lg"),
  control: numberToken("--pds-radius-control"),
  panel: numberToken("--pds-radius-panel"),
  pill: numberToken("--pds-radius-pill")
};
const native = source.pds.native;
if (native?.schemaVersion !== "pds.native.policy@1") throw new Error("missing canonical PDS native policy");
const typography = Object.fromEntries(Object.entries(native.typography).map(([role, values]) => [
  role,
  {
    fontSize: numberToken(values.fontSize),
    lineHeight: values.lineHeight,
    fontWeight: scalarToken(values.fontWeight)
  }
]));
const geometry = native.geometry;
const motion = {
  standard: {
    enabled: native.motion.standard.enabled,
    fastDurationMs: durationToken("--pds-motion-duration-fast"),
    standardDurationMs: durationToken("--pds-motion-duration-standard"),
    slowDurationMs: durationToken("--pds-motion-duration-slow")
  },
  reduced: {
    enabled: native.motion.reduced.enabled,
    fastDurationMs: native.motion.reduced.durationMs,
    standardDurationMs: native.motion.reduced.durationMs,
    slowDurationMs: native.motion.reduced.durationMs
  }
};
const fontPolicy = native.fontPolicy;
const nativeVisualThemes = native.visualThemes;
const platforms = native.platforms;

Object.assign(sources, {
  spacing: {
    "4": "--pds-space-1", "8": "--pds-space-2", "12": "--pds-space-3",
    "16": "--pds-space-4", "20": "--pds-space-5", "24": "--pds-space-6", "32": "--pds-space-8"
  },
  shape: {
    small: "--pds-radius-sm", medium: "--pds-radius-md", large: "--pds-radius-lg",
    control: "--pds-radius-control", panel: "--pds-radius-panel", pill: "--pds-radius-pill"
  },
  typography: {
    caption: { fontSize: "--pds-font-size-sm", lineHeight: "pds.native.typography.caption.lineHeight", fontWeight: "--pds-font-weight-medium" },
    label: { fontSize: "--pds-font-size-md", lineHeight: "pds.native.typography.label.lineHeight", fontWeight: "--pds-font-weight-semibold" },
    body: { fontSize: "--pds-font-size-base", lineHeight: "pds.native.typography.body.lineHeight", fontWeight: "--pds-font-weight-regular" },
    title: { fontSize: "--pds-font-size-lg", lineHeight: "pds.native.typography.title.lineHeight", fontWeight: "--pds-font-weight-semibold" },
    display: { fontSize: "--pds-font-size-metric", lineHeight: "pds.native.typography.display.lineHeight", fontWeight: "--pds-font-weight-bold" }
  },
  geometry: "pds.native.geometry",
  motion: {
    fastDurationMs: "--pds-motion-duration-fast",
    standardDurationMs: "--pds-motion-duration-standard",
    slowDurationMs: "--pds-motion-duration-slow",
    reduced: "pds.native.motion.reduced"
  },
  fontPolicy: "pds.native.fontPolicy",
  visualThemes: "pds.native.visualThemes",
  platforms: "pds.native.platforms"
});

function nativeTokenSet(color, visualTheme, colorScheme) {
  return {
    identity: { visualTheme, colorScheme },
    color,
    spacing,
    typography,
    shape,
    geometry,
    motion,
    fontPolicy
  };
}

if (nativeVisualThemes?.default !== "apple-like"
  || JSON.stringify(nativeVisualThemes.supported) !== JSON.stringify(["apple-like"])) {
  throw new Error("native projection currently supports exactly the canonical apple-like visual theme");
}
if (platforms?.["native-ios"]?.visualTheme !== "apple-like"
  || platforms?.["native-ios"]?.qualification !== "not-qualified"
  || platforms?.["native-android"]?.visualTheme !== null
  || platforms?.["native-android"]?.qualification !== "not-qualified") {
  throw new Error("native platform qualification must remain truthful until physical platform evidence exists");
}

const projectionWithoutDigest = {
  schemaVersion: "pds.native.design-data@1",
  source: {
    path: "appfw_ui/pds_health/tokens/tokens.dtcg.json",
    sha256: sha256(sourceBytes)
  },
  defaultSelection: {
    visualTheme: nativeVisualThemes.default,
    colorScheme: "light"
  },
  visualThemes: {
    "apple-like": {
      light: nativeTokenSet(lightColors, "apple-like", "light"),
      dark: nativeTokenSet(darkColors, "apple-like", "dark")
    }
  },
  platforms,
  sources
};

const projection = {
  ...projectionWithoutDigest,
  projectionSha256: sha256(`${JSON.stringify(projectionWithoutDigest)}\n`)
};
const jsonOutput = `${JSON.stringify(projection, null, 2)}\n`;
const tsOutput = `// Generated from tokens.dtcg.json by generate-pds-native-design-data.mjs.\n// Do not hand-edit.\nexport const pdsNativeDesignData = ${JSON.stringify(projection, null, 2)} as const;\n\nexport type PdsNativeVisualTheme = keyof typeof pdsNativeDesignData.visualThemes;\nexport type PdsNativeColorScheme = keyof (typeof pdsNativeDesignData.visualThemes)[PdsNativeVisualTheme];\nexport type PdsNativeTokenSelection = { readonly visualTheme?: PdsNativeVisualTheme; readonly colorScheme?: PdsNativeColorScheme };\nexport type PdsNativeTokens = (typeof pdsNativeDesignData.visualThemes)[PdsNativeVisualTheme][PdsNativeColorScheme];\n\nexport function pdsNativeTokensFor({ visualTheme = "apple-like", colorScheme = "light" }: PdsNativeTokenSelection = {}): PdsNativeTokens {\n  return pdsNativeDesignData.visualThemes[visualTheme][colorScheme];\n}\n`;

const currentJson = (() => { try { return readFileSync(jsonPath, "utf8"); } catch { return ""; } })();
const currentTs = (() => { try { return readFileSync(tsPath, "utf8"); } catch { return ""; } })();
const currentNativePackageTs = (() => { try { return readFileSync(nativePackageTsPath, "utf8"); } catch { return ""; } })();
const inSync = currentJson === jsonOutput && currentTs === tsOutput && currentNativePackageTs === tsOutput;

if (!check) {
  writeFileSync(jsonPath, jsonOutput);
  writeFileSync(tsPath, tsOutput);
  mkdirSync(dirname(nativePackageTsPath), { recursive: true });
  writeFileSync(nativePackageTsPath, tsOutput);
}

const result = {
  command: "generate-pds-native-design-data",
  mode: check ? "check" : "generate",
  ok: check ? inSync : true,
  sourceSha256: projection.source.sha256,
  projectionSha256: projection.projectionSha256,
  jsonInSync: currentJson === jsonOutput,
  tsInSync: currentTs === tsOutput,
  nativePackageTsInSync: currentNativePackageTs === tsOutput
};

if (json) process.stdout.write(`${JSON.stringify(result, null, 2)}\n`);
else if (!result.ok) process.stderr.write("PDS native design-data projection drifted; regenerate it.\n");
else process.stdout.write(`${result.projectionSha256}\n`);

if (!result.ok) process.exit(1);
