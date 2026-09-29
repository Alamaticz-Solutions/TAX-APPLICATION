#!/usr/bin/env node

import { createHash } from "node:crypto";
import { readFile } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";

const repoRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const presentationProvenancePath = path.join(
  repoRoot,
  "appfw_runtime/contracts/pds_health/pds.ix.presentation.v1.provenance.json"
);
const recipeProvenancePath = path.join(
  repoRoot,
  "appfw_runtime/contracts/pds_health/pds.ix.recipe-registry.v1.provenance.json"
);
const expected = Object.freeze({
  sourcePackage: "@appfw/pds-ix-presentation-contract",
  sourceVersion: "0.1.0",
  currentPackageVersion: "0.2.0",
  reviewedSourceCommit: "b6e5e5e1bfff6be0a3eeed4ccf3f068798ab9b33",
  integratedSourceCommit: "49cb47b0b6b8a0719f22493766d382d86430a7b2",
  schemaSha256: "efda83abe50ab4402977fc00408daf233bee3c10f04e718bfac62dd61ae0120a",
  fixtureSha256: "dac3de336e92b79a518208120e6190fcd9caf3cd519076efa446d33302dbc6cf",
  sourcePath: "appfw_ui/pds_health/ix-presentation-contract/schema/pds.ix.presentation.v1.schema.json",
  projectionPath: "appfw_runtime/contracts/pds_health/pds.ix.presentation.v1.schema.json",
  fixtureSourcePath: "appfw_ui/pds_health/ix-presentation-contract/fixtures/working-brief.presentation.json",
  fixtureProjectionPath: "appfw_runtime/contracts/pds_health/working-brief.presentation.json",
  packagePath: "appfw_ui/pds_health/ix-presentation-contract/package.json",
  recipe: Object.freeze({
    sourceVersion: "0.2.0",
    sha256: "ef182296762e424522febed024117006d2bfa499e048a07fc0199a70e5556e8f",
    sourcePath: "appfw_ui/pds_health/ix-presentation-contract/registry/pds.ix.recipe-registry.v1.json",
    projectionPath: "appfw_runtime/contracts/pds_health/pds.ix.recipe-registry.v1.json",
    lineageStatus: "reviewed_source_assembled",
    reviewedSourceCommit: "8e5b33119b84fa23eee1f1d3534481101afed2a9",
    integratedSourceCommit: "1e919fafe783b68cef4abc65f99d585be482b4e7",
    integrationBaseCommit: "9ea26157a2bed29424e1052a909303985ba466bd",
    integrationRelation: "history_preserving_cherry_pick_assembly"
  })
});

function sha256(bytes) {
  return createHash("sha256").update(bytes).digest("hex");
}

function requireSafeRelativePath(value, field) {
  if (
    typeof value !== "string"
    || value.length === 0
    || path.isAbsolute(value)
    || value.split(/[\\/]/u).includes("..")
  ) {
    throw new Error(`${field} must be a safe repository-relative path`);
  }
  return value;
}

function requireFullCommit(value, field) {
  if (typeof value !== "string" || !/^[0-9a-f]{40}$/u.test(value)) {
    throw new Error(`${field} must be a full lowercase Git commit`);
  }
  return value;
}

async function verifyProjection({
  sourcePath,
  projectionPath,
  expectedSourceSha256,
  expectedProjectionSha256,
  label
}) {
  const source = await readFile(path.join(repoRoot, sourcePath));
  const projection = await readFile(path.join(repoRoot, projectionPath));
  const sourceSha256 = sha256(source);
  const projectionSha256 = sha256(projection);
  if (!source.equals(projection)) {
    throw new Error(`${label} projection is not byte-identical to its PDS source`);
  }
  if (sourceSha256 !== expectedSourceSha256) {
    throw new Error(`${label} source SHA-256 does not match provenance`);
  }
  if (projectionSha256 !== expectedProjectionSha256) {
    throw new Error(`${label} projection SHA-256 does not match provenance`);
  }
  return { sourcePath, projectionPath, sourceSha256, projectionSha256 };
}

const provenance = JSON.parse(await readFile(presentationProvenancePath, "utf8"));
if (provenance.schema !== "appfw.pds_ix_presentation_schema_projection@1") {
  throw new Error("unsupported PDS IX presentation projection provenance schema");
}
if (provenance.authority !== "PDS Health Design System") {
  throw new Error("PDS Health Design System must remain the schema authority");
}
if (provenance.generated !== true || provenance.hand_edit !== "forbidden") {
  throw new Error("the runtime projection must remain generated and non-editable");
}
if (provenance.source_package !== expected.sourcePackage) {
  throw new Error("unexpected PDS IX presentation source package");
}
if (provenance.source_version !== expected.sourceVersion) {
  throw new Error("unexpected PDS IX presentation source version");
}
if (provenance.current_package_version !== expected.currentPackageVersion) {
  throw new Error("unexpected current PDS IX contract package version");
}
if (
  provenance.source_path !== expected.sourcePath
  || provenance.projection_path !== expected.projectionPath
  || provenance.fixture_source_path !== expected.fixtureSourcePath
  || provenance.fixture_projection_path !== expected.fixtureProjectionPath
) {
  throw new Error("PDS IX presentation projection paths are not canonical");
}
if (
  provenance.reviewed_source_commit !== expected.reviewedSourceCommit
  || provenance.integrated_source_commit !== expected.integratedSourceCommit
) {
  throw new Error("PDS IX presentation source lineage is not the reviewed integration lineage");
}
if (
  provenance.source_sha256 !== expected.schemaSha256
  || provenance.projection_sha256 !== expected.schemaSha256
  || provenance.fixture_source_sha256 !== expected.fixtureSha256
  || provenance.fixture_projection_sha256 !== expected.fixtureSha256
) {
  throw new Error("PDS IX presentation digests are not the reviewed contract digests");
}
const sourcePackage = JSON.parse(
  await readFile(path.join(repoRoot, expected.packagePath), "utf8")
);
if (sourcePackage.name !== provenance.source_package
  || sourcePackage.version !== provenance.current_package_version) {
  throw new Error("PDS IX presentation package identity does not match provenance");
}

