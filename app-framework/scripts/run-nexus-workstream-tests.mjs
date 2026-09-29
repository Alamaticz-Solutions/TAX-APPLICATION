#!/usr/bin/env node

import assert from "node:assert/strict";
import { execFileSync, spawn } from "node:child_process";
import { createHash, randomUUID } from "node:crypto";
import {
  mkdirSync,
  readFileSync,
  readdirSync,
  renameSync,
  rmSync,
  writeFileSync,
} from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { performance } from "node:perf_hooks";

export const NEXUS_WORKSTREAM_TEST_MANIFEST = Object.freeze([
  "[fixture-regression] discovers and copies a novel safe registered plan_ref without a filename allowlist",
  "[admission-smoke] v4 readiness remains non-authoritative for copied assignments",
  "rejects exact accepted-main base without the named integration ref",
  "v4 binds the workstation to its Product Increment and Delivery Lane without manufacturing a lease",
  "active assignment without live validation grants no source authority",
  "legacy v3 assignment remains context-only even when local lineage checks pass",
  "v4 context selection accepts assigned capability domains and fails closed otherwise",
  "legacy v3 assignment rejects a mismatched context",
  "copied static assignments cannot create two source authorities",
  "v4 selects a non-Nexus Product Increment from the portfolio",
  "[integration-regression] self-authored accepted-main evidence cannot award durable credit",
  "[fixture-regression] completing router review preserves an unrelated review lane",
  "v4 rejects capability domains that do not supply the selected lane",
  "expired active assignment grants no source authority",
  "assignment identities must exactly match the Delivery Lane authorities",
  "assignment economics cannot exceed the Delivery Lane budget or silently escalate",
  "ready lane is not counted or authorized as active source WIP",
  "context-only mode grants no source write authority",
  "blocked Product Increment cannot grant source authority",
  "invalid Product Increment plan cannot grant source authority",
  "portfolio plan traversal cannot enter the source-authority path",
  "unreachable capability evidence cannot authorize source",
  "invalid assignment emits no source roots",
  "rejects malformed Product Increment topology control",
  "rejects legacy workstation assignment schema v2",
  "v4 rejects an assignment to an unknown Delivery Lane",
  "v4 rejects an assignment whose immutable base differs from its Delivery Lane",
  "v4 rejects Delivery Lane roots reserved for Integration",
  "accepts exact local or agreeing local/remote named-integration tip",
  "accepts sibling main advancement and reports the unique accepted-main merge base",
  "rejects a base that is not an ancestor of producer HEAD",
  "rejects missing, mismatched, and drifting named integration refs distinctly",
  "rejects a matching remote-only integration ref",
  "rejects an unrelated integration lineage and similarly named containing ref",
  "rejects malformed integration branch input without resolving it",
  "rejects ambiguous criss-cross accepted-main merge bases",
]);

const MAX_SHARDS = 4;
const FIXTURE_PREFIX = "appfw-nexus-live-";
const sourceRoot = dirname(dirname(fileURLToPath(import.meta.url)));
const CANONICAL_TEST_RELATIVE_PATH = "scripts/check-nexus-workstreams.test.mjs";
const CANONICAL_TEST_FILE = join(sourceRoot, CANONICAL_TEST_RELATIVE_PATH);

function sha256Json(value) {
  return createHash("sha256")
    .update(`${JSON.stringify(value)}\n`)
    .digest("hex");
}

function captureSourceState() {
  const git = (...args) =>
    execFileSync("git", args, { cwd: sourceRoot, encoding: "utf8" }).trim();
  const status = execFileSync("git", ["status", "--porcelain=v1"], {
    cwd: sourceRoot,
    encoding: "utf8",
  });
  return {
    head: git("rev-parse", "HEAD"),
    tree: git("rev-parse", "HEAD^{tree}"),
    clean: status.length === 0,
    status_sha256: createHash("sha256").update(status).digest("hex"),
  };
}

