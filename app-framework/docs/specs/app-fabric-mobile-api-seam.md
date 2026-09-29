# App Fabric Mobile API Compatibility Seam

| Field | Value |
| --- | --- |
| Record | `AF-MOBILE-M0-02` |
| Contract version | `0.1.0-prototype` |
| Stage | Prototype planning, pre-candidate |
| Baseline | `origin/main@2de105e001ab8029eb21bce4691eded49e48ff81` |
| Planning disposition | Proposed technical wire decision; executable red real-schema proof and Integration freeze SHA pending; implementation correction still required |
| Candidate ready | false |
| Release ready | false |
| Authority | Technical seam only; no API migration, source Assignment, candidate, or release authority |

## Decision

The generated backend GraphQL schema is the wire authority for a given
framework release. `.appfw/model` is semantic source; generated web and mobile
contracts are projections and must never become upstream inputs to another
generator.

For the current v1 reference proof, mobile must use the same App Framework
query envelope already consumed by the web client:

- connection-like queries return `items`, `query_count`, `next_cursor`,
  `previous_cursor`, `skip`, `limit`, `page_count`, `page_index`, `date_time`,
  and `request_duration`;
- record lookup binds `id` as GraphQL `String!` while the backend exposes a
  Rust `String` argument;
- query inputs use `filter: JSON`, `sort: JSON`, `skip: Int`, `limit: Int`, and
  `after: String` when applicable;
- the client may map wire names into idiomatic application names, but the
  generated query and decoder must preserve the real schema; and
- a later move to Relay pagination or GraphQL `ID` is a coordinated,
  versioned server/client migration, not a client-only formatting choice.

For the current wire, generated operation documents must not contain the
semantic-only scalar name `JsonValue`, an `ID!` find-by-id variable,
`AccountConnection`, `nodes`, `pageInfo`, or `totalCount`. The concrete list
field returns non-null `AccountQueryResult`; `findAccount` is nullable because
the current backend returns `Option<AccountProjection>`.

The source-bound reference operations are retained in
[`app-fabric-mobile-api-compatibility.graphql`](./app-fabric-mobile-api-compatibility.graphql).

## Verified Current Mismatch

Accepted `main` contains a real incompatibility, not merely missing polish.

| Surface | Current evidence | Consequence |
| --- | --- | --- |
| Backend record argument | `examples/products/crm/backend/src/handlers/crm/mod.rs` exposes `find_account(id: String)` | a generated `$id: ID!` operation is invalid against the current schema |
| Backend list result | `examples/products/crm/backend/src/schemas/crm.rs` exposes `AccountQueryResult` with `items`, `query_count`, cursor, and page fields | Relay-only selections do not exist on this type |
| Generated mobile request | `examples/products/crm/mobile/src/generated/AppfwMobileDataClient.ts` selects `nodes`, `totalCount`, and `pageInfo` for `returnsShape:"connection"` | a real server validates and rejects the generated list query |
| Generated mobile metadata | `examples/products/crm/mobile/src/generated/appfw-mobile-contract.ts` declares record `id` as `ID` | the generated record operation conflicts with the backend argument type |
| Mobile unit test | `examples/products/crm/mobile/__tests__/mobile-contract.test.ts` asserts `nodes/pageInfo` and mocks `nodes/totalCount` | the test currently preserves the defect and cannot be used as client/server compatibility evidence |
| Web client comparator | `examples/products/crm/frontend/src/lib/appfwClient.ts` already emits the current JSON/list envelope and `String!` record lookup | this is useful observed evidence, but generated web TypeScript is still not a generator source |

The mismatch means static mobile generation, TypeScript, and mocked unit tests
can pass while the generated client cannot execute its core read operations
against the generated backend. All current mobile readiness claims must remain
non-candidate and non-release.

## Frozen Source And Ownership Boundary

The implementation correction belongs in canonical generator/schema sources,
not in checked-in generated CRM files:

1. `app_gen` must own a renderer-neutral operation IR derived from model and
   backend schema intent, including exact variables, wire return shape,
   selections, pagination, errors, auth, and tenant requirements.
2. Web and native client generators consume that IR independently.
3. `scripts/appfw` orchestrates generation; its embedded mobile templates must
   be retired as M1 moves emission into `app_gen`.
4. Product-generated `mobile/src/generated/**` and generated CRM backend files
   remain overwrite-safe outputs.
