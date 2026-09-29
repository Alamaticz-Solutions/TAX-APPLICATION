import assert from "node:assert/strict";
import test from "node:test";

import {
  evaluateRuntimeConsumers,
  expectedCrates,
  parseLockPackages,
  parseManifest
} from "./check-ix-runtime-consumers.mjs";

const protectedRootLockSha256 =
  "e65ecfa882ef0997f8710b67e2e578fcbe17e0735d4b2482797baf3178f1ca89";

function packageRecords(overrides = {}) {
  return expectedCrates.map(([name, version]) => ({
    name,
    version: overrides[name] ?? version
  }));
}

function model() {
  const manifests = Object.fromEntries(expectedCrates.map(([
    name,
    version,
    manifestPath,
    runtimeVersion
  ]) => [manifestPath, {
    name,
    version,
    runtime_dependency_version: runtimeVersion,
    runtime_dependency_path: runtimeVersion === null ? null : "../appfw_runtime",
    runtime_dependency_registry: runtimeVersion === null ? null : "pds-app-framework-crates"
  }]));
  return {
    manifests,
    consumers: {
      "database/Cargo.toml": {
        runtime_dependency_path: "../appfw_runtime",
        runtime_dependency_version: null
      },
      "examples/products/crm/backend/Cargo.toml": {
        runtime_dependency_path: "../../../../appfw_runtime",
        runtime_dependency_version: null
      },
      "examples/products/pds-nexus/backend/Cargo.toml": {
        runtime_dependency_path: "../../../../appfw_runtime",
        runtime_dependency_version: null
      },
      "app_gen/_golden/downstream_apps/second-consumer/starter/backend/Cargo.toml": {
        runtime_dependency_path: "../../../../appfw_runtime",
        runtime_dependency_version: null
      }
    },
    oldRuntimeConstraints: [],
    crmLockPackages: packageRecords().filter(({ name }) => name !== "appfw-cli"),
    rootLockPackages: packageRecords({
      "appfw-runtime": "0.1.2",
      "appfw-cli": "0.1.6",
      "appfw-provider-mongo": "0.1.1",
      "appfw-provider-mssql": "0.1.1",
      "appfw-provider-neo4j": "0.1.1",
      "appfw-provider-postgres": "0.1.1",
      "appfw-provider-snowflake": "0.1.1"
    }),
    rootLockSha256: protectedRootLockSha256,
    candidateCommit: "32160570b7d8d5b8e813e95da053918cba878af8"
  };
}

test("manifest parser extracts package and registry runtime dependency", () => {
  assert.deepEqual(parseManifest(`
[package]
name = "appfw-cli"
version = "0.1.7"

[dependencies]
appfw_runtime = { package = "appfw-runtime", version = "0.2.0", path = "../appfw_runtime", registry = "pds-app-framework-crates" }
`), {
    name: "appfw-cli",
    version: "0.1.7",
    runtime_dependency_version: "0.2.0",
    runtime_dependency_path: "../appfw_runtime",
    runtime_dependency_registry: "pds-app-framework-crates"
  });
});

test("lock parser retains duplicate identities for fail-closed validation", () => {
  assert.deepEqual(parseLockPackages(`
version = 3
[[package]]
name = "appfw-runtime"
version = "0.2.0"
[[package]]
name = "appfw-runtime"
version = "0.1.2"
`), [
    { name: "appfw-runtime", version: "0.2.0" },
    { name: "appfw-runtime", version: "0.1.2" }
  ]);
});

test("Leaf A may retain only the exact protected Integration-pending root lock", () => {
  const result = evaluateRuntimeConsumers(model());
  assert.equal(result.ok, true);
  assert.equal(result.locks.crm, "aligned");
  assert.equal(result.locks.root, "integration_pending_exact_protected_base");
  assert.equal(result.publication_state, "unpublished_candidate");
});

test("assembled candidate accepts one aligned runtime identity", () => {
  const candidate = model();
  candidate.rootLockPackages = packageRecords();
  candidate.rootLockSha256 = "f".repeat(64);
  const result = evaluateRuntimeConsumers(candidate);
  assert.equal(result.locks.root, "aligned");
  assert.deepEqual(result.runtime_nodes, [{ name: "appfw-runtime", version: "0.2.0", count: 1 }]);
});

test("manifest identity drift fails closed", () => {
  const candidate = model();
  candidate.manifests["appfw_cli/Cargo.toml"].version = "0.1.6";
  assert.throws(() => evaluateRuntimeConsumers(candidate), /appfw-cli@0\.1\.7/u);
});

test("old or missing registry runtime dependency fails closed", () => {
  const candidate = model();
  candidate.manifests["appfw_provider_mongo/Cargo.toml"].runtime_dependency_version = "0.1.2";
  assert.throws(() => evaluateRuntimeConsumers(candidate), /appfw-runtime@0\.2\.0/u);
  const missingRegistry = model();
  missingRegistry.manifests["appfw_cli/Cargo.toml"].runtime_dependency_registry = null;
  assert.throws(() => evaluateRuntimeConsumers(missingRegistry), /registry appfw-runtime/u);
});

test("repository-wide explicit old runtime constraint fails closed", () => {
  const candidate = model();
  candidate.oldRuntimeConstraints = ["downstream/Cargo.toml"];
  assert.throws(() => evaluateRuntimeConsumers(candidate), /explicit old appfw-runtime constraint/u);
});

test("CRM lock missing, old, or duplicate runtime identity fails closed", () => {
  const old = model();
  old.crmLockPackages.find(({ name }) => name === "appfw-runtime").version = "0.1.2";
  assert.throws(() => evaluateRuntimeConsumers(old), /CRM lock must contain exactly appfw-runtime@0\.2\.0/u);
  const duplicate = model();
  duplicate.crmLockPackages.push({ name: "appfw-runtime", version: "0.2.0" });
  assert.throws(() => evaluateRuntimeConsumers(duplicate), /CRM lock must contain exactly/u);
});

test("unreviewed stale root lock fails closed", () => {
  const candidate = model();
  candidate.rootLockSha256 = "0".repeat(64);
  assert.throws(() => evaluateRuntimeConsumers(candidate), /neither aligned nor the exact protected/u);
});

test("missing or redirected path consumer fails closed", () => {
  const missing = model();
  delete missing.consumers["database/Cargo.toml"];
  assert.throws(() => evaluateRuntimeConsumers(missing), /missing required path consumer/u);
  const redirected = model();
  redirected.consumers["examples/products/crm/backend/Cargo.toml"].runtime_dependency_path = "vendor/runtime";
  assert.throws(() => evaluateRuntimeConsumers(redirected), /one local appfw-runtime candidate/u);
});

test("known MSSQL collision remains retained and unpublished", () => {
  const result = evaluateRuntimeConsumers(model());
  assert.deepEqual(result.known_collision, {
    name: "appfw-provider-mssql",
    version: "0.1.1",
    checksum: "09afb7c4acd00c97fa7cfafe3a234319e681d4c8228e6b30c8892fdbbbfb7572",
    runtime_dependency: "0.1.1",
    disposition: "retained_do_not_reuse"
  });
  assert.match(result.nonclaims.join(" "), /No package publication/u);
});
