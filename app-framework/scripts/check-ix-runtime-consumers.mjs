#!/usr/bin/env node

import { execFileSync } from "node:child_process";
import { createHash } from "node:crypto";
import { mkdir, readFile, writeFile } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";

export const expectedCrates = Object.freeze([
  ["appfw-runtime", "0.2.0", "appfw_runtime/Cargo.toml", null],
  ["appfw-cli", "0.1.7", "appfw_cli/Cargo.toml", "0.2.0"],
  ["appfw-provider-mongo", "0.2.0", "appfw_provider_mongo/Cargo.toml", "0.2.0"],
  ["appfw-provider-mssql", "0.2.0", "appfw_provider_mssql/Cargo.toml", "0.2.0"],
  ["appfw-provider-neo4j", "0.2.0", "appfw_provider_neo4j/Cargo.toml", "0.2.0"],
  ["appfw-provider-postgres", "0.2.0", "appfw_provider_postgres/Cargo.toml", "0.2.0"],
  ["appfw-provider-snowflake", "0.2.0", "appfw_provider_snowflake/Cargo.toml", "0.2.0"]
]);

const consumerManifests = Object.freeze([
  ["database", "database/Cargo.toml", "../appfw_runtime"],
  ["crm_backend", "examples/products/crm/backend/Cargo.toml", "../../../../appfw_runtime"],
  ["pds_nexus_backend", "examples/products/pds-nexus/backend/Cargo.toml", "../../../../appfw_runtime"],
  [
    "generated_second_consumer",
    "app_gen/_golden/downstream_apps/second-consumer/starter/backend/Cargo.toml",
    "../../../../appfw_runtime"
  ]
]);

const protectedRootLockSha256 =
  "e65ecfa882ef0997f8710b67e2e578fcbe17e0735d4b2482797baf3178f1ca89";

function sha256(value) {
  return createHash("sha256").update(value).digest("hex");
}

function packageSection(toml) {
  const match = toml.match(/(?:^|\n)\[package\]\s*\n([\s\S]*?)(?=\n\[[^\]]+\]|$)/u);
  if (!match) throw new Error("manifest is missing [package]");
  return match[1];
}

function scalar(section, key) {
  const match = section.match(new RegExp(`^\\s*${key}\\s*=\\s*"([^"]+)"\\s*$`, "mu"));
  return match?.[1] ?? null;
}

export function parseManifest(toml) {
  const section = packageSection(toml);
  const runtimeLines = toml.split("\n").filter((line) => (
    /^\s*appfw_runtime\s*=/u.test(line) && /package\s*=\s*"appfw-runtime"/u.test(line)
  ));
  if (runtimeLines.length > 1) throw new Error("manifest has ambiguous appfw-runtime dependencies");
  const runtimeLine = runtimeLines[0] ?? null;
  return {
    name: scalar(section, "name"),
    version: scalar(section, "version"),
    runtime_dependency_version: runtimeLine?.match(/version\s*=\s*"([^"]+)"/u)?.[1] ?? null,
    runtime_dependency_path: runtimeLine?.match(/path\s*=\s*"([^"]+)"/u)?.[1] ?? null,
    runtime_dependency_registry: runtimeLine?.match(/registry\s*=\s*"([^"]+)"/u)?.[1] ?? null
  };
}

export function parseLockPackages(lockText) {
  const result = [];
  for (const block of lockText.split("[[package]]").slice(1)) {
    const name = block.match(/^\s*name\s*=\s*"([^"]+)"/mu)?.[1];
    const version = block.match(/^\s*version\s*=\s*"([^"]+)"/mu)?.[1];
    if (name && version) result.push({ name, version });
  }
  return result;
}

function versionsFor(packages, name) {
  return packages.filter((item) => item.name === name).map((item) => item.version).sort();
}

function requireSingleVersion(packages, name, expectedVersion, label) {
  const versions = versionsFor(packages, name);
  if (versions.length !== 1 || versions[0] !== expectedVersion) {
    throw new Error(`${label} must contain exactly ${name}@${expectedVersion}; got ${versions.join(",") || "none"}`);
  }
}