function sha256File(path) {
  return createHash("sha256").update(readFileSync(path)).digest("hex");
}

function captureTestSourceIdentity(testFile) {
  const requestedPath = resolve(testFile);
  const requestedIsCanonical = requestedPath === CANONICAL_TEST_FILE;
  let trackedGitBlob = null;
  let executedGitBlob = null;
  let contentSha256 = null;
  let error = null;

  try {
    trackedGitBlob = execFileSync(
      "git",
      ["rev-parse", `HEAD:${CANONICAL_TEST_RELATIVE_PATH}`],
      { cwd: sourceRoot, encoding: "utf8" },
    ).trim();
    executedGitBlob = execFileSync("git", ["hash-object", "--no-filters", requestedPath], {
      cwd: sourceRoot,
      encoding: "utf8",
    }).trim();
    contentSha256 = sha256File(requestedPath);
  } catch (caught) {
    error = errorRecord(caught);
  }

  return {
    canonical_path: CANONICAL_TEST_RELATIVE_PATH,
    requested_path: requestedIsCanonical ? CANONICAL_TEST_RELATIVE_PATH : "noncanonical",
    requested_is_canonical: requestedIsCanonical,
    tracked_git_blob: trackedGitBlob,
    executed_git_blob: executedGitBlob,
    content_sha256: contentSha256,
    blob_bound:
      error === null &&
      requestedIsCanonical &&
      trackedGitBlob !== null &&
      trackedGitBlob === executedGitBlob,
    error,
  };
}

function assertUniqueNames(names, label) {
  assert(Array.isArray(names), `${label} must be an array`);
  assert(names.length > 0, `${label} must not be empty`);
  for (const name of names) {
    assert.equal(typeof name, "string", `${label} names must be strings`);
    assert(name.length > 0, `${label} names must not be empty`);
  }
  assert.equal(
    new Set(names).size,
    names.length,
    `${label} contains duplicate test names`,
  );
}

export function assertExactNexusWorkstreamTestManifest(actualNames) {
  assertUniqueNames(actualNames, "registered Nexus workstream tests");
  assert.deepEqual(
    actualNames,
    NEXUS_WORKSTREAM_TEST_MANIFEST,
    "registered Nexus workstream tests must exactly match the canonical manifest",
  );
}

export function partitionNexusWorkstreamTests(
  shardCount = MAX_SHARDS,
  manifest = NEXUS_WORKSTREAM_TEST_MANIFEST,
) {
  assertUniqueNames(manifest, "Nexus workstream test manifest");
  assert(
    Number.isInteger(shardCount) && shardCount >= 1 && shardCount <= MAX_SHARDS,
    `Nexus workstream shard count must be an integer from 1 through ${MAX_SHARDS}`,
  );
  const effectiveShardCount = Math.min(shardCount, manifest.length);
  const shards = Array.from({ length: effectiveShardCount }, () => []);
  manifest.forEach((name, index) => shards[index % effectiveShardCount].push(name));
  const union = shards.flat();
  assert.equal(union.length, manifest.length, "shard union omitted tests");
  assert.equal(new Set(union).size, manifest.length, "shard union duplicated tests");
  assert.deepEqual(
    new Set(union),
    new Set(manifest),
    "shard union differs from the canonical manifest",
  );
  return shards;
}

function summaryValue(tap, label) {
  const matches = [
    ...tap.matchAll(new RegExp(`^# ${label} (\\d+)$`, "gm")),
  ];
  assert.equal(matches.length, 1, `TAP must contain exactly one ${label} summary`);
  return Number.parseInt(matches[0][1], 10);
}

