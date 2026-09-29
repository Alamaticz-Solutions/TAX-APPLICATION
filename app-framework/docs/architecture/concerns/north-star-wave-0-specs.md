# Wave 0 Spec Artifact Contracts

> **Status: active Wave 0 contract.** This page turns the Roadmap's needs-spec
> queue into artifact-shaped contracts. It is intentionally compact: it names
> the future artifact, the minimum schema, and the first failing assertion each
> lane must add before implementation work claims progress.

The Roadmap is still the current priority source. This page answers a narrower
question: *what exactly must a later lane prove, and where does that proof live?*

## Spec Done Rule

A spec is complete only when:

1. The lane has a named source contract and retained JSON evidence artifact.
2. A `docs-check` or `release-evidence-check.sh` assertion fails if the artifact
   contract is removed.
3. The command is namespaced as `scripts/appfw product ...` for product work or
   `scripts/appfw framework ...` for framework stewardship.
4. The artifact records `ok`, `generated_at`, `inputs`, `checks`, and any
   `risk_acceptance` or `not_applicable` decision explicitly.

## Evidence Registry Freeze

Wave 0 freezes the evidence registries that later work extends:

| Registry | Current Source | Extension Rule |
| --- | --- | --- |
| Release evidence | `scripts/ci/release-evidence-check.sh` | Add new release artifact categories here; do not create a parallel release gate. |
| Security assurance | `scripts/ci/security-assurance-decision.sh` | Use the existing DAST, SAST, ASVS, release-provenance, and artifact-signing decision model for G4. |
| PDS component evidence | `scripts/check-pds-components.mjs`, `appfw_ui/pds_health/reference/catalog.json` | Add conversational/action UI components to the source catalog and component checker together. |
| PDS token evidence | `scripts/check-pds-tokens.mjs`, `appfw_ui/pds_health/tokens` | Product/mobile token bridges consume from source and must not fork token literals. |
| CLI command evidence | `scripts/appfw`, `docs/reference/cli.md`, `scripts/check-doc-examples.sh` | Reserved commands start as `--plan --json`; executable commands must retain JSON artifacts. |

## G1 Delegated Auth And Governed Write

**Command slot:** posture through `scripts/appfw framework governed-write-check --json`; provider-specific proof through `scripts/appfw framework provider-test --provider <provider> --json` plus graduation summary through `scripts/appfw framework provider-graduation --json`. External API governed-write work starts with `scripts/appfw framework provider-test --provider servicenow --area governed-write --plan --json`, then graduates to a provider-backed live run that may retain the evidence artifact below.

**Retained artifacts:** `target/appfw/governed-write-posture.json` for the
fail-closed posture report; `target/appfw/governed-write-evidence.json` only
for real G1 provider-test/live evidence.

**Minimum schema:**

```json
{
  "command": "provider-test",
  "lane": "G1",
  "ok": true,
  "release_ready": true,
  "provider": "servicenow",
  "operation": "named_mutation",
  "redaction": {
    "secrets_removed": true,
    "raw_payloads_removed": true,
    "tenant_data_removed": true
  },
  "delegated_actor_context": {
    "principal_type": "user",
    "tenant": "tenant-id",
    "on_behalf_of": "user-id"
  },
  "token_store_isolation": {
    "partition_key": ["user", "tenant", "provider"],
    "revocation_checked": true
  },
  "mutation_registry": {
    "name": "servicenow.create_incident",
    "mcp_enabled": false,
    "policy_scope": "servicenow.incident.write"
  },
  "request_binding": {
    "provider_owned_request": true,
    "raw_url_or_query_from_caller": false
  },
  "idempotency": {
    "key_source": "request",
    "replay_rejected": true
  },
  "policy": {
    "scope_enforced": true,
    "decision": "allow"
  },
  "audit": {
    "source": "http|mcp|kafka",
    "correlation_id": "required",
    "provider_request_id": "redacted-or-hash",
    "sink": "required-audit-sink"
  },
  "live_provider_call": {
    "executed": true,
    "non_production": true,
    "mutation_name": "servicenow.create_incident",
    "result": "success",
    "replay_attempted": true
  }
}
```

