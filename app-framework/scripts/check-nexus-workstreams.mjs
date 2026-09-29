#!/usr/bin/env node

import { execFileSync, spawnSync } from "node:child_process";
import { existsSync, readFileSync } from "node:fs";
import { dirname, isAbsolute, join, relative, resolve } from "node:path";
import { fileURLToPath } from "node:url";

import { validateProductIncrementPlan } from "./check-product-increment-plan.mjs";
import { validateProductIncrementPortfolio } from "./check-product-increment-portfolio.mjs";
import { repositoryPathIsReviewable } from "./product-increment-path-safety.mjs";

const EXPECTED_PRIORITY = [
  "WS-01",
  "WS-02",
  "WS-03",
  "WS-04",
  "WS-05",
  "WS-06",
  "WS-07",
  "WS-08",
];

const ASSIGNMENT_KEYS_V3 = new Set([
  "schema",
  "program_id",
  "workstream_id",
  "product_increment_id",
  "delivery_lane_id",
  "workstation_id",
  "workstream_lead",
  "technical_lead",
  "branch_owner",
  "independent_reviewer",
  "admitted_by",
  "integration_owner",
  "branch",
  "base_sha",
  "integration_branch",
  "activated_at",
  "review_after",
  "status",
]);
const ASSIGNMENT_KEYS_V4 = new Set([
  "schema",
  "assignment_id",
  "portfolio_id",
  "program_id",
  "product_increment_id",
  "delivery_lane_id",
  "capability_domain_ids",
  "workstation_id",
  "lane_outcome_owner",
  "technical_lead",
  "branch_owner",
  "independent_reviewer",
  "admitted_by",
  "integration_owner",
  "branch",
  "base_sha",
  "integration_branch",
  "execution_target",
  "planned_model_tier",
  "routing_basis",
  "routing_justification",
  "economics_budget",
  "delegation_depth",
  "activated_at",
  "review_after",
  "status",
]);
const ASSIGNMENT_OPTIONAL_KEYS_V4 = new Set(["routing_justification"]);

