import assert from "node:assert/strict";
import { execFileSync, spawnSync } from "node:child_process";
import { createHash } from "node:crypto";
import {
  copyFileSync,
  mkdirSync,
  mkdtempSync,
  readFileSync,
  rmSync,
  writeFileSync
} from "node:fs";
import os from "node:os";
import path from "node:path";
import test from "node:test";
import { fileURLToPath } from "node:url";

const repoRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const recipeLineage = Object.freeze({
  status: "reviewed_source_assembled",
  reviewedSourceCommit: "8e5b33119b84fa23eee1f1d3534481101afed2a9",
  integratedSourceCommit: "1e919fafe783b68cef4abc65f99d585be482b4e7",
  integrationBaseCommit: "9ea26157a2bed29424e1052a909303985ba466bd",
  integrationRelation: "history_preserving_cherry_pick_assembly",
  sha256: "ef182296762e424522febed024117006d2bfa499e048a07fc0199a70e5556e8f",
  sourcePath: "appfw_ui/pds_health/ix-presentation-contract/registry/pds.ix.recipe-registry.v1.json",
  projectionPath: "appfw_runtime/contracts/pds_health/pds.ix.recipe-registry.v1.json"
});
const i2OverlayReceipt = Object.freeze({
  commit: "bb9b8423bbd72003c050079c3f27a279c08d0fd5",
  parent: "1e919fafe783b68cef4abc65f99d585be482b4e7",
  tree: "4bb816efd6112051ae49db317e933a227b5beeb7",
  paths: [
    "appfw_runtime/contracts/pds_health/pds.ix.recipe-registry.v1.provenance.json",
    "docs/specs/ix-eight-vignette-shared-foundation-r1.md",
    "scripts/check-pds-ix-runtime-contract.mjs",
    "scripts/check-pds-ix-runtime-contract.test.mjs"
  ],
  blobs: {
    "appfw_runtime/contracts/pds_health/pds.ix.recipe-registry.v1.provenance.json":
      ["9312fb477ec2c5a29b154289838fc0307d04f053", "df29219f619638f0ed114956a1afa988b3f6c458"],
    "docs/specs/ix-eight-vignette-shared-foundation-r1.md":
      ["1f835fe77a5033dbb0c7e3144f04272b46ccb420", "527add52e45ab0cf72c64786a738bf168ebb2d4d"],
    "scripts/check-pds-ix-runtime-contract.mjs":
      ["1d330cd32b1276134bad1882f7c1e532a2f3a549", "7f7813fba98e610ec5144a06efac4e8f0bd920e9"],
    "scripts/check-pds-ix-runtime-contract.test.mjs":
      ["fdd195c3f90e90be71a4dbfc47311d104e9340fa", "ec392772ac64e4484c1e394a4139af627f3b8ce5"]
  }
});
const projectionPaths = [
  "scripts/check-pds-ix-runtime-contract.mjs",
  "appfw_runtime/contracts/pds_health/pds.ix.presentation.v1.provenance.json",
  "appfw_runtime/contracts/pds_health/pds.ix.recipe-registry.v1.provenance.json",
  "appfw_runtime/contracts/pds_health/pds.ix.presentation.v1.schema.json",
  "appfw_runtime/contracts/pds_health/pds.ix.recipe-registry.v1.json",
  "appfw_runtime/contracts/pds_health/working-brief.presentation.json",
  "appfw_ui/pds_health/ix-presentation-contract/schema/pds.ix.presentation.v1.schema.json",
  "appfw_ui/pds_health/ix-presentation-contract/registry/pds.ix.recipe-registry.v1.json",
  "appfw_ui/pds_health/ix-presentation-contract/fixtures/working-brief.presentation.json",
  "appfw_ui/pds_health/ix-presentation-contract/package.json"
];

function copyProjectionFixture(destinationRoot) {
  for (const relative of projectionPaths) {
    const destination = path.join(destinationRoot, relative);
    mkdirSync(path.dirname(destination), { recursive: true });
    copyFileSync(path.join(repoRoot, relative), destination);
  }
}