export function parseNexusWorkstreamShardTap(tap, expectedNames) {
  assertUniqueNames(expectedNames, "expected shard tests");
  assert.equal(typeof tap, "string", "shard TAP output must be a string");
  assert(tap.startsWith("TAP version 13\n"), "shard output is not TAP version 13");
  assert.doesNotMatch(
    tap,
    /^\s*Bail out!(?:\s|$)/m,
    "shard TAP contains a bailout",
  );

  const suiteName = "workstation assignment and Product Increment admission";
  const outerSubtests = [...tap.matchAll(/^# Subtest: (.+)$/gm)].map(
    (match) => match[1],
  );
  assert.deepEqual(
    outerSubtests,
    [suiteName],
    "shard TAP must contain exactly one canonical outer suite",
  );

  const outerResults = [...tap.matchAll(/^(?:ok|not ok)\b.*$/gm)].map(
    (match) => match[0],
  );
  assert.deepEqual(
    outerResults,
    [`ok 1 - ${suiteName}`],
    "shard TAP outer result records must exactly match plan 1..1",
  );

  const outerPlans = [...tap.matchAll(/^\d+\.\.\d+.*$/gm)].map(
    (match) => match[0],
  );
  assert.deepEqual(
    outerPlans,
    ["1..1"],
    "shard TAP must contain exactly one outer plan 1..1",
  );
  assert.doesNotMatch(tap, /^\s*not ok\b/m, "shard TAP contains a failing test");

  const subtests = [
    ...tap.matchAll(/^    # Subtest: (.+)$/gm),
  ].map((match) => match[1]);
  const passed = [
    ...tap.matchAll(/^    ok \d+ - (.+)$/gm),
  ].map((match) => match[1]);
  const innerPlans = [
    ...tap.matchAll(/^    1\.\.(\d+)$/gm),
  ].map((match) => Number.parseInt(match[1], 10));

  assert.deepEqual(subtests, expectedNames, "shard TAP subtests differ from its assignment");
  assert.deepEqual(passed, expectedNames, "shard TAP pass records differ from its assignment");
  assert.deepEqual(innerPlans, [expectedNames.length], "shard TAP has an invalid inner plan");

  const suiteHeaderOffset = tap.indexOf(`# Subtest: ${suiteName}`);
  const innerPlanOffset = tap.indexOf(`    1..${expectedNames.length}`);
  const outerResultOffset = tap.indexOf(`ok 1 - ${suiteName}`);
  const outerPlanOffset = tap.indexOf("\n1..1\n");
  assert(
    suiteHeaderOffset < innerPlanOffset &&
      innerPlanOffset < outerResultOffset &&
      outerResultOffset < outerPlanOffset,
    "shard TAP outer suite, result, and plan are not in canonical order",
  );
  assert.equal(summaryValue(tap, "tests"), expectedNames.length);
  assert.equal(summaryValue(tap, "suites"), 1);
  assert.equal(summaryValue(tap, "pass"), expectedNames.length);
  assert.equal(summaryValue(tap, "fail"), 0);
  assert.equal(summaryValue(tap, "cancelled"), 0);
  assert.equal(summaryValue(tap, "skipped"), 0);
  assert.equal(summaryValue(tap, "todo"), 0);
  return passed;
}

export function validateNexusWorkstreamShardResult(result, expectedNames) {
  assert.equal(result.error, undefined, result.error?.message);
  assert.equal(
    result.signal,
    null,
    `Nexus workstream shard terminated by signal ${result.signal}`,
  );
  assert.equal(
    result.status,
    0,
    `Nexus workstream shard exited with status ${result.status}`,
  );
  return parseNexusWorkstreamShardTap(result.stdout, expectedNames);
}

function escapePattern(value) {
  return value.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
}

function runShard({ names, index, testFile, runId }) {
  const pattern = `^(?:${names.map(escapePattern).join("|")})$`;
  const child = spawn(
    process.execPath,
    [
      "--test",
      "--test-concurrency=1",
      "--test-reporter=tap",
      `--test-name-pattern=${pattern}`,
      testFile,
    ],
    {
      cwd: sourceRoot,
      env: {
        ...process.env,
        APPFW_NEXUS_WORKSTREAM_TEST_RUN_ID: runId,
      },
      stdio: ["ignore", "pipe", "pipe"],
    },
  );
  const stdout = [];
  const stderr = [];
  child.stdout.on("data", (chunk) => stdout.push(chunk));
  child.stderr.on("data", (chunk) => stderr.push(chunk));
  return new Promise((resolveResult) => {
    child.on("error", (error) => {
      resolveResult({ index, names, status: null, signal: null, error, stdout: "", stderr: "" });
    });
    child.on("close", (status, signal) => {
      resolveResult({
        index,
        names,
        status,
        signal,
        error: undefined,
        stdout: Buffer.concat(stdout).toString("utf8"),
        stderr: Buffer.concat(stderr).toString("utf8"),
      });
    });
  });
}

function runFixtureNames(runId) {
  const prefix = `${FIXTURE_PREFIX}${runId}-`;
  return readdirSync(tmpdir())
    .filter((name) => name.startsWith(prefix))
    .sort();
}

function removeRunFixtures(runId) {
  const errors = [];
  for (const name of runFixtureNames(runId)) {
    try {
      rmSync(join(tmpdir(), name), { recursive: true, force: true });
    } catch (error) {
      errors.push(new Error(`failed to remove ${name}: ${error.message}`));
    }
  }
  if (errors.length > 0) {
    throw new AggregateError(errors, "Nexus workstream fixture cleanup failed");
  }
}

async function executeShardSet({ shards, testFile, runId }) {
  return Promise.all(
    shards.map((names, index) => runShard({ names, index, testFile, runId })),
  );
}

function atomicWriteSummary(runLogRoot, summary) {
  const summaryPath = join(runLogRoot, "summary.json");
  const temporaryPath = join(
    runLogRoot,
    `.summary-${process.pid}-${randomUUID()}.tmp`,
  );
  writeFileSync(temporaryPath, `${JSON.stringify(summary, null, 2)}\n`, {
    flag: "wx",
  });
  renameSync(temporaryPath, summaryPath);
  return summaryPath;
}

function errorRecord(error) {
  if (!error) return null;
  return {
    name: error.name ?? "Error",
    message: error.message ?? String(error),
    errors:
      error instanceof AggregateError
        ? error.errors.map((nested) => errorRecord(nested))
        : undefined,
  };
}

function aggregateErrors(label, errors) {
  const present = errors.filter(Boolean);
  if (present.length === 0) return null;
  if (present.length === 1) return present[0];
  return new AggregateError(present, label);
}

function configuredShardCount() {
  const raw = process.env.APPFW_NEXUS_WORKSTREAM_TEST_SHARDS ?? String(MAX_SHARDS);
  assert.match(raw, /^[1-4]$/, `APPFW_NEXUS_WORKSTREAM_TEST_SHARDS must be 1 through ${MAX_SHARDS}`);
  return Number.parseInt(raw, 10);
}

export async function runNexusWorkstreamTestShards({
  shardCount = configuredShardCount(),
  logRoot = join(sourceRoot, "target", "appfw", "nexus-workstream-test-shards"),
  testFile = CANONICAL_TEST_FILE,
  lifecycleTestHooks = {},
} = {}) {
  assert(
    lifecycleTestHooks !== null &&
      (typeof lifecycleTestHooks === "object" ||
        typeof lifecycleTestHooks === "function"),
    "lifecycleTestHooks must be an object",
  );
  const allowedHookNames = [
    "executeShards",
    "cleanupRunFixtures",
    "captureSourceState",
  ];
  const allowedHookNameSet = new Set(allowedHookNames);
  for (const hookName of Reflect.ownKeys(lifecycleTestHooks)) {
    assert(
      typeof hookName === "string" && allowedHookNameSet.has(hookName),
      `unknown lifecycle test hook: ${String(hookName)}`,
    );
  }
  // Snapshot every supported property exactly once. This catches inherited and
  // non-enumerable hooks, and prevents a getter or Proxy from changing the
  // authority decision after it has been made.
  const lifecycleHookSnapshot = Object.fromEntries(
    allowedHookNames.map((hookName) => {
      const hook = Reflect.get(lifecycleTestHooks, hookName);
      assert(
        hook === undefined || typeof hook === "function",
        `lifecycle test hook must be a function: ${hookName}`,
      );
      return [hookName, hook];
    }),
  );
  const lifecycleHookNames = allowedHookNames
    .filter((hookName) => lifecycleHookSnapshot[hookName] !== undefined)
    .sort();
  const testSource = captureTestSourceIdentity(testFile);
  const authorityReasons = [];
  if (lifecycleHookNames.length > 0) {
    authorityReasons.push("lifecycle_test_hooks_active");
  }
  if (!testSource.requested_is_canonical) {
    authorityReasons.push("noncanonical_test_file");
  } else if (!testSource.blob_bound) {
    authorityReasons.push("canonical_test_blob_not_bound_to_head");
  }
  const authoritative = authorityReasons.length === 0;
  const executeShards = lifecycleHookSnapshot.executeShards ?? executeShardSet;
  const cleanupRunFixtures =
    lifecycleHookSnapshot.cleanupRunFixtures ?? removeRunFixtures;
  const readSourceState =
    lifecycleHookSnapshot.captureSourceState ?? captureSourceState;
  const shards = partitionNexusWorkstreamTests(shardCount);
  const timestamp = new Date().toISOString().replaceAll(/[-:.]/g, "");
  const runId = `${timestamp}-${process.pid}-${randomUUID().slice(0, 8)}`;
  const runLogRoot = join(logRoot, runId);
  mkdirSync(runLogRoot, { recursive: true });
  const started = performance.now();
  const canonicalManifestSha256 = sha256Json(NEXUS_WORKSTREAM_TEST_MANIFEST);
  const initialFixtures = runFixtureNames(runId);
  let sourceStart = null;
  let sourceEnd = null;
  let results = [];
  let observed = [];
  let executionError = null;
  let cleanupError = null;
  let sourceBindingError = null;

  // Execute every assigned shard and retain its raw logs before terminal cleanup.
  try {
    sourceStart = readSourceState();
    results = await executeShards({ shards, testFile, runId });
    for (const result of results) {
      writeFileSync(join(runLogRoot, `shard-${result.index + 1}.tap`), result.stdout);
      writeFileSync(join(runLogRoot, `shard-${result.index + 1}.stderr.log`), result.stderr);
    }

    observed = results.flatMap((result) =>
      validateNexusWorkstreamShardResult(result, result.names),
    );
    assert.equal(observed.length, NEXUS_WORKSTREAM_TEST_MANIFEST.length);
    assert.equal(new Set(observed).size, observed.length, "observed shard results contain duplicates");
    assert.deepEqual(
      new Set(observed),
      new Set(NEXUS_WORKSTREAM_TEST_MANIFEST),
      "observed shard results omitted canonical tests",
    );
  } catch (error) {
    executionError = error;
  }

  // Cleanup is terminal: inventory, attempt every run-owned root, then inventory again.
  let cleanupBefore = [];
  let cleanupAfter = [];
  try {
    cleanupBefore = runFixtureNames(runId);
  } catch (error) {
    cleanupError = aggregateErrors("Nexus workstream fixture inventory failed", [
      cleanupError,
      error,
    ]);
  }
  try {
    cleanupRunFixtures(runId);
  } catch (error) {
    cleanupError = aggregateErrors("Nexus workstream fixture cleanup failed", [
      cleanupError,
      error,
    ]);
  }
  try {
    cleanupAfter = runFixtureNames(runId);
    if (cleanupAfter.length > 0) {
      throw new Error(
        `Nexus workstream test fixtures remain after cleanup: ${cleanupAfter.join(", ")}`,
      );
    }
  } catch (error) {
    cleanupError = aggregateErrors("Nexus workstream fixture cleanup failed", [
      cleanupError,
      error,
    ]);
  }

  // Bind the terminal verdict to the same source identity seen before execution.
  try {
    sourceEnd = readSourceState();
    assert(sourceStart, "source state was not captured before shard execution");
    assert.equal(sourceEnd.head, sourceStart.head, "source HEAD changed during shard execution");
    assert.equal(sourceEnd.tree, sourceStart.tree, "source tree changed during shard execution");
    assert.equal(
      sourceEnd.status_sha256,
      sourceStart.status_sha256,
      "source worktree state changed during shard execution",
    );
  } catch (error) {
    sourceBindingError = error;
  }

  const sourceStable = sourceBindingError === null;
  const exactCandidateBound =
    authoritative &&
    sourceStable &&
    sourceStart?.clean === true &&
    sourceEnd?.clean === true;
  const cleanupOk = cleanupError === null && cleanupAfter.length === 0;
  const terminalOk =
    executionError === null && cleanupOk && sourceBindingError === null;
  const observedSet = new Set(observed);
  const observedUnion = NEXUS_WORKSTREAM_TEST_MANIFEST.filter((name) =>
    observedSet.has(name),
  );
  const summary = {
    schema: "appfw_nexus_workstream_test_shards@2",
    finalized: true,
    ok: terminalOk,
    authoritative,
    exact_candidate_bound: exactCandidateBound,
    run_id: runId,
    shard_count: shards.length,
    max_shards: MAX_SHARDS,
    test_count: observedUnion.length,
    expected_test_count: NEXUS_WORKSTREAM_TEST_MANIFEST.length,
    duration_ms: Math.round((performance.now() - started) * 1000) / 1000,
    log_root: runLogRoot,
    source: {
      start: sourceStart,
      end: sourceEnd,
      stable: sourceStable,
    },
    authority: {
      mode: authoritative ? "canonical_native" : "non_authoritative",
      reasons: authorityReasons,
      lifecycle_test_hooks: lifecycleHookNames,
      test_source: testSource,
    },
    canonical_manifest: {
      test_count: NEXUS_WORKSTREAM_TEST_MANIFEST.length,
      sha256: canonicalManifestSha256,
    },
    observed_union: {
      test_count: observedUnion.length,
      sha256: sha256Json(observedUnion),
    },
    cleanup: {
      initial: initialFixtures,
      before: cleanupBefore,
      after: cleanupAfter,
      ok: cleanupOk,
      error: errorRecord(cleanupError),
    },
    errors: {
      execution: errorRecord(executionError),
      cleanup: errorRecord(cleanupError),
      source_binding: errorRecord(sourceBindingError),
    },
    shards: results.map((result) => ({
      index: result.index + 1,
      test_count: result.names.length,
      status: result.status,
      signal: result.signal,
      tap_log: join(runLogRoot, `shard-${result.index + 1}.tap`),
      stderr_log: join(runLogRoot, `shard-${result.index + 1}.stderr.log`),
    })),
  };

  // Publish exactly once, atomically, only after cleanup and the terminal verdict.
  atomicWriteSummary(runLogRoot, summary);
  if (!terminalOk) {
    throw new AggregateError(
      [executionError, cleanupError, sourceBindingError].filter(Boolean),
      "Nexus workstream sharded test runner failed terminal lifecycle checks",
    );
  }
  return summary;
}

const isMain = process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url);
if (isMain) {
  try {
    const summary = await runNexusWorkstreamTestShards();
    process.stdout.write(`${JSON.stringify(summary, null, 2)}\n`);
  } catch (error) {
    process.stderr.write(`Nexus workstream sharded test runner failed: ${error.stack ?? error.message}\n`);
    process.exitCode = 1;
  }
}
