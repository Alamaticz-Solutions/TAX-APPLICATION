import assert from "node:assert/strict";
import { mkdtempSync, readFileSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { spawnSync } from "node:child_process";
import test from "node:test";

import {
  RouteError,
  canonicalJson,
  createAssignmentAuthority,
  digest,
  independentReviewRequired,
  routeTask,
  runSelfCheck,
  validateAdapter,
  validatePolicy,
  validateRouteRecord
} from "./appfw-model-route.mjs";

const cases = JSON.parse(readFileSync(new URL("./routing-cases.v1.json", import.meta.url), "utf8"));
const schema = JSON.parse(readFileSync(new URL("./appfw-model-route.v1.schema.json", import.meta.url), "utf8"));
const policy = JSON.parse(readFileSync(new URL("./route-policy.v1.json", import.meta.url), "utf8"));
const adapters = {
  codex: JSON.parse(readFileSync(new URL("./codex-adapter.v1.json", import.meta.url), "utf8")),
  claude: JSON.parse(readFileSync(new URL("./claude-adapter.v1.json", import.meta.url), "utf8"))
};
const cli = new URL("./appfw-model-route.mjs", import.meta.url);
const ACTUAL_COST_SOURCE = "provider_measurement:pm_123e4567-e89b-42d3-a456-426614174000";
const ESTIMATED_COST_SOURCE = "declared_rate:dr_123e4567-e89b-42d3-a456-426614174001";
const IMPLEMENTED_EVIDENCE = "evidence:60000000-0000-4000-8000-000000000001";
const ACCEPTED_EVIDENCE = "evidence:60000000-0000-4000-8000-000000000002";
const BASELINE_EVIDENCE = "evidence:60000000-0000-4000-8000-000000000003";
const PARTIAL_EVIDENCE = "evidence:60000000-0000-4000-8000-000000000004";
const PRIVATE_KEY_PAYLOADS = (() => {
  const rawKey = Buffer.alloc(32, 0xfb);
  const base64 = rawKey.toString("base64");
  const base64url = rawKey.toString("base64url");
  return [
    base64,
    base64.replace(/=+$/, ""),
    `${base64url}=`,
    base64url,
    "A".repeat(43),
    "B".repeat(44),
    "A".repeat(64),
    "_".repeat(43),
    "-".repeat(44),
    "_".repeat(64),
    rawKey.toString("hex"),
    "MC4CAQAwBQYDK2VwBCIEIAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA",
    Buffer.from("302e020100300506032b657004220420".padEnd(96, "0"), "hex").toString("latin1"),
    "-----BEGIN PRIVATE KEY-----\nabc\n-----END PRIVATE KEY-----"
  ];
})();

function clone(value) {
  return structuredClone(value);
}

function material(record) {
  const { record_digest: _digest, ...value } = record;
  return value;
}

function authorityFor(record) {
  const request = {
    schema: record.schema,
    subject: clone(record.subject),
    task_shape: clone(record.task_shape),
    routing_context: clone(record.routing_context),
    privacy: clone(record.privacy)
  };
  return createAssignmentAuthority(request, record.adapter_resolution.requested_provider);
}

function expectRouteError(fn, code) {
  assert.throws(fn, (error) => error instanceof RouteError && error.code === code);
}

function expectValueFreePrivacyError(fn, rejectedValue, label) {
  assert.throws(
    fn,
    (error) => error instanceof RouteError
      && error.code === "privacy_violation"
      && !error.message.includes(rejectedValue),
    label
  );
}

function completeRecord(record) {
  record.record_state = "completed";
  Object.assign(record.actual_run, {
    status: "completed",
    actual_provider: record.adapter_resolution.requested_provider,
    actual_model: record.adapter_resolution.implementer.model,
    actual_effort: record.adapter_resolution.implementer.effort,
    started_at: "2026-07-22T21:30:00Z",
    completed_at: "2026-07-22T21:40:00Z",
    input_tokens: 1000,
    output_tokens: 400,
    context_bytes: 8000,
    always_loaded_bytes: 4000,
    startup_to_first_correct_action_ms: 30000,
    elapsed_ms: 600000,
    review_wait_ms: 60000,
    telemetry_complete: true,
    implemented_evidence_refs: [IMPLEMENTED_EVIDENCE],
    cost: {
      amount: 1.25,
      currency: "USD",
      measurement_source: ACTUAL_COST_SOURCE,
      quality: "actual",
      measured_at: "2026-07-22T21:40:00Z"
    }
  });
  return record;
}

test("the executable proof passes exactly seven canonical and all negative cases", () => {
  const result = runSelfCheck();
  assert.equal(result.ok, true);
  assert.equal(result.canonical_cases.passed, 7);
  assert.equal(result.canonical_cases.total, 7);
  assert.equal(result.negative_cases.passed, cases.negative_cases.length);
  assert.equal(result.network_access, false);
  assert.equal(result.dispatch_performed, false);
});

test("route selection is byte-deterministic and provider-neutral before adapter resolution", () => {
  const codex = routeTask(clone(cases.base_task), "codex");
  const repeated = routeTask(clone(cases.base_task), "codex");
  const claude = routeTask(clone(cases.base_task), "claude");
  assert.equal(canonicalJson(codex), canonicalJson(repeated));
  assert.equal(codex.route_plan.implementer_profile, claude.route_plan.implementer_profile);
  assert.equal(codex.route_plan.reviewer_profile, claude.route_plan.reviewer_profile);
  assert.notEqual(codex.adapter_resolution.adapter_digest, claude.adapter_resolution.adapter_digest);
  assert.notEqual(codex.adapter_resolution.implementer.model, claude.adapter_resolution.implementer.model);
});

test("both provider adapters fail visibly when availability is false", () => {
  for (const provider of ["codex", "claude"]) {
    const request = clone(cases.base_task);
    request.routing_context.provider_available = false;
    expectRouteError(() => routeTask(request, provider), "provider_unavailable");
  }
});

test("schema closes the route record and every major nested contract", () => {
  assert.equal(schema.additionalProperties, false);
  for (const definition of ["subject", "taskShape", "actorRoute", "routingContext", "taskRequest", "instructionReference", "routePlan", "authorityRoutePlan", "adapterResolution", "assignmentAuthority", "resolvedProfile", "actualRun", "findings", "cost", "privacy"]) {
    assert.equal(schema.$defs[definition].additionalProperties, false, definition);
  }
  assert.equal(schema.properties.schema.const, "appfw_model_route@1");
  assert.equal(schema.$defs.id, undefined);
  assert.equal(schema.$defs.stringArray, undefined);
  assert.equal(schema.$defs.sourceBoundId, undefined);
  assert.equal(schema.$defs.sourceBoundIdArray, undefined);
  for (const definition of ["actorReference", "assignmentReference", "runReference", "taskReference", "routeReference", "evidenceReference"]) {
    assert.match(schema.$defs[definition].pattern, /^\^/);
  }
});

test("every machine identity uses its context-specific canonical authority reference", () => {
  const references = {
    actorReference: "actor:20000000-0000-4000-8000-000000000001",
    assignmentReference: "assignment:10000000-0000-4000-8000-000000000001",
    runReference: "run:30000000-0000-4000-8000-000000000001",
    taskReference: "task:40000000-0000-4000-8000-000000000001",
    routeReference: "route:50000000-0000-4000-8000-000000000001",
    evidenceReference: IMPLEMENTED_EVIDENCE
  };
  for (const [definition, reference] of Object.entries(references)) {
    const pattern = new RegExp(schema.$defs[definition].pattern);
    assert.equal(pattern.test(reference), true, definition);
    for (const [otherDefinition, otherReference] of Object.entries(references)) {
      if (otherDefinition !== definition) assert.equal(pattern.test(otherReference), false, `${definition}:${otherDefinition}`);
    }
  }

  for (const provider of ["codex", "claude"]) {
    const request = clone(cases.base_task);
    request.routing_context.strategic_decision = true;
    request.routing_context.human_decision_id = "evidence:50000000-0000-4000-8000-000000000001";
    const record = routeTask(request, provider);
    const authority = authorityFor(record);
    completeRecord(record);
    record.actual_run.accepted_evidence_refs = [ACCEPTED_EVIDENCE];
    record.actual_run.value_claim = {
      claimed: true,
      metric: "elapsed_time",
      improvement_percent: 10,
      baseline_evidence_refs: [BASELINE_EVIDENCE]
    };
    record.record_digest = digest(material(record));
    assert.equal(validateRouteRecord(record, authority), record);
  }
});

test("provider-neutral policy contains profiles but no raw model mapping", () => {
  const source = JSON.stringify(policy);
  assert.doesNotMatch(source, /"(?:model|model_id)"\s*:/);
  assert.deepEqual(policy.provider_neutral_profiles, [
    "bounded_read@1",
    "routine_delivery@1",
    "complex_delivery@1",
    "consequential_decision@1",
    "focused_independent_review@1",
    "comprehensive_independent_review@1",
    "adversarial_independent_review@1"
  ]);
});

test("a completed route joins actual economics to implementation and Accepted evidence", () => {
  const record = routeTask(clone(cases.base_task), "codex");
  const expectedAssignmentDigest = authorityFor(record);
  completeRecord(record);
  Object.assign(record.actual_run, {
    accepted_evidence_refs: [ACCEPTED_EVIDENCE],
    value_claim: {
      claimed: true,
      metric: "elapsed_time",
      improvement_percent: 16,
      baseline_evidence_refs: [BASELINE_EVIDENCE]
    }
  });
  record.record_digest = digest(material(record));
  assert.equal(validateRouteRecord(record, expectedAssignmentDigest), record);
});

test("PFC-held assignment authority rejects mutation after both public digests are recomputed", () => {
  const record = routeTask(clone(cases.base_task), "codex");
  const expectedAssignmentDigest = authorityFor(record);
  record.route_plan.rationale_codes = ["post-activation-change"];
  record.route_plan.assignment_digest = digest(material(record));
  record.record_digest = digest(material(record));
  expectRouteError(() => validateRouteRecord(record, expectedAssignmentDigest), "assignment_authority_mismatch");
  expectRouteError(() => validateRouteRecord(record), "assignment_authority_required");
  expectRouteError(() => validateRouteRecord(record, record.route_plan.assignment_digest), "assignment_authority_required");
});

test("external assignment authority binds provider adapter profile model effort and route identity", () => {
  for (const [authorizedProvider, substitutedProvider] of [["codex", "claude"], ["claude", "codex"]]) {
    const authorizedRecord = routeTask(clone(cases.base_task), authorizedProvider);
    const authority = authorityFor(authorizedRecord);
    const substituted = routeTask(clone(cases.base_task), substitutedProvider);
    substituted.route_plan.assignment_digest = authorizedRecord.route_plan.assignment_digest;
    substituted.record_digest = digest(material(substituted));
    expectRouteError(() => validateRouteRecord(substituted, authority), "assignment_authority_mismatch");
  }

  const mutations = [
    ["route-decision", (record) => { record.route_decision_id = "route:50000000-0000-4000-8000-000000000099"; }],
    ["adapter-id", (record) => { record.adapter_resolution.adapter_id = "codex-local@2"; }],
    ["adapter-version", (record) => { record.adapter_resolution.adapter_version = "1.0.1"; }],
    ["adapter-digest", (record) => { record.adapter_resolution.adapter_digest = "a".repeat(64); }],
    ["implementer-profile", (record) => {
      record.adapter_resolution.implementer = clone(record.adapter_resolution.reviewer);
    }],
    ["implementer-model", (record) => { record.adapter_resolution.implementer.model = "gpt-5.5"; }],
    ["implementer-effort", (record) => { record.adapter_resolution.implementer.effort = "xhigh"; }],
    ["implementer-capabilities", (record) => { record.adapter_resolution.implementer.capabilities = ["code_edit"]; }],
    ["reviewer-model", (record) => { record.adapter_resolution.reviewer.model = "gpt-5.5"; }]
  ];
  for (const [label, mutate] of mutations) {
    const record = routeTask(clone(cases.base_task), "codex");
    const authority = authorityFor(record);
    mutate(record);
    record.route_plan.assignment_digest = digest({ self_authorized_record: material(record) });
    record.record_digest = digest(material(record));
    assert.throws(
      () => validateRouteRecord(record, authority),
      (error) => error instanceof RouteError && error.code === "assignment_authority_mismatch",
      label
    );
  }
});

test("completed telemetry cannot silently change provider model or effort", () => {
  const record = routeTask(clone(cases.base_task), "codex");
  const expectedAssignmentDigest = authorityFor(record);
  completeRecord(record);
  record.actual_run.actual_model = "unapproved-model";
  record.record_digest = digest(material(record));
  expectRouteError(() => validateRouteRecord(record, expectedAssignmentDigest), "silent_reroute");
});

test("running and failed telemetry cannot partially record or change the approved route", () => {
  for (const status of ["running", "failed"]) {
    for (const field of ["actual_provider", "actual_model", "actual_effort"]) {
      const record = routeTask(clone(cases.base_task), "codex");
      const expectedAssignmentDigest = authorityFor(record);
      record.record_state = status;
      Object.assign(record.actual_run, {
        status,
        actual_provider: record.adapter_resolution.requested_provider,
        actual_model: record.adapter_resolution.implementer.model,
        actual_effort: record.adapter_resolution.implementer.effort,
        started_at: "2026-07-22T21:30:00Z",
        completed_at: status === "failed" ? "2026-07-22T21:40:00Z" : null,
        telemetry_complete: status === "failed"
      });
      record.actual_run[field] = `unapproved-${field}`;
      record.record_digest = digest(material(record));
      expectRouteError(() => validateRouteRecord(record, expectedAssignmentDigest), "silent_reroute");
    }
  }

  const partial = routeTask(clone(cases.base_task), "codex");
  const partialAuthority = authorityFor(partial);
  partial.record_state = "failed";
  Object.assign(partial.actual_run, {
    status: "failed",
    actual_provider: partial.adapter_resolution.requested_provider,
    started_at: "2026-07-22T21:30:00Z",
    completed_at: "2026-07-22T21:40:00Z",
    telemetry_complete: true
  });
  partial.record_digest = digest(material(partial));
  expectRouteError(() => validateRouteRecord(partial, partialAuthority), "telemetry_incomplete");

  const preDispatchFailure = routeTask(clone(cases.base_task), "codex");
  const failureAuthority = authorityFor(preDispatchFailure);
  preDispatchFailure.record_state = "failed";
  Object.assign(preDispatchFailure.actual_run, {
    status: "failed",
    telemetry_complete: true,
    findings: { blocker: 0, critical: 0, important: 1, should_address: 0, nice_to_address: 0 }
  });
  preDispatchFailure.record_digest = digest(material(preDispatchFailure));
  assert.equal(validateRouteRecord(preDispatchFailure, failureAuthority), preDispatchFailure);
});

test("all-null not-started and failed routes reject every kind of execution evidence", () => {
  const measuredMutations = [
    ["started_at", (run) => { run.started_at = "2026-07-22T21:30:00Z"; }],
    ["completed_at", (run) => { run.completed_at = "2026-07-22T21:31:00Z"; }],
    ["input_tokens", (run) => { run.input_tokens = 1; }],
    ["output_tokens", (run) => { run.output_tokens = 1; }],
    ["context_bytes", (run) => { run.context_bytes = 1; }],
    ["always_loaded_bytes", (run) => { run.always_loaded_bytes = 1; }],
    ["startup_to_first_correct_action_ms", (run) => { run.startup_to_first_correct_action_ms = 1; }],
    ["elapsed_ms", (run) => { run.elapsed_ms = 1; }],
    ["review_wait_ms", (run) => { run.review_wait_ms = 1; }],
    ["retries", (run) => { run.retries = 1; }],
    ["material_human_corrections", (run) => { run.material_human_corrections = 1; }],
    ["rework_events", (run) => { run.rework_events = 1; }],
    ["escaped_defect_count", (run) => { run.escaped_defect_count = 1; }],
    ["cost", (run) => {
      run.cost = { amount: 0.5, currency: "USD", measurement_source: ACTUAL_COST_SOURCE, quality: "actual", measured_at: "2026-07-22T21:31:00Z" };
    }],
    ["implemented_evidence_refs", (run) => { run.implemented_evidence_refs = [IMPLEMENTED_EVIDENCE]; }],
    ["accepted_evidence_refs", (run) => { run.accepted_evidence_refs = [ACCEPTED_EVIDENCE]; }],
    ["value_claim", (run) => {
      run.value_claim = { claimed: true, metric: "elapsed_time", improvement_percent: 1, baseline_evidence_refs: [BASELINE_EVIDENCE] };
    }]
  ];

  for (const provider of ["codex", "claude"]) {
    for (const status of ["not_started", "failed"]) {
      for (const [field, mutate] of measuredMutations) {
        const record = routeTask(clone(cases.base_task), provider);
        const authority = authorityFor(record);
        if (status === "failed") {
          record.record_state = "failed";
          record.actual_run.status = "failed";
          record.actual_run.telemetry_complete = true;
        }
        mutate(record.actual_run);
        record.record_digest = digest(material(record));
        const expectedCode = field === "accepted_evidence_refs"
          ? "accepted_evidence_incomplete"
          : field === "value_claim"
            ? "value_claim_without_accepted_outcome"
            : "telemetry_without_actual_route";
        assert.throws(
          () => validateRouteRecord(record, authority),
          (error) => error instanceof RouteError && error.code === expectedCode,
          `${provider}:${status}:${field}`
        );
      }
    }

    for (const mutate of [
      (run) => { run.telemetry_complete = true; },
      (run) => { run.findings.important = 1; }
    ]) {
      const record = routeTask(clone(cases.base_task), provider);
      const authority = authorityFor(record);
      mutate(record.actual_run);
      record.record_digest = digest(material(record));
      expectRouteError(() => validateRouteRecord(record, authority), "telemetry_without_actual_route");
    }
  }
});

test("post-dispatch failure retains truthful partial implementation telemetry only on the approved route", () => {
  for (const provider of ["codex", "claude"]) {
    const record = routeTask(clone(cases.base_task), provider);
    const authority = authorityFor(record);
    record.record_state = "failed";
    Object.assign(record.actual_run, {
      status: "failed",
      actual_provider: record.adapter_resolution.requested_provider,
      actual_model: record.adapter_resolution.implementer.model,
      actual_effort: record.adapter_resolution.implementer.effort,
      started_at: "2026-07-22T21:30:00Z",
      completed_at: "2026-07-22T21:31:00Z",
      input_tokens: 250,
      output_tokens: 20,
      context_bytes: 2048,
      always_loaded_bytes: 1024,
      elapsed_ms: 60000,
      retries: 1,
      telemetry_complete: true,
      implemented_evidence_refs: [PARTIAL_EVIDENCE],
      findings: { blocker: 0, critical: 1, important: 0, should_address: 0, nice_to_address: 0 },
      cost: { amount: 0.2, currency: "USD", measurement_source: ACTUAL_COST_SOURCE, quality: "actual", measured_at: "2026-07-22T21:31:00Z" }
    });
    record.record_digest = digest(material(record));
    assert.equal(validateRouteRecord(record, authority), record);
  }
});

test("only completed runs can retain Accepted evidence or positive value claims", () => {
  for (const status of ["running", "failed"]) {
    const accepted = routeTask(clone(cases.base_task), "codex");
    const authority = authorityFor(accepted);
    accepted.record_state = status;
    Object.assign(accepted.actual_run, {
      status,
      actual_provider: accepted.adapter_resolution.requested_provider,
      actual_model: accepted.adapter_resolution.implementer.model,
      actual_effort: accepted.adapter_resolution.implementer.effort,
      started_at: "2026-07-22T21:30:00Z",
      completed_at: status === "failed" ? "2026-07-22T21:40:00Z" : null,
      telemetry_complete: status === "failed",
      implemented_evidence_refs: [IMPLEMENTED_EVIDENCE],
      accepted_evidence_refs: [ACCEPTED_EVIDENCE]
    });
    accepted.record_digest = digest(material(accepted));
    expectRouteError(() => validateRouteRecord(accepted, authority), "accepted_evidence_incomplete");

    accepted.actual_run.accepted_evidence_refs = [];
    accepted.actual_run.value_claim = {
      claimed: true,
      metric: "elapsed_time",
      improvement_percent: 10,
      baseline_evidence_refs: [BASELINE_EVIDENCE]
    };
    accepted.record_digest = digest(material(accepted));
    expectRouteError(() => validateRouteRecord(accepted, authority), "value_claim_without_accepted_outcome");
  }
});

test("cost provenance uses canonical authority UUIDs and rejects encoded payloads without echo", () => {
  const rawKey = Buffer.alloc(32, 0xfb);
  const compactPkcs8Ed25519 = "MC4CAQAwBQYDK2VwBCIEIAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA";
  const paddedBase64 = rawKey.toString("base64");
  const paddedBase64Url = paddedBase64.replaceAll("+", "-").replaceAll("/", "_");
  const encodedPayloads = new Set([
    compactPkcs8Ed25519,
    Buffer.from(compactPkcs8Ed25519, "base64").toString("base64url"),
    paddedBase64,
    paddedBase64.replace(/=+$/, ""),
    paddedBase64Url,
    rawKey.toString("base64url"),
    rawKey.toString("hex"),
    "A".repeat(43),
    "A".repeat(44),
    "A".repeat(64)
  ]);
  const sources = [
    "-----BEGIN ENCRYPTED PRIVATE KEY-----\nabc\n-----END ENCRYPTED PRIVATE KEY-----",
    "-----BEGIN PRIVATE KEY-----\nabc\n-----END PRIVATE KEY-----",
    "-----BEGIN RSA PRIVATE KEY-----\nabc\n-----END RSA PRIVATE KEY-----",
    "-----BEGIN EC PRIVATE KEY-----\nabc\n-----END EC PRIVATE KEY-----",
    "-----BEGIN OPENSSH PRIVATE KEY-----\nabc\n-----END OPENSSH PRIVATE KEY-----",
    "-----BEGIN PGP PRIVATE KEY BLOCK-----\nabc\n-----END PGP PRIVATE KEY BLOCK-----",
    "A".repeat(160),
    `${"A".repeat(64)}\n${"B".repeat(64)}\n${"C".repeat(32)}`,
    JSON.stringify({ raw: "payload" })
  ];
  for (const payload of encodedPayloads) {
    for (const prefix of ["provider_measurement:", "declared_rate:"]) sources.push(`${prefix}${payload}`);
  }
  for (const source of sources) {
    const record = routeTask(clone(cases.base_task), "codex");
    const authority = authorityFor(record);
    completeRecord(record);
    record.actual_run.cost.measurement_source = source;
    record.record_digest = digest(material(record));
    assert.throws(
      () => validateRouteRecord(record, authority),
      (error) => error instanceof RouteError
        && ["privacy_violation", "invalid_schema", "invalid_cost_evidence"].includes(error.code)
        && !error.message.includes(source)
    );
  }

  for (const [quality, measurementSource, expectedError] of [
    ["actual", ESTIMATED_COST_SOURCE, "invalid_cost_evidence"],
    ["estimated", ACTUAL_COST_SOURCE, "invalid_cost_evidence"],
    ["actual", "provider_measurement:pm_not-a-uuid", "invalid_schema"],
    ["actual", "provider_measurement:pm_123E4567-E89B-42D3-A456-426614174000", "invalid_schema"],
    ["actual", "free text source", "invalid_schema"]
  ]) {
    const record = routeTask(clone(cases.base_task), "codex");
    const authority = authorityFor(record);
    completeRecord(record);
    record.actual_run.cost.quality = quality;
    record.actual_run.cost.measurement_source = measurementSource;
    record.record_digest = digest(material(record));
    expectRouteError(() => validateRouteRecord(record, authority), expectedError);
  }

  for (const provider of ["codex", "claude"]) {
    for (const [quality, measurementSource] of [
      ["actual", ACTUAL_COST_SOURCE],
      ["estimated", ESTIMATED_COST_SOURCE]
    ]) {
      const record = routeTask(clone(cases.base_task), provider);
      const authority = authorityFor(record);
      completeRecord(record);
      record.actual_run.cost.quality = quality;
      record.actual_run.cost.measurement_source = measurementSource;
      record.record_digest = digest(material(record));
      assert.equal(validateRouteRecord(record, authority), record);
    }
  }
});

test("temporal economics distinguish actual measurements from declared rates", () => {
  for (const provider of ["codex", "claude"]) {
    for (const measuredAt of ["2026-07-22T21:29:59Z", "2026-07-22T21:40:01Z"]) {
      const record = routeTask(clone(cases.base_task), provider);
      const authority = authorityFor(record);
      completeRecord(record);
      record.actual_run.cost.measured_at = measuredAt;
      record.record_digest = digest(material(record));
      expectRouteError(() => validateRouteRecord(record, authority), "invalid_timestamp_order");
    }

    const declaredRate = routeTask(clone(cases.base_task), provider);
    const declaredAuthority = authorityFor(declaredRate);
    completeRecord(declaredRate);
    Object.assign(declaredRate.actual_run.cost, {
      quality: "estimated",
      measurement_source: ESTIMATED_COST_SOURCE,
      measured_at: "2026-07-01T00:00:00Z"
    });
    declaredRate.record_digest = digest(material(declaredRate));
    assert.equal(validateRouteRecord(declaredRate, declaredAuthority), declaredRate);

    declaredRate.actual_run.cost.measured_at = "2026-07-22T21:40:01Z";
    declaredRate.record_digest = digest(material(declaredRate));
    expectRouteError(() => validateRouteRecord(declaredRate, declaredAuthority), "invalid_timestamp_order");

    for (const mutate of [
      (run) => { run.startup_to_first_correct_action_ms = run.elapsed_ms + 1; },
      (run) => { run.review_wait_ms = run.elapsed_ms + 1; },
      (run) => { run.elapsed_ms = 600001; }
    ]) {
      const record = routeTask(clone(cases.base_task), provider);
      const authority = authorityFor(record);
      completeRecord(record);
      mutate(record.actual_run);
      record.record_digest = digest(material(record));
      expectRouteError(() => validateRouteRecord(record, authority), "invalid_duration");
    }
  }
});

test("caller state cannot suppress review for writable or material work", () => {
  const noReview = clone(cases.base_task);
  noReview.routing_context.reviewer_required = false;
  noReview.routing_context.reviewer = null;
  expectRouteError(() => routeTask(noReview, "codex"), "implicit_child_inheritance");

  const writableRoot = clone(cases.base_task);
  writableRoot.task_shape.material_child = false;
  writableRoot.subject.parent_run_id = null;
  writableRoot.task_shape.delegation_depth = 0;
  writableRoot.task_shape.delegation_type = "none";
  writableRoot.routing_context.reviewer_required = false;
  writableRoot.routing_context.reviewer = null;
  expectRouteError(() => routeTask(writableRoot, "codex"), "review_requirement_downgrade");
});

test("overlapping risks combine monotonically instead of accepting the first matching rule", () => {
  const combinations = [
    {
      id: "strategic-sensitive",
      patch(request) {
        request.routing_context.strategic_decision = true;
        request.task_shape.consequence = "irreversible";
        request.task_shape.sensitive_surfaces = ["auth"];
      },
      implementer: "consequential_decision@1",
      reviewer: "adversarial_independent_review@1"
    },
    {
      id: "signature-sensitive",
      patch(request) {
        request.task_shape.signature_significance = true;
        request.task_shape.sensitive_surfaces = ["runtime"];
      },
      implementer: "complex_delivery@1",
      reviewer: "adversarial_independent_review@1"
    },
    {
      id: "class-a-strategic",
      patch(request) {
        request.task_shape.change_class = "A";
        request.routing_context.strategic_decision = true;
      },
      implementer: "consequential_decision@1",
      reviewer: "comprehensive_independent_review@1"
    },
    {
      id: "multi-risk",
      patch(request) {
        request.task_shape.change_class = "A";
        request.task_shape.signature_significance = true;
        request.task_shape.consequence = "irreversible";
        request.task_shape.sensitive_surfaces = ["auth", "runtime"];
        request.routing_context.strategic_decision = true;
      },
      implementer: "consequential_decision@1",
      reviewer: "adversarial_independent_review@1"
    }
  ];

  for (const combination of combinations) {
    const request = clone(cases.base_task);
    combination.patch(request);
    if (request.task_shape.sensitive_surfaces.length > 0) {
      request.task_shape.data_classification = "restricted";
      request.privacy.data_classification = "restricted";
    }
    request.routing_context.human_decision_id = "evidence:70000000-0000-4000-8000-000000000001";
    for (const provider of ["codex", "claude"]) {
      const record = routeTask(request, provider);
      assert.equal(record.route_plan.implementer_profile, combination.implementer, `${provider}:${combination.id}`);
      assert.equal(record.route_plan.reviewer_profile, combination.reviewer, `${provider}:${combination.id}`);
      assert.equal(record.route_plan.requires_human_decision, true, `${provider}:${combination.id}`);
      assert.ok(!record.route_plan.matched_rule_ids.includes("routine-delivery"), `${provider}:${combination.id}`);
      if (combination.id.includes("strategic") || combination.id === "multi-risk") {
        assert.ok(adapters[provider].profiles[record.route_plan.reviewer_profile].capabilities.includes("strategic_challenge"));
      }
    }
  }
});

test("adapter fallback profiles cannot weaken selected or required capabilities", () => {
  for (const provider of ["codex", "claude"]) {
    const adapter = clone(adapters[provider]);
    adapter.profiles["comprehensive_independent_review@1"].fallback_profile = "adversarial_independent_review@1";
    adapter.profiles["adversarial_independent_review@1"].capabilities = ["independent_review", "sensitive_boundary"];
    expectRouteError(() => validateAdapter(adapter, provider, policy), "invalid_adapter");
  }
});

test("integration and release stages retain independent-review floors from subject.stage", () => {
  const otherwiseRoutine = clone(cases.base_task);
  otherwiseRoutine.task_shape.write_roots = [];
  otherwiseRoutine.task_shape.material_child = false;
  otherwiseRoutine.task_shape.change_class = "D";
  assert.equal(independentReviewRequired(otherwiseRoutine, { reviewer_profile: null }), false);

  for (const stage of ["integration_candidate", "release_candidate"]) {
    const request = clone(cases.base_task);
    request.subject.stage = stage;
    request.subject.parent_run_id = null;
    request.task_shape.write_roots = [];
    request.task_shape.material_child = false;
    request.task_shape.delegation_depth = 0;
    request.task_shape.delegation_type = "none";
    request.routing_context.authorized_write_roots = [];
    request.routing_context.reviewer_required = false;
    request.routing_context.reviewer = null;
    assert.equal(independentReviewRequired(request, { reviewer_profile: null }), true);
    expectRouteError(() => routeTask(request, "codex"), "review_requirement_downgrade");
  }
});

test("closed schema enforces primitive types, timestamps, and lifecycle semantics", () => {
  const malformed = routeTask(clone(cases.base_task), "codex");
  const malformedAuthority = authorityFor(malformed);
  malformed.actual_run.telemetry_complete = "yes";
  malformed.record_digest = digest(material(malformed));
  expectRouteError(() => validateRouteRecord(malformed, malformedAuthority), "invalid_schema");

  const timestamp = routeTask(clone(cases.base_task), "codex");
  const timestampAuthority = authorityFor(timestamp);
  completeRecord(timestamp);
  timestamp.actual_run.completed_at = "not-a-timestamp";
  timestamp.record_digest = digest(material(timestamp));
  expectRouteError(() => validateRouteRecord(timestamp, timestampAuthority), "invalid_timestamp");

  const ordering = routeTask(clone(cases.base_task), "codex");
  const orderingAuthority = authorityFor(ordering);
  completeRecord(ordering);
  ordering.actual_run.completed_at = "2026-07-22T21:20:00Z";
  ordering.actual_run.cost.measured_at = "2026-07-22T21:20:00Z";
  ordering.record_digest = digest(material(ordering));
  expectRouteError(() => validateRouteRecord(ordering, orderingAuthority), "invalid_timestamp_order");
});

test("provider fields and sensitive metadata fail before routing", () => {
  const providerLeak = clone(cases.base_task);
  providerLeak.task_shape.model = "hidden-model";
  expectRouteError(() => routeTask(providerLeak, "codex"), "provider_leakage");

  const sensitive = clone(cases.base_task);
  sensitive.task_shape.read_roots = ["/Users/example/private-source"];
  sensitive.routing_context.authorized_read_roots = ["/Users/example/private-source"];
  expectRouteError(() => routeTask(sensitive, "codex"), "privacy_violation");

  for (const actorId of ["person@example.com", "ghp_123456789012345678901234567890123456", "patient-12345678", "949-555-0123"]) {
    const request = clone(cases.base_task);
    request.routing_context.implementer.actor_id = actorId;
    expectRouteError(() => routeTask(request, "codex"), "privacy_violation");
  }

  const traversal = clone(cases.base_task);
  traversal.task_shape.read_roots = ["docs/../private"];
  traversal.routing_context.authorized_read_roots = ["docs/../private"];
  expectRouteError(() => routeTask(traversal, "codex"), "privacy_violation");
});

test("recursive privacy rejects compact key material in every caller-controlled identifier for both adapters", () => {
  const requestMutators = [
    ["subject.outcome_id", (request, value) => { request.subject.outcome_id = value; }],
    ["subject.product_increment_id", (request, value) => { request.subject.product_increment_id = value; }],
    ["subject.deliverable_id", (request, value) => { request.subject.deliverable_id = value; }],
    ["subject.task_id", (request, value) => { request.subject.task_id = value; }],
    ["subject.assignment_id", (request, value) => { request.subject.assignment_id = value; }],
    ["subject.workstream_id", (request, value) => { request.subject.workstream_id = value; }],
    ["subject.role_card_id", (request, value) => { request.subject.role_card_id = value; }],
    ["subject.parent_run_id", (request, value) => { request.subject.parent_run_id = value; }],
    ["implementer.assignment_id", (request, value) => { request.routing_context.implementer.assignment_id = value; }],
    ["implementer.actor_id", (request, value) => { request.routing_context.implementer.actor_id = value; }],
    ["implementer.run_id", (request, value) => { request.routing_context.implementer.run_id = value; }],
    ["reviewer.assignment_id", (request, value) => { request.routing_context.reviewer.assignment_id = value; }],
    ["reviewer.actor_id", (request, value) => { request.routing_context.reviewer.actor_id = value; }],
    ["reviewer.run_id", (request, value) => { request.routing_context.reviewer.run_id = value; }],
    ["requested_capabilities", (request, value) => { request.task_shape.requested_capabilities = [value]; }],
    ["allowed_tools", (request, value) => { request.task_shape.allowed_tools = [value]; }],
    ["required_instruction_ids", (request, value) => { request.routing_context.required_instruction_ids = [value]; }],
    ["authorized_tools", (request, value) => { request.routing_context.authorized_tools = [value]; }],
    ["human_decision_id", (request, value) => { request.routing_context.human_decision_id = value; }]
  ];

  for (const provider of ["codex", "claude"]) {
    for (const [label, mutate] of requestMutators) {
      for (const rejectedValue of PRIVATE_KEY_PAYLOADS) {
        const request = clone(cases.base_task);
        mutate(request, rejectedValue);
        expectValueFreePrivacyError(() => routeTask(request, provider), rejectedValue, `${provider}:${label}`);
      }
    }

    const recordMutators = [
      ["route_decision_id", (record, value) => { record.route_decision_id = value; }],
      ["implemented_evidence_refs", (record, value) => { record.actual_run.implemented_evidence_refs = [value]; }],
      ["accepted_evidence_refs", (record, value) => { record.actual_run.accepted_evidence_refs = [value]; }],
      ["baseline_evidence_refs", (record, value) => { record.actual_run.value_claim.baseline_evidence_refs = [value]; }]
    ];
    const baseRecord = routeTask(clone(cases.base_task), provider);
    for (const [label, mutate] of recordMutators) {
      for (const rejectedValue of PRIVATE_KEY_PAYLOADS) {
        const record = clone(baseRecord);
        const authority = authorityFor(record);
        mutate(record, rejectedValue);
        record.record_digest = digest(material(record));
        expectValueFreePrivacyError(() => validateRouteRecord(record, authority), rejectedValue, `${provider}:${label}`);
      }
    }
  }
});

test("recursive privacy covers tracked policy and adapter keys and values", () => {
  for (const rejectedValue of PRIVATE_KEY_PAYLOADS) {
    const policyValue = clone(policy);
    policyValue.instructions[0].id = rejectedValue;
    expectValueFreePrivacyError(() => validatePolicy(policyValue), rejectedValue, "policy:instruction-id");

    for (const provider of ["codex", "claude"]) {
      const adapterValue = clone(adapters[provider]);
      adapterValue.adapter_id = rejectedValue;
      expectValueFreePrivacyError(() => validateAdapter(adapterValue, provider, policy), rejectedValue, `${provider}:adapter-id`);

      const adapterKey = clone(adapters[provider]);
      adapterKey.profiles[rejectedValue] = adapterKey.profiles["bounded_read@1"];
      expectValueFreePrivacyError(() => validateAdapter(adapterKey, provider, policy), rejectedValue, `${provider}:profile-key`);
    }
  }
});

test("whitespace-wrapped private keys fail closed across both adapters and retained resolutions", () => {
  const compact = "MC4CAQAwBQYDK2VwBCIEIAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA";
  const encodings = [
    compact,
    Buffer.from(compact, "base64").toString("base64url"),
    `${Buffer.from(compact, "base64").toString("base64url")}==`
  ];
  const wrapped = new Set();
  for (const value of encodings) {
    for (const interval of [4, 7, 11, 19]) {
      wrapped.add(value.match(new RegExp(`.{1,${interval}}`, "g")).join(interval % 2 === 0 ? " " : "\n"));
    }
  }

  for (const provider of ["codex", "claude"]) {
    for (const rejectedValue of wrapped) {
      const adapter = clone(adapters[provider]);
      adapter.profiles["routine_delivery@1"].model = rejectedValue;
      expectValueFreePrivacyError(() => validateAdapter(adapter, provider, policy), rejectedValue, `${provider}:adapter-model`);

      const record = routeTask(clone(cases.base_task), provider);
      const authority = authorityFor(record);
      record.adapter_resolution.implementer.model = rejectedValue;
      record.record_digest = digest(material(record));
      expectValueFreePrivacyError(() => validateRouteRecord(record, authority), rejectedValue, `${provider}:resolved-model`);
    }
  }
});

test("tracked policy privacy is a closed metadata-only contract", () => {
  for (const mutate of [
    (value) => { value.privacy.prompt_content_retained = true; },
    (value) => { value.privacy.secret_content_retained = true; },
    (value) => { value.privacy.prompt_content_retained = "false"; },
    (value) => { value.privacy.retained_field_allowlist_version = "allowlist@2"; },
    (value) => { value.privacy.unapproved = false; }
  ]) {
    const candidate = clone(policy);
    mutate(candidate);
    assert.throws(
      () => validatePolicy(candidate),
      (error) => error instanceof RouteError && ["invalid_policy", "unknown_field"].includes(error.code)
    );
  }

  for (const provider of ["codex", "claude"]) {
    const candidate = clone(adapters[provider]);
    candidate.profiles["routine_delivery@1"].model = "arbitrary-model";
    expectRouteError(() => validateAdapter(candidate, provider, policy), "invalid_adapter");
  }
});

test("PFC-held authority rejects source-bound and tracked identifier substitution after internal rehash", () => {
  const mutations = [
    ["business-subject", (record) => { record.subject.outcome_id = "AF-OG02"; }],
    ["machine-task", (record) => { record.subject.task_id = "task:40000000-0000-4000-8000-000000000099"; }],
    ["assignment", (record) => {
      const replacement = "assignment:10000000-0000-4000-8000-000000000099";
      record.subject.assignment_id = replacement;
      record.routing_context.implementer.assignment_id = replacement;
    }],
    ["actor", (record) => { record.routing_context.implementer.actor_id = "actor:20000000-0000-4000-8000-000000000099"; }],
    ["required-instruction", (record) => { record.routing_context.required_instruction_ids = ["role-card"]; }],
    ["tracked-rationale", (record) => { record.route_plan.rationale_codes = ["post_activation_substitution"]; }]
  ];

  for (const provider of ["codex", "claude"]) {
    for (const [label, mutate] of mutations) {
      const record = routeTask(clone(cases.base_task), provider);
      const retainedAuthority = authorityFor(record);
      mutate(record);
      record.route_plan.assignment_digest = digest(material(record));
      record.record_digest = digest(material(record));
      assert.throws(
        () => validateRouteRecord(record, retainedAuthority),
        (error) => error instanceof RouteError && error.code === "assignment_authority_mismatch",
        `${provider}:${label}`
      );
    }
  }
});

test("CLI emits one redacted JSON error and a nonzero status", () => {
  const directory = mkdtempSync(join(tmpdir(), "appfw-route-test-"));
  const input = join(directory, "task.json");
  const request = clone(cases.base_task);
  request.task_shape.read_roots = ["/Users/example/do-not-echo"];
  request.routing_context.authorized_read_roots = ["/Users/example/do-not-echo"];
  writeFileSync(input, JSON.stringify(request));
  const result = spawnSync(process.execPath, [cli.pathname, "--task", input, "--provider", "codex", "--json"], { encoding: "utf8" });
  assert.equal(result.status, 1);
  assert.equal(result.stderr, "");
  const lines = result.stdout.trim().split("\n");
  assert.equal(lines.length, 1);
  assert.equal(JSON.parse(lines[0]).ok, false);
  assert.equal(JSON.parse(lines[0]).error.code, "privacy_violation");
  assert.doesNotMatch(result.stdout, /do-not-echo|\/Users\/example/);
});

test("CLI emits the PFC authority envelope separately from the mutable route record", () => {
  const directory = mkdtempSync(join(tmpdir(), "appfw-route-output-test-"));
  const input = join(directory, "task.json");
  writeFileSync(input, JSON.stringify(cases.base_task));
  const result = spawnSync(process.execPath, [cli.pathname, "--task", input, "--provider", "codex", "--json"], { encoding: "utf8" });
  assert.equal(result.status, 0);
  const output = JSON.parse(result.stdout);
  assert.equal(output.ok, true);
  assert.equal(output.assignment_authority.schema, "assignment_authority@1");
  assert.equal(output.route.route_plan.assignment_digest, digest(output.assignment_authority));
  assert.equal("assignment_authority" in output.route, false);
});

test("CLI record validation requires and verifies PFC-held assignment authority", () => {
  const directory = mkdtempSync(join(tmpdir(), "appfw-route-authority-test-"));
  const input = join(directory, "route.json");
  const authorityInput = join(directory, "authority.json");
  const mismatchInput = join(directory, "mismatch-authority.json");
  const record = routeTask(clone(cases.base_task), "codex");
  const authority = authorityFor(record);
  writeFileSync(input, JSON.stringify(record));
  writeFileSync(authorityInput, JSON.stringify(authority));
  writeFileSync(mismatchInput, JSON.stringify(createAssignmentAuthority(clone(cases.base_task), "claude")));

  const missing = spawnSync(process.execPath, [cli.pathname, "--validate-record", input, "--json"], { encoding: "utf8" });
  assert.equal(missing.status, 1);
  assert.equal(JSON.parse(missing.stdout).error.code, "invalid_arguments");

  const mismatch = spawnSync(process.execPath, [cli.pathname, "--validate-record", input, "--assignment-authority", mismatchInput, "--json"], { encoding: "utf8" });
  assert.equal(mismatch.status, 1);
  assert.equal(JSON.parse(mismatch.stdout).error.code, "assignment_authority_mismatch");

  const valid = spawnSync(process.execPath, [cli.pathname, "--validate-record", input, "--assignment-authority", authorityInput, "--json"], { encoding: "utf8" });
  assert.equal(valid.status, 0);
  assert.equal(JSON.parse(valid.stdout).ok, true);
});

test("canonical JSON sorts object keys while preserving array order", () => {
  assert.equal(canonicalJson({ z: 1, a: [3, 2, 1], nested: { y: true, b: null } }), '{"a":[3,2,1],"nested":{"b":null,"y":true},"z":1}');
});
