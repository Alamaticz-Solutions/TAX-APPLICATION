#!/usr/bin/env node

import assert from "node:assert/strict";
import { execFileSync } from "node:child_process";
import { createHash } from "node:crypto";
import {
  mkdirSync,
  mkdtempSync,
  readFileSync,
  rmSync,
  symlinkSync,
  unlinkSync,
  writeFileSync,
} from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

import {
  readCurrentDiff,
  validateProductIncrementPlan,
} from "./check-product-increment-plan.mjs";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const example = JSON.parse(
  readFileSync(
    join(root, "docs/specs/nexus-add-provider.product-increment.json"),
    "utf8",
  ),
);
const schema = JSON.parse(
  readFileSync(
    join(root, "docs/start/product-increment-plan.schema.json"),
    "utf8",
  ),
);
const evidenceSchema = JSON.parse(
  readFileSync(
    join(root, "docs/start/product-increment-evidence.schema.json"),
    "utf8",
  ),
);
const EVIDENCE_DOCUMENTS = new Map(
  [
    ["proof", "retained Product Increment execution proof\n"],
    ["metering", "trusted provider usage receipt\n"],
    ["destination", "trusted destination verification receipt\n"],
    ["review", "retained independent review receipt\n"],
    ["lane-acceptance", "retained lane acceptance receipt\n"],
    ["product-acceptance", "retained Product acceptance receipt\n"],
  ].map(([kind, text]) => [
    `docs/evidence/product-increments/unit-${kind}.txt`,
    Buffer.from(text),
  ]),
);

function clone(value) {
  return structuredClone(value);
}

function expectError(plan, fragment, options = {}) {
  const result = validateProductIncrementPlan(plan, options);
  assert.equal(result.ok, false, `expected invalid plan containing ${fragment}`);
  assert.ok(
    result.errors.some((error) => error.includes(fragment)),
    `missing error containing "${fragment}":\n${result.errors.join("\n")}`,
  );
}

function git(cwd, ...args) {
  return execFileSync("git", args, { cwd, encoding: "utf8" }).trim();
}

function allowedChangedPath(lane) {
  const root = lane.write_roots[0];
  return root.endsWith("/**")
    ? `${root.slice(0, -3)}/unit-change.ts`
    : root;
}

function retainedEvidenceFields(kind) {
  const path = `docs/evidence/product-increments/unit-${kind}.txt`;
  const bytes = EVIDENCE_DOCUMENTS.get(path);
  return {
    evidence_ref: `document:${path}`,
    evidence_sha256: createHash("sha256").update(bytes).digest("hex"),
  };
}

function eligibleReview(lane, candidateSha = "a".repeat(40)) {
  return {
    reviewer: lane.independent_reviewer,
    reviewed_sha: candidateSha,
    disposition: "go",
    completed_at: "2026-07-23T18:05:00Z",
    findings: {
      blocker: 0,
      critical: 0,
      important: 0,
      should_address: 0,
      nice_to_address: 0,
    },
    ...retainedEvidenceFields("review"),
  };
}

function acceptanceReceipt(
  owner,
  candidateSha = "a".repeat(40),
  kind = "lane-acceptance",
) {
  return {
    owner,
    decision: "accepted",
    accepted_sha: candidateSha,
    decided_at: "2026-07-23T18:10:00Z",
    ...retainedEvidenceFields(kind),
  };
}

function destinationVerification(plan, lane, candidateSha = "a".repeat(40)) {
  return {
    destination_branch:
      lane.destination_branch ??
      plan.integration.destination_branch ??
      plan.integration.branch,
    candidate_sha: candidateSha,
    verified_at: "2026-07-23T18:08:00Z",
    ...retainedEvidenceFields("destination"),
  };
}

