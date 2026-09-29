import assert from "node:assert/strict";
import { AsyncLocalStorage } from "node:async_hooks";
import { execFileSync, spawnSync } from "node:child_process";
import { createHash } from "node:crypto";
import { randomUUID } from "node:crypto";
import { cpSync, existsSync, mkdtempSync, mkdirSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { describe, test } from "node:test";

import {
  assertExactNexusWorkstreamTestManifest,
} from "./run-nexus-workstream-tests.mjs";

const sourceRoot = dirname(dirname(fileURLToPath(import.meta.url)));
const fixtureRoots = new AsyncLocalStorage();
const registeredTestNames = [];

function nexusTest(name, callback) {
  registeredTestNames.push(name);
  test(name, async (...args) => {
    const roots = new Set();
    try {
      return await fixtureRoots.run(roots, () => callback(...args));
    } finally {
      for (const root of roots) {
        rmSync(root, { recursive: true, force: true });
      }
    }
  });
}

function registeredFixtureRoot(label) {
  const roots = fixtureRoots.getStore();
  assert(
    roots,
    "registeredFixtureRoot() must run inside a registered Nexus workstream test",
  );
  assert.match(label, /^[a-z0-9-]+$/, "invalid Nexus fixture label");
  const runId = process.env.APPFW_NEXUS_WORKSTREAM_TEST_RUN_ID;
  if (runId) {
    assert.match(runId, /^[A-Za-z0-9-]+$/, "invalid Nexus workstream test run ID");
  }
  const prefix = runId
    ? `appfw-nexus-live-${runId}-${label}-`
    : `appfw-nexus-live-${label}-`;
  const root = mkdtempSync(join(tmpdir(), prefix));
  roots.add(root);
  return root;
}

const LOCAL_PLAN_REF =
  /^[a-z0-9][a-z0-9._-]*\.product-increment\.json$/;

function registeredPlanRefs(portfolio) {
  const planRefs = [];
  for (const item of portfolio.product_increments) {
    const planRef = item.plan_ref;
    if (planRef == null || planRef === "") continue;
    if (typeof planRef !== "string" || !LOCAL_PLAN_REF.test(planRef)) {
      throw new Error(`unsafe registered plan_ref in fixture: ${String(planRef)}`);
    }
    planRefs.push(planRef);
  }
  return [...new Set(planRefs)];
}

function copyRegisteredPlans(portfolio, sourceSpecsDir, targetSpecsDir) {
  for (const planRef of registeredPlanRefs(portfolio)) {
    cpSync(join(sourceSpecsDir, planRef), join(targetSpecsDir, planRef));
  }
}

function reconcilePortfolioLaneCounts(root, portfolio) {
  const counts = registeredPlanRefs(portfolio)
    .flatMap((planRef) => {
      const plan = JSON.parse(
        readFileSync(join(root, "docs", "specs", planRef), "utf8"),
      );
      return plan.lanes ?? [];
    })
    .reduce(
      (current, lane) => {
        if (lane.status === "in_progress") current.active += 1;
        if (lane.status === "under_review") current.review += 1;
        return current;
      },
      { active: 0, review: 0 },
    );
  portfolio.operating_capacity.current_active_lanes = counts.active;
  portfolio.operating_capacity.current_review_lanes = counts.review;
}

function git(root, ...args) {
  return execFileSync("git", args, { cwd: root, encoding: "utf8" }).trim();
}

function commitTree(root, tree, parents, message) {
  return execFileSync("git", ["commit-tree", tree, ...parents.flatMap((parent) => ["-p", parent])], {
    cwd: root,
    encoding: "utf8",
    input: `${message}\n`,
  }).trim();
}

function fixture() {
  const root = registeredFixtureRoot("repository");
  mkdirSync(join(root, "scripts"));
  mkdirSync(join(root, "docs", "specs"), { recursive: true });
  mkdirSync(join(root, "docs", "start"), { recursive: true });
  mkdirSync(join(root, "docs", "evidence", "product-increments"), {
    recursive: true,
  });
  cpSync(join(sourceRoot, "scripts", "check-nexus-workstreams.mjs"), join(root, "scripts", "check-nexus-workstreams.mjs"));
  cpSync(
    join(sourceRoot, "scripts", "product-increment-path-safety.mjs"),
    join(root, "scripts", "product-increment-path-safety.mjs"),
  );
  cpSync(
    join(sourceRoot, "scripts", "check-product-increment-plan.mjs"),
    join(root, "scripts", "check-product-increment-plan.mjs"),
  );
  cpSync(
    join(sourceRoot, "scripts", "check-product-increment-portfolio.mjs"),
    join(root, "scripts", "check-product-increment-portfolio.mjs"),
  );
  cpSync(
    join(sourceRoot, "docs", "specs", "nexus-poc-workstream-topology.json"),
    join(root, "docs", "specs", "nexus-poc-workstream-topology.json"),
  );
  for (const file of [
    "nexus-add-provider-seams.json",
    "product-increment-portfolio.json",
  ]) {
    cpSync(
      join(sourceRoot, "docs", "specs", file),
      join(root, "docs", "specs", file),
    );
  }
  cpSync(
    join(sourceRoot, "docs", "start", "product-increment-delivery-model.md"),
    join(root, "docs", "start", "product-increment-delivery-model.md"),
  );
  cpSync(
    join(sourceRoot, "docs", "evidence", "product-increments", "afs-p1-router.json"),
    join(root, "docs", "evidence", "product-increments", "afs-p1-router.json"),
  );
  const portfolioPath = join(
    root,
    "docs/specs/product-increment-portfolio.json",
  );
  const portfolio = JSON.parse(readFileSync(portfolioPath, "utf8"));
  copyRegisteredPlans(
    portfolio,
    join(sourceRoot, "docs", "specs"),
    join(root, "docs", "specs"),
  );
  const now = new Date().toISOString();
  portfolio.last_reconciled_at = now;
  for (const productIncrement of portfolio.product_increments) {
    productIncrement.status_updated_at = now;
  }
  const nexus = portfolio.product_increments.find(
    (item) => item.id === "NXP-PI-ADD-PROVIDER-01",
  );
  nexus.lifecycle_state = "active";
  nexus.requires = [];
  const nexusPlanPath = join(
    root,
    "docs/specs/nexus-add-provider.product-increment.json",
  );
  const nexusPlan = JSON.parse(readFileSync(nexusPlanPath, "utf8"));
  nexusPlan.product_increment.planning_posture = "active";
  nexusPlan.lanes[0].status = "in_progress";
  writeFileSync(nexusPlanPath, JSON.stringify(nexusPlan));
  reconcilePortfolioLaneCounts(root, portfolio);
  writeFileSync(portfolioPath, JSON.stringify(portfolio));
  const topology = JSON.parse(
    readFileSync(join(root, "docs", "specs", "nexus-poc-workstream-topology.json"), "utf8"),
  );
  for (const contextPath of topology.workstreams.flatMap((workstream) => workstream.required_context)) {
    const target = join(root, contextPath);
    mkdirSync(dirname(target), { recursive: true });
    if (!existsSync(target)) writeFileSync(target, "fixture\n");
  }
  git(root, "init", "-q");
  git(root, "config", "user.name", "Test");
  git(root, "config", "user.email", "test@example.com");
  writeFileSync(join(root, "seed"), "main\n");
  git(root, "add", "seed");
  git(root, "commit", "-qm", "main");
  git(root, "branch", "-M", "main");
  const main = git(root, "rev-parse", "HEAD");
  git(root, "update-ref", "refs/remotes/origin/main", main);
  for (const branch of [
    "feature/product-increment-lane-operating-model",
    "feature/pds-w1a-dtcg-token-source",
    "feature/pds-w1s-executive-signature-reference",
  ]) {
    git(root, "branch", branch, main);
  }
  const routerPlanPath = join(
    root,
    "docs/specs/afs-transparent-routing.product-increment.json",
  );
  const routerPlan = JSON.parse(readFileSync(routerPlanPath, "utf8"));
  routerPlan.lanes[0].base_sha = main;
  routerPlan.integration.base_ref = main;
  writeFileSync(routerPlanPath, JSON.stringify(routerPlan));
  const routerEvidencePath = join(
    root,
    "docs/evidence/product-increments/afs-p1-router.json",
  );
  const routerEvidence = JSON.parse(readFileSync(routerEvidencePath, "utf8"));
  routerEvidence.candidate_sha = main;
  routerEvidence.base_sha = main;
  writeFileSync(routerEvidencePath, JSON.stringify(routerEvidence));
  const currentPortfolio = JSON.parse(readFileSync(portfolioPath, "utf8"));
  for (const increment of currentPortfolio.product_increments) {
    for (const evidence of increment.evidence) {
      if (evidence.kind === "git") evidence.ref = main;
    }
  }
  writeFileSync(portfolioPath, JSON.stringify(currentPortfolio));
  for (const documentRef of currentPortfolio.product_increments.flatMap(
    (item) =>
      item.evidence
        .filter((evidence) => evidence.kind === "document")
        .map((evidence) => evidence.ref),
  )) {
    const target = join(root, documentRef);
    mkdirSync(dirname(target), { recursive: true });
    if (!existsSync(target)) writeFileSync(target, "fixture evidence\n");
  }
  return { root, main };
}

function commit(root, name) {
  writeFileSync(join(root, "seed"), `${name}\n`);
  git(root, "add", "seed");
  git(root, "commit", "-qm", name);
  return git(root, "rev-parse", "HEAD");
}

function addImplementedRouterCredit(root, base) {
  const proofPath = "docs/evidence/product-increments/afs-p1-router-proof.txt";
  const proofBytes = Buffer.from("retained independent review proof\n");
  const proofSha256 = createHash("sha256").update(proofBytes).digest("hex");
  const proofAbsolutePath = join(root, proofPath);
  const meteringPath =
    "docs/evidence/product-increments/afs-p1-router-metering.txt";
  const meteringBytes = Buffer.from("self-authored metering receipt\n");
  const meteringSha256 = createHash("sha256")
    .update(meteringBytes)
    .digest("hex");
  const laneReviewPath =
    "docs/evidence/product-increments/afs-p1-router-review.txt";
  const laneReviewBytes = Buffer.from("retained lane review receipt\n");
  const laneReviewSha256 = createHash("sha256")
    .update(laneReviewBytes)
    .digest("hex");
  const integrationReviewPath =
    "docs/evidence/product-increments/afs-p1-integration-review.txt";
  const integrationReviewBytes = Buffer.from(
    "retained aggregate review receipt\n",
  );
  const integrationReviewSha256 = createHash("sha256")
    .update(integrationReviewBytes)
    .digest("hex");

  git(root, "checkout", "-qb", "feature/afs-008-transparent-routing-telemetry", base);
  const candidatePath = join(root, "scripts/agent-routing/appfw-model-route.mjs");
  mkdirSync(dirname(candidatePath), { recursive: true });
  writeFileSync(candidatePath, "export const route = 'reviewed';\n");
  git(root, "add", "scripts/agent-routing/appfw-model-route.mjs");
  git(root, "commit", "-qm", "implemented router credit");
  const candidate = git(root, "rev-parse", "HEAD");

  git(root, "checkout", "main");
  git(root, "merge", "--ff-only", candidate);
  writeFileSync(proofAbsolutePath, proofBytes);
  writeFileSync(join(root, meteringPath), meteringBytes);
  writeFileSync(join(root, laneReviewPath), laneReviewBytes);
  writeFileSync(join(root, integrationReviewPath), integrationReviewBytes);

  const planPath = join(root, "docs/specs/afs-transparent-routing.product-increment.json");
  const plan = JSON.parse(readFileSync(planPath, "utf8"));
  const lane = plan.lanes[0];
  lane.status = "implemented";
  lane.delivery_credit = "implemented";
  lane.provides[0].state = "implemented";
  lane.provides[0].evidence_ref = `git:${candidate}`;
  lane.evidence_record_ref =
    "document:docs/evidence/product-increments/afs-p1-router.json";
  plan.integration.evidence_record_ref =
    "document:docs/evidence/product-increments/afs-p1-integration.json";
  const requiredChecks = [...lane.focused_checks, ...plan.integration.integration_checks];
  const evidenceRecord = ({
    checks,
    reviewer,
    reviewPath,
    reviewSha256,
  }) => ({
    schema: "appfw_product_increment_evidence@1",
    product_increment_id: plan.product_increment.id,
    outcome_id: plan.product_increment.outcome_id,
    scope: { kind: "direct_main_candidate" },
    candidate_sha: candidate,
    base_sha: base,
    proof: {
      stage: plan.product_increment.stage,
      profile: "full",
      status: "passed",
      completed_at: new Date().toISOString(),
      checks: checks.map((command) => ({
        command,
        result: "passed",
        output_ref: `document:${proofPath}`,
        output_sha256: proofSha256,
      })),
    },
    economics: {
      measurement_state: "measured",
      outcome: "implemented",
      planned_model: "efficient-tier",
      actual_model: "efficient-tier",
      reasoning_effort: "medium",
      routing_basis: "lowest_capable",
      metering_receipt_ref: `document:${meteringPath}`,
      metering_receipt_sha256: meteringSha256,
      elapsed_ms: 1,
      input_tokens: 1,
      output_tokens: 1,
      cost_usd: 0,
      retries: 0,
      corrections: 0,
      rework_cycles: 0,
      context_source_count: 1,
      delegation_depth: 1,
    },
    review: {
      reviewer,
      reviewed_sha: candidate,
      disposition: "go",
      completed_at: new Date().toISOString(),
      findings: {
        blocker: 0,
        critical: 0,
        important: 0,
        should_address: 0,
        nice_to_address: 0,
      },
      evidence_ref: `document:${reviewPath}`,
      evidence_sha256: reviewSha256,
    },
    lane_acceptance: null,
    product_acceptance: null,
  });
  const evidence = evidenceRecord({
    checks: requiredChecks,
    reviewer: lane.independent_reviewer,
    reviewPath: laneReviewPath,
    reviewSha256: laneReviewSha256,
  });
  const integrationEvidence = evidenceRecord({
    checks: plan.integration.integration_checks,
    reviewer: plan.integration.independent_reviewer,
    reviewPath: integrationReviewPath,
    reviewSha256: integrationReviewSha256,
  });
  lane.base_sha = base;
  plan.integration.base_ref = base;
  writeFileSync(
    join(root, "docs/evidence/product-increments/afs-p1-router.json"),
    JSON.stringify(evidence),
  );
  writeFileSync(
    join(root, "docs/evidence/product-increments/afs-p1-integration.json"),
    JSON.stringify(integrationEvidence),
  );
  writeFileSync(planPath, JSON.stringify(plan));

  const portfolioPath = join(root, "docs/specs/product-increment-portfolio.json");
  const portfolio = JSON.parse(readFileSync(portfolioPath, "utf8"));
  const increment = portfolio.product_increments.find((item) => item.id === "AFS-PI-P1");
  increment.lifecycle_state = "implemented";
  increment.provides[0].state = "implemented";
  increment.provides[0].evidence_ref = `git:${candidate}`;
  increment.evidence.find((item) => item.kind === "git").ref = candidate;
  reconcilePortfolioLaneCounts(root, portfolio);
  writeFileSync(portfolioPath, JSON.stringify(portfolio));
  git(
    root,
    "add",
    proofPath,
    meteringPath,
    laneReviewPath,
    integrationReviewPath,
    "docs/evidence/product-increments/afs-p1-router.json",
    "docs/evidence/product-increments/afs-p1-integration.json",
    "docs/specs/afs-transparent-routing.product-increment.json",
    "docs/specs/product-increment-portfolio.json",
  );
  git(root, "commit", "-qm", "retain accepted router implementation evidence");
  git(root, "update-ref", "refs/remotes/origin/main", git(root, "rev-parse", "HEAD"));
  return { evidence, proofAbsolutePath, proofPath };
}

function assignment(root, base, integrationBranch = "integrate/ws01-test") {
  const now = Date.now();
  const planPath = join(
    root,
    "docs/specs/nexus-add-provider.product-increment.json",
  );
  const plan = JSON.parse(readFileSync(planPath, "utf8"));
  const value = {
    schema: "appfw_workstation_assignment@4",
    assignment_id: randomUUID(),
    portfolio_id: "app-framework-and-nexus",
    program_id: "nexus-poc",
    product_increment_id: "NXP-PI-ADD-PROVIDER-01",
    delivery_lane_id: "NXP-L1-EXPERIENCE",
    capability_domain_ids: ["WS-07"],
    workstation_id: "ws01-live-test",
    lane_outcome_owner: "Nexus Experience Owner",
    technical_lead: "Nexus Experience Technical Lead",
    branch_owner: "Nexus Experience Branch Owner",
    independent_reviewer: "Nexus Experience Reviewer",
    admitted_by: "Program Flow Controller",
    integration_owner: "Nexus Integration Branch Manager",
    branch: "feature/nexus-add-provider-experience",
    base_sha: base,
    integration_branch: integrationBranch,
    execution_target: "local_agent",
    planned_model_tier: "efficient-tier",
    routing_basis: "lowest_capable",
    economics_budget: structuredClone(plan.lanes[0].economics_budget),
    delegation_depth: 1,
    activated_at: new Date(now - 60 * 60 * 1000).toISOString(),
    review_after: new Date(now + 24 * 60 * 60 * 1000).toISOString(),
    status: "active",
  };
  plan.integration.branch = integrationBranch;
  plan.integration.base_ref = base;
  for (const lane of plan.lanes) {
    lane.destination_branch = integrationBranch;
    lane.base_sha = base;
  }
  writeFileSync(planPath, JSON.stringify(plan));
  const path = join(root, "assignment.json");
  writeFileSync(path, JSON.stringify(value));
  return path;
}

function assignmentV4(root, base, laneId = "NXP-L1-EXPERIENCE") {
  const path = assignment(root, base, "integrate/nexus-add-provider-poc");
  const value = JSON.parse(readFileSync(path, "utf8"));
  value.delivery_lane_id = laneId;
  writeFileSync(path, JSON.stringify(value));
  return path;
}

function assignmentV3(root, base) {
  const v4Path = assignmentV4(root, base);
  const v4 = JSON.parse(readFileSync(v4Path, "utf8"));
  const value = {
    schema: "appfw_workstation_assignment@3",
    program_id: v4.program_id,
    workstream_id: "WS-07",
    product_increment_id: v4.product_increment_id,
    delivery_lane_id: v4.delivery_lane_id,
    workstation_id: v4.workstation_id,
    workstream_lead: v4.lane_outcome_owner,
    technical_lead: v4.technical_lead,
    branch_owner: v4.branch_owner,
    independent_reviewer: v4.independent_reviewer,
    admitted_by: v4.admitted_by,
    integration_owner: v4.integration_owner,
    branch: v4.branch,
    base_sha: v4.base_sha,
    integration_branch: v4.integration_branch,
    activated_at: v4.activated_at,
    review_after: v4.review_after,
    status: v4.status,
  };
  writeFileSync(v4Path, JSON.stringify(value));
  return v4Path;
}

function check(root, assignmentPath, context = null) {
  const args = [
    "scripts/check-nexus-workstreams.mjs",
    "--assignment",
    assignmentPath,
    "--live",
    "--json",
  ];
  if (context) args.push("--context", context);
  const result = spawnSync(
    process.execPath,
    args,
    { cwd: root, encoding: "utf8", maxBuffer: 10 * 1024 * 1024 },
  );
  assert.equal(
    result.error,
    undefined,
    result.stderr || result.error?.message,
  );
  return { status: result.status, output: parseCheckerOutput(result) };
}

function checkWithoutLive(root, assignmentPath, context = null) {
  const args = [
    "scripts/check-nexus-workstreams.mjs",
    "--assignment",
    assignmentPath,
    "--json",
  ];
  if (context) args.push("--context", context);
  const result = spawnSync(process.execPath, args, {
    cwd: root,
    encoding: "utf8",
    maxBuffer: 10 * 1024 * 1024,
  });
  assert.equal(
    result.error,
    undefined,
    result.stderr || result.error?.message,
  );
  return { status: result.status, output: parseCheckerOutput(result) };
}

function parseCheckerOutput(result) {
  try {
    return JSON.parse(result.stdout);
  } catch (error) {
    throw new Error(
      `checker emitted invalid JSON: status=${result.status} signal=${result.signal ?? "none"} stdout_bytes=${Buffer.byteLength(result.stdout ?? "")} stderr=${JSON.stringify(result.stderr ?? "")}: ${error.message}`,
    );
  }
}

function producer(root) {
  git(root, "checkout", "-qb", "feature/nexus-add-provider-experience");
}

describe("workstation assignment and Product Increment admission", {
  concurrency: 4,
}, () => {

nexusTest("[fixture-regression] discovers and copies a novel safe registered plan_ref without a filename allowlist", () => {
  const root = registeredFixtureRoot("plan-copy");
  const sourceSpecsDir = join(root, "source");
  const targetSpecsDir = join(root, "target");
  mkdirSync(sourceSpecsDir, { recursive: true });
  mkdirSync(targetSpecsDir, { recursive: true });
  const planRef = "novel-fixture-shape.product-increment.json";
  const plan = { fixture: "registered-plan-bytes" };
  writeFileSync(join(sourceSpecsDir, planRef), JSON.stringify(plan));
  const portfolio = {
    product_increments: [
      { plan_ref: planRef },
      { plan_ref: null },
      { plan_ref: planRef },
    ],
  };

  assert.deepEqual(registeredPlanRefs(portfolio), [planRef]);
  assert.throws(
    () =>
      registeredPlanRefs({
        product_increments: [{ plan_ref: "../outside.product-increment.json" }],
      }),
    /unsafe registered plan_ref/,
  );
  copyRegisteredPlans(portfolio, sourceSpecsDir, targetSpecsDir);
  const target = join(targetSpecsDir, planRef);
  if (!existsSync(target)) writeFileSync(target, "fixture evidence\n");
  assert.deepEqual(JSON.parse(readFileSync(target, "utf8")), plan);
});

nexusTest("[admission-smoke] v4 readiness remains non-authoritative for copied assignments", () => {
  const { root, main } = fixture();
  git(root, "checkout", "-qb", "feature/nexus-add-provider-experience", main);
  git(root, "branch", "integrate/nexus-add-provider-poc", main);
  const firstPath = assignmentV4(root, main);
  const second = JSON.parse(readFileSync(firstPath, "utf8"));
  second.assignment_id = randomUUID();
  second.workstation_id = "ws01-live-copy";
  const secondPath = join(root, "assignment-copy.json");
  writeFileSync(secondPath, JSON.stringify(second));

  for (const assignmentPath of [firstPath, secondPath]) {
    const result = check(root, assignmentPath, "WS-07");
    assert.equal(result.status, 0, JSON.stringify(result.output.errors, null, 2));
    assert.equal(result.output.assignment.product_increment_id, "NXP-PI-ADD-PROVIDER-01");
    assert.equal(result.output.assignment.delivery_lane_id, "NXP-L1-EXPERIENCE");
    assert.equal(result.output.assignment.source_admission_ready, true);
    assert.equal(result.output.assignment.source_authorized, false);
    assert.deepEqual(result.output.context.allowed_write_roots, []);
    assert.equal(
      result.output.assignment.authority_reason,
      "shared_live_lease_not_verified",
    );
  }
});

nexusTest("rejects exact accepted-main base without the named integration ref", () => {
  const { root, main } = fixture();
  producer(root);
  const result = check(root, assignment(root, main));
  assert.equal(result.status, 1);
  assert(
    result.output.errors.some((error) =>
      error.startsWith("named integration ref is missing:"),
    ),
  );
});

nexusTest("v4 binds the workstation to its Product Increment and Delivery Lane without manufacturing a lease", () => {
  const { root, main } = fixture();
  git(root, "checkout", "-qb", "feature/nexus-add-provider-experience");
  git(root, "branch", "integrate/nexus-add-provider-poc", main);
  const result = check(root, assignmentV4(root, main), "WS-07");
  assert.equal(result.status, 0, JSON.stringify(result.output.errors, null, 2));
  assert.equal(result.output.ok, true);
  assert.equal(
    result.output.assignment.product_increment_id,
    "NXP-PI-ADD-PROVIDER-01",
  );
  assert.equal(result.output.assignment.delivery_lane_id, "NXP-L1-EXPERIENCE");
  assert.deepEqual(result.output.context.allowed_write_roots, []);
  assert.equal(result.output.assignment.source_admission_ready, true);
  assert.equal(result.output.context.source_authorized, false);
  assert.equal(result.output.context.delivery_lane.id, "NXP-L1-EXPERIENCE");
  assert(
    result.output.context.forbidden_roots.includes("appfw_runtime/src/chat.rs"),
  );
});

nexusTest("active assignment without live validation grants no source authority", () => {
  const { root, main } = fixture();
  producer(root);
  const result = checkWithoutLive(
    root,
    assignmentV4(root, main),
    "WS-07",
  );
  assert.equal(result.status, 0, JSON.stringify(result.output.errors, null, 2));
  assert.equal(result.output.context.source_authorized, false);
  assert.deepEqual(result.output.context.allowed_write_roots, []);
  assert(
    result.output.warnings.some((warning) =>
      warning.includes("without --live grants no source authority"),
    ),
  );
});

nexusTest("legacy v3 assignment remains context-only even when local lineage checks pass", () => {
  const { root, main } = fixture();
  producer(root);
  git(root, "branch", "integrate/nexus-add-provider-poc", main);
  const result = check(root, assignmentV3(root, main), "WS-07");
  assert.equal(result.status, 0, JSON.stringify(result.output.errors, null, 2));
  assert.equal(result.output.assignment.source_admission_ready, false);
  assert.equal(result.output.assignment.source_authorized, false);
  assert(
    result.output.warnings.some((warning) =>
      warning.includes("@3 is legacy context-only evidence"),
    ),
  );
});

nexusTest("v4 context selection accepts assigned capability domains and fails closed otherwise", () => {
  const { root, main } = fixture();
  producer(root);
  git(root, "branch", "integrate/nexus-add-provider-poc", main);
  const path = assignmentV4(root, main);
  const value = JSON.parse(readFileSync(path, "utf8"));
  value.capability_domain_ids = ["WS-01", "WS-04", "WS-06", "WS-07"];
  writeFileSync(path, JSON.stringify(value));

  const assigned = check(root, path, "WS-07");
  assert.equal(assigned.status, 0, JSON.stringify(assigned.output.errors, null, 2));
  assert.equal(assigned.output.context.workstream.id, "WS-07");
  assert.equal(assigned.output.context.source_authorized, false);

  const unassigned = check(root, path, "WS-02");
  assert.equal(unassigned.status, 1);
  assert(
    unassigned.output.errors.includes(
      "assignment does not include requested context WS-02",
    ),
  );
  assert.equal(unassigned.output.context.source_authorized, false);
});

nexusTest("legacy v3 assignment rejects a mismatched context", () => {
  const { root, main } = fixture();
  producer(root);
  git(root, "branch", "integrate/nexus-add-provider-poc", main);
  const result = check(root, assignmentV3(root, main), "WS-01");
  assert.equal(result.status, 1);
  assert(
    result.output.errors.includes(
      "assignment is for WS-07, not requested context WS-01",
    ),
  );
  assert.equal(result.output.context.source_authorized, false);
});

nexusTest("copied static assignments cannot create two source authorities", () => {
  const { root, main } = fixture();
  producer(root);
  git(root, "branch", "integrate/nexus-add-provider-poc", main);
  const firstPath = assignmentV4(root, main);
  const second = JSON.parse(readFileSync(firstPath, "utf8"));
  second.assignment_id = randomUUID();
  second.workstation_id = "ws01-live-copy";
  const secondPath = join(root, "assignment-copy.json");
  writeFileSync(secondPath, JSON.stringify(second));
  const first = check(root, firstPath, "WS-07");
  const copied = check(root, secondPath, "WS-07");
  for (const result of [first, copied]) {
    assert.equal(result.status, 0, JSON.stringify(result.output.errors, null, 2));
    assert.equal(result.output.assignment.source_admission_ready, true);
    assert.equal(result.output.assignment.source_authorized, false);
    assert.equal(
      result.output.assignment.authority_reason,
      "shared_live_lease_not_verified",
    );
  }
});

nexusTest("v4 selects a non-Nexus Product Increment from the portfolio", () => {
  const { root, main } = fixture();
  const portfolioPath = join(
    root,
    "docs/specs/product-increment-portfolio.json",
  );
  const portfolio = JSON.parse(readFileSync(portfolioPath, "utf8"));
  portfolio.product_increments.find(
    (item) => item.id === "AFS-PI-P1",
  ).lifecycle_state = "active";
  const planPath = join(
    root,
    "docs/specs/afs-transparent-routing.product-increment.json",
  );
  const plan = JSON.parse(readFileSync(planPath, "utf8"));
  plan.product_increment.planning_posture = "active";
  plan.lanes[0].status = "in_progress";
  writeFileSync(planPath, JSON.stringify(plan));
  reconcilePortfolioLaneCounts(root, portfolio);
  writeFileSync(portfolioPath, JSON.stringify(portfolio));
  const evidencePath = join(
    root,
    "docs/evidence/product-increments/afs-p1-router.json",
  );
  const evidence = JSON.parse(readFileSync(evidencePath, "utf8"));
  evidence.economics = {
    measurement_state: "unavailable",
    outcome: "candidate",
    retries: 0,
    corrections: 0,
    rework_cycles: 0,
    reason: "Fixture activates the held increment without claiming durable credit.",
  };
  writeFileSync(evidencePath, JSON.stringify(evidence));
  git(root, "checkout", "-qb", plan.lanes[0].branch);
  const now = Date.now();
  const value = {
    schema: "appfw_workstation_assignment@4",
    assignment_id: randomUUID(),
    portfolio_id: "app-framework-and-nexus",
    program_id: plan.program_id,
    product_increment_id: plan.product_increment.id,
    delivery_lane_id: plan.lanes[0].id,
    capability_domain_ids: [...plan.lanes[0].workstreams],
    workstation_id: "afs-router-live",
    lane_outcome_owner: plan.lanes[0].owner,
    technical_lead: plan.lanes[0].technical_lead,
    branch_owner: plan.lanes[0].branch_owner,
    independent_reviewer: plan.lanes[0].independent_reviewer,
    admitted_by: "Program Flow Controller",
    integration_owner: plan.integration.owner,
    branch: plan.lanes[0].branch,
    base_sha: main,
    integration_branch: "main",
    execution_target: "local_agent",
    planned_model_tier: "efficient-tier",
    routing_basis: "lowest_capable",
    economics_budget: structuredClone(plan.lanes[0].economics_budget),
    delegation_depth: 1,
    activated_at: new Date(now - 60 * 60 * 1000).toISOString(),
    review_after: new Date(now + 24 * 60 * 60 * 1000).toISOString(),
    status: "active",
  };
  const path = join(root, "afs-assignment.json");
  writeFileSync(path, JSON.stringify(value));
  const result = check(root, path);
  assert.equal(result.status, 0, JSON.stringify(result.output.errors, null, 2));
  assert.equal(result.output.assignment.product_increment_id, "AFS-PI-P1");
  assert.equal(result.output.assignment.source_admission_ready, true);
  assert.equal(result.output.assignment.source_authorized, false);
});

nexusTest("[integration-regression] self-authored accepted-main evidence cannot award durable credit", () => {
  const { root, main } = fixture();
  addImplementedRouterCredit(root, main);
  producer(root);
  git(root, "branch", "integrate/nexus-add-provider-poc", main);
  const assignmentPath = assignmentV4(root, main);

  const rejected = check(root, assignmentPath, "WS-07");
  assert.equal(rejected.status, 1);
  assert.equal(rejected.output.context.source_authorized, false);
  assert(
    rejected.output.errors.some((error) =>
      error.includes("requires a trusted external verifier"),
    ),
  );
});

nexusTest("[fixture-regression] completing router review preserves an unrelated review lane", () => {
  const { root, main } = fixture();
  const unrelatedPlanPath = join(
    root,
    "docs/specs/nexus-add-provider.product-increment.json",
  );
  const unrelatedPlan = JSON.parse(readFileSync(unrelatedPlanPath, "utf8"));
  unrelatedPlan.lanes[1].status = "under_review";
  writeFileSync(unrelatedPlanPath, JSON.stringify(unrelatedPlan));

  addImplementedRouterCredit(root, main);

  const portfolio = JSON.parse(
    readFileSync(
      join(root, "docs/specs/product-increment-portfolio.json"),
      "utf8",
    ),
  );
  assert.equal(portfolio.operating_capacity.current_review_lanes, 1);
  const expected = structuredClone(portfolio);
  reconcilePortfolioLaneCounts(root, expected);
  assert.equal(
    portfolio.operating_capacity.current_review_lanes,
    expected.operating_capacity.current_review_lanes,
  );
});

nexusTest("v4 rejects capability domains that do not supply the selected lane", () => {
  const { root, main } = fixture();
  producer(root);
  git(root, "branch", "integrate/nexus-add-provider-poc", main);
  const path = assignmentV4(root, main);
  const value = JSON.parse(readFileSync(path, "utf8"));
  value.capability_domain_ids = ["WS-03"];
  writeFileSync(path, JSON.stringify(value));
  const result = check(root, path);
  assert.equal(result.status, 1);
  assert(
    result.output.errors.includes(
      "assignment capability domain WS-03 does not supply lane NXP-L1-EXPERIENCE",
    ),
  );
});

nexusTest("expired active assignment grants no source authority", () => {
  const { root, main } = fixture();
  producer(root);
  git(root, "branch", "integrate/nexus-add-provider-poc", main);
  const path = assignmentV4(root, main);
  const value = JSON.parse(readFileSync(path, "utf8"));
  value.activated_at = new Date(Date.now() - 48 * 60 * 60 * 1000).toISOString();
  value.review_after = new Date(Date.now() - 24 * 60 * 60 * 1000).toISOString();
  writeFileSync(path, JSON.stringify(value));
  const result = check(root, path, "WS-07");
  assert.equal(result.status, 1);
  assert.equal(result.output.context.source_authorized, false);
  assert(
    result.output.errors.includes("active assignment review window has expired"),
  );
});

nexusTest("assignment identities must exactly match the Delivery Lane authorities", () => {
  const { root, main } = fixture();
  producer(root);
  git(root, "branch", "integrate/nexus-add-provider-poc", main);
  const path = assignmentV4(root, main);
  const value = JSON.parse(readFileSync(path, "utf8"));
  value.technical_lead = "Substitute Technical Lead";
  value.admitted_by = "Substitute Admission Authority";
  writeFileSync(path, JSON.stringify(value));
  const result = check(root, path, "WS-07");
  assert.equal(result.status, 1);
  assert.equal(result.output.context.source_authorized, false);
  assert(
    result.output.errors.includes(
      "assignment.technical_lead must be Nexus Experience Technical Lead",
    ),
  );
  assert(
    result.output.errors.includes(
      "assignment.admitted_by must be Program Flow Controller",
    ),
  );
});

nexusTest("assignment economics cannot exceed the Delivery Lane budget or silently escalate", () => {
  const { root, main } = fixture();
  producer(root);
  git(root, "branch", "integrate/nexus-add-provider-poc", main);
  const path = assignmentV4(root, main);
  const value = JSON.parse(readFileSync(path, "utf8"));
  value.planned_model_tier = "advanced-tier";
  value.economics_budget.default_model_tier = "advanced-tier";
  value.economics_budget.max_input_tokens += 1;
  writeFileSync(path, JSON.stringify(value));
  const result = check(root, path);
  assert.equal(result.status, 1);
  assert(
    result.output.errors.includes(
      "assignment.planned_model_tier must equal Delivery Lane default_model_tier for lowest_capable routing",
    ),
  );
  assert(
    result.output.errors.includes(
      "assignment.economics_budget.default_model_tier must equal Delivery Lane default_model_tier",
    ),
  );
  assert(
    result.output.errors.some((error) =>
      error.includes(
        "assignment.economics_budget.max_input_tokens exceeds Delivery Lane budget",
      ),
    ),
  );
});

nexusTest("ready lane is not counted or authorized as active source WIP", () => {
  const { root, main } = fixture();
  producer(root);
  git(root, "branch", "integrate/nexus-add-provider-poc", main);
  const path = assignmentV4(root, main);
  const planPath = join(
    root,
    "docs/specs/nexus-add-provider.product-increment.json",
  );
  const plan = JSON.parse(readFileSync(planPath, "utf8"));
  plan.lanes[0].status = "ready_to_start";
  writeFileSync(planPath, JSON.stringify(plan));
  const portfolioPath = join(
    root,
    "docs/specs/product-increment-portfolio.json",
  );
  const portfolio = JSON.parse(readFileSync(portfolioPath, "utf8"));
  reconcilePortfolioLaneCounts(root, portfolio);
  writeFileSync(portfolioPath, JSON.stringify(portfolio));
  const result = check(root, path, "WS-07");
  assert.equal(result.status, 1);
  assert.equal(result.output.context.source_authorized, false);
  assert(
    result.output.errors.includes(
      "active assignment requires lane NXP-L1-EXPERIENCE to be in_progress",
    ),
  );
  assert(
    !result.output.errors.some((error) =>
      error.includes("does not match plan-derived"),
    ),
  );
});

nexusTest("context-only mode grants no source write authority", () => {
  const { root } = fixture();
  const result = spawnSync(
    process.execPath,
    ["scripts/check-nexus-workstreams.mjs", "--context", "WS-01", "--json"],
    { cwd: root, encoding: "utf8" },
  );
  const output = JSON.parse(result.stdout);
  assert.equal(result.status, 0, JSON.stringify(output.errors, null, 2));
  assert.equal(output.ok, true);
  assert.equal(output.context.source_authorized, false);
  assert.equal(output.context.write_authority, "none_context_only");
  assert.deepEqual(output.context.allowed_write_roots, []);
  assert(output.context.capability_domain_reference_roots.length > 0);
});

nexusTest("blocked Product Increment cannot grant source authority", () => {
  const { root, main } = fixture();
  producer(root);
  const path = assignmentV4(root, main);
  const portfolioPath = join(
    root,
    "docs/specs/product-increment-portfolio.json",
  );
  const portfolio = JSON.parse(readFileSync(portfolioPath, "utf8"));
  portfolio.product_increments.find(
    (item) => item.id === "NXP-PI-ADD-PROVIDER-01",
  ).lifecycle_state = "blocked";
  writeFileSync(portfolioPath, JSON.stringify(portfolio));
  const result = check(root, path, "WS-07");
  assert.equal(result.status, 1);
  assert.equal(result.output.context.source_authorized, false);
  assert.deepEqual(result.output.context.allowed_write_roots, []);
  assert(
    result.output.errors.some((error) =>
      error.includes("not source-admissible active"),
    ),
  );
});

nexusTest("invalid Product Increment plan cannot grant source authority", () => {
  const { root, main } = fixture();
  producer(root);
  const path = assignmentV4(root, main);
  const planPath = join(
    root,
    "docs/specs/nexus-add-provider.product-increment.json",
  );
  const plan = JSON.parse(readFileSync(planPath, "utf8"));
  plan.schema = "appfw_product_increment_plan@2";
  writeFileSync(planPath, JSON.stringify(plan));
  const result = check(root, path, "WS-07");
  assert.equal(result.status, 1);
  assert.equal(result.output.context.source_authorized, false);
  assert.deepEqual(result.output.context.allowed_write_roots, []);
  assert(
    result.output.errors.some((error) =>
      error.includes("plan.schema must be appfw_product_increment_plan@1"),
    ),
  );
});

nexusTest("portfolio plan traversal cannot enter the source-authority path", () => {
  const { root, main } = fixture();
  producer(root);
  git(root, "branch", "integrate/nexus-add-provider-poc", main);
  const path = assignmentV4(root, main);
  const portfolioPath = join(
    root,
    "docs/specs/product-increment-portfolio.json",
  );
  const portfolio = JSON.parse(readFileSync(portfolioPath, "utf8"));
  portfolio.product_increments.find(
    (item) => item.id === "NXP-PI-ADD-PROVIDER-01",
  ).plan_ref = "../outside.product-increment.json";
  writeFileSync(portfolioPath, JSON.stringify(portfolio));
  const result = check(root, path, "WS-07");
  assert.equal(result.status, 1);
  assert.equal(result.output.context.source_authorized, false);
  assert(
    result.output.errors.some((error) =>
      error.includes("must be a local Product Increment plan filename"),
    ),
  );
});

nexusTest("unreachable capability evidence cannot authorize source", () => {
  const { root, main } = fixture();
  producer(root);
  git(root, "branch", "integrate/nexus-add-provider-poc", main);
  const path = assignmentV4(root, main);
  const missingSha = "a".repeat(40);
  const routerPlanPath = join(
    root,
    "docs/specs/afs-transparent-routing.product-increment.json",
  );
  const routerPlan = JSON.parse(readFileSync(routerPlanPath, "utf8"));
  routerPlan.lanes[0].provides[0].evidence_ref = `git:${missingSha}`;
  writeFileSync(routerPlanPath, JSON.stringify(routerPlan));
  const portfolioPath = join(
    root,
    "docs/specs/product-increment-portfolio.json",
  );
  const portfolio = JSON.parse(readFileSync(portfolioPath, "utf8"));
  const routerIncrement = portfolio.product_increments.find(
    (item) => item.id === "AFS-PI-P1",
  );
  routerIncrement.provides[0].evidence_ref = `git:${missingSha}`;
  routerIncrement.evidence.find((item) => item.kind === "git").ref = missingSha;
  writeFileSync(portfolioPath, JSON.stringify(portfolio));
  const result = check(root, path, "WS-07");
  assert.equal(result.status, 1);
  assert.equal(result.output.context.source_authorized, false);
  assert(
    result.output.errors.some((error) =>
      error.includes("not durably reachable"),
    ),
  );
});

nexusTest("invalid assignment emits no source roots", () => {
  const { root, main } = fixture();
  producer(root);
  const path = assignmentV4(root, main);
  const value = JSON.parse(readFileSync(path, "utf8"));
  value.independent_reviewer = value.branch_owner;
  writeFileSync(path, JSON.stringify(value));
  const result = check(root, path, "WS-07");
  assert.equal(result.status, 1);
  assert.equal(result.output.context.source_authorized, false);
  assert.deepEqual(result.output.context.allowed_write_roots, []);
});

nexusTest("rejects malformed Product Increment topology control", () => {
  const { root } = fixture();
  const topologyPath = join(
    root,
    "docs/specs/nexus-poc-workstream-topology.json",
  );
  const topology = JSON.parse(readFileSync(topologyPath, "utf8"));
  topology.product_increment_delivery.schema = "wrong";
  topology.product_increment_delivery.target_active_lanes = 99;
  topology.product_increment_delivery.proof_boundaries.leaf = "release";
  writeFileSync(topologyPath, JSON.stringify(topology));
  const result = spawnSync(
    process.execPath,
    ["scripts/check-nexus-workstreams.mjs", "--json"],
    { cwd: root, encoding: "utf8" },
  );
  const output = JSON.parse(result.stdout);
  assert.equal(result.status, 1);
  assert(
    output.errors.some((error) =>
      error.includes("product_increment_delivery.schema"),
    ),
  );
  assert(
    output.errors.some((error) =>
      error.includes("target_active_lanes must be an integer"),
    ),
  );
  assert(
    output.errors.some((error) =>
      error.includes("proof_boundaries must be accelerated"),
    ),
  );
});

nexusTest("rejects legacy workstation assignment schema v2", () => {
  const { root, main } = fixture();
  git(root, "checkout", "-qb", "feature/nexus-add-provider-experience");
  const path = assignmentV4(root, main);
  const value = JSON.parse(readFileSync(path, "utf8"));
  value.schema = "appfw_workstation_assignment@2";
  delete value.assignment_id;
  delete value.portfolio_id;
  delete value.product_increment_id;
  delete value.delivery_lane_id;
  delete value.capability_domain_ids;
  delete value.lane_outcome_owner;
  writeFileSync(path, JSON.stringify(value));
  const result = check(root, path);
  assert.equal(result.status, 1);
  assert(
    result.output.errors.includes(
      "assignment.schema must be appfw_workstation_assignment@4; @3 is read-only compatibility",
    ),
  );
});

nexusTest("v4 rejects an assignment to an unknown Delivery Lane", () => {
  const { root, main } = fixture();
  git(root, "checkout", "-qb", "feature/nexus-add-provider-experience");
  const result = check(root, assignmentV4(root, main, "NXP-L9-MISSING"));
  assert.equal(result.status, 1);
  assert(
    result.output.errors.includes(
      "assignment.delivery_lane_id is unknown: NXP-L9-MISSING",
    ),
  );
});

nexusTest("v4 rejects an assignment whose immutable base differs from its Delivery Lane", () => {
  const { root, main } = fixture();
  git(root, "checkout", "-qb", "feature/nexus-add-provider-experience");
  const path = assignmentV4(root, main);
  const value = JSON.parse(readFileSync(path, "utf8"));
  value.base_sha = "0".repeat(40);
  writeFileSync(path, JSON.stringify(value));
  const result = check(root, path, "WS-07");
  assert.equal(result.status, 1);
  assert.equal(result.output.context.source_authorized, false);
  assert.deepEqual(result.output.context.allowed_write_roots, []);
  assert(
    result.output.errors.some((error) =>
      error.includes("assignment.base_sha must equal lane base_sha"),
    ),
  );
});

nexusTest("v4 rejects Delivery Lane roots reserved for Integration", () => {
  const { root, main } = fixture();
  const planPath = join(
    root,
    "docs/specs/nexus-add-provider.product-increment.json",
  );
  const plan = JSON.parse(readFileSync(planPath, "utf8"));
  plan.lanes[0].write_roots.push("appfw_runtime/src/chat.rs");
  writeFileSync(planPath, JSON.stringify(plan));
  git(root, "checkout", "-qb", "feature/nexus-add-provider-experience");
  const result = check(root, assignmentV4(root, main));
  assert.equal(result.status, 1);
  assert(
    result.output.errors.some((error) =>
      error.includes("overlaps integration-only path appfw_runtime/src/chat.rs"),
    ),
  );
});

nexusTest("accepts exact local or agreeing local/remote named-integration tip", () => {
  for (const withRemote of [false, true]) {
    const { root } = fixture();
    git(root, "checkout", "-qb", "integrate/ws01-test");
    const base = commit(root, "integration");
    if (withRemote) git(root, "update-ref", "refs/remotes/origin/integrate/ws01-test", base);
    producer(root);
    const result = check(root, assignment(root, base));
    assert.equal(result.status, 0, JSON.stringify(result.output.errors, null, 2));
    assert.equal(result.output.ok, true);
    assert.equal(result.output.live_validation.local_integration_tip, base);
    assert.equal(result.output.live_validation.remote_integration_tip, withRemote ? base : null);
  }
});

nexusTest("accepts sibling main advancement and reports the unique accepted-main merge base", () => {
  const { root, main } = fixture();
  git(root, "checkout", "-qb", "integrate/ws01-test");
  const base = commit(root, "integration");
  git(root, "checkout", "main");
  const currentMain = commit(root, "main-advanced");
  git(root, "update-ref", "refs/remotes/origin/main", currentMain);
  git(root, "checkout", "integrate/ws01-test");
  producer(root);
  commit(root, "producer");

  const result = check(root, assignment(root, base));
  assert.equal(result.status, 0, JSON.stringify(result.output.errors, null, 2));
  assert.equal(result.output.live_validation.route, "named_integration");
  assert.equal(result.output.live_validation.local_integration_tip, base);
  assert.equal(result.output.live_validation.accepted_main_merge_base, main);
});

nexusTest("rejects a base that is not an ancestor of producer HEAD", () => {
  const { root } = fixture();
  git(root, "checkout", "-qb", "integrate/ws01-test");
  const base = commit(root, "integration");
  git(
    root,
    "checkout",
    "-qb",
    "feature/nexus-add-provider-experience",
    "origin/main",
  );
  const result = check(root, assignment(root, base));
  assert.equal(result.status, 1);
  assert(result.output.errors.includes("assignment base SHA is not an ancestor of HEAD"));
});

nexusTest("rejects missing, mismatched, and drifting named integration refs distinctly", () => {
  const { root } = fixture();
  git(root, "checkout", "-qb", "integrate/ws01-other");
  const base = commit(root, "integration");
  producer(root);

  let result = check(root, assignment(root, base));
  assert(result.output.errors.some((error) => error.startsWith("named integration ref is missing:")));

  git(root, "branch", "integrate/ws01-test", "origin/main");
  result = check(root, assignment(root, base));
  assert(result.output.errors.some((error) => error.startsWith("assignment base SHA does not equal named integration tip:")));

  git(root, "update-ref", "refs/remotes/origin/integrate/ws01-test", base);
  result = check(root, assignment(root, base));
  assert(result.output.errors.some((error) => error.startsWith("local/remote integration ref drift")));
});

nexusTest("rejects a matching remote-only integration ref", () => {
  const { root } = fixture();
  git(root, "checkout", "-qb", "integrate/ws01-other");
  const base = commit(root, "integration");
  git(root, "update-ref", "refs/remotes/origin/integrate/ws01-test", base);
  producer(root);
  const result = check(root, assignment(root, base));
  assert.equal(result.status, 1);
  assert(result.output.errors.some((error) => error.startsWith("named integration ref is missing:")));
  assert.equal(result.output.live_validation.local_integration_tip, null);
  assert.equal(result.output.live_validation.remote_integration_tip, base);
});

nexusTest("rejects an unrelated integration lineage and similarly named containing ref", () => {
  const { root } = fixture();
  git(root, "checkout", "--orphan", "unrelated");
  git(root, "rm", "-qf", "seed");
  writeFileSync(join(root, "unrelated"), "unrelated\n");
  git(root, "add", "unrelated");
  git(root, "commit", "-qm", "unrelated");
  const base = git(root, "rev-parse", "HEAD");
  git(root, "branch", "integrate/ws01-test-similar", base);
  git(root, "checkout", "-qb", "feature/nexus-add-provider-experience");

  let result = check(root, assignment(root, base));
  assert(result.output.errors.some((error) => error.startsWith("named integration ref is missing:")));

  git(root, "branch", "integrate/ws01-test", base);
  result = check(root, assignment(root, base));
  assert(result.output.errors.includes("named integration tip is not based on accepted origin/main"));
});

nexusTest("rejects malformed integration branch input without resolving it", () => {
  const { root, main } = fixture();
  producer(root);
  const path = assignment(root, main, "integrate/ws01-test..lock");
  const result = check(root, path);
  assert.equal(result.status, 1);
  assert(result.output.errors.includes("assignment.integration_branch must start with integrate/"));
  assert.equal(readFileSync(path, "utf8").includes("..lock"), true);
});

nexusTest("rejects ambiguous criss-cross accepted-main merge bases", () => {
  const { root, main } = fixture();
  const tree = git(root, "rev-parse", `${main}^{tree}`);
  const left = commitTree(root, tree, [main], "left");
  const right = commitTree(root, tree, [main], "right");
  const integration = commitTree(root, tree, [left, right], "integration-merge");
  const currentMain = commitTree(root, tree, [right, left], "main-merge");
  git(root, "update-ref", "refs/heads/integrate/ws01-test", integration);
  git(root, "update-ref", "refs/remotes/origin/main", currentMain);
  git(
    root,
    "checkout",
    "-qb",
    "feature/nexus-add-provider-experience",
    integration,
  );

  const result = check(root, assignment(root, integration));
  assert.equal(result.status, 1);
  assert(result.output.errors.includes("named integration lineage has ambiguous accepted-main merge bases: 2"));
  assert.equal(result.output.live_validation.accepted_main_merge_base, null);
});

assertExactNexusWorkstreamTestManifest(registeredTestNames);

});
