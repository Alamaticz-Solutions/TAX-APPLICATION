#!/usr/bin/env node

import { spawnSync } from "node:child_process";
import { existsSync, readFileSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

import { validateProductIncrementPlan } from "./check-product-increment-plan.mjs";
import {
  isSafeRepositoryPath,
  repositoryPathIsReviewable,
} from "./product-increment-path-safety.mjs";

const PORTFOLIO_SCHEMA = "appfw_product_increment_portfolio@1";
const LIFECYCLE_STATES = new Set([
  "envisioned",
  "planned",
  "ready",
  "active",
  "under_review",
  "implemented",
  "accepted",
  "blocked",
  "held",
  "needs_reconciliation",
  "stopped",
  "superseded",
]);
const STAGES = new Set([
  "prototype",
  "integration_candidate",
  "release_candidate",
]);
const INTEGRATION_STRATEGIES = new Set([
  "integration_branch",
  "direct_main",
  "not_yet_planned",
]);
const CAPABILITY_STATE_RANK = {
  planned: 0,
  implemented: 1,
  accepted: 2,
};
const CAPABILITY_STATES_BY_LIFECYCLE = {
  envisioned: ["planned"],
  planned: ["planned"],
  ready: ["planned"],
  active: ["planned", "implemented"],
  under_review: ["planned", "implemented"],
  implemented: ["implemented"],
  accepted: ["accepted"],
  blocked: ["planned"],
  held: ["planned"],
  needs_reconciliation: ["planned"],
  stopped: ["planned"],
  superseded: ["planned"],
};
const DELIVERY_CREDIT_RANK = {
  none: 0,
  implemented: 1,
  accepted: 2,
};
const ROUTING_MATURITY_RANK = {
  manual_explicit: 0,
  transparent_candidate: 1,
  transparent_accepted: 2,
  measured: 3,
  adaptive: 4,
};
const EXECUTING_STATES = new Set([
  "ready",
  "active",
  "under_review",
  "implemented",
  "accepted",
]);
const PLAN_REQUIRED_STATES = new Set([
  "ready",
  "active",
  "under_review",
  "implemented",
  "accepted",
]);
const PORTFOLIO_RECONCILIATION_MAX_AGE_MS = 24 * 60 * 60 * 1000;
const INCREMENT_STATUS_MAX_AGE_MS = 7 * 24 * 60 * 60 * 1000;
const FUTURE_CLOCK_SKEW_MS = 5 * 60 * 1000;
const PORTFOLIO_ID = /^[a-z0-9][a-z0-9._-]{1,63}$/;
const RECORD_ID = /^[A-Z0-9][A-Z0-9._-]{1,63}$/;
const CAPABILITY_ID = /^[a-z0-9][a-z0-9._-]{1,95}$/;
const FULL_SHA = /^[0-9a-f]{40}$/;
const BRANCH_REF =
  /^(feature|fix|docs|maintenance|integrate|release)\/[a-z0-9][a-z0-9._/-]*$/;
const RFC3339_DATE_TIME =
  /^(\d{4})-(\d{2})-(\d{2})T(?:[01]\d|2[0-3]):[0-5]\d:[0-5]\d(?:\.\d+)?(?:Z|[+-](?:[01]\d|2[0-3]):[0-5]\d)$/;
const EVIDENCE_KINDS = new Set([
  "git",
  "branch",
  "document",
]);
const PLAN_REF = /^[a-z0-9][a-z0-9._-]*\.product-increment\.json$/;
const REQUIRED_ECONOMICS_METRICS = new Set([
  "accepted_lead_time",
  "cost_per_accepted_deliverable",
  "tokens_per_accepted_deliverable",
  "review_queue_age",
  "rework_rate",
]);

function usage() {
  return `Usage:
  node scripts/check-product-increment-portfolio.mjs \
    --portfolio <path> [--id <product-increment-id>] [--json]

Validates the human-readable Product Increment portfolio, referenced execution
plans, two-to-four lane portfolio capacity, and cross-increment requires and
benefits_from relationships.`;
}

function parseArgs(argv) {
  const options = { json: false, portfolio: null, id: null };
  for (let index = 0; index < argv.length; index += 1) {
    const arg = argv[index];
    if (arg === "--json") {
      options.json = true;
    } else if (arg === "--help" || arg === "-h") {
      options.help = true;
    } else if (arg === "--portfolio" || arg === "--id") {
      const value = argv[index + 1];
      if (!value || value.startsWith("--")) {
        throw new Error(`${arg} requires a value`);
      }
      options[arg.slice(2)] = value;
      index += 1;
    } else {
      throw new Error(`unknown argument: ${arg}`);
    }
  }
  if (!options.help && !options.portfolio) {
    throw new Error("--portfolio is required");
  }
  return options;
}

function isObject(value) {
  return value !== null && typeof value === "object" && !Array.isArray(value);
}

function nonEmptyString(value) {
  return typeof value === "string" && value.trim() !== "";
}

function requireString(record, key, label, errors) {
  if (!nonEmptyString(record?.[key])) {
    errors.push(`${label}.${key} must be a non-empty string`);
    return false;
  }
  return true;
}

function requirePattern(record, key, pattern, label, errors) {
  if (!requireString(record, key, label, errors)) return false;
  if (!pattern.test(record[key])) {
    errors.push(`${label}.${key} does not match the required format`);
    return false;
  }
  return true;
}

function requireOptionalString(record, key, label, errors) {
  if (record?.[key] !== undefined && typeof record[key] !== "string") {
    errors.push(`${label}.${key} must be a string when present`);
    return false;
  }
  return true;
}

function rejectUnknownKeys(record, allowedKeys, label, errors) {
  if (!isObject(record)) return;
  const allowed = new Set(allowedKeys);
  for (const key of Object.keys(record)) {
    if (!allowed.has(key)) {
      errors.push(`${label}.${key} is not allowed`);
    }
  }
}

function validTimestamp(value) {
  if (!nonEmptyString(value)) return false;
  const match = RFC3339_DATE_TIME.exec(value);
  if (!match || !Number.isFinite(Date.parse(value))) return false;
  const year = Number(match[1]);
  const month = Number(match[2]);
  const day = Number(match[3]);
  if (month < 1 || month > 12 || day < 1) return false;
  const leapYear = year % 4 === 0 && (year % 100 !== 0 || year % 400 === 0);
  const daysInMonth = [
    31,
    leapYear ? 29 : 28,
    31,
    30,
    31,
    30,
    31,
    31,
    30,
    31,
    30,
    31,
  ];
  return day <= daysInMonth[month - 1];
}

function validateCapabilityEvidenceRef(
  value,
  label,
  errors,
  {
    evidenceExists = null,
    gitEvidenceExists = null,
    pathIsReviewable = null,
  } = {},
) {
  if (!nonEmptyString(value)) return;
  if (value.startsWith("git:")) {
    const sha = value.slice(4);
    if (!FULL_SHA.test(sha)) {
      errors.push(`${label} git evidence must use a full lowercase Git SHA`);
    } else if (
      typeof gitEvidenceExists === "function" &&
      !gitEvidenceExists(sha)
    ) {
      errors.push(`${label} Git evidence is not durably reachable: ${sha}`);
    }
    return;
  }
  if (value.startsWith("document:")) {
    const path = value.slice("document:".length);
    if (!isSafeRepositoryPath(path) || path.startsWith("target/")) {
      errors.push(
        `${label} document evidence must be a durable repository-relative path`,
      );
    } else if (
      typeof evidenceExists === "function" &&
      !evidenceExists(path)
    ) {
      errors.push(`${label} document evidence does not exist: ${path}`);
    } else if (
      typeof pathIsReviewable === "function" &&
      !pathIsReviewable(path)
    ) {
      errors.push(
        `${label} document evidence is ignored, reserved, or traverses a symlink: ${path}`,
      );
    }
    return;
  }
  errors.push(
    `${label} must use git:<full-sha> or document:<repository-relative-path>`,
  );
}

function validateDocumentRef(
  value,
  label,
  errors,
  { evidenceExists = null, pathIsReviewable = null } = {},
) {
  if (!nonEmptyString(value) || !value.startsWith("document:")) {
    errors.push(
      `${label} must use document:<durable-repository-relative-path>`,
    );
    return;
  }
  const path = value.slice("document:".length);
  if (!isSafeRepositoryPath(path) || path.startsWith("target/")) {
    errors.push(`${label} must identify a durable repository document`);
  } else if (
    typeof evidenceExists === "function" &&
    !evidenceExists(path)
  ) {
    errors.push(`${label} does not exist: ${path}`);
  } else if (
    typeof pathIsReviewable === "function" &&
    !pathIsReviewable(path)
  ) {
    errors.push(
      `${label} is ignored, reserved, or traverses a symlink: ${path}`,
    );
  }
}

function pathBase(pattern) {
  return String(pattern)
    .replace(/\/\*\*$/, "")
    .replace(/\/\*$/, "")
    .replace(/\/$/, "");
}

function pathsOverlap(left, right) {
  const leftBase = pathBase(left);
  const rightBase = pathBase(right);
  return (
    leftBase === rightBase ||
    leftBase.startsWith(`${rightBase}/`) ||
    rightBase.startsWith(`${leftBase}/`)
  );
}

function stateSatisfies(actual, required) {
  return (
    Object.hasOwn(CAPABILITY_STATE_RANK, actual) &&
    Object.hasOwn(CAPABILITY_STATE_RANK, required) &&
    CAPABILITY_STATE_RANK[actual] >= CAPABILITY_STATE_RANK[required]
  );
}

function findCycle(graph) {
  const visiting = new Set();
  const visited = new Set();
  const path = [];

  function visit(node) {
    if (visiting.has(node)) {
      const start = path.indexOf(node);
      return [...path.slice(start), node];
    }
    if (visited.has(node)) return null;

    visiting.add(node);
    path.push(node);
    for (const dependency of graph.get(node) ?? []) {
      const cycle = visit(dependency);
      if (cycle) return cycle;
    }
    path.pop();
    visiting.delete(node);
    visited.add(node);
    return null;
  }

  for (const node of graph.keys()) {
    const cycle = visit(node);
    if (cycle) return cycle;
  }
  return null;
}

export function validateProductIncrementPortfolio(
  portfolio,
  {
    acceptedEvidenceExists = null,
    branchExists = null,
    loadPlan = null,
    loadEvidenceRecord = null,
    evidenceExists = null,
    gitChangedPaths = null,
    gitEvidenceExists = null,
    gitIsAncestor = null,
    loadAcceptedEvidenceBytes = null,
    loadAcceptedEvidenceRecord = null,
    loadEvidenceBytes = null,
    pathIsReviewable = null,
    verifyAuthorityReceipt = null,
    verifyDestinationReceipt = null,
    verifyUsageReceipt = null,
    now = Date.now(),
    strictFreshness = false,
  } = {},
) {
  const errors = [];
  const warnings = [];

  if (!isObject(portfolio)) {
    return {
      ok: false,
      errors: ["portfolio must be an object"],
      warnings,
      summary: null,
    };
  }
  if (portfolio.schema !== PORTFOLIO_SCHEMA) {
    errors.push(`portfolio.schema must be ${PORTFOLIO_SCHEMA}`);
  }
  requireOptionalString(portfolio, "$schema", "portfolio", errors);
  rejectUnknownKeys(
    portfolio,
    [
      "$schema",
      "schema",
      "portfolio_id",
      "title",
      "last_reconciled_at",
      "catalog_state",
      "operating_capacity",
      "economics_control",
      "product_increments",
      "known_unregistered_work",
    ],
    "portfolio",
    errors,
  );
  requirePattern(portfolio, "portfolio_id", PORTFOLIO_ID, "portfolio", errors);
  for (const key of ["title", "last_reconciled_at"]) {
    requireString(portfolio, key, "portfolio", errors);
  }
  if (!validTimestamp(portfolio.last_reconciled_at)) {
    errors.push("portfolio.last_reconciled_at must be an RFC3339 timestamp");
  }
  if (!["migration_required", "current"].includes(portfolio.catalog_state)) {
    errors.push("portfolio.catalog_state is not recognized");
  }
  const nowMs = now instanceof Date ? now.getTime() : Number(now);
  if (!Number.isFinite(nowMs)) {
    errors.push("portfolio validation time must be finite");
  }
  const reconciliationTime = validTimestamp(portfolio.last_reconciled_at)
    ? Date.parse(portfolio.last_reconciled_at)
    : null;
  const reportFreshness = (message) => {
    if (strictFreshness || portfolio.catalog_state === "current") {
      errors.push(message);
    }
    else warnings.push(message);
  };
  if (Number.isFinite(nowMs) && Number.isFinite(reconciliationTime)) {
    if (reconciliationTime > nowMs + FUTURE_CLOCK_SKEW_MS) {
      errors.push("portfolio.last_reconciled_at is in the future");
    } else if (
      nowMs - reconciliationTime >
      PORTFOLIO_RECONCILIATION_MAX_AGE_MS
    ) {
      reportFreshness(
        "portfolio reconciliation is older than 24 hours; refresh current status before admission",
      );
    }
  }
  if (!Array.isArray(portfolio.known_unregistered_work)) {
    errors.push("portfolio.known_unregistered_work must be an array");
  } else if (
    portfolio.known_unregistered_work.some((item) => !nonEmptyString(item))
  ) {
    errors.push(
      "portfolio.known_unregistered_work must contain only non-empty strings",
    );
  } else if (
    new Set(portfolio.known_unregistered_work).size !==
    portfolio.known_unregistered_work.length
  ) {
    errors.push("portfolio.known_unregistered_work must not contain duplicates");
  }
  if (
    portfolio.catalog_state === "current" &&
    portfolio.known_unregistered_work?.length > 0
  ) {
    errors.push(
      "catalog_state current cannot retain known_unregistered_work entries",
    );
  }

  const capacity = portfolio.operating_capacity;
  if (!isObject(capacity)) {
    errors.push("portfolio.operating_capacity must be an object");
  } else {
    rejectUnknownKeys(
      capacity,
      [
        "target_active_lanes",
        "maximum_active_lanes",
        "current_active_lanes",
        "current_review_lanes",
        "below_target_reason",
      ],
      "portfolio.operating_capacity",
      errors,
    );
    for (const key of ["target_active_lanes", "maximum_active_lanes"]) {
      if (!Number.isInteger(capacity[key]) || capacity[key] < 2 || capacity[key] > 4) {
        errors.push(
          `portfolio.operating_capacity.${key} must be an integer from 2 through 4`,
        );
      }
    }
    for (const key of ["current_active_lanes", "current_review_lanes"]) {
      if (!Number.isInteger(capacity[key]) || capacity[key] < 0) {
        errors.push(
          `portfolio.operating_capacity.${key} must be a non-negative integer`,
        );
      }
    }
    requireOptionalString(
      capacity,
      "below_target_reason",
      "portfolio.operating_capacity",
      errors,
    );
    if (
      Number.isInteger(capacity.target_active_lanes) &&
      Number.isInteger(capacity.maximum_active_lanes) &&
      capacity.target_active_lanes > capacity.maximum_active_lanes
    ) {
      errors.push(
        "portfolio target_active_lanes must not exceed maximum_active_lanes",
      );
    }
    if (
      Number.isInteger(capacity.current_active_lanes) &&
      Number.isInteger(capacity.maximum_active_lanes) &&
      capacity.current_active_lanes > capacity.maximum_active_lanes
    ) {
      errors.push(
        "portfolio current_active_lanes exceeds maximum_active_lanes",
      );
    }
    if (
      Number.isInteger(capacity.current_active_lanes) &&
      Number.isInteger(capacity.target_active_lanes) &&
      capacity.current_active_lanes < capacity.target_active_lanes &&
      !nonEmptyString(capacity.below_target_reason)
    ) {
      errors.push(
        "portfolio below_target_reason is required below target lane capacity",
      );
    }
  }

  const economicsControl = portfolio.economics_control;
  if (!isObject(economicsControl)) {
    errors.push("portfolio.economics_control must be an object");
  } else {
    rejectUnknownKeys(
      economicsControl,
      [
        "context_capsule_max_controlling_records",
        "default_max_delegation_depth",
        "correction_cycles_before_replan",
        "idle_agent_policy",
        "model_routing_policy",
        "evidence_measurement_required",
        "primary_metrics",
      ],
      "portfolio.economics_control",
      errors,
    );
    const exactValues = {
      context_capsule_max_controlling_records: 4,
      default_max_delegation_depth: 1,
      correction_cycles_before_replan: 2,
      idle_agent_policy: "unload",
      model_routing_policy: "lowest_capable_then_escalate",
      evidence_measurement_required: true,
    };
    for (const [key, expected] of Object.entries(exactValues)) {
      if (economicsControl[key] !== expected) {
        errors.push(
          `portfolio.economics_control.${key} must be ${JSON.stringify(expected)}`,
        );
      }
    }
    if (
      !Array.isArray(economicsControl.primary_metrics) ||
      economicsControl.primary_metrics.length !==
        REQUIRED_ECONOMICS_METRICS.size ||
      new Set(economicsControl.primary_metrics).size !==
        REQUIRED_ECONOMICS_METRICS.size ||
      [...REQUIRED_ECONOMICS_METRICS].some(
        (metric) => !economicsControl.primary_metrics.includes(metric),
      )
    ) {
      errors.push(
        "portfolio.economics_control.primary_metrics must contain the five required accepted-throughput metrics exactly once",
      );
    }
  }

  if (
    !Array.isArray(portfolio.product_increments) ||
    portfolio.product_increments.length === 0
  ) {
    errors.push("portfolio.product_increments must be a non-empty array");
  }

  const incrementById = new Map();
  const capabilityById = new Map();
  const planSummaries = new Map();
  const activePlanLanes = [];
  let derivedActiveLanes = 0;
  let derivedReviewLanes = 0;

  for (const [index, increment] of (
    Array.isArray(portfolio.product_increments)
      ? portfolio.product_increments
      : []
  ).entries()) {
    const label = `portfolio.product_increments[${index}]`;
    if (!isObject(increment)) {
      errors.push(`${label} must be an object`);
      continue;
    }
    rejectUnknownKeys(
      increment,
      [
        "id",
        "outcome_id",
        "title",
        "plain_language_summary",
        "owner",
        "lifecycle_state",
        "stage",
        "status_updated_at",
        "current_status",
        "next_action",
        "plan_ref",
        "integration_strategy",
        "routing",
        "provides",
        "requires",
        "benefits_from",
        "evidence",
      ],
      label,
      errors,
    );
    requirePattern(increment, "id", RECORD_ID, label, errors);
    requirePattern(increment, "outcome_id", RECORD_ID, label, errors);
    for (const key of [
      "title",
      "plain_language_summary",
      "owner",
      "lifecycle_state",
      "stage",
      "status_updated_at",
      "current_status",
      "next_action",
      "integration_strategy",
    ]) {
      requireString(increment, key, label, errors);
    }
    requireOptionalString(increment, "plan_ref", label, errors);
    if (
      nonEmptyString(increment.plan_ref) &&
      !PLAN_REF.test(increment.plan_ref)
    ) {
      errors.push(
        `${label}.plan_ref must be a local Product Increment plan filename`,
      );
    }
    if (!validTimestamp(increment.status_updated_at)) {
      errors.push(`${label}.status_updated_at must be an RFC3339 timestamp`);
    } else if (Number.isFinite(nowMs)) {
      const statusTime = Date.parse(increment.status_updated_at);
      if (statusTime > nowMs + FUTURE_CLOCK_SKEW_MS) {
        errors.push(`${label}.status_updated_at is in the future`);
      } else if (nowMs - statusTime > INCREMENT_STATUS_MAX_AGE_MS) {
        reportFreshness(
          `${label}.status_updated_at is older than seven days; verify its plain-language status`,
        );
      }
      if (
        Number.isFinite(reconciliationTime) &&
        statusTime > reconciliationTime + FUTURE_CLOCK_SKEW_MS
      ) {
        errors.push(
          `${label}.status_updated_at is newer than portfolio.last_reconciled_at`,
        );
      }
    }
    if (!LIFECYCLE_STATES.has(increment.lifecycle_state)) {
      errors.push(`${label}.lifecycle_state is not recognized`);
    }
    if (!STAGES.has(increment.stage)) {
      errors.push(`${label}.stage is not recognized`);
    }
    if (!INTEGRATION_STRATEGIES.has(increment.integration_strategy)) {
      errors.push(`${label}.integration_strategy is not recognized`);
    }
    if (incrementById.has(increment.id)) {
      errors.push(`Product Increment id ${increment.id} is duplicated`);
    } else if (nonEmptyString(increment.id)) {
      incrementById.set(increment.id, increment);
    }

    if (!isObject(increment.routing)) {
      errors.push(`${label}.routing must be an object`);
    } else {
      rejectUnknownKeys(
        increment.routing,
        ["current_maturity", "minimum_maturity", "fallback"],
        `${label}.routing`,
        errors,
      );
      for (const key of ["current_maturity", "minimum_maturity", "fallback"]) {
        requireString(increment.routing, key, `${label}.routing`, errors);
      }
      const current = increment.routing.current_maturity;
      const minimum = increment.routing.minimum_maturity;
      if (!Object.hasOwn(ROUTING_MATURITY_RANK, current)) {
        errors.push(`${label}.routing.current_maturity is not recognized`);
      }
      if (!Object.hasOwn(ROUTING_MATURITY_RANK, minimum)) {
        errors.push(`${label}.routing.minimum_maturity is not recognized`);
      }
      if (
        EXECUTING_STATES.has(increment.lifecycle_state) &&
        Object.hasOwn(ROUTING_MATURITY_RANK, current) &&
        Object.hasOwn(ROUTING_MATURITY_RANK, minimum) &&
        ROUTING_MATURITY_RANK[current] < ROUTING_MATURITY_RANK[minimum]
      ) {
        errors.push(
          `${label} is executing below its minimum routing maturity`,
        );
      }
    }

    if (!Array.isArray(increment.provides) || increment.provides.length === 0) {
      errors.push(`${label}.provides must be a non-empty array`);
    }
    for (const [capabilityIndex, capability] of (
      Array.isArray(increment.provides) ? increment.provides : []
    ).entries()) {
      const capabilityLabel = `${label}.provides[${capabilityIndex}]`;
      if (!isObject(capability)) {
        errors.push(`${capabilityLabel} must be an object`);
        continue;
      }
      rejectUnknownKeys(
        capability,
        ["capability_id", "summary", "state", "evidence_ref"],
        capabilityLabel,
        errors,
      );
      requirePattern(
        capability,
        "capability_id",
        CAPABILITY_ID,
        capabilityLabel,
        errors,
      );
      for (const key of ["summary", "state"]) {
        requireString(capability, key, capabilityLabel, errors);
      }
      requireOptionalString(
        capability,
        "evidence_ref",
        capabilityLabel,
        errors,
      );
      if (!Object.hasOwn(CAPABILITY_STATE_RANK, capability.state)) {
        errors.push(`${capabilityLabel}.state is not recognized`);
      }
      if (
        ["implemented", "accepted"].includes(capability.state) &&
        !nonEmptyString(capability.evidence_ref)
      ) {
        errors.push(
          `${capabilityLabel}.evidence_ref is required for ${capability.state}`,
        );
      }
      validateCapabilityEvidenceRef(
        capability.evidence_ref,
        `${capabilityLabel}.evidence_ref`,
        errors,
        { evidenceExists, gitEvidenceExists, pathIsReviewable },
      );
      if (capabilityById.has(capability.capability_id)) {
        errors.push(
          `capability ${capability.capability_id} has more than one Product Increment provider`,
        );
      } else if (nonEmptyString(capability.capability_id)) {
        capabilityById.set(capability.capability_id, {
          capability,
          incrementId: increment.id,
        });
      }
    }
    const allowedCapabilityStates =
      CAPABILITY_STATES_BY_LIFECYCLE[increment.lifecycle_state];
    if (allowedCapabilityStates) {
      for (const [capabilityIndex, capability] of (
        Array.isArray(increment.provides) ? increment.provides : []
      ).entries()) {
        if (!allowedCapabilityStates.includes(capability?.state)) {
          errors.push(
            `${label}.provides[${capabilityIndex}].state ${capability?.state} is incompatible with lifecycle_state ${increment.lifecycle_state}; expected ${allowedCapabilityStates.join(" or ")}`,
          );
        }
      }
    }

    if (
      increment.lifecycle_state === "accepted" &&
      (increment.provides ?? []).some(
        (capability) => capability.state !== "accepted",
      )
    ) {
      errors.push(`${label} accepted requires all provided capabilities accepted`);
    }

    for (const key of ["requires", "benefits_from", "evidence"]) {
      if (!Array.isArray(increment[key])) {
        errors.push(`${label}.${key} must be an array`);
      }
    }
    for (const [evidenceIndex, evidence] of (
      Array.isArray(increment.evidence) ? increment.evidence : []
    ).entries()) {
      const evidenceLabel = `${label}.evidence[${evidenceIndex}]`;
      if (!isObject(evidence)) {
        errors.push(`${evidenceLabel} must be an object`);
        continue;
      }
      rejectUnknownKeys(
        evidence,
        ["kind", "ref", "summary"],
        evidenceLabel,
        errors,
      );
      for (const key of ["kind", "ref", "summary"]) {
        requireString(evidence, key, evidenceLabel, errors);
      }
      if (!EVIDENCE_KINDS.has(evidence.kind)) {
        errors.push(`${evidenceLabel}.kind is not recognized`);
      }
      if (
        evidence.kind === "document" &&
        nonEmptyString(evidence.ref)
      ) {
        if (
          !isSafeRepositoryPath(evidence.ref) ||
          evidence.ref.startsWith("target/")
        ) {
          errors.push(
            `${evidenceLabel}.ref must identify a durable repository-relative document, not ${evidence.ref}`,
          );
        } else if (
          typeof evidenceExists === "function" &&
          !evidenceExists(evidence.ref)
        ) {
          errors.push(
            `${evidenceLabel}.ref does not exist in this checkout: ${evidence.ref}`,
          );
        } else if (
          typeof pathIsReviewable === "function" &&
          !pathIsReviewable(evidence.ref)
        ) {
          errors.push(
            `${evidenceLabel}.ref is ignored, reserved, or traverses a symlink: ${evidence.ref}`,
          );
        }
      }
      if (
        evidence.kind === "git" &&
        nonEmptyString(evidence.ref)
      ) {
        if (!FULL_SHA.test(evidence.ref)) {
          errors.push(`${evidenceLabel}.ref must be a full lowercase Git SHA`);
        } else if (
          typeof gitEvidenceExists === "function" &&
          !gitEvidenceExists(evidence.ref)
        ) {
          errors.push(
            `${evidenceLabel}.ref is not durably reachable from a branch or tag`,
          );
        }
      }
      if (
        evidence.kind === "branch" &&
        nonEmptyString(evidence.ref)
      ) {
        if (!BRANCH_REF.test(evidence.ref)) {
          errors.push(`${evidenceLabel}.ref must be a purpose-named branch`);
        } else if (
          typeof branchExists === "function" &&
          !branchExists(evidence.ref)
        ) {
          errors.push(
            `${evidenceLabel}.ref does not resolve to a local or remote branch`,
          );
        }
      }
    }

    if (PLAN_REQUIRED_STATES.has(increment.lifecycle_state)) {
      requireString(increment, "plan_ref", label, errors);
    }
    if (
      nonEmptyString(increment.plan_ref) &&
      increment.integration_strategy === "not_yet_planned"
    ) {
      errors.push(
        `${label} with plan_ref cannot use integration_strategy not_yet_planned`,
      );
    }
    if (
      nonEmptyString(increment.plan_ref) &&
      PLAN_REF.test(increment.plan_ref) &&
      loadPlan
    ) {
      let plan;
      try {
        plan = loadPlan(increment.plan_ref);
      } catch (error) {
        errors.push(`${label}.plan_ref is not readable: ${error.message}`);
      }
      if (plan) {
        const planResult = validateProductIncrementPlan(plan, {
          acceptedEvidenceExists,
          economicsLimits: {
            context_source_count:
              portfolio.economics_control?.context_capsule_max_controlling_records,
            delegation_depth:
              portfolio.economics_control?.default_max_delegation_depth,
          },
          evidenceExists,
          gitChangedPaths,
          gitEvidenceExists,
          gitIsAncestor,
          loadAcceptedEvidenceBytes,
          loadAcceptedEvidenceRecord,
          loadEvidenceBytes,
          loadEvidenceRecord,
          pathIsReviewable,
          verifyAuthorityReceipt,
          verifyDestinationReceipt,
          verifyUsageReceipt,
        });
        for (const error of planResult.errors) {
          errors.push(`${label}.plan_ref: ${error}`);
        }
        for (const warning of planResult.warnings) {
          warnings.push(`${increment.id}: ${warning}`);
        }
        if (plan.product_increment?.id !== increment.id) {
          errors.push(
            `${label}.plan_ref Product Increment id does not match ${increment.id}`,
          );
        }
        if (plan.product_increment?.outcome_id !== increment.outcome_id) {
          errors.push(
            `${label}.plan_ref Outcome id does not match ${increment.outcome_id}`,
          );
        }
        if (plan.product_increment?.stage !== increment.stage) {
          errors.push(
            `${label}.plan_ref stage does not match portfolio`,
          );
        }
        if (plan.integration?.strategy !== increment.integration_strategy) {
          errors.push(
            `${label}.plan_ref integration strategy does not match portfolio`,
          );
        }
        if (
          ["active", "under_review", "implemented"].includes(
            increment.lifecycle_state,
          ) &&
          plan.product_increment?.planning_posture !== "active"
        ) {
          errors.push(
            `${label}.plan_ref must have active planning_posture for ${increment.lifecycle_state}`,
          );
        }
        const integrationEvidence = planResult.summary?.integration_evidence;
        if (
          ["under_review", "implemented", "accepted"].includes(
            increment.lifecycle_state,
          ) &&
          !integrationEvidence?.proof_passed
        ) {
          errors.push(
            `${label} ${increment.lifecycle_state} requires a passing Product Increment integration evidence record`,
          );
        }
        if (
          ["implemented", "accepted"].includes(increment.lifecycle_state) &&
          !integrationEvidence?.review_eligible
        ) {
          errors.push(
            `${label} ${increment.lifecycle_state} requires an eligible exact-candidate integration review`,
          );
        }
        if (
          increment.lifecycle_state === "accepted" &&
          !integrationEvidence?.product_accepted
        ) {
          errors.push(
            `${label} accepted requires a Product Increment acceptance receipt from ${increment.owner}`,
          );
        }
        const laneCreditRanks = (plan.lanes ?? []).map(
          (lane) => DELIVERY_CREDIT_RANK[lane.delivery_credit] ?? -1,
        );
        const planDeliveryRank =
          laneCreditRanks.length > 0 ? Math.min(...laneCreditRanks) : -1;
        const planCapabilities = new Map(
          (plan.lanes ?? []).flatMap((lane) =>
            (lane.provides ?? []).map((capability) => [
              capability.capability_id,
              capability,
            ]),
          ),
        );
        const portfolioCapabilities = new Map(
          (increment.provides ?? []).map((capability) => [
            capability.capability_id,
            capability,
          ]),
        );
        for (const capability of increment.provides ?? []) {
          const plannedCapability = planCapabilities.get(
            capability.capability_id,
          );
          if (!plannedCapability) {
            errors.push(
              `${label}.provides capability ${capability.capability_id} is not declared by plan_ref`,
            );
            continue;
          }
          if (capability.state !== plannedCapability.state) {
            errors.push(
              `${label}.provides capability ${capability.capability_id} state must exactly match plan_ref`,
            );
          }
          if (
            (capability.evidence_ref ?? null) !==
            (plannedCapability.evidence_ref ?? null)
          ) {
            errors.push(
              `${label}.provides capability ${capability.capability_id} evidence_ref must exactly match plan_ref`,
            );
          }
        }
        for (const capabilityId of planCapabilities.keys()) {
          if (!portfolioCapabilities.has(capabilityId)) {
            errors.push(
              `${label}.provides omits plan_ref capability ${capabilityId}`,
            );
          }
        }
        if (
          increment.lifecycle_state === "under_review" &&
          !(plan.lanes ?? []).some((lane) => lane.status === "under_review")
        ) {
          errors.push(
            `${label}.plan_ref must contain an under_review lane for under_review lifecycle`,
          );
        }
        if (
          increment.lifecycle_state === "implemented" &&
          (!(increment.provides ?? []).every(
            (capability) =>
              CAPABILITY_STATE_RANK[capability.state] >=
              CAPABILITY_STATE_RANK.implemented,
          ) ||
            planDeliveryRank < CAPABILITY_STATE_RANK.implemented)
        ) {
          errors.push(
            `${label} implemented lifecycle requires implemented capabilities and plan-wide delivery credit`,
          );
        }
        if (
          increment.lifecycle_state === "accepted" &&
          planDeliveryRank < CAPABILITY_STATE_RANK.accepted
        ) {
          errors.push(
            `${label}.plan_ref requires plan-wide accepted delivery credit when accepted`,
          );
        }
        derivedActiveLanes += planResult.summary?.active_lane_count ?? 0;
        derivedReviewLanes += (plan.lanes ?? []).filter(
          (lane) => lane.status === "under_review",
        ).length;
        for (const lane of plan.lanes ?? []) {
          if (lane.status !== "in_progress") continue;
          activePlanLanes.push({
            productIncrementId: increment.id,
            laneId: lane.id,
            branchOwner: lane.branch_owner,
            branch: lane.branch,
            writeRoots: lane.write_roots ?? [],
          });
        }
        planSummaries.set(increment.id, planResult.summary);
      }
    }
  }

  const requirementGraph = new Map(
    [...incrementById.keys()].map((id) => [id, []]),
  );
  const relationKeys = new Set();

  for (const [index, increment] of (
    Array.isArray(portfolio.product_increments)
      ? portfolio.product_increments
      : []
  ).entries()) {
    const label = `portfolio.product_increments[${index}]`;
    for (const [requirementIndex, requirement] of (
      Array.isArray(increment.requires) ? increment.requires : []
    ).entries()) {
      const relationLabel = `${label}.requires[${requirementIndex}]`;
      if (!isObject(requirement)) {
        errors.push(`${relationLabel} must be an object`);
        continue;
      }
      rejectUnknownKeys(
        requirement,
        [
          "from_product_increment_id",
          "capability_id",
          "required_state",
          "reason",
          "decoupling",
          "decoupling_ref",
        ],
        relationLabel,
        errors,
      );
      requirePattern(
        requirement,
        "from_product_increment_id",
        RECORD_ID,
        relationLabel,
        errors,
      );
      requirePattern(
        requirement,
        "capability_id",
        CAPABILITY_ID,
        relationLabel,
        errors,
      );
      for (const key of ["required_state", "reason", "decoupling"]) {
        requireString(requirement, key, relationLabel, errors);
      }
      requireOptionalString(
        requirement,
        "decoupling_ref",
        relationLabel,
        errors,
      );
      const key = `${increment.id}|requires|${requirement.from_product_increment_id}|${requirement.capability_id}`;
      if (relationKeys.has(key)) {
        errors.push(`${relationLabel} duplicates an existing relationship`);
      }
      relationKeys.add(key);
      const provider = incrementById.get(requirement.from_product_increment_id);
      if (!provider) {
        errors.push(
          `${relationLabel}.from_product_increment_id is not registered`,
        );
      }
      if (requirement.from_product_increment_id === increment.id) {
        errors.push(`${relationLabel} cannot require itself`);
      }
      const provided = capabilityById.get(requirement.capability_id);
      if (!provided) {
        errors.push(`${relationLabel}.capability_id has no registered provider`);
      } else if (
        provided.incrementId !== requirement.from_product_increment_id
      ) {
        errors.push(
          `${relationLabel}.capability_id is provided by ${provided.incrementId}`,
        );
      }
      if (!["implemented", "accepted"].includes(requirement.required_state)) {
        errors.push(`${relationLabel}.required_state is not recognized`);
      }
      if (
        !["none", "frozen_contract", "fixture", "adapter", "manual_fallback"].includes(
          requirement.decoupling,
        )
      ) {
        errors.push(`${relationLabel}.decoupling is not recognized`);
      }
      if (
        requirement.decoupling !== "none" &&
        !nonEmptyString(requirement.decoupling_ref)
      ) {
        errors.push(
          `${relationLabel}.decoupling_ref is required for ${requirement.decoupling}`,
        );
      }
      if (
        requirement.decoupling === "none" &&
        requirement.decoupling_ref !== undefined
      ) {
        errors.push(
          `${relationLabel}.decoupling_ref is not allowed when decoupling is none`,
        );
      }
      if (
        requirement.decoupling !== "none" &&
        nonEmptyString(requirement.decoupling_ref)
      ) {
        validateDocumentRef(
          requirement.decoupling_ref,
          `${relationLabel}.decoupling_ref`,
          errors,
          { evidenceExists, pathIsReviewable },
        );
      }
      const sufficient = stateSatisfies(
        provided?.capability?.state,
        requirement.required_state,
      );
      if (
        EXECUTING_STATES.has(increment.lifecycle_state) &&
        !sufficient &&
        requirement.decoupling === "none"
      ) {
        errors.push(
          `${relationLabel} leaves executing ${increment.id} blocked by an unfinished required capability`,
        );
      }
      if (!sufficient && requirement.decoupling === "none") {
        requirementGraph
          .get(increment.id)
          ?.push(requirement.from_product_increment_id);
      }
    }

    for (const [benefitIndex, benefit] of (
      Array.isArray(increment.benefits_from) ? increment.benefits_from : []
    ).entries()) {
      const relationLabel = `${label}.benefits_from[${benefitIndex}]`;
      if (!isObject(benefit)) {
        errors.push(`${relationLabel} must be an object`);
        continue;
      }
      rejectUnknownKeys(
        benefit,
        [
          "from_product_increment_id",
          "capability_id",
          "expected_benefit",
          "adoption_trigger",
        ],
        relationLabel,
        errors,
      );
      requirePattern(
        benefit,
        "from_product_increment_id",
        RECORD_ID,
        relationLabel,
        errors,
      );
      requirePattern(
        benefit,
        "capability_id",
        CAPABILITY_ID,
        relationLabel,
        errors,
      );
      for (const key of ["expected_benefit", "adoption_trigger"]) {
        requireString(benefit, key, relationLabel, errors);
      }
      const key = `${increment.id}|benefits_from|${benefit.from_product_increment_id}|${benefit.capability_id}`;
      if (relationKeys.has(key)) {
        errors.push(`${relationLabel} duplicates an existing relationship`);
      }
      relationKeys.add(key);
      if (!incrementById.has(benefit.from_product_increment_id)) {
        errors.push(
          `${relationLabel}.from_product_increment_id is not registered`,
        );
      }
      if (benefit.from_product_increment_id === increment.id) {
        errors.push(`${relationLabel} cannot benefit from itself`);
      }
      const provided = capabilityById.get(benefit.capability_id);
      if (!provided) {
        errors.push(`${relationLabel}.capability_id has no registered provider`);
      } else if (provided.incrementId !== benefit.from_product_increment_id) {
        errors.push(
          `${relationLabel}.capability_id is provided by ${provided.incrementId}`,
        );
      }
    }
  }

  const cycle = findCycle(requirementGraph);
  if (cycle) {
    errors.push(`unresolved Product Increment dependency cycle: ${cycle.join(" -> ")}`);
  }

  for (let leftIndex = 0; leftIndex < activePlanLanes.length; leftIndex += 1) {
    for (
      let rightIndex = leftIndex + 1;
      rightIndex < activePlanLanes.length;
      rightIndex += 1
    ) {
      const left = activePlanLanes[leftIndex];
      const right = activePlanLanes[rightIndex];
      if (left.branchOwner === right.branchOwner) {
        errors.push(
          `active branch owner ${left.branchOwner} owns lanes ${left.productIncrementId}/${left.laneId} and ${right.productIncrementId}/${right.laneId}`,
        );
      }
      if (left.branch === right.branch) {
        errors.push(
          `active lanes ${left.productIncrementId}/${left.laneId} and ${right.productIncrementId}/${right.laneId} share branch ${left.branch}`,
        );
      }
      for (const leftRoot of left.writeRoots) {
        for (const rightRoot of right.writeRoots) {
          if (pathsOverlap(leftRoot, rightRoot)) {
            errors.push(
              `active lanes ${left.productIncrementId}/${left.laneId} and ${right.productIncrementId}/${right.laneId} overlap write roots ${leftRoot} and ${rightRoot}`,
            );
          }
        }
      }
    }
  }

  if (
    loadPlan &&
    Number.isInteger(capacity?.current_active_lanes) &&
    capacity.current_active_lanes !== derivedActiveLanes
  ) {
    errors.push(
      `portfolio current_active_lanes ${capacity.current_active_lanes} does not match plan-derived ${derivedActiveLanes}`,
    );
  }
  if (
    loadPlan &&
    Number.isInteger(capacity?.current_review_lanes) &&
    capacity.current_review_lanes !== derivedReviewLanes
  ) {
    errors.push(
      `portfolio current_review_lanes ${capacity.current_review_lanes} does not match plan-derived ${derivedReviewLanes}`,
    );
  }

  const increments = [...incrementById.values()].map((increment) => ({
    id: increment.id,
    title: increment.title,
    lifecycle_state: increment.lifecycle_state,
    stage: increment.stage,
    current_status: increment.current_status,
    next_action: increment.next_action,
    integration_strategy: increment.integration_strategy,
    routing_maturity: increment.routing?.current_maturity ?? null,
    provides: (increment.provides ?? []).map((capability) => ({
      capability_id: capability.capability_id,
      state: capability.state,
    })),
    requires: (increment.requires ?? []).map((requirement) => ({
      product_increment_id: requirement.from_product_increment_id,
      capability_id: requirement.capability_id,
      required_state: requirement.required_state,
    })),
    benefits_from: (increment.benefits_from ?? []).map((benefit) => ({
      product_increment_id: benefit.from_product_increment_id,
      capability_id: benefit.capability_id,
    })),
    plan: planSummaries.get(increment.id) ?? null,
  }));

  return {
    ok: errors.length === 0,
    errors,
    warnings,
    summary: {
      portfolio_id: portfolio.portfolio_id ?? null,
      catalog_state: portfolio.catalog_state ?? null,
      last_reconciled_at: portfolio.last_reconciled_at ?? null,
      target_active_lanes: capacity?.target_active_lanes ?? null,
      maximum_active_lanes: capacity?.maximum_active_lanes ?? null,
      current_active_lanes: capacity?.current_active_lanes ?? null,
      current_review_lanes: capacity?.current_review_lanes ?? null,
      below_target_reason: capacity?.below_target_reason ?? null,
      economics_control: isObject(economicsControl)
        ? {
            context_capsule_max_controlling_records:
              economicsControl.context_capsule_max_controlling_records ?? null,
            default_max_delegation_depth:
              economicsControl.default_max_delegation_depth ?? null,
            correction_cycles_before_replan:
              economicsControl.correction_cycles_before_replan ?? null,
            idle_agent_policy: economicsControl.idle_agent_policy ?? null,
            model_routing_policy: economicsControl.model_routing_policy ?? null,
            evidence_measurement_required:
              economicsControl.evidence_measurement_required ?? null,
            primary_metrics: [...(economicsControl.primary_metrics ?? [])],
          }
        : null,
      product_increment_count: incrementById.size,
      implemented_product_increment_count: increments.filter((increment) =>
        increment.provides.some(
          (capability) =>
            CAPABILITY_STATE_RANK[capability.state] >=
            CAPABILITY_STATE_RANK.implemented,
        ),
      ).length,
      accepted_product_increment_count: increments.filter(
        (increment) => increment.lifecycle_state === "accepted",
      ).length,
      increments,
      known_unregistered_work: portfolio.known_unregistered_work ?? [],
    },
  };
}

function printHuman(result, portfolioPath, selectedId = null) {
  const status = result.ok ? "valid" : "invalid";
  console.log(`Product Increment portfolio ${status}: ${portfolioPath}`);
  if (result.summary) {
    console.log(
      `  ${result.summary.product_increment_count} increments; ${result.summary.current_active_lanes}/${result.summary.target_active_lanes} active lanes; ${result.summary.current_review_lanes} in review`,
    );
    console.log(
      `  ${result.summary.implemented_product_increment_count} with implemented capability; ${result.summary.accepted_product_increment_count} accepted`,
    );
    if (result.summary.economics_control) {
      console.log(
        `  Economics: ${result.summary.economics_control.model_routing_policy}; context <= ${result.summary.economics_control.context_capsule_max_controlling_records} records; delegation depth <= ${result.summary.economics_control.default_max_delegation_depth}`,
      );
    }
    const increments = selectedId
      ? result.summary.increments.filter((item) => item.id === selectedId)
      : result.summary.increments;
    for (const increment of increments) {
      console.log(
        `\n${increment.id} [${increment.lifecycle_state}; ${increment.stage}] ${increment.title}`,
      );
      console.log(`  Now: ${increment.current_status}`);
      console.log(`  Next: ${increment.next_action}`);
      console.log(
        `  Provides: ${increment.provides.map((item) => `${item.capability_id} (${item.state})`).join(", ")}`,
      );
      console.log(
        `  Requires: ${
          increment.requires.length === 0
            ? "none"
            : increment.requires
                .map(
                  (item) =>
                    `${item.product_increment_id}/${item.capability_id} (${item.required_state})`,
                )
                .join(", ")
        }`,
      );
      console.log(
        `  Benefits from: ${
          increment.benefits_from.length === 0
            ? "none"
            : increment.benefits_from
                .map(
                  (item) =>
                    `${item.product_increment_id}/${item.capability_id}`,
                )
                .join(", ")
        }`,
      );
    }
  }
  for (const warning of result.warnings) console.log(`  warning: ${warning}`);
  for (const error of result.errors) console.error(`  error: ${error}`);
}

function gitEvidenceReachable(repositoryRoot, sha) {
  const object = spawnSync(
    "git",
    ["cat-file", "-e", `${sha}^{commit}`],
    { cwd: repositoryRoot, encoding: "utf8" },
  );
  if (object.status !== 0) return false;
  const containingRefs = spawnSync(
    "git",
    [
      "for-each-ref",
      "--format=%(refname)",
      "--contains",
      sha,
      "refs/heads",
      "refs/remotes",
      "refs/tags",
    ],
    { cwd: repositoryRoot, encoding: "utf8" },
  );
  return containingRefs.status === 0 && containingRefs.stdout.trim() !== "";
}

function loadAcceptedMainBytes(repositoryRoot, evidencePath) {
  const result = spawnSync(
    "git",
    ["show", `refs/remotes/origin/main:${evidencePath}`],
    { cwd: repositoryRoot, encoding: null },
  );
  if (result.status !== 0) {
    throw new Error(
      Buffer.from(result.stderr ?? "").toString("utf8").trim() ||
        `accepted main does not retain ${evidencePath}`,
    );
  }
  return result.stdout;
}

function main() {
  let options;
  try {
    options = parseArgs(process.argv.slice(2));
  } catch (error) {
    console.error(error.message);
    console.error(usage());
    process.exitCode = 2;
    return;
  }
  if (options.help) {
    console.log(usage());
    return;
  }

  const portfolioPath = resolve(process.cwd(), options.portfolio);
  let portfolio;
  try {
    portfolio = JSON.parse(readFileSync(portfolioPath, "utf8"));
  } catch (error) {
    const result = {
      command: "check-product-increment-portfolio",
      schema: PORTFOLIO_SCHEMA,
      ok: false,
      portfolio: options.portfolio,
      errors: [`portfolio is not readable JSON: ${error.message}`],
      warnings: [],
      summary: null,
    };
    if (options.json) console.log(JSON.stringify(result, null, 2));
    else printHuman(result, options.portfolio, options.id);
    process.exitCode = 1;
    return;
  }

  const portfolioDir = dirname(portfolioPath);
  const repositoryRoot = resolve(portfolioDir, "../..");
  const validation = validateProductIncrementPortfolio(portfolio, {
    acceptedEvidenceExists(evidencePath) {
      return (
        spawnSync(
          "git",
          [
            "cat-file",
            "-e",
            `refs/remotes/origin/main:${evidencePath}`,
          ],
          { cwd: repositoryRoot, encoding: "utf8" },
        ).status === 0
      );
    },
    branchExists(branch) {
      return [
        `refs/heads/${branch}`,
        `refs/remotes/origin/${branch}`,
      ].some(
        (ref) =>
          spawnSync("git", ["show-ref", "--verify", "--quiet", ref], {
            cwd: repositoryRoot,
            encoding: "utf8",
          }).status === 0,
      );
    },
    loadPlan(planRef) {
      const planPath = resolve(portfolioDir, planRef);
      if (!existsSync(planPath)) {
        throw new Error(`missing ${planPath}`);
      }
      return JSON.parse(readFileSync(planPath, "utf8"));
    },
    evidenceExists(evidenceRef) {
      return existsSync(resolve(repositoryRoot, evidenceRef));
    },
    gitEvidenceExists(sha) {
      return gitEvidenceReachable(repositoryRoot, sha);
    },
    gitIsAncestor(baseSha, candidateSha) {
      return (
        spawnSync(
          "git",
          ["merge-base", "--is-ancestor", baseSha, candidateSha],
          { cwd: repositoryRoot, encoding: "utf8" },
        ).status === 0
      );
    },
    gitChangedPaths(baseSha, candidateSha) {
      const result = spawnSync(
        "git",
        [
          "diff",
          "--name-only",
          "--no-renames",
          baseSha,
          candidateSha,
          "--",
        ],
        { cwd: repositoryRoot, encoding: "utf8" },
      );
      if (result.status !== 0) {
        throw new Error(result.stderr.trim() || "git diff failed");
      }
      return result.stdout.split("\n").filter(Boolean);
    },
    loadAcceptedEvidenceBytes(evidenceRef) {
      return loadAcceptedMainBytes(repositoryRoot, evidenceRef);
    },
    loadAcceptedEvidenceRecord(evidenceRef) {
      return JSON.parse(
        loadAcceptedMainBytes(repositoryRoot, evidenceRef).toString("utf8"),
      );
    },
    loadEvidenceBytes(evidenceRef) {
      return readFileSync(resolve(repositoryRoot, evidenceRef));
    },
    loadEvidenceRecord(evidenceRef) {
      return JSON.parse(
        readFileSync(resolve(repositoryRoot, evidenceRef), "utf8"),
      );
    },
    pathIsReviewable(path, options = {}) {
      return repositoryPathIsReviewable(repositoryRoot, path, options);
    },
  });
  if (
    options.id &&
    !validation.summary?.increments.some((item) => item.id === options.id)
  ) {
    validation.errors.push(`Product Increment ${options.id} is not registered`);
    validation.ok = false;
  }
  const result = {
    command: "check-product-increment-portfolio",
    schema: PORTFOLIO_SCHEMA,
    portfolio: options.portfolio,
    selected_product_increment_id: options.id,
    ...validation,
  };
  if (options.json) console.log(JSON.stringify(result, null, 2));
  else printHuman(result, options.portfolio, options.id);
  if (!result.ok) process.exitCode = 1;
}

const scriptPath = fileURLToPath(import.meta.url);
if (process.argv[1] && resolve(process.argv[1]) === resolve(scriptPath)) {
  main();
}