function evidenceRecord(
  plan,
  lane,
  {
    candidateSha = "a".repeat(40),
    baseSha = lane.base_sha,
    profile = plan.integration.leaf_ci_profile,
    review = null,
    laneAcceptance = null,
    productAcceptance = null,
    destinationReceipt = undefined,
  } = {},
) {
  const outcome =
    lane.delivery_credit === "none" ? "candidate" : lane.delivery_credit;
  const requiredCommands = [
    ...lane.focused_checks,
    ...(plan.integration.strategy === "direct_main"
      ? plan.integration.integration_checks
      : []),
  ];
  return {
    schema: "appfw_product_increment_evidence@1",
    product_increment_id: plan.product_increment.id,
    outcome_id: plan.product_increment.outcome_id,
    scope: {
      kind:
        plan.integration.strategy === "direct_main"
          ? "direct_main_candidate"
          : "delivery_lane",
      ...(plan.integration.strategy === "direct_main"
        ? {}
        : { lane_id: lane.id }),
    },
    candidate_sha: candidateSha,
    base_sha: baseSha,
    proof: {
      stage: plan.product_increment.stage,
      profile,
      status: "passed",
      completed_at: "2026-07-23T18:00:00Z",
      checks: requiredCommands.map((command) => ({
          command,
          result: "passed",
          output_ref: retainedEvidenceFields("proof").evidence_ref,
          output_sha256: retainedEvidenceFields("proof").evidence_sha256,
      })),
    },
    economics:
      outcome === "accepted"
        ? {
            measurement_state: "measured",
            outcome,
            planned_model: "efficient-tier",
            actual_model: "efficient-tier",
            reasoning_effort: "medium",
            routing_basis: "lowest_capable",
            metering_receipt_ref:
              retainedEvidenceFields("metering").evidence_ref,
            metering_receipt_sha256:
              retainedEvidenceFields("metering").evidence_sha256,
            elapsed_ms: 1000,
            input_tokens: 100,
            output_tokens: 50,
            cost_usd: 0.01,
            retries: 0,
            corrections: 0,
            rework_cycles: 0,
            context_source_count: 4,
            delegation_depth: 1,
          }
        : {
            measurement_state: "unavailable",
            outcome,
            retries: 0,
            corrections: 0,
            rework_cycles: 0,
            reason: "The unit fixture does not execute a billable model.",
          },
    destination_verification:
      destinationReceipt === undefined
        ? outcome === "candidate" || outcome === "rejected"
          ? null
          : destinationVerification(plan, lane, candidateSha)
        : destinationReceipt,
    review,
    lane_acceptance: laneAcceptance,
    product_acceptance: productAcceptance,
  };
}

function evidenceOptions(record) {
  return {
    evidenceExists: () => true,
    gitChangedPaths: () => [
      allowedChangedPath(
        example.lanes.find(
          (lane) =>
            record.scope.lane_id === undefined ||
            lane.id === record.scope.lane_id,
        ) ?? example.lanes[0],
      ),
    ],
    gitEvidenceExists: () => true,
    gitIsAncestor: () => true,
    acceptedEvidenceExists: () => true,
    loadAcceptedEvidenceBytes: (path) => EVIDENCE_DOCUMENTS.get(path),
    loadAcceptedEvidenceRecord: () => record,
    loadEvidenceBytes: (path) => EVIDENCE_DOCUMENTS.get(path),
    loadEvidenceRecord: () => record,
    pathIsReviewable: () => true,
    verifyAuthorityReceipt: () => true,
    verifyDestinationReceipt: () => true,
    verifyUsageReceipt: () => true,
  };
}

{
  const result = validateProductIncrementPlan(example);
  assert.equal(result.ok, true, result.errors.join("\n"));
  assert.equal(result.summary.lane_count, 4);
  assert.equal(result.summary.required_link_count, 3);
  assert.equal(result.summary.benefit_link_count, 1);
}

{
  const plan = clone(example);
  const fifth = clone(plan.lanes[0]);
  fifth.id = "NXP-L5-EXTRA";
  fifth.branch = "feature/nexus-extra";
  fifth.branch_owner = "Extra Branch Owner";
  fifth.independent_reviewer = "Extra Reviewer";
  fifth.write_roots = ["examples/products/pds-nexus/extra/**"];
  fifth.provides[0].capability_id = "nexus.extra";
  plan.lanes.push(fifth);
  plan.integration.merge_order.push(fifth.id);
  expectError(plan, "between 1 and 4 Delivery Lanes");
}

{
  const plan = clone(example);
  plan.product_increment.planning_posture = "active";
  plan.lanes[0].status = "in_progress";
  plan.lanes[1].status = "in_progress";
  plan.lanes[1].write_roots = ["examples/products/pds-nexus/frontend/providers/**"];
  expectError(plan, "overlap write roots");
}

{
  const plan = clone(example);
  plan.lanes[0].status = "in_progress";
  plan.lane_links[0].decoupling = { mode: "none" };
  expectError(plan, "waiting on unfinished provider");
}

{
  const plan = clone(example);
  plan.lane_links[0].capability_id = "missing.capability";
  expectError(plan, "has no declared provider");
}

{
  const plan = clone(example);
  plan.lanes[0].destination_branch = "main";
  expectError(plan, "destination_branch must equal");
}

{
  const plan = clone(example);
  plan.lanes[0].independent_reviewer = plan.lanes[0].branch_owner;
  expectError(plan, "branch owner and independent reviewer must differ");
}