**First failing assertion:** `release-evidence-check.sh` rejects a write-capable
SaaS provider unless `target/appfw/governed-write-evidence.json` proves every
`SaasReadArea` governed-write gate.
`governed-write-check` must not create that evidence file; it only proves that
all external API providers remain write-disabled and MCP-disabled until real
provider-test evidence exists. The governed-write provider-test plan/preflight
surface records the live checks and environment inputs that a provider-backed
runner must satisfy. In live mode it may retain the evidence file only after
validating the external provider-backed evidence artifact named by
`APPFW_<PROVIDER>_GOVERNED_WRITE_EVIDENCE_FILE`, including explicit redaction
claims and absence of obvious secret, tenant-data, PHI, or raw-payload fields.
The retained G1 artifact is a governed-write contract proof, not a managed
release approval. The provider-specific preflight must make these input groups
visible:

- connection/auth inputs such as base URL, auth mode, token reference, test
  tenant, and test user;
- operation-contract inputs such as named mutation, policy scope, idempotency
  key, audit sink, and ingress source (`http`, `mcp`, or `kafka`);
- evidence-retention inputs such as the external live evidence file path;
- the provider-owned named mutation allow-list. For the ServiceNow lane, the
  first planned candidate is `servicenow.create_incident` with policy scope
  `servicenow.incident.write`; it remains non-executable until live G1
  evidence proves every governed-write gate.

## G2 Conversational And Agentic UI Primitives

**Command slot:** `scripts/appfw product frontend-test --json` plus PDS checker evidence.

**Retained artifact:** `target/appfw/pds-component-check.json`

**Minimum catalog additions:** G2 extends the existing PDS `Actions` family
instead of creating a second command family. `ConfirmDialog` already covers the
confirm-before-act shell; the new governed-action primitives add review,
audit, and undo/compensation evidence around it.

```text
Actions.IntentPreview
Actions.ConfirmDialog
Actions.ActionAudit
Actions.UndoCompensationState
Feedback.InlineAlert / Feedback.ForbiddenState for denied-policy state
```

**First failing assertion:** `scripts/check-pds-components.mjs --json` rejects
the catalog if the `governed-action` recipe lacks accessible examples,
component exports, visible catalog coverage, readiness evidence for the
governed-action primitives, or the G1/U2 evidence references that prove UI
controls are not being treated as backend write authorization. The required
recipe evidence ids are `g1-governed-write-posture`,
`g1-governed-write-live-evidence`, and `u2-agent-harness-profile`.
The same checker also retains `governed_action_live_readiness`; normal mode
reports whether those external artifacts are present and valid, while
`--enforce-governed-action` fails if a lane claims live governed-write readiness
without valid G1 posture, G1 provider-test write evidence, and U2 harness
approval.

## G3 Agentic Threat Model Catalog

**Command slot:** `scripts/appfw framework governance-check --json` may later
include this; until then docs-check owns the assertion.

**Source contract:** `docs/architecture/concerns/agentic-threat-model.md`

**Retained artifact:** `target/appfw/agentic-threat-model-check.json`

**Minimum mapping:**

```text
ASI01 instruction/control-flow hijacking
ASI02 tool misuse
ASI03 identity and privilege abuse
filesystem sandbox posture
network sandbox posture
human review checkpoint
```

**First failing assertion:** `scripts/appfw framework docs-check --json` rejects
the threat model if ASI01, ASI02, ASI03, filesystem sandboxing, network
sandboxing, and human review checkpoints are not all documented.

## G4 PHI Governance, Provenance, And Signing

**Command slot:** `scripts/appfw framework governance-check --json`; enforced
readiness slot: `scripts/appfw framework governance-check --enforce --json`

**Retained artifact:** `target/appfw/governance-check.json`; enforced artifact:
`target/appfw/governance-check-enforced.json`; PHI/data pipeline evidence:
`target/appfw/phi-pipeline-governance-evidence.json`, or the path named by
`APPFW_PHI_PIPELINE_EVIDENCE_FILE`

**Minimum schema:**