5. Product-owned native workflows consume the generated client and may map
   its typed result into product state; they do not rewrite the wire contract.

No implementation source is changed by this planning record. The shared
checkout contained overlapping work, and the current request is to preserve
research and planning from a clean base. A future exact Assignment must own the
generator/source roots and regenerate the downstream fixture.

## Required Compatibility Proof

M0-02 first retains this source-owned test red against accepted-main. M1 cannot
claim this seam implemented until the same test is green and performs all of
the following on a clean generated downstream product:

1. generate the backend schema and mobile client from the same model and
   generator SHA;
2. validate every generated mobile operation document against that exact
   GraphQL schema;
3. execute list and record operations against an in-process or real generated
   backend, with no mock response shape between client and server;
4. assert list fields, cursor progression, identifier binding, nullability,
   selection safety, and decode mapping;
5. assert extensions preserve request and correlation identifiers;
6. exercise unauthenticated, expired-token, wrong-tenant, policy-denied,
   invalid-ID, invalid-cursor, validation, partial-data, and unexpected-error
   cases without converting denial into empty or offline state;
7. make the generated mobile unit test use the real App Framework envelope;
8. fail generation/check when a client operation and server schema diverge;
   and
9. retain the model, generator, schema, operation-document, client, backend,
   and test hashes in the evidence result.

The minimum headless M0/M1 machine result should identify:

```text
schema: appfw_mobile_api_compatibility@1
source_sha
model_hash
generator_version
backend_schema_hash
operation_document_hash
phase: red | green
expected_failure_ref: required-when-red
cases[]: { id, execution: headless, failure_stage, result,
           request_id: null-or-string, correlation_id: null-or-string }
candidate_ready: false
release_ready: false
```

`failure_stage` distinguishes document parse, schema validation, transport,
server execution, and decode/assertion failure. Request and correlation IDs are
required once a backend request begins; they are `null` for a legitimate
pre-execution parse/schema failure, never fabricated merely to satisfy the
result shape.

M3/M5 later add a separate platform-execution result that references the green
headless compatibility-result hash and records `platform`, exact
`client_build_or_bundle_id`, runtime identity, and cases. Simulator/emulator
evidence is required for development journey proof, but it is not a dependency
of the M0 red or M1 green headless API seam. One browser pass is not native
evidence; one iOS pass does not qualify Android.

## Versioning And Failure Rules

- A client advertises a supported API compatibility range and sends its app,
  binary/runtime, bundle, and contract identity through an approved diagnostic
  mechanism that does not expose secrets.
- The server rejects unsupported client/API combinations predictably and
  returns a safe upgrade-required result; it never silently changes action or
  policy meaning for an old client.
- Additive nullable response fields are normally compatible. Removing or
  renaming selected fields, changing nullability, scalar type, argument shape,
  pagination semantics, authorization semantics, or error classification is a
  compatibility event requiring versioned proof.
- Cached data must not mask an incompatible write or policy decision. Reads may
  present explicitly stale data only when classification and product semantics
  permit it.
- A generated client/server mismatch blocks the reference journey, candidate
  entry, OTA promotion, and any native-readiness claim.

## Next Exact Implementation Card

The API/operation-transport slice of canonical card `M1-01` should:

- introduce the canonical operation IR in `app_gen`;
- emit the App Framework list envelope and `String!` record binding for the
  current schema;
- regenerate the CRM mobile fixture rather than hand-edit it;
- replace the self-confirming Relay mock with schema validation and a real
  backend compatibility test; and
- run the generator safe loop plus focused mobile tests in an isolated
  worktree.

Changing the backend to Relay/`ID` is an alternative migration, but it is
broader and must update web, native, generated API tests, compatibility policy,
and downstream consumers together. It is not implied by this record.

## Role Card Check

- **Card used:** Architect Agent, operating within the Fabric/Mobile seam.
- **Within role:** source hierarchy, wire compatibility, versioning, proof, and
  safe implementation boundary.
- **Authority not assumed:** source Assignment/admission, API migration,
  product acceptance, risk acceptance, candidate, release, merge, or push.
- **Routed decisions:** canonical generator change to an exact implementation
  Assignment; API migration to Fabric/Runtime Architecture; WIP to the Program
  Flow Controller; candidate promotion to named human authorities.
- **Drift signal:** `needs-correction` until the generated client is
  schema-valid and executes against the generated backend.
