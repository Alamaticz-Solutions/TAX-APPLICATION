import assert from "node:assert/strict";
import crypto from "node:crypto";
import { execFileSync, spawnSync } from "node:child_process";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import test from "node:test";
import { fileURLToPath } from "node:url";

const scriptDirectory = path.dirname(fileURLToPath(import.meta.url));
const repositoryRoot = path.resolve(scriptDirectory, "../..");
const packager = path.join(scriptDirectory, "package-pr-fast-evidence.py");
const freshnessCheck = path.join(scriptDirectory, "pr-destination-freshness.py");
const releaseLiteGuard = path.join(scriptDirectory, "release-lite-guard.sh");
const BITBUCKET_IDENTITY_KEYS = [
  "BITBUCKET_BUILD_NUMBER",
  "BITBUCKET_COMMIT",
  "BITBUCKET_PIPELINE_UUID",
  "BITBUCKET_PR_DESTINATION_BRANCH",
  "BITBUCKET_PR_DESTINATION_COMMIT",
  "BITBUCKET_REPO_FULL_NAME",
  "BITBUCKET_STEP_UUID",
];

function sha256(file) {
  return crypto.createHash("sha256").update(fs.readFileSync(file)).digest("hex");
}

function anchorBlock(yaml, anchor, nextAnchor) {
  const start = yaml.indexOf(`&${anchor}`);
  assert.notEqual(start, -1, `missing pipeline anchor &${anchor}`);
  const end = nextAnchor ? yaml.indexOf(`&${nextAnchor}`, start) : yaml.length;
  assert.notEqual(end, -1, `missing following pipeline anchor &${nextAnchor}`);
  return yaml.slice(start, end);
}

function writeJson(root, relative, payload) {
  const file = path.join(root, relative);
  fs.mkdirSync(path.dirname(file), { recursive: true });
  fs.writeFileSync(file, `${JSON.stringify(payload, null, 2)}\n`);
}

function writeFixtureEvidence(root, testedSha) {
  writeJson(root, "target/appfw/ci-evidence-root.json", {
    command: "prepare-appfw-evidence-root",
    ok: true,
    git: { commit: testedSha },
  });
  const productReportRoot = "examples/products/crm/.appfw/target/appfw";
  const personalPathSentinel = "/Users/fixture-owner/private-product-worktree";
  writeJson(root, `${productReportRoot}/validation.json`, {
    valid: true,
    roots: { app_root: personalPathSentinel },
  });
  writeJson(root, `${productReportRoot}/app_topology.json`, {
    manifest_path: `${personalPathSentinel}/.appfw/manifest.yaml`,
  });
  writeJson(root, `${productReportRoot}/config_contract.json`, {
    schema: "fixture-config-contract@1",
  });
  writeJson(root, `${productReportRoot}/sync_descriptors.json`, {
    schema: "fixture-sync-descriptors@1",
  });
  writeJson(root, "target/appfw/cli-test.json", { command: "cli-test", ok: true });
  writeJson(root, "target/appfw/golden-downstream.json", {
    command: "golden-downstream",
    ok: true,
  });
  writeJson(root, "target/appfw/docs-check-timing.json", {
    command: "docs-check",
    ok: true,
    budget_ok: true,
    budget_enforced: true,
  });
  writeJson(root, "target/appfw/docs-check-changed-surface.json", {
    command: "docs-check-changed-surface",
  });
  writeJson(root, "target/appfw/wave3-pr-gates.json", {
    command: "wave3-pr-gates",
    ok: true,
  });
  fs.mkdirSync(path.join(root, "target/appfw/wave3-pr-gates"), { recursive: true });
  fs.writeFileSync(
    path.join(root, "target/appfw/wave3-pr-gates/product-spa-build.stdout.log"),
    "fixture diagnostic\n",
  );
  fs.writeFileSync(path.join(root, "target/appfw/cli-test-shell-appfw.log"), "ok\n");

  // These were carried by target/appfw/**. None is retained by the allowlist.
  fs.mkdirSync(path.join(root, "target/appfw/npm-cache"), { recursive: true });
  fs.writeFileSync(path.join(root, "target/appfw/npm-cache/cache.json"), "{}\n");
  fs.mkdirSync(path.join(root, "target/appfw/packages"), { recursive: true });
  fs.writeFileSync(path.join(root, "target/appfw/packages/application.tgz"), "package\n");
  fs.writeFileSync(path.join(root, "target/appfw/generated-workspace.bin"), "binary\n");
}

