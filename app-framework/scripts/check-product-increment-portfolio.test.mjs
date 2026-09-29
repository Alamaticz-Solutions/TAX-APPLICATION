#!/usr/bin/env node

import assert from "node:assert/strict";
import { createHash } from "node:crypto";
import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

import { validateProductIncrementPortfolio } from "./check-product-increment-portfolio.mjs";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const portfolioPath = join(
  root,
  "docs/specs/product-increment-portfolio.json",
);
const portfolioDir = dirname(portfolioPath);
const LOCAL_PRODUCT_INCREMENT_PLAN_REF =
  /^[a-z0-9][a-z0-9._-]*\.product-increment\.json$/;
const example = JSON.parse(readFileSync(portfolioPath, "utf8"));
const FIXTURE_TIMESTAMPS = [
  example.last_reconciled_at,
  ...example.product_increments.map((item) => item.status_updated_at),
];
const FIXTURE_TIMESTAMP_MS = FIXTURE_TIMESTAMPS.map((value) =>
  Date.parse(value),
);
assert.ok(
  FIXTURE_TIMESTAMP_MS.every(Number.isFinite),
  `canonical fixture contains an invalid timestamp: ${FIXTURE_TIMESTAMPS.join(", ")}`,
);
const NOW = Math.max(...FIXTURE_TIMESTAMP_MS);
const EVIDENCE_DOCUMENTS = new Map(
  [
    ["proof", "retained portfolio execution proof\n"],
    ["metering", "trusted portfolio metering receipt\n"],
    ["destination", "trusted portfolio destination receipt\n"],
    ["review", "retained portfolio review receipt\n"],
    ["lane-acceptance", "retained lane acceptance receipt\n"],
    ["product-acceptance", "retained Product acceptance receipt\n"],
  ].map(([kind, text]) => [
    `docs/evidence/product-increments/unit-${kind}.txt`,
    Buffer.from(text),
  ]),
);

function evidenceFields(kind) {
  const path = `docs/evidence/product-increments/unit-${kind}.txt`;
  const bytes = EVIDENCE_DOCUMENTS.get(path);
  return {
    evidence_ref: `document:${path}`,
    evidence_sha256: createHash("sha256").update(bytes).digest("hex"),
  };
}

function clone(value) {
  return structuredClone(value);
}

function increment(portfolio, id) {
  return portfolio.product_increments.find((item) => item.id === id);
}

function loadPlan(planRef) {
  return JSON.parse(readFileSync(join(portfolioDir, planRef), "utf8"));
}

function planLaneCounts(portfolio) {
  const planRefs = [];
  for (const item of portfolio.product_increments) {
    const planRef = item.plan_ref;
    if (planRef == null || planRef === "") continue;
    if (
      typeof planRef !== "string" ||
      !LOCAL_PRODUCT_INCREMENT_PLAN_REF.test(planRef)
    ) {
      throw new Error(`unsafe local Product Increment plan_ref: ${String(planRef)}`);
    }
    planRefs.push(planRef);
  }
  return [...new Set(planRefs)]
    .flatMap((planRef) => loadPlan(planRef).lanes ?? [])
    .reduce(
      (counts, lane) => {
        if (lane.status === "in_progress") counts.active += 1;
        if (lane.status === "under_review") counts.review += 1;
        return counts;
      },
      { active: 0, review: 0 },
    );
}

function loadEvidenceRecord(evidenceRef) {
  return JSON.parse(readFileSync(join(root, evidenceRef), "utf8"));
}

const DEFAULT_VALIDATION_OPTIONS = {
  acceptedEvidenceExists: () => true,
  branchExists: () => true,
  evidenceExists: () => true,
  gitChangedPaths: () => [
    "scripts/agent-routing/appfw-model-route.mjs",
  ],
  gitEvidenceExists: () => true,
  gitIsAncestor: () => true,
  loadAcceptedEvidenceBytes: (path) => EVIDENCE_DOCUMENTS.get(path),
  loadAcceptedEvidenceRecord: loadEvidenceRecord,
  loadEvidenceBytes: (path) => EVIDENCE_DOCUMENTS.get(path),
  loadEvidenceRecord,
  loadPlan,
  now: NOW,
  pathIsReviewable: () => true,
  verifyAuthorityReceipt: () => true,
  verifyDestinationReceipt: () => true,
  verifyUsageReceipt: () => true,
};