{
  const plan = clone(example);
  plan.lanes[0].status = "waiting_for_team";
  plan.lanes[1].status = "waiting_for_team";
  plan.lanes[0].provides[0].state = "planned";
  plan.lanes[1].provides[0].state = "planned";
  plan.lane_links.push({
    provider_lane: "NXP-L1-EXPERIENCE",
    consumer_lane: "NXP-L2-SERVICENOW",
    kind: "requires",
    capability_id: "nexus.add-provider.experience",
    required_provider_state: "implemented",
    contract_ref: "Cycle fixture",
    decoupling: { mode: "none" },
    consumer_action: "Wait",
    integration_check: "Cycle must fail",
  });
  plan.lane_links[0].decoupling = { mode: "none" };
  expectError(plan, "unresolved hard dependency cycle");
}

{
  const plan = clone(example);
  plan.lanes = [plan.lanes[0]];
  plan.concurrency = {
    target_active_lanes: 1,
    maximum_active_lanes: 1,
  };
  plan.integration = {
    ...plan.integration,
    strategy: "direct_main",
    branch: "main",
    leaf_ci_profile: "full",
    merge_order: [plan.lanes[0].id],
  };
  plan.lanes[0].destination_branch = "main";
  plan.lanes[0].workstreams = ["Nexus experience"];
  plan.lane_links = [];
  const result = validateProductIncrementPlan(plan);
  assert.equal(result.ok, true, result.errors.join("\n"));
  assert.equal(result.summary.integration_strategy, "direct_main");
}

{
  const destinationPattern = new RegExp(
    schema.$defs.lane.properties.destination_branch.pattern,
  );
  assert.equal(destinationPattern.test("integrate/nexus-add-provider-poc"), true);
  assert.equal(destinationPattern.test("main"), true);
  assert.equal(destinationPattern.test("feature/not-a-destination"), false);
}

{
  const plan = clone(example);
  plan.integration.strategy = "direct_main";
  plan.integration.branch = "main";
  plan.integration.leaf_ci_profile = "full";
  for (const lane of plan.lanes) lane.destination_branch = "main";
  expectError(plan, "multiple Delivery Lanes must use an integration branch");
}

{
  const plan = clone(example);
  plan.lanes = [plan.lanes[0]];
  plan.concurrency = {
    target_active_lanes: 1,
    maximum_active_lanes: 1,
  };
  plan.integration = {
    ...plan.integration,
    strategy: "direct_main",
    branch: "main",
    leaf_ci_profile: "full",
    merge_order: [plan.lanes[0].id],
  };
  plan.lanes[0].destination_branch = "main";
  plan.lane_links = [];
  expectError(plan, "cross-domain Product Increment");
}

{
  const plan = clone(example);
  plan.lanes[0].write_roots.push("scripts/ci/**");
  const result = validateProductIncrementPlan(plan);
  assert.equal(result.ok, true, result.errors.join("\n"));
  assert.deepEqual(result.summary.sensitive_write_roots, [
    "scripts/ci/**",
    "appfw_runtime/src/chat.rs",
  ]);
  plan.product_increment.stage = "release_candidate";
  expectError(plan, "release-candidate Product Increments must use full leaf CI");
}

{
  const plan = clone(example);
  plan.unexpected = true;
  plan.lanes[0].unexpected = true;
  expectError(plan, "plan.unexpected is not allowed");
  expectError(plan, "plan.lanes[0].unexpected is not allowed");
}

{
  const plan = clone(example);
  plan.lanes[0].status = "complete";
  expectError(plan, "status is not recognized");
}

{
  const plan = clone(example);
  plan.product_increment.id = "NXP PI WITH SPACES";
  expectError(plan, "id does not match the required format");
}

{
  const plan = clone(example);
  plan.lanes[0].delivery_credit = "implemented";
  plan.lanes[0].provides[0].state = "implemented";
  plan.lanes[0].provides[0].evidence_ref = { unsafe: true };
  expectError(plan, "evidence_ref must be a string when present");
}

{
  const plan = clone(example);
  delete plan.lanes[0].technical_lead;
  expectError(plan, "technical_lead must be a non-empty string");
}

{
  for (const unsafeRoot of [
    "../../outside/**",
    "/absolute/path",
    "**",
    "safe/*/unsafe",
    "safe/../outside",
    "safe/[abc]",
    "safe\\windows",
  ]) {
    const plan = clone(example);
    plan.lanes[0].write_roots = [unsafeRoot];
    expectError(plan, "must be an exact repository-relative path");
  }
}