function createGitTransportShim(context) {
  const root = fs.mkdtempSync(path.join(os.tmpdir(), "appfw-pr-git-transport-"));
  const shim = path.join(root, "git");
  const realGit = execFileSync("sh", ["-c", "command -v git"], {
    encoding: "utf8",
  }).trim();
  fs.writeFileSync(
    shim,
    [
      "#!/usr/bin/env bash",
      "set -euo pipefail",
      'if [[ "${1:-}" == "fetch" || "${1:-}" == "ls-remote" ]]; then',
      '  [[ "${GIT_TERMINAL_PROMPT:-}" == "0" ]] || exit 38',
      '  case "${APPFW_TEST_GIT_TRANSPORT_MODE:-}" in',
      '    hang) exec sleep "${APPFW_TEST_GIT_HANG_SECONDS:-30}" ;;',
      "    fail) exit 37 ;;",
      "  esac",
      "fi",
      'exec "${APPFW_TEST_REAL_GIT:?}" "$@"',
      "",
    ].join("\n"),
  );
  fs.chmodSync(shim, 0o755);
  context.after(() => fs.rmSync(root, { recursive: true, force: true }));
  return {
    environment: {
      APPFW_TEST_REAL_GIT: realGit,
      PATH: `${root}${path.delimiter}${process.env.PATH}`,
    },
  };
}

function initFixtureRepository(prefix = "appfw-pr-fast-evidence-") {
  const root = fs.mkdtempSync(path.join(os.tmpdir(), prefix));
  execFileSync("git", ["init", "-q"], { cwd: root });
  execFileSync("git", ["config", "user.email", "fixture@example.invalid"], { cwd: root });
  execFileSync("git", ["config", "user.name", "PR Fast Fixture"], { cwd: root });
  fs.writeFileSync(path.join(root, ".gitignore"), "target/\n.appfw/target/\n");
  execFileSync("git", ["add", ".gitignore"], { cwd: root });
  execFileSync("git", ["commit", "-qm", "fixture"], { cwd: root });
  return root;
}

function installReleaseLiteGuard(root) {
  const destination = path.join(root, "scripts/ci/release-lite-guard.sh");
  fs.mkdirSync(path.dirname(destination), { recursive: true });
  fs.copyFileSync(releaseLiteGuard, destination);
  fs.chmodSync(destination, 0o755);
  return destination;
}

function createFixture() {
  const root = initFixtureRepository();
  const sourceSha = execFileSync("git", ["rev-parse", "HEAD"], {
    cwd: root,
    encoding: "utf8",
  }).trim();
  writeFixtureEvidence(root, sourceSha);
  return { root, sourceSha };
}

function createSyntheticMergeFixture() {
  const root = initFixtureRepository("appfw-pr-fast-synthetic-merge-");
  execFileSync("git", ["branch", "destination"], { cwd: root });
  execFileSync("git", ["checkout", "-qb", "source"], { cwd: root });
  fs.writeFileSync(path.join(root, "source.txt"), "source\n");
  execFileSync("git", ["add", "source.txt"], { cwd: root });
  execFileSync("git", ["commit", "-qm", "source"], { cwd: root });
  const sourceSha = execFileSync("git", ["rev-parse", "HEAD"], {
    cwd: root,
    encoding: "utf8",
  }).trim();
  execFileSync("git", ["checkout", "-q", "destination"], { cwd: root });
  fs.writeFileSync(path.join(root, "destination.txt"), "destination\n");
  execFileSync("git", ["add", "destination.txt"], { cwd: root });
  execFileSync("git", ["commit", "-qm", "destination"], { cwd: root });
  const destinationSha = execFileSync("git", ["rev-parse", "HEAD"], {
    cwd: root,
    encoding: "utf8",
  }).trim();
  execFileSync("git", ["checkout", "-q", "source"], { cwd: root });
  execFileSync("git", ["merge", "--no-ff", "-qm", "synthetic merge", "destination"], {
    cwd: root,
  });
  const testedSha = execFileSync("git", ["rev-parse", "HEAD"], {
    cwd: root,
    encoding: "utf8",
  }).trim();
  writeFixtureEvidence(root, testedSha);
  return { root, sourceSha, destinationSha, testedSha };
}

function runPackager(root, sourceSha, extraEnvironment = {}, gateExitStatus = 0) {
  const environment = { ...process.env };
  for (const key of BITBUCKET_IDENTITY_KEYS) {
    delete environment[key];
  }
  return spawnSync(
    "python3",
    [packager, "--repo-root", root, "--gate-exit-status", String(gateExitStatus)],
    {
      cwd: root,
      encoding: "utf8",
      env: { ...environment, BITBUCKET_COMMIT: sourceSha, ...extraEnvironment },
    },
  );
}

function assertDestinationIdentityHelper(script, fullSha, suppliedId, expected) {
  const probe = [
    "import importlib.util, sys",
    "spec = importlib.util.spec_from_file_location('identity_probe', sys.argv[1])",
    "module = importlib.util.module_from_spec(spec)",
    "spec.loader.exec_module(module)",
    "print('true' if module.destination_identity_matches(sys.argv[2], sys.argv[3]) else 'false')",
  ].join("; ");
  const result = spawnSync("python3", ["-c", probe, script, fullSha, suppliedId], {
    encoding: "utf8",
  });
  assert.equal(result.status, 0, `${result.stdout}\n${result.stderr}`);
  assert.equal(result.stdout.trim(), expected ? "true" : "false");
}