const schema = await verifyProjection({
  sourcePath: requireSafeRelativePath(provenance.source_path, "source_path"),
  projectionPath: requireSafeRelativePath(provenance.projection_path, "projection_path"),
  expectedSourceSha256: provenance.source_sha256,
  expectedProjectionSha256: provenance.projection_sha256,
  label: "schema"
});
const fixture = await verifyProjection({
  sourcePath: requireSafeRelativePath(provenance.fixture_source_path, "fixture_source_path"),
  projectionPath: requireSafeRelativePath(
    provenance.fixture_projection_path,
    "fixture_projection_path"
  ),
  expectedSourceSha256: provenance.fixture_source_sha256,
  expectedProjectionSha256: provenance.fixture_projection_sha256,
  label: "fixture"
});

const recipeProvenance = JSON.parse(await readFile(recipeProvenancePath, "utf8"));
if (recipeProvenance.schema !== "appfw.pds_ix_recipe_registry_projection@1") {
  throw new Error("unsupported PDS IX recipe registry projection provenance schema");
}
if (recipeProvenance.authority !== "PDS Health Design System") {
  throw new Error("PDS Health Design System must remain the recipe registry authority");
}
if (recipeProvenance.generated !== true || recipeProvenance.hand_edit !== "forbidden") {
  throw new Error("the runtime recipe projection must remain generated and non-editable");
}
if (
  recipeProvenance.source_package !== expected.sourcePackage
  || recipeProvenance.source_version !== expected.recipe.sourceVersion
  || recipeProvenance.current_package_version !== expected.currentPackageVersion
  || sourcePackage.name !== recipeProvenance.source_package
  || sourcePackage.version !== recipeProvenance.current_package_version
) {
  throw new Error("PDS IX recipe registry package identity does not match provenance");
}
if (
  recipeProvenance.source_path !== expected.recipe.sourcePath
  || recipeProvenance.projection_path !== expected.recipe.projectionPath
) {
  throw new Error("PDS IX recipe registry projection paths are not canonical");
}
if (
  recipeProvenance.source_sha256 !== expected.recipe.sha256
  || recipeProvenance.projection_sha256 !== expected.recipe.sha256
) {
  throw new Error("PDS IX recipe registry digests are not the accepted contract digests");
}
requireFullCommit(recipeProvenance.reviewed_source_commit, "reviewed_source_commit");
requireFullCommit(recipeProvenance.integrated_source_commit, "integrated_source_commit");
requireFullCommit(recipeProvenance.integration_base_commit, "integration_base_commit");
if (
  recipeProvenance.lineage_status !== expected.recipe.lineageStatus
  || recipeProvenance.reviewed_source_commit !== expected.recipe.reviewedSourceCommit
  || recipeProvenance.integrated_source_commit !== expected.recipe.integratedSourceCommit
  || recipeProvenance.integration_base_commit !== expected.recipe.integrationBaseCommit
  || recipeProvenance.integration_relation !== expected.recipe.integrationRelation
) {
  throw new Error("PDS IX recipe registry lineage is not the approved reviewed-source assembly");
}
const recipeRegistry = await verifyProjection({
  sourcePath: requireSafeRelativePath(recipeProvenance.source_path, "recipe.source_path"),
  projectionPath: requireSafeRelativePath(
    recipeProvenance.projection_path,
    "recipe.projection_path"
  ),
  expectedSourceSha256: recipeProvenance.source_sha256,
  expectedProjectionSha256: recipeProvenance.projection_sha256,
  label: "recipe registry"
});

process.stdout.write(`${JSON.stringify({
  ok: true,
  authority: provenance.authority,
  sourcePackage: provenance.source_package,
  sourceVersion: provenance.source_version,
  reviewedSourceCommit: provenance.reviewed_source_commit,
  integratedSourceCommit: provenance.integrated_source_commit,
  schema,
  fixture,
  recipeRegistry: {
    ...recipeRegistry,
    lineageStatus: recipeProvenance.lineage_status,
    reviewedSourceCommit: recipeProvenance.reviewed_source_commit,
    integratedSourceCommit: recipeProvenance.integrated_source_commit,
    integrationBaseCommit: recipeProvenance.integration_base_commit,
    integrationRelation: recipeProvenance.integration_relation
  }
}, null, 2)}\n`);