function validate(portfolio, options = {}) {
  return validateProductIncrementPortfolio(portfolio, {
    ...DEFAULT_VALIDATION_OPTIONS,
    ...options,
  });
}

function expectError(portfolio, fragment, options = {}) {
  const result = validate(portfolio, options);
  assert.equal(
    result.ok,
    false,
    `expected invalid portfolio containing ${fragment}`,
  );
  assert.ok(
    result.errors.some((error) => error.includes(fragment)),
    `missing error containing "${fragment}":\n${result.errors.join("\n")}`,
  );
}

{
  const result = validate(example);
  assert.equal(result.ok, true, result.errors.join("\n"));
  const expectedLaneCounts = planLaneCounts(example);
  assert.equal(
    result.summary.product_increment_count,
    example.product_increments.length,
  );
  assert.equal(
    result.summary.current_active_lanes,
    expectedLaneCounts.active,
  );
  assert.equal(
    result.summary.current_review_lanes,
    expectedLaneCounts.review,
  );
  assert.equal(
    result.summary.implemented_product_increment_count,
    example.product_increments.filter((item) =>
      item.provides.some((capability) =>
        ["implemented", "accepted"].includes(capability.state),
      ),
    ).length,
  );
  assert.equal(
    result.summary.accepted_product_increment_count,
    example.product_increments.filter(
      (item) => item.lifecycle_state === "accepted",
    ).length,
  );
  assert.ok(
    result.summary.increments.some((item) => item.id === "AFS-PI-P1"),
    "canonical portfolio must expose the current router Product Increment",
  );
  assert.ok(
    example.product_increments.some(
      (item) => (item.benefits_from ?? []).length > 0,
    ),
    "canonical portfolio must expose cross-increment benefits",
  );
  assert.equal(
    result.summary.economics_control.model_routing_policy,
    "lowest_capable_then_escalate",
  );
  assert.deepEqual(result.summary.economics_control.primary_metrics, [
    "accepted_lead_time",
    "cost_per_accepted_deliverable",
    "tokens_per_accepted_deliverable",
    "review_queue_age",
    "rework_rate",
  ]);
}

{
  const portfolio = clone(example);
  increment(
    portfolio,
    "NXP-PI-ADD-PROVIDER-01",
  ).plan_ref = "../outside.product-increment.json";
  assert.throws(
    () => planLaneCounts(portfolio),
    /unsafe local Product Increment plan_ref/,
  );
}

{
  const portfolio = clone(example);
  const overlappingLoader = (planRef) => {
    const plan = loadPlan(planRef);
    if (planRef === "nexus-add-provider.product-increment.json") {
      plan.lanes[0].status = "in_progress";
      plan.lanes[0].branch_owner = "Shared Branch Owner";
      plan.lanes[0].write_roots = ["shared/**"];
    }
    if (planRef === "afs-transparent-routing.product-increment.json") {
      plan.lanes[0].status = "in_progress";
      plan.lanes[0].branch_owner = "Shared Branch Owner";
      plan.lanes[0].write_roots = ["shared/runtime/**"];
    }
    return plan;
  };
  increment(portfolio, "AFS-PI-P1").lifecycle_state = "active";
  portfolio.operating_capacity.current_active_lanes = 3;
  portfolio.operating_capacity.current_review_lanes = 0;
  expectError(portfolio, "active branch owner Shared Branch Owner", {
    loadPlan: overlappingLoader,
    now: NOW,
  });
  expectError(portfolio, "overlap write roots shared/** and shared/runtime/**", {
    loadPlan: overlappingLoader,
    now: NOW,
  });
}

{
  const portfolio = clone(example);
  const inconsistentLoader = (planRef) => {
    const plan = loadPlan(planRef);
    if (planRef === "afs-transparent-routing.product-increment.json") {
      plan.lanes[0].status = "ready_to_start";
      plan.lanes[0].delivery_credit = "none";
      plan.lanes[0].provides[0].state = "implemented";
      plan.lanes[0].provides[0].evidence_ref = `git:${"a".repeat(40)}`;
    }
    return plan;
  };
  expectError(portfolio, "state must exactly match plan_ref", {
    loadPlan: inconsistentLoader,
    now: NOW,
  });
  increment(portfolio, "AFS-PI-P1").lifecycle_state = "under_review";
  portfolio.operating_capacity.current_review_lanes = 1;
  expectError(portfolio, "must contain an under_review lane", {
    loadPlan: inconsistentLoader,
    now: NOW,
  });
}