export function evaluateRuntimeConsumers(model) {
  const manifests = [];
  for (const [name, version, manifestPath, runtimeVersion] of expectedCrates) {
    const manifest = model.manifests[manifestPath];
    if (!manifest) throw new Error(`missing candidate manifest ${manifestPath}`);
    if (manifest.name !== name || manifest.version !== version) {
      throw new Error(`${manifestPath} must identify ${name}@${version}`);
    }
    if (runtimeVersion !== null) {
      if (manifest.runtime_dependency_version !== runtimeVersion
        || manifest.runtime_dependency_registry !== "pds-app-framework-crates") {
        throw new Error(`${manifestPath} must depend on registry appfw-runtime@${runtimeVersion}`);
      }
    }
    manifests.push({
      name,
      version,
      manifest_path: manifestPath,
      runtime_dependency_version: manifest.runtime_dependency_version,
      runtime_dependency_registry: manifest.runtime_dependency_registry,
      publication_state: "unpublished_candidate"
    });
  }

  for (const oldConstraint of model.oldRuntimeConstraints) {
    throw new Error(`explicit old appfw-runtime constraint remains in ${oldConstraint}`);
  }

  for (const [id, manifestPath, expectedPath] of consumerManifests) {
    const consumer = model.consumers[manifestPath];
    if (!consumer) throw new Error(`missing required path consumer ${manifestPath}`);
    if (consumer.runtime_dependency_path !== expectedPath
      || (consumer.runtime_dependency_version !== null
        && consumer.runtime_dependency_version !== "0.2.0")) {
      throw new Error(`${manifestPath} does not consume the one local appfw-runtime candidate`);
    }
    consumer.id = id;
  }

  for (const [name, version] of expectedCrates.filter(([crate]) => crate !== "appfw-cli")) {
    requireSingleVersion(model.crmLockPackages, name, version, "CRM lock");
  }

  let rootLockState = "aligned";
  let rootLockAligned = true;
  for (const [name, version] of expectedCrates) {
    const versions = versionsFor(model.rootLockPackages, name);
    if (versions.length !== 1 || versions[0] !== version) rootLockAligned = false;
  }
  if (!rootLockAligned) {
    if (model.rootLockSha256 !== protectedRootLockSha256) {
      throw new Error("root Cargo.lock is neither aligned nor the exact protected Integration-pending base");
    }
    rootLockState = "integration_pending_exact_protected_base";
  }

  return {
    ok: true,
    schema: "appfw.ix_runtime_consumers@1",
    candidate_commit: model.candidateCommit,
    manifests,
    runtime_nodes: [{ name: "appfw-runtime", version: "0.2.0", count: 1 }],
    locks: {
      crm: "aligned",
      root: rootLockState,
      root_sha256: model.rootLockSha256
    },
    consumers: Object.entries(model.consumers).map(([manifest_path, value]) => ({
      id: value.id,
      manifest_path,
      runtime_dependency_path: value.runtime_dependency_path,
      executable_proof: "required_by_focused_cargo_or_golden_downstream_gate"
    })),
    known_collision: {
      name: "appfw-provider-mssql",
      version: "0.1.1",
      checksum: "09afb7c4acd00c97fa7cfafe3a234319e681d4c8228e6b30c8892fdbbbfb7572",
      runtime_dependency: "0.1.1",
      disposition: "retained_do_not_reuse"
    },
    publication_state: "unpublished_candidate",
    nonclaims: [
      "No package publication, registry availability, Product acceptance, release, deployment, or production readiness is established.",
      "An Integration-pending root lock is not aggregate locked-build evidence."
    ]
  };
}

export async function inspectRuntimeConsumers(repoRoot) {
  const manifests = {};
  for (const [, , manifestPath] of expectedCrates) {
    manifests[manifestPath] = parseManifest(await readFile(path.join(repoRoot, manifestPath), "utf8"));
  }
  const consumers = {};
  for (const [, manifestPath] of consumerManifests) {
    consumers[manifestPath] = parseManifest(await readFile(path.join(repoRoot, manifestPath), "utf8"));
  }
  const cargoManifestPaths = execFileSync(
    "git", ["ls-files", "-co", "--exclude-standard", "--", "*Cargo.toml"],
    { cwd: repoRoot, encoding: "utf8" }
  ).trim().split("\n").filter(Boolean);
  const oldRuntimeConstraints = [];
  for (const manifestPath of cargoManifestPaths) {
    const contents = await readFile(path.join(repoRoot, manifestPath), "utf8");
    if (/appfw[-_]runtime\s*=\s*\{[^\n}]*version\s*=\s*"0\.1/u.test(contents)
      || /appfw-runtime\s*=\s*"0\.1/u.test(contents)) {
      oldRuntimeConstraints.push(manifestPath);
    }
  }
  const rootLock = await readFile(path.join(repoRoot, "Cargo.lock"));
  const crmLock = await readFile(path.join(repoRoot, "examples/products/crm/Cargo.lock"));
  const candidateCommit = execFileSync("git", ["rev-parse", "HEAD"], {
    cwd: repoRoot,
    encoding: "utf8"
  }).trim();
  return evaluateRuntimeConsumers({
    manifests,
    consumers,
    oldRuntimeConstraints,
    rootLockPackages: parseLockPackages(rootLock.toString("utf8")),
    rootLockSha256: sha256(rootLock),
    crmLockPackages: parseLockPackages(crmLock.toString("utf8")),
    candidateCommit
  });
}

function parseArgs(argv) {
  const options = { output: "target/appfw/ix-runtime-consumers.json", json: false };
  for (let index = 0; index < argv.length; index += 1) {
    if (argv[index] === "--output") options.output = argv[++index];
    else if (argv[index] === "--json") options.json = true;
    else throw new Error(`unknown argument: ${argv[index]}`);
  }
  return options;
}

async function main() {
  const repoRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
  const options = parseArgs(process.argv.slice(2));
  const outputPath = path.resolve(repoRoot, options.output);
  if (!outputPath.startsWith(`${repoRoot}${path.sep}`)) throw new Error("output path escapes repository root");
  const result = await inspectRuntimeConsumers(repoRoot);
  const artifact = {
    ...result,
    observed_at: new Date().toISOString(),
    command: "check-ix-runtime-consumers"
  };
  await mkdir(path.dirname(outputPath), { recursive: true });
  await writeFile(outputPath, `${JSON.stringify(artifact, null, 2)}\n`, "utf8");
  process.stdout.write(`${JSON.stringify(artifact, null, options.json ? 2 : 0)}\n`);
}

const invokedPath = process.argv[1] ? pathToFileURL(path.resolve(process.argv[1])).href : "";
if (invokedPath === import.meta.url) {
  main().catch((error) => {
    process.stderr.write(`check-ix-runtime-consumers: ${error.message}\n`);
    process.exitCode = 1;
  });
}
