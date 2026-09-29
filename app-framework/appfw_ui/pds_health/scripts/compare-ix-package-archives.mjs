#!/usr/bin/env node

import assert from "node:assert/strict";
import { execFileSync } from "node:child_process";
import { createHash } from "node:crypto";
import {
  cpSync,
  mkdtempSync,
  mkdirSync,
  readFileSync,
  readdirSync,
  rmSync,
  statSync,
  writeFileSync
} from "node:fs";
import { tmpdir } from "node:os";
import path from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";

export const frozenBaseline = Object.freeze({
  "@appfw/pds-ix-presentation-contract": Object.freeze({
    version: "0.2.0",
    bytes: 8498,
    sha256: "f1e4a187507eb39f1f115967fbbc0bb829902a510acad036ec7e42e28c76dea2"
  }),
  "@appfw/pds-health-components": Object.freeze({
    version: "0.12.0",
    bytes: 139535,
    sha256: "0f0029b7c45c492ffe3a2bae76dd34837a8c3f21b67ca93500b68c024bffa597"
  }),
  "@appfw/pds-health-native": Object.freeze({
    version: "0.2.0",
    bytes: 9398,
    sha256: "5f8fc29df15ec879956669ff94750e0799dc9f31584911112b2c57611c70f06e"
  })
});

const requiredToolchain = Object.freeze({
  node: "v24.19.0",
  npm: "10.8.2",
  typescript: "Version 6.0.3"
});
const packageDirectories = Object.freeze([
  ["@appfw/pds-ix-presentation-contract", "ix-presentation-contract"],
  ["@appfw/pds-health-components", "components"],
  ["@appfw/pds-health-native", "native-components"]
]);

function digest(algorithm, bytes, encoding = "hex") {
  return createHash(algorithm).update(bytes).digest(encoding);
}

function run(command, args, options = {}) {
  return execFileSync(command, args, {
    encoding: "utf8",
    stdio: ["ignore", "pipe", "pipe"],
    ...options
  });
}

function walk(root, base = root) {
  return readdirSync(root).flatMap((name) => {
    const absolute = path.join(root, name);
    const details = statSync(absolute);
    return details.isDirectory() ? walk(absolute, base) : [{
      absolute,
      path: path.relative(base, absolute).split(path.sep).join("/")
    }];
  });
}

function stagePackage(sourceRoot, targetRoot, contractManifest = null) {
  cpSync(sourceRoot, targetRoot, {
    recursive: true,
    filter(candidate) {
      return !["node_modules", "dist"].includes(path.basename(candidate));
    }
  });
  const manifestPath = path.join(targetRoot, "package.json");
  const portable = JSON.parse(readFileSync(manifestPath, "utf8"));
  if (contractManifest) {
    const local = "file:../ix-presentation-contract";
    const staged = structuredClone(portable);
    staged.devDependencies = {
      ...staged.devDependencies,
      "@appfw/pds-ix-presentation-contract": local
    };
    writeFileSync(manifestPath, `${JSON.stringify(staged, null, 2)}\n`);
    const lockPath = path.join(targetRoot, "package-lock.json");
    const lock = JSON.parse(readFileSync(lockPath, "utf8"));
    lock.packages[""].devDependencies = {
      ...lock.packages[""].devDependencies,
      "@appfw/pds-ix-presentation-contract": local
    };
    lock.packages["../ix-presentation-contract"] = {
      name: contractManifest.name,
      version: contractManifest.version,
      dev: true,
      devDependencies: contractManifest.devDependencies
    };
    lock.packages["node_modules/@appfw/pds-ix-presentation-contract"] = {
      resolved: "../ix-presentation-contract",
      link: true
    };
    writeFileSync(lockPath, `${JSON.stringify(lock, null, 2)}\n`);
  }
  return portable;
}

function inventoryArchive(archivePath, extractionRoot) {
  mkdirSync(extractionRoot, { recursive: true });
  run("tar", ["-xzf", archivePath, "-C", extractionRoot]);
  const packageRoot = path.join(extractionRoot, "package");
  return walk(packageRoot).map(({ absolute, path: relativePath }) => {
    const bytes = readFileSync(absolute);
    return { path: relativePath, bytes: bytes.length, sha256: digest("sha256", bytes) };
  }).sort((left, right) => left.path.localeCompare(right.path));
}