{
  const portfolio = clone(example);
  portfolio.last_reconciled_at = "2000-01-01T00:00:00Z";
  expectError(portfolio, "reconciliation is older than 24 hours", {
    now: NOW,
    strictFreshness: true,
  });
}

{
  const portfolio = clone(example);
  portfolio.catalog_state = "current";
  portfolio.known_unregistered_work = [];
  portfolio.last_reconciled_at = "2000-01-01T00:00:00Z";
  for (const increment of portfolio.product_increments) {
    increment.status_updated_at = "1999-12-31T00:00:00Z";
  }
  expectError(portfolio, "reconciliation is older than 24 hours", {
    now: NOW,
  });
}

{
  const portfolio = clone(example);
  portfolio.unexpected = true;
  portfolio.product_increments[0].unexpected = true;
  expectError(portfolio, "portfolio.unexpected is not allowed", { now: NOW });
  expectError(
    portfolio,
    "portfolio.product_increments[0].unexpected is not allowed",
    { now: NOW },
  );
}

{
  const portfolio = clone(example);
  increment(portfolio, "NXP-PI-ADD-PROVIDER-01").current_status = "";
  expectError(portfolio, "current_status must be a non-empty string");
}

{
  const portfolio = clone(example);
  increment(portfolio, "AFS-PI-P2").requires[0].from_product_increment_id =
    "AFS-PI-MISSING";
  expectError(portfolio, "is not registered");
}

{
  const portfolio = clone(example);
  increment(portfolio, "AFS-PI-P2").lifecycle_state = "active";
  expectError(portfolio, "blocked by an unfinished required capability");
}

{
  const portfolio = clone(example);
  increment(
    portfolio,
    "AFS-PI-DELIVERY-CONTROL-01",
  ).routing.minimum_maturity =
    "transparent_accepted";
  expectError(portfolio, "executing below its minimum routing maturity");
}

{
  const portfolio = clone(example);
  portfolio.operating_capacity.current_review_lanes = 1;
  expectError(portfolio, "does not match plan-derived 0", { loadPlan });
}

{
  const portfolio = clone(example);
  portfolio.economics_control.default_max_delegation_depth = 3;
  portfolio.economics_control.primary_metrics.pop();
  expectError(portfolio, "default_max_delegation_depth must be 1");
  expectError(portfolio, "must contain the five required accepted-throughput metrics");
}

{
  const portfolio = clone(example);
  increment(
    portfolio,
    "PDS-PI-EXPERIENCE-FOUNDATION-01",
  ).provides[0].capability_id =
    "harness.transparent-routing";
  expectError(portfolio, "has more than one Product Increment provider");
}

{
  const portfolio = clone(example);
  const badLoader = (planRef) => {
    const plan = loadPlan(planRef);
    if (planRef === "afs-transparent-routing.product-increment.json") {
      plan.product_increment.id = "AFS-PI-WRONG";
    }
    return plan;
  };
  expectError(portfolio, "Product Increment id does not match", {
    loadPlan: badLoader,
  });
}

{
  const portfolio = clone(example);
  increment(portfolio, "AFS-PI-P1").stage = "release_candidate";
  expectError(portfolio, "plan_ref stage does not match portfolio", {
    loadPlan,
    now: NOW,
  });
}

{
  const portfolio = clone(example);
  increment(portfolio, "AFS-PI-P1").outcome_id = "AFS-OUTCOME-WRONG";
  expectError(portfolio, "plan_ref Outcome id does not match", {
    loadPlan,
    now: NOW,
  });
}

{
  const portfolio = clone(example);
  const held = increment(portfolio, "AFS-PI-P2");
  held.provides[0].state = "accepted";
  held.provides[0].evidence_ref = "evidence:invalid-acceptance";
  expectError(portfolio, "is incompatible with lifecycle_state held", {
    now: NOW,
  });
}

{
  const portfolio = clone(example);
  const mismatchedLoader = (planRef) => {
    const plan = loadPlan(planRef);
    if (planRef === "afs-product-increment-delivery-control.product-increment.json") {
      plan.lanes[0].provides[0].capability_id =
        "different.delivery.capability";
    }
    return plan;
  };
  expectError(portfolio, "is not declared by plan_ref", {
    loadPlan: mismatchedLoader,
    now: NOW,
  });
}

