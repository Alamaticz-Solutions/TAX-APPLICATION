#!/usr/bin/env node

import { createHash } from "node:crypto";
import { execFileSync } from "node:child_process";
import { readFileSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";

const scriptDir = dirname(fileURLToPath(import.meta.url));
const repoRoot = resolve(scriptDir, "../..");
const paths = {
  schema: resolve(scriptDir, "appfw-model-route.v1.schema.json"),
  policy: resolve(scriptDir, "route-policy.v1.json"),
  codex: resolve(scriptDir, "codex-adapter.v1.json"),
  claude: resolve(scriptDir, "claude-adapter.v1.json"),
  cases: resolve(scriptDir, "routing-cases.v1.json")
};

const REGISTRY_ID_SYNTAX = /^[A-Za-z0-9][A-Za-z0-9._:@-]{0,79}$/;
const CANONICAL_UUID = "[0-9a-f]{8}-[0-9a-f]{4}-[1-8][0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}";
const AUTHORITY_REFERENCE = Object.fromEntries(
  ["actor", "assignment", "run", "task", "route", "evidence"].map((kind) => [kind, new RegExp(`^${kind}:${CANONICAL_UUID}$`)])
);
const SHA256 = /^[a-f0-9]{64}$/;
const RFC3339 = /^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}(?:\.\d{1,9})?(?:Z|[+-]\d{2}:\d{2})$/;
const REPOSITORY_ROOT = /^(?!\.\.?$)(?!.*(?:^|\/)\.\.(?:\/|$))[A-Za-z0-9._-]+(?:\/[A-Za-z0-9._-]+)*$/;
const COST_MEASUREMENT_SOURCE = /^(?:provider_measurement:pm_|declared_rate:dr_)[0-9a-f]{8}-[0-9a-f]{4}-[1-8][0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/;
const PROVIDERS = new Set(["codex", "claude"]);
const MODEL_ID = {
  codex: /^gpt-[a-z0-9]+(?:[.-][a-z0-9]+)*$/,
  claude: /^claude-(?:haiku|sonnet|opus)-[a-z0-9]+(?:-[a-z0-9]+)*$/
};
const EFFORT = new Set(["low", "medium", "high", "xhigh"]);
const NETWORK_RANK = new Map([["none", 0], ["restricted", 1], ["approved", 2]]);
const FINDING_KEYS = ["blocker", "critical", "important", "should_address", "nice_to_address"];

export class RouteError extends Error {
  constructor(code, message) {
    super(message);
    this.name = "RouteError";
    this.code = code;
  }
}

function fail(code, message) {
  throw new RouteError(code, message);
}

function parseJsonFile(path, code = "invalid_json") {
  try {
    return JSON.parse(readFileSync(path, "utf8"));
  } catch {
    fail(code, "JSON input could not be parsed");
  }
}

function isObject(value) {
  return value !== null && typeof value === "object" && !Array.isArray(value);
}

function exactObject(value, required, optional = [], code = "invalid_schema") {
  if (!isObject(value)) fail(code, "Expected an object");
  const allowed = new Set([...required, ...optional]);
  for (const key of Object.keys(value)) {
    if (!allowed.has(key)) fail("unknown_field", "An unknown field is not allowed");
  }
  for (const key of required) {
    if (!(key in value)) fail(code, "A required field is missing");
  }
}

function validRegistryId(value, className, nullable = false) {
  if (nullable && value === null) return;
  if (typeof value !== "string" || !REGISTRY_ID_SYNTAX.test(value)) {
    fail("invalid_identifier", `A ${className} identifier is invalid`);
  }
}

function validAuthorityReference(value, kind, nullable = false) {
  if (nullable && value === null) return;
  if (typeof value !== "string" || !AUTHORITY_REFERENCE[kind]?.test(value)) {
    fail("invalid_identifier", "An authority-issued identifier is invalid");
  }
}

function authorityUuid(material) {
  const hex = digest(material).slice(0, 32).split("");
  hex[12] = "5";
  hex[16] = ["8", "9", "a", "b"][Number.parseInt(hex[16], 16) % 4];
  return `${hex.slice(0, 8).join("")}-${hex.slice(8, 12).join("")}-${hex.slice(12, 16).join("")}-${hex.slice(16, 20).join("")}-${hex.slice(20).join("")}`;
}

function authorityReference(kind, material) {
  return `${kind}:${authorityUuid({ authority: "appfw-model-route", kind, material })}`;
}

function stringArray(value, code = "invalid_schema") {
  if (!Array.isArray(value) || value.some((item) => typeof item !== "string" || item.length === 0)) {
    fail(code, "Expected an array of non-empty strings");
  }
  if (new Set(value).size !== value.length) fail(code, "Duplicate array values are not allowed");
}

function repositoryRootArray(value) {
  stringArray(value);
  if (value.some((item) => !REPOSITORY_ROOT.test(item))) {
    fail("privacy_violation", "Repository roots must be normalized and relative");
  }
}

function enumValue(value, allowed, code = "invalid_schema") {
  if (!allowed.includes(value)) fail(code, "A field contains an unsupported value");
}

function nonNegative(value, nullable = false) {
  if (nullable && value === null) return;
  if (!Number.isInteger(value) || value < 0) fail("invalid_telemetry", "Telemetry counts must be non-negative integers");
}

function derSequenceHasExactLength(bytes) {
  if (bytes.length < 4 || bytes[0] !== 0x30) return false;
  const firstLength = bytes[1];
  if (firstLength < 0x80) return firstLength + 2 === bytes.length;
  const lengthBytes = firstLength & 0x7f;
  if (lengthBytes === 0 || lengthBytes > 4 || bytes.length < lengthBytes + 2) return false;
  let declared = 0;
  for (let index = 0; index < lengthBytes; index += 1) declared = (declared * 256) + bytes[index + 2];
  return declared + lengthBytes + 2 === bytes.length;
}

function includesBytes(haystack, needle) {
  return haystack.indexOf(needle) !== -1;
}

function looksLikePrivateKeyBytes(bytes) {
  if (bytes.length > 16384 || bytes.length < 16) return false;
  if (bytes.subarray(0, 15).toString("ascii") === "openssh-key-v1\0") return true;
  if (!derSequenceHasExactLength(bytes)) return false;
  const hex = bytes.toString("hex");
  const privateKeyOids = [
    "06092a864886f70d010101", // rsaEncryption
    "06072a8648ce3d0201", // ecPublicKey in SEC1/PKCS#8
    "06032b6570", // Ed25519
    "06032b6571" // Ed448
  ];
  if (privateKeyOids.some((oid) => includesBytes(hex, oid))) return true;
  // PKCS#1 RSA and SEC1 EC begin with a version integer followed by key material.
  return /^30(?:[0-9a-f]{2}){1,5}02010002[0-9a-f]{2}/.test(hex)
    || /^30(?:[0-9a-f]{2}){1,5}02010104[0-9a-f]{2}/.test(hex);
}

function decodedPrivateKeyCandidate(value) {
  if (typeof value !== "string" || value.length > 32768) return false;
  const candidates = [value, ...(value.match(/[A-Za-z0-9+/_=\-\t\n\v\f\r ]{40,}/g) ?? [])];
  for (const candidate of candidates) {
    const normalized = candidate.replace(/[\t\n\v\f\r ]/g, "");
    if (normalized.length < 24 || normalized.length > 22000 || !/^[A-Za-z0-9+/_-]+={0,2}$/.test(normalized)) continue;
    const base64 = normalized.replaceAll("-", "+").replaceAll("_", "/").replace(/=+$/, "");
    const padded = `${base64}${"=".repeat((4 - (base64.length % 4)) % 4)}`;
    let decoded;
    try {
      decoded = Buffer.from(padded, "base64");
    } catch {
      continue;
    }
    if (decoded.length === 0) continue;
    const canonical = decoded.toString("base64").replace(/=+$/, "");
    if (canonical !== base64.replace(/=+$/, "") || !looksLikePrivateKeyBytes(decoded)) continue;
    return true;
  }
  return false;
}

function assertSafeMetadata(value) {
  const forbiddenKeys = new Set([
    "prompt", "messages", "content", "raw_payload", "request_body", "response_body",
    "secret", "password", "api_key", "access_token", "refresh_token", "private_key"
  ]);
  const independentlyValidatedHashes = new Set([
    "policy_digest", "assignment_digest", "adapter_digest", "record_digest", "source_commit", "source_blob"
  ]);
  const base64KeyToken = /(?:^|[^A-Za-z0-9+/])(?:[A-Za-z0-9+/]{43,}={0,2})(?=$|[^A-Za-z0-9+/=])/;
  const base64UrlKeyToken = /(?:^|[^A-Za-z0-9_-])(?:[A-Za-z0-9_-]{43,}={0,2})(?=$|[^A-Za-z0-9_=-])/;
  const compactHexKey = /(?:^|[^A-Fa-f0-9])[A-Fa-f0-9]{64,}(?=$|[^A-Fa-f0-9])/;
  const visit = (current, retainedKey = null) => {
    if (typeof current === "string") {
      const trimmed = current.trim();
      let containsSerializedPayload = false;
      if ((trimmed.startsWith("{") && trimmed.endsWith("}")) || (trimmed.startsWith("[") && trimmed.endsWith("]"))) {
        try {
          containsSerializedPayload = typeof JSON.parse(trimmed) === "object";
        } catch {
          containsSerializedPayload = false;
        }
      }
      if (
        /(?:^|[^A-Za-z0-9])sk-[A-Za-z0-9_-]{8,}/i.test(current)
        || /\b(?:AKIA|ASIA)[A-Z0-9]{16}\b/.test(current)
        || /\bgh[pousr]_[A-Za-z0-9]{20,}\b/.test(current)
        || /\bxox[baprs]-[A-Za-z0-9-]{10,}\b/.test(current)
        || /\bAIza[0-9A-Za-z_-]{20,}\b/.test(current)
        || /\beyJ[A-Za-z0-9_-]+\.[A-Za-z0-9_-]+\.[A-Za-z0-9_-]+\b/.test(current)
        || /\bBearer\s+[A-Za-z0-9._~-]{8,}/i.test(current)
        || /\b[A-Z0-9._%+-]+@[A-Z0-9.-]+\.[A-Z]{2,}\b/i.test(current)
        || /\b\d{3}-\d{2}-\d{4}\b/.test(current)
        || /\b(?:\+?1[-. ]?)?\(?\d{3}\)?[-. ]\d{3}[-. ]\d{4}\b/.test(current)
        || /\b(?:patient|member|mrn|medical[-_ ]?record)[-_: ]+[A-Za-z0-9-]{4,}\b/i.test(current)
        || /-----BEGIN (?:(?:ENCRYPTED|RSA|EC|OPENSSH) )?PRIVATE KEY-----/.test(current)
        || /-----BEGIN PGP PRIVATE KEY BLOCK-----/.test(current)
        || decodedPrivateKeyCandidate(current)
        || /[\u0000-\u0008\u000B\u000C\u000E-\u001F\u007F]/.test(current)
        || (!independentlyValidatedHashes.has(retainedKey) && (base64KeyToken.test(current) || base64UrlKeyToken.test(current)))
        || (!independentlyValidatedHashes.has(retainedKey) && compactHexKey.test(current))
        || containsSerializedPayload
        || /(?:^|[/\\])\.env(?:$|[/\\])/i.test(current)
        || /^\//.test(current)
        || /^[A-Za-z]:[\\/]/.test(current)
        || /^\\\\/.test(current)
        || /^file:\/\//i.test(current)
        || /^[a-z][a-z0-9+.-]*:\/\/[^/\s]+@/i.test(current)
        || /(?:^|[/\\])\.\.(?:$|[/\\])/.test(current)
        || /(?:password|api[_-]?key|access[_-]?token)\s*[:=]\s*\S+/i.test(current)
      ) {
        fail("privacy_violation", "Sensitive content is not allowed in route metadata");
      }
      return;
    }
    if (Array.isArray(current)) {
      current.forEach((item) => visit(item, retainedKey));
      return;
    }
    if (isObject(current)) {
      for (const [key, nested] of Object.entries(current)) {
        visit(key);
        const normalizedKey = key.toLowerCase().replace(/[^a-z0-9]+/g, "_");
        if (forbiddenKeys.has(normalizedKey)) fail("privacy_violation", "Raw or sensitive payload fields are not allowed in route metadata");
        visit(nested, normalizedKey);
      }
    }
  };
  visit(value);
}

function assertProviderNeutral(value) {
  const forbidden = new Set(["provider", "provider_id", "model", "model_id", "effort"]);
  const visit = (current) => {
    if (Array.isArray(current)) return current.forEach(visit);
    if (!isObject(current)) return;
    for (const [key, nested] of Object.entries(current)) {
      if (forbidden.has(key)) fail("provider_leakage", "Provider-specific fields are forbidden in the task contract");
      visit(nested);
    }
  };
  visit(value);
}

function canonicalize(value) {
  if (value === null || typeof value === "boolean" || typeof value === "string") return JSON.stringify(value);
  if (typeof value === "number") {
    if (!Number.isFinite(value)) fail("invalid_number", "Non-finite numbers are not canonical JSON");
    return JSON.stringify(value);
  }
  if (Array.isArray(value)) return `[${value.map(canonicalize).join(",")}]`;
  if (isObject(value)) {
    return `{${Object.keys(value).sort().map((key) => `${JSON.stringify(key)}:${canonicalize(value[key])}`).join(",")}}`;
  }
  fail("invalid_json_value", "Unsupported JSON value");
}

export function canonicalJson(value) {
  return canonicalize(value);
}

export function digest(value) {
  return createHash("sha256").update(canonicalJson(value)).digest("hex");
}

function typeMatches(value, type) {
  if (type === "null") return value === null;
  if (type === "array") return Array.isArray(value);
  if (type === "object") return isObject(value);
  if (type === "integer") return Number.isInteger(value);
  if (type === "number") return typeof value === "number" && Number.isFinite(value);
  return typeof value === type;
}

function resolveSchemaRef(root, reference) {
  if (!reference.startsWith("#/") || reference.includes("..")) fail("invalid_schema_document", "The route schema contains an unsupported reference");
  return reference.slice(2).split("/").reduce((current, key) => current?.[key.replaceAll("~1", "/").replaceAll("~0", "~")], root);
}

function validateSchemaValue(value, schema, root, location = "$") {
  if (schema.$ref) {
    const resolved = resolveSchemaRef(root, schema.$ref);
    if (!resolved) fail("invalid_schema_document", "The route schema contains an unresolved reference");
    return validateSchemaValue(value, resolved, root, location);
  }
  if (schema.oneOf) {
    let matches = 0;
    for (const option of schema.oneOf) {
      try {
        validateSchemaValue(value, option, root, location);
        matches += 1;
      } catch (error) {
        if (!(error instanceof RouteError)) throw error;
      }
    }
    if (matches !== 1) fail("invalid_schema", "A route value does not match its closed schema");
    return;
  }
  if (schema.const !== undefined && canonicalJson(value) !== canonicalJson(schema.const)) fail("invalid_schema", "A route value does not match its closed schema");
  if (schema.enum && !schema.enum.some((item) => canonicalJson(item) === canonicalJson(value))) fail("invalid_schema", "A route value does not match its closed schema");
  if (schema.type) {
    const types = Array.isArray(schema.type) ? schema.type : [schema.type];
    if (!types.some((type) => typeMatches(value, type))) fail("invalid_schema", "A route value has an invalid primitive type");
  }
  if (value === null) return;
  if (typeof value === "string") {
    if (schema.minLength !== undefined && value.length < schema.minLength) fail("invalid_schema", "A route string is too short");
    if (schema.maxLength !== undefined && value.length > schema.maxLength) fail("invalid_schema", "A route string is too long");
    if (schema.pattern && !new RegExp(schema.pattern).test(value)) fail("invalid_schema", "A route string has an invalid shape");
    if (schema.format === "date-time" && (!RFC3339.test(value) || Number.isNaN(Date.parse(value)))) fail("invalid_timestamp", "A route timestamp is not RFC3339");
  }
  if (typeof value === "number" && schema.minimum !== undefined && value < schema.minimum) fail("invalid_schema", "A route number is below its minimum");
  if (Array.isArray(value)) {
    if (schema.minItems !== undefined && value.length < schema.minItems) fail("invalid_schema", "A route array is too short");
    if (schema.uniqueItems && new Set(value.map(canonicalJson)).size !== value.length) fail("invalid_schema", "A route array contains duplicate values");
    if (schema.items) value.forEach((item, index) => validateSchemaValue(item, schema.items, root, `${location}/${index}`));
  }
  if (isObject(value)) {
    for (const key of schema.required ?? []) if (!(key in value)) fail("invalid_schema", "A required route field is missing");
    if (schema.additionalProperties === false) {
      for (const key of Object.keys(value)) if (!(key in (schema.properties ?? {}))) fail("unknown_field", "An unknown field is not allowed");
    }
    for (const [key, nested] of Object.entries(value)) {
      if (schema.properties?.[key]) validateSchemaValue(nested, schema.properties[key], root, `${location}/${key}`);
    }
  }
}

function validateClosedRecordSchema(record) {
  const schema = parseJsonFile(paths.schema, "invalid_schema_document");
  validateSchemaValue(record, schema, schema);
}

function git(args) {
  try {
    return execFileSync("git", args, { cwd: repoRoot, encoding: "utf8", stdio: ["ignore", "pipe", "pipe"] }).trim();
  } catch {
    fail("source_reference_unavailable", "A required source reference is unavailable");
  }
}

function valueAt(value, path) {
  return path.split(".").reduce((current, key) => current?.[key], value);
}

function predicateMatches(predicate, value) {
  if (predicate.op === "always") return true;
  if (predicate.op === "any") return predicate.predicates.some((item) => predicateMatches(item, value));
  if (predicate.op === "all") return predicate.predicates.every((item) => predicateMatches(item, value));
  const actual = valueAt(value, predicate.path);
  if (predicate.op === "equals") return actual === predicate.value;
  if (predicate.op === "in") return Array.isArray(predicate.value) && predicate.value.includes(actual);
  if (predicate.op === "nonempty") return Array.isArray(actual) ? actual.length > 0 : typeof actual === "string" && actual.length > 0;
  if (predicate.op === "empty") return Array.isArray(actual) && actual.length === 0;
  fail("invalid_policy", "The route policy contains an unsupported predicate");
}

function validateActor(actor) {
  exactObject(actor, ["assignment_id", "actor_id", "run_id"]);
  validAuthorityReference(actor.assignment_id, "assignment");
  validAuthorityReference(actor.actor_id, "actor");
  validAuthorityReference(actor.run_id, "run");
}

function validateSubject(subject) {
  exactObject(subject, ["outcome_id", "product_increment_id", "deliverable_id", "task_id", "role_card_id", "stage"], ["assignment_id", "workstream_id", "parent_run_id"]);
  for (const key of ["outcome_id", "product_increment_id", "deliverable_id"]) validRegistryId(subject[key], "business");
  validRegistryId(subject.role_card_id, "role-card");
  validRegistryId(subject.workstream_id ?? null, "workstream", true);
  validAuthorityReference(subject.task_id, "task");
  validAuthorityReference(subject.assignment_id ?? null, "assignment", true);
  validAuthorityReference(subject.parent_run_id ?? null, "run", true);
  enumValue(subject.stage, ["prototype", "integration_candidate", "release_candidate"]);
}

function validateTaskShape(shape) {
  exactObject(shape, [
    "role", "read_roots", "write_roots", "allowed_tools", "network_posture", "data_classification",
    "change_class", "change_class_status", "ambiguity", "consequence", "sensitive_surfaces",
    "signature_significance", "delegation_depth", "delegation_type", "execution_target", "material_child",
    "requested_capabilities"
  ]);
  enumValue(shape.role, ["researcher", "product", "architecture", "coordination", "implementer", "reviewer", "integration"]);
  repositoryRootArray(shape.read_roots);
  repositoryRootArray(shape.write_roots);
  stringArray(shape.allowed_tools);
  stringArray(shape.requested_capabilities);
  shape.allowed_tools.forEach((item) => validRegistryId(item, "tool"));
  shape.requested_capabilities.forEach((item) => validRegistryId(item, "capability"));
  enumValue(shape.network_posture, [...NETWORK_RANK.keys()]);
  enumValue(shape.data_classification, ["public", "internal", "confidential", "restricted", "phi"]);
  enumValue(shape.change_class, ["A", "B", "C", "D"]);
  enumValue(shape.change_class_status, ["provisional", "exact"]);
  enumValue(shape.ambiguity, ["low", "medium", "high"]);
  enumValue(shape.consequence, ["low", "medium", "high", "irreversible"]);
  stringArray(shape.sensitive_surfaces);
  for (const surface of shape.sensitive_surfaces) enumValue(surface, ["tenant", "auth", "runtime", "provider", "generated", "security", "privacy", "financial", "clinical"]);
  if (typeof shape.signature_significance !== "boolean" || typeof shape.material_child !== "boolean") fail("invalid_schema", "Boolean task fields are required");
  nonNegative(shape.delegation_depth);
  if (shape.delegation_depth > 1) fail("delegation_depth_exceeded", "Delegation depth exceeds the P1 maximum");
  enumValue(shape.delegation_type, ["none", "direct_child"]);
  enumValue(shape.execution_target, ["local", "approved_cloud"]);
}

function validatePrivacy(privacy, classification) {
  exactObject(privacy, ["data_classification", "redaction_posture", "retained_field_allowlist_version", "prompt_content_retained", "secret_content_retained"]);
  if (privacy.data_classification !== classification) fail("privacy_violation", "Privacy classification does not match the task");
  enumValue(privacy.redaction_posture, ["strict", "metadata_only"]);
  if (privacy.retained_field_allowlist_version !== "appfw_route_telemetry_allowlist@1" || privacy.prompt_content_retained !== false || privacy.secret_content_retained !== false) {
    fail("privacy_violation", "Route telemetry must use the metadata-only allowlist");
  }
}

function subset(requested, authorized) {
  const allowed = new Set(authorized);
  return requested.every((item) => allowed.has(item));
}

function validateRoutingContext(context, subject, shape) {
  exactObject(context, [
    "implementer", "reviewer", "reviewer_required", "provider_available", "required_instruction_ids",
    "authorized_read_roots", "authorized_write_roots", "authorized_tools", "authorized_network_posture",
    "strategic_decision", "human_decision_id"
  ]);
  validateActor(context.implementer);
  if (context.reviewer !== null) validateActor(context.reviewer);
  if (typeof context.reviewer_required !== "boolean" || typeof context.provider_available !== "boolean" || typeof context.strategic_decision !== "boolean") {
    fail("invalid_schema", "Routing flags must be boolean");
  }
  stringArray(context.required_instruction_ids);
  context.required_instruction_ids.forEach((item) => validRegistryId(item, "instruction"));
  repositoryRootArray(context.authorized_read_roots);
  repositoryRootArray(context.authorized_write_roots);
  stringArray(context.authorized_tools);
  context.authorized_tools.forEach((item) => validRegistryId(item, "tool"));
  enumValue(context.authorized_network_posture, [...NETWORK_RANK.keys()]);
  validAuthorityReference(context.human_decision_id, "evidence", true);
  if (subject.assignment_id && subject.assignment_id !== context.implementer.assignment_id) fail("assignment_mismatch", "The task and implementer assignments differ");
  if (!subset(shape.read_roots, context.authorized_read_roots) || !subset(shape.write_roots, context.authorized_write_roots) || !subset(shape.allowed_tools, context.authorized_tools)) {
    fail("scope_widening", "Task roots or tools exceed the authorized boundary");
  }
  if (NETWORK_RANK.get(shape.network_posture) > NETWORK_RANK.get(context.authorized_network_posture)) fail("scope_widening", "Task network posture exceeds the authorized boundary");
  if (shape.delegation_depth === 0 && (shape.delegation_type !== "none" || subject.parent_run_id)) fail("delegation_shape_invalid", "Root execution cannot declare a parent route");
  if (shape.delegation_depth === 1 && (shape.delegation_type !== "direct_child" || !subject.parent_run_id)) fail("implicit_child_inheritance", "A direct child requires an explicit parent route");
  if (shape.material_child && (shape.delegation_depth !== 1 || !context.reviewer_required)) fail("implicit_child_inheritance", "A material child requires an explicit direct-child and review route");
  if (context.reviewer_required && context.reviewer === null) fail("independent_reviewer_unavailable", "An independent reviewer route is required");
  if (context.reviewer) {
    for (const key of ["assignment_id", "actor_id", "run_id"]) {
      if (context.implementer[key] === context.reviewer[key]) fail("reviewer_not_independent", "Implementer and reviewer identities must be distinct");
    }
  }
}

function validateTaskRequest(request) {
  assertSafeMetadata(request);
  assertProviderNeutral(request);
  exactObject(request, ["schema", "subject", "task_shape", "routing_context", "privacy"]);
  if (request.schema !== "appfw_model_route@1") fail("unsupported_contract_major", "Only appfw_model_route@1 is supported");
  validateSubject(request.subject);
  validateTaskShape(request.task_shape);
  validateRoutingContext(request.routing_context, request.subject, request.task_shape);
  validatePrivacy(request.privacy, request.task_shape.data_classification);
}

export function validatePolicy(policy) {
  assertSafeMetadata(policy);
  exactObject(policy, ["schema", "semantic_version", "max_delegation_depth", "profile_strength", "provider_neutral_profiles", "instructions", "routing_rules", "privacy", "otel"]);
  if (policy.schema !== "appfw_route_policy@1" || policy.semantic_version !== "1.0.0" || policy.max_delegation_depth !== 1) fail("invalid_policy", "Route policy identity or delegation limit is invalid");
  stringArray(policy.provider_neutral_profiles, "invalid_policy");
  policy.provider_neutral_profiles.forEach((item) => validRegistryId(item, "profile"));
  exactObject(policy.profile_strength, ["implementer", "reviewer"], [], "invalid_policy");
  stringArray(policy.profile_strength.implementer, "invalid_policy");
  stringArray(policy.profile_strength.reviewer, "invalid_policy");
  for (const profile of [...policy.profile_strength.implementer, ...policy.profile_strength.reviewer]) {
    if (!policy.provider_neutral_profiles.includes(profile)) fail("invalid_policy", "A profile-strength entry is not provider-neutral");
  }
  if (canonicalJson(policy).match(/"(?:model|model_id)"\s*:/)) fail("provider_leakage", "Provider model identifiers are forbidden in route policy");
  if (!Array.isArray(policy.instructions) || !Array.isArray(policy.routing_rules) || policy.routing_rules.length === 0) fail("invalid_policy", "Route policy rules are missing");
  const instructionIds = new Set();
  for (const instruction of policy.instructions) {
    exactObject(instruction, ["id", "path", "heading", "precedence", "required", "rationale_code", "applies_when"], [], "invalid_policy");
    validRegistryId(instruction.id, "instruction");
    validRegistryId(instruction.rationale_code, "policy");
    if (instructionIds.has(instruction.id) || !REPOSITORY_ROOT.test(instruction.path) || typeof instruction.heading !== "string" || instruction.heading.length === 0 || !Number.isInteger(instruction.precedence) || typeof instruction.required !== "boolean") fail("invalid_policy", "An instruction policy entry is invalid");
    instructionIds.add(instruction.id);
  }
  const ruleIds = new Set();
  for (const rule of policy.routing_rules) {
    exactObject(rule, ["id", "match", "implementer_profile", "reviewer_profile", "rationale_codes", "requires_human_decision"], [], "invalid_policy");
    validRegistryId(rule.id, "policy");
    validRegistryId(rule.implementer_profile, "profile");
    validRegistryId(rule.reviewer_profile, "profile");
    stringArray(rule.rationale_codes, "invalid_policy");
    rule.rationale_codes.forEach((item) => validRegistryId(item, "policy"));
    if (ruleIds.has(rule.id) || !policy.profile_strength.implementer.includes(rule.implementer_profile) || !policy.profile_strength.reviewer.includes(rule.reviewer_profile) || typeof rule.requires_human_decision !== "boolean") fail("invalid_policy", "A route policy rule is invalid");
    ruleIds.add(rule.id);
  }
  if (!ruleIds.has("routine-delivery")) fail("invalid_policy", "The route policy requires an explicit routine default");
  exactObject(policy.privacy, ["retained_field_allowlist_version", "prompt_content_retained", "secret_content_retained"], [], "invalid_policy");
  if (
    policy.privacy.retained_field_allowlist_version !== "appfw_route_telemetry_allowlist@1"
    || policy.privacy.prompt_content_retained !== false
    || policy.privacy.secret_content_retained !== false
  ) {
    fail("invalid_policy", "Route policy privacy must enforce the metadata-only allowlist");
  }
}

export function validateAdapter(adapter, provider, policy) {
  assertSafeMetadata(adapter);
  exactObject(adapter, ["schema", "adapter_id", "adapter_version", "provider", "consumes_semantic_range", "effective_at", "unsupported_conditions", "profiles"]);
  if (adapter.schema !== "appfw_model_adapter@1" || adapter.provider !== provider || adapter.consumes_semantic_range !== ">=1.0.0 <2.0.0") fail("invalid_adapter", "Adapter identity or semantic range is invalid");
  validRegistryId(adapter.adapter_id, "adapter");
  if (!/^1\.\d+\.\d+$/.test(adapter.adapter_version)) fail("invalid_adapter", "Adapter version is invalid");
  if (!RFC3339.test(adapter.effective_at) || Number.isNaN(Date.parse(adapter.effective_at))) fail("invalid_adapter", "Adapter effective time is invalid");
  stringArray(adapter.unsupported_conditions, "invalid_adapter");
  if (!isObject(adapter.profiles)) fail("invalid_adapter", "Adapter profiles are missing");
  if (Object.keys(adapter.profiles).some((profile) => !policy.provider_neutral_profiles.includes(profile))) fail("invalid_adapter", "Adapter contains an unknown profile");
  for (const profile of policy.provider_neutral_profiles) {
    const mapping = adapter.profiles[profile];
    exactObject(mapping, ["model", "effort", "execution_targets", "capabilities", "fallback_profile"], [], "invalid_adapter");
    if (!MODEL_ID[provider].test(mapping.model) || !EFFORT.has(mapping.effort)) fail("invalid_adapter", "Adapter model or effort is outside the provider grammar");
    stringArray(mapping.execution_targets, "invalid_adapter");
    mapping.execution_targets.forEach((item) => enumValue(item, ["local", "approved_cloud"], "invalid_adapter"));
    stringArray(mapping.capabilities, "invalid_adapter");
    mapping.capabilities.forEach((item) => validRegistryId(item, "capability"));
    if (mapping.fallback_profile !== null) {
      if (!policy.provider_neutral_profiles.includes(mapping.fallback_profile)) fail("invalid_adapter", "Adapter fallback profile is invalid");
      const strength = policy.profile_strength.implementer.includes(profile) ? policy.profile_strength.implementer : policy.profile_strength.reviewer;
      const currentRank = strength.indexOf(profile);
      const fallbackRank = strength.indexOf(mapping.fallback_profile);
      if (currentRank < 0 || fallbackRank < currentRank) fail("invalid_adapter", "Adapter fallback cannot reduce route obligations");
    }
  }
  for (const profile of policy.provider_neutral_profiles) {
    const mapping = adapter.profiles[profile];
    if (mapping.fallback_profile === null) continue;
    const fallback = adapter.profiles[mapping.fallback_profile];
    if (!mapping.capabilities.every((capability) => fallback.capabilities.includes(capability))) {
      fail("invalid_adapter", "Adapter fallback cannot reduce required capabilities");
    }
  }
}

function selectInstructions(request, policy) {
  const requiredIds = new Set(request.routing_context.required_instruction_ids);
  const selected = policy.instructions.filter((instruction) => predicateMatches(instruction.applies_when, request) || requiredIds.has(instruction.id));
  for (const id of requiredIds) {
    if (!policy.instructions.some((instruction) => instruction.id === id)) fail("instruction_missing", "A required instruction identifier is unknown");
  }
  const sourceCommit = git(["rev-parse", "HEAD"]);
  const references = selected.map((instruction) => {
    let sourceBlob;
    let source;
    try {
      sourceBlob = git(["rev-parse", `${sourceCommit}:${instruction.path}`]);
      source = execFileSync("git", ["show", `${sourceCommit}:${instruction.path}`], { cwd: repoRoot, encoding: "utf8", stdio: ["ignore", "pipe", "pipe"] });
    } catch {
      fail("instruction_missing", "A required instruction source is missing");
    }
    if (!source.includes(instruction.heading)) fail("instruction_heading_missing", "A required instruction heading is missing");
    return {
      id: instruction.id,
      path: instruction.path,
      heading: instruction.heading,
      source_commit: sourceCommit,
      source_blob: sourceBlob,
      required: instruction.required || requiredIds.has(instruction.id),
      precedence: instruction.precedence,
      rationale_code: instruction.rationale_code
    };
  });
  return references.sort((left, right) => left.precedence - right.precedence || left.id.localeCompare(right.id));
}

function strongerProfile(profiles, left, right) {
  const leftRank = profiles.indexOf(left);
  const rightRank = profiles.indexOf(right);
  if (leftRank < 0 || rightRank < 0) fail("invalid_policy", "A routing rule uses an unranked profile");
  return rightRank > leftRank ? right : left;
}

function chooseRouteObligations(request, policy) {
  const allMatches = policy.routing_rules.filter((candidate) => predicateMatches(candidate.match, request));
  if (allMatches.length === 0) fail("no_lawful_route", "No route policy rule matched the task");
  const specialized = allMatches.filter((rule) => rule.id !== "routine-delivery");
  const matches = specialized.length > 0 ? specialized : allMatches;
  let implementerProfile = matches[0].implementer_profile;
  let reviewerProfile = matches[0].reviewer_profile;
  for (const rule of matches.slice(1)) {
    implementerProfile = strongerProfile(policy.profile_strength.implementer, implementerProfile, rule.implementer_profile);
    reviewerProfile = strongerProfile(policy.profile_strength.reviewer, reviewerProfile, rule.reviewer_profile);
  }
  const requiresHumanDecision = matches.some((rule) => rule.requires_human_decision);
  if (requiresHumanDecision && !request.routing_context.human_decision_id) fail("human_decision_required", "This route requires a named human decision");
  const dominantRule = [...matches].sort((left, right) => {
    const implementerDelta = policy.profile_strength.implementer.indexOf(right.implementer_profile) - policy.profile_strength.implementer.indexOf(left.implementer_profile);
    if (implementerDelta !== 0) return implementerDelta;
    return policy.profile_strength.reviewer.indexOf(right.reviewer_profile) - policy.profile_strength.reviewer.indexOf(left.reviewer_profile);
  })[0];
  return {
    rule_id: dominantRule.id,
    matched_rule_ids: matches.map((rule) => rule.id),
    matched_reviewer_profiles: [...new Set(matches.map((rule) => rule.reviewer_profile))],
    implementer_profile: implementerProfile,
    reviewer_profile: reviewerProfile,
    rationale_codes: [...new Set(matches.flatMap((rule) => rule.rationale_codes))],
    requires_human_decision: requiresHumanDecision
  };
}

export function independentReviewRequired(request, obligations) {
  return Boolean(
    obligations.reviewer_profile
    || request.task_shape.write_roots.length > 0
    || request.task_shape.material_child
    || request.subject.stage !== "prototype"
    || request.task_shape.change_class === "A"
    || request.routing_context.strategic_decision
    || request.task_shape.signature_significance
    || request.task_shape.sensitive_surfaces.length > 0
  );
}

function enforceReviewAuthority(request, obligations) {
  const minimumReviewRequired = independentReviewRequired(request, obligations);
  if (minimumReviewRequired && !request.routing_context.reviewer_required) fail("review_requirement_downgrade", "Caller state cannot lower the required independent review");
  if (minimumReviewRequired && request.routing_context.reviewer === null) fail("independent_reviewer_unavailable", "An independent reviewer route is required");
}

function resolveProfile(adapter, profile, executionTarget, requiredCapabilities = []) {
  const mapping = adapter.profiles[profile];
  if (!mapping) fail("unsupported_provider_profile", "The provider does not support a required profile");
  if (!mapping.execution_targets.includes(executionTarget)) fail("unsupported_execution_target", "The provider profile does not support the execution target");
  if (!requiredCapabilities.every((capability) => mapping.capabilities.includes(capability))) fail("unsupported_required_capability", "The provider profile does not support a required capability");
  if (mapping.fallback_profile !== null) {
    const fallback = adapter.profiles[mapping.fallback_profile];
    if (!fallback || !requiredCapabilities.every((capability) => fallback.capabilities.includes(capability))) {
      fail("unsupported_required_capability", "The provider fallback profile does not preserve required capabilities");
    }
  }
  return { profile, model: mapping.model, effort: mapping.effort, capabilities: [...mapping.capabilities] };
}

export function computeAssignmentAuthorityDigest(authority) {
  return digest(authority);
}

function emptyActualRun() {
  return {
    status: "not_started",
    actual_provider: null,
    actual_model: null,
    actual_effort: null,
    started_at: null,
    completed_at: null,
    input_tokens: null,
    output_tokens: null,
    context_bytes: null,
    always_loaded_bytes: null,
    startup_to_first_correct_action_ms: null,
    elapsed_ms: null,
    review_wait_ms: null,
    retries: 0,
    material_human_corrections: 0,
    rework_events: 0,
    findings: { blocker: 0, critical: 0, important: 0, should_address: 0, nice_to_address: 0 },
    cost: { amount: null, currency: null, measurement_source: null, quality: "unavailable", measured_at: null },
    implemented_evidence_refs: [],
    accepted_evidence_refs: [],
    escaped_defect_count: 0,
    telemetry_complete: false,
    value_claim: { claimed: false, metric: null, improvement_percent: null, baseline_evidence_refs: [] }
  };
}

function recordMaterial(record) {
  const { record_digest: _recordDigest, ...material } = record;
  return material;
}

function config() {
  const policy = parseJsonFile(paths.policy, "invalid_policy");
  assertSafeMetadata(policy);
  validatePolicy(policy);
  const adapters = { codex: parseJsonFile(paths.codex, "invalid_adapter"), claude: parseJsonFile(paths.claude, "invalid_adapter") };
  assertSafeMetadata(adapters);
  validateAdapter(adapters.codex, "codex", policy);
  validateAdapter(adapters.claude, "claude", policy);
  return { policy, adapters };
}

function deriveAuthoritativeDecision(request, provider, policy, adapter) {
  const instructionSelection = selectInstructions(request, policy);
  const obligations = chooseRouteObligations(request, policy);
  enforceReviewAuthority(request, obligations);
  if (!request.routing_context.provider_available) fail("provider_unavailable", "The requested provider is unavailable");
  const implementer = resolveProfile(adapter, obligations.implementer_profile, request.task_shape.execution_target, request.task_shape.requested_capabilities);
  const requiredReviewerCapabilities = [...new Set(
    obligations.matched_reviewer_profiles.flatMap((profile) => adapter.profiles[profile]?.capabilities ?? [])
  )];
  const reviewer = resolveProfile(adapter, obligations.reviewer_profile, request.task_shape.execution_target, requiredReviewerCapabilities);
  const adapterDigest = digest(adapter);
  return {
    route_decision_id: authorityReference("route", { request, provider, policy_version: policy.semantic_version, policy_digest: digest(policy), adapter_digest: adapterDigest }),
    instruction_selection: instructionSelection,
    route_plan: {
      policy_version: policy.semantic_version,
      policy_digest: digest(policy),
      rule_id: obligations.rule_id,
      matched_rule_ids: obligations.matched_rule_ids,
      implementer_profile: obligations.implementer_profile,
      reviewer_profile: obligations.reviewer_profile,
      rationale_codes: obligations.rationale_codes,
      escalation_owner: "program-flow-controller",
      approved_fallback_profile: null,
      max_delegation_depth: policy.max_delegation_depth,
      requires_human_decision: obligations.requires_human_decision
    },
    adapter_resolution: {
      requested_provider: provider,
      adapter_id: adapter.adapter_id,
      adapter_version: adapter.adapter_version,
      adapter_digest: adapterDigest,
      status: "supported",
      implementer,
      reviewer,
      fallback_deviation: null,
      failure_code: null
    }
  };
}

export function createAssignmentAuthority(request, provider) {
  if (!PROVIDERS.has(provider)) fail("unsupported_provider", "The requested provider is unsupported");
  validateTaskRequest(request);
  const { policy, adapters } = config();
  const adapter = adapters[provider];
  const decision = deriveAuthoritativeDecision(request, provider, policy, adapter);
  const authority = {
    schema: "assignment_authority@1",
    semantic_version: "1.0.0",
    route_decision_id: decision.route_decision_id,
    authorized_request: structuredClone(request),
    instruction_selection: structuredClone(decision.instruction_selection),
    route_plan: structuredClone(decision.route_plan),
    adapter_resolution: structuredClone(decision.adapter_resolution)
  };
  validateAssignmentAuthority(authority);
  return authority;
}

export function validateAssignmentAuthority(authority) {
  assertSafeMetadata(authority);
  const schemaDocument = parseJsonFile(paths.schema, "invalid_schema_document");
  validateSchemaValue(authority, schemaDocument.$defs.assignmentAuthority, schemaDocument, "$authority");
  exactObject(authority, [
    "schema", "semantic_version", "route_decision_id", "authorized_request", "instruction_selection", "route_plan", "adapter_resolution"
  ], [], "invalid_assignment_authority");
  if (authority.schema !== "assignment_authority@1" || authority.semantic_version !== "1.0.0") {
    fail("invalid_assignment_authority", "Assignment authority identity is invalid");
  }
  validAuthorityReference(authority.route_decision_id, "route");
  validateTaskRequest(authority.authorized_request);
  if (!Array.isArray(authority.instruction_selection) || authority.instruction_selection.length === 0) {
    fail("invalid_assignment_authority", "Assignment authority requires instruction evidence");
  }
  authority.instruction_selection.forEach(validateInstructionReference);
  const { policy, adapters } = config();
  const provider = authority.adapter_resolution?.requested_provider;
  if (!PROVIDERS.has(provider)) fail("invalid_assignment_authority", "Assignment authority provider is invalid");
  const decision = deriveAuthoritativeDecision(authority.authorized_request, provider, policy, adapters[provider]);
  if (
    authority.route_decision_id !== decision.route_decision_id
    || canonicalJson(authority.instruction_selection) !== canonicalJson(decision.instruction_selection)
    || canonicalJson(authority.route_plan) !== canonicalJson(decision.route_plan)
    || canonicalJson(authority.adapter_resolution) !== canonicalJson(decision.adapter_resolution)
  ) {
    fail("invalid_assignment_authority", "Assignment authority does not match authoritative configuration");
  }
  return authority;
}

export function routeTaskWithAuthority(request, provider) {
  const assignmentAuthority = createAssignmentAuthority(request, provider);
  const routePlan = { ...assignmentAuthority.route_plan, assignment_digest: computeAssignmentAuthorityDigest(assignmentAuthority) };
  const record = {
    schema: "appfw_model_route@1",
    semantic_version: "1.0.0",
    record_state: "planned",
    route_decision_id: assignmentAuthority.route_decision_id,
    subject: structuredClone(request.subject),
    task_shape: structuredClone(request.task_shape),
    routing_context: structuredClone(request.routing_context),
    instruction_selection: structuredClone(assignmentAuthority.instruction_selection),
    route_plan: routePlan,
    adapter_resolution: structuredClone(assignmentAuthority.adapter_resolution),
    actual_run: emptyActualRun(),
    privacy: structuredClone(request.privacy),
    record_digest: ""
  };
  record.record_digest = digest(recordMaterial(record));
  validateRouteRecord(record, assignmentAuthority);
  return { record, assignment_authority: assignmentAuthority };
}

export function routeTask(request, provider) {
  return routeTaskWithAuthority(request, provider).record;
}

function validateInstructionReference(reference) {
  exactObject(reference, ["id", "path", "heading", "source_commit", "source_blob", "required", "precedence", "rationale_code"]);
  validRegistryId(reference.id, "instruction");
  if (!REPOSITORY_ROOT.test(reference.path) || typeof reference.heading !== "string" || reference.heading.length === 0 || typeof reference.required !== "boolean" || !Number.isInteger(reference.precedence)) {
    fail("invalid_source_reference", "Instruction source metadata is invalid");
  }
  validRegistryId(reference.rationale_code, "policy");
  if (!/^[a-f0-9]{40}$/.test(reference.source_commit) || !/^[a-f0-9]{40,64}$/.test(reference.source_blob)) fail("invalid_source_reference", "Instruction source identity is invalid");
  const blob = git(["rev-parse", `${reference.source_commit}:${reference.path}`]);
  if (blob !== reference.source_blob) fail("source_reference_drift", "Instruction source blob does not match its commit");
  const source = git(["show", `${reference.source_commit}:${reference.path}`]);
  if (!source.includes(reference.heading)) fail("instruction_heading_missing", "A required instruction heading is missing");
}

function validateActualRun(run, record) {
  exactObject(run, [
    "status", "actual_provider", "actual_model", "actual_effort", "started_at", "completed_at", "input_tokens",
    "output_tokens", "context_bytes", "always_loaded_bytes", "startup_to_first_correct_action_ms", "elapsed_ms",
    "review_wait_ms", "retries", "material_human_corrections", "rework_events", "findings", "cost",
    "implemented_evidence_refs", "accepted_evidence_refs", "escaped_defect_count", "telemetry_complete", "value_claim"
  ]);
  enumValue(run.status, ["not_started", "running", "completed", "failed"]);
  for (const key of ["input_tokens", "output_tokens", "context_bytes", "always_loaded_bytes", "startup_to_first_correct_action_ms", "elapsed_ms", "review_wait_ms"]) nonNegative(run[key], true);
  for (const key of ["retries", "material_human_corrections", "rework_events", "escaped_defect_count"]) nonNegative(run[key]);
  exactObject(run.findings, FINDING_KEYS);
  FINDING_KEYS.forEach((key) => nonNegative(run.findings[key]));
  exactObject(run.cost, ["amount", "currency", "measurement_source", "quality", "measured_at"]);
  enumValue(run.cost.quality, ["actual", "estimated", "unavailable"]);
  if (run.cost.quality === "unavailable") {
    if (run.cost.amount !== null || run.cost.currency !== null || run.cost.measurement_source !== null || run.cost.measured_at !== null) fail("invalid_cost_evidence", "Unavailable cost cannot contain a measured value");
  } else if (typeof run.cost.amount !== "number" || run.cost.amount < 0 || !/^[A-Z]{3}$/.test(run.cost.currency ?? "") || !run.cost.measurement_source || !run.cost.measured_at) {
    fail("invalid_cost_evidence", "Measured cost requires amount, currency, source, and time");
  }
  if (run.cost.measurement_source !== null) {
    if (typeof run.cost.measurement_source !== "string" || !COST_MEASUREMENT_SOURCE.test(run.cost.measurement_source)) {
      fail("invalid_cost_evidence", "Cost evidence requires a typed opaque measurement source");
    }
    const expectedPrefix = run.cost.quality === "actual" ? "provider_measurement:" : "declared_rate:";
    if (!run.cost.measurement_source.startsWith(expectedPrefix)) fail("invalid_cost_evidence", "Cost source type must match evidence quality");
  }
  stringArray(run.implemented_evidence_refs);
  stringArray(run.accepted_evidence_refs);
  run.implemented_evidence_refs.forEach((item) => validAuthorityReference(item, "evidence"));
  run.accepted_evidence_refs.forEach((item) => validAuthorityReference(item, "evidence"));
  exactObject(run.value_claim, ["claimed", "metric", "improvement_percent", "baseline_evidence_refs"]);
  if (typeof run.value_claim.claimed !== "boolean") fail("invalid_value_claim", "Value-claim state must be explicit");
  stringArray(run.value_claim.baseline_evidence_refs);
  run.value_claim.baseline_evidence_refs.forEach((item) => validAuthorityReference(item, "evidence"));
  const timestamps = [run.started_at, run.completed_at, run.cost.measured_at].filter((value) => value !== null);
  if (timestamps.some((value) => typeof value !== "string" || !RFC3339.test(value) || Number.isNaN(Date.parse(value)))) fail("invalid_timestamp", "Route timestamps must be valid RFC3339 values");
  if (run.started_at && run.completed_at && Date.parse(run.completed_at) < Date.parse(run.started_at)) fail("invalid_timestamp_order", "Route completion cannot precede its start");
  if (run.cost.measured_at && run.completed_at && Date.parse(run.cost.measured_at) > Date.parse(run.completed_at)) fail("invalid_timestamp_order", "Cost measurement cannot follow route completion");
  if (record.record_state === "planned" && run.status !== "not_started") fail("invalid_record_state", "A planned route cannot claim an actual run");
  if (record.record_state === "running" && run.status !== "running") fail("invalid_record_state", "A running route requires running telemetry");
  if (record.record_state === "completed" && run.status !== "completed") fail("invalid_record_state", "A completed route requires completed telemetry");
  if (record.record_state === "failed" && run.status !== "failed") fail("invalid_record_state", "A failed route requires failed telemetry");
  const actualRoute = [run.actual_provider, run.actual_model, run.actual_effort];
  const measuredFields = [
    "input_tokens", "output_tokens", "context_bytes", "always_loaded_bytes",
    "startup_to_first_correct_action_ms", "elapsed_ms", "review_wait_ms"
  ];
  if (actualRoute.some((value) => value !== null) && actualRoute.some((value) => value === null)) fail("telemetry_incomplete", "Actual provider, model, and effort must be recorded together");
  const assertApprovedActualRoute = () => {
    if (run.actual_provider !== record.adapter_resolution.requested_provider) fail("silent_reroute", "Actual provider differs from the approved route");
    if (run.actual_model !== record.adapter_resolution.implementer.model || run.actual_effort !== record.adapter_resolution.implementer.effort) fail("silent_reroute", "Actual model or effort differs from the approved route");
  };
  const assertNoExecutionEvidence = (allowFailureMetadata) => {
    const hasExecutionEvidence =
      run.started_at !== null
      || run.completed_at !== null
      || measuredFields.some((key) => run[key] !== null)
      || run.retries !== 0
      || run.material_human_corrections !== 0
      || run.rework_events !== 0
      || run.escaped_defect_count !== 0
      || run.cost.quality !== "unavailable"
      || run.cost.amount !== null
      || run.cost.currency !== null
      || run.cost.measurement_source !== null
      || run.cost.measured_at !== null
      || run.implemented_evidence_refs.length > 0;
    const hasOutcomeMetadata = run.telemetry_complete || FINDING_KEYS.some((key) => run.findings[key] !== 0);
    if (actualRoute.some((value) => value !== null) || hasExecutionEvidence || (!allowFailureMetadata && hasOutcomeMetadata)) {
      fail("telemetry_without_actual_route", "Execution evidence requires the complete approved route");
    }
  };
  if (run.status !== "completed" && run.accepted_evidence_refs.length > 0) fail("accepted_evidence_incomplete", "Only a completed run can retain Accepted evidence");
  if (run.status !== "completed" && run.value_claim.claimed) fail("value_claim_without_accepted_outcome", "Only a completed run can claim economic value");
  if (run.status === "not_started") {
    assertNoExecutionEvidence(false);
  }
  if (run.status === "running") {
    if (!run.started_at || run.completed_at !== null || actualRoute.some((value) => value === null) || run.telemetry_complete) fail("telemetry_incomplete", "A running route requires an actual route and start time only");
    assertApprovedActualRoute();
  }
  if (run.status === "completed" && (!run.started_at || !run.completed_at)) fail("telemetry_incomplete", "A completed route requires start and completion times");
  if (run.status === "completed") {
    if (actualRoute.some((value) => value === null) || !run.telemetry_complete) fail("telemetry_incomplete", "A completed run requires its actual route and complete telemetry");
    assertApprovedActualRoute();
    for (const key of measuredFields) {
      if (run[key] === null) fail("telemetry_incomplete", "Completed telemetry is missing a measured field");
    }
    if (run.cost.quality === "unavailable") fail("telemetry_incomplete", "Completed telemetry requires measured or estimated cost evidence");
  }
  if (run.status === "failed") {
    if (actualRoute.every((value) => value === null)) {
      assertNoExecutionEvidence(true);
    } else {
      if (!run.started_at || !run.completed_at) fail("telemetry_incomplete", "A post-dispatch failure requires start and completion times");
      assertApprovedActualRoute();
    }
  }
  if (run.cost.quality === "actual") {
    if (!run.started_at || !run.completed_at) fail("invalid_cost_evidence", "Actual provider cost requires a completed execution interval");
    const measuredAt = Date.parse(run.cost.measured_at);
    if (measuredAt < Date.parse(run.started_at) || measuredAt > Date.parse(run.completed_at)) {
      fail("invalid_timestamp_order", "Actual provider cost must be measured during execution");
    }
  }
  if (run.elapsed_ms !== null) {
    for (const key of ["startup_to_first_correct_action_ms", "review_wait_ms"]) {
      if (run[key] !== null && run[key] > run.elapsed_ms) fail("invalid_duration", "A route duration cannot exceed elapsed time");
    }
    if (run.started_at && run.completed_at) {
      const wallClockMs = Date.parse(run.completed_at) - Date.parse(run.started_at);
      if (run.elapsed_ms > wallClockMs) fail("invalid_duration", "Elapsed execution time cannot exceed its wall-clock interval");
    }
  }
  if (run.status === "completed" && run.accepted_evidence_refs.length > 0 && (!run.telemetry_complete || run.implemented_evidence_refs.length === 0)) fail("accepted_evidence_incomplete", "Accepted evidence requires complete telemetry and implementation evidence");
  if (run.value_claim.claimed) {
    enumValue(run.value_claim.metric, ["startup_to_first_correct_action", "elapsed_time", "cost_per_accepted_outcome", "retry_rework_findings"]);
    if (typeof run.value_claim.improvement_percent !== "number" || run.value_claim.improvement_percent < 0 || run.value_claim.baseline_evidence_refs.length === 0 || run.accepted_evidence_refs.length === 0 || !run.telemetry_complete) {
      fail("value_claim_without_accepted_outcome", "A value claim requires baseline and Accepted-outcome evidence");
    }
  } else if (run.value_claim.metric !== null || run.value_claim.improvement_percent !== null || run.value_claim.baseline_evidence_refs.length > 0) {
    fail("invalid_value_claim", "An inactive value claim cannot retain claim evidence");
  }
}

export function validateRouteRecord(record, expectedAssignmentAuthority) {
  assertSafeMetadata(record);
  validateClosedRecordSchema(record);
  exactObject(record, [
    "schema", "semantic_version", "record_state", "route_decision_id", "subject", "task_shape", "routing_context",
    "instruction_selection", "route_plan", "adapter_resolution", "actual_run", "privacy", "record_digest"
  ]);
  if (record.schema !== "appfw_model_route@1" || record.semantic_version !== "1.0.0") fail("unsupported_contract_major", "Only appfw_model_route@1 is supported");
  enumValue(record.record_state, ["planned", "running", "completed", "failed"]);
  validAuthorityReference(record.route_decision_id, "route");
  validateSubject(record.subject);
  validateTaskShape(record.task_shape);
  validateRoutingContext(record.routing_context, record.subject, record.task_shape);
  validatePrivacy(record.privacy, record.task_shape.data_classification);
  if (!Array.isArray(record.instruction_selection) || record.instruction_selection.length === 0) fail("instruction_missing", "At least one instruction reference is required");
  record.instruction_selection.forEach(validateInstructionReference);
  exactObject(record.route_plan, ["policy_version", "policy_digest", "rule_id", "matched_rule_ids", "implementer_profile", "reviewer_profile", "rationale_codes", "escalation_owner", "approved_fallback_profile", "max_delegation_depth", "requires_human_decision", "assignment_digest"]);
  if (record.route_plan.policy_version !== "1.0.0" || record.route_plan.max_delegation_depth !== 1) fail("invalid_route_plan", "Route plan version or depth is invalid");
  if (!SHA256.test(record.route_plan.policy_digest)) fail("invalid_route_plan", "Route policy evidence is invalid");
  validRegistryId(record.route_plan.rule_id, "policy");
  stringArray(record.route_plan.matched_rule_ids);
  record.route_plan.matched_rule_ids.forEach((item) => validRegistryId(item, "policy"));
  validRegistryId(record.route_plan.implementer_profile, "profile");
  validRegistryId(record.route_plan.reviewer_profile, "profile", true);
  validRegistryId(record.route_plan.escalation_owner, "policy");
  validRegistryId(record.route_plan.approved_fallback_profile, "profile", true);
  stringArray(record.route_plan.rationale_codes);
  record.route_plan.rationale_codes.forEach((item) => validRegistryId(item, "policy"));
  if (typeof record.route_plan.requires_human_decision !== "boolean" || !SHA256.test(record.route_plan.assignment_digest)) fail("invalid_route_plan", "Route plan evidence is invalid");
  if (!isObject(expectedAssignmentAuthority)) fail("assignment_authority_required", "Validation requires the PFC-retained assignment authority envelope");
  validateAssignmentAuthority(expectedAssignmentAuthority);
  if (computeAssignmentAuthorityDigest(expectedAssignmentAuthority) !== record.route_plan.assignment_digest) {
    fail("assignment_authority_mismatch", "The route does not match the PFC-retained assignment authority");
  }
  exactObject(record.adapter_resolution, ["requested_provider", "adapter_id", "adapter_version", "adapter_digest", "status", "implementer", "reviewer", "fallback_deviation", "failure_code"]);
  if (!PROVIDERS.has(record.adapter_resolution.requested_provider) || record.adapter_resolution.status !== "supported" || record.adapter_resolution.failure_code !== null || record.adapter_resolution.fallback_deviation !== null) fail("invalid_adapter_resolution", "Resolved route must be supported without a hidden fallback");
  const authorizedRequest = expectedAssignmentAuthority.authorized_request;
  if (
    record.route_decision_id !== expectedAssignmentAuthority.route_decision_id
    || canonicalJson(record.subject) !== canonicalJson(authorizedRequest.subject)
    || canonicalJson(record.task_shape) !== canonicalJson(authorizedRequest.task_shape)
    || canonicalJson(record.routing_context) !== canonicalJson(authorizedRequest.routing_context)
    || canonicalJson(record.privacy) !== canonicalJson(authorizedRequest.privacy)
    || canonicalJson(record.instruction_selection) !== canonicalJson(expectedAssignmentAuthority.instruction_selection)
  ) {
    fail("assignment_authority_mismatch", "The route fields differ from the PFC-retained assignment authority");
  }
  const { assignment_digest: _assignmentDigest, ...declaredRoutePlan } = record.route_plan;
  if (canonicalJson(declaredRoutePlan) !== canonicalJson(expectedAssignmentAuthority.route_plan)) {
    fail("assignment_authority_mismatch", "The route plan differs from the PFC-retained assignment authority");
  }
  if (canonicalJson(record.adapter_resolution) !== canonicalJson(expectedAssignmentAuthority.adapter_resolution)) {
    fail("assignment_authority_mismatch", "Adapter resolution differs from the PFC-retained assignment authority");
  }
  validateActualRun(record.actual_run, record);
  const expectedRecordDigest = digest(recordMaterial(record));
  if (!SHA256.test(record.record_digest) || record.record_digest !== expectedRecordDigest) fail("record_digest_mismatch", "The route record digest is invalid");
  return record;
}

function deepMerge(base, patch) {
  if (!isObject(base) || !isObject(patch)) return structuredClone(patch);
  const result = structuredClone(base);
  for (const [key, value] of Object.entries(patch)) result[key] = isObject(value) && isObject(result[key]) ? deepMerge(result[key], value) : structuredClone(value);
  return result;
}

export function runSelfCheck() {
  parseJsonFile(paths.schema, "invalid_schema_document");
  config();
  const fixtures = parseJsonFile(paths.cases, "invalid_case_fixture");
  exactObject(fixtures, ["schema", "base_task", "canonical_cases", "negative_cases"]);
  if (fixtures.schema !== "appfw_routing_cases@1" || fixtures.canonical_cases.length !== 7) fail("invalid_case_fixture", "Exactly seven canonical routing cases are required");
  const canonicalResults = [];
  for (const item of fixtures.canonical_cases) {
    const task = deepMerge(fixtures.base_task, item.task_patch ?? {});
    try {
      const first = routeTask(task, item.provider);
      const second = routeTask(task, item.provider);
      if (canonicalJson(first) !== canonicalJson(second)) fail("nondeterministic_route", "Repeated route selection differed");
      if (item.expected_error) fail("case_expectation_failed", "A canonical failure case unexpectedly routed");
      if (first.route_plan.implementer_profile !== item.expected_implementer_profile || first.route_plan.reviewer_profile !== item.expected_reviewer_profile) fail("case_expectation_failed", "A canonical case selected the wrong profile");
    } catch (error) {
      if (!(error instanceof RouteError) || error.code !== item.expected_error) throw error;
    }
    canonicalResults.push(item.id);
  }
  const negativeResults = [];
  for (const item of fixtures.negative_cases) {
    let observed = null;
    try {
      if (item.mode === "record") {
        const routed = routeTaskWithAuthority(deepMerge(fixtures.base_task, item.task_patch ?? {}), item.provider ?? "codex");
        const record = routed.record;
        const mutated = deepMerge(record, item.record_patch ?? {});
        if (item.record_mutation === "break_instruction_heading") mutated.instruction_selection[0].heading = "# Missing Required Heading";
        if (item.record_mutation === "remove_required_instruction") mutated.instruction_selection.shift();
        if (item.record_mutation === "change_adapter_model") {
          mutated.adapter_resolution.implementer.model = mutated.adapter_resolution.requested_provider === "codex" ? "gpt-5.5" : "claude-opus-4-5";
        }
        if (item.recompute_assignment_digest) mutated.route_plan.assignment_digest = digest(recordMaterial(mutated));
        if (item.recompute_record_digest) mutated.record_digest = digest(recordMaterial(mutated));
        validateRouteRecord(mutated, routed.assignment_authority);
      } else {
        routeTask(deepMerge(fixtures.base_task, item.task_patch ?? {}), item.provider ?? "codex");
      }
    } catch (error) {
      if (error instanceof RouteError) observed = error.code;
      else throw error;
    }
    if (observed !== item.expected_error) fail("case_expectation_failed", `Negative case ${item.id} did not fail with its expected code`);
    negativeResults.push(item.id);
  }
  return {
    ok: true,
    schema: "appfw_route_check@1",
    semantic_version: "1.0.0",
    canonical_cases: { passed: canonicalResults.length, total: fixtures.canonical_cases.length, ids: canonicalResults },
    negative_cases: { passed: negativeResults.length, total: fixtures.negative_cases.length, ids: negativeResults },
    adapters: ["codex", "claude"],
    network_access: false,
    dispatch_performed: false
  };
}

function parseArgs(argv) {
  const options = { task: null, provider: null, validateRecord: null, assignmentAuthority: null, check: false };
  for (let index = 0; index < argv.length; index += 1) {
    const arg = argv[index];
    if (arg === "--json") continue;
    if (arg === "--check") options.check = true;
    else if (["--task", "--provider", "--validate-record", "--assignment-authority"].includes(arg)) {
      const value = argv[index + 1];
      if (!value || value.startsWith("--")) fail("invalid_arguments", "A command option is missing its value");
      if (arg === "--task") options.task = value;
      else if (arg === "--provider") options.provider = value;
      else if (arg === "--validate-record") options.validateRecord = value;
      else options.assignmentAuthority = value;
      index += 1;
    } else fail("invalid_arguments", "An unsupported command option was supplied");
  }
  const modes = [options.check, Boolean(options.task), Boolean(options.validateRecord)].filter(Boolean).length;
  if (modes !== 1 || (options.task && !options.provider) || (options.validateRecord && !options.assignmentAuthority) || (!options.validateRecord && options.assignmentAuthority)) {
    fail("invalid_arguments", "Select exactly one command mode and provide its required options");
  }
  return options;
}

function safeError(error) {
  if (error instanceof RouteError) return { ok: false, error: { code: error.code, message: error.message } };
  return { ok: false, error: { code: "internal_error", message: "Routing failed without exposing task content" } };
}

export function main(argv = process.argv.slice(2)) {
  try {
    const options = parseArgs(argv);
    let result;
    if (options.check) result = runSelfCheck();
    else if (options.task) {
      const routed = routeTaskWithAuthority(parseJsonFile(resolve(process.cwd(), options.task)), options.provider);
      result = { ok: true, route: routed.record, assignment_authority: routed.assignment_authority };
    } else {
      result = {
        ok: true,
        route: validateRouteRecord(
          parseJsonFile(resolve(process.cwd(), options.validateRecord)),
          parseJsonFile(resolve(process.cwd(), options.assignmentAuthority), "invalid_assignment_authority")
        )
      };
    }
    process.stdout.write(`${JSON.stringify(result)}\n`);
    return 0;
  } catch (error) {
    process.stdout.write(`${JSON.stringify(safeError(error))}\n`);
    return 1;
  }
}

if (process.argv[1] && import.meta.url === pathToFileURL(resolve(process.argv[1])).href) {
  process.exitCode = main();
}