const RFC3339_DATE_TIME = /^(\d{4})-(\d{2})-(\d{2})T(?:[01]\d|2[0-3]):[0-5]\d:[0-5]\d(?:\.\d+)?(?:Z|[+-](?:[01]\d|2[0-3]):[0-5]\d)$/;
const FUTURE_CLOCK_SKEW_MS = 5 * 60 * 1000;
const UUID =
  /^[0-9a-f]{8}-[0-9a-f]{4}-[1-8][0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/;
const EXECUTION_TARGETS = new Set([
  "local_agent",
  "approved_cloud_worker",
  "human_developer",
]);
const ROUTING_BASES = new Set([
  "lowest_capable",
  "escalated_novel_synthesis",
  "escalated_independent_review",
]);
const ECONOMICS_BUDGET_KEYS = new Set([
  "default_model_tier",
  "allowed_model_tiers",
  "max_elapsed_ms",
  "max_input_tokens",
  "max_output_tokens",
  "max_cost_usd",
  "max_retries",
  "max_corrections",
  "max_rework_cycles",
]);

function usage() {
  return `Usage:
  node scripts/check-nexus-workstreams.mjs [--topology <path>] [--json]
  node scripts/check-nexus-workstreams.mjs --context <WS-ID> [--assignment <path>] --json
  node scripts/check-nexus-workstreams.mjs --assignment <path> [--live] --json

The command is read-only. --live checks the assignment against the current
branch, local Git graph, and origin/main without fetching or writing files.`;
}

function parseArgs(argv) {
  const options = {
    assignment: null,
    context: null,
    json: false,
    live: false,
    topology: "docs/specs/nexus-poc-workstream-topology.json",
  };

  for (let index = 0; index < argv.length; index += 1) {
    const arg = argv[index];
    if (arg === "--json") options.json = true;
    else if (arg === "--live") options.live = true;
    else if (arg === "--help" || arg === "-h") options.help = true;
    else if (arg === "--assignment" || arg === "--context" || arg === "--topology") {
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

  if (options.live && !options.assignment) {
    throw new Error("--live requires --assignment");
  }
  return options;
}

function git(args, cwd, allowFailure = false) {
  const result = spawnSync("git", args, { cwd, encoding: "utf8" });
  if (result.status !== 0 && !allowFailure) {
    throw new Error(result.stderr.trim() || `git ${args.join(" ")} failed`);
  }
  return result;
}

function resolveExactRef(refName, cwd) {
  const result = git(["show-ref", "--verify", "--hash", refName], cwd, true);
  return result.status === 0 ? result.stdout.trim() : null;
}

function gitEvidenceReachable(root, sha) {
  if (
    git(["cat-file", "-e", `${sha}^{commit}`], root, true).status !== 0
  ) {
    return false;
  }
  const containingRefs = git(
    [
      "for-each-ref",
      "--format=%(refname)",
      "--contains",
      sha,
      "refs/heads",
      "refs/remotes",
      "refs/tags",
    ],
    root,
    true,
  );
  return containingRefs.status === 0 && containingRefs.stdout.trim() !== "";
}

function gitChangedPaths(root, baseSha, candidateSha) {
  for (const [label, sha] of [
    ["base", baseSha],
    ["candidate", candidateSha],
  ]) {
    if (git(["cat-file", "-e", `${sha}^{commit}`], root, true).status !== 0) {
      throw new Error(`${label} SHA is not an exact Git commit object: ${sha}`);
    }
  }
  const result = git(
    ["diff", "--name-only", "--no-renames", baseSha, candidateSha, "--"],
    root,
    true,
  );
  if (result.status !== 0) {
    throw new Error(result.stderr.trim() || "git diff failed");
  }
  return result.stdout.split("\n").filter(Boolean);
}

function loadOriginMainEvidenceBytes(root, evidencePath) {
  if (!repositoryPathIsReviewable(root, evidencePath)) {
    throw new Error(`evidence path is not reviewable: ${evidencePath}`);
  }
  const originMain = resolveExactRef("refs/remotes/origin/main", root);
  if (!originMain) throw new Error("refs/remotes/origin/main is unavailable");
  const result = spawnSync("git", ["show", `${originMain}:${evidencePath}`], {
    cwd: root,
    encoding: null,
  });
  if (result.status !== 0) {
    throw new Error(
      Buffer.from(result.stderr ?? "").toString("utf8").trim() ||
        `origin/main does not retain ${evidencePath}`,
    );
  }
  return result.stdout;
}

function repositoryRoot() {
  const scriptDir = dirname(fileURLToPath(import.meta.url));
  return execFileSync("git", ["rev-parse", "--show-toplevel"], {
    cwd: scriptDir,
    encoding: "utf8",
  }).trim();
}

function loadJson(filePath, label, errors) {
  try {
    return JSON.parse(readFileSync(filePath, "utf8"));
  } catch (error) {
    errors.push(`${label} is not readable JSON: ${error.message}`);
    return null;
  }
}

function isObject(value) {
  return value !== null && typeof value === "object" && !Array.isArray(value);
}

function nonEmptyString(value) {
  return typeof value === "string" && value.trim() !== "";
}

function requireString(record, key, label, errors) {
  if (typeof record?.[key] !== "string" || record[key].trim() === "") {
    errors.push(`${label}.${key} must be a non-empty string`);
    return false;
  }
  return true;
}

function rejectUnknownKeys(record, allowedKeys, label, errors) {
  if (!isObject(record)) return;
  const allowed = new Set(allowedKeys);
  for (const key of Object.keys(record)) {
    if (!allowed.has(key)) errors.push(`${label}.${key} is not allowed`);
  }
}

function parseIsoDateTime(value) {
  if (typeof value !== "string") return Number.NaN;
  const match = RFC3339_DATE_TIME.exec(value);
  if (!match) return Number.NaN;

  const year = Number(match[1]);
  const month = Number(match[2]);
  const day = Number(match[3]);
  const leapYear = year % 4 === 0 && (year % 100 !== 0 || year % 400 === 0);
  const daysByMonth = [31, leapYear ? 29 : 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
  if (month < 1 || month > 12 || day < 1 || day > daysByMonth[month - 1]) {
    return Number.NaN;
  }

  const parsed = Date.parse(value);
  return Number.isFinite(parsed) ? parsed : Number.NaN;
}

function requireStringArray(record, key, label, errors, { allowEmpty = false } = {}) {
  const value = record?.[key];
  if (!Array.isArray(value) || (!allowEmpty && value.length === 0)) {
    errors.push(`${label}.${key} must be ${allowEmpty ? "an" : "a non-empty"} array`);
    return false;
  }
  if (value.some((item) => typeof item !== "string" || item.trim() === "")) {
    errors.push(`${label}.${key} must contain only non-empty strings`);
    return false;
  }
  if (new Set(value).size !== value.length) {
    errors.push(`${label}.${key} must not contain duplicates`);
    return false;
  }
  return true;
}

function pathBase(pattern) {
  return pattern.replace(/\/\*\*$/, "").replace(/\/$/, "");
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

function validateTopology(topology, root, errors, warnings) {
  if (!isObject(topology)) {
    errors.push("topology must be an object");
    return [];
  }
  if (topology.schema !== "appfw_multi_workstation_topology@2") {
    errors.push("topology.schema must be appfw_multi_workstation_topology@2");
  }
  if (topology.program_id !== "nexus-poc") {
    errors.push("topology.program_id must be nexus-poc");
  }
  if (JSON.stringify(topology.priority_order) !== JSON.stringify(EXPECTED_PRIORITY)) {
    errors.push(`priority_order must be ${EXPECTED_PRIORITY.join(" -> ")}`);
  }
  if (!isObject(topology.wip_policy)) errors.push("topology.wip_policy must be an object");
  if (!isObject(topology.worktree_policy)) errors.push("topology.worktree_policy must be an object");
  if (!isObject(topology.context_policy)) errors.push("topology.context_policy must be an object");
  if (!isObject(topology.authority_model)) errors.push("topology.authority_model must be an object");
  if (!isObject(topology.transition_policy)) errors.push("topology.transition_policy must be an object");
  if (topology.authority_model?.legacy_xo_status !== "superseded") {
    errors.push("authority_model.legacy_xo_status must be superseded");
  }
  for (const key of [
    "program_product_manager",
    "app_framework_chief_architect",
    "program_flow_controller",
    "integration_branch_manager",
    "workstream_lead_outcome_owner",
    "workstream_technical_lead",
    "branch_owner",
    "independent_reviewer",
    "human_sponsor",
  ]) {
    requireString(topology.authority_model, key, "topology.authority_model", errors);
  }
  if (topology.transition_policy?.fresh_control_agents_required !== true) {
    errors.push("transition_policy.fresh_control_agents_required must be true");
  }
  if (topology.transition_policy?.legacy_agents_may_dispatch !== false) {
    errors.push("transition_policy.legacy_agents_may_dispatch must be false");
  }
  requireStringArray(topology, "integration_only_paths", "topology", errors);
  requireStringArray(topology, "baseline_obligations", "topology", errors);

  const productIncrementDelivery = topology.product_increment_delivery;
  if (!isObject(productIncrementDelivery)) {
    errors.push("topology.product_increment_delivery must be an object");
  } else {
    const deliveryKeys = [
      "schema",
      "governing_model",
      "portfolio",
      "portfolio_validator",
      "workstream_semantics",
      "assignment_schema",
      "assignment_plan_selection",
      "assignment_write_scope",
      "assignment_lease_mode",
      "admission_authority",
      "target_active_lanes",
      "minimum_concurrent_capacity",
      "maximum_active_lanes",
      "integration_branch_rule",
      "leaf_destination",
      "main_destination",
      "cross_lane_links",
      "proof_boundaries",
    ];
    rejectUnknownKeys(
      productIncrementDelivery,
      deliveryKeys,
      "topology.product_increment_delivery",
      errors,
    );
    for (const key of [
      "schema",
      "governing_model",
      "portfolio",
      "portfolio_validator",
      "workstream_semantics",
      "assignment_schema",
      "assignment_plan_selection",
      "assignment_write_scope",
      "assignment_lease_mode",
      "admission_authority",
      "leaf_destination",
      "main_destination",
    ]) {
      requireString(
        productIncrementDelivery,
        key,
        "topology.product_increment_delivery",
        errors,
      );
    }
    if (productIncrementDelivery.schema !== "appfw_product_increment_plan@1") {
      errors.push(
        "product_increment_delivery.schema must be appfw_product_increment_plan@1",
      );
    }
    if (
      productIncrementDelivery.workstream_semantics !==
      "capability_suppliers_not_standing_queues"
    ) {
      errors.push(
        "product_increment_delivery.workstream_semantics must be capability_suppliers_not_standing_queues",
      );
    }
    if (
      productIncrementDelivery.assignment_schema !==
      "appfw_workstation_assignment@4"
    ) {
      errors.push(
        "product_increment_delivery.assignment_schema must be appfw_workstation_assignment@4",
      );
    }
    if (
      productIncrementDelivery.assignment_plan_selection !==
      "portfolio_by_product_increment_id"
    ) {
      errors.push(
        "product_increment_delivery.assignment_plan_selection must be portfolio_by_product_increment_id",
      );
    }
    if (
      productIncrementDelivery.assignment_write_scope !==
      "validated_product_increment_lane"
    ) {
      errors.push(
        "product_increment_delivery.assignment_write_scope must be validated_product_increment_lane",
      );
    }
    if (
      productIncrementDelivery.assignment_lease_mode !==
      "shared_live_required_for_source_authority"
    ) {
      errors.push(
        "product_increment_delivery.assignment_lease_mode must fail closed without a shared live lease",
      );
    }
    if (
      productIncrementDelivery.admission_authority !==
      "Program Flow Controller"
    ) {
      errors.push(
        "product_increment_delivery.admission_authority must be Program Flow Controller",
      );
    }
    if (
      productIncrementDelivery.integration_branch_rule !==
      "required_for_multi_lane_or_cross_domain"
    ) {
      errors.push(
        "product_increment_delivery.integration_branch_rule must require an integration branch for multi-lane or cross-domain increments",
      );
    }
    if (
      productIncrementDelivery.leaf_destination !==
        "product_increment_integration_branch" ||
      productIncrementDelivery.main_destination !==
        "product_increment_integration_pr_only"
    ) {
      errors.push(
        "product_increment_delivery destinations must preserve leaf-to-integration and integration-to-main boundaries",
      );
    }
    if (
      JSON.stringify(productIncrementDelivery.cross_lane_links) !==
      JSON.stringify(["requires", "benefits_from"])
    ) {
      errors.push(
        "product_increment_delivery.cross_lane_links must be requires, benefits_from",
      );
    }
    for (const key of [
      "target_active_lanes",
      "minimum_concurrent_capacity",
      "maximum_active_lanes",
    ]) {
      const value = productIncrementDelivery[key];
      if (!Number.isInteger(value) || value < 2 || value > 4) {
        errors.push(
          `product_increment_delivery.${key} must be an integer from 2 through 4`,
        );
      }
    }
    if (
      Number.isInteger(productIncrementDelivery.minimum_concurrent_capacity) &&
      Number.isInteger(productIncrementDelivery.target_active_lanes) &&
      productIncrementDelivery.minimum_concurrent_capacity >
        productIncrementDelivery.target_active_lanes
    ) {
      errors.push(
        "product_increment_delivery minimum capacity must not exceed target active lanes",
      );
    }
    if (
      Number.isInteger(productIncrementDelivery.target_active_lanes) &&
      Number.isInteger(productIncrementDelivery.maximum_active_lanes) &&
      productIncrementDelivery.target_active_lanes >
        productIncrementDelivery.maximum_active_lanes
    ) {
      errors.push(
        "product_increment_delivery target active lanes must not exceed maximum active lanes",
      );
    }
    const proof = productIncrementDelivery.proof_boundaries;
    if (!isObject(proof)) {
      errors.push(
        "product_increment_delivery.proof_boundaries must be an object",
      );
    } else {
      rejectUnknownKeys(
        proof,
        [
          "leaf",
          "integration_to_main",
          "release",
          "sensitive_change",
        ],
        "topology.product_increment_delivery.proof_boundaries",
        errors,
      );
      if (
        proof.leaf !== "accelerated" ||
        proof.integration_to_main !== "full" ||
        proof.release !== "strict" ||
        proof.sensitive_change !== "fail_closed_to_full_or_strict"
      ) {
        errors.push(
          "product_increment_delivery.proof_boundaries must be accelerated, full, strict, and fail_closed_to_full_or_strict",
        );
      }
    }
    for (const key of ["governing_model", "portfolio"]) {
      const value = productIncrementDelivery[key];
      if (
        typeof value === "string" &&
        (isAbsolute(value) || value.includes(".."))
      ) {
        errors.push(
          `product_increment_delivery.${key} must be a repository-relative path`,
        );
      } else if (
        typeof value === "string" &&
        value.trim() !== "" &&
        !existsSync(resolve(root, value))
      ) {
        errors.push(
          `product_increment_delivery.${key} does not exist: ${value}`,
        );
      }
    }
    const portfolioPath = resolve(root, productIncrementDelivery.portfolio);
    if (existsSync(portfolioPath)) {
      try {
        const evidenceExists = (evidencePath) =>
          existsSync(resolve(root, evidencePath));
        const gitEvidenceExists = (sha) =>
          gitEvidenceReachable(root, sha);
        const portfolio = JSON.parse(readFileSync(portfolioPath, "utf8"));
        const portfolioDirectory = dirname(portfolioPath);
        const portfolioResult = validateProductIncrementPortfolio(portfolio, {
          acceptedEvidenceExists: (evidencePath) => {
            try {
              loadOriginMainEvidenceBytes(root, evidencePath);
              return true;
            } catch {
              return false;
            }
          },
          branchExists: (branch) =>
            [`refs/heads/${branch}`, `refs/remotes/origin/${branch}`].some(
              (ref) => resolveExactRef(ref, root),
            ),
          loadPlan: (planRef) =>
            JSON.parse(
              readFileSync(resolve(portfolioDirectory, planRef), "utf8"),
            ),
          loadEvidenceRecord: (evidencePath) =>
            JSON.parse(readFileSync(resolve(root, evidencePath), "utf8")),
          evidenceExists,
          gitEvidenceExists,
          gitIsAncestor: (baseSha, candidateSha) =>
            git(
              ["merge-base", "--is-ancestor", baseSha, candidateSha],
              root,
              true,
            ).status === 0,
          gitChangedPaths: (baseSha, candidateSha) =>
            gitChangedPaths(root, baseSha, candidateSha),
          loadAcceptedEvidenceBytes: (evidencePath) =>
            loadOriginMainEvidenceBytes(root, evidencePath),
          loadAcceptedEvidenceRecord: (evidencePath) =>
            JSON.parse(
              loadOriginMainEvidenceBytes(root, evidencePath).toString("utf8"),
            ),
          loadEvidenceBytes: (evidencePath) =>
            loadOriginMainEvidenceBytes(root, evidencePath),
          pathIsReviewable: (path, options = {}) =>
            repositoryPathIsReviewable(root, path, options),
        });
        for (const error of portfolioResult.errors) {
          errors.push(`product_increment_delivery.portfolio: ${error}`);
        }
      } catch (error) {
        errors.push(
          `product_increment_delivery portfolio is unreadable: ${error.message}`,
        );
      }
    }
  }

  const producerLimit = topology.wip_policy?.initial_active_producer_limit;
  if (!Number.isInteger(producerLimit) || producerLimit < 1 || producerLimit > 4) {
    errors.push("wip_policy.initial_active_producer_limit must be an integer from 1 through 4");
  }
  if (topology.wip_policy?.integration_authority_slots !== 1) {
    errors.push("wip_policy.integration_authority_slots must be exactly 1");
  }
  if (
    topology.wip_policy?.minimum_concurrent_delivery_lanes !==
      productIncrementDelivery?.minimum_concurrent_capacity ||
    topology.wip_policy?.maximum_active_delivery_lanes !==
      productIncrementDelivery?.maximum_active_lanes ||
    producerLimit !== productIncrementDelivery?.target_active_lanes
  ) {
    errors.push(
      "Product Increment delivery capacity must match the topology WIP policy",
    );
  }
  if (topology.worktree_policy?.never_auto_delete !== true) {
    errors.push("worktree_policy.never_auto_delete must be true");
  }
  if (topology.worktree_policy?.one_active_worktree_per_assignment !== true) {
    errors.push("worktree_policy.one_active_worktree_per_assignment must be true");
  }

  const maxSources = topology.context_policy?.max_required_sources_per_workstream;
  if (!Number.isInteger(maxSources) || maxSources < 1 || maxSources > 8) {
    errors.push("context_policy.max_required_sources_per_workstream must be an integer from 1 through 8");
  }

  if (!Array.isArray(topology.workstreams) || topology.workstreams.length !== EXPECTED_PRIORITY.length) {
    errors.push(`topology.workstreams must contain exactly ${EXPECTED_PRIORITY.length} records`);
    return [];
  }

  const ids = new Set();
  const priorities = new Set();
  const writeOwners = [];
  const workstreamById = new Map();

  for (const workstream of topology.workstreams) {
    const label = `workstream[${workstream?.id ?? "unknown"}]`;
    for (const key of ["id", "slug", "name", "owns", "delivers"]) {
      requireString(workstream, key, label, errors);
    }
    for (const key of [
      "depends_on",
      "activation_gates",
      "write_roots",
      "forbidden_roots",
      "required_context",
      "publishes",
      "inner_loop",
      "handoff_checks",
    ]) {
      requireStringArray(workstream, key, label, errors, { allowEmpty: key === "depends_on" });
    }

    if (!/^WS-[0-9]{2}$/.test(workstream.id ?? "")) {
      errors.push(`${label}.id must match WS-00`);
    }
    if (ids.has(workstream.id)) errors.push(`duplicate workstream id: ${workstream.id}`);
    ids.add(workstream.id);
    workstreamById.set(workstream.id, workstream);

    if (!Number.isInteger(workstream.priority) || workstream.priority < 1) {
      errors.push(`${label}.priority must be a positive integer`);
    }
    if (priorities.has(workstream.priority)) {
      errors.push(`duplicate workstream priority: ${workstream.priority}`);
    }
    priorities.add(workstream.priority);

    if (Array.isArray(workstream.required_context)) {
      if (workstream.required_context.length > maxSources) {
        errors.push(`${label}.required_context exceeds the ${maxSources}-source context budget`);
      }
      for (const contextPath of workstream.required_context) {
        if (isAbsolute(contextPath) || contextPath.includes("..")) {
          errors.push(`${label}.required_context must use repository-relative paths: ${contextPath}`);
        } else if (!existsSync(join(root, contextPath))) {
          errors.push(`${label}.required_context does not exist: ${contextPath}`);
        }
      }
    }

    for (const writeRoot of workstream.write_roots ?? []) {
      if (isAbsolute(writeRoot) || writeRoot.includes("..") || writeRoot === "**") {
        errors.push(`${label}.write_roots contains an unsafe path: ${writeRoot}`);
      }
      writeOwners.push({ id: workstream.id, path: writeRoot });
    }
  }

  for (let index = 0; index < EXPECTED_PRIORITY.length; index += 1) {
    const workstream = workstreamById.get(EXPECTED_PRIORITY[index]);
    if (workstream?.priority !== index + 1) {
      errors.push(`${EXPECTED_PRIORITY[index]} must have priority ${index + 1}`);
    }
  }

  for (const workstream of topology.workstreams) {
    for (const dependency of workstream.depends_on) {
      const dependencyRecord = workstreamById.get(dependency);
      if (!dependencyRecord) {
        errors.push(`${workstream.id} depends on unknown workstream ${dependency}`);
      } else if (dependencyRecord.priority >= workstream.priority) {
        errors.push(`${workstream.id} dependency ${dependency} must precede it in priority order`);
      }
    }
  }

  const visiting = new Set();
  const visited = new Set();
  function visit(id) {
    if (visiting.has(id)) {
      errors.push(`workstream dependency cycle includes ${id}`);
      return;
    }
    if (visited.has(id)) return;
    visiting.add(id);
    for (const dependency of workstreamById.get(id)?.depends_on ?? []) visit(dependency);
    visiting.delete(id);
    visited.add(id);
  }
  for (const id of workstreamById.keys()) visit(id);

  for (let left = 0; left < writeOwners.length; left += 1) {
    for (let right = left + 1; right < writeOwners.length; right += 1) {
      const first = writeOwners[left];
      const second = writeOwners[right];
      if (first.id !== second.id && pathsOverlap(first.path, second.path)) {
        errors.push(`write-root overlap: ${first.id}:${first.path} and ${second.id}:${second.path}`);
      }
    }
  }

  for (const integrationPath of topology.integration_only_paths ?? []) {
    for (const owner of writeOwners) {
      if (pathsOverlap(integrationPath, owner.path)) {
        errors.push(`integration-only path ${integrationPath} overlaps ${owner.id}:${owner.path}`);
      }
    }
  }

  const initialEligible = topology.priority_order
    .map((id) => workstreamById.get(id))
    .filter((workstream) => workstream?.depends_on.length === 0)
    .slice(0, producerLimit)
    .map((workstream) => workstream.id);
  if (JSON.stringify(initialEligible) !== JSON.stringify(["WS-01", "WS-02", "WS-03"])) {
    errors.push("initial eligible producer set must be WS-01, WS-02, WS-03");
  }

  if (workstreamById.get("WS-02")?.write_roots.some((item) => item.startsWith("appfw_ui/"))) {
    errors.push("WS-02 must not own UI source");
  }
  if (!workstreamById.get("WS-07")?.activation_gates.some((item) => item.includes("source alias"))) {
    errors.push("WS-07 must explicitly prohibit PDS package source aliases");
  }
  if (!topology.baseline_obligations.some((item) => item.includes("OpenTelemetry"))) {
    warnings.push("baseline obligations should name OpenTelemetry explicitly");
  }

  return topology.workstreams;
}

function validateAssignmentEconomics(assignment, lane, errors) {
  if (!EXECUTION_TARGETS.has(assignment.execution_target)) {
    errors.push("assignment.execution_target is not recognized");
  }
  if (!ROUTING_BASES.has(assignment.routing_basis)) {
    errors.push("assignment.routing_basis is not recognized");
  }
  if (!nonEmptyString(assignment.planned_model_tier)) {
    errors.push("assignment.planned_model_tier must be a non-empty string");
  }
  if (
    assignment.routing_basis === "lowest_capable" &&
    assignment.routing_justification !== undefined
  ) {
    errors.push(
      "assignment.routing_justification is not allowed for lowest_capable routing",
    );
  }
  if (
    assignment.routing_basis !== "lowest_capable" &&
    !nonEmptyString(assignment.routing_justification)
  ) {
    errors.push(
      "assignment.routing_justification is required for escalated routing",
    );
  }
  if (
    !Number.isInteger(assignment.delegation_depth) ||
    assignment.delegation_depth < 0 ||
    assignment.delegation_depth > 1
  ) {
    errors.push("assignment.delegation_depth must be 0 or 1");
  }

  const budget = assignment.economics_budget;
  const laneBudget = lane?.economics_budget;
  if (!isObject(budget)) {
    errors.push("assignment.economics_budget must be an object");
    return;
  }
  for (const key of ECONOMICS_BUDGET_KEYS) {
    if (!(key in budget)) {
      errors.push(`assignment.economics_budget.${key} is required`);
    }
  }
  for (const key of Object.keys(budget)) {
    if (!ECONOMICS_BUDGET_KEYS.has(key)) {
      errors.push(`assignment.economics_budget contains unsupported field: ${key}`);
    }
  }
  if (
    !Array.isArray(budget.allowed_model_tiers) ||
    budget.allowed_model_tiers.length === 0 ||
    budget.allowed_model_tiers.some((tier) => !nonEmptyString(tier)) ||
    new Set(budget.allowed_model_tiers).size !==
      budget.allowed_model_tiers.length
  ) {
    errors.push(
      "assignment.economics_budget.allowed_model_tiers must be a non-empty unique string array",
    );
  }
  if (
    !nonEmptyString(budget.default_model_tier) ||
    !budget.allowed_model_tiers?.includes(budget.default_model_tier)
  ) {
    errors.push(
      "assignment.economics_budget.default_model_tier must be in allowed_model_tiers",
    );
  }
  if (!budget.allowed_model_tiers?.includes(assignment.planned_model_tier)) {
    errors.push(
      "assignment.planned_model_tier must be in economics_budget.allowed_model_tiers",
    );
  }
  if (
    assignment.routing_basis === "lowest_capable" &&
    assignment.planned_model_tier !== budget.default_model_tier
  ) {
    errors.push(
      "assignment.planned_model_tier must equal economics_budget.default_model_tier for lowest_capable routing",
    );
  }

  for (const key of [
    "max_elapsed_ms",
    "max_input_tokens",
    "max_output_tokens",
  ]) {
    if (!Number.isInteger(budget[key]) || budget[key] <= 0) {
      errors.push(`assignment.economics_budget.${key} must be a positive integer`);
    }
  }
  if (
    typeof budget.max_cost_usd !== "number" ||
    !Number.isFinite(budget.max_cost_usd) ||
    budget.max_cost_usd <= 0
  ) {
    errors.push(
      "assignment.economics_budget.max_cost_usd must be a positive number",
    );
  }
  for (const key of [
    "max_retries",
    "max_corrections",
    "max_rework_cycles",
  ]) {
    if (!Number.isInteger(budget[key]) || budget[key] < 0) {
      errors.push(
        `assignment.economics_budget.${key} must be a non-negative integer`,
      );
    }
  }

  if (!isObject(laneBudget)) {
    errors.push(`Delivery Lane ${lane?.id ?? "unknown"} has no economics_budget`);
    return;
  }
  if (budget.default_model_tier !== laneBudget.default_model_tier) {
    errors.push(
      "assignment.economics_budget.default_model_tier must equal Delivery Lane default_model_tier",
    );
  }
  if (
    assignment.routing_basis === "lowest_capable" &&
    assignment.planned_model_tier !== laneBudget.default_model_tier
  ) {
    errors.push(
      "assignment.planned_model_tier must equal Delivery Lane default_model_tier for lowest_capable routing",
    );
  }
  for (const tier of budget.allowed_model_tiers ?? []) {
    if (!laneBudget.allowed_model_tiers?.includes(tier)) {
      errors.push(
        `assignment economics model tier ${tier} exceeds Delivery Lane allowance`,
      );
    }
  }
  const boundedFields = [
    "max_elapsed_ms",
    "max_input_tokens",
    "max_output_tokens",
    "max_cost_usd",
    "max_retries",
    "max_corrections",
    "max_rework_cycles",
  ];
  for (const key of boundedFields) {
    if (
      typeof budget[key] === "number" &&
      typeof laneBudget[key] === "number" &&
      budget[key] > laneBudget[key]
    ) {
      errors.push(
        `assignment.economics_budget.${key} exceeds Delivery Lane budget ${laneBudget[key]}`,
      );
    }
  }
}

function validateAssignment(assignment, topology, root, errors, warnings, live, liveValidation) {
  if (!isObject(assignment)) {
    errors.push("assignment must be an object");
    return null;
  }
  let deliveryLane = null;
  let sourceAdmissionReady = false;
  let integrationStrategy = null;
  const isV4 = assignment.schema === "appfw_workstation_assignment@4";
  const isV3 = assignment.schema === "appfw_workstation_assignment@3";
  const assignmentKeys = isV4 ? ASSIGNMENT_KEYS_V4 : ASSIGNMENT_KEYS_V3;
  for (const key of assignmentKeys) {
    if (
      !(key in assignment) &&
      !(isV4 && ASSIGNMENT_OPTIONAL_KEYS_V4.has(key))
    ) {
      errors.push(`assignment.${key} is required`);
    }
  }
  for (const key of Object.keys(assignment)) {
    if (!assignmentKeys.has(key)) errors.push(`assignment contains unsupported field: ${key}`);
  }
  if (!isV4 && !isV3) {
    errors.push(
      "assignment.schema must be appfw_workstation_assignment@4; @3 is read-only compatibility",
    );
  }
  if (isV3) {
    warnings.push(
      "assignment schema @3 is legacy context-only evidence and cannot become source-authorized",
    );
  }
  if (isV4 && !UUID.test(assignment.assignment_id ?? "")) {
    errors.push("assignment.assignment_id must be a canonical lowercase UUID");
  }
  const capabilityDomainIds = isV4
    ? assignment.capability_domain_ids
    : [assignment.workstream_id];
  if (
    !Array.isArray(capabilityDomainIds) ||
    capabilityDomainIds.length === 0 ||
    capabilityDomainIds.some((item) => !nonEmptyString(item)) ||
    new Set(capabilityDomainIds).size !== capabilityDomainIds.length
  ) {
    errors.push(
      "assignment.capability_domain_ids must be a non-empty unique string array",
    );
  }
  const workstream = topology?.workstreams?.find((item) =>
    capabilityDomainIds?.includes(item.id),
  ) ?? null;
  if (isV3 && !workstream) {
    errors.push(`assignment.workstream_id is unknown: ${assignment.workstream_id}`);
  }
  if (isV4 || isV3) {
    requireString(assignment, "product_increment_id", "assignment", errors);
    requireString(assignment, "delivery_lane_id", "assignment", errors);
    const portfolioPath = topology?.product_increment_delivery?.portfolio;
    if (
      typeof portfolioPath !== "string" ||
      portfolioPath.trim() === ""
    ) {
      errors.push("topology does not identify the Product Increment portfolio");
    } else {
      try {
        const evidenceExists = (evidencePath) =>
          existsSync(resolve(root, evidencePath));
        const gitEvidenceExists = (sha) =>
          gitEvidenceReachable(root, sha);
        const resolvedPortfolioPath = resolve(root, portfolioPath);
        const portfolio = JSON.parse(
          readFileSync(resolvedPortfolioPath, "utf8"),
        );
        const portfolioDirectory = dirname(resolvedPortfolioPath);
        const portfolioResult = validateProductIncrementPortfolio(portfolio, {
          acceptedEvidenceExists: (evidencePath) => {
            try {
              loadOriginMainEvidenceBytes(root, evidencePath);
              return true;
            } catch {
              return false;
            }
          },
          branchExists: (branch) =>
            [`refs/heads/${branch}`, `refs/remotes/origin/${branch}`].some(
              (ref) => resolveExactRef(ref, root),
            ),
          loadPlan: (planRef) =>
            JSON.parse(
              readFileSync(resolve(portfolioDirectory, planRef), "utf8"),
            ),
          loadEvidenceRecord: (evidencePath) =>
            JSON.parse(readFileSync(resolve(root, evidencePath), "utf8")),
          evidenceExists,
          gitEvidenceExists,
          gitIsAncestor: (baseSha, candidateSha) =>
            git(
              ["merge-base", "--is-ancestor", baseSha, candidateSha],
              root,
              true,
            ).status === 0,
          gitChangedPaths: (baseSha, candidateSha) =>
            gitChangedPaths(root, baseSha, candidateSha),
          loadAcceptedEvidenceBytes: (evidencePath) =>
            loadOriginMainEvidenceBytes(root, evidencePath),
          loadAcceptedEvidenceRecord: (evidencePath) =>
            JSON.parse(
              loadOriginMainEvidenceBytes(root, evidencePath).toString("utf8"),
            ),
          loadEvidenceBytes: (evidencePath) =>
            loadOriginMainEvidenceBytes(root, evidencePath),
          pathIsReviewable: (path, options = {}) =>
            repositoryPathIsReviewable(root, path, options),
          strictFreshness: assignment.status === "active",
        });
        for (const error of portfolioResult.errors) {
          errors.push(`Product Increment portfolio: ${error}`);
        }
        if (
          isV4 &&
          assignment.portfolio_id !== portfolio.portfolio_id
        ) {
          errors.push(
            `assignment.portfolio_id must be ${portfolio.portfolio_id}`,
          );
        }
        const portfolioIncrement = portfolio.product_increments?.find(
          (item) => item.id === assignment.product_increment_id,
        );
        const selectedPlanSummary =
          portfolioResult.summary?.increments?.find(
            (item) => item.id === assignment.product_increment_id,
          )?.plan ?? null;
        if (!portfolioIncrement) {
          errors.push(
            `assignment Product Increment ${assignment.product_increment_id} is not registered in the portfolio`,
          );
        }
        if (!nonEmptyString(portfolioIncrement?.plan_ref)) {
          errors.push(
            `assignment Product Increment ${assignment.product_increment_id} has no executable plan_ref`,
          );
        }
        const plan = nonEmptyString(portfolioIncrement?.plan_ref)
          ? JSON.parse(
              readFileSync(
                resolve(portfolioDirectory, portfolioIncrement.plan_ref),
                "utf8",
              ),
            )
          : null;
        const planResult = plan
          ? {
              ok: portfolioResult.ok && selectedPlanSummary !== null,
              errors: [],
              warnings: [],
              summary: selectedPlanSummary,
            }
          : null;
        if (plan && assignment.program_id !== plan.program_id) {
          errors.push(`assignment.program_id must be ${plan.program_id}`);
        }
        if (
          plan &&
          assignment.product_increment_id !== plan.product_increment?.id
        ) {
          errors.push(
            `assignment.product_increment_id must be ${plan.product_increment?.id}`,
          );
        }
        const lane = plan?.lanes?.find(
          (item) => item.id === assignment.delivery_lane_id,
        );
        if (!lane) {
          errors.push(
            `assignment.delivery_lane_id is unknown: ${assignment.delivery_lane_id}`,
          );
        } else {
          deliveryLane = lane;
          integrationStrategy = plan.integration?.strategy ?? null;
          if (isV4) {
            validateAssignmentEconomics(assignment, lane, errors);
          }
          for (const domainId of capabilityDomainIds ?? []) {
            if (!lane.workstreams?.includes(domainId)) {
              errors.push(
                `assignment capability domain ${domainId} does not supply lane ${lane.id}`,
              );
            }
          }
          if (assignment.branch !== lane.branch) {
            errors.push(`assignment.branch must be ${lane.branch}`);
          }
          if (assignment.integration_branch !== plan.integration?.branch) {
            errors.push(
              `assignment.integration_branch must be ${plan.integration?.branch}`,
            );
          }
          if (assignment.base_sha !== lane.base_sha) {
            errors.push(`assignment.base_sha must equal lane base_sha ${lane.base_sha}`);
          }
          const exactIdentityBindings = [
            [isV4 ? "lane_outcome_owner" : "workstream_lead", lane.owner],
            ["technical_lead", lane.technical_lead],
            ["branch_owner", lane.branch_owner],
            ["independent_reviewer", lane.independent_reviewer],
            ["integration_owner", plan.integration?.owner],
            [
              "admitted_by",
              topology.product_increment_delivery?.admission_authority,
            ],
          ];
          for (const [key, expected] of exactIdentityBindings) {
            if (assignment[key] !== expected) {
              errors.push(`assignment.${key} must be ${expected}`);
            }
          }
          if (
            assignment.status === "active" &&
            portfolioIncrement?.lifecycle_state !== "active"
          ) {
            errors.push(
              `assignment Product Increment ${assignment.product_increment_id} is ${portfolioIncrement?.lifecycle_state ?? "missing"}, not source-admissible active`,
            );
          }
          if (
            assignment.status === "active" &&
            lane.status !== "in_progress"
          ) {
            errors.push(
              `active assignment requires lane ${lane.id} to be in_progress`,
            );
          }
          sourceAdmissionReady =
            isV4 &&
            live &&
            assignment.status === "active" &&
            portfolioIncrement?.lifecycle_state === "active" &&
            lane.status === "in_progress" &&
            planResult?.ok === true &&
            portfolioResult?.ok === true;
          for (const laneRoot of lane.write_roots ?? []) {
            for (const integrationPath of topology.integration_only_paths ?? []) {
              if (pathsOverlap(laneRoot, integrationPath)) {
                errors.push(
                  `delivery lane ${lane.id} write root ${laneRoot} overlaps integration-only path ${integrationPath}`,
                );
              }
            }
          }
        }
      } catch (error) {
        errors.push(
          `selected Product Increment plan is unreadable: ${error.message}`,
        );
      }
    }
  }
  if (!/^[a-z0-9][a-z0-9-]{2,63}$/.test(assignment.workstation_id ?? "")) {
    errors.push("assignment.workstation_id must be a lowercase purpose identifier");
  }
  for (const key of [
    isV4 ? "lane_outcome_owner" : "workstream_lead",
    "technical_lead",
    "branch_owner",
    "independent_reviewer",
    "admitted_by",
    "integration_owner",
  ]) {
    requireString(assignment, key, "assignment", errors);
  }
  if (assignment.branch_owner === assignment.independent_reviewer) {
    errors.push("assignment.independent_reviewer must differ from branch_owner");
  }
  if (!/^(feature|fix|docs|maintenance)\/[a-z0-9][a-z0-9._/-]*$/.test(assignment.branch ?? "")) {
    errors.push("assignment.branch must use a purpose-based producer prefix");
  }
  if (!/^[0-9a-f]{40}$/.test(assignment.base_sha ?? "")) {
    errors.push("assignment.base_sha must be a full lowercase Git SHA");
  }
  const integrationBranch = assignment.integration_branch ?? "";
  const integrationRef = `refs/heads/${integrationBranch}`;
  if (integrationStrategy === "direct_main") {
    if (integrationBranch !== "main") {
      errors.push("assignment.integration_branch must be main for direct_main");
    }
  } else if (
    !/^integrate\/[a-z0-9][a-z0-9._/-]*$/.test(integrationBranch) ||
    git(["check-ref-format", integrationRef], root, true).status !== 0
  ) {
    errors.push("assignment.integration_branch must start with integrate/");
  }
  if (!["active", "blocked", "handoff"].includes(assignment.status)) {
    errors.push("assignment.status must be active, blocked, or handoff");
  }

  const activatedAt = parseIsoDateTime(assignment.activated_at);
  const reviewAfter = parseIsoDateTime(assignment.review_after);
  if (!Number.isFinite(activatedAt)) errors.push("assignment.activated_at must be an ISO date-time");
  if (!Number.isFinite(reviewAfter)) errors.push("assignment.review_after must be an ISO date-time");
  if (Number.isFinite(activatedAt) && Number.isFinite(reviewAfter)) {
    if (reviewAfter <= activatedAt) errors.push("assignment.review_after must be after activated_at");
    const leaseHours = (reviewAfter - activatedAt) / 3_600_000;
    if (leaseHours > 168) errors.push("assignment review window must not exceed seven days");
  }
  if (assignment.status === "active") {
    const now = Date.now();
    if (Number.isFinite(activatedAt) && activatedAt > now + FUTURE_CLOCK_SKEW_MS) {
      errors.push("active assignment activated_at is in the future");
    }
    if (Number.isFinite(reviewAfter) && reviewAfter <= now) {
      errors.push("active assignment review window has expired");
    }
    if (!live) {
      warnings.push(
        "active assignment validated without --live grants no source authority",
      );
    }
  }

  if (live && /^[0-9a-f]{40}$/.test(assignment.base_sha ?? "")) {
    const currentBranch = git(["branch", "--show-current"], root).stdout.trim();
    if (currentBranch !== assignment.branch) {
      errors.push(`current branch ${currentBranch || "DETACHED"} does not match assignment ${assignment.branch}`);
    }
    if (git(["cat-file", "-e", `${assignment.base_sha}^{commit}`], root, true).status !== 0) {
      errors.push(`assignment base SHA is not available locally: ${assignment.base_sha}`);
    } else {
      if (git(["merge-base", "--is-ancestor", assignment.base_sha, "HEAD"], root, true).status !== 0) {
        errors.push("assignment base SHA is not an ancestor of HEAD");
      }
      if (integrationStrategy === "direct_main") {
        const acceptedMain = git(
          ["merge-base", "--is-ancestor", assignment.base_sha, "origin/main"],
          root,
          true,
        ).status === 0;
        if (!acceptedMain) {
          errors.push(
            "direct-main assignment base is not accepted by origin/main",
          );
        } else {
          liveValidation.route = "accepted_main";
        }
      } else if (integrationStrategy === "integration_branch") {
        const localRef = `refs/heads/${integrationBranch}`;
        const remoteRef = `refs/remotes/origin/${integrationBranch}`;
        const localTip = resolveExactRef(localRef, root);
        const remoteTip = resolveExactRef(remoteRef, root);
        let integrationRouteAccepted = false;
        liveValidation.local_integration_tip = localTip;
        liveValidation.remote_integration_tip = remoteTip;

        if (!localTip) {
          errors.push(`named integration ref is missing: ${integrationBranch}`);
        } else if (localTip && remoteTip && localTip !== remoteTip) {
          errors.push(
            `local/remote integration ref drift for ${integrationBranch}: ${localTip} != ${remoteTip}`,
          );
        } else {
          if (localTip !== assignment.base_sha) {
            errors.push(
              `assignment base SHA does not equal named integration tip: ${assignment.base_sha} != ${localTip}`,
            );
          } else {
            const mergeBaseResult = git(
              ["merge-base", "--all", assignment.base_sha, "origin/main"],
              root,
              true,
            );
            const mergeBases = mergeBaseResult.stdout
              .trim()
              .split("\n")
              .filter((line) => /^[0-9a-f]{40}$/.test(line));
            if (mergeBases.length === 0) {
              errors.push("named integration tip is not based on accepted origin/main");
            } else if (mergeBases.length !== 1) {
              errors.push(
                `named integration lineage has ambiguous accepted-main merge bases: ${mergeBases.length}`,
              );
            } else {
              const acceptedMainMergeBase = mergeBases[0];
              liveValidation.accepted_main_merge_base = acceptedMainMergeBase;
              const mergeBaseIsValid =
                git(
                  ["merge-base", "--is-ancestor", acceptedMainMergeBase, assignment.base_sha],
                  root,
                  true,
                ).status === 0 &&
                git(
                  ["merge-base", "--is-ancestor", acceptedMainMergeBase, "origin/main"],
                  root,
                  true,
                ).status === 0;
              if (!mergeBaseIsValid) {
                errors.push("named integration tip is not based on accepted origin/main");
              } else {
                integrationRouteAccepted = true;
                liveValidation.route = "named_integration";
              }
            }
          }
        }

        if (!integrationRouteAccepted) {
          errors.push("assignment base does not satisfy the named-integration route");
        }
      } else {
        errors.push("assignment cannot resolve an unknown integration strategy");
      }
      const commitEpoch = Number(git(["show", "-s", "--format=%ct", assignment.base_sha], root).stdout.trim());
      if (Number.isFinite(activatedAt) && Number.isFinite(commitEpoch)) {
        const baseAgeHours = activatedAt / 3_600_000 - commitEpoch / 3_600;
        const limit = topology.worktree_policy.base_max_age_hours_at_activation;
        if (baseAgeHours > limit) {
          warnings.push(`assignment base was ${Math.round(baseAgeHours)} hours old at activation; policy is ${limit}`);
        }
      }
    }
  }

  return {
    workstream,
    deliveryLane,
    sourceAdmissionReady,
    capabilityDomainIds,
  };
}

function contextEnvelope(
  topologyPath,
  topology,
  workstream,
  assignment,
  deliveryLane,
  sourceAuthorized,
) {
  const laneBound =
    ["appfw_workstation_assignment@3", "appfw_workstation_assignment@4"].includes(
      assignment?.schema,
    ) && deliveryLane;
  const forbiddenRoots = [
    ...new Set([
      ...(topology.integration_only_paths ?? []),
      ...(workstream.forbidden_roots ?? []),
    ]),
  ];
  return {
    schema: "appfw_workstream_context@1",
    program_id: topology.program_id,
    generated_from: topologyPath,
    workstream: {
      id: workstream.id,
      priority: workstream.priority,
      slug: workstream.slug,
      name: workstream.name,
      owns: workstream.owns,
      delivers: workstream.delivers,
      depends_on: workstream.depends_on,
      activation_gates: workstream.activation_gates,
    },
    assignment: assignment
      ? {
          assignment_id: assignment.assignment_id ?? null,
          portfolio_id: assignment.portfolio_id ?? null,
          workstation_id: assignment.workstation_id,
          capability_domain_ids:
            assignment.capability_domain_ids ??
            (assignment.workstream_id ? [assignment.workstream_id] : []),
          lane_outcome_owner:
            assignment.lane_outcome_owner ?? assignment.workstream_lead,
          technical_lead: assignment.technical_lead,
          branch_owner: assignment.branch_owner,
          independent_reviewer: assignment.independent_reviewer,
          admitted_by: assignment.admitted_by,
          integration_owner: assignment.integration_owner,
          branch: assignment.branch,
          base_sha: assignment.base_sha,
          integration_branch: assignment.integration_branch,
          execution_target: assignment.execution_target ?? null,
          planned_model_tier: assignment.planned_model_tier ?? null,
          routing_basis: assignment.routing_basis ?? null,
          economics_budget: assignment.economics_budget ?? null,
          delegation_depth: assignment.delegation_depth ?? null,
          status: assignment.status,
          review_after: assignment.review_after,
        }
      : null,
    delivery_lane: laneBound
      ? {
          id: deliveryLane.id,
          title: deliveryLane.title,
          result: deliveryLane.result,
          branch: deliveryLane.branch,
          destination_branch: deliveryLane.destination_branch,
          workstreams: deliveryLane.workstreams,
        }
      : null,
    read_first: workstream.required_context,
    source_authorized: sourceAuthorized,
    write_authority: sourceAuthorized
      ? "validated_delivery_lane_assignment_with_shared_live_lease"
      : "none_context_only",
    allowed_write_roots: sourceAuthorized ? deliveryLane.write_roots : [],
    forbidden_roots: forbiddenRoots,
    capability_domain_reference_roots: workstream.write_roots,
    integration_only_paths: topology.integration_only_paths,
    authority_model: topology.authority_model,
    baseline_obligations: topology.baseline_obligations,
    publishes: workstream.publishes,
    proof: {
      inner_loop: workstream.inner_loop,
      handoff: workstream.handoff_checks,
    },
  };
}

function main() {
  let options;
  try {
    options = parseArgs(process.argv.slice(2));
  } catch (error) {
    console.error(error.message);
    console.error(usage());
    process.exit(2);
  }
  if (options.help) {
    console.log(usage());
    return;
  }

  const root = repositoryRoot();
  const topologyPath = resolve(root, options.topology);
  const errors = [];
  const warnings = [];
  const topology = loadJson(topologyPath, "topology", errors);
  const workstreams = topology ? validateTopology(topology, root, errors, warnings) : [];

  let assignment = null;
  let assignmentBinding = null;
  const liveValidation = {
    route: null,
    local_integration_tip: null,
    remote_integration_tip: null,
    accepted_main_merge_base: null,
  };
  if (options.assignment) {
    const assignmentPath = resolve(root, options.assignment);
    assignment = loadJson(assignmentPath, "assignment", errors);
    if (assignment && topology) {
      assignmentBinding = validateAssignment(
        assignment,
        topology,
        root,
        errors,
        warnings,
        options.live,
        liveValidation,
      );
    }
  }

  let selectedWorkstream = null;
  if (options.context) {
    selectedWorkstream = workstreams.find((item) => item.id === options.context);
    if (!selectedWorkstream) errors.push(`unknown context workstream: ${options.context}`);
    const assignedCapabilityDomainIds = Array.isArray(
      assignmentBinding?.capabilityDomainIds,
    )
      ? assignmentBinding.capabilityDomainIds
      : [];
    if (
      assignmentBinding &&
      selectedWorkstream &&
      !assignedCapabilityDomainIds.includes(selectedWorkstream.id)
    ) {
      errors.push(
        assignment.schema === "appfw_workstation_assignment@3"
          ? `assignment is for ${assignedCapabilityDomainIds[0] ?? "unknown"}, not requested context ${selectedWorkstream.id}`
          : `assignment does not include requested context ${selectedWorkstream.id}`,
      );
    }
  }

  const output = {
    schema: "appfw_workstream_check@1",
    ok: errors.length === 0,
    topology: relative(root, topologyPath),
    program_id: topology?.program_id ?? null,
    priority_order: topology?.priority_order ?? [],
    initial_active_producer_limit: topology?.wip_policy?.initial_active_producer_limit ?? null,
    workstream_count: workstreams.length,
    errors,
    warnings,
  };
  const sourceAdmissionReady = Boolean(
    assignmentBinding?.sourceAdmissionReady && errors.length === 0,
  );
  const sourceAuthorized = false;
  if (sourceAdmissionReady) {
    warnings.push(
      "local admission checks passed, but source authority remains false until the Program Flow Controller verifies and holds a shared live exclusive lease",
    );
  }
  if (selectedWorkstream && topology) {
    output.context = contextEnvelope(
      relative(root, topologyPath),
      topology,
      selectedWorkstream,
      assignment,
      assignmentBinding?.deliveryLane ?? null,
      sourceAuthorized,
    );
  }
  if (assignment) {
    output.assignment = {
      product_increment_id: assignment.product_increment_id ?? null,
      delivery_lane_id: assignment.delivery_lane_id ?? null,
      capability_domain_ids:
        assignment.capability_domain_ids ??
        (assignment.workstream_id ? [assignment.workstream_id] : []),
      workstation_id: assignment.workstation_id,
      branch: assignment.branch,
      base_sha: assignment.base_sha,
      execution_target: assignment.execution_target ?? null,
      planned_model_tier: assignment.planned_model_tier ?? null,
      routing_basis: assignment.routing_basis ?? null,
      economics_budget: assignment.economics_budget ?? null,
      delegation_depth: assignment.delegation_depth ?? null,
      status: assignment.status,
      source_admission_ready: sourceAdmissionReady,
      source_authorized: sourceAuthorized,
      authority_reason: sourceAdmissionReady
        ? "shared_live_lease_not_verified"
        : "local_assignment_or_lineage_not_ready",
    };
  }
  if (options.live) output.live_validation = liveValidation;

  if (options.json) {
    console.log(JSON.stringify(output, null, 2));
  } else if (output.ok) {
    console.log(`Nexus workstream topology is valid (${workstreams.length} workstreams).`);
    console.log(`Priority: ${topology.priority_order.join(" -> ")}`);
    if (selectedWorkstream) console.log(`Context ready for ${selectedWorkstream.id} ${selectedWorkstream.name}.`);
    for (const warning of warnings) console.warn(`warning: ${warning}`);
  } else {
    console.error("Nexus workstream topology is invalid:");
    for (const error of errors) console.error(`- ${error}`);
  }

  if (!output.ok) process.exitCode = 1;
}

main();