{
  const plan = clone(example);
  plan.lanes[0].status = "implemented";
  plan.lanes[0].delivery_credit = "implemented";
  for (const capability of plan.lanes[0].provides) {
    capability.state = "implemented";
    capability.evidence_ref = `git:${"a".repeat(40)}`;
  }
  expectError(plan, "Git evidence is not durably reachable", {
    gitEvidenceExists: () => false,
  });
}

{
  const plan = clone(example);
  plan.lanes[0].status = "implemented";
  plan.lanes[0].delivery_credit = "implemented";
  for (const capability of plan.lanes[0].provides) {
    capability.state = "implemented";
    capability.evidence_ref = "document:target/appfw/transient.json";
  }
  expectError(plan, "must be a durable repository-relative path", {
    evidenceExists: () => true,
  });
}

{
  const plan = clone(example);
  plan.lanes[0].write_roots.push(
    "docs/specs/example.product-increment.json",
  );
  const result = validateProductIncrementPlan(plan);
  assert.equal(result.ok, true, result.errors.join("\n"));
  assert.ok(
    result.summary.sensitive_write_roots.includes(
      "docs/specs/example.product-increment.json",
    ),
  );
}

{
  const plan = clone(example);
  const lane = plan.lanes[0];
  lane.status = "under_review";
  lane.delivery_credit = "none";
  lane.evidence_record_ref =
    "document:docs/evidence/product-increments/candidate.json";
  const result = validateProductIncrementPlan(
    plan,
    evidenceOptions(evidenceRecord(plan, lane)),
  );
  assert.equal(result.ok, true, result.errors.join("\n"));
}

{
  const plan = clone(example);
  const lane = plan.lanes[0];
  lane.status = "implemented";
  lane.delivery_credit = "implemented";
  lane.evidence_record_ref =
    "document:docs/evidence/product-increments/candidate.json";
  for (const capability of lane.provides) {
    capability.state = "implemented";
    capability.evidence_ref = `git:${"a".repeat(40)}`;
  }
  expectError(
    plan,
    "requires an eligible exact-candidate independent review",
    evidenceOptions(evidenceRecord(plan, lane)),
  );
}

{
  const plan = clone(example);
  const lane = plan.lanes[0];
  lane.status = "under_review";
  lane.evidence_record_ref =
    "document:docs/evidence/product-increments/candidate.json";
  const record = evidenceRecord(plan, lane);
  delete record.economics;
  expectError(plan, "economics must be an object", evidenceOptions(record));
}

{
  const plan = clone(example);
  const lane = plan.lanes[0];
  lane.status = "under_review";
  lane.evidence_record_ref =
    "document:docs/evidence/product-increments/candidate.json";
  const record = evidenceRecord(plan, lane);
  record.economics = {
    measurement_state: "measured",
    outcome: "candidate",
    planned_model: "efficient-tier",
    actual_model: "efficient-tier",
    reasoning_effort: "medium",
    elapsed_ms: 1000,
    input_tokens: 100,
    output_tokens: 50,
    retries: 0,
    corrections: 0,
    rework_cycles: 0,
    context_source_count: 4,
    delegation_depth: 1,
  };
  expectError(
    plan,
    "cost_usd must be a non-negative number when measured",
    evidenceOptions(record),
  );
}

{
  const plan = clone(example);
  const lane = plan.lanes[0];
  lane.status = "accepted";
  lane.delivery_credit = "accepted";
  lane.evidence_record_ref =
    "document:docs/evidence/product-increments/candidate.json";
  for (const capability of lane.provides) {
    capability.state = "accepted";
    capability.evidence_ref = `git:${"a".repeat(40)}`;
  }
  const review = {
    reviewer: lane.independent_reviewer,
    reviewed_sha: "a".repeat(40),
    disposition: "go",
    completed_at: "2026-07-23T18:05:00Z",
    findings: {
      blocker: 0,
      critical: 0,
      important: 0,
      should_address: 0,
      nice_to_address: 0,
    },
  };
  const wrongAcceptance = {
    owner: "Unrelated approver",
    decision: "accepted",
    accepted_sha: "a".repeat(40),
    decided_at: "2026-07-23T18:10:00Z",
  };
  expectError(
    plan,
    `accepted requires an acceptance receipt from ${lane.owner}`,
    evidenceOptions(
      evidenceRecord(plan, lane, {
        review,
        laneAcceptance: wrongAcceptance,
      }),
    ),
  );
}

{
  const plan = clone(example);
  const lane = plan.lanes[0];
  lane.status = "under_review";
  lane.evidence_record_ref =
    "document:docs/evidence/product-increments/candidate.json";
  expectError(
    plan,
    "proof.profile must be accelerated",
    evidenceOptions(evidenceRecord(plan, lane, { profile: "full" })),
  );
}

