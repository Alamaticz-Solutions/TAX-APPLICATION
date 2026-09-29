#!/usr/bin/env node

import assert from "node:assert/strict";
import {
  existsSync,
  mkdtempSync,
  readFileSync,
  readdirSync,
  rmSync,
  writeFileSync,
} from "node:fs";
import { tmpdir } from "node:os";
import { basename, join } from "node:path";

import {
  NEXUS_WORKSTREAM_TEST_MANIFEST,
  assertExactNexusWorkstreamTestManifest,
  parseNexusWorkstreamShardTap,
  partitionNexusWorkstreamTests,
  runNexusWorkstreamTestShards,
  validateNexusWorkstreamShardResult,
} from "./run-nexus-workstream-tests.mjs";

function tapFor(names) {
  const records = names.flatMap((name, index) => [
    `    # Subtest: ${name}`,
    `    ok ${index + 1} - ${name}`,
  ]);
  return [
    "TAP version 13",
    "# Subtest: workstation assignment and Product Increment admission",
    ...records,
    `    1..${names.length}`,
    "ok 1 - workstation assignment and Product Increment admission",
    "1..1",
    `# tests ${names.length}`,
    "# suites 1",
    `# pass ${names.length}`,
    "# fail 0",
    "# cancelled 0",
    "# skipped 0",
    "# todo 0",
    "",
  ].join("\n");
}

assert.equal(NEXUS_WORKSTREAM_TEST_MANIFEST.length, 36);
assertExactNexusWorkstreamTestManifest([...NEXUS_WORKSTREAM_TEST_MANIFEST]);

const shards = partitionNexusWorkstreamTests(4);
assert.equal(shards.length, 4);
assert(shards.every((shard) => shard.length === 9));
assert.equal(shards.flat().length, 36);
assert.equal(new Set(shards.flat()).size, 36);
assert.deepEqual(new Set(shards.flat()), new Set(NEXUS_WORKSTREAM_TEST_MANIFEST));
assert.throws(() => partitionNexusWorkstreamTests(0), /1 through 4/);
assert.throws(() => partitionNexusWorkstreamTests(5), /1 through 4/);

const expected = NEXUS_WORKSTREAM_TEST_MANIFEST.slice(0, 2);
const validTap = tapFor(expected);
assert.deepEqual(parseNexusWorkstreamShardTap(validTap, expected), expected);
assert.deepEqual(
  validateNexusWorkstreamShardResult(
    { status: 0, signal: null, error: undefined, stdout: validTap },
    expected,
  ),
  expected,
);
assert.throws(
  () => validateNexusWorkstreamShardResult(
    { status: 1, signal: null, error: undefined, stdout: validTap },
    expected,
  ),
  /exited with status 1/,
);
assert.throws(
  () => validateNexusWorkstreamShardResult(
    { status: null, signal: "SIGTERM", error: undefined, stdout: validTap },
    expected,
  ),
  /terminated by signal SIGTERM/,
);
assert.throws(
  () => parseNexusWorkstreamShardTap("not TAP\n", expected),
  /not TAP version 13/,
);
assert.throws(
  () => parseNexusWorkstreamShardTap(tapFor(expected.slice(0, 1)), expected),
  /subtests differ from its assignment/,
);
assert.throws(
  () => parseNexusWorkstreamShardTap(tapFor([expected[0], expected[0]]), expected),
  /subtests differ from its assignment/,
);

const withUnplannedOuterRecord = validTap.replace(
  "\n1..1\n# tests",
  "\nok 2 - unplanned outer record\n1..1\n# tests",
);
assert.throws(
  () => parseNexusWorkstreamShardTap(withUnplannedOuterRecord, expected),
  /outer result records must exactly match plan 1\.\.1/,
);

const withMalformedUnplannedOuterRecord = validTap.replace(
  "\n1..1\n# tests",
  "\nok 2 # unexpected directive\n1..1\n# tests",
);
assert.throws(
  () => parseNexusWorkstreamShardTap(withMalformedUnplannedOuterRecord, expected),
  /outer result records must exactly match plan 1\.\.1/,
);

const withDuplicateOuterRecord = validTap.replace(
  "\n1..1\n# tests",
  "\nok 1 - workstation assignment and Product Increment admission\n1..1\n# tests",
);
assert.throws(
  () => parseNexusWorkstreamShardTap(withDuplicateOuterRecord, expected),
  /outer result records must exactly match plan 1\.\.1/,
);

const withContradictoryOuterPlan = validTap.replace(
  "\n1..1\n# tests",
  "\n1..2\n# tests",
);
assert.throws(
  () => parseNexusWorkstreamShardTap(withContradictoryOuterPlan, expected),
  /exactly one outer plan 1\.\.1/,
);