{
  const portfolio = clone(example);
  increment(
    portfolio,
    "NXP-PI-ADD-PROVIDER-01",
  ).provides = increment(
    portfolio,
    "NXP-PI-ADD-PROVIDER-01",
  ).provides.filter(
    (capability) => capability.capability_id !== "servicenow.case-projection",
  );
  expectError(portfolio, "omits plan_ref capability servicenow.case-projection", {
    loadPlan,
    now: NOW,
  });
}

{
  const portfolio = clone(example);
  increment(
    portfolio,
    "NXP-PI-ADD-PROVIDER-01",
  ).plan_ref = "../outside.product-increment.json";
  expectError(portfolio, "must be a local Product Increment plan filename", {
    now: NOW,
  });
}

{
  const portfolio = clone(example);
  expectError(portfolio, "is not durably reachable", {
    gitEvidenceExists: () => false,
    loadPlan,
    now: NOW,
  });
}

{
  const portfolio = clone(example);
  portfolio.last_reconciled_at = "July 23, 2026 09:00:00";
  expectError(portfolio, "must be an RFC3339 timestamp", { now: NOW });
}

{
  const portfolio = clone(example);
  portfolio.last_reconciled_at = "2026-02-31T12:00:00Z";
  expectError(portfolio, "must be an RFC3339 timestamp", { now: NOW });
}

{
  const portfolio = clone(example);
  portfolio.last_reconciled_at = "2028-02-29T12:00:00Z";
  for (const productIncrement of portfolio.product_increments) {
    productIncrement.status_updated_at = "2028-02-29T12:00:00Z";
  }
  const result = validate(portfolio, {
    now: Date.parse("2028-03-01T00:00:00Z"),
  });
  assert.equal(result.ok, true, result.errors.join("\n"));
}

{
  const portfolio = clone(example);
  portfolio.operating_capacity.current_active_lanes = 3;
  portfolio.operating_capacity.below_target_reason = { invalid: true };
  expectError(portfolio, "below_target_reason must be a string when present", {
    now: NOW,
  });
}

{
  const portfolio = clone(example);
  increment(portfolio, "AFS-PI-DELIVERY-CONTROL-01").evidence.push({
    kind: "review",
    ref: "target/appfw/framework-pr-review.md",
    summary: "Transient review output is not durable portfolio evidence.",
  });
  expectError(portfolio, "kind is not recognized", {
    now: NOW,
  });
}

{
  const portfolio = clone(example);
  increment(portfolio, "AFS-PI-DELIVERY-CONTROL-01").evidence.push({
    kind: "branch",
    ref: "feature/product-increment-lane-operating-model",
    summary: "Open source branch used to exercise branch-evidence resolution.",
  });
  expectError(portfolio, "does not resolve to a local or remote branch", {
    branchExists: () => false,
    now: NOW,
  });
}

{
  const portfolio = clone(example);
  const badLoader = (planRef) => {
    const plan = loadPlan(planRef);
    if (planRef === "afs-transparent-routing.product-increment.json") {
      plan.integration.evidence_record_ref =
        "document:docs/specs/nexus-add-provider-seams.json";
      plan.lanes[0].evidence_record_ref =
        "document:docs/specs/nexus-add-provider-seams.json";
    }
    return plan;
  };
  expectError(portfolio, "schema must be appfw_product_increment_evidence@1", {
    loadEvidenceRecord: (evidenceRef) =>
      JSON.parse(readFileSync(join(root, evidenceRef), "utf8")),
    loadPlan: badLoader,
    now: NOW,
  });
}