test("PDS IX runtime projection is byte-identical and provenance-bound", () => {
  const output = execFileSync(
    process.execPath,
    [path.join(repoRoot, "scripts/check-pds-ix-runtime-contract.mjs")],
    { cwd: repoRoot, encoding: "utf8" }
  );
  const result = JSON.parse(output);
  assert.equal(result.ok, true);
  assert.equal(result.authority, "PDS Health Design System");
  assert.equal(
    result.reviewedSourceCommit,
    "b6e5e5e1bfff6be0a3eeed4ccf3f068798ab9b33"
  );
  assert.equal(
    result.integratedSourceCommit,
    "49cb47b0b6b8a0719f22493766d382d86430a7b2"
  );
  assert.equal(
    result.schema.sourceSha256,
    "efda83abe50ab4402977fc00408daf233bee3c10f04e718bfac62dd61ae0120a"
  );
  assert.equal(
    result.fixture.sourceSha256,
    "dac3de336e92b79a518208120e6190fcd9caf3cd519076efa446d33302dbc6cf"
  );
  assert.equal(
    result.recipeRegistry.sourceSha256,
    "ef182296762e424522febed024117006d2bfa499e048a07fc0199a70e5556e8f"
  );
  assert.equal(result.recipeRegistry.lineageStatus, recipeLineage.status);
  assert.equal(
    result.recipeRegistry.reviewedSourceCommit,
    recipeLineage.reviewedSourceCommit
  );
  assert.equal(
    result.recipeRegistry.integratedSourceCommit,
    recipeLineage.integratedSourceCommit
  );
  assert.equal(
    result.recipeRegistry.integrationBaseCommit,
    recipeLineage.integrationBaseCommit
  );
  assert.equal(
    result.recipeRegistry.integrationRelation,
    recipeLineage.integrationRelation
  );
});

test("approved recipe lineage and I2 overlay resolve without requiring candidate ancestry", () => {
  for (const commit of [
    recipeLineage.reviewedSourceCommit,
    recipeLineage.integratedSourceCommit,
    recipeLineage.integrationBaseCommit
  ]) {
    execFileSync("git", ["cat-file", "-e", `${commit}^{commit}`], { cwd: repoRoot });
  }
  execFileSync(
    "git",
    [
      "merge-base",
      "--is-ancestor",
      recipeLineage.integrationBaseCommit,
      recipeLineage.reviewedSourceCommit
    ],
    { cwd: repoRoot }
  );
  execFileSync(
    "git",
    [
      "merge-base",
      "--is-ancestor",
      recipeLineage.integrationBaseCommit,
      recipeLineage.integratedSourceCommit
    ],
    { cwd: repoRoot }
  );
  const sourceAncestry = spawnSync(
    "git",
    [
      "merge-base",
      "--is-ancestor",
      recipeLineage.reviewedSourceCommit,
      recipeLineage.integratedSourceCommit
    ],
    { cwd: repoRoot, encoding: "utf8" }
  );
  assert.equal(sourceAncestry.status, 1);

  assert.equal(
    execFileSync("git", ["show", "-s", "--format=%P", i2OverlayReceipt.commit], {
      cwd: repoRoot,
      encoding: "utf8"
    }).trim(),
    i2OverlayReceipt.parent
  );
  assert.equal(
    execFileSync("git", ["show", "-s", "--format=%T", i2OverlayReceipt.commit], {
      cwd: repoRoot,
      encoding: "utf8"
    }).trim(),
    i2OverlayReceipt.tree
  );
  assert.deepEqual(
    execFileSync(
      "git",
      ["diff-tree", "--no-commit-id", "--name-only", "-r", i2OverlayReceipt.commit],
      { cwd: repoRoot, encoding: "utf8" }
    ).trim().split("\n"),
    i2OverlayReceipt.paths
  );
  for (const [relativePath, [i1Blob, i2Blob]] of Object.entries(i2OverlayReceipt.blobs)) {
    assert.equal(
      execFileSync("git", ["rev-parse", `${i2OverlayReceipt.parent}:${relativePath}`], {
        cwd: repoRoot,
        encoding: "utf8"
      }).trim(),
      i1Blob
    );
    assert.equal(
      execFileSync("git", ["rev-parse", `${i2OverlayReceipt.commit}:${relativePath}`], {
        cwd: repoRoot,
        encoding: "utf8"
      }).trim(),
      i2Blob
    );
  }

  const reviewedSource = execFileSync(
    "git",
    ["show", `${recipeLineage.reviewedSourceCommit}:${recipeLineage.sourcePath}`],
    { cwd: repoRoot }
  );
  const reviewedProjection = execFileSync(
    "git",
    ["show", `${recipeLineage.reviewedSourceCommit}:${recipeLineage.projectionPath}`],
    { cwd: repoRoot }
  );
  const integratedSource = execFileSync(
    "git",
    ["show", `${recipeLineage.integratedSourceCommit}:${recipeLineage.sourcePath}`],
    { cwd: repoRoot }
  );
  const integratedProjection = execFileSync(
    "git",
    ["show", `${recipeLineage.integratedSourceCommit}:${recipeLineage.projectionPath}`],
    { cwd: repoRoot }
  );
  assert.deepEqual(reviewedSource, reviewedProjection);
  assert.deepEqual(reviewedSource, integratedSource);
  assert.deepEqual(reviewedSource, integratedProjection);
  assert.equal(createHash("sha256").update(reviewedSource).digest("hex"), recipeLineage.sha256);
});