{
  const plan = clone(example);
  expectError(plan, "does not exist:", {
    evidenceExists: () => false,
  });
}

{
  const plan = clone(example);
  plan.lanes[0].write_roots.push("docs/specs/ordinary-component-contract.md");
  const result = validateProductIncrementPlan(plan);
  assert.equal(result.ok, true, result.errors.join("\n"));
}

{
  for (const reservedRoot of [
    ".git/**",
    "target/appfw/**",
    "packages/example/node_modules/**",
  ]) {
    const plan = clone(example);
    plan.lanes[0].write_roots = [reservedRoot];
    expectError(plan, "must be an exact repository-relative path");
  }
}

{
  const plan = clone(example);
  expectError(plan, "ignored, reserved, or traverses a symlink", {
    pathIsReviewable: () => false,
  });
}

{
  const plan = clone(example);
  plan.lanes[0].status = "not_started";
  plan.lanes[0].delivery_credit = "accepted";
  for (const capability of plan.lanes[0].provides) {
    capability.state = "accepted";
    capability.evidence_ref = "evidence:invalid-acceptance";
  }
  expectError(plan, "status not_started permits delivery_credit none");
}

{
  const plan = clone(example);
  plan.lanes[0].status = "implemented";
  plan.lanes[0].delivery_credit = "implemented";
  expectError(plan, "state must be implemented when delivery_credit is implemented");
}

{
  const plan = clone(example);
  plan.integration.merge_order = [
    "NXP-L1-EXPERIENCE",
    "NXP-L4-INTELLIGENCE",
    "NXP-L3-WORKDAY",
    "NXP-L2-SERVICENOW",
  ];
  expectError(plan, "must place required provider NXP-L2-SERVICENOW before consumer");
}

{
  const plan = clone(example);
  plan.integration.base_ref = "origin/main";
  expectError(plan, "base_ref must be an immutable full lowercase Git SHA");
}

{
  const plan = clone(example);
  plan.lanes[0].base_sha = "origin/main";
  expectError(plan, "base_sha must be a full lowercase Git SHA");
}

{
  const safeResult = validateProductIncrementPlan(example, {
    actualChangedPaths: [
      "examples/products/pds-nexus/frontend/src/App.tsx",
    ],
    currentDiffLaneId: "NXP-L1-EXPERIENCE",
  });
  assert.equal(safeResult.ok, true, safeResult.errors.join("\n"));
  assert.deepEqual(safeResult.summary.observed_sensitive_paths, []);

  expectError(
    example,
    "outside Delivery Lane NXP-L1-EXPERIENCE write_roots",
    {
      actualChangedPaths: ["scripts/ci/hidden-change.sh"],
      currentDiffLaneId: "NXP-L1-EXPERIENCE",
    },
  );
}

{
  const repository = mkdtempSync(join(tmpdir(), "appfw-pi-current-diff-"));
  try {
    git(repository, "init", "-q");
    git(repository, "config", "user.email", "test@example.com");
    git(repository, "config", "user.name", "Product Increment Test");
    mkdirSync(join(repository, "old"), { recursive: true });
    mkdirSync(join(repository, "new"), { recursive: true });
    writeFileSync(join(repository, "old/path.txt"), "before\n");
    writeFileSync(join(repository, "new/type-target"), "regular\n");
    git(repository, "add", ".");
    git(repository, "commit", "-qm", "base");
    const base = git(repository, "rev-parse", "HEAD");
    git(repository, "checkout", "-qb", "feature/current-diff-test");
    git(repository, "mv", "old/path.txt", "new/renamed.txt");
    unlinkSync(join(repository, "new/type-target"));
    symlinkSync("renamed.txt", join(repository, "new/type-target"));

    const plan = clone(example);
    plan.integration.base_ref = base;
    plan.lanes[0].branch = "feature/current-diff-test";
    plan.lanes[0].base_sha = base;
    const paths = readCurrentDiff(plan, plan.lanes[0].id, repository);
    assert(paths.includes("old/path.txt"), "rename source path must be checked");
    assert(paths.includes("new/renamed.txt"), "rename destination must be checked");
    assert(paths.includes("new/type-target"), "type-change path must be checked");
  } finally {
    rmSync(repository, { recursive: true, force: true });
  }
}

{
  const plan = clone(example);
  plan.lanes[0].write_roots = ["AGENTS.md"];
  expectError(plan, "outside Delivery Lane NXP-L1-EXPERIENCE write_roots", {
    actualChangedPaths: ["AGENTS.md/unauthorized.txt"],
    currentDiffLaneId: "NXP-L1-EXPERIENCE",
  });
}