{
  const portfolio = clone(example);
  const router = increment(portfolio, "AFS-PI-P1");
  const candidateSha = "a".repeat(40);
  router.lifecycle_state = "accepted";
  router.provides[0].state = "accepted";
  router.provides[0].evidence_ref = `git:${candidateSha}`;
  router.evidence.find((item) => item.kind === "git").ref = candidateSha;
  const acceptedPlanLoader = (planRef) => {
    const plan = loadPlan(planRef);
    if (planRef === "afs-transparent-routing.product-increment.json") {
      const lane = plan.lanes[0];
      lane.status = "accepted";
      lane.delivery_credit = "accepted";
      lane.provides[0].state = "accepted";
      lane.provides[0].evidence_ref = `git:${candidateSha}`;
      plan.integration.evidence_record_ref =
        "document:docs/evidence/product-increments/afs-p1-integration.json";
    }
    return plan;
  };
  const acceptedEvidenceLoader = (evidenceRef) => {
    const plan = acceptedPlanLoader(
      "afs-transparent-routing.product-increment.json",
    );
    const lane = plan.lanes[0];
    const integrationRecord = evidenceRef.endsWith("afs-p1-integration.json");
    const reviewer = integrationRecord
      ? plan.integration.independent_reviewer
      : lane.independent_reviewer;
    return {
      schema: "appfw_product_increment_evidence@1",
      product_increment_id: plan.product_increment.id,
      outcome_id: plan.product_increment.outcome_id,
      scope: { kind: "direct_main_candidate" },
      candidate_sha: candidateSha,
      base_sha: lane.base_sha,
      proof: {
        stage: "prototype",
        profile: "full",
        status: "passed",
        completed_at: "2026-07-23T18:00:00Z",
        checks: [
          ...lane.focused_checks,
          ...plan.integration.integration_checks,
        ].map((command) => ({
            command,
            result: "passed",
            output_ref: evidenceFields("proof").evidence_ref,
            output_sha256: evidenceFields("proof").evidence_sha256,
        })),
      },
      economics: {
        measurement_state: "measured",
        outcome: integrationRecord ? "implemented" : "accepted",
        planned_model: "efficient-tier",
        actual_model: "efficient-tier",
        reasoning_effort: "medium",
        routing_basis: "lowest_capable",
        metering_receipt_ref: evidenceFields("metering").evidence_ref,
        metering_receipt_sha256:
          evidenceFields("metering").evidence_sha256,
        elapsed_ms: 1000,
        input_tokens: 100,
        output_tokens: 50,
        cost_usd: 0.01,
        retries: 0,
        corrections: 0,
        rework_cycles: 0,
        context_source_count: 4,
        delegation_depth: 1,
      },
      review: {
        reviewer,
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
        ...evidenceFields("review"),
      },
      lane_acceptance: integrationRecord
        ? null
        : {
            owner: lane.owner,
            decision: "accepted",
            accepted_sha: candidateSha,
            decided_at: "2026-07-23T18:10:00Z",
            ...evidenceFields("lane-acceptance"),
          },
      product_acceptance: null,
    };
  };
  expectError(
    portfolio,
    `accepted requires a Product Increment acceptance receipt from ${router.owner}`,
    {
      loadAcceptedEvidenceRecord: acceptedEvidenceLoader,
      loadEvidenceRecord: acceptedEvidenceLoader,
      loadPlan: acceptedPlanLoader,
      now: NOW,
    },
  );
  expectError(portfolio, "economics must be measured for durable delivery credit", {
    loadAcceptedEvidenceRecord: (evidenceRef) => {
      const record = acceptedEvidenceLoader(evidenceRef);
      if (!evidenceRef.endsWith("afs-p1-integration.json")) {
        record.economics = {
          measurement_state: "unavailable",
          outcome: "accepted",
          retries: 0,
          corrections: 0,
          rework_cycles: 0,
          reason: "No trustworthy cost or token measurement was retained.",
        };
      }
      return record;
    },
    loadEvidenceRecord: (evidenceRef) => {
      const record = acceptedEvidenceLoader(evidenceRef);
      if (!evidenceRef.endsWith("afs-p1-integration.json")) {
        record.economics = {
          measurement_state: "unavailable",
          outcome: "accepted",
          retries: 0,
          corrections: 0,
          rework_cycles: 0,
          reason: "No trustworthy cost or token measurement was retained.",
        };
      }
      return record;
    },
    loadPlan: acceptedPlanLoader,
    now: NOW,
  });
}

{
  const portfolio = clone(example);
  increment(portfolio, "AFS-PI-DELIVERY-CONTROL-01").evidence.push({
    kind: "document",
    ref: "target/appfw/transient-review.md",
    summary: "Transient local evidence must not become portfolio authority.",
  });
  expectError(portfolio, "must identify a durable repository-relative document", {
    loadPlan,
    evidenceExists: () => true,
    now: NOW,
  });
}

{
  const portfolio = clone(example);
  increment(portfolio, "AFS-PI-DELIVERY-CONTROL-01").evidence.push({
    kind: "document",
    ref: "docs/specs/does-not-exist.md",
    summary: "Missing retained evidence must fail validation.",
  });
  expectError(portfolio, "does not exist in this checkout", {
    loadPlan,
    evidenceExists: () => false,
    now: NOW,
  });
}

console.log("check-product-increment-portfolio: 32 cases passed");
