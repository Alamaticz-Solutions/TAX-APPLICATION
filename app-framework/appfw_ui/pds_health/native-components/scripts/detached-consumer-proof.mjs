#!/usr/bin/env node

import assert from "node:assert/strict";
import { execFileSync } from "node:child_process";
import {
  mkdtempSync,
  mkdirSync,
  readFileSync,
  readdirSync,
  rmSync,
  statSync,
  writeFileSync
} from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const nativeRoot = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const pdsRoot = resolve(nativeRoot, "..");
const packageRoots = [
  resolve(pdsRoot, "ix-presentation-contract"),
  resolve(pdsRoot, "components"),
  nativeRoot
];
const scratch = mkdtempSync(join(tmpdir(), "pds-native-detached-consumer-"));
const archives = resolve(scratch, "archives");
const consumer = resolve(scratch, "consumer");
mkdirSync(archives);
mkdirSync(consumer);

function run(command, args, options = {}) {
  return execFileSync(command, args, {
    encoding: "utf8",
    stdio: ["ignore", "pipe", "pipe"],
    ...options
  });
}

function walk(root) {
  return readdirSync(root).flatMap((name) => {
    const path = join(root, name);
    return statSync(path).isDirectory() ? walk(path) : [path];
  });
}

try {
  run("npm", ["run", "build"], { cwd: resolve(pdsRoot, "components") });
  run("npm", ["run", "build"], { cwd: nativeRoot });

  const archivePaths = packageRoots.map((packageRoot) => {
    const output = JSON.parse(run("npm", [
      "pack",
      "--pack-destination",
      archives,
      "--json"
    ], {
      cwd: packageRoot,
      env: { ...process.env, npm_config_cache: resolve(scratch, "pack-cache") }
    }));
    assert.equal(output.length, 1);
    return resolve(archives, output[0].filename);
  });

  const archiveByFragment = (fragment) => archivePaths.find((path) => path.includes(fragment));
  const contractArchive = archiveByFragment("pds-ix-presentation-contract");
  const webArchive = archiveByFragment("pds-health-components");
  const nativeArchive = archiveByFragment("pds-health-native-");
  assert.ok(contractArchive && webArchive && nativeArchive);

  writeFileSync(resolve(consumer, "package.json"), `${JSON.stringify({
    private: true,
    type: "module",
    dependencies: {
      "@appfw/pds-health-components": "0.12.0",
      "@appfw/pds-health-native": "0.2.0",
      "@appfw/pds-ix-presentation-contract": "0.2.0",
      "@types/react": "19.2.18",
      react: "19.2.3",
      "react-dom": "19.2.3",
      "react-native": "0.85.3",
      typescript: "6.0.3"
    }
  }, null, 2)}\n`);
  writeFileSync(resolve(consumer, "tsconfig.json"), `${JSON.stringify({
    compilerOptions: {
      module: "NodeNext",
      moduleResolution: "NodeNext",
      target: "ES2022",
      strict: true,
      skipLibCheck: false,
      verbatimModuleSyntax: true,
      noEmit: true,
      jsx: "react-jsx"
    },
    include: ["index.ts"]
  }, null, 2)}\n`);
  writeFileSync(resolve(consumer, "index.ts"), `
import { assertPdsIxPresentation, pdsIxPresentationSchema } from "@appfw/pds-ix-presentation-contract";
import { getPdsIxRecipe, pdsIxRecipeRegistrationSchema } from "@appfw/pds-ix-presentation-contract/recipes";
import { pdsIxPresentationSchema as webSchema } from "@appfw/pds-health-components/intelligence-presentation-model";
import { resolvePdsIxWebRecipe } from "@appfw/pds-health-components/ix-recipes";
import type { PdsIxPresentationProps } from "@appfw/pds-health-native";
import { pdsNativeTokensFor } from "@appfw/pds-health-native/design-data";
import {
  PdsIxRecipePresentation,
  resolvePdsIxNativeRecipe,
  type PdsIxRecipePresentationProps
} from "@appfw/pds-health-native/ix-recipes";

const presentation = assertPdsIxPresentation({
  identity: { schemaVersion: pdsIxPresentationSchema, presentationId: "detached", revision: 1 },
  announcement: "Detached archive proof ready.",
  response: {
    eyebrow: "Proof",
    title: "Detached consumer",
    announcement: "Detached archive proof ready.",
    active: false,
    regions: [],
    emptyState: "No regions required."
  }
});
const props: PdsIxPresentationProps = { presentation };
const recipe = getPdsIxRecipe("analyze-why");
if (!recipe) throw new Error("canonical recipe missing");
const registration = {
  schemaVersion: pdsIxRecipeRegistrationSchema,
  recipeId: recipe.id,
  intentKey: recipe.intentKey,
  artifactType: "detached.product.analysis@1",
  contentSchemaVersion: recipe.presentationSchemaVersion,
  rendererKey: recipe.rendererKey,
  requiredCapabilities: recipe.requiredCapabilities
};
const webRecipe = resolvePdsIxWebRecipe(registration);
const nativeRecipe = resolvePdsIxNativeRecipe(registration, "native-ios");
const recipeProps: PdsIxRecipePresentationProps = {
  registration,
  presentation,
  projection: "native-ios"
};
const recipeComponent: typeof PdsIxRecipePresentation = PdsIxRecipePresentation;
const action = pdsNativeTokensFor({ visualTheme: "apple-like", colorScheme: "dark" }).color.action;
if (webSchema !== pdsIxPresentationSchema || !props.presentation || !recipeProps.presentation || !recipeComponent || !action || webRecipe.recipe !== recipe || nativeRecipe.readiness !== "not-qualified") throw new Error("public contract mismatch");
`);
  writeFileSync(resolve(consumer, "runtime-proof.mjs"), `
import { getPdsIxRecipe, pdsIxPresentationSchema as contractSchema, pdsIxRecipeIds, pdsIxRecipeRegistrationSchema } from "@appfw/pds-ix-presentation-contract";
import { pdsIxPresentationSchema as webSchema } from "@appfw/pds-health-components/intelligence-presentation-model";
import { resolvePdsIxWebRecipe } from "@appfw/pds-health-components/ix-recipes";
import { pdsNativeDesignData, pdsNativeTokensFor } from "@appfw/pds-health-native/design-data";
import { resolvePdsIxNativeRecipe } from "@appfw/pds-health-native/ix-recipe-projection";
if (contractSchema !== webSchema) throw new Error("web adapter contract mismatch");
if (pdsIxRecipeIds.length !== 8 || !Object.isFrozen(pdsIxRecipeIds)) throw new Error("recipe registry mismatch");
if (pdsNativeDesignData.schemaVersion !== "pds.native.design-data@1") throw new Error("native design-data mismatch");
if (!pdsNativeTokensFor({ visualTheme: "apple-like", colorScheme: "light" }).color.action) throw new Error("native selector failed");
const recipe = getPdsIxRecipe("analyze-why");
if (!recipe) throw new Error("canonical recipe missing at runtime");
const registration = {
  schemaVersion: pdsIxRecipeRegistrationSchema,
  recipeId: recipe.id,
  intentKey: recipe.intentKey,
  artifactType: "detached.runtime.analysis@1",
  contentSchemaVersion: recipe.presentationSchemaVersion,
  rendererKey: recipe.rendererKey,
  requiredCapabilities: recipe.requiredCapabilities
};
if (resolvePdsIxWebRecipe(registration).recipe !== recipe) throw new Error("packed Web recipe resolver failed");
if (resolvePdsIxNativeRecipe(registration, "native-ios").recipe !== recipe) throw new Error("packed native recipe resolver failed");
`);

  const installEnvironment = { ...process.env };
  if (process.env.PDS_DETACHED_NPM_CACHE) {
    installEnvironment.npm_config_cache = process.env.PDS_DETACHED_NPM_CACHE;
  }
  run("npm", [
    "install",
    "--offline",
    "--ignore-scripts",
    "--no-audit",
    "--no-fund",
    "--no-save",
    "--package-lock=false",
    contractArchive,
    webArchive,
    nativeArchive
  ], { cwd: consumer, env: installEnvironment });

  run("npm", ["ls", "--all"], { cwd: consumer, env: installEnvironment });
  run(resolve(consumer, "node_modules/.bin/tsc"), ["--project", "tsconfig.json"], { cwd: consumer });
  run(process.execPath, ["runtime-proof.mjs"], { cwd: consumer });

  for (const packageName of ["@appfw/pds-health-components", "@appfw/pds-health-native"]) {
    const packageRoot = resolve(consumer, "node_modules", packageName);
    for (const path of walk(packageRoot)) {
      if (!/\.(?:js|d\.ts|json)$/.test(path)) continue;
      const content = readFileSync(path, "utf8");
      assert.doesNotMatch(content, /file:\.\.\/|\.\.\/(?:tokens|ix-presentation-contract)|\/src\/|\bexpo\b/i, path);
    }
  }

  process.stdout.write(`${JSON.stringify({
    ok: true,
    installMode: "npm-install-offline",
    toolchain: {
      node: process.version,
      npm: run("npm", ["--version"]).trim(),
      typescript: run(resolve(consumer, "node_modules/.bin/tsc"), ["--version"], { cwd: consumer }).trim()
    },
    peerResolution: { mode: "default", validation: "npm-ls-all" },
    archives: archivePaths.map((path) => path.split("/").at(-1)),
    checks: ["three-archive-install", "public-imports", "ix-recipes-component-and-props-typecheck", "consumer-typecheck-nodenext-strict-libcheck", "runtime-imports", "executed-web-native-recipe-resolvers", "peer-resolution", "no-adjacent-source", "no-expo"]
  }, null, 2)}\n`);
} finally {
  rmSync(scratch, { recursive: true, force: true });
}