```json
{
  "command": "governance-check",
  "lane": "G4",
  "ok": true,
  "enforced_artifact": "target/appfw/governance-check-enforced.json",
  "gate": {
    "enforced": false,
    "ready_to_enforce": false
  },
  "phi_pipeline": {
    "schema": "appfw.phi-pipeline-governance.v1",
    "evidence_env": "APPFW_PHI_PIPELINE_EVIDENCE_FILE",
    "required_controls": [
      "classification_propagation",
      "deidentification",
      "environment_movement",
      "rag_curation",
      "retention_policy",
      "deletion_tombstone",
      "redaction",
      "audit_lineage"
    ],
    "summary": {
      "schema_ok": false,
      "release_ready": false,
      "missing_controls": []
    }
  },
  "provenance": {
    "local_evidence": "target/appfw/release-provenance.json",
    "production_attestation": "slsa-or-risk-accepted"
  },
  "artifact_signing": {
    "local_evidence": "target/appfw/artifact-signing.json",
    "production_signing": "cosign-or-risk-accepted"
  },
  "security_assurance_decision": "target/appfw/security-assurance-decision.json",
  "release_identity": "target/appfw/release-identity.json",
  "category_dispositions": {},
  "transitional_items": [],
  "blocking_violations": [],
  "enforcement_violations": []
}
```

**First failing assertion:** executable `governance-check` fails when the
framework sources that own PHI log lint, release evidence, security assurance,
or release identity decisions are missing. Strict regulated-release rejection
for absent or schema-thin PHI pipeline evidence, release provenance, artifact
signing, and security-assurance decisions remains in
`governance-check --enforce`, `release-evidence-check.sh`, and
`security-assurance-decision.sh`; the report-only gate records transitional
items until release authority evidence is complete, while `--enforce` fails
closed on those transitional items and retains the enforced JSON artifact.

## U2 Least-Privilege Product Agent Harness

**Command slot:** `scripts/appfw product harness-check --json`

**Source contract:** `.appfw/agent-profile.yaml`

**Retained artifact:** `.appfw/target/appfw/harness-check.json`

**Minimum schema:**

```json
{
  "command": "harness-check",
  "lane": "U2",
  "ok": true,
  "artifact": ".appfw/target/appfw/harness-check.json",
  "profile": {
    "allowed_commands": [
      {
        "command": "scripts/appfw product validate --json",
        "namespace": "product"
      }
    ],
    "writable_paths": [
      {
        "path": ".appfw/model",
        "ownership": "application_source"
      }
    ],
    "network_policy": "disabled",
    "live_service_policy": "disabled",
    "sensitive_capabilities": {
      "mcp": false,
      "kafka": false,
      "release": false,
      "saas_governed_write": false
    },
    "threat_controls": ["ASI01", "ASI02", "ASI03"]
  },
  "checks": [
    {
      "name": "commands-product-scoped",
      "ok": true
    }
  ],
  "violations": []
}
```

**First failing assertion:** product `harness-check` rejects a profile that
grants framework-source writes, framework commands, broad network access,
live-service access, sensitive capabilities, or SaaS governed-write access
without valid G1 governed-write evidence and enforced posture artifacts.

## U4 Connector Graduation

**Command slot:** `scripts/appfw framework provider-graduation --json`

**Retained artifact:** `target/appfw/provider-graduation.json`

**Minimum schema:**

```json
{
  "command": "provider-graduation",
  "lane": "U4",
  "ok": true,
  "mode": "report-only",
  "summary": {
    "provider_count": 12,
    "unsupported_capability_count": 103,
    "ungraduated_capability_count": 116,
    "graduated_capability_count": 105,
    "promotion_violation_count": 0
  },
  "providers": [
    {
      "provider": "salesforce",
      "family": "external_api",
      "profile": "saas_read",
      "ok": true,
      "unsupported_capability_count": 12,
      "ungraduated_capability_count": 12,
      "graduated_capability_count": 8,
      "promotion_violation_count": 0,
      "graduated_capabilities": ["named_operation_registry"],
      "areas": [
        {
          "area": "named_operation_registry",
          "label": "named operation registry",
          "status": "compiler-contracted",
          "graduated": true,
          "ok": true,
          "promotion_violation": false,
          "compiler_contracts": ["..."],
          "live_contracts": [],
          "required_evidence": ["compiler-contract"]
        }
      ]
    }
  ]
}
```

**First failing assertion:** provider graduation reports every framework
provider and rejects a promoted capability whose `live-certified` or
`compiler-contracted` status lacks the matching retained evidence.

## U5 React Native Mobile Contract

**Command slots:** `scripts/appfw product generate --target mobile-rn --json`,
`scripts/appfw product mobile-test --json`,
`scripts/appfw product mobile-test --run-local --json`, and
`scripts/appfw product mobile-test --device-preflight --json`

**Source contract:** generated `appfw-mobile-contract.ts` plus `.appfw-mobile/manifest.json`

