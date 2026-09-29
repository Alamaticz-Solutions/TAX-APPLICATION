#!/usr/bin/env node

import { existsSync, readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const frontendRoot = join(dirname(fileURLToPath(import.meta.url)), "..");
const jsonMode = process.argv.includes("--json");
const failures = [];
const warnings = [];

const packageJson = readJson("package.json", true);
const manifest = readJson(".appfw-ui/scaffold-manifest.json", true);
const ownership = readJson(".appfw-ui/ownership.json", true);
const contractPath = manifest?.generatedContract ?? "src/generated/appfw-ui-contract.ts";
const contractText = readText(contractPath, true);

if (manifest) {
  assertEqual(manifest.source, "app_gen", "manifest source should be app_gen");
  assertEqual(
    manifest.packageKind,
    "product_frontend_scaffold",
    "manifest packageKind should be product_frontend_scaffold"
  );
  assertArray("manifest.schemaNames", manifest.schemaNames);
  assertArray("manifest.contracts", manifest.contracts);
  assertArray("manifest.generatedRoots", manifest.generatedRoots);
  assertArray("manifest.scaffoldRoots", manifest.scaffoldRoots);
  assertArray("manifest.humanOwnedRoots", manifest.humanOwnedRoots);
  assertArray("manifest.requiredPackageScripts", manifest.requiredPackageScripts);
  assertExists(manifest.ownershipManifest ?? ".appfw-ui/ownership.json");
  for (const root of [
    ...(manifest.generatedRoots ?? []),
    ...(manifest.scaffoldRoots ?? []),
    ...(manifest.humanOwnedRoots ?? [])
  ]) {
    assertGlobRootExists(root);
  }
}

if (ownership) {
  assertArray("ownership.roots.generated", ownership.roots?.generated);
  assertArray("ownership.roots.scaffold", ownership.roots?.scaffold);
  assertArray("ownership.roots.humanOwned", ownership.roots?.humanOwned);
}

if (packageJson && manifest) {
  const scripts = packageJson.scripts ?? {};
  for (const scriptName of manifest.requiredPackageScripts ?? []) {
    if (!scripts[scriptName]) {
      failures.push(`package.json is missing required script: ${scriptName}`);
    }
  }
}

if (contractText && manifest) {
  if (!contractText.includes("export const appfwUiContracts")) {
    failures.push(`${contractPath} does not export appfwUiContracts`);
  }
  for (const schemaName of manifest.schemaNames ?? []) {
    if (!contractText.includes(`"schemaName": "${schemaName}"`)) {
      failures.push(`${contractPath} does not contain schema contract ${schemaName}`);
    }
  }
}

if (manifest?.contracts && manifest?.schemaNames) {
  const contractSchemas = manifest.contracts.map((contract) => contract.schemaName).filter(Boolean);
  const missingSummaries = manifest.schemaNames.filter((schemaName) => !contractSchemas.includes(schemaName));
  for (const schemaName of missingSummaries) {
    failures.push(`manifest.contracts is missing summary for schema ${schemaName}`);
  }
}

const result = {
  command: "appfw:check",
  ok: failures.length === 0,
  failures,
  warnings,
  checked: {
    manifest: ".appfw-ui/scaffold-manifest.json",
    ownership: ".appfw-ui/ownership.json",
    contract: contractPath
  }
};

if (jsonMode) {
  console.log(JSON.stringify(result, null, 2));
} else if (result.ok) {
  console.log("appfw:check ok");
} else {
  console.error("appfw:check failed");
  for (const failure of failures) {
    console.error(`- ${failure}`);
  }
}

process.exit(result.ok ? 0 : 1);

function readJson(relativePath, required = false) {
  const text = readText(relativePath, required);
  if (!text) return null;
  try {
    return JSON.parse(text);
  } catch (error) {
    failures.push(`${relativePath} is not valid JSON: ${error.message}`);
    return null;
  }
}

function readText(relativePath, required = false) {
  const absolutePath = join(frontendRoot, relativePath);
  if (!existsSync(absolutePath)) {
    if (required) failures.push(`missing ${relativePath}`);
    return null;
  }
  return readFileSync(absolutePath, "utf8");
}

function assertEqual(actual, expected, message) {
  if (actual !== expected) {
    failures.push(`${message}; received ${JSON.stringify(actual)}`);
  }
}

function assertArray(label, value) {
  if (!Array.isArray(value)) {
    failures.push(`${label} must be an array`);
  }
}

function assertExists(relativePath) {
  if (!existsSync(join(frontendRoot, relativePath))) {
    failures.push(`missing ${relativePath}`);
  }
}

function assertGlobRootExists(pattern) {
  if (typeof pattern !== "string" || pattern.length === 0) {
    failures.push(`invalid manifest root pattern: ${JSON.stringify(pattern)}`);
    return;
  }
  const root = pattern.replace(/\/\*\*$/, "");
  if (!existsSync(join(frontendRoot, root))) {
    failures.push(`manifest root does not exist: ${pattern}`);
  }
}
