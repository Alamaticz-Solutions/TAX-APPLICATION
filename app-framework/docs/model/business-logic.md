# Business Logic

Business logic belongs in explicit, human-owned modules. The generator should
wire the API and data access path; services should express domain decisions.

## Recommended Layers

```text
GraphQL route
Generated handler module
Human-owned handler impl
Service/domain module
DataAccess
Provider client
```

Generated files should stay boring. Business rules should be readable and
testable without understanding template internals.

## Rules And Decisions

For deterministic business rules, model the decision inputs and outputs:

```rust
pub struct ApprovalInput {
    pub amount: f64,
    pub customer_tier: String,
    pub risk_score: f64,
}

pub struct ApprovalDecision {
    pub approved: bool,
    pub reason: Option<String>,
}

pub fn decide_approval(input: ApprovalInput) -> ApprovalDecision {
    if input.risk_score > 80.0 {
        return ApprovalDecision {
            approved: false,
            reason: Some("risk score too high".to_string()),
        };
    }

    ApprovalDecision {
        approved: true,
        reason: None,
    }
}
```

Pure functions like this are easy to unit test and can be called from generated
custom method implementations.

## Workflows

Use a workflow service when an operation has ordered steps:

```text
load state
validate authorization
apply domain rules
write changes
publish audit/business event
return API result
```

Keep the workflow explicit. Avoid hiding important side effects inside helper
methods with vague names.

## Rego Policies

Access-control rules are evaluated separately from business workflows. Policy
fixtures and contract tests live under:

```text
rego_test/
```

Backend data access should preserve policy decisions and row-level filters when
custom behavior calls into `DataAccess`.

## External Systems

Put external clients behind small traits or service structs. Keep retries,
timeouts, and error mapping near the client boundary, not in generated handlers.

## Observability

Use `tracing` on high-value paths:

```rust
#[tracing::instrument(skip(self), fields(account_id = %account_id))]
pub async fn calculate_health(&self, account_id: String) -> anyhow::Result<Health> {
    // ...
}
```

Do not log secrets, access tokens, raw credentials, or tenant-sensitive payloads.

## Verification

```bash
scripts/appfw validate
scripts/appfw test
```

For behavior exposed through GraphQL, add API scenarios under:

```text
.appfw/model/schemas/<schema>/tests/
```