**Retained artifact:** `.appfw/target/appfw/mobile-test.json`, with deeper
mobile evidence under `mobile/.appfw-mobile/`

**Minimum schema:**

```json
{
  "command": "mobile-test",
  "lane": "U5",
  "ok": true,
  "artifact": ".appfw/target/appfw/mobile-test.json",
  "mode": "static-readiness",
  "readiness_level": "static-scaffold",
  "legacy_evidence_satisfied": false,
  "candidate_ready": false,
  "release_ready": false,
  "release_authority": "none",
  "readiness_authority": {
    "authoritative": false,
    "status": "contained-non-authoritative",
    "containment_card": "M0-05",
    "release_authority": "none",
    "successor": "source-bound-mobile-candidate-checker"
  },
  "native_runtime": "react-native-expo-new-architecture",
  "inputs": {
    "mobile_plan": ".appfw/target/appfw/mobile-rn-conversion-plan.json",
    "mobile_root": "mobile",
    "contract": "mobile/src/generated/appfw-mobile-contract.ts",
    "ownership": "mobile/.appfw-mobile/ownership.json",
    "scaffold_manifest": "mobile/.appfw-mobile/scaffold-manifest.json",
    "typecheck_evidence": "mobile/.appfw-mobile/typecheck-evidence.json",
    "test_evidence": "mobile/.appfw-mobile/test-evidence.json",
    "expo_doctor_evidence": "mobile/.appfw-mobile/expo-doctor-evidence.json",
    "npm_audit_evidence": "mobile/.appfw-mobile/npm-audit-evidence.json",
    "npm_audit_disposition": "mobile/.appfw-mobile/npm-audit-disposition.json",
    "device_tooling_evidence": "mobile/.appfw-mobile/device-tooling-evidence.json"
  },
  "runtime_audit": {
    "evidence_passed": false,
    "disposition_present": false,
    "disposition_valid": false,
    "disposition_violations": [],
    "legacy_condition_satisfied": false,
    "release_ready": false
  },
  "checks": [
    {
      "name": "generated-contract-present",
      "ok": true
    },
    {
      "name": "pds-native-token-bridge-present",
      "ok": true
    },
    {
      "name": "offline-secure-storage-posture-present",
      "ok": true
    }
  ],
  "blocking_violations": [],
  "not_applicable": [],
  "risk_acceptance": []
}
```

**First failing assertion:** `mobile-test` rejects a mobile scaffold that does
not expose the expected generated-contract, PDS-token, auth, tenant, policy,
offline, and secure-storage source indicators. These are static diagnostics,
not proof that the source executes correctly against a real server or native
runtime. M0-05 requires `candidate_ready:false`, `release_ready:false`, and a
non-authoritative readiness envelope even when every legacy diagnostic passes.
Runtime audit findings must either pass or have a valid
`npm-audit-runtime-disposition` artifact that is approved, unexpired, and tied
to `mobile/.appfw-mobile/npm-audit-evidence.json` before
`legacy_condition_satisfied` can be true. A future source-bound mobile
candidate checker—not `mobile-test`—must own candidate readiness.

## U6 Fork Detection

**Command slot:** `scripts/appfw framework fork-check --json`

**Retained artifact:** `target/appfw/fork-check.json`

**Minimum schema:**

```json
{
  "command": "fork-check",
  "lane": "U6",
  "ok": true,
  "gate": {
    "enforced": true,
    "ready_to_enforce": true
  },
  "token_consumers": [],
  "source_consumers": [],
  "generated_contracts": [],
  "transitional_items": [],
  "blocking_violations": []
}
```

**First failing assertion:** executable `fork-check` fails on known
product-local PDS token copies, missing shared-source consumption, or generated
`tokenCssPath` values that do not point at the canonical PDS token source alias.
U6 now enforces rejection for the known admin/CRM token-copy paths; future
downstream product exceptions require a named, expiring risk acceptance.

## U7 Manifest Composition

**Command slot:** `scripts/appfw framework composition-check --json`;
`scripts/appfw framework composition-check --enforce --json` for release-gating.

**Source contract:** `.appfw/manifest.yaml`

**Retained artifact:** `target/appfw/composition-check.json`

**Minimum schema:**

