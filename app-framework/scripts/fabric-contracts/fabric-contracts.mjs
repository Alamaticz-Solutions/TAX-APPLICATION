// Fabric contract library: loads the nine fabric contract schemas, validates
// records against them (closed-schema structural validation plus the
// cross-field business rules the schemas cannot express), and provides the
// canonical-JSON digest used for record integrity.
//
// Normative source: docs/specs/hosted-product-factory.md (Contracts Touched,
// Registration and composition, Mobile platform surfaces). Validation style
// follows scripts/agent-routing/appfw-model-route.mjs.

import { createHash } from "node:crypto";
import { readFileSync, readdirSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const here = dirname(fileURLToPath(import.meta.url));

const RFC3339 =
  /^\d{4}-\d{2}-\d{2}[Tt]\d{2}:\d{2}:\d{2}(?:\.\d+)?(?:[Zz]|[+-]\d{2}:\d{2})$/;

export class ContractError extends Error {
  constructor(code, message, location = "$") {
    super(`${code}: ${message} (${location})`);
    this.code = code;
    this.location = location;
  }
}

function fail(code, message, location) {
  throw new ContractError(code, message, location);
}

function isObject(value) {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

function canonicalize(value) {
  if (Array.isArray(value)) return `[${value.map(canonicalize).join(",")}]`;
  if (isObject(value)) {
    const keys = Object.keys(value).sort();
    return `{${keys
      .map((key) => `${JSON.stringify(key)}:${canonicalize(value[key])}`)
      .join(",")}}`;
  }
  return JSON.stringify(value);
}

export function canonicalJson(value) {
  return canonicalize(value);
}

export function digest(value) {
  return createHash("sha256").update(canonicalJson(value)).digest("hex");
}

// Computes the integrity digest for a record: the sha256 of the canonical
// JSON of every field except record_digest itself.
export function recordDigest(record) {
  const { record_digest: _ignored, ...rest } = record;
  return digest(rest);
}

function resolveSchemaRef(root, reference, location) {
  if (!reference.startsWith("#/") || reference.includes("..")) {
    fail("invalid_schema_document", "unsupported schema reference", location);
  }
  return reference
    .slice(2)
    .split("/")
    .reduce(
      (current, key) =>
        current?.[key.replaceAll("~1", "/").replaceAll("~0", "~")],
      root
    );
}

function typeMatches(value, type) {
  if (type === "null") return value === null;
  if (type === "array") return Array.isArray(value);
  if (type === "object") return isObject(value);
  if (type === "integer") return Number.isInteger(value);
  if (type === "number")
    return typeof value === "number" && Number.isFinite(value);
  return typeof value === type;
}

function validateSchemaValue(value, schema, root, location = "$") {
  if (schema.$ref) {
    const resolved = resolveSchemaRef(root, schema.$ref, location);
    if (!resolved)
      fail("invalid_schema_document", "unresolved schema reference", location);
    return validateSchemaValue(value, resolved, root, location);
  }
  if (schema.oneOf) {
    let matches = 0;
    for (const option of schema.oneOf) {
      try {
        validateSchemaValue(value, option, root, location);
        matches += 1;
      } catch (error) {
        if (!(error instanceof ContractError)) throw error;
      }
    }
    if (matches !== 1)
      fail("invalid_record", "value matches no closed schema branch", location);
    return;
  }
  if (
    schema.const !== undefined &&
    canonicalJson(value) !== canonicalJson(schema.const)
  ) {
    fail("invalid_record", "value does not match its declared constant", location);
  }
  if (
    schema.enum &&
    !schema.enum.some((item) => canonicalJson(item) === canonicalJson(value))
  ) {
    fail("invalid_record", "value is not one of the allowed values", location);
  }
  if (schema.type) {
    const types = Array.isArray(schema.type) ? schema.type : [schema.type];
    if (!types.some((type) => typeMatches(value, type))) {
      fail("invalid_record", "value has an invalid primitive type", location);
    }
  }
  if (value === null) return;
  if (typeof value === "string") {
    if (schema.minLength !== undefined && value.length < schema.minLength)
      fail("invalid_record", "string is too short", location);
    if (schema.maxLength !== undefined && value.length > schema.maxLength)
      fail("invalid_record", "string is too long", location);
    if (schema.pattern && !new RegExp(schema.pattern).test(value))
      fail("invalid_record", "string has an invalid shape", location);
    if (
      schema.format === "date-time" &&
      (!RFC3339.test(value) || Number.isNaN(Date.parse(value)))
    ) {
      fail("invalid_record", "timestamp is not RFC3339", location);
    }
  }
  if (typeof value === "number") {
    if (schema.minimum !== undefined && value < schema.minimum)
      fail("invalid_record", "number is below its minimum", location);
    if (schema.maximum !== undefined && value > schema.maximum)
      fail("invalid_record", "number is above its maximum", location);
  }
  if (Array.isArray(value)) {
    if (schema.minItems !== undefined && value.length < schema.minItems)
      fail("invalid_record", "array is too short", location);
    if (schema.maxItems !== undefined && value.length > schema.maxItems)
      fail("invalid_record", "array is too long", location);
    if (
      schema.uniqueItems &&
      new Set(value.map(canonicalJson)).size !== value.length
    ) {
      fail("invalid_record", "array contains duplicate values", location);
    }
    if (schema.items) {
      value.forEach((item, index) =>
        validateSchemaValue(item, schema.items, root, `${location}/${index}`)
      );
    }
  }
  if (isObject(value)) {
    for (const key of schema.required ?? []) {
      if (!(key in value))
        fail("invalid_record", `required field "${key}" is missing`, location);
    }
    if (schema.additionalProperties === false) {
      for (const key of Object.keys(value)) {
        if (!(key in (schema.properties ?? {})))
          fail("unknown_field", `unknown field "${key}" is not allowed`, location);
      }
    }
    for (const [key, nested] of Object.entries(value)) {
      if (schema.properties?.[key]) {
        validateSchemaValue(
          nested,
          schema.properties[key],
          root,
          `${location}/${key}`
        );
      }
    }
  }
}

// Contract registry -----------------------------------------------------------

export const CONTRACTS = Object.freeze({
  "fabric_app_registration@1": "fabric-app-registration.v1.schema.json",
  "app_component_snapshot@1": "app-component-snapshot.v1.schema.json",
  "fabric_observation_event@1": "fabric-observation-event.v1.schema.json",
  "fabric_command_proposal@1": "fabric-command-proposal.v1.schema.json",
  "factory_run_manifest@1": "factory-run-manifest.v1.schema.json",
  "factory_required_action@1": "factory-required-action.v1.schema.json",
  "product_work_model@1": "product-work-model.v1.schema.json",
  "portfolio_model@1": "portfolio-model.v1.schema.json",
  "framework_request@1": "framework-request.v1.schema.json"
});

const schemaCache = new Map();

export function loadSchema(contractId) {
  const file = CONTRACTS[contractId];
  if (!file) fail("unknown_contract", `unknown contract "${contractId}"`);
  if (!schemaCache.has(contractId)) {
    schemaCache.set(
      contractId,
      JSON.parse(readFileSync(join(here, file), "utf8"))
    );
  }
  return schemaCache.get(contractId);
}

export function listContractIds() {
  return Object.keys(CONTRACTS);
}

export function listSchemaFiles() {
  return readdirSync(here).filter((name) => name.endsWith(".schema.json"));
}

// Cross-field business rules ---------------------------------------------------
// Structural closed-schema validation cannot express these; they come from the
// spec's registration/lifecycle, mobile, economics, and work-model rules.

function requirePayloadFields(record, fields, kind) {
  for (const field of fields) {
    if (!(field in (record.payload ?? {}))) {
      fail(
        "business_rule",
        `${kind} observation payload requires "${field}"`,
        "$/payload"
      );
    }
  }
}

const TCO_COMPONENTS = new Set([
  "human_effort",
  "agent_tokens",
  "compute",
  "review",
  "retry",
  "defect",
  "support",
  "rollback",
  "runtime_infrastructure",
  "operations"
]);

const BUSINESS_RULES = {
  "fabric_app_registration@1"(record) {
    if (record.lifecycle_state === "idea") {
      if (record.cmdb_ci_ref !== null)
        fail(
          "business_rule",
          "idea-stage applications are not in the CMDB (cmdb_ci_ref must be null)",
          "$/cmdb_ci_ref"
        );
      if (record.matriculated_at !== null)
        fail(
          "business_rule",
          "idea-stage applications cannot be matriculated",
          "$/matriculated_at"
        );
    }
    if ((record.cmdb_ci_ref === null) !== (record.matriculated_at === null)) {
      fail(
        "business_rule",
        "cmdb_ci_ref and matriculated_at are set together at matriculation",
        "$/cmdb_ci_ref"
      );
    }
    for (const surface of record.platform_surfaces) {
      if (surface.surface === "native_mobile" && !(surface.targets?.length > 0)) {
        fail(
          "business_rule",
          "native_mobile surfaces must declare at least one target",
          "$/platform_surfaces"
        );
      }
    }
    const environments = record.environment_refs.map((ref) => ref.environment);
    if (record.lifecycle_state === "idea") {
      if (environments.some((environment) => environment !== "dev")) {
        fail(
          "business_rule",
          "idea-stage applications have dev-environment presence only (G-F13)",
          "$/environment_refs"
        );
      }
      if (record.repository_ref === null) {
        fail(
          "business_rule",
          "idea-stage applications are fabric-registered with a repository",
          "$/repository_ref"
        );
      }
    }
    if (
      environments.some((environment) => environment !== "dev") &&
      record.cmdb_ci_ref === null
    ) {
      fail(
        "business_rule",
        "promotion beyond dev mechanically requires the CMDB configuration item",
        "$/environment_refs"
      );
    }
  },
  "app_component_snapshot@1"(record) {
    const mobile = record.platform_surface.startsWith("native_mobile");
    if (record.built !== null) {
      if (mobile && !record.built.store_build)
        fail(
          "business_rule",
          "native-mobile built state requires store_build identity",
          "$/built"
        );
      if (!mobile && !record.built.image_digest)
        fail(
          "business_rule",
          "web/backend built state requires image_digest",
          "$/built"
        );
    }
    if (record.deployed !== null) {
      const isStoreRelease = "store_track" in record.deployed;
      if (mobile !== isStoreRelease) {
        fail(
          "business_rule",
          "deployed state variant must match the platform surface",
          "$/deployed"
        );
      }
    }
    if (record.running !== null && mobile && !record.running.version_distribution) {
      fail(
        "business_rule",
        "native-mobile running state requires version_distribution (version skew is first-class)",
        "$/running"
      );
    }
  },
  "fabric_observation_event@1"(record) {
    if (record.kind === "framework_release") {
      if (record.scope !== "framework" || record.app_id !== null) {
        fail(
          "business_rule",
          "framework_release observations are framework-scoped with null app_id",
          "$/scope"
        );
      }
    } else if (record.app_id === null) {
      fail(
        "business_rule",
        `${record.kind} observations require an app_id`,
        "$/app_id"
      );
    }
    if (record.kind === "whole_cost") {
      requirePayloadFields(
        record,
        ["cost_component", "amount", "currency", "measurement_source"],
        "whole_cost"
      );
      if (!TCO_COMPONENTS.has(record.payload.cost_component)) {
        fail(
          "business_rule",
          "whole_cost cost_component must come from the AF-M08.3 extended taxonomy",
          "$/payload/cost_component"
        );
      }
    }
    if (record.kind === "value_realization") {
      requirePayloadFields(record, ["metric", "value"], "value_realization");
    }
    if (record.kind === "store_release") {
      requirePayloadFields(record, ["store_track", "review_status"], "store_release");
    }
    if (record.kind === "version_adoption") {
      requirePayloadFields(record, ["version_distribution"], "version_adoption");
    }
    if (record.kind === "crash_free") {
      requirePayloadFields(record, ["rate"], "crash_free");
    }
    if (record.kind === "satisfaction" || record.kind === "store_rating") {
      requirePayloadFields(record, ["source", "period"], record.kind);
      if ("free_text" in record.payload) {
        fail(
          "business_rule",
          "satisfaction-class payloads exclude free text until a governed contract exists",
          "$/payload/free_text"
        );
      }
      if (record.kind === "store_rating" && record.payload.public_aggregate !== true) {
        fail(
          "business_rule",
          "store_rating observations must be public aggregates",
          "$/payload/public_aggregate"
        );
      }
      if (record.kind === "satisfaction" && !("cohort_size" in record.payload)) {
        fail(
          "business_rule",
          "satisfaction payloads require cohort_size for suppression",
          "$/payload/cohort_size"
        );
      }
    }
  },
  "fabric_command_proposal@1"(record) {
    const disposition = record.disposition;
    if (disposition && disposition.state !== "submitted") {
      if (!disposition.decided_by || !disposition.decided_at) {
        fail(
          "business_rule",
          "a dispositioned proposal records who decided and when",
          "$/disposition"
        );
      }
    }
  },
  "factory_run_manifest@1"(record) {
    if (record.profile === "candidate" && record.gates.deferred.length > 0) {
      fail(
        "business_rule",
        "candidate-profile runs may not defer gates",
        "$/gates/deferred"
      );
    }
    if (
      ["success", "failure"].includes(record.result) &&
      !record.finished_at
    ) {
      fail(
        "business_rule",
        "completed runs record finished_at",
        "$/finished_at"
      );
    }
  },
  "factory_required_action@1"(record) {
    if (record.status === "fulfilled") {
      if (!record.fulfillment || record.fulfillment.evidence_refs.length === 0) {
        fail(
          "business_rule",
          "fulfillment attaches evidence, not prose",
          "$/fulfillment"
        );
      }
    }
  },
  "product_work_model@1"(record) {
    const incrementIds = new Set(record.increments.map((item) => item.id));
    if (incrementIds.size !== record.increments.length) {
      fail("business_rule", "increment ids must be unique", "$/increments");
    }
    const taskIds = new Set(record.tasks.map((item) => item.id));
    if (taskIds.size !== record.tasks.length) {
      fail("business_rule", "task ids must be unique", "$/tasks");
    }
    for (const increment of record.increments) {
      if (increment.status === "accepted" && !increment.accepted_by) {
        fail(
          "business_rule",
          "only a named Outcome Owner acceptance marks an increment Accepted",
          "$/increments"
        );
      }
    }
    for (const task of record.tasks) {
      if (!incrementIds.has(task.increment_id)) {
        fail(
          "business_rule",
          `task "${task.id}" references unknown increment "${task.increment_id}"`,
          "$/tasks"
        );
      }
      if (task.state === "accepted" && !task.accepted_by) {
        fail(
          "business_rule",
          "only a named Outcome Owner acceptance marks a task Accepted",
          "$/tasks"
        );
      }
    }
  },
  "portfolio_model@1"(record) {
    const themeIds = new Set(record.themes.map((theme) => theme.id));
    if (themeIds.size !== record.themes.length) {
      fail("business_rule", "theme ids must be unique", "$/themes");
    }
    for (const participation of record.participations) {
      if (!themeIds.has(participation.theme_id)) {
        fail(
          "business_rule",
          `participation references unknown theme "${participation.theme_id}"`,
          "$/participations"
        );
      }
    }
    for (const strategy of record.strategies ?? []) {
      for (const ref of strategy.theme_refs ?? []) {
        if (!themeIds.has(ref)) {
          fail(
            "business_rule",
            `strategy "${strategy.id}" references unknown theme "${ref}"`,
            "$/strategies"
          );
        }
      }
    }
  },
  "framework_request@1"(record) {
    const disposition = record.disposition;
    if (disposition?.state === "duplicate" && !disposition.duplicate_of) {
      fail(
        "business_rule",
        "duplicate dispositions name the surviving request",
        "$/disposition/duplicate_of"
      );
    }
    if (disposition?.state === "accepted" && !disposition.framework_work_ref) {
      fail(
        "business_rule",
        "an accepted request names the framework work item that now carries it — the request record is never the tracking record",
        "$/disposition/framework_work_ref"
      );
    }
    if (disposition && disposition.state !== "submitted") {
      if (!disposition.dispositioned_by || !disposition.dispositioned_at) {
        fail(
          "business_rule",
          "framework request dispositions record who decided and when",
          "$/disposition"
        );
      }
    }
  }
};

// Public validation entry point -------------------------------------------------

export function validateRecord(contractId, record, { verifyDigest = true } = {}) {
  const schema = loadSchema(contractId);
  validateSchemaValue(record, schema, schema);
  if (record.schema !== contractId) {
    fail(
      "invalid_record",
      `record schema field "${record.schema}" does not match contract "${contractId}"`,
      "$/schema"
    );
  }
  if (verifyDigest) {
    const expected = recordDigest(record);
    if (record.record_digest !== expected) {
      fail(
        "digest_mismatch",
        "record_digest does not match the canonical record content",
        "$/record_digest"
      );
    }
  }
  BUSINESS_RULES[contractId]?.(record);
  return true;
}

// Fixture-case runner -------------------------------------------------------------

export function loadCases() {
  return JSON.parse(
    readFileSync(join(here, "fabric-contract-cases.v1.json"), "utf8")
  );
}

// Cases carry records without record_digest unless digest handling is "keep";
// the runner injects the correct digest for every other case so fixtures stay
// hand-maintainable while digest verification stays on.
export function prepareCaseRecord(entry) {
  const record = structuredClone(entry.record);
  if (entry.digest !== "keep") {
    record.record_digest = recordDigest(record);
  }
  return record;
}

export function runCases(cases = loadCases()) {
  const results = [];
  for (const entry of cases.cases) {
    const record = prepareCaseRecord(entry);
    let outcome = "valid";
    let error = null;
    try {
      validateRecord(entry.contract, record);
    } catch (caught) {
      if (!(caught instanceof ContractError)) throw caught;
      outcome = "invalid";
      error = caught.message;
    }
    results.push({
      contract: entry.contract,
      name: entry.name,
      expect: entry.expect,
      outcome,
      ok: outcome === entry.expect,
      error
    });
  }
  return results;
}
