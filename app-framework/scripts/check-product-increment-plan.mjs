#!/usr/bin/env node

import { spawnSync } from "node:child_process";
import { createHash } from "node:crypto";
import { existsSync, readFileSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

import {
  isSensitiveAuthorityPath,
  isSafeRepositoryPath,
  repositoryPathIsReviewable,
} from "./product-increment-path-safety.mjs";

const PLAN_SCHEMA = "appfw_product_increment_plan@1";
const EVIDENCE_SCHEMA = "appfw_product_increment_evidence@1";
const LANE_STATUSES = new Set([
  "not_started",
  "ready_to_start",
  "in_progress",
  "waiting_for_decision",
  "waiting_for_team",
  "under_review",
  "implemented",
  "accepted",
  "deferred",
  "stopped",
]);
const RUNNABLE_STATUSES = new Set(["ready_to_start", "in_progress"]);
const CAPABILITY_STATE_RANK = {
  planned: 0,
  implemented: 1,
  accepted: 2,
};
const DELIVERY_CREDIT_RANK = {
  none: 0,
  implemented: 1,
  accepted: 2,
};
const LEAF_BRANCH = /^(feature|fix|docs|maintenance)\/[a-z0-9][a-z0-9._/-]*$/;
const INTEGRATION_BRANCH = /^integrate\/[a-z0-9][a-z0-9._/-]*$/;
const PROGRAM_ID = /^[a-z0-9][a-z0-9._-]{1,63}$/;
const RECORD_ID = /^[A-Z0-9][A-Z0-9._-]{1,63}$/;
const CAPABILITY_ID = /^[a-z0-9][a-z0-9._-]{1,95}$/;
const FULL_SHA = /^[0-9a-f]{40}$/;
const LANE_CREDIT_BY_STATUS = {
  not_started: ["none"],
  ready_to_start: ["none"],
  in_progress: ["none"],
  waiting_for_decision: ["none", "implemented"],
  waiting_for_team: ["none"],
  under_review: ["none", "implemented"],
  implemented: ["implemented"],
  accepted: ["accepted"],
  deferred: ["none"],
  stopped: ["none"],
};
const EVIDENCE_REQUIRED_STATUSES = new Set([
  "waiting_for_decision",
  "under_review",
  "implemented",
  "accepted",
]);
const RFC3339_DATE_TIME =
  /^(\d{4})-(\d{2})-(\d{2})T(?:[01]\d|2[0-3]):[0-5]\d:[0-5]\d(?:\.\d+)?(?:Z|[+-](?:[01]\d|2[0-3]):[0-5]\d)$/;
const OUTPUT_SHA256 = /^[0-9a-f]{64}$/;
const DEFAULT_ECONOMICS_LIMITS = {
  context_source_count: 4,
  delegation_depth: 1,
};
const MAX_ECONOMICS_BUDGET = {
  max_elapsed_ms: 28_800_000,
  max_input_tokens: 2_000_000,
  max_output_tokens: 500_000,
  max_cost_usd: 500,
  max_retries: 3,
  max_corrections: 2,
  max_rework_cycles: 2,
};
const ROUTING_BASES = new Set([
  "lowest_capable",
  "escalated_novel_synthesis",
  "escalated_independent_review",
]);

function usage() {
  return `Usage:
  node scripts/check-product-increment-plan.mjs --plan <path> [--json]
  node scripts/check-product-increment-plan.mjs --plan <path> \
    --current-diff <delivery-lane-id> [--json]

Validates the required Product Increment, one-to-four Delivery Lane,
cross-lane capability, branch, and integration-plan contract. --current-diff
also verifies the current Git branch and changed paths against one lane.`;
}

function parseArgs(argv) {
  const options = { currentDiffLane: null, json: false, plan: null };
  for (let index = 0; index < argv.length; index += 1) {
    const arg = argv[index];
    if (arg === "--json") {
      options.json = true;
    } else if (arg === "--help" || arg === "-h") {
      options.help = true;
    } else if (arg === "--plan" || arg === "--current-diff") {
      const value = argv[index + 1];
      if (!value || value.startsWith("--")) {
        throw new Error(`${arg} requires a value`);
      }
      if (arg === "--plan") options.plan = value;
      else options.currentDiffLane = value;
      index += 1;
    } else {
      throw new Error(`unknown argument: ${arg}`);
    }
  }
  if (!options.help && !options.plan) {
    throw new Error("--plan is required");
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

function requireStringArray(
  record,
  key,
  label,
  errors,
  { allowEmpty = false } = {},
) {
  const value = record?.[key];
  if (!Array.isArray(value) || (!allowEmpty && value.length === 0)) {
    errors.push(
      `${label}.${key} must be ${allowEmpty ? "an" : "a non-empty"} array`,
    );
    return false;
  }
  if (value.some((item) => !nonEmptyString(item))) {
    errors.push(`${label}.${key} must contain only non-empty strings`);
    return false;
  }
  if (new Set(value).size !== value.length) {
    errors.push(`${label}.${key} must not contain duplicates`);
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

function validateEconomicsBudget(budget, label, errors) {
  if (!isObject(budget)) {
    errors.push(`${label} must be an object`);
    return;
  }
  rejectUnknownKeys(
    budget,
    [
      "default_model_tier",
      "allowed_model_tiers",
      "max_elapsed_ms",
      "max_input_tokens",
      "max_output_tokens",
      "max_cost_usd",
      "max_retries",
      "max_corrections",
      "max_rework_cycles",
    ],
    label,
    errors,
  );
  requireString(budget, "default_model_tier", label, errors);
  requireStringArray(budget, "allowed_model_tiers", label, errors);
  if (
    nonEmptyString(budget.default_model_tier) &&
    Array.isArray(budget.allowed_model_tiers) &&
    !budget.allowed_model_tiers.includes(budget.default_model_tier)
  ) {
    errors.push(
      `${label}.default_model_tier must be included in allowed_model_tiers`,
    );
  }
  for (const key of [
    "max_elapsed_ms",
    "max_input_tokens",
    "max_output_tokens",
    "max_retries",
    "max_corrections",
    "max_rework_cycles",
  ]) {
    if (
      !Number.isInteger(budget[key]) ||
      budget[key] < 0 ||
      budget[key] > MAX_ECONOMICS_BUDGET[key]
    ) {
      errors.push(
        `${label}.${key} must be an integer from 0 through ${MAX_ECONOMICS_BUDGET[key]}`,
      );
    }
  }
  if (
    typeof budget.max_cost_usd !== "number" ||
    !Number.isFinite(budget.max_cost_usd) ||
    budget.max_cost_usd < 0 ||
    budget.max_cost_usd > MAX_ECONOMICS_BUDGET.max_cost_usd
  ) {
    errors.push(
      `${label}.max_cost_usd must be a number from 0 through ${MAX_ECONOMICS_BUDGET.max_cost_usd}`,
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

function pathIsAuthorizedByRoot(path, root) {
  if (root.endsWith("/**")) {
    const rootBase = pathBase(root);
    return path === rootBase || path.startsWith(`${rootBase}/`);
  }
  return path === root;
}

function validateDurableEvidenceRef(
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
    if (
      !isSafeRepositoryPath(path) ||
      path.startsWith("target/")
    ) {
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

function validateDurableDocumentRef(
  value,
  label,
  errors,
  { evidenceExists = null, pathIsReviewable = null } = {},
) {
  if (!nonEmptyString(value) || !value.startsWith("document:")) {
    errors.push(
      `${label} must use document:<durable-repository-relative-path>`,
    );
    return null;
  }
  const path = value.slice("document:".length);
  if (!isSafeRepositoryPath(path) || path.startsWith("target/")) {
    errors.push(`${label} must identify a durable repository document`);
    return null;
  }
  if (typeof evidenceExists === "function" && !evidenceExists(path)) {
    errors.push(`${label} does not exist: ${path}`);
    return null;
  }
  if (
    typeof pathIsReviewable === "function" &&
    !pathIsReviewable(path)
  ) {
    errors.push(
      `${label} is ignored, reserved, or traverses a symlink: ${path}`,
    );
    return null;
  }
  return path;
}

function validateDocumentDigest(
  reference,
  expectedSha256,
  label,
  errors,
  {
    evidenceExists = null,
    loadEvidenceBytes = null,
    pathIsReviewable = null,
  } = {},
) {
  const path = validateDurableDocumentRef(reference, `${label}.ref`, errors, {
    evidenceExists,
    pathIsReviewable,
  });
  if (!OUTPUT_SHA256.test(expectedSha256 ?? "")) {
    errors.push(`${label}.sha256 must be a lowercase SHA-256`);
    return path;
  }
  if (path && typeof loadEvidenceBytes === "function") {
    let bytes;
    try {
      bytes = loadEvidenceBytes(path);
    } catch (error) {
      errors.push(`${label}.ref is not readable: ${error.message}`);
      return path;
    }
    const actualSha256 = createHash("sha256").update(bytes).digest("hex");
    if (actualSha256 !== expectedSha256) {
      errors.push(`${label}.sha256 does not match the retained document bytes`);
    }
  }
  return path;
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

function reviewIsEligible(review) {
  return (
    isObject(review) &&
    ["go", "go_with_conditions"].includes(review.disposition) &&
    ["blocker", "critical", "important"].every(
      (key) => review.findings?.[key] === 0,
    )
  );
}

function acceptanceIsValid(acceptance, expectedOwner, candidateSha) {
  return (
    isObject(acceptance) &&
    acceptance.owner === expectedOwner &&
    acceptance.decision === "accepted" &&
    acceptance.accepted_sha === candidateSha &&
    validTimestamp(acceptance.decided_at)
  );
}

function requireTrustedReceipt(
  verifier,
  payload,
  label,
  errors,
) {
  if (typeof verifier !== "function") {
    errors.push(
      `${label} requires a trusted external verifier; repository evidence and local remote-tracking refs cannot establish issuer authority`,
    );
    return false;
  }
  try {
    if (verifier(payload) !== true) {
      errors.push(`${label} was not verified by the trusted authority adapter`);
      return false;
    }
  } catch (error) {
    errors.push(`${label} verification failed: ${error.message}`);
    return false;
  }
  return true;
}

function validateDeliveryEvidenceRecord(
  record,
  {
    economicsBudget,
    economicsLimits,
    evidenceExists = null,
    expectedEconomicsOutcome,
    expectedReviewer,
    label,
    loadEvidenceBytes = null,
    plan,
    lane = null,
    expectedScope,
    requiredChecks = [],
    requireDurableProof = false,
    requireMeasuredEconomics = false,
    requiredProfile,
    gitChangedPaths = null,
    gitEvidenceExists = null,
    gitIsAncestor = null,
    pathIsReviewable = null,
    verifyAuthorityReceipt = null,
    verifyDestinationReceipt = null,
    verifyUsageReceipt = null,
  },
  errors,
) {
  if (!isObject(record)) {
    errors.push(`${label} must contain a JSON object`);
    return null;
  }
  rejectUnknownKeys(
    record,
    [
      "$schema",
      "schema",
      "product_increment_id",
      "outcome_id",
      "scope",
      "candidate_sha",
      "base_sha",
      "proof",
      "economics",
      "review",
      "destination_verification",
      "lane_acceptance",
      "product_acceptance",
    ],
    label,
    errors,
  );
  if (record.schema !== EVIDENCE_SCHEMA) {
    errors.push(`${label}.schema must be ${EVIDENCE_SCHEMA}`);
  }
  if (record.product_increment_id !== plan.product_increment?.id) {
    errors.push(`${label}.product_increment_id does not match the plan`);
  }
  if (record.outcome_id !== plan.product_increment?.outcome_id) {
    errors.push(`${label}.outcome_id does not match the plan`);
  }
  if (!FULL_SHA.test(record.candidate_sha ?? "")) {
    errors.push(`${label}.candidate_sha must be a full lowercase Git SHA`);
  } else if (
    typeof gitEvidenceExists === "function" &&
    !gitEvidenceExists(record.candidate_sha)
  ) {
    errors.push(`${label}.candidate_sha is not durably reachable`);
  }
  if (!FULL_SHA.test(record.base_sha ?? "")) {
    errors.push(`${label}.base_sha must be a full lowercase Git SHA`);
  } else if (
    typeof gitEvidenceExists === "function" &&
    !gitEvidenceExists(record.base_sha)
  ) {
    errors.push(`${label}.base_sha is not durably reachable`);
  }
  const expectedBase = lane?.base_sha ?? plan.integration?.base_ref;
  if (FULL_SHA.test(record.base_sha ?? "") && record.base_sha !== expectedBase) {
    errors.push(`${label}.base_sha must equal ${expectedBase}`);
  }
  if (
    FULL_SHA.test(record.base_sha ?? "") &&
    FULL_SHA.test(record.candidate_sha ?? "") &&
    typeof gitIsAncestor === "function" &&
    !gitIsAncestor(record.base_sha, record.candidate_sha)
  ) {
      errors.push(`${label}.base_sha is not an ancestor of candidate_sha`);
  }
  if (
    requireDurableProof &&
    FULL_SHA.test(record.base_sha ?? "") &&
    FULL_SHA.test(record.candidate_sha ?? "")
  ) {
    if (record.base_sha === record.candidate_sha) {
      errors.push(`${label}.candidate_sha must differ from base_sha`);
    } else if (typeof gitChangedPaths !== "function") {
      errors.push(`${label} requires candidate changed-path verification`);
    } else {
      let changedPaths = [];
      try {
        changedPaths = gitChangedPaths(record.base_sha, record.candidate_sha);
      } catch (error) {
        errors.push(`${label} candidate changed paths are unavailable: ${error.message}`);
      }
      if (!Array.isArray(changedPaths) || changedPaths.length === 0) {
        errors.push(`${label}.candidate_sha must contain at least one changed path`);
      } else {
        const allowedRoots = lane
          ? lane.write_roots ?? []
          : [
              ...(plan.lanes ?? []).flatMap((item) => item.write_roots ?? []),
              ...(plan.integration?.write_roots ?? []),
            ];
        for (const changedPath of changedPaths) {
          if (
            !isSafeRepositoryPath(changedPath) ||
            !allowedRoots.some((root) =>
              pathIsAuthorizedByRoot(changedPath, root),
            )
          ) {
            errors.push(
              `${label} candidate changed path ${changedPath} is outside its declared write roots`,
            );
          }
        }
      }
    }
  }

  if (!isObject(record.scope)) {
    errors.push(`${label}.scope must be an object`);
  } else {
    rejectUnknownKeys(
      record.scope,
      ["kind", "lane_id"],
      `${label}.scope`,
      errors,
    );
    if (record.scope.kind !== expectedScope) {
      errors.push(`${label}.scope.kind must be ${expectedScope}`);
    }
    if (
      expectedScope === "delivery_lane" &&
      record.scope.lane_id !== lane?.id
    ) {
      errors.push(`${label}.scope.lane_id must be ${lane?.id}`);
    }
    if (
      expectedScope !== "delivery_lane" &&
      record.scope.lane_id !== undefined
    ) {
      errors.push(`${label}.scope.lane_id is not allowed for ${expectedScope}`);
    }
  }

  const proof = record.proof;
  const proofDocumentPaths = new Set();
  let usageReceiptVerified = false;
  let reviewAuthorityVerified = false;
  let destinationVerified = false;
  const acceptanceAuthorityVerified = {
    lane_acceptance: false,
    product_acceptance: false,
  };
  if (!isObject(proof)) {
    errors.push(`${label}.proof must be an object`);
  } else {
    rejectUnknownKeys(
      proof,
      ["stage", "profile", "status", "completed_at", "checks"],
      `${label}.proof`,
      errors,
    );
    if (proof.stage !== plan.product_increment?.stage) {
      errors.push(`${label}.proof.stage does not match the plan stage`);
    }
    if (proof.profile !== requiredProfile) {
      errors.push(`${label}.proof.profile must be ${requiredProfile}`);
    }
    if (proof.status !== "passed") {
      errors.push(`${label}.proof.status must be passed`);
    }
    if (!validTimestamp(proof.completed_at)) {
      errors.push(`${label}.proof.completed_at must be an RFC3339 timestamp`);
    }
    if (!Array.isArray(proof.checks) || proof.checks.length === 0) {
      errors.push(`${label}.proof.checks must be a non-empty array`);
    }
    for (const [index, check] of (
      Array.isArray(proof.checks) ? proof.checks : []
    ).entries()) {
      const checkLabel = `${label}.proof.checks[${index}]`;
      if (!isObject(check)) {
        errors.push(`${checkLabel} must be an object`);
        continue;
      }
      rejectUnknownKeys(
        check,
        ["command", "result", "output_ref", "output_sha256"],
        checkLabel,
        errors,
      );
      if (!nonEmptyString(check.command)) {
        errors.push(`${checkLabel}.command must be a non-empty string`);
      }
      if (check.result !== "passed") {
        errors.push(`${checkLabel}.result must be passed`);
      }
      if (!OUTPUT_SHA256.test(check.output_sha256 ?? "")) {
        errors.push(
          `${checkLabel}.output_sha256 must be a lowercase SHA-256`,
        );
      }
      if (requireDurableProof) {
        const proofDocumentPath = validateDocumentDigest(
          check.output_ref,
          check.output_sha256,
          `${checkLabel}.output`,
          errors,
          { evidenceExists, loadEvidenceBytes, pathIsReviewable },
        );
        if (proofDocumentPath) proofDocumentPaths.add(proofDocumentPath);
      } else if (
        check.output_ref !== undefined &&
        !nonEmptyString(check.output_ref)
      ) {
        errors.push(`${checkLabel}.output_ref must be a non-empty string`);
      }
    }
    if (requireDurableProof) {
      const observedCommands = new Set(
        (Array.isArray(proof.checks) ? proof.checks : [])
          .map((check) => check?.command)
          .filter(nonEmptyString),
      );
      for (const requiredCheck of requiredChecks) {
        if (!observedCommands.has(requiredCheck)) {
          errors.push(
            `${label}.proof.checks must include declared check: ${requiredCheck}`,
          );
        }
      }
    }
  }

  let meteringDocumentPath = null;
  const economics = record.economics;
  if (!isObject(economics)) {
    errors.push(`${label}.economics must be an object`);
  } else {
    rejectUnknownKeys(
      economics,
      [
        "measurement_state",
        "outcome",
        "planned_model",
        "actual_model",
        "reasoning_effort",
        "routing_basis",
        "routing_justification",
        "metering_receipt_ref",
        "metering_receipt_sha256",
        "elapsed_ms",
        "input_tokens",
        "output_tokens",
        "cost_usd",
        "retries",
        "corrections",
        "rework_cycles",
        "context_source_count",
        "delegation_depth",
        "reason",
      ],
      `${label}.economics`,
      errors,
    );
    if (
      !["measured", "unavailable", "not_applicable"].includes(
        economics.measurement_state,
      )
    ) {
      errors.push(`${label}.economics.measurement_state is not recognized`);
    }
    if (
      !["candidate", "implemented", "accepted", "rejected"].includes(
        economics.outcome,
      )
    ) {
      errors.push(`${label}.economics.outcome is not recognized`);
    }
    if (
      nonEmptyString(expectedEconomicsOutcome) &&
      economics.outcome !== expectedEconomicsOutcome
    ) {
      errors.push(
        `${label}.economics.outcome must be ${expectedEconomicsOutcome}`,
      );
    }
    if (
      requireMeasuredEconomics &&
      economics.measurement_state !== "measured"
    ) {
      errors.push(`${label}.economics must be measured for durable delivery credit`);
    }
    for (const key of ["retries", "corrections", "rework_cycles"]) {
      if (!Number.isInteger(economics[key]) || economics[key] < 0) {
        errors.push(
          `${label}.economics.${key} must be a non-negative integer`,
        );
      }
    }
    if (isObject(economicsBudget)) {
      for (const [measuredKey, budgetKey] of Object.entries({
        retries: "max_retries",
        corrections: "max_corrections",
        rework_cycles: "max_rework_cycles",
      })) {
        if (
          typeof economics[measuredKey] === "number" &&
          typeof economicsBudget[budgetKey] === "number" &&
          economics[measuredKey] > economicsBudget[budgetKey] &&
          economics.outcome !== "rejected"
        ) {
          errors.push(
            `${label}.economics.${measuredKey} exceeds ${budgetKey} ${economicsBudget[budgetKey]}`,
          );
        }
      }
    }
    if (economics.measurement_state === "measured") {
      for (const key of [
        "planned_model",
        "actual_model",
        "reasoning_effort",
        "routing_basis",
      ]) {
        if (!nonEmptyString(economics[key])) {
          errors.push(
            `${label}.economics.${key} must be a non-empty string when measured`,
          );
        }
      }
      if (!ROUTING_BASES.has(economics.routing_basis)) {
        errors.push(`${label}.economics.routing_basis is not recognized`);
      }
      if (
        economics.routing_basis === "lowest_capable" &&
        economics.routing_justification !== undefined
      ) {
        errors.push(
          `${label}.economics.routing_justification is not allowed for lowest_capable routing`,
        );
      }
      if (
        economics.routing_basis !== "lowest_capable" &&
        !nonEmptyString(economics.routing_justification)
      ) {
        errors.push(
          `${label}.economics.routing_justification is required for escalated routing`,
        );
      }
      for (const key of [
        "elapsed_ms",
        "input_tokens",
        "output_tokens",
        "context_source_count",
        "delegation_depth",
      ]) {
        if (!Number.isInteger(economics[key]) || economics[key] < 0) {
          errors.push(
            `${label}.economics.${key} must be a non-negative integer when measured`,
          );
        }
      }
      if (
        typeof economics.cost_usd !== "number" ||
        !Number.isFinite(economics.cost_usd) ||
        economics.cost_usd < 0
      ) {
        errors.push(
          `${label}.economics.cost_usd must be a non-negative number when measured`,
        );
      }
      if (economics.reason !== undefined) {
        errors.push(`${label}.economics.reason is not allowed when measured`);
      }
      meteringDocumentPath = validateDocumentDigest(
        economics.metering_receipt_ref,
        economics.metering_receipt_sha256,
        `${label}.economics.metering_receipt`,
        errors,
        { evidenceExists, loadEvidenceBytes, pathIsReviewable },
      );
      if (meteringDocumentPath && requireMeasuredEconomics) {
          const meteringBytes = loadEvidenceBytes(meteringDocumentPath);
          usageReceiptVerified = requireTrustedReceipt(
            verifyUsageReceipt,
            {
              kind: "usage_metering",
              candidate_sha: record.candidate_sha,
              product_increment_id: record.product_increment_id,
              lane_id: lane?.id ?? null,
              economics,
              document_path: meteringDocumentPath,
              document_bytes: meteringBytes,
            },
            `${label}.economics.metering_receipt`,
            errors,
          );
      }
      const limits = {
        ...DEFAULT_ECONOMICS_LIMITS,
        ...(economicsLimits ?? {}),
      };
      for (const key of ["context_source_count", "delegation_depth"]) {
        if (
          Number.isInteger(economics[key]) &&
          economics[key] > limits[key]
        ) {
          errors.push(
            `${label}.economics.${key} must not exceed ${limits[key]}`,
          );
        }
      }
      if (economics.measurement_state === "measured" && isObject(economicsBudget)) {
        const budgetFields = {
          elapsed_ms: "max_elapsed_ms",
          input_tokens: "max_input_tokens",
          output_tokens: "max_output_tokens",
          cost_usd: "max_cost_usd",
        };
        for (const [measuredKey, budgetKey] of Object.entries(budgetFields)) {
          if (
            typeof economics[measuredKey] === "number" &&
            typeof economicsBudget[budgetKey] === "number" &&
            economics[measuredKey] > economicsBudget[budgetKey]
          ) {
            errors.push(
              `${label}.economics.${measuredKey} exceeds ${budgetKey} ${economicsBudget[budgetKey]}`,
            );
          }
        }
        for (const key of ["planned_model", "actual_model"]) {
          if (
            nonEmptyString(economics[key]) &&
            !economicsBudget.allowed_model_tiers?.includes(economics[key])
          ) {
            errors.push(
              `${label}.economics.${key} is outside allowed_model_tiers`,
            );
          }
        }
        if (
          economics.routing_basis === "lowest_capable" &&
          economics.planned_model !== economicsBudget.default_model_tier
        ) {
          errors.push(
            `${label}.economics.planned_model must equal default_model_tier for lowest_capable routing`,
          );
        }
      }
    } else if (!nonEmptyString(economics.reason)) {
      errors.push(
        `${label}.economics.reason is required when measurement is not measured`,
      );
    }
  }

  let destinationDocumentPath = null;
  const destination = record.destination_verification;
  if (destination !== null && !isObject(destination)) {
    errors.push(`${label}.destination_verification must be null or an object`);
  } else if (isObject(destination)) {
    rejectUnknownKeys(
      destination,
      [
        "destination_branch",
        "candidate_sha",
        "verified_at",
        "evidence_ref",
        "evidence_sha256",
      ],
      `${label}.destination_verification`,
      errors,
    );
    const expectedDestinationBranch =
      lane?.destination_branch ??
      plan.integration?.destination_branch ??
      plan.integration?.branch;
    if (destination.destination_branch !== expectedDestinationBranch) {
      errors.push(
        `${label}.destination_verification.destination_branch must be ${expectedDestinationBranch}`,
      );
    }
    if (destination.candidate_sha !== record.candidate_sha) {
      errors.push(
        `${label}.destination_verification.candidate_sha must equal candidate_sha`,
      );
    }
    if (!validTimestamp(destination.verified_at)) {
      errors.push(
        `${label}.destination_verification.verified_at must be an RFC3339 timestamp`,
      );
    }
    if (requireDurableProof) {
      destinationDocumentPath = validateDocumentDigest(
        destination.evidence_ref,
        destination.evidence_sha256,
        `${label}.destination_verification.evidence`,
        errors,
        { evidenceExists, loadEvidenceBytes, pathIsReviewable },
      );
      if (destinationDocumentPath && proofDocumentPaths.has(destinationDocumentPath)) {
        errors.push(
          `${label}.destination_verification.evidence must be distinct from execution proof`,
        );
      }
      if (destinationDocumentPath) {
        destinationVerified = requireTrustedReceipt(
          verifyDestinationReceipt,
          {
            kind: "destination_verification",
            candidate_sha: record.candidate_sha,
            destination_branch: destination.destination_branch,
            document_path: destinationDocumentPath,
            document_bytes: loadEvidenceBytes(destinationDocumentPath),
            verified_at: destination.verified_at,
          },
          `${label}.destination_verification`,
          errors,
        );
      }
    }
  } else if (requireDurableProof) {
    errors.push(`${label}.destination_verification is required for durable delivery credit`);
  }

  const review = record.review;
  if (review !== null && !isObject(review)) {
    errors.push(`${label}.review must be null or an object`);
  } else if (isObject(review)) {
    rejectUnknownKeys(
      review,
      [
        "reviewer",
        "reviewed_sha",
        "disposition",
        "completed_at",
        "findings",
        "evidence_ref",
        "evidence_sha256",
      ],
      `${label}.review`,
      errors,
    );
    if (review.reviewer !== expectedReviewer) {
      errors.push(
        `${label}.review.reviewer must be ${expectedReviewer}`,
      );
    }
    if (review.reviewed_sha !== record.candidate_sha) {
      errors.push(`${label}.review.reviewed_sha must equal candidate_sha`);
    }
    if (!["go", "go_with_conditions", "no_go"].includes(review.disposition)) {
      errors.push(`${label}.review.disposition is not recognized`);
    }
    if (!validTimestamp(review.completed_at)) {
      errors.push(`${label}.review.completed_at must be an RFC3339 timestamp`);
    }
    if (!isObject(review.findings)) {
      errors.push(`${label}.review.findings must be an object`);
    } else {
      rejectUnknownKeys(
        review.findings,
        [
          "blocker",
          "critical",
          "important",
          "should_address",
          "nice_to_address",
        ],
        `${label}.review.findings`,
        errors,
      );
      for (const key of [
        "blocker",
        "critical",
        "important",
        "should_address",
        "nice_to_address",
      ]) {
        if (
          !Number.isInteger(review.findings[key]) ||
          review.findings[key] < 0
        ) {
          errors.push(
            `${label}.review.findings.${key} must be a non-negative integer`,
          );
        }
      }
    }
    if (requireDurableProof) {
      const reviewDocumentPath = validateDocumentDigest(
        review.evidence_ref,
        review.evidence_sha256,
        `${label}.review.evidence`,
        errors,
        { evidenceExists, loadEvidenceBytes, pathIsReviewable },
      );
      if (
        reviewDocumentPath &&
        (proofDocumentPaths.has(reviewDocumentPath) ||
          reviewDocumentPath === meteringDocumentPath)
      ) {
        errors.push(
          `${label}.review.evidence must be distinct from execution proof and metering receipts`,
        );
      }
      if (reviewDocumentPath) {
        reviewAuthorityVerified = requireTrustedReceipt(
          verifyAuthorityReceipt,
          {
            kind: "independent_review",
            candidate_sha: record.candidate_sha,
            expected_actor: expectedReviewer,
            actual_actor: review.reviewer,
            document_path: reviewDocumentPath,
            document_bytes: loadEvidenceBytes(reviewDocumentPath),
            decision: review.disposition,
          },
          `${label}.review.authority`,
          errors,
        );
      }
    }
  }

  const authorityDocumentPaths = new Set(proofDocumentPaths);
  if (destinationDocumentPath) {
    if (authorityDocumentPaths.has(destinationDocumentPath)) {
      errors.push(
        `${label}.destination_verification.evidence must be distinct from proof output`,
      );
    }
    authorityDocumentPaths.add(destinationDocumentPath);
  }
  if (meteringDocumentPath) {
    if (authorityDocumentPaths.has(meteringDocumentPath)) {
      errors.push(
        `${label}.economics.metering_receipt must be distinct from execution proof output`,
      );
    }
    authorityDocumentPaths.add(meteringDocumentPath);
  }
  if (isObject(review) && nonEmptyString(review.evidence_ref)) {
    authorityDocumentPaths.add(review.evidence_ref.slice("document:".length));
  }
  for (const key of ["lane_acceptance", "product_acceptance"]) {
    if (record[key] !== null && !isObject(record[key])) {
      errors.push(`${label}.${key} must be null or an object`);
    } else if (isObject(record[key])) {
      rejectUnknownKeys(
        record[key],
        [
          "owner",
          "decision",
          "accepted_sha",
          "decided_at",
          "evidence_ref",
          "evidence_sha256",
        ],
        `${label}.${key}`,
        errors,
      );
      if (!nonEmptyString(record[key].owner)) {
        errors.push(`${label}.${key}.owner must be a non-empty string`);
      }
      if (record[key].decision !== "accepted") {
        errors.push(`${label}.${key}.decision must be accepted`);
      }
      if (record[key].accepted_sha !== record.candidate_sha) {
        errors.push(`${label}.${key}.accepted_sha must equal candidate_sha`);
      }
      if (!validTimestamp(record[key].decided_at)) {
        errors.push(`${label}.${key}.decided_at must be an RFC3339 timestamp`);
      }
      if (requireDurableProof) {
        const acceptanceDocumentPath = validateDocumentDigest(
          record[key].evidence_ref,
          record[key].evidence_sha256,
          `${label}.${key}.evidence`,
          errors,
          { evidenceExists, loadEvidenceBytes, pathIsReviewable },
        );
        if (
          acceptanceDocumentPath &&
          authorityDocumentPaths.has(acceptanceDocumentPath)
        ) {
          errors.push(
            `${label}.${key}.evidence must be distinct from proof and prior authority receipts`,
          );
        }
        if (acceptanceDocumentPath) {
          authorityDocumentPaths.add(acceptanceDocumentPath);
          acceptanceAuthorityVerified[key] = requireTrustedReceipt(
            verifyAuthorityReceipt,
            {
              kind: key,
              candidate_sha: record.candidate_sha,
              expected_actor:
                key === "lane_acceptance"
                  ? lane?.owner
                  : plan.product_increment?.owner,
              actual_actor: record[key].owner,
              document_path: acceptanceDocumentPath,
              document_bytes: loadEvidenceBytes(acceptanceDocumentPath),
              decision: record[key].decision,
            },
            `${label}.${key}.authority`,
            errors,
          );
        }
      }
    }
  }

  return {
    candidate_sha: record.candidate_sha ?? null,
    proof_passed: proof?.status === "passed",
    profile: proof?.profile ?? null,
    economics_state: economics?.measurement_state ?? null,
    economics_outcome: economics?.outcome ?? null,
    economics_trusted:
      economics?.measurement_state === "measured" &&
      (!requireMeasuredEconomics || usageReceiptVerified),
    review_eligible:
      reviewIsEligible(review) &&
      (!requireDurableProof || (reviewAuthorityVerified && destinationVerified)),
    lane_owner_accepted: acceptanceIsValid(
      record.lane_acceptance,
      lane?.owner,
      record.candidate_sha,
    ) && (!requireDurableProof || acceptanceAuthorityVerified.lane_acceptance),
    product_accepted: acceptanceIsValid(
      record.product_acceptance,
      plan.product_increment?.owner,
      record.candidate_sha,
    ) && (!requireDurableProof || acceptanceAuthorityVerified.product_acceptance),
    lane_accepted:
      acceptanceIsValid(
        record.lane_acceptance,
        lane?.owner,
        record.candidate_sha,
      ) &&
      acceptanceIsValid(
        record.product_acceptance,
        plan.product_increment?.owner,
        record.candidate_sha,
      ) &&
      (!requireDurableProof ||
        (acceptanceAuthorityVerified.lane_acceptance &&
          acceptanceAuthorityVerified.product_acceptance &&
          destinationVerified)),
  };
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

function capabilityStateSatisfies(actual, required) {
  return (
    Object.hasOwn(CAPABILITY_STATE_RANK, actual) &&
    Object.hasOwn(CAPABILITY_STATE_RANK, required) &&
    CAPABILITY_STATE_RANK[actual] >= CAPABILITY_STATE_RANK[required]
  );
}

export function validateProductIncrementPlan(
  plan,
  {
    acceptedEvidenceExists = null,
    actualChangedPaths = null,
    currentDiffLaneId = null,
    evidenceExists = null,
    economicsLimits = DEFAULT_ECONOMICS_LIMITS,
    gitChangedPaths = null,
    gitEvidenceExists = null,
    gitIsAncestor = null,
    loadAcceptedEvidenceBytes = null,
    loadAcceptedEvidenceRecord = null,
    loadEvidenceBytes = null,
    loadEvidenceRecord = null,
    pathIsReviewable = null,
    verifyAuthorityReceipt = null,
    verifyDestinationReceipt = null,
    verifyUsageReceipt = null,
  } = {},
) {
  const errors = [];
  const warnings = [];

  if (!isObject(plan)) {
    return {
      ok: false,
      errors: ["plan must be an object"],
      warnings,
      summary: null,
    };
  }
  if (plan.schema !== PLAN_SCHEMA) {
    errors.push(`plan.schema must be ${PLAN_SCHEMA}`);
  }
  requireOptionalString(plan, "$schema", "plan", errors);
  rejectUnknownKeys(
    plan,
    [
      "$schema",
      "schema",
      "program_id",
      "product_increment",
      "concurrency",
      "integration",
      "lanes",
      "lane_links",
    ],
    "plan",
    errors,
  );
  requirePattern(plan, "program_id", PROGRAM_ID, "plan", errors);

  const increment = plan.product_increment;
  if (!isObject(increment)) {
    errors.push("plan.product_increment must be an object");
  } else {
    rejectUnknownKeys(
      increment,
      [
        "id",
        "outcome_id",
        "title",
        "intent",
        "owner",
        "planning_posture",
        "stage",
        "acceptance_criteria",
      ],
      "plan.product_increment",
      errors,
    );
    requirePattern(
      increment,
      "id",
      RECORD_ID,
      "plan.product_increment",
      errors,
    );
    requirePattern(
      increment,
      "outcome_id",
      RECORD_ID,
      "plan.product_increment",
      errors,
    );
    for (const key of ["title", "intent", "owner", "planning_posture", "stage"]) {
      requireString(increment, key, "plan.product_increment", errors);
    }
    requireStringArray(
      increment,
      "acceptance_criteria",
      "plan.product_increment",
      errors,
    );
    if (
      nonEmptyString(increment.planning_posture) &&
      !new Set([
        "active",
        "next",
        "awaiting_input",
        "held",
        "later",
      ]).has(increment.planning_posture)
    ) {
      errors.push("plan.product_increment.planning_posture is not recognized");
    }
    if (
      nonEmptyString(increment.stage) &&
      !new Set([
        "prototype",
        "integration_candidate",
        "release_candidate",
      ]).has(increment.stage)
    ) {
      errors.push("plan.product_increment.stage is not recognized");
    }
  }

  const concurrency = plan.concurrency;
  if (!isObject(concurrency)) {
    errors.push("plan.concurrency must be an object");
  } else {
    rejectUnknownKeys(
      concurrency,
      ["target_active_lanes", "maximum_active_lanes", "below_target_reason"],
      "plan.concurrency",
      errors,
    );
    for (const key of ["target_active_lanes", "maximum_active_lanes"]) {
      const value = concurrency[key];
      if (!Number.isInteger(value) || value < 1 || value > 4) {
        errors.push(`plan.concurrency.${key} must be an integer from 1 through 4`);
      }
    }
    if (
      Number.isInteger(concurrency.target_active_lanes) &&
      Number.isInteger(concurrency.maximum_active_lanes) &&
      concurrency.target_active_lanes > concurrency.maximum_active_lanes
    ) {
      errors.push(
        "plan.concurrency.target_active_lanes must not exceed maximum_active_lanes",
      );
    }
    requireOptionalString(
      concurrency,
      "below_target_reason",
      "plan.concurrency",
      errors,
    );
  }

  const integration = plan.integration;
  if (!isObject(integration)) {
    errors.push("plan.integration must be an object");
  } else {
    rejectUnknownKeys(
      integration,
      [
        "strategy",
        "branch",
        "destination_branch",
        "owner",
        "independent_reviewer",
        "base_ref",
        "write_roots",
        "economics_budget",
        "leaf_ci_profile",
        "integration_ci_profile",
        "release_ci_profile",
        "merge_order",
        "integration_checks",
        "shared_path_owner",
        "evidence_record_ref",
      ],
      "plan.integration",
      errors,
    );
    for (const key of [
      "strategy",
      "branch",
      "destination_branch",
      "owner",
      "independent_reviewer",
      "base_ref",
      "leaf_ci_profile",
      "integration_ci_profile",
      "release_ci_profile",
      "shared_path_owner",
    ]) {
      requireString(integration, key, "plan.integration", errors);
    }
    requireStringArray(
      integration,
      "write_roots",
      "plan.integration",
      errors,
      { allowEmpty: true },
    );
    for (const [rootIndex, writeRoot] of (
      Array.isArray(integration.write_roots) ? integration.write_roots : []
    ).entries()) {
      if (!isSafeRepositoryPath(writeRoot, { allowTrailingGlob: true })) {
        errors.push(
          `plan.integration.write_roots[${rootIndex}] must be an exact repository-relative path or a repository-relative prefix ending in /**`,
        );
      } else if (
        typeof pathIsReviewable === "function" &&
        !pathIsReviewable(writeRoot, { allowTrailingGlob: true })
      ) {
        errors.push(
          `plan.integration.write_roots[${rootIndex}] is ignored, reserved, or traverses a symlink`,
        );
      }
    }
    validateEconomicsBudget(
      integration.economics_budget,
      "plan.integration.economics_budget",
      errors,
    );
    requireStringArray(integration, "merge_order", "plan.integration", errors);
    requireStringArray(
      integration,
      "integration_checks",
      "plan.integration",
      errors,
    );
    requireOptionalString(
      integration,
      "evidence_record_ref",
      "plan.integration",
      errors,
    );
    if (
      nonEmptyString(integration.owner) &&
      integration.owner === integration.independent_reviewer
    ) {
      errors.push(
        "plan.integration owner and independent reviewer must differ",
      );
    }
    if (
      nonEmptyString(integration.strategy) &&
      !["integration_branch", "direct_main"].includes(integration.strategy)
    ) {
      errors.push("plan.integration.strategy is not recognized");
    }
    if (
      integration.strategy === "integration_branch" &&
      nonEmptyString(integration.branch) &&
      !INTEGRATION_BRANCH.test(integration.branch)
    ) {
      errors.push(
        "plan.integration.branch must start with integrate/ for integration_branch",
      );
    }
    if (
      integration.strategy === "direct_main" &&
      integration.branch !== "main"
    ) {
      errors.push("plan.integration.branch must be main for direct_main");
    }
    if (
      nonEmptyString(integration.destination_branch) &&
      integration.destination_branch !== "main"
    ) {
      errors.push("plan.integration.destination_branch must be main");
    }
    if (
      nonEmptyString(integration.leaf_ci_profile) &&
      !["accelerated", "full"].includes(integration.leaf_ci_profile)
    ) {
      errors.push(
        "plan.integration.leaf_ci_profile must be accelerated or full",
      );
    }
    if (
      integration.strategy === "direct_main" &&
      integration.leaf_ci_profile !== "full"
    ) {
      errors.push(
        "plan.integration.leaf_ci_profile must be full for direct_main",
      );
    }
    if (
      nonEmptyString(integration.integration_ci_profile) &&
      integration.integration_ci_profile !== "full"
    ) {
      errors.push("plan.integration.integration_ci_profile must be full");
    }
    if (
      nonEmptyString(integration.release_ci_profile) &&
      integration.release_ci_profile !== "strict"
    ) {
      errors.push("plan.integration.release_ci_profile must be strict");
    }
    if (
      nonEmptyString(integration.base_ref) &&
      !FULL_SHA.test(integration.base_ref)
    ) {
      errors.push(
        "plan.integration.base_ref must be an immutable full lowercase Git SHA",
      );
    }
  }

  const lanes = plan.lanes;
  if (!Array.isArray(lanes)) {
    errors.push("plan.lanes must be an array");
  } else if (lanes.length < 1 || lanes.length > 4) {
    errors.push("plan.lanes must contain between 1 and 4 Delivery Lanes");
  } else if (
    lanes.length > 1 &&
    integration?.strategy !== "integration_branch"
  ) {
    errors.push(
      "a Product Increment with multiple Delivery Lanes must use an integration branch",
    );
  }

  const laneById = new Map();
  const capabilityById = new Map();
  const runnableLanes = [];
  const activeLanes = [];
  const implementedLanes = [];
  const acceptedLanes = [];
  const laneEvidenceSummaries = new Map();

  for (const [index, lane] of (Array.isArray(lanes) ? lanes : []).entries()) {
    const label = `plan.lanes[${index}]`;
    if (!isObject(lane)) {
      errors.push(`${label} must be an object`);
      continue;
    }
    rejectUnknownKeys(
      lane,
      [
        "id",
        "title",
        "result",
        "owner",
        "technical_lead",
        "branch_owner",
        "independent_reviewer",
        "status",
        "delivery_credit",
        "workstreams",
        "deliverables",
        "branch",
        "base_sha",
        "destination_branch",
        "write_roots",
        "economics_budget",
        "focused_checks",
        "evidence_record_ref",
        "provides",
      ],
      label,
      errors,
    );
    requirePattern(lane, "id", RECORD_ID, label, errors);
    for (const key of [
      "title",
      "result",
      "owner",
      "technical_lead",
      "branch_owner",
      "independent_reviewer",
      "status",
      "delivery_credit",
      "branch",
      "base_sha",
      "destination_branch",
    ]) {
      requireString(lane, key, label, errors);
    }
    for (const key of [
      "workstreams",
      "deliverables",
      "write_roots",
      "focused_checks",
    ]) {
      requireStringArray(lane, key, label, errors);
    }
    requireOptionalString(lane, "evidence_record_ref", label, errors);
    validateEconomicsBudget(
      lane.economics_budget,
      `${label}.economics_budget`,
      errors,
    );
    for (const [rootIndex, writeRoot] of (
      Array.isArray(lane.write_roots) ? lane.write_roots : []
    ).entries()) {
      if (!isSafeRepositoryPath(writeRoot, { allowTrailingGlob: true })) {
        errors.push(
          `${label}.write_roots[${rootIndex}] must be an exact repository-relative path or a repository-relative prefix ending in /**`,
        );
      } else if (
        typeof pathIsReviewable === "function" &&
        !pathIsReviewable(writeRoot, { allowTrailingGlob: true })
      ) {
        errors.push(
          `${label}.write_roots[${rootIndex}] is ignored, reserved, or traverses a symlink`,
        );
      }
    }
    if (!Array.isArray(lane.provides) || lane.provides.length === 0) {
      errors.push(`${label}.provides must be a non-empty array`);
    }
    if (nonEmptyString(lane.id)) {
      if (laneById.has(lane.id)) {
        errors.push(`lane id ${lane.id} is duplicated`);
      } else {
        laneById.set(lane.id, lane);
      }
    }
    if (nonEmptyString(lane.status) && !LANE_STATUSES.has(lane.status)) {
      errors.push(`${label}.status is not recognized`);
    }
    if (RUNNABLE_STATUSES.has(lane.status)) runnableLanes.push(lane);
    if (lane.status === "in_progress") activeLanes.push(lane);
    if (
      Object.hasOwn(DELIVERY_CREDIT_RANK, lane.delivery_credit) &&
      DELIVERY_CREDIT_RANK[lane.delivery_credit] >=
        DELIVERY_CREDIT_RANK.implemented
    ) {
      implementedLanes.push(lane);
    }
    if (!Object.hasOwn(DELIVERY_CREDIT_RANK, lane.delivery_credit)) {
      errors.push(`${label}.delivery_credit is not recognized`);
    }
    const allowedCredits = LANE_CREDIT_BY_STATUS[lane.status];
    if (
      allowedCredits &&
      !allowedCredits.includes(lane.delivery_credit)
    ) {
      errors.push(
        `${label} status ${lane.status} permits delivery_credit ${allowedCredits.join(" or ")}, not ${lane.delivery_credit}`,
      );
    }

    if (nonEmptyString(lane.branch) && !LEAF_BRANCH.test(lane.branch)) {
      errors.push(`${label}.branch must be a feature, fix, docs, or maintenance branch`);
    }
    if (nonEmptyString(lane.base_sha) && !FULL_SHA.test(lane.base_sha)) {
      errors.push(`${label}.base_sha must be a full lowercase Git SHA`);
    }
    if (
      nonEmptyString(lane.destination_branch) &&
      nonEmptyString(integration?.branch) &&
      lane.destination_branch !== integration.branch
    ) {
      errors.push(
        `${label}.destination_branch must equal ${integration.branch}`,
      );
    }
    if (
      nonEmptyString(lane.branch_owner) &&
      lane.branch_owner === lane.independent_reviewer
    ) {
      errors.push(`${label} branch owner and independent reviewer must differ`);
    }

    for (const [capabilityIndex, capability] of (
      Array.isArray(lane.provides) ? lane.provides : []
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
      if (
        nonEmptyString(capability.state) &&
        !Object.hasOwn(CAPABILITY_STATE_RANK, capability.state)
      ) {
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
      validateDurableEvidenceRef(
        capability.evidence_ref,
        `${capabilityLabel}.evidence_ref`,
        errors,
        { evidenceExists, gitEvidenceExists, pathIsReviewable },
      );
      if (
        Object.hasOwn(CAPABILITY_STATE_RANK, capability.state) &&
        Object.hasOwn(DELIVERY_CREDIT_RANK, lane.delivery_credit) &&
        CAPABILITY_STATE_RANK[capability.state] >
          DELIVERY_CREDIT_RANK[lane.delivery_credit]
      ) {
        errors.push(
          `${capabilityLabel}.state exceeds lane delivery_credit ${lane.delivery_credit}`,
        );
      }
      if (nonEmptyString(capability.capability_id)) {
        if (capabilityById.has(capability.capability_id)) {
          errors.push(
            `capability ${capability.capability_id} has more than one provider`,
          );
        } else {
          capabilityById.set(capability.capability_id, {
            capability,
            laneId: lane.id,
          });
        }
      }
    }
    const requiredCapabilityState = {
      none: "planned",
      implemented: "implemented",
      accepted: "accepted",
    }[lane.delivery_credit];
    if (requiredCapabilityState) {
      for (const [capabilityIndex, capability] of (
        Array.isArray(lane.provides) ? lane.provides : []
      ).entries()) {
        if (capability?.state !== requiredCapabilityState) {
          errors.push(
            `${label}.provides[${capabilityIndex}].state must be ${requiredCapabilityState} when delivery_credit is ${lane.delivery_credit}`,
          );
        }
      }
    }

    const evidenceRequired = EVIDENCE_REQUIRED_STATUSES.has(lane.status);
    const durableCreditRequired =
      DELIVERY_CREDIT_RANK[lane.delivery_credit] >=
      DELIVERY_CREDIT_RANK.implemented;
    let laneEvidence = null;
    if (evidenceRequired && !nonEmptyString(lane.evidence_record_ref)) {
      errors.push(`${label}.evidence_record_ref is required for ${lane.status}`);
    }
    if (nonEmptyString(lane.evidence_record_ref)) {
      const evidencePath = validateDurableDocumentRef(
        lane.evidence_record_ref,
        `${label}.evidence_record_ref`,
        errors,
        {
          evidenceExists: durableCreditRequired
            ? acceptedEvidenceExists
            : evidenceExists,
          pathIsReviewable,
        },
      );
      const evidenceRecordLoader = durableCreditRequired
        ? loadAcceptedEvidenceRecord
        : loadEvidenceRecord;
      if (
        durableCreditRequired &&
        typeof loadAcceptedEvidenceRecord !== "function"
      ) {
        errors.push(
          `${label}.evidence_record_ref requires an accepted-main record loader for durable credit`,
        );
      }
      if (
        durableCreditRequired &&
        typeof loadAcceptedEvidenceBytes !== "function"
      ) {
        errors.push(
          `${label}.evidence_record_ref requires an accepted-main byte loader for durable credit`,
        );
      }
      if (
        durableCreditRequired &&
        typeof acceptedEvidenceExists !== "function"
      ) {
        errors.push(
          `${label}.evidence_record_ref requires accepted-main existence verification for durable credit`,
        );
      }
      if (evidencePath && typeof evidenceRecordLoader === "function") {
        let record = null;
        try {
          record = evidenceRecordLoader(evidencePath);
        } catch (error) {
          errors.push(
            `${label}.evidence_record_ref is not readable JSON: ${error.message}`,
          );
        }
        if (record) {
          const requiredProfile =
            increment?.stage === "release_candidate"
              ? "strict"
              : increment?.stage === "integration_candidate"
                ? "full"
                : integration?.leaf_ci_profile;
          laneEvidence = validateDeliveryEvidenceRecord(
            record,
            {
              economicsBudget: lane.economics_budget,
              economicsLimits,
              evidenceExists: durableCreditRequired
                ? acceptedEvidenceExists
                : evidenceExists,
              expectedEconomicsOutcome:
                lane.delivery_credit === "none"
                  ? lane.status === "stopped"
                    ? "rejected"
                    : "candidate"
                  : lane.delivery_credit,
              expectedReviewer: lane.independent_reviewer,
              label: `${label}.evidence_record`,
              loadEvidenceBytes: durableCreditRequired
                ? loadAcceptedEvidenceBytes
                : loadEvidenceBytes,
              plan,
              lane,
              expectedScope:
                integration?.strategy === "direct_main"
                  ? "direct_main_candidate"
                  : "delivery_lane",
              requiredChecks: [
                ...(lane.focused_checks ?? []),
                ...(integration?.strategy === "direct_main"
                  ? integration.integration_checks ?? []
                  : []),
              ],
              requireDurableProof:
                DELIVERY_CREDIT_RANK[lane.delivery_credit] >=
                DELIVERY_CREDIT_RANK.implemented,
              requireMeasuredEconomics: durableCreditRequired,
              requiredProfile,
              gitChangedPaths,
              gitEvidenceExists,
              gitIsAncestor,
              pathIsReviewable,
              verifyAuthorityReceipt,
              verifyDestinationReceipt,
              verifyUsageReceipt,
            },
            errors,
          );
          laneEvidenceSummaries.set(lane.id, laneEvidence);
        }
      }
    }
    if (laneEvidence && evidenceRequired) {
      for (const [capabilityIndex, capability] of (
        Array.isArray(lane.provides) ? lane.provides : []
      ).entries()) {
        if (
          ["implemented", "accepted"].includes(capability?.state) &&
          capability.evidence_ref !== `git:${laneEvidence.candidate_sha}`
        ) {
          errors.push(
            `${label}.provides[${capabilityIndex}].evidence_ref must identify the evidence-record candidate SHA`,
          );
        }
      }
      if (
        ["implemented", "accepted"].includes(lane.status) &&
        !laneEvidence.review_eligible
      ) {
        errors.push(
          `${label} ${lane.status} requires an eligible exact-candidate independent review`,
        );
      }
      if (lane.status === "accepted" && !laneEvidence.lane_owner_accepted) {
        errors.push(
          `${label} accepted requires an acceptance receipt from ${lane.owner}`,
        );
      }
      if (lane.status === "accepted" && !laneEvidence.product_accepted) {
        errors.push(
          `${label} accepted requires an acceptance receipt from ${plan.product_increment?.owner}`,
        );
      }
      if (lane.status === "accepted" && laneEvidence.lane_accepted) {
        acceptedLanes.push(lane);
      }
    }
  }

  const workstreamDomains = new Set(
    (Array.isArray(lanes) ? lanes : []).flatMap((lane) =>
      Array.isArray(lane?.workstreams) ? lane.workstreams : [],
    ),
  );
  for (const lane of Array.isArray(lanes) ? lanes : []) {
    if (
      nonEmptyString(integration?.independent_reviewer) &&
      [
        lane?.branch_owner,
        lane?.technical_lead,
        lane?.independent_reviewer,
      ].includes(integration.independent_reviewer)
    ) {
      errors.push(
        `plan.integration independent reviewer must differ from Delivery Lane ${lane?.id} branch owner, technical lead, and reviewer`,
      );
    }
    for (const integrationRoot of integration?.write_roots ?? []) {
      for (const laneRoot of lane?.write_roots ?? []) {
        if (pathsOverlap(integrationRoot, laneRoot)) {
          errors.push(
            `plan.integration write root ${integrationRoot} overlaps Delivery Lane ${lane?.id} write root ${laneRoot}`,
          );
        }
      }
    }
  }
  if (
    integration?.strategy === "direct_main" &&
    workstreamDomains.size > 1
  ) {
    errors.push(
      "a cross-domain Product Increment must use an integration branch",
    );
  }

  let integrationEvidenceSummary = null;
  if (nonEmptyString(integration?.evidence_record_ref)) {
    const integrationRequiresDurableCredit = (lanes ?? []).some(
      (lane) =>
        DELIVERY_CREDIT_RANK[lane?.delivery_credit] >=
        DELIVERY_CREDIT_RANK.implemented,
    );
    const evidencePath = validateDurableDocumentRef(
      integration.evidence_record_ref,
      "plan.integration.evidence_record_ref",
      errors,
      {
        evidenceExists: integrationRequiresDurableCredit
          ? acceptedEvidenceExists
          : evidenceExists,
        pathIsReviewable,
      },
    );
    const integrationRecordLoader = integrationRequiresDurableCredit
      ? loadAcceptedEvidenceRecord
      : loadEvidenceRecord;
    if (
      integrationRequiresDurableCredit &&
      typeof loadAcceptedEvidenceRecord !== "function"
    ) {
      errors.push(
        "plan.integration.evidence_record_ref requires an accepted-main record loader for durable credit",
      );
    }
    if (
      integrationRequiresDurableCredit &&
      typeof loadAcceptedEvidenceBytes !== "function"
    ) {
      errors.push(
        "plan.integration.evidence_record_ref requires an accepted-main byte loader for durable credit",
      );
    }
    if (
      integrationRequiresDurableCredit &&
      typeof acceptedEvidenceExists !== "function"
    ) {
      errors.push(
        "plan.integration.evidence_record_ref requires accepted-main existence verification for durable credit",
      );
    }
    if (evidencePath && typeof integrationRecordLoader === "function") {
      let record = null;
      try {
        record = integrationRecordLoader(evidencePath);
      } catch (error) {
        errors.push(
          `plan.integration.evidence_record_ref is not readable ${integrationRequiresDurableCredit ? "from accepted main" : "JSON"}: ${error.message}`,
        );
      }
      if (record) {
        const integrationRequiresCreditProof =
          isObject(record.product_acceptance) ||
          integrationRequiresDurableCredit;
        integrationEvidenceSummary = validateDeliveryEvidenceRecord(
          record,
          {
            economicsBudget: integration.economics_budget,
            economicsLimits,
            evidenceExists: integrationRequiresCreditProof
              ? acceptedEvidenceExists
              : evidenceExists,
            expectedEconomicsOutcome: isObject(record.product_acceptance)
              ? "accepted"
              : integrationRequiresCreditProof
                ? "implemented"
                : (lanes ?? []).length > 0 &&
                    (lanes ?? []).every((lane) => lane?.status === "stopped")
                  ? "rejected"
                  : "candidate",
            expectedReviewer: integration.independent_reviewer,
            label: "plan.integration.evidence_record",
            loadEvidenceBytes: integrationRequiresCreditProof
              ? loadAcceptedEvidenceBytes
              : loadEvidenceBytes,
            plan,
            expectedScope:
              integration.strategy === "direct_main"
                ? "direct_main_candidate"
                : "product_increment_integration",
            requiredChecks: integration.integration_checks ?? [],
            requireDurableProof: integrationRequiresCreditProof,
            requireMeasuredEconomics: integrationRequiresCreditProof,
            requiredProfile:
              increment?.stage === "release_candidate" ? "strict" : "full",
            gitChangedPaths,
            gitEvidenceExists,
            gitIsAncestor,
            pathIsReviewable,
            verifyAuthorityReceipt,
            verifyDestinationReceipt,
            verifyUsageReceipt,
          },
          errors,
        );
      }
    }
  }

  const sensitiveWriteRoots = [
    ...(Array.isArray(lanes) ? lanes : [])
      .flatMap((lane) =>
        Array.isArray(lane?.write_roots) ? lane.write_roots : [],
      ),
    ...(Array.isArray(integration?.write_roots)
      ? integration.write_roots
      : []),
  ]
    .filter((root) => isSensitiveAuthorityPath(root));
  const observedChangedPaths = Array.isArray(actualChangedPaths)
    ? [...new Set(actualChangedPaths)]
    : [];
  const observedSensitivePaths = observedChangedPaths.filter((path) =>
    isSensitiveAuthorityPath(path),
  );
  if (actualChangedPaths !== null) {
    if (!nonEmptyString(currentDiffLaneId) || !laneById.has(currentDiffLaneId)) {
      errors.push(
        "current Git diff validation requires a known currentDiffLaneId",
      );
    } else {
      const currentLane = laneById.get(currentDiffLaneId);
      for (const changedPath of observedChangedPaths) {
        if (!isSafeRepositoryPath(changedPath)) {
          errors.push(
            `observed changed path must be an exact repository-relative path: ${changedPath}`,
          );
          continue;
        }
        if (
          typeof pathIsReviewable === "function" &&
          !pathIsReviewable(changedPath)
        ) {
          errors.push(
            `observed changed path is ignored, reserved, or traverses a symlink: ${changedPath}`,
          );
          continue;
        }
        if (
          !(currentLane.write_roots ?? []).some((root) =>
            pathIsAuthorizedByRoot(changedPath, root),
          )
        ) {
          errors.push(
            `observed changed path ${changedPath} is outside Delivery Lane ${currentDiffLaneId} write_roots`,
          );
        }
      }
    }
  }
  if (
    integration?.strategy === "integration_branch" &&
    increment?.stage === "prototype" &&
    integration.leaf_ci_profile !== "accelerated"
  ) {
    errors.push(
      "prototype Product Increments using an integration branch must use accelerated leaf CI; full proof belongs at integration",
    );
  }
  if (
    integration?.strategy === "integration_branch" &&
    increment?.stage === "release_candidate" &&
    integration.leaf_ci_profile !== "full"
  ) {
    errors.push(
      "release-candidate Product Increments must use full leaf CI before strict release proof",
    );
  }

  if (
    Number.isInteger(concurrency?.maximum_active_lanes) &&
    activeLanes.length > concurrency.maximum_active_lanes
  ) {
    errors.push(
      `${activeLanes.length} active lanes exceeds maximum_active_lanes ${concurrency.maximum_active_lanes}`,
    );
  }
  if (
    increment?.planning_posture === "active" &&
    Number.isInteger(concurrency?.target_active_lanes) &&
    runnableLanes.length < concurrency.target_active_lanes
  ) {
    const reason = concurrency.below_target_reason;
    if (!nonEmptyString(reason)) {
      warnings.push(
        `active Product Increment has ${runnableLanes.length} runnable lanes below target ${concurrency.target_active_lanes}; record concurrency.below_target_reason`,
      );
    }
  }

  const activeOwners = new Set();
  for (const lane of activeLanes) {
    if (activeOwners.has(lane.branch_owner)) {
      errors.push(
        `active branch owner ${lane.branch_owner} owns more than one active lane`,
      );
    }
    activeOwners.add(lane.branch_owner);
  }

  for (let leftIndex = 0; leftIndex < runnableLanes.length; leftIndex += 1) {
    for (
      let rightIndex = leftIndex + 1;
      rightIndex < runnableLanes.length;
      rightIndex += 1
    ) {
      const left = runnableLanes[leftIndex];
      const right = runnableLanes[rightIndex];
      for (const leftRoot of left.write_roots ?? []) {
        for (const rightRoot of right.write_roots ?? []) {
          if (pathsOverlap(leftRoot, rightRoot)) {
            errors.push(
              `runnable lanes ${left.id} and ${right.id} overlap write roots ${leftRoot} and ${rightRoot}`,
            );
          }
        }
      }
    }
  }

  const laneIds = [...laneById.keys()];
  const mergeOrder = Array.isArray(integration?.merge_order)
    ? integration.merge_order
    : [];
  if (
    mergeOrder.length !== laneIds.length ||
    new Set(mergeOrder).size !== laneIds.length ||
    laneIds.some((laneId) => !mergeOrder.includes(laneId))
  ) {
    errors.push("plan.integration.merge_order must contain every lane exactly once");
  }

  const links = Array.isArray(plan.lane_links) ? plan.lane_links : null;
  if (!links) {
    errors.push("plan.lane_links must be an array");
  }
  const linkKeys = new Set();
  const hardDependencyGraph = new Map(laneIds.map((laneId) => [laneId, []]));
  const incomingLinks = new Map(laneIds.map((laneId) => [laneId, []]));
  const outgoingLinks = new Map(laneIds.map((laneId) => [laneId, []]));

  for (const [index, link] of (links ?? []).entries()) {
    const label = `plan.lane_links[${index}]`;
    if (!isObject(link)) {
      errors.push(`${label} must be an object`);
      continue;
    }
    rejectUnknownKeys(
      link,
      [
        "provider_lane",
        "consumer_lane",
        "kind",
        "capability_id",
        "required_provider_state",
        "contract_ref",
        "decoupling",
        "consumer_action",
        "integration_check",
        "expected_benefit",
        "adoption_trigger",
      ],
      label,
      errors,
    );
    for (const key of [
      "provider_lane",
      "consumer_lane",
      "kind",
      "capability_id",
      "contract_ref",
      "consumer_action",
      "integration_check",
    ]) {
      requireString(link, key, label, errors);
    }
    validateDurableDocumentRef(
      link.contract_ref,
      `${label}.contract_ref`,
      errors,
      { evidenceExists, pathIsReviewable },
    );
    if (!laneById.has(link.provider_lane)) {
      errors.push(`${label}.provider_lane does not identify a lane`);
    }
    if (!laneById.has(link.consumer_lane)) {
      errors.push(`${label}.consumer_lane does not identify a lane`);
    }
    if (
      nonEmptyString(link.provider_lane) &&
      link.provider_lane === link.consumer_lane
    ) {
      errors.push(`${label} cannot link a lane to itself`);
    }
    if (!["requires", "benefits_from"].includes(link.kind)) {
      errors.push(`${label}.kind must be requires or benefits_from`);
    }

    const linkKey = [
      link.provider_lane,
      link.consumer_lane,
      link.kind,
      link.capability_id,
    ].join("|");
    if (linkKeys.has(linkKey)) {
      errors.push(`${label} duplicates an existing lane link`);
    }
    linkKeys.add(linkKey);

    const provided = capabilityById.get(link.capability_id);
    if (!provided) {
      errors.push(`${label}.capability_id has no declared provider`);
    } else if (provided.laneId !== link.provider_lane) {
      errors.push(
        `${label}.capability_id is provided by ${provided.laneId}, not ${link.provider_lane}`,
      );
    }

    if (!isObject(link.decoupling)) {
      errors.push(`${label}.decoupling must be an object`);
    } else {
      rejectUnknownKeys(
        link.decoupling,
        ["mode", "ref"],
        `${label}.decoupling`,
        errors,
      );
      if (
        !["none", "frozen_contract", "fixture", "adapter"].includes(
          link.decoupling.mode,
        )
      ) {
        errors.push(`${label}.decoupling.mode is not recognized`);
      } else if (
        link.decoupling.mode !== "none" &&
        !nonEmptyString(link.decoupling.ref)
      ) {
        errors.push(
          `${label}.decoupling.ref is required when decoupling.mode is not none`,
        );
      }
      requireOptionalString(
        link.decoupling,
        "ref",
        `${label}.decoupling`,
        errors,
      );
      if (
        link.decoupling.mode === "none" &&
        link.decoupling.ref !== undefined
      ) {
        errors.push(
          `${label}.decoupling.ref is not allowed when decoupling.mode is none`,
        );
      }
      if (
        link.decoupling.mode !== "none" &&
        nonEmptyString(link.decoupling.ref)
      ) {
        validateDurableDocumentRef(
          link.decoupling.ref,
          `${label}.decoupling.ref`,
          errors,
          { evidenceExists, pathIsReviewable },
        );
      }
    }

    if (link.kind === "requires") {
      for (const forbiddenKey of ["expected_benefit", "adoption_trigger"]) {
        if (link[forbiddenKey] !== undefined) {
          errors.push(`${label}.${forbiddenKey} is not allowed for requires`);
        }
      }
      if (!["implemented", "accepted"].includes(link.required_provider_state)) {
        errors.push(
          `${label}.required_provider_state must be implemented or accepted`,
        );
      }
      const providerState = provided?.capability?.state;
      const providerSufficient = capabilityStateSatisfies(
        providerState,
        link.required_provider_state,
      );
      const consumerRunnable = RUNNABLE_STATUSES.has(
        laneById.get(link.consumer_lane)?.status,
      );
      if (
        consumerRunnable &&
        !providerSufficient &&
        link.decoupling?.mode === "none"
      ) {
        errors.push(
          `${label} leaves runnable consumer ${link.consumer_lane} waiting on unfinished provider ${link.provider_lane}`,
        );
      }
      if (!providerSufficient && link.decoupling?.mode === "none") {
        hardDependencyGraph.get(link.consumer_lane)?.push(link.provider_lane);
      }
    } else if (link.kind === "benefits_from") {
      if (link.required_provider_state !== undefined) {
        errors.push(
          `${label}.required_provider_state is not allowed for benefits_from`,
        );
      }
      if (!nonEmptyString(link.expected_benefit)) {
        errors.push(`${label}.expected_benefit is required for benefits_from`);
      }
      if (!nonEmptyString(link.adoption_trigger)) {
        errors.push(`${label}.adoption_trigger is required for benefits_from`);
      }
    }
    requireOptionalString(link, "expected_benefit", label, errors);
    requireOptionalString(link, "adoption_trigger", label, errors);

    incomingLinks.get(link.consumer_lane)?.push(link);
    outgoingLinks.get(link.provider_lane)?.push(link);
  }

  const cycle = findCycle(hardDependencyGraph);
  if (cycle) {
    errors.push(`unresolved hard dependency cycle: ${cycle.join(" -> ")}`);
  }
  const mergePosition = new Map(
    mergeOrder.map((laneId, index) => [laneId, index]),
  );
  for (const [index, link] of (links ?? []).entries()) {
    if (link?.kind !== "requires") continue;
    const providerPosition = mergePosition.get(link.provider_lane);
    const consumerPosition = mergePosition.get(link.consumer_lane);
    if (
      Number.isInteger(providerPosition) &&
      Number.isInteger(consumerPosition) &&
      providerPosition >= consumerPosition
    ) {
      errors.push(
        `plan.integration.merge_order must place required provider ${link.provider_lane} before consumer ${link.consumer_lane} for lane_links[${index}]`,
      );
    }
  }

  const summary = {
    product_increment_id: increment?.id ?? null,
    outcome_id: increment?.outcome_id ?? null,
    planning_posture: increment?.planning_posture ?? null,
    stage: increment?.stage ?? null,
    integration_branch: integration?.branch ?? null,
    integration_strategy: integration?.strategy ?? null,
    integration_write_roots: [...(integration?.write_roots ?? [])],
    integration_economics_budget: integration?.economics_budget ?? null,
    sensitive_write_roots: sensitiveWriteRoots,
    observed_changed_paths: observedChangedPaths,
    observed_sensitive_paths: observedSensitivePaths,
    lane_count: laneById.size,
    runnable_lane_count: runnableLanes.length,
    active_lane_count: activeLanes.length,
    implemented_lane_count: implementedLanes.length,
    accepted_lane_count: acceptedLanes.length,
    target_active_lanes: concurrency?.target_active_lanes ?? null,
    maximum_active_lanes: concurrency?.maximum_active_lanes ?? null,
    required_link_count: (links ?? []).filter((link) => link?.kind === "requires")
      .length,
    benefit_link_count: (links ?? []).filter(
      (link) => link?.kind === "benefits_from",
    ).length,
    integration_evidence: integrationEvidenceSummary,
    lanes: laneIds.map((laneId) => ({
      id: laneId,
      status: laneById.get(laneId)?.status ?? null,
      delivery_credit: laneById.get(laneId)?.delivery_credit ?? null,
      economics_budget: laneById.get(laneId)?.economics_budget ?? null,
      provides: (laneById.get(laneId)?.provides ?? []).map(
        (capability) => capability.capability_id,
      ),
      required_by: (outgoingLinks.get(laneId) ?? [])
        .filter((link) => link.kind === "requires")
        .map((link) => link.consumer_lane),
      benefits: (outgoingLinks.get(laneId) ?? [])
        .filter((link) => link.kind === "benefits_from")
        .map((link) => link.consumer_lane),
      consumes: (incomingLinks.get(laneId) ?? []).map(
        (link) => link.capability_id,
      ),
      evidence: laneEvidenceSummaries.get(laneId) ?? null,
    })),
  };

  return { ok: errors.length === 0, errors, warnings, summary };
}

function loadPlan(path) {
  return JSON.parse(readFileSync(path, "utf8"));
}

function runGit(args, cwd) {
  const result = spawnSync("git", args, { cwd, encoding: "utf8" });
  if (result.status !== 0) {
    throw new Error(
      result.stderr.trim() || `git ${args.join(" ")} failed`,
    );
  }
  return result.stdout.trim();
}

function runGitAllowFailure(args, cwd) {
  return spawnSync("git", args, { cwd, encoding: "utf8" });
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

export function readCurrentDiff(plan, laneId, cwd) {
  const lane = plan.lanes?.find((item) => item.id === laneId);
  if (!lane) {
    throw new Error(`--current-diff lane is unknown: ${laneId}`);
  }
  const repositoryRoot = runGit(["rev-parse", "--show-toplevel"], cwd);
  const currentBranch = runGit(["branch", "--show-current"], repositoryRoot);
  if (currentBranch !== lane.branch) {
    throw new Error(
      `current Git branch ${currentBranch || "(detached)"} does not match Delivery Lane branch ${lane.branch}`,
    );
  }
  const baseRef = lane.base_sha;
  if (!nonEmptyString(baseRef)) {
    throw new Error("Delivery Lane base_sha is required for --current-diff");
  }
  if (!FULL_SHA.test(baseRef)) {
    throw new Error("Delivery Lane base_sha must be an immutable full Git SHA");
  }
  runGit(["cat-file", "-e", `${baseRef}^{commit}`], repositoryRoot);
  runGit(["merge-base", "--is-ancestor", baseRef, "HEAD"], repositoryRoot);
  const tracked = runGit(
    ["diff", "--name-only", "--no-renames", baseRef, "--"],
    repositoryRoot,
  )
    .split("\n")
    .filter(Boolean);
  const untracked = runGit(
    ["ls-files", "--others", "--exclude-standard"],
    repositoryRoot,
  )
    .split("\n")
    .filter(Boolean);
  return [...new Set([...tracked, ...untracked])].sort();
}

function printHuman(result, planPath) {
  const status = result.ok ? "valid" : "invalid";
  console.log(`Product Increment plan ${status}: ${planPath}`);
  if (result.summary) {
    console.log(
      `  ${result.summary.product_increment_id}: ${result.summary.lane_count} lanes, ${result.summary.active_lane_count} active, ${result.summary.implemented_lane_count} implemented, ${result.summary.accepted_lane_count} accepted`,
    );
    console.log(`  integration: ${result.summary.integration_branch}`);
  }
  for (const warning of result.warnings) console.log(`  warning: ${warning}`);
  for (const error of result.errors) console.error(`  error: ${error}`);
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

  const planPath = resolve(process.cwd(), options.plan);
  let plan;
  try {
    plan = loadPlan(planPath);
  } catch (error) {
    const result = {
      command: "check-product-increment-plan",
      schema: PLAN_SCHEMA,
      ok: false,
      plan: options.plan,
      errors: [`plan is not readable JSON: ${error.message}`],
      warnings: [],
      summary: null,
    };
    if (options.json) console.log(JSON.stringify(result, null, 2));
    else printHuman(result, options.plan);
    process.exitCode = 1;
    return;
  }

  let actualChangedPaths = null;
  if (options.currentDiffLane) {
    try {
      actualChangedPaths = readCurrentDiff(
        plan,
        options.currentDiffLane,
        dirname(planPath),
      );
    } catch (error) {
      const result = {
        command: "check-product-increment-plan",
        schema: PLAN_SCHEMA,
        ok: false,
        plan: options.plan,
        errors: [`current Git diff is not valid: ${error.message}`],
        warnings: [],
        summary: null,
      };
      if (options.json) console.log(JSON.stringify(result, null, 2));
      else printHuman(result, options.plan);
      process.exitCode = 1;
      return;
    }
  }

  const repositoryRoot = runGit(
    ["rev-parse", "--show-toplevel"],
    dirname(planPath),
  );
  const validation = validateProductIncrementPlan(plan, {
    acceptedEvidenceExists(evidencePath) {
      return (
        runGitAllowFailure(
          ["cat-file", "-e", `refs/remotes/origin/main:${evidencePath}`],
          repositoryRoot,
        ).status === 0
      );
    },
    actualChangedPaths,
    currentDiffLaneId: options.currentDiffLane,
    evidenceExists(evidencePath) {
      return existsSync(resolve(repositoryRoot, evidencePath));
    },
    gitEvidenceExists(sha) {
      if (
        runGitAllowFailure(
          ["cat-file", "-e", `${sha}^{commit}`],
          repositoryRoot,
        ).status !== 0
      ) {
        return false;
      }
      const containingRefs = runGitAllowFailure(
        [
          "for-each-ref",
          "--format=%(refname)",
          "--contains",
          sha,
          "refs/heads",
          "refs/remotes",
          "refs/tags",
        ],
        repositoryRoot,
      );
      return containingRefs.status === 0 && containingRefs.stdout.trim() !== "";
    },
    gitIsAncestor(baseSha, candidateSha) {
      return (
        runGitAllowFailure(
          ["merge-base", "--is-ancestor", baseSha, candidateSha],
          repositoryRoot,
        ).status === 0
      );
    },
    gitChangedPaths(baseSha, candidateSha) {
      return runGit(
        [
          "diff",
          "--name-only",
          "--no-renames",
          baseSha,
          candidateSha,
          "--",
        ],
        repositoryRoot,
      )
        .split("\n")
        .filter(Boolean);
    },
    loadAcceptedEvidenceBytes(evidencePath) {
      return loadAcceptedMainBytes(repositoryRoot, evidencePath);
    },
    loadAcceptedEvidenceRecord(evidencePath) {
      return JSON.parse(
        loadAcceptedMainBytes(repositoryRoot, evidencePath).toString("utf8"),
      );
    },
    loadEvidenceBytes(evidencePath) {
      return readFileSync(resolve(repositoryRoot, evidencePath));
    },
    loadEvidenceRecord(evidencePath) {
      return JSON.parse(
        readFileSync(resolve(repositoryRoot, evidencePath), "utf8"),
      );
    },
    pathIsReviewable(path, options = {}) {
      return repositoryPathIsReviewable(repositoryRoot, path, options);
    },
  });
  const result = {
    command: "check-product-increment-plan",
    schema: PLAN_SCHEMA,
    plan: options.plan,
    ...validation,
  };
  if (options.json) console.log(JSON.stringify(result, null, 2));
  else printHuman(result, options.plan);
  if (!result.ok) process.exitCode = 1;
}

const scriptPath = fileURLToPath(import.meta.url);
if (process.argv[1] && resolve(process.argv[1]) === resolve(scriptPath)) {
  main();
}