```json
{
  "command": "composition-check",
  "lane": "U7",
  "ok": true,
  "gate": {
    "enforced": false,
    "ready_to_enforce": true
  },
  "runtime_ingress": ["http", "mcp", "kafka"],
  "ingress_modules": [],
  "providers": ["postgres", "salesforce"],
  "runtime_providers": ["postgres"],
  "provider_dependencies": [
    {
      "provider": "postgres",
      "optional": true,
      "feature": "provider-postgres",
      "feature_declared": true,
      "feature_links_dependency": true,
      "default_feature_enabled": true,
      "selected_by_runtime": true
    }
  ],
  "frontend": {
    "product_spa": true,
    "product_ui": {
      "enabled": true,
      "packaging": "backend-product-dist",
      "dist_present": true,
      "embedded": true
    },
    "admin_ui": {
      "enabled": false,
      "packaging": "disabled",
      "dist_present": false,
      "embedded": false
    },
    "served_by_backend_image": true
  },
  "binary_features": {
    "duplicate_version_budget_ok": true,
    "provider_feature_gate_ready": true,
    "provider_default_selection_ready": true,
    "unused_provider_features_disabled": true
  },
  "compile_evidence": {
    "feature_check": {
      "path": "target/appfw/feature-check.json",
      "present": true,
      "ok": true,
      "plan_only": false,
      "retained_compile_evidence_ready": true,
      "expected_checks": [
        "runtime core without ingress defaults",
        "runtime http ingress feature",
        "product backend provider-neo4j provider feature"
      ],
      "missing_or_failed_checks": []
    }
  },
  "transitional_items": [],
  "blocking_violations": []
}
```

**First failing assertion:** executable `composition-check --enforce` fails on
missing manifest/topology evidence, enabled ingress without a backend
feature/source module, enabled Kafka ingress without generated ingress config,
generated config that contradicts the manifest, enabled UI packaging without a
retained backend dist, default-enabled provider-certification crates, or missing
non-plan retained compile evidence for any expected runtime, ingress, and
provider feature subcheck. Plain `composition-check --json` remains available
for report-only inspection, but release-check runs enforced mode and retains the
artifact in the active release evidence directory. The report must distinguish
all topology providers from runtime-selected providers so certification-only
provider fixtures do not appear release-ready merely because they are listed in
`.appfw/manifest.yaml`.
The frontend section must also distinguish product-owned UI packaging intent
from framework source-tree presence: `ui.admin_ui.enabled:false` with
`packaging:"disabled"` is an explicit product posture, while
`enabled:true` without `backend/admin_dist/index.html` remains transitional.

## P6 Product Package Compatibility

**Command slot:** `scripts/appfw product compat-verify --json`

**Source contracts:** product `.appfw/manifest.yaml`, product `appfw.lock`,
product `Cargo.toml` dependency declarations, product `frontend/package.json`,
and framework `target/appfw/package-manifest.json`

**Retained artifact:** `.appfw/target/appfw/compat-verify.json`

**Minimum schema:**

```json
{
  "command": "compat-verify",
  "lane": "P6",
  "ok": true,
  "artifact": ".appfw/target/appfw/compat-verify.json",
  "gate": {
    "enforced": false,
    "ready_to_enforce": false
  },
  "inputs": {
    "manifest": ".appfw/manifest.yaml",
    "appfw_lock": "appfw.lock",
    "cargo_files": ["backend/Cargo.toml"],
    "frontend_package": "frontend/package.json",
    "framework_package_manifest": "target/appfw/package-manifest.json"
  },
  "current_framework": {
    "appfw_cli": "0.1.1",
    "appfw_codegen": "0.1.1",
    "appfw_runtime": "0.1.1"
  },
  "appfw_lock": {
    "present": true,
    "sha256": "sha256:..."
  },
  "rust_dependencies": [],
  "frontend": {
    "present": true,
    "appfw_pds_dependency": "0.1.1"
  },
  "checks": [
    {
      "name": "product-manifest-present",
      "ok": true
    },
    {
      "name": "no-local-appfw-path-dependencies",
      "ok": true,
      "report_only": true
    }
  ],
  "transitional_items": [],
  "blocking_violations": [],
  "not_applicable": [],
  "risk_acceptance": []
}
```

**First failing assertion:** executable `compat-verify` fails when the product
manifest is missing. The hard release rejection for missing `appfw.lock`, local
App Framework path dependencies, absent package manifests, and frontend package
gaps remains off until packaged ProGet consumption is the product default; until
then those gaps are recorded as `transitional_items`.