test("projection checker rejects unsafe provenance paths before filesystem access", () => {
  const implementation = readFileSync(
    path.join(repoRoot, "scripts/check-pds-ix-runtime-contract.mjs"),
    "utf8"
  );
  assert.match(implementation, /path\.isAbsolute\(value\)/u);
  assert.match(implementation, /includes\("\.\."\)/u);
  assert.match(implementation, /source\.equals\(projection\)/u);
});

test("projection checker fails closed when projection bytes drift", () => {
  const temporary = mkdtempSync(path.join(os.tmpdir(), "appfw-pds-ix-projection-"));
  try {
    copyProjectionFixture(temporary);
    const projection = path.join(
      temporary,
      "appfw_runtime/contracts/pds_health/pds.ix.presentation.v1.schema.json"
    );
    writeFileSync(projection, Buffer.concat([readFileSync(projection), Buffer.from("\n")]));
    const result = spawnSync(
      process.execPath,
      [path.join(temporary, "scripts/check-pds-ix-runtime-contract.mjs")],
      { encoding: "utf8" }
    );
    assert.equal(result.status, 1);
    assert.match(result.stderr, /schema projection is not byte-identical/u);
  } finally {
    rmSync(temporary, { recursive: true, force: true });
  }
});

test("projection checker fails closed when recipe registry bytes drift", () => {
  const temporary = mkdtempSync(path.join(os.tmpdir(), "appfw-pds-ix-recipe-projection-"));
  try {
    copyProjectionFixture(temporary);
    const projection = path.join(
      temporary,
      "appfw_runtime/contracts/pds_health/pds.ix.recipe-registry.v1.json"
    );
    writeFileSync(projection, Buffer.concat([readFileSync(projection), Buffer.from("\n")]));
    const result = spawnSync(
      process.execPath,
      [path.join(temporary, "scripts/check-pds-ix-runtime-contract.mjs")],
      { encoding: "utf8" }
    );
    assert.equal(result.status, 1);
    assert.match(result.stderr, /recipe registry projection is not byte-identical/u);
  } finally {
    rmSync(temporary, { recursive: true, force: true });
  }
});

for (const [label, mutate] of [
  ["old status", (value) => { value.lineage_status = "working_tree_unreviewed"; }],
  ["wrong reviewed source", (value) => { value.reviewed_source_commit = "f".repeat(40); }],
  ["null reviewed source", (value) => { value.reviewed_source_commit = null; }],
  ["wrong integrated source", (value) => { value.integrated_source_commit = "0".repeat(40); }],
  ["null integrated source", (value) => { value.integrated_source_commit = null; }],
  ["wrong integration base", (value) => { value.integration_base_commit = "a".repeat(40); }],
  ["abbreviated integration base", (value) => { value.integration_base_commit = "9ea26157"; }],
  ["missing integration base", (value) => { delete value.integration_base_commit; }],
  ["wrong assembly relation", (value) => { value.integration_relation = "merge_commit"; }],
  ["missing assembly relation", (value) => { delete value.integration_relation; }]
]) {
  test(`projection checker rejects recipe lineage with ${label}`, () => {
    const temporary = mkdtempSync(path.join(os.tmpdir(), "appfw-pds-ix-recipe-lineage-"));
    try {
      copyProjectionFixture(temporary);
      const provenancePath = path.join(
        temporary,
        "appfw_runtime/contracts/pds_health/pds.ix.recipe-registry.v1.provenance.json"
      );
      const provenance = JSON.parse(readFileSync(provenancePath, "utf8"));
      mutate(provenance);
      writeFileSync(provenancePath, `${JSON.stringify(provenance, null, 2)}\n`);
      const result = spawnSync(
        process.execPath,
        [path.join(temporary, "scripts/check-pds-ix-runtime-contract.mjs")],
        { encoding: "utf8" }
      );
      assert.equal(result.status, 1);
      assert.match(result.stderr, /lineage|commit/u);
    } finally {
      rmSync(temporary, { recursive: true, force: true });
    }
  });
}