{
  const plan = clone(example);
  plan.lanes[0].write_roots = ["examples/products/pds-nexus/frontend/**"];
  const descendant = validateProductIncrementPlan(plan, {
    actualChangedPaths: [
      "examples/products/pds-nexus/frontend/src/authorized.ts",
    ],
    currentDiffLaneId: "NXP-L1-EXPERIENCE",
  });
  assert.equal(descendant.ok, true, descendant.errors.join("\n"));
  expectError(plan, "outside Delivery Lane NXP-L1-EXPERIENCE write_roots", {
    actualChangedPaths: ["examples/products/pds-nexus"],
    currentDiffLaneId: "NXP-L1-EXPERIENCE",
  });
}

{
  const plan = clone(example);
  const lane = plan.lanes[0];
  lane.status = "implemented";
  lane.delivery_credit = "implemented";
  lane.evidence_record_ref =
    "document:docs/evidence/product-increments/candidate.json";
  for (const capability of lane.provides) {
    capability.state = "implemented";
    capability.evidence_ref = `git:${"a".repeat(40)}`;
  }
  const record = evidenceRecord(plan, lane, {
    review: eligibleReview(lane),
  });
  record.proof.checks = [
    {
      command: "printf 'looks green but proves nothing'",
      result: "passed",
      output_ref: retainedEvidenceFields("proof").evidence_ref,
      output_sha256: retainedEvidenceFields("proof").evidence_sha256,
    },
  ];
  expectError(
    plan,
    "proof.checks must include declared check",
    evidenceOptions(record),
  );
}

{
  const plan = clone(example);
  const lane = plan.lanes[0];
  lane.status = "implemented";
  lane.delivery_credit = "implemented";
  lane.evidence_record_ref =
    "document:docs/evidence/product-increments/candidate.json";
  for (const capability of lane.provides) {
    capability.state = "implemented";
    capability.evidence_ref = `git:${"a".repeat(40)}`;
  }
  const record = evidenceRecord(plan, lane, {
    review: eligibleReview(lane),
  });
  record.proof.checks[0].output_sha256 = "d".repeat(64);
  expectError(
    plan,
    "sha256 does not match the retained document bytes",
    evidenceOptions(record),
  );
}

{
  const plan = clone(example);
  const lane = plan.lanes[0];
  lane.status = "implemented";
  lane.delivery_credit = "implemented";
  lane.evidence_record_ref =
    "document:docs/evidence/product-increments/candidate.json";
  for (const capability of lane.provides) {
    capability.state = "implemented";
    capability.evidence_ref = `git:${"a".repeat(40)}`;
  }
  const record = evidenceRecord(plan, lane, {
    review: eligibleReview(lane),
  });
  expectError(plan, "candidate changed path scripts/unauthorized.mjs", {
    ...evidenceOptions(record),
    gitChangedPaths: () => ["scripts/unauthorized.mjs"],
  });
}

{
  const plan = clone(example);
  const lane = plan.lanes[0];
  lane.status = "accepted";
  lane.delivery_credit = "accepted";
  lane.evidence_record_ref =
    "document:docs/evidence/product-increments/candidate.json";
  for (const capability of lane.provides) {
    capability.state = "accepted";
    capability.evidence_ref = `git:${"a".repeat(40)}`;
  }
  const record = evidenceRecord(plan, lane, {
    review: eligibleReview(lane),
    laneAcceptance: acceptanceReceipt(lane.owner),
    productAcceptance: acceptanceReceipt(
      plan.product_increment.owner,
      "a".repeat(40),
      "product-acceptance",
    ),
  });
  record.economics = {
    measurement_state: "unavailable",
    outcome: "accepted",
    retries: 0,
    corrections: 0,
    rework_cycles: 0,
    reason: "No trustworthy usage or cost telemetry was retained.",
  };
  expectError(
    plan,
    "economics must be measured for durable delivery credit",
    evidenceOptions(record),
  );
}

{
  const plan = clone(example);
  const lane = plan.lanes[0];
  lane.status = "accepted";
  lane.delivery_credit = "accepted";
  lane.evidence_record_ref =
    "document:docs/evidence/product-increments/candidate.json";
  for (const capability of lane.provides) {
    capability.state = "accepted";
    capability.evidence_ref = `git:${"a".repeat(40)}`;
  }
  const record = evidenceRecord(plan, lane, {
    review: eligibleReview(lane),
    laneAcceptance: acceptanceReceipt(lane.owner),
  });
  const result = validateProductIncrementPlan(plan, evidenceOptions(record));
  assert.equal(result.ok, false, result.errors.join("\n"));
  assert.ok(
    result.errors.some((error) =>
      error.includes(
        `accepted requires an acceptance receipt from ${plan.product_increment.owner}`,
      ),
    ),
  );
  assert.equal(result.summary.accepted_lane_count, 0);
}

