# Service Layer

The generator does not require a service layer, but enterprise applications
usually need one once custom behavior spans multiple entities, providers, or
workflows.

Use services for human-owned business logic. Keep generated handlers thin.

## When To Add A Service

Add a service when logic:

- Is reused by multiple custom methods.
- Coordinates multiple entities.
- Needs focused unit tests.
- Calls external systems.
- Contains domain rules that should not live in GraphQL glue.

Simple generated CRUD should stay generated.

## Suggested Layout

```text
backend/src/services/
|-- mod.rs
|-- account_service.rs
`-- opportunity_service.rs
```

This directory is human-owned. CRM-sample products include and wire
`backend/src/services/mod.rs` so teams can add service modules without changing
crate wiring. The CRM sample keeps the `account_health` custom method as a
thin handler that delegates to `backend/src/services/account_health.rs`.

## Basic Pattern

```rust
use std::sync::Arc;

use crate::{
    product_api::{DataAccess, EntityType, HandlerResult, JsonValue, UserAuth},
};

pub struct AccountService {
    data_access: Arc<DataAccess>,
    entity_type: Arc<EntityType>,
}

impl AccountService {
    pub fn new(data_access: Arc<DataAccess>, entity_type: Arc<EntityType>) -> Self {
        Self {
            data_access,
            entity_type,
        }
    }

    pub async fn health_summary(
        &self,
        user: Option<UserAuth>,
        account_id: String,
    ) -> HandlerResult<JsonValue> {
        // Keep provider-specific logic behind DataAccess where possible.
        let _ = (user, account_id);
        Ok(serde_json::json!({ "status": "unknown" }))
    }
}
```

## Handler Integration

Generated custom method defaults live in:

```text
backend/src/handlers/<schema>/generated.rs
```

Product-owned handler overrides live in:

```text
backend/src/handlers/<schema>/<entity>.rs
```

Use local handler overrides to call services:

```rust
pub async fn account_health_impl(
    user: Option<UserAuth>,
    data_access: &Arc<DataAccess>,
    entity_type: &Arc<EntityType>,
    selections: JsonValue,
    account_id: String,
) -> HandlerResult<JsonValue> {
    let _ = selections;
    crate::services::account_health::summarize_account_health(
        user,
        data_access,
        entity_type,
        account_id,
    )
    .await
}
```

## Transactions

Transaction support is provider-specific today. Prefer provider-neutral
operations through `DataAccess`; when a service requires an explicit transaction,
document the provider assumption and keep the provider-specific code isolated.

## Testing

Test pure domain logic without the framework runtime. For logic tied to generated
API behavior, add generated API scenarios and run:

```bash
scripts/appfw test
scripts/appfw migrate
```

Run the backend in a separate terminal:

```bash
ENV_NAME=local API_PORT=8080 scripts/appfw serve
```

Then run API scenarios:

```bash
scripts/appfw api-test
```

## Design Rules

- Keep handlers thin.
- Keep services human-owned.
- Avoid duplicating provider query semantics in services.
- Prefer `crate::product_api` imports for handler/service runtime dependencies.
- Prefer explicit inputs and outputs over passing raw GraphQL context.
- Use tracing instrumentation on high-value service paths.
