#!/usr/bin/env node

import assert from "node:assert/strict";
import { execFileSync } from "node:child_process";
import { createHash } from "node:crypto";
import {
  accessSync,
  constants,
  cpSync,
  existsSync,
  mkdtempSync,
  mkdirSync,
  readFileSync,
  readdirSync,
  realpathSync,
  rmSync,
  statSync,
  writeFileSync
} from "node:fs";
import { tmpdir } from "node:os";
import { basename, dirname, join, relative, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const pdsRoot = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const repoRoot = resolve(pdsRoot, "../../");

function parseArgs(argv) {
  const result = {
    cacheMode: "populate-then-offline",
    cacheParent: "target/appfw/npm-cache",
    cachePrefix: "ix-clean-proof-",
    cacheEvidence: "target/appfw/ix-clean-package-cache.json",
    json: false
  };
  for (let index = 0; index < argv.length; index += 1) {
    if (argv[index] === "--cache-mode") result.cacheMode = argv[++index];
    else if (argv[index] === "--cache-parent") result.cacheParent = argv[++index];
    else if (argv[index] === "--cache-prefix") result.cachePrefix = argv[++index];
    else if (argv[index] === "--cache-evidence") result.cacheEvidence = argv[++index];
    else if (argv[index] === "--json") result.json = true;
    else throw new Error(`unknown argument: ${argv[index]}`);
  }
  if (result.cacheMode !== "populate-then-offline") {
    throw new Error("--cache-mode must be populate-then-offline");
  }
  if (!/^[a-z0-9][a-z0-9._-]{0,48}-$/u.test(result.cachePrefix)) {
    throw new Error("--cache-prefix must be a bounded lowercase mkdtemp prefix ending in '-'");
  }
  return result;
}

const options = parseArgs(process.argv.slice(2));
const cacheParent = resolve(repoRoot, options.cacheParent);
const cacheEvidencePath = resolve(repoRoot, options.cacheEvidence);
for (const [label, candidate] of [
  ["cache parent", cacheParent],
  ["cache evidence", cacheEvidencePath]
]) {
  if (!candidate.startsWith(`${repoRoot}/`)) throw new Error(`${label} must remain inside repository target evidence`);
}
mkdirSync(cacheParent, { recursive: true });
if (!realpathSync(cacheParent).startsWith(`${realpathSync(repoRoot)}/`)) {
  throw new Error("cache parent resolves outside the repository");
}
accessSync(cacheParent, constants.W_OK);
const npmCache = mkdtempSync(join(cacheParent, options.cachePrefix));
const scratch = mkdtempSync(join(tmpdir(), "pds-native-clean-checkout-proof-"));
const checkout = resolve(scratch, "checkout");
const cleanPdsRoot = resolve(checkout, "appfw_ui/pds_health");
const stageRoot = resolve(scratch, "stage");
const archives = resolve(scratch, "archives");
const consumer = resolve(scratch, "consumer");
const commandReceipts = [];
let proofResult = null;
let proofError = null;
let cacheCleanup = { attempted: false, ok: false };

function sha256(bytes) {
  return createHash("sha256").update(bytes).digest("hex");
}

function sanitize(value) {
  return String(value)
    .replaceAll(scratch, "<scratch>")
    .replaceAll(npmCache, "<fresh-cache>")
    .replaceAll(repoRoot, "<repo>");
}

function run(command, args, options = {}) {
  return execFileSync(command, args, {
    encoding: "utf8",
    stdio: ["ignore", "pipe", "pipe"],
    ...options
  });
}

function npm(args, cwd) {
  const startedAt = Date.now();
  try {
    const output = run("npm", args, {
      cwd,
      env: { ...process.env, npm_config_cache: npmCache }
    });
    commandReceipts.push({
      phase: args.includes("--offline") ? "offline" : "populate_or_verify",
      command: ["npm", ...args].map(sanitize),
      cwd: sanitize(cwd),
      ok: true,
      duration_ms: Date.now() - startedAt,
      output_sha256: sha256(output)
    });
    return output;
  } catch (error) {
    commandReceipts.push({
      phase: args.includes("--offline") ? "offline" : "populate_or_verify",
      command: ["npm", ...args].map(sanitize),
      cwd: sanitize(cwd),
      ok: false,
      duration_ms: Date.now() - startedAt
    });
    throw error;
  }
}

function walk(root) {
  return readdirSync(root).flatMap((name) => {
    const path = join(root, name);
    return statSync(path).isDirectory() ? walk(path) : [path];
  });
}

function stagePackage(packageRoot, targetRoot, localContractManifest) {
  cpSync(packageRoot, targetRoot, {
    recursive: true,
    filter(path) {
      return !["node_modules", "dist"].includes(path.split("/").at(-1));
    }
  });
  const manifestPath = resolve(targetRoot, "package.json");
  const manifest = JSON.parse(readFileSync(manifestPath, "utf8"));
  const portableManifest = structuredClone(manifest);
  if (localContractManifest) {
    const localSpecifier = "file:../ix-presentation-contract";
    manifest.devDependencies = {
      ...manifest.devDependencies,
      "@appfw/pds-ix-presentation-contract": localSpecifier
    };
    writeFileSync(manifestPath, `${JSON.stringify(manifest, null, 2)}\n`);
    const lockPath = resolve(targetRoot, "package-lock.json");
    const lock = JSON.parse(readFileSync(lockPath, "utf8"));
    lock.packages[""].devDependencies = {
      ...lock.packages[""].devDependencies,
      "@appfw/pds-ix-presentation-contract": localSpecifier
    };
    lock.packages["../ix-presentation-contract"] = {
      name: localContractManifest.name,
      version: localContractManifest.version,
      dev: true,
      devDependencies: localContractManifest.devDependencies
    };
    lock.packages["node_modules/@appfw/pds-ix-presentation-contract"] = {
      resolved: "../ix-presentation-contract",
      link: true
    };
    writeFileSync(lockPath, `${JSON.stringify(lock, null, 2)}\n`);
  }
  return portableManifest;
}

function restorePortableManifest(targetRoot, portableManifest) {
  writeFileSync(resolve(targetRoot, "package.json"), `${JSON.stringify(portableManifest, null, 2)}\n`);
}

function pack(packageRoot) {
  const output = JSON.parse(npm(["pack", "--pack-destination", archives, "--json"], packageRoot));
  assert.equal(output.length, 1);
  return resolve(archives, output[0].filename);
}

function assertPackedManifest(archivePath, expected) {
  const packed = JSON.parse(run("tar", ["-xOf", archivePath, "package/package.json"]));
  assert.deepEqual(packed, expected);
  assert.doesNotMatch(JSON.stringify(packed), /file:\.\.\/|\bexpo\b/i);
}

try {
  assert.equal(
    run("git", ["status", "--porcelain=v1", "--untracked-files=all"], { cwd: repoRoot }).trim(),
    "",
    "clean-checkout proof requires an exact clean source SHA"
  );
  mkdirSync(checkout);
  const archivePath = resolve(scratch, "source.tar");
  execFileSync("git", ["archive", "--format=tar", "--output", archivePath, "HEAD"], {
    cwd: repoRoot,
    stdio: ["ignore", "pipe", "pipe"]
  });
  execFileSync("tar", ["-xf", archivePath, "-C", checkout], { stdio: ["ignore", "pipe", "pipe"] });
  assert.equal(existsSync(resolve(checkout, "node_modules")), false);
  assert.equal(existsSync(resolve(cleanPdsRoot, "native-components/node_modules")), false);

  mkdirSync(stageRoot);
  mkdirSync(archives);
  mkdirSync(consumer);
  const contractStage = resolve(stageRoot, "ix-presentation-contract");
  const webStage = resolve(stageRoot, "components");
  const nativeStage = resolve(stageRoot, "native-components");
  cpSync(resolve(cleanPdsRoot, "tokens"), resolve(stageRoot, "tokens"), { recursive: true });
  mkdirSync(resolve(stageRoot, "reference"));
  cpSync(
    resolve(cleanPdsRoot, "reference/catalog.json"),
    resolve(stageRoot, "reference/catalog.json")
  );
  assert.equal(
    existsSync(resolve(stageRoot, "reference/catalog.json")),
    true,
    "clean Web contract stage must retain the static lifecycle catalog parity input"
  );
  const cleanContractManifest = stagePackage(resolve(cleanPdsRoot, "ix-presentation-contract"), contractStage);
  const cleanWebManifest = stagePackage(resolve(cleanPdsRoot, "components"), webStage, cleanContractManifest);
  const cleanNativeManifest = stagePackage(resolve(cleanPdsRoot, "native-components"), nativeStage, cleanContractManifest);

  const lockDigests = Object.fromEntries([
    "ix-presentation-contract/package-lock.json",
    "components/package-lock.json",
    "native-components/package-lock.json"
  ].map((lockPath) => [lockPath, sha256(readFileSync(resolve(cleanPdsRoot, lockPath)))]));
  const registryUrl = new URL(npm(["config", "get", "registry"], stageRoot).trim());
  if (registryUrl.username || registryUrl.password || registryUrl.search || registryUrl.hash) {
    throw new Error("npm registry evidence must not contain credentials or query material");
  }

  assert.deepEqual(readdirSync(npmCache), [], "clean proof cache must begin empty");
  for (const packageRoot of [contractStage, webStage, nativeStage]) {
    npm(["ci", "--ignore-scripts", "--no-audit", "--no-fund"], packageRoot);
    rmSync(resolve(packageRoot, "node_modules"), { recursive: true, force: true });
  }
  const populatedCacheFileCount = walk(npmCache).length;
  assert.ok(populatedCacheFileCount > 0, "online populate phase must receipt a non-empty cache");
  const cacheVerifyOutput = npm(["cache", "verify"], stageRoot);

  for (const [lockPath, before] of Object.entries(lockDigests)) {
    assert.equal(
      sha256(readFileSync(resolve(cleanPdsRoot, lockPath))),
      before,
      `${lockPath} changed during cache population`
    );
  }

  for (const packageRoot of [contractStage, webStage, nativeStage]) {
    assert.equal(existsSync(resolve(packageRoot, "node_modules")), false);
    npm(["ci", "--offline", "--ignore-scripts", "--no-audit", "--no-fund"], packageRoot);
  }
  npm(["run", "test"], contractStage);
  npm(["run", "test:contracts"], webStage);
  npm(["run", "test"], nativeStage);

  restorePortableManifest(contractStage, cleanContractManifest);
  restorePortableManifest(webStage, cleanWebManifest);
  restorePortableManifest(nativeStage, cleanNativeManifest);

  for (const path of walk(resolve(nativeStage, "dist"))) {
    if (!/\.(?:js|d\.ts)$/.test(path)) continue;
    const content = readFileSync(path, "utf8");
    assert.doesNotMatch(content, /file:\.\.\/|\.\.\/(?:tokens|ix-presentation-contract)|\/src\/|\bexpo\b/i, path);
  }

  const contractArchive = pack(contractStage);
  const webArchive = pack(webStage);
  const nativeArchive = pack(nativeStage);
  assertPackedManifest(contractArchive, cleanContractManifest);
  assertPackedManifest(webArchive, cleanWebManifest);
  assertPackedManifest(nativeArchive, cleanNativeManifest);

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
import { resolvePdsIxNativeRecipe } from "@appfw/pds-health-native/ix-recipes";
const presentation = assertPdsIxPresentation({
  identity: { schemaVersion: pdsIxPresentationSchema, presentationId: "clean-proof", revision: 1 },
  announcement: "Clean checkout proof ready.",
  response: { eyebrow: "Proof", title: "Clean checkout", announcement: "Clean checkout proof ready.", active: false, regions: [], emptyState: "No regions required." }
});
const props: PdsIxPresentationProps = { presentation };
const recipe = getPdsIxRecipe("ambient-agent-continuity");
if (!recipe) throw new Error("canonical recipe missing");
const registration = {
  schemaVersion: pdsIxRecipeRegistrationSchema,
  recipeId: recipe.id,
  intentKey: recipe.intentKey,
  artifactType: "clean.product.agent-continuity@1",
  contentSchemaVersion: recipe.presentationSchemaVersion,
  rendererKey: recipe.rendererKey,
  requiredCapabilities: recipe.requiredCapabilities
};
if (webSchema !== pdsIxPresentationSchema || !props.presentation || !pdsNativeTokensFor({ visualTheme: "apple-like", colorScheme: "dark" }).color.action || resolvePdsIxWebRecipe(registration).recipe !== recipe || resolvePdsIxNativeRecipe(registration, "native-android").readiness !== "not-qualified") throw new Error("public contract mismatch");
`);
  writeFileSync(resolve(consumer, "runtime-proof.mjs"), `
import { getPdsIxRecipe, pdsIxPresentationSchema as contractSchema, pdsIxRecipeIds, pdsIxRecipeRegistrationSchema } from "@appfw/pds-ix-presentation-contract";
import { pdsIxPresentationSchema as webSchema } from "@appfw/pds-health-components/intelligence-presentation-model";
import { resolvePdsIxWebRecipe } from "@appfw/pds-health-components/ix-recipes";
import { pdsNativeDesignData } from "@appfw/pds-health-native/design-data";
import { resolvePdsIxNativeRecipe } from "@appfw/pds-health-native/ix-recipe-projection";
if (contractSchema !== webSchema || pdsIxRecipeIds.length !== 8 || !Object.isFrozen(pdsIxRecipeIds) || pdsNativeDesignData.schemaVersion !== "pds.native.design-data@1") throw new Error("runtime contract mismatch");
const recipe = getPdsIxRecipe("ambient-agent-continuity");
if (!recipe) throw new Error("canonical recipe missing at runtime");
const registration = {
  schemaVersion: pdsIxRecipeRegistrationSchema,
  recipeId: recipe.id,
  intentKey: recipe.intentKey,
  artifactType: "clean.runtime.agent-continuity@1",
  contentSchemaVersion: recipe.presentationSchemaVersion,
  rendererKey: recipe.rendererKey,
  requiredCapabilities: recipe.requiredCapabilities
};
if (resolvePdsIxWebRecipe(registration).recipe !== recipe) throw new Error("packed Web recipe resolver failed");
if (resolvePdsIxNativeRecipe(registration, "native-android").recipe !== recipe) throw new Error("packed native recipe resolver failed");
`);
  npm([
    "install", "--offline", "--ignore-scripts", "--no-audit", "--no-fund", "--no-save", "--package-lock=false",
    contractArchive, webArchive, nativeArchive
  ], consumer);
  npm(["ls", "--all"], consumer);
  run(resolve(consumer, "node_modules/.bin/tsc"), ["--project", "tsconfig.json"], { cwd: consumer });
  run(process.execPath, ["runtime-proof.mjs"], { cwd: consumer });
  for (const packageName of ["@appfw/pds-health-components", "@appfw/pds-health-native"]) {
    for (const path of walk(resolve(consumer, "node_modules", packageName))) {
      if (!/\.(?:js|d\.ts|json)$/.test(path)) continue;
      assert.doesNotMatch(readFileSync(path, "utf8"), /file:\.\.\/|\.\.\/(?:tokens|ix-presentation-contract)|\/src\/|\bexpo\b/i, path);
    }
  }

  proofResult = {
    ok: true,
    sourceSha: run("git", ["rev-parse", "HEAD"], { cwd: repoRoot }).trim(),
    sourceTree: run("git", ["rev-parse", "HEAD^{tree}"], { cwd: repoRoot }).trim(),
    sourceMode: "fresh-git-archive-no-node-modules",
    installMode: "fresh-cache-populate-then-npm-install-offline-default-peer-resolution",
    cacheReceipt: {
      initialState: "empty",
      writable: true,
      populatedFileCount: populatedCacheFileCount,
      offlinePhase: true,
      cacheMode: options.cacheMode,
      cacheChild: basename(npmCache),
      registry: `${registryUrl.origin}${registryUrl.pathname}`,
      lockDigests,
      cacheVerifySha256: sha256(cacheVerifyOutput)
    },
    manifestMode: "all-packed-manifests-unchanged",
    toolchain: {
      node: process.version,
      npm: npm(["--version"], consumer).trim(),
      typescript: run(resolve(consumer, "node_modules/.bin/tsc"), ["--version"], { cwd: consumer }).trim()
    },
    checks: ["clean-source", "fresh-writable-cache", "populate-then-offline", "exact-lock-offline-ci", "staged-local-development-dependencies", "unchanged-packed-manifest", "three-archive-install", "strict-nodenext", "public-runtime-imports", "executed-web-native-recipe-resolvers", "default-peer-resolution", "no-source-or-expo-leakage"]
  };
} catch (error) {
  proofError = error;
} finally {
  rmSync(scratch, { recursive: true, force: true });
  cacheCleanup.attempted = true;
  try {
    rmSync(npmCache, { recursive: true, force: true });
    cacheCleanup.ok = !existsSync(npmCache);
  } catch (error) {
    cacheCleanup = { attempted: true, ok: false, error: sanitize(error.message) };
  }

  mkdirSync(dirname(cacheEvidencePath), { recursive: true });
  writeFileSync(cacheEvidencePath, `${JSON.stringify({
    schema: "appfw.ix_clean_package_cache_receipt@1",
    ok: proofError === null && proofResult?.ok === true && cacheCleanup.ok,
    generatedAt: new Date().toISOString(),
    cacheMode: options.cacheMode,
    cacheParent: relative(repoRoot, cacheParent),
    cachePrefix: options.cachePrefix,
    cacheChild: basename(npmCache),
    cacheCleanup,
    commands: commandReceipts,
    proof: proofResult,
    error: proofError ? sanitize(proofError.message) : null,
    nonclaims: [
      "Population followed by offline proof does not claim an initially populated runner cache.",
      "Local archives do not establish publication, native-device qualification, release, or production readiness."
    ]
  }, null, 2)}\n`);
}

if (proofError) throw proofError;
if (!cacheCleanup.ok) throw new Error("fresh npm cache cleanup failed");
process.stdout.write(`${JSON.stringify({
  ...proofResult,
  cacheEvidence: relative(repoRoot, cacheEvidencePath),
  cacheCleanup
}, null, options.json ? 2 : 0)}\n`);
