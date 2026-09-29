# PHI Pipeline Governance

> Status: Wave 4 executable governance contract. This page defines the evidence
> shape behind `scripts/appfw framework governance-check --json` for regulated
> data moving through materialized projections, cached product stores, AI/RAG
> corpora, logs, and lower environments.

## Why This Exists

Classification validation and PHI log lint are necessary, but they do not prove
that a regulated data pipeline is safe to operate. A production claim also needs
evidence for how sensitive data is classified, transformed, moved, retained,
redacted, deleted, and audited across environments and AI search surfaces.

This contract closes the S9 litmus gap without creating a second release gate:
`governance-check` inventories the evidence in report mode and fails closed in
`--enforce` mode when the PHI pipeline package is absent or schema-thin.

## Evidence Location

Default retained evidence:

```text
target/appfw/phi-pipeline-governance-evidence.json
```

Override for managed release environments:

```text
APPFW_PHI_PIPELINE_EVIDENCE_FILE=/path/to/phi-pipeline-governance-evidence.json
```

The evidence file is an external release artifact. Local fixtures may prove the
validator, but they must not claim production readiness.

## Evidence Schema

Minimum release-ready shape:

```json
{
  "schema_version": "appfw.phi-pipeline-governance.v1",
  "ok": true,
  "release_ready": true,
  "controls": {
    "classification_propagation": {
      "status": "approved",
      "evidence_refs": ["target/appfw/config_contract.json"]
    },
    "deidentification": {
      "status": "approved",
      "evidence_refs": ["release/deid-approval.json"]
    },
    "environment_movement": {
      "status": "approved",
      "evidence_refs": ["release/lower-env-data-decision.json"]
    },
    "rag_curation": {
      "status": "approved",
      "evidence_refs": ["release/rag-corpus-curation.json"]
    },
    "retention_policy": {
      "status": "approved",
      "evidence_refs": ["release/retention-policy.json"]
    },
    "deletion_tombstone": {
      "status": "approved",
      "evidence_refs": ["release/deletion-tombstone-proof.json"]
    },
    "redaction": {
      "status": "approved",
      "evidence_refs": ["target/appfw/phi-log-lint.json"]
    },
    "audit_lineage": {
      "status": "approved",
      "evidence_refs": ["target/appfw/audit-lineage.json"]
    }
  }
}
```

`status` may be `approved`, `passed`, `release-ready`, or `risk-accepted` when
there is at least one evidence reference. It may be `not-applicable` only when a
non-empty `rationale` explains why the control does not apply to that release.

## Required Controls

| Control | What It Proves |
| --- | --- |
| `classification_propagation` | Data-source, schema, entity, property, projection, and AI/RAG metadata carry the right classification. |
| `deidentification` | PHI/ePHI is de-identified before lower-environment or AI/RAG use, or an approved exception exists. |
| `environment_movement` | Lower-environment movement follows synthetic/de-identified-data rules and approved exceptions. |
| `rag_curation` | RAG corpus ingestion has allow-lists, exclusions, freshness, access scope, and removal paths. |
| `retention_policy` | Retention and purge policy is named and tied to the data classification. |
| `deletion_tombstone` | Deletes, revocations, and tombstones propagate through projections and AI/RAG indexes. |
| `redaction` | Logs, prompts, errors, diagnostics, and retained artifacts do not carry raw PHI or secrets. |
| `audit_lineage` | Audit and lineage connect source records, transformations, projections, RAG corpora, and release evidence. |

## Command Contract

Report mode:

```bash
scripts/appfw framework governance-check --json
```

Enforced mode:

```bash
scripts/appfw framework governance-check --enforce --json
```

Report mode may return `ok:true` with transitional items. Enforced mode exits
non-zero until PHI pipeline evidence, provenance/signing evidence, release
identity, PDS baseline evidence, and security-assurance decisions are
release-ready or formally risk-accepted.

## Related Work

- [Threat Model Litmus](threat-model-litmus.md), especially S9.
- [SaaS Connectors](../../runtime/saas-connectors.md), for materialized
  projections, CDC, object maps, watermarks, and retention.
- [AI Chat And Search](../../runtime/ai-chat-search.md), for RAG/search answer
  grounding and prompt/log governance.
- [Release Gate CI/CD](../../release/release-gate-ci-cd.md), for retained
  release artifacts.