test("pipeline retains every assurance step while bounding and parallelizing PR producers", () => {
  const yaml = fs.readFileSync(path.join(repositoryRoot, "bitbucket-pipelines.yml"), "utf8");
  const fast = anchorBlock(yaml, "fast-framework-check", "supply-chain-gate");
  const supply = anchorBlock(yaml, "supply-chain-gate", "secret-scan");
  const secret = anchorBlock(yaml, "secret-scan", "release-lite-guard");
  const releaseLite = anchorBlock(yaml, "release-lite-guard", "release-lite-closing-guard");
  const closingGuardDefinition = anchorBlock(
    yaml,
    "release-lite-closing-guard",
    "release-gate",
  );
  const releaseGate = anchorBlock(yaml, "release-gate", "publish-proget");
  const publish = anchorBlock(yaml, "publish-proget", "release-gate-focused");
  const focused = anchorBlock(yaml, "release-gate-focused", null);

  assert.match(fast, /artifacts:\s+download: false\s+upload:/);
  assert.match(fast, /name: pr-fast-evidence/);
  assert.match(fast, /type: shared/);
  assert.match(fast, /target\/appfw\/pr-fast-evidence-manifest\.json/);
  assert.match(fast, /target\/appfw\/pr-fast-evidence\/\*\*/);
  assert.match(fast, /capture-on: always/);
  assert.doesNotMatch(fast, /^\s*- target\/appfw\/\*\*\s*$/m);

  for (const [name, block] of [
    ["supply-chain", supply],
    ["secret-scan", secret],
    ["release-lite", releaseLite],
    ["closing-guard", closingGuardDefinition],
  ]) {
    assert.match(block, /artifacts:\s+download: false/, `${name} must skip inherited artifacts`);
  }
  assert.match(supply, /prepare-appfw-evidence-root\.sh/);
  assert.match(secret, /prepare-appfw-evidence-root\.sh/);
  assert.doesNotMatch(releaseGate, /download: false/);
  assert.doesNotMatch(focused, /download: false/);
  assert.doesNotMatch(publish, /download: false/);
  assert.match(closingGuardDefinition, /pr-destination-freshness\.py/);
  assert.match(
    closingGuardDefinition,
    /target\/appfw\/pr-destination-freshness\.json/,
  );
  assert.match(releaseLite, /max-time: 5/);
  assert.match(closingGuardDefinition, /max-time: 5/);
  assert.match(
    releaseLite,
    /APPFW_PR_REMOTE_GIT_TIMEOUT_SECONDS=240 bash scripts\/ci\/release-lite-guard\.sh/,
  );
  assert.match(releaseLite, /name: pr-release-lite-opening-evidence/);
  assert.match(releaseLite, /type: scoped/);
  assert.match(releaseLite, /capture-on: always/);
  assert.match(
    closingGuardDefinition,
    /APPFW_PR_REMOTE_GIT_TIMEOUT_SECONDS=120 bash scripts\/ci\/release-lite-guard\.sh/,
  );
  assert.match(
    closingGuardDefinition,
    /APPFW_PR_REMOTE_GIT_TIMEOUT_SECONDS=120 python3 scripts\/ci\/pr-destination-freshness\.py/,
  );
  assert.match(closingGuardDefinition, /name: pr-release-lite-closing-evidence/);
  assert.match(closingGuardDefinition, /type: scoped/);
  assert.match(closingGuardDefinition, /capture-on: always/);
  const closingRemoteTimeouts = [
    ...closingGuardDefinition.matchAll(/APPFW_PR_REMOTE_GIT_TIMEOUT_SECONDS=(\d+)/g),
  ].map((match) => Number.parseInt(match[1], 10));
  assert.deepEqual(closingRemoteTimeouts, [120, 120]);
  assert.ok(
    closingRemoteTimeouts.reduce((total, value) => total + value, 0) < 5 * 60,
    "serial closing-guard remote waits must fit below the five-minute step backstop",
  );

  const pullRequestLane = yaml.slice(yaml.indexOf("pull-requests:"), yaml.indexOf("  branches:"));
  for (const reference of [
    "*release-lite-guard",
    "*release-lite-closing-guard",
    "*fast-framework-check",
    "*supply-chain-gate",
    "*secret-scan",
  ]) {
    assert.match(pullRequestLane, new RegExp(reference.replace("*", "\\*")));
  }
  assert.equal((pullRequestLane.match(/\*release-lite-guard/g) || []).length, 1);
  assert.equal((pullRequestLane.match(/\*release-lite-closing-guard/g) || []).length, 1);
  assert.equal((pullRequestLane.match(/\*fast-framework-check/g) || []).length, 1);
  assert.equal((pullRequestLane.match(/\*supply-chain-gate/g) || []).length, 1);
  assert.equal((pullRequestLane.match(/\*secret-scan/g) || []).length, 1);
  assert.match(
    pullRequestLane,
    /- parallel:\s+fail-fast: true\s+steps:\s+- step: \*fast-framework-check\s+- step: \*supply-chain-gate\s+- step: \*secret-scan/,
    "independent Fast, supply-chain, and secret-scan producers must share one fail-fast PR parallel block",
  );
  const openingGuard = pullRequestLane.indexOf("*release-lite-guard");
  const producerParallel = pullRequestLane.indexOf("- parallel:", openingGuard);
  const closingGuard = pullRequestLane.lastIndexOf("*release-lite-closing-guard");
  assert.ok(openingGuard < producerParallel, "preflight guard must run before producer parallelism");
  assert.ok(producerParallel < closingGuard, "closing guard must wait for all PR producers");

  const wrapper = fs.readFileSync(
    path.join(repositoryRoot, "scripts/ci/pr-fast-framework-check.sh"),
    "utf8",
  );
  const orderedCommands = [
    "scripts/appfw framework validate --json",
    "scripts/appfw framework cli-test --json",
    "scripts/appfw framework golden-downstream --profile second-consumer --execute --json",
    "scripts/appfw docs-check --changed-only --json --progress --enforce-budget",
    'bash "$script_dir/wave3-pr-gates.sh" --reuse-docs-check',
    "scripts/appfw test --fast --json",
  ];
  let previous = -1;
  for (const command of orderedCommands) {
    const index = wrapper.indexOf(command);
    assert.ok(index > previous, `missing or reordered Fast gate: ${command}`);
    previous = index;
  }
  assert.match(wrapper, /trap finalize_pr_fast_evidence EXIT/);
  assert.match(wrapper, /package-pr-fast-evidence\.py/);
  assert.match(wrapper, /node --test scripts\/ci\/pr-fast-evidence-contract\.test\.mjs/);

  const releaseGateScript = fs.readFileSync(
    path.join(repositoryRoot, "scripts/ci/bitbucket-release-gate.sh"),
    "utf8",
  );
  assert.match(releaseGateScript, /report_dir \/ "supply-chain-gate\.json"/);
  assert.match(releaseGateScript, /report_dir \/ "secret-scan\.json"/);
  assert.match(releaseGateScript, /load_retained_json\("supply-chain gate"/);
  assert.match(releaseGateScript, /load_retained_json\("secret scan"/);

  const publishScript = fs.readFileSync(
    path.join(repositoryRoot, "scripts/ci/proget-publish.sh"),
    "utf8",
  );
  assert.match(publishScript, /consumes the release-gate evidence downloaded from the prior/);
  assert.match(publishScript, /must NOT call prepare-appfw-evidence-root\.sh/);
  assert.match(publishScript, /bitbucket-release-gate\.json/);
});

test("release gate uses the checksum-pinned PR Fast Node runtime", () => {
  const releaseGateScript = fs.readFileSync(
    path.join(repositoryRoot, "scripts/ci/bitbucket-release-gate.sh"),
    "utf8",
  );
  const installNodeScript = fs.readFileSync(
    path.join(repositoryRoot, "scripts/ci/install-node.sh"),
    "utf8",
  );
  const releaseVersion = releaseGateScript.match(
    /required_version="\$\{APPFW_NODE_VERSION:-([^}]+)\}"/,
  )?.[1];
  const installerVersion = installNodeScript.match(
    /node_version="\$\{APPFW_NODE_VERSION:-([^}]+)\}"/,
  )?.[1];

  assert.ok(releaseVersion, "release gate must declare an exact default Node version");
  assert.equal(
    releaseVersion,
    installerVersion,
    "release gate and checksum-verified installer defaults must not drift",
  );
  assert.match(
    releaseGateScript,
    /bash "\$script_dir\/install-node\.sh" \|\| return "\$\?"/,
    "release gate must propagate installer failure even when its stage runner disables errexit",
  );
  assert.match(
    releaseGateScript,
    /ensure_frontend_node_runtime \|\| return "\$\?"/,
    "CI prerequisite stage must propagate the guarded runtime installer failure",
  );
  assert.match(releaseGateScript, /"\$current_version" != "\$required_version"/);
  assert.match(releaseGateScript, /xz-utils/);
  assert.match(
    installNodeScript,
    /install_dir="\$\{APPFW_NODE_INSTALL_DIR:-\/usr\/local\}"/,
    "root release jobs must retain the checksum installer's /usr/local default",
  );
  assert.match(
    releaseGateScript,
    /APPFW_NODE_INSTALL_DIR="\$\{APPFW_NODE_INSTALL_DIR:-\$repo_root\/target\/appfw-tools\/node\}"/,
    "non-root release jobs must use the repository-local installer path",
  );
  const installerCall = releaseGateScript.indexOf('bash "$script_dir/install-node.sh"');
  const runtimeCheck = releaseGateScript.indexOf(
    'current_version="$(node_runtime_version)"',
  );
  assert.ok(
    installerCall >= 0 && installerCall < runtimeCheck,
    "release runtime identity must be checked after the verified installer runs",
  );
  assert.doesNotMatch(releaseGateScript, /deb\.nodesource\.com/);
  assert.doesNotMatch(releaseGateScript, /APPFW_RELEASE_FRONTEND_NODE_MAJOR/);
});

test("packager retains only hash-bound exact-SHA review evidence", (context) => {
  const { root, sourceSha } = createFixture();
  context.after(() => fs.rmSync(root, { recursive: true, force: true }));
  const result = runPackager(root, sourceSha);
  assert.equal(result.status, 0, `${result.stdout}\n${result.stderr}`);

  const manifestPath = path.join(root, "target/appfw/pr-fast-evidence-manifest.json");
  const manifest = JSON.parse(fs.readFileSync(manifestPath, "utf8"));
  assert.equal(manifest.schema, "appfw_pr_fast_evidence_manifest@1");
  assert.equal(manifest.ok, true);
  assert.equal(manifest.source_sha, sourceSha);
  assert.equal(manifest.tested_sha, sourceSha);
  assert.equal(manifest.source_worktree_clean, true);
  assert.equal(manifest.bitbucket.bitbucket_commit, sourceSha);
  assert.equal(manifest.size.under_limit, true);
  assert.equal(manifest.size.maximum_uncompressed_bytes, 100 * 1024 * 1024);
  assert.equal(manifest.size.compressed_bytes, null);
  assert.equal(manifest.size.remote_upload_seconds, null);

  const sourcePaths = new Set(manifest.artifacts.map((item) => item.source_path));
  assert.ok(sourcePaths.has("target/appfw/cli-test.json"));
  assert.ok(sourcePaths.has("target/appfw/cli-test-shell-appfw.log"));
  assert.ok(
    sourcePaths.has("target/appfw/wave3-pr-gates/product-spa-build.stdout.log"),
  );
  assert.deepEqual(manifest.allowlist.product_report_files, []);
  assert.ok(
    manifest.assurance.required_reports_checked_on_success.includes(
      "examples/products/crm/.appfw/target/appfw/validation.json",
    ),
  );
  assert.ok(
    [...sourcePaths].every(
      (sourcePath) =>
        !sourcePath.startsWith("examples/products/crm/.appfw/target/appfw/"),
    ),
  );
  assert.doesNotMatch(JSON.stringify(manifest), /\/Users\/fixture-owner\//);
  assert.ok(!sourcePaths.has("target/appfw/npm-cache/cache.json"));
  assert.ok(!sourcePaths.has("target/appfw/packages/application.tgz"));
  assert.ok(!sourcePaths.has("target/appfw/generated-workspace.bin"));

  let summedBytes = 0;
  for (const artifact of manifest.artifacts) {
    const source = path.join(root, artifact.source_path);
    const staged = path.join(root, artifact.artifact_path);
    assert.ok(fs.statSync(staged).isFile());
    assert.equal(artifact.sha256, sha256(source));
    assert.equal(artifact.sha256, sha256(staged));
    assert.equal(artifact.bytes, fs.statSync(source).size);
    assert.doesNotMatch(fs.readFileSync(staged, "utf8"), /\/Users\/fixture-owner\//);
    summedBytes += artifact.bytes;
  }
  assert.equal(manifest.size.retained_file_bytes, summedBytes);
  assert.equal(manifest.size.manifest_bytes, fs.statSync(manifestPath).size);
  assert.equal(
    manifest.size.total_uncompressed_bytes,
    summedBytes + fs.statSync(manifestPath).size,
  );
});

test("packager accepts an exact two-parent Bitbucket synthetic merge", (context) => {
  const { root, sourceSha, destinationSha, testedSha } = createSyntheticMergeFixture();
  context.after(() => fs.rmSync(root, { recursive: true, force: true }));
  const result = runPackager(root, sourceSha, {
    BITBUCKET_PR_DESTINATION_BRANCH: "main",
    BITBUCKET_PR_DESTINATION_COMMIT: destinationSha.slice(0, 12),
  });
  assert.equal(result.status, 0, `${result.stdout}\n${result.stderr}`);
  const manifest = JSON.parse(
    fs.readFileSync(path.join(root, "target/appfw/pr-fast-evidence-manifest.json"), "utf8"),
  );
  assert.equal(manifest.source_sha, sourceSha);
  assert.equal(manifest.tested_sha, testedSha);
  assert.equal(manifest.destination_sha, destinationSha);
  assert.equal(
    manifest.tested_commit_relation,
    "bitbucket-tested-merge-of-source-and-destination",
  );
});

test("destination identity allows only a 12-character prefix or exact full ID", () => {
  const sha40 = "a".repeat(40);
  const sha64 = `${"b".repeat(40)}${"c".repeat(24)}`;
  for (const script of [packager, freshnessCheck]) {
    assertDestinationIdentityHelper(script, sha40, sha40.slice(0, 12), true);
    assertDestinationIdentityHelper(script, sha40, sha40, true);
    assertDestinationIdentityHelper(script, sha64, sha64.slice(0, 12), true);
    assertDestinationIdentityHelper(script, sha64, sha64, true);
    assertDestinationIdentityHelper(script, sha64, sha64.slice(0, 40), false);
  }
});

test("packager fails closed when a required Fast report is missing", (context) => {
  const { root, sourceSha } = createFixture();
  context.after(() => fs.rmSync(root, { recursive: true, force: true }));
  fs.rmSync(path.join(root, "target/appfw/cli-test.json"));

  const result = runPackager(root, sourceSha);
  assert.notEqual(result.status, 0);
  const manifest = JSON.parse(
    fs.readFileSync(path.join(root, "target/appfw/pr-fast-evidence-manifest.json"), "utf8"),
  );
  assert.equal(manifest.ok, false);
  assert.ok(
    manifest.root_causes.includes("missing required Fast evidence: target/appfw/cli-test.json"),
  );
});

test("packager fails closed when a required Fast report is not green", (context) => {
  const { root, sourceSha } = createFixture();
  context.after(() => fs.rmSync(root, { recursive: true, force: true }));
  writeJson(root, "target/appfw/golden-downstream.json", {
    command: "golden-downstream",
    ok: false,
  });

  const result = runPackager(root, sourceSha);
  assert.notEqual(result.status, 0);
  const manifest = JSON.parse(
    fs.readFileSync(path.join(root, "target/appfw/pr-fast-evidence-manifest.json"), "utf8"),
  );
  assert.equal(manifest.ok, false);
  assert.ok(
    manifest.root_causes.includes(
      "required Fast evidence is not green: target/appfw/golden-downstream.json",
    ),
  );
});

test("packager fails closed above the bounded evidence ceiling", (context) => {
  const { root, sourceSha } = createFixture();
  context.after(() => fs.rmSync(root, { recursive: true, force: true }));
  fs.writeFileSync(path.join(root, "target/appfw/oversized.log"), "x".repeat(2048));
  const result = runPackager(root, sourceSha, {
    APPFW_PR_FAST_EVIDENCE_MAX_BYTES: "1024",
  });
  assert.notEqual(result.status, 0);
  const manifest = JSON.parse(
    fs.readFileSync(path.join(root, "target/appfw/pr-fast-evidence-manifest.json"), "utf8"),
  );
  assert.equal(manifest.ok, false);
  assert.equal(manifest.size.under_limit, false);
  assert.ok(manifest.root_causes.some((cause) => cause.includes("above the 1024-byte ceiling")));
  assert.equal(
    fs.existsSync(path.join(root, "target/appfw/pr-fast-evidence")),
    false,
    "an over-limit failure must not leave the payload inside Bitbucket's capture-on: always glob",
  );
});

test("packager fails closed when Bitbucket and Git identify different commits", (context) => {
  const { root, sourceSha } = createFixture();
  context.after(() => fs.rmSync(root, { recursive: true, force: true }));
  const differentSha = sourceSha === "0".repeat(40) ? "1".repeat(40) : "0".repeat(40);
  const result = runPackager(root, sourceSha, { BITBUCKET_COMMIT: differentSha });
  assert.notEqual(result.status, 0);
  const manifest = JSON.parse(
    fs.readFileSync(path.join(root, "target/appfw/pr-fast-evidence-manifest.json"), "utf8"),
  );
  assert.equal(manifest.ok, false);
  assert.ok(
    manifest.root_causes.some((cause) => cause.includes("does not resolve to its exact commit")),
  );
});

test("packager retains diagnostics without presenting a failed Fast gate as green", (context) => {
  const { root, sourceSha } = createFixture();
  context.after(() => fs.rmSync(root, { recursive: true, force: true }));
  const result = runPackager(root, sourceSha, {}, 9);
  assert.notEqual(result.status, 0);
  const manifest = JSON.parse(
    fs.readFileSync(path.join(root, "target/appfw/pr-fast-evidence-manifest.json"), "utf8"),
  );
  assert.equal(manifest.ok, false);
  assert.equal(manifest.gate_exit_status, 9);
  assert.ok(manifest.artifacts.length > 0);
  assert.ok(manifest.root_causes.includes("Fast gate sequence exited with status 9"));
});

test("packager publishes staged evidence only after size validation and an atomic replacement", () => {
  const source = fs.readFileSync(packager, "utf8");
  assert.match(source, /temporary_root = staging_root\.with_name/);
  assert.match(source, /if manifest\["size"\]\["under_limit"\]:/);
  assert.match(source, /os\.replace\(temporary_root, staging_root\)/);
  assert.match(source, /shutil\.rmtree\(temporary_root, ignore_errors=True\)/);
  assert.match(source, /shutil\.rmtree\(staging_root, ignore_errors=True\)/);
});

test("packager refuses an exact-SHA claim for a dirty source tree", (context) => {
  const { root, sourceSha } = createFixture();
  context.after(() => fs.rmSync(root, { recursive: true, force: true }));
  fs.writeFileSync(path.join(root, "uncommitted-source.txt"), "not in tested SHA\n");
  const result = runPackager(root, sourceSha);
  assert.notEqual(result.status, 0);
  const manifest = JSON.parse(
    fs.readFileSync(path.join(root, "target/appfw/pr-fast-evidence-manifest.json"), "utf8"),
  );
  assert.equal(manifest.ok, false);
  assert.equal(manifest.source_worktree_clean, false);
  assert.ok(manifest.root_causes.some((cause) => cause.includes("clean source tree")));
});

test("closing freshness check rejects a destination that advanced", (context) => {
  const remoteRoot = fs.mkdtempSync(path.join(os.tmpdir(), "appfw-pr-freshness-remote-"));
  const remote = path.join(remoteRoot, "origin.git");
  const root = initFixtureRepository("appfw-pr-freshness-work-");
  context.after(() => fs.rmSync(remoteRoot, { recursive: true, force: true }));
  context.after(() => fs.rmSync(root, { recursive: true, force: true }));
  execFileSync("git", ["init", "--bare", "-q", remote]);
  execFileSync("git", ["branch", "-M", "main"], { cwd: root });
  execFileSync("git", ["remote", "add", "origin", remote], { cwd: root });
  execFileSync("git", ["push", "-q", "-u", "origin", "main"], { cwd: root });
  const pinned = execFileSync("git", ["rev-parse", "HEAD"], {
    cwd: root,
    encoding: "utf8",
  }).trim();
  const environment = {
    ...process.env,
    BITBUCKET_PR_DESTINATION_BRANCH: "main",
    BITBUCKET_PR_DESTINATION_COMMIT: pinned.slice(0, 12),
  };
  const current = spawnSync("python3", [freshnessCheck], {
    cwd: root,
    encoding: "utf8",
    env: environment,
  });
  assert.equal(current.status, 0, `${current.stdout}\n${current.stderr}`);
  const currentFullIdentity = spawnSync("python3", [freshnessCheck], {
    cwd: root,
    encoding: "utf8",
    env: { ...environment, BITBUCKET_PR_DESTINATION_COMMIT: pinned },
  });
  assert.equal(
    currentFullIdentity.status,
    0,
    `${currentFullIdentity.stdout}\n${currentFullIdentity.stderr}`,
  );

  fs.writeFileSync(path.join(root, "advance.txt"), "advanced\n");
  execFileSync("git", ["add", "advance.txt"], { cwd: root });
  execFileSync("git", ["commit", "-qm", "advance destination"], { cwd: root });
  execFileSync("git", ["push", "-q", "origin", "main"], { cwd: root });
  const stale = spawnSync("python3", [freshnessCheck], {
    cwd: root,
    encoding: "utf8",
    env: environment,
  });
  assert.notEqual(stale.status, 0);
  const report = JSON.parse(
    fs.readFileSync(path.join(root, "target/appfw/pr-destination-freshness.json"), "utf8"),
  );
  assert.equal(report.ok, false);
  assert.ok(report.root_causes.includes("destination branch advanced after the pipeline identity was pinned"));
});

test("opening release-lite guard compares only the fetched PR destination", (context) => {
  const remoteRoot = fs.mkdtempSync(path.join(os.tmpdir(), "appfw-release-lite-remote-"));
  const remote = path.join(remoteRoot, "origin.git");
  const root = initFixtureRepository("appfw-release-lite-work-");
  context.after(() => fs.rmSync(remoteRoot, { recursive: true, force: true }));
  context.after(() => fs.rmSync(root, { recursive: true, force: true }));
  execFileSync("git", ["init", "--bare", "-q", remote]);
  execFileSync("git", ["branch", "-M", "main"], { cwd: root });
  execFileSync("git", ["remote", "add", "origin", remote], { cwd: root });
  execFileSync("git", ["push", "-q", "-u", "origin", "main"], { cwd: root });
  execFileSync("git", ["checkout", "-qb", "feature"], { cwd: root });
  fs.mkdirSync(path.join(root, "docs"), { recursive: true });
  fs.writeFileSync(path.join(root, "docs/change.md"), "bounded destination comparison\n");
  execFileSync("git", ["add", "docs/change.md"], { cwd: root });
  execFileSync("git", ["commit", "-qm", "feature change"], { cwd: root });

  const fixtureGuard = installReleaseLiteGuard(root);
  const result = spawnSync("bash", [fixtureGuard], {
    cwd: root,
    encoding: "utf8",
    env: {
      ...process.env,
      APPFW_PR_REMOTE_GIT_TIMEOUT_SECONDS: "5",
      BITBUCKET_PR_DESTINATION_BRANCH: "main",
    },
    timeout: 10000,
  });
  assert.equal(result.status, 0, `${result.stdout}\n${result.stderr}`);
  const report = JSON.parse(
    fs.readFileSync(path.join(root, "target/appfw/release-lite-guard.json"), "utf8"),
  );
  assert.equal(report.ok, true);
  assert.equal(report.comparison.ok, true);
  assert.equal(report.comparison.mode, "merge-base");
  assert.equal(report.comparison.detail, "refs/remotes/origin/main");
  assert.deepEqual(report.changed_files, ["docs/change.md"]);
});

test("release-lite classifies the complete scripts/ci control surface", (context) => {
  const reportRoot = fs.mkdtempSync(path.join(os.tmpdir(), "appfw-release-lite-sensitive-"));
  context.after(() => fs.rmSync(reportRoot, { recursive: true, force: true }));
  const changedFiles = [
    "scripts/ci/pr-preflight.py",
    "scripts/ci/package-pr-fast-evidence.py",
    "scripts/ci/pr-destination-freshness.py",
    "scripts/ci/pr-fast-evidence-contract.test.mjs",
  ];
  const result = spawnSync("bash", [releaseLiteGuard], {
    cwd: repositoryRoot,
    encoding: "utf8",
    env: {
      ...process.env,
      APPFW_RELEASE_ARTIFACT_DIR: reportRoot,
      APPFW_RELEASE_LITE_CHANGED_FILES: changedFiles.join(","),
      APPFW_RELEASE_LITE_EVIDENCE_URL: "",
      APPFW_RELEASE_LITE_APPROVER: "",
      APPFW_RELEASE_LITE_REASON: "",
    },
  });
  assert.notEqual(result.status, 0, `${result.stdout}\n${result.stderr}`);
  const report = JSON.parse(
    fs.readFileSync(path.join(reportRoot, "release-lite-guard.json"), "utf8"),
  );
  assert.equal(report.ok, false);
  assert.equal(report.release_lite_required, true);
  assert.deepEqual(
    report.sensitive_files.map((item) => item.path),
    [...changedFiles].sort(),
  );
  assert.ok(report.sensitive_files.every((item) => item.pattern === "scripts/ci/**"));
  assert.deepEqual(report.approval.missing_fields, [
    "APPFW_RELEASE_LITE_EVIDENCE_URL",
    "APPFW_RELEASE_LITE_APPROVER",
    "APPFW_RELEASE_LITE_REASON",
  ]);
});

test("remote guard transports are bounded and fail closed with structured evidence", (context) => {
  const root = initFixtureRepository("appfw-pr-remote-guard-");
  const reportRoot = fs.mkdtempSync(path.join(os.tmpdir(), "appfw-release-lite-report-"));
  const transport = createGitTransportShim(context);
  context.after(() => fs.rmSync(root, { recursive: true, force: true }));
  context.after(() => fs.rmSync(reportRoot, { recursive: true, force: true }));

  const identity = "a".repeat(40);
  for (const scenario of [
    {
      mode: "hang",
      freshnessCause: "remote destination query timed out after 1 seconds",
      freshnessStatus: "timed_out",
      openingCause: "destination fetch timed out after 1 seconds",
    },
    {
      mode: "fail",
      freshnessCause: "remote destination query failed with status 37",
      freshnessStatus: "failed",
      openingCause: "destination fetch failed with status 37",
    },
  ]) {
    const environment = {
      ...process.env,
      ...transport.environment,
      APPFW_PR_REMOTE_GIT_TIMEOUT_SECONDS: "1",
      APPFW_TEST_GIT_TRANSPORT_MODE: scenario.mode,
      BITBUCKET_PR_DESTINATION_BRANCH: "main",
      BITBUCKET_PR_DESTINATION_COMMIT: identity,
    };

    fs.rmSync(path.join(root, "target"), { recursive: true, force: true });
    const started = Date.now();
    const freshness = spawnSync("python3", [freshnessCheck], {
      cwd: root,
      encoding: "utf8",
      env: environment,
      timeout: 5000,
    });
    assert.notEqual(freshness.status, 0, `${freshness.stdout}\n${freshness.stderr}`);
    assert.ok(Date.now() - started < 5000, `${scenario.mode} freshness query was not bounded`);
    const freshnessReport = JSON.parse(
      fs.readFileSync(path.join(root, "target/appfw/pr-destination-freshness.json"), "utf8"),
    );
    assert.equal(freshnessReport.ok, false);
    assert.equal(freshnessReport.remote_query.status, scenario.freshnessStatus);
    assert.equal(freshnessReport.remote_query.timeout_seconds, 1);
    assert.equal(freshnessReport.remote_query.terminal_prompt_disabled, true);
    assert.ok(freshnessReport.root_causes.includes(scenario.freshnessCause));

    const openingReportRoot = path.join(reportRoot, scenario.mode);
    const opening = spawnSync("bash", [releaseLiteGuard], {
      cwd: repositoryRoot,
      encoding: "utf8",
      env: {
        ...environment,
        APPFW_RELEASE_ARTIFACT_DIR: openingReportRoot,
        APPFW_RELEASE_LITE_BASE_REF: "",
        APPFW_RELEASE_LITE_CHANGED_FILES: "",
      },
      timeout: 5000,
    });
    assert.notEqual(opening.status, 0, `${opening.stdout}\n${opening.stderr}`);
    const openingReport = JSON.parse(
      fs.readFileSync(path.join(openingReportRoot, "release-lite-guard.json"), "utf8"),
    );
    assert.equal(openingReport.ok, false);
    assert.equal(openingReport.comparison.ok, false);
    assert.equal(openingReport.comparison.mode, "destination-fetch-failed");
    assert.equal(openingReport.comparison.error, scenario.openingCause);
    assert.ok(openingReport.failure_summary.root_causes.includes(scenario.openingCause));
    assert.equal(openingReport.changed_file_count, 0);
  }
});