const withDuplicateOuterPlan = validTap.replace(
  "\n1..1\n# tests",
  "\n1..1\n1..1\n# tests",
);
assert.throws(
  () => parseNexusWorkstreamShardTap(withDuplicateOuterPlan, expected),
  /exactly one outer plan 1\.\.1/,
);

const withDirectiveOuterPlan = validTap.replace(
  "\n1..1\n# tests",
  "\n1..2 # unexpected directive\n1..1\n# tests",
);
assert.throws(
  () => parseNexusWorkstreamShardTap(withDirectiveOuterPlan, expected),
  /exactly one outer plan 1\.\.1/,
);

const withOuterPlanBeforeResult = validTap.replace(
  "ok 1 - workstation assignment and Product Increment admission\n1..1",
  "1..1\nok 1 - workstation assignment and Product Increment admission",
);
assert.throws(
  () => parseNexusWorkstreamShardTap(withOuterPlanBeforeResult, expected),
  /not in canonical order/,
);

const withBailout = validTap.replace("\n1..1\n", "\nBail out! injected\n1..1\n");
assert.throws(
  () => parseNexusWorkstreamShardTap(withBailout, expected),
  /contains a bailout/,
);

function syntheticSuccessfulResults(shards) {
  return shards.map((names, index) => ({
    index,
    names,
    status: 0,
    signal: null,
    error: undefined,
    stdout: tapFor(names),
    stderr: "",
  }));
}

function stableCleanSourceState() {
  return {
    head: "1".repeat(40),
    tree: "2".repeat(40),
    clean: true,
    status_sha256: "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
  };
}

function summaryFrom(logRoot) {
  const runRoots = readdirSync(logRoot);
  assert.equal(runRoots.length, 1, "contract test must retain exactly one run root");
  const runRoot = join(logRoot, runRoots[0]);
  const summary = JSON.parse(readFileSync(join(runRoot, "summary.json"), "utf8"));
  assert.deepEqual(
    readdirSync(runRoot).filter((name) => name.includes("summary")),
    ["summary.json"],
    "runner must atomically publish exactly one terminal summary",
  );
  return summary;
}

const successLogRoot = mkdtempSync(join(tmpdir(), "appfw-nexus-runner-success-"));
let successExtraRoot;
try {
  const summary = await runNexusWorkstreamTestShards({
    shardCount: 4,
    logRoot: successLogRoot,
    lifecycleTestHooks: {
      captureSourceState: stableCleanSourceState,
      executeShards: async ({ shards: assignedShards, runId }) => {
        successExtraRoot = mkdtempSync(
          join(tmpdir(), `appfw-nexus-live-${runId}-contract-extra-`),
        );
        return syntheticSuccessfulResults(assignedShards);
      },
    },
  });
  assert.equal(summary.ok, true);
  assert.equal(summary.finalized, true);
  assert.equal(summary.source.stable, true);
  assert.equal(summary.authoritative, false);
  assert.equal(summary.exact_candidate_bound, false);
  assert.deepEqual(summary.authority.reasons, ["lifecycle_test_hooks_active"]);
  assert.deepEqual(summary.authority.lifecycle_test_hooks, [
    "captureSourceState",
    "executeShards",
  ]);
  assert.equal(summary.authority.test_source.requested_is_canonical, true);
  assert.equal(
    summary.authority.test_source.blob_bound,
    summary.authority.test_source.tracked_git_blob ===
      summary.authority.test_source.executed_git_blob,
  );
  assert.match(summary.authority.test_source.content_sha256, /^[0-9a-f]{64}$/);
  assert.equal(summary.schema, "appfw_nexus_workstream_test_shards@2");
  assert.equal(summary.test_count, 36);
  assert.equal(summary.canonical_manifest.sha256, summary.observed_union.sha256);
  assert(summary.cleanup.before.includes(basename(successExtraRoot)));
  assert.deepEqual(summary.cleanup.after, []);
  assert.equal(summary.cleanup.ok, true);
  assert.equal(existsSync(successExtraRoot), false);
  assert.deepEqual(summaryFrom(successLogRoot), summary);
} finally {
  if (successExtraRoot) rmSync(successExtraRoot, { recursive: true, force: true });
  rmSync(successLogRoot, { recursive: true, force: true });
}