{
  const plan = clone(example);
  const lane = plan.lanes[0];
  lane.status = "waiting_for_decision";
  lane.delivery_credit = "none";
  lane.evidence_record_ref =
    "document:docs/evidence/product-increments/candidate.json";
  const record = evidenceRecord(plan, lane);
  record.economics.corrections = lane.economics_budget.max_corrections + 1;
  expectError(
    plan,
    "economics.corrections exceeds max_corrections",
    evidenceOptions(record),
  );
}

{
  const plan = clone(example);
  const lane = plan.lanes[0];
  lane.status = "implemented";
  lane.delivery_credit = "implemented";
  lane.evidence_record_ref =
    "document:docs/evidence/product-increments/candidate.json";
  for (const capability of lane.provides) {
    capability.state = "implemented";
    capability.evidence_ref = `git:${"a".repeat(40)}`;
  }
  const record = evidenceRecord(plan, lane, {
    review: eligibleReview(lane),
  });
  record.economics = {
    measurement_state: "measured",
    outcome: "implemented",
    planned_model: "efficient-tier",
    actual_model: "efficient-tier",
    reasoning_effort: "medium",
    routing_basis: "lowest_capable",
    elapsed_ms: 1000,
    input_tokens: 100,
    output_tokens: 50,
    cost_usd: 0.01,
    retries: 0,
    corrections: 0,
    rework_cycles: 0,
    context_source_count: 999,
    delegation_depth: 99,
  };
  const result = validateProductIncrementPlan(plan, evidenceOptions(record));
  assert.equal(result.ok, false);
  assert.ok(
    result.errors.some((error) =>
      error.includes("context_source_count must not exceed 4"),
    ),
    result.errors.join("\n"),
  );
  assert.ok(
    result.errors.some((error) =>
      error.includes("delegation_depth must not exceed 1"),
    ),
    result.errors.join("\n"),
  );
}

{
  const plan = clone(example);
  const lane = plan.lanes[0];
  lane.status = "implemented";
  lane.delivery_credit = "implemented";
  lane.evidence_record_ref =
    "document:docs/evidence/product-increments/candidate.json";
  for (const capability of lane.provides) {
    capability.state = "implemented";
    capability.evidence_ref = `git:${"a".repeat(40)}`;
  }
  const review = eligibleReview(lane);
  review.reviewer = "Producer pretending to be reviewer";
  const record = evidenceRecord(plan, lane, { review });
  expectError(
    plan,
    `review.reviewer must be ${lane.independent_reviewer}`,
    evidenceOptions(record),
  );
}

{
  const requiresPlan = clone(example);
  requiresPlan.lane_links[0].expected_benefit = "This field is not allowed.";
  expectError(requiresPlan, "expected_benefit is not allowed for requires");

  const benefitPlan = clone(example);
  benefitPlan.lane_links[2].required_provider_state = "implemented";
  expectError(
    benefitPlan,
    "required_provider_state is not allowed for benefits_from",
  );
}

{
  const plan = clone(example);
  const lane = plan.lanes[0];
  lane.status = "waiting_for_decision";
  lane.delivery_credit = "implemented";
  for (const capability of lane.provides) {
    capability.state = "implemented";
    capability.evidence_ref = `git:${"a".repeat(40)}`;
  }
  expectError(
    plan,
    "evidence_record_ref is required for waiting_for_decision",
  );
}

{
  const plan = clone(example);
  const lane = plan.lanes[0];
  lane.status = "implemented";
  lane.delivery_credit = "implemented";
  lane.evidence_record_ref =
    "document:docs/evidence/product-increments/candidate.json";
  for (const capability of lane.provides) {
    capability.state = "implemented";
    capability.evidence_ref = `git:${"a".repeat(40)}`;
  }
  const record = evidenceRecord(plan, lane, {
    review: eligibleReview(lane),
  });
  const withoutDestinationAuthority = evidenceOptions(record);
  delete withoutDestinationAuthority.verifyDestinationReceipt;
  expectError(
    plan,
    "destination_verification requires a trusted external verifier",
    withoutDestinationAuthority,
  );
}