function pack(packageRoot, archiveRoot, extractionRoot, npmCache) {
  const output = JSON.parse(run("npm", [
    "pack", "--pack-destination", archiveRoot, "--json", "--ignore-scripts"
  ], { cwd: packageRoot, env: { ...process.env, npm_config_cache: npmCache } }));
  assert.equal(output.length, 1);
  const archivePath = path.join(archiveRoot, output[0].filename);
  const bytes = readFileSync(archivePath);
  const manifest = JSON.parse(run("tar", ["-xOf", archivePath, "package/package.json"]));
  return {
    name: manifest.name,
    version: manifest.version,
    archive_path: archivePath,
    archive_filename: output[0].filename,
    raw_bytes: bytes.length,
    sha256: digest("sha256", bytes),
    sha512: digest("sha512", bytes),
    integrity: `sha512-${digest("sha512", bytes, "base64")}`,
    inventory: inventoryArchive(archivePath, extractionRoot),
    packed_manifest: manifest
  };
}

function proveDetachedConsumer(packages, workRoot, npmCache) {
  const consumer = path.join(workRoot, "consumer");
  mkdirSync(consumer);
  writeFileSync(path.join(consumer, "package.json"), `${JSON.stringify({
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
  writeFileSync(path.join(consumer, "tsconfig.json"), `${JSON.stringify({
    compilerOptions: {
      module: "NodeNext",
      moduleResolution: "NodeNext",
      target: "ES2022",
      strict: true,
      skipLibCheck: false,
      noEmit: true,
      jsx: "react-jsx"
    },
    include: ["index.ts"]
  }, null, 2)}\n`);
  writeFileSync(path.join(consumer, "index.ts"), `
import { getPdsIxRecipe, pdsIxPresentationSchema, pdsIxRecipeRegistrationSchema } from "@appfw/pds-ix-presentation-contract";
import { pdsIxPresentationSchema as webSchema } from "@appfw/pds-health-components/intelligence-presentation-model";
import { resolvePdsIxWebRecipe } from "@appfw/pds-health-components/ix-recipes";
import { pdsNativeTokensFor } from "@appfw/pds-health-native/design-data";
import { resolvePdsIxNativeRecipe } from "@appfw/pds-health-native/ix-recipes";
const recipe = getPdsIxRecipe("analyze-why");
if (!recipe) throw new Error("recipe missing");
const registration = { schemaVersion: pdsIxRecipeRegistrationSchema, recipeId: recipe.id, intentKey: recipe.intentKey, artifactType: "detached.analysis@1", contentSchemaVersion: recipe.presentationSchemaVersion, rendererKey: recipe.rendererKey, requiredCapabilities: recipe.requiredCapabilities };
if (webSchema !== pdsIxPresentationSchema || resolvePdsIxWebRecipe(registration).recipe !== recipe || resolvePdsIxNativeRecipe(registration, "native-ios").readiness !== "not-qualified" || !pdsNativeTokensFor({ visualTheme: "apple-like", colorScheme: "dark" }).color.action) throw new Error("detached contract mismatch");
`);
  writeFileSync(path.join(consumer, "runtime-proof.mjs"), `
import { getPdsIxRecipe, pdsIxRecipeIds, pdsIxRecipeRegistrationSchema } from "@appfw/pds-ix-presentation-contract";
import { resolvePdsIxWebRecipe } from "@appfw/pds-health-components/ix-recipes";
import { resolvePdsIxNativeRecipe } from "@appfw/pds-health-native/ix-recipe-projection";
if (pdsIxRecipeIds.length !== 8 || !Object.isFrozen(pdsIxRecipeIds)) throw new Error("registry mismatch");
const recipe = getPdsIxRecipe("analyze-why");
const registration = { schemaVersion: pdsIxRecipeRegistrationSchema, recipeId: recipe.id, intentKey: recipe.intentKey, artifactType: "detached.runtime@1", contentSchemaVersion: recipe.presentationSchemaVersion, rendererKey: recipe.rendererKey, requiredCapabilities: recipe.requiredCapabilities };
if (resolvePdsIxWebRecipe(registration).recipe !== recipe || resolvePdsIxNativeRecipe(registration, "native-android").readiness !== "not-qualified") throw new Error("resolver mismatch");
`);
  run("npm", [
    "install", "--offline", "--ignore-scripts", "--no-audit", "--no-fund",
    "--no-save", "--package-lock=false", ...packages.map(({ archive_path: archive }) => archive)
  ], { cwd: consumer, env: { ...process.env, npm_config_cache: npmCache } });
  run("npm", ["ls", "--all"], { cwd: consumer, env: { ...process.env, npm_config_cache: npmCache } });
  run(path.join(consumer, "node_modules/.bin/tsc"), ["--project", "tsconfig.json"], { cwd: consumer });
  run(process.execPath, ["runtime-proof.mjs"], { cwd: consumer });
  return {
    ok: true,
    install_mode: "archive-only-offline",
    adjacent_source: false,
    recipe_count: 8,
    native_readiness: "not-qualified"
  };
}

function buildRef(repoRoot, ref, label, npmCache, detached) {
  const commit = run("git", ["rev-parse", `${ref}^{commit}`], { cwd: repoRoot }).trim();
  const tree = run("git", ["rev-parse", `${ref}^{tree}`], { cwd: repoRoot }).trim();
  const root = mkdtempSync(path.join(tmpdir(), `appfw-ix-archive-${label}-`));
  try {
    const checkout = path.join(root, "checkout");
    const stage = path.join(root, "stage");
    const archiveRoot = path.join(root, "archives");
    mkdirSync(checkout);
    mkdirSync(stage);
    mkdirSync(archiveRoot);
    const sourceArchive = path.join(root, "source.tar");
    execFileSync("git", ["archive", "--format=tar", "--output", sourceArchive, commit], {
      cwd: repoRoot,
      stdio: ["ignore", "pipe", "pipe"]
    });
    run("tar", ["-xf", sourceArchive, "-C", checkout]);
    const pdsRoot = path.join(checkout, "appfw_ui/pds_health");
    const contractStage = path.join(stage, "ix-presentation-contract");
    const webStage = path.join(stage, "components");
    const nativeStage = path.join(stage, "native-components");
    cpSync(path.join(pdsRoot, "tokens"), path.join(stage, "tokens"), { recursive: true });
    mkdirSync(path.join(stage, "reference"));
    cpSync(path.join(pdsRoot, "reference/catalog.json"), path.join(stage, "reference/catalog.json"));
    const contractManifest = stagePackage(path.join(pdsRoot, "ix-presentation-contract"), contractStage);
    const webManifest = stagePackage(path.join(pdsRoot, "components"), webStage, contractManifest);
    const nativeManifest = stagePackage(path.join(pdsRoot, "native-components"), nativeStage, contractManifest);
    const lock_digests = Object.fromEntries(packageDirectories.map(([, directory]) => {
      const lock = readFileSync(path.join(pdsRoot, directory, "package-lock.json"));
      return [directory, digest("sha256", lock)];
    }));
    for (const packageRoot of [contractStage, webStage, nativeStage]) {
      run("npm", ["ci", "--ignore-scripts", "--no-audit", "--no-fund"], {
        cwd: packageRoot,
        env: { ...process.env, npm_config_cache: npmCache }
      });
    }
    run("npm", ["run", "build"], { cwd: webStage, env: { ...process.env, npm_config_cache: npmCache } });
    run("npm", ["run", "build"], { cwd: nativeStage, env: { ...process.env, npm_config_cache: npmCache } });
    assert.equal(
      run(path.join(contractStage, "node_modules/.bin/tsc"), ["--version"]).trim(),
      requiredToolchain.typescript
    );
    for (const [packageRoot, manifest] of [
      [contractStage, contractManifest],
      [webStage, webManifest],
      [nativeStage, nativeManifest]
    ]) {
      writeFileSync(path.join(packageRoot, "package.json"), `${JSON.stringify(manifest, null, 2)}\n`);
    }
    const packages = [contractStage, webStage, nativeStage].map((packageRoot, index) => (
      pack(packageRoot, archiveRoot, path.join(root, `extract-${index}`), npmCache)
    ));
    const detached_consumer = detached ? proveDetachedConsumer(packages, root, npmCache) : null;
    return {
      commit,
      tree,
      lock_digests,
      packages: packages.map(({ archive_path, ...item }) => item),
      detached_consumer
    };
  } finally {
    rmSync(root, { recursive: true, force: true });
  }
}

export function assertRepeatable(first, second, label) {
  if (first.commit !== second.commit || first.tree !== second.tree) {
    throw new Error(`${label} repeat source identity changed`);
  }
  for (const firstPackage of first.packages) {
    const secondPackage = second.packages.find(({ name }) => name === firstPackage.name);
    if (!secondPackage || firstPackage.version !== secondPackage.version
      || firstPackage.raw_bytes !== secondPackage.raw_bytes
      || firstPackage.sha256 !== secondPackage.sha256
      || firstPackage.sha512 !== secondPackage.sha512
      || JSON.stringify(firstPackage.inventory) !== JSON.stringify(secondPackage.inventory)) {
      throw new Error(`${label} package ${firstPackage.name} is not byte-repeatable`);
    }
  }
}

export function assertFrozenBaseline(build) {
  for (const [name, expected] of Object.entries(frozenBaseline)) {
    const observed = build.packages.find((item) => item.name === name);
    if (!observed || observed.version !== expected.version
      || observed.raw_bytes !== expected.bytes || observed.sha256 !== expected.sha256) {
      throw new Error(`${name} does not reproduce the frozen I1 archive baseline`);
    }
  }
}

export function classifyInventory(baselinePackage, candidatePackage) {
  const baseline = new Map(baselinePackage.inventory.map((item) => [item.path, item]));
  const candidate = new Map(candidatePackage.inventory.map((item) => [item.path, item]));
  return [...new Set([...baseline.keys(), ...candidate.keys()])].sort().map((relativePath) => {
    const before = baseline.get(relativePath) ?? null;
    const after = candidate.get(relativePath) ?? null;
    const classification = before === null ? "added"
      : after === null ? "removed"
        : before.sha256 === after.sha256 && before.bytes === after.bytes ? "unchanged" : "changed";
    return { path: relativePath, classification, baseline: before, candidate: after };
  });
}

function parseArgs(argv) {
  const result = {
    baseline: "1e919fafe783b68cef4abc65f99d585be482b4e7",
    candidate: "HEAD",
    output: "target/appfw/ix-package-archive-comparison.json",
    json: false
  };
  for (let index = 0; index < argv.length; index += 1) {
    if (argv[index] === "--baseline") result.baseline = argv[++index];
    else if (argv[index] === "--candidate") result.candidate = argv[++index];
    else if (argv[index] === "--output") result.output = argv[++index];
    else if (argv[index] === "--json") result.json = true;
    else throw new Error(`unknown argument: ${argv[index]}`);
  }
  return result;
}

function main() {
  const repoRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "../../../");
  const options = parseArgs(process.argv.slice(2));
  if (options.baseline !== "1e919fafe783b68cef4abc65f99d585be482b4e7") {
    throw new Error("baseline must be the exact reviewed I1 commit");
  }
  if (process.version !== requiredToolchain.node
    || run("npm", ["--version"]).trim() !== requiredToolchain.npm) {
    throw new Error(`toolchain must be Node ${requiredToolchain.node} and npm ${requiredToolchain.npm}`);
  }
  if (options.candidate === "HEAD"
    && run("git", ["status", "--porcelain=v1", "--untracked-files=all"], { cwd: repoRoot }).trim()) {
    throw new Error("candidate HEAD must be an immutable clean checkout");
  }
  const outputPath = path.resolve(repoRoot, options.output);
  if (!outputPath.startsWith(`${repoRoot}${path.sep}`)) throw new Error("output path escapes repository");
  const workRoot = mkdtempSync(path.join(tmpdir(), "appfw-ix-archive-comparison-"));
  try {
    const npmCache = path.join(workRoot, "npm-cache");
    mkdirSync(npmCache);
    const baselineA = buildRef(repoRoot, options.baseline, "baseline-a", npmCache, true);
    const baselineB = buildRef(repoRoot, options.baseline, "baseline-b", npmCache, false);
    assertRepeatable(baselineA, baselineB, "baseline");
    assertFrozenBaseline(baselineA);
    const candidateA = buildRef(repoRoot, options.candidate, "candidate-a", npmCache, true);
    const candidateB = buildRef(repoRoot, options.candidate, "candidate-b", npmCache, false);
    assertRepeatable(candidateA, candidateB, "candidate");
    const package_differences = baselineA.packages.map((baselinePackage) => {
      const candidatePackage = candidateA.packages.find(({ name }) => name === baselinePackage.name);
      if (!candidatePackage || candidatePackage.version !== baselinePackage.version) {
        throw new Error(`${baselinePackage.name} identity changed without compatibility review`);
      }
      return {
        name: baselinePackage.name,
        version: baselinePackage.version,
        archive_bytes_equal: baselinePackage.sha256 === candidatePackage.sha256,
        baseline_sha256: baselinePackage.sha256,
        candidate_sha256: candidatePackage.sha256,
        inventory: classifyInventory(baselinePackage, candidatePackage)
      };
    });
    const result = {
      schema: "appfw.ix_package_archive_comparison@1",
      ok: true,
      toolchain: requiredToolchain,
      baseline: baselineA,
      candidate: candidateA,
      repeatability: { baseline: "byte_equal", candidate: "byte_equal" },
      package_differences,
      nonclaims: [
        "Same-host byte repeatability is not cross-machine reproducibility or publication evidence.",
        "Detached consumers do not establish Nexus acceptance, native-device qualification, release, or production readiness."
      ]
    };
    mkdirSync(path.dirname(outputPath), { recursive: true });
    writeFileSync(outputPath, `${JSON.stringify(result, null, 2)}\n`);
    process.stdout.write(`${JSON.stringify(result, null, options.json ? 2 : 0)}\n`);
  } finally {
    rmSync(workRoot, { recursive: true, force: true });
  }
}

const invokedPath = process.argv[1] ? pathToFileURL(path.resolve(process.argv[1])).href : "";
if (invokedPath === import.meta.url) {
  try {
    main();
  } catch (error) {
    process.stderr.write(`compare-ix-package-archives: ${error.message}\n`);
    process.exitCode = 1;
  }
}