test("projection checker rejects redirected identity even when copied bytes match", () => {
  const temporary = mkdtempSync(path.join(os.tmpdir(), "appfw-pds-ix-identity-"));
  try {
    copyProjectionFixture(temporary);
    const provenancePath = path.join(
      temporary,
      "appfw_runtime/contracts/pds_health/pds.ix.presentation.v1.provenance.json"
    );
    const provenance = JSON.parse(readFileSync(provenancePath, "utf8"));
    provenance.source_path = provenance.fixture_source_path;
    writeFileSync(provenancePath, `${JSON.stringify(provenance, null, 2)}\n`);
    const result = spawnSync(
      process.execPath,
      [path.join(temporary, "scripts/check-pds-ix-runtime-contract.mjs")],
      { encoding: "utf8" }
    );
    assert.equal(result.status, 1);
    assert.match(result.stderr, /projection paths are not canonical/u);
  } finally {
    rmSync(temporary, { recursive: true, force: true });
  }
});

test("projection checker rejects package identity drift", () => {
  const temporary = mkdtempSync(path.join(os.tmpdir(), "appfw-pds-ix-package-"));
  try {
    copyProjectionFixture(temporary);
    const packagePath = path.join(
      temporary,
      "appfw_ui/pds_health/ix-presentation-contract/package.json"
    );
    const sourcePackage = JSON.parse(readFileSync(packagePath, "utf8"));
    sourcePackage.version = "0.1.1";
    writeFileSync(packagePath, `${JSON.stringify(sourcePackage, null, 2)}\n`);
    const result = spawnSync(
      process.execPath,
      [path.join(temporary, "scripts/check-pds-ix-runtime-contract.mjs")],
      { encoding: "utf8" }
    );
    assert.equal(result.status, 1);
    assert.match(result.stderr, /package identity does not match provenance/u);
  } finally {
    rmSync(temporary, { recursive: true, force: true });
  }
});

test("projection checker rejects provenance digest substitution", () => {
  const temporary = mkdtempSync(path.join(os.tmpdir(), "appfw-pds-ix-digest-"));
  try {
    copyProjectionFixture(temporary);
    const provenancePath = path.join(
      temporary,
      "appfw_runtime/contracts/pds_health/pds.ix.presentation.v1.provenance.json"
    );
    const provenance = JSON.parse(readFileSync(provenancePath, "utf8"));
    provenance.source_sha256 = "0".repeat(64);
    provenance.projection_sha256 = "0".repeat(64);
    writeFileSync(provenancePath, `${JSON.stringify(provenance, null, 2)}\n`);
    const result = spawnSync(
      process.execPath,
      [path.join(temporary, "scripts/check-pds-ix-runtime-contract.mjs")],
      { encoding: "utf8" }
    );
    assert.equal(result.status, 1);
    assert.match(result.stderr, /digests are not the reviewed contract digests/u);
  } finally {
    rmSync(temporary, { recursive: true, force: true });
  }
});

test("projection checker binds canonical package identity and paths", () => {
  const implementation = readFileSync(
    path.join(repoRoot, "scripts/check-pds-ix-runtime-contract.mjs"),
    "utf8"
  );
  assert.match(implementation, /@appfw\/pds-ix-presentation-contract/u);
  assert.match(implementation, /ix-presentation-contract\/package\.json/u);
  assert.match(implementation, /projection paths are not canonical/u);
  assert.match(implementation, /reviewed_source_commit/u);
  assert.match(implementation, /integrated_source_commit/u);
  assert.match(implementation, /integration_base_commit/u);
  assert.match(implementation, /history_preserving_cherry_pick_assembly/u);
  assert.match(implementation, /pds\.ix\.recipe-registry\.v1\.json/u);
  assert.match(implementation, /reviewed_source_assembled/u);
});

test("isolated generator proof copies packaged runtime contracts", () => {
  const implementation = readFileSync(
    path.join(repoRoot, "scripts/check_app_gen_backend_equivalence.sh"),
    "utf8"
  );
  assert.match(
    implementation,
    /Cargo\.toml Cargo\.lock build\.rs src tests benches examples contracts/u
  );
});