const concealedHooksLogRoot = mkdtempSync(
  join(tmpdir(), "appfw-nexus-runner-concealed-hooks-"),
);
try {
  const hookContainer = Object.create({
    captureSourceState: stableCleanSourceState,
  });
  Object.defineProperty(hookContainer, "executeShards", {
    configurable: true,
    enumerable: false,
    value: async ({ shards }) => syntheticSuccessfulResults(shards),
  });
  const hookReadCounts = new Map();
  const statefulHookContainer = new Proxy(hookContainer, {
    get(target, property, receiver) {
      if (typeof property === "string") {
        hookReadCounts.set(property, (hookReadCounts.get(property) ?? 0) + 1);
      }
      return Reflect.get(target, property, receiver);
    },
  });

  const summary = await runNexusWorkstreamTestShards({
    shardCount: 4,
    logRoot: concealedHooksLogRoot,
    lifecycleTestHooks: statefulHookContainer,
  });
  assert.equal(summary.ok, true);
  assert.equal(summary.authoritative, false);
  assert.equal(summary.exact_candidate_bound, false);
  assert.deepEqual(summary.authority.reasons, ["lifecycle_test_hooks_active"]);
  assert.deepEqual(summary.authority.lifecycle_test_hooks, [
    "captureSourceState",
    "executeShards",
  ]);
  assert.deepEqual(
    Object.fromEntries(hookReadCounts),
    {
      executeShards: 1,
      cleanupRunFixtures: 1,
      captureSourceState: 1,
    },
    "each allowed hook must be snapshotted exactly once",
  );
  assert.deepEqual(summaryFrom(concealedHooksLogRoot), summary);
} finally {
  rmSync(concealedHooksLogRoot, { recursive: true, force: true });
}

await assert.rejects(
  runNexusWorkstreamTestShards({
    lifecycleTestHooks: { unrecognizedHook: () => {} },
  }),
  /unknown lifecycle test hook: unrecognizedHook/,
);
await assert.rejects(
  runNexusWorkstreamTestShards({
    lifecycleTestHooks: { executeShards: true },
  }),
  /lifecycle test hook must be a function: executeShards/,
);

const failureLogRoot = mkdtempSync(join(tmpdir(), "appfw-nexus-runner-failure-"));
let failureExtraRoot;
try {
  await assert.rejects(
    runNexusWorkstreamTestShards({
      shardCount: 4,
      logRoot: failureLogRoot,
      lifecycleTestHooks: {
        captureSourceState: stableCleanSourceState,
        executeShards: async ({ shards: assignedShards, runId }) => {
          failureExtraRoot = mkdtempSync(
            join(tmpdir(), `appfw-nexus-live-${runId}-contract-extra-`),
          );
          return syntheticSuccessfulResults(assignedShards);
        },
        cleanupRunFixtures: (runId) => {
          const prefix = `appfw-nexus-live-${runId}-`;
          for (const name of readdirSync(tmpdir()).filter((entry) => entry.startsWith(prefix))) {
            rmSync(join(tmpdir(), name), { recursive: true, force: true });
          }
          throw new Error("injected cleanup failure after real removal");
        },
      },
    }),
    /terminal lifecycle checks/,
  );
  const summary = summaryFrom(failureLogRoot);
  assert.equal(summary.ok, false);
  assert.equal(summary.finalized, true);
  assert.equal(summary.authoritative, false);
  assert.equal(summary.exact_candidate_bound, false);
  assert.equal(summary.cleanup.ok, false);
  assert.match(summary.cleanup.error.message, /injected cleanup failure/);
  assert.deepEqual(summary.cleanup.after, []);
  assert.equal(existsSync(failureExtraRoot), false);
} finally {
  if (failureExtraRoot) rmSync(failureExtraRoot, { recursive: true, force: true });
  rmSync(failureLogRoot, { recursive: true, force: true });
}

const alternateTestRoot = mkdtempSync(join(tmpdir(), "appfw-nexus-runner-alternate-"));
const alternateLogRoot = join(alternateTestRoot, "logs");
const alternateTestFile = join(alternateTestRoot, "alternate.test.mjs");
try {
  writeFileSync(
    alternateTestFile,
    [
      'import { describe, test } from "node:test";',
      `const names = ${JSON.stringify([...NEXUS_WORKSTREAM_TEST_MANIFEST])};`,
      'describe("workstation assignment and Product Increment admission", () => {',
      "  for (const name of names) {",
      "    test(name, () => {});",
      "  }",
      "});",
      "",
    ].join("\n"),
  );
  const summary = await runNexusWorkstreamTestShards({
    shardCount: 4,
    logRoot: alternateLogRoot,
    testFile: alternateTestFile,
  });
  assert.equal(summary.ok, true);
  assert.equal(summary.authoritative, false);
  assert.equal(summary.exact_candidate_bound, false);
  assert.deepEqual(summary.authority.reasons, ["noncanonical_test_file"]);
  assert.deepEqual(summary.authority.lifecycle_test_hooks, []);
  assert.equal(summary.authority.test_source.requested_path, "noncanonical");
  assert.equal(summary.authority.test_source.requested_is_canonical, false);
  assert.equal(summary.authority.test_source.blob_bound, false);
  assert.match(summary.authority.test_source.content_sha256, /^[0-9a-f]{64}$/);
  assert.deepEqual(summaryFrom(alternateLogRoot), summary);
} finally {
  rmSync(alternateTestRoot, { recursive: true, force: true });
}

console.log("Nexus workstream sharded-runner contract tests passed");