{
  const plan = clone(example);
  plan.integration.independent_reviewer = plan.lanes[0].branch_owner;
  expectError(
    plan,
    "independent reviewer must differ from Delivery Lane",
  );
}

{
  const plan = clone(example);
  plan.integration.write_roots = [plan.lanes[0].write_roots[0]];
  expectError(plan, "overlaps Delivery Lane");
}

{
  const plan = clone(example);
  const lane = plan.lanes[0];
  lane.status = "implemented";
  lane.delivery_credit = "implemented";
  lane.evidence_record_ref =
    "document:docs/evidence/product-increments/candidate.json";
  for (const capability of lane.provides) {
    capability.state = "implemented";
    capability.evidence_ref = `git:${"a".repeat(40)}`;
  }
  const record = evidenceRecord(plan, lane, {
    review: eligibleReview(lane),
  });
  record.economics = {
    measurement_state: "measured",
    outcome: "implemented",
    planned_model: "efficient-tier",
    actual_model: "efficient-tier",
    reasoning_effort: "medium",
    routing_basis: "lowest_capable",
    elapsed_ms: lane.economics_budget.max_elapsed_ms + 1,
    input_tokens: 100,
    output_tokens: 50,
    cost_usd: 0.01,
    retries: 0,
    corrections: 0,
    rework_cycles: 0,
    context_source_count: 1,
    delegation_depth: 1,
  };
  expectError(plan, "elapsed_ms exceeds max_elapsed_ms", evidenceOptions(record));
}

{
  const plan = clone(example);
  const lane = plan.lanes[0];
  lane.status = "implemented";
  lane.delivery_credit = "implemented";
  lane.evidence_record_ref =
    "document:docs/evidence/product-increments/candidate.json";
  for (const capability of lane.provides) {
    capability.state = "implemented";
    capability.evidence_ref = `git:${"a".repeat(40)}`;
  }
  const record = evidenceRecord(plan, lane, {
    review: eligibleReview(lane),
  });
  record.economics = {
    measurement_state: "measured",
    outcome: "implemented",
    planned_model: "advanced-tier",
    actual_model: "advanced-tier",
    reasoning_effort: "high",
    routing_basis: "escalated_novel_synthesis",
    elapsed_ms: 1000,
    input_tokens: 100,
    output_tokens: 50,
    cost_usd: 0.01,
    retries: 0,
    corrections: 0,
    rework_cycles: 0,
    context_source_count: 1,
    delegation_depth: 1,
  };
  expectError(
    plan,
    "routing_justification is required for escalated routing",
    evidenceOptions(record),
  );
}

{
  const plan = clone(example);
  const lane = plan.lanes[0];
  lane.status = "accepted";
  lane.delivery_credit = "accepted";
  lane.evidence_record_ref =
    "document:docs/evidence/product-increments/candidate.json";
  for (const capability of lane.provides) {
    capability.state = "accepted";
    capability.evidence_ref = `git:${"a".repeat(40)}`;
  }
  const record = evidenceRecord(plan, lane, {
    review: eligibleReview(lane),
    laneAcceptance: acceptanceReceipt(lane.owner),
  });
  const withoutAuthority = evidenceOptions(record);
  delete withoutAuthority.verifyAuthorityReceipt;
  expectError(
    plan,
    "requires a trusted external verifier",
    withoutAuthority,
  );
  const withoutMetering = evidenceOptions(record);
  delete withoutMetering.verifyUsageReceipt;
  expectError(
    plan,
    "metering_receipt requires a trusted external verifier",
    withoutMetering,
  );
}

{
  const scopeConditional = evidenceSchema.$defs.scope.allOf[0];
  assert.deepEqual(scopeConditional.then.required, ["lane_id"]);
  assert.deepEqual(scopeConditional.else.not.required, ["lane_id"]);
  const measuredConditional =
    evidenceSchema.$defs.economics.allOf[0].then.required;
  assert.ok(measuredConditional.includes("metering_receipt_ref"));
  assert.ok(measuredConditional.includes("metering_receipt_sha256"));
  const routingConditional = evidenceSchema.$defs.economics.allOf[1];
  assert.deepEqual(routingConditional.then.required, [
    "routing_justification",
  ]);
  assert.deepEqual(routingConditional.else.not.required, [
    "routing_justification",
  ]);
  assert.ok(evidenceSchema.required.includes("destination_verification"));
  assert.deepEqual(
    evidenceSchema.$defs.destinationVerification.required,
    [
      "destination_branch",
      "candidate_sha",
      "verified_at",
      "evidence_ref",
      "evidence_sha256",
    ],
  );
}

console.log("check-product-increment-plan: 58 cases passed");
