# API Tests

`api_tests` contains two separate live-test surfaces:

- Product-generated API scenarios under `src/schemas`.
- Framework-owned provider certification contracts under
  `src/provider_*_contracts.rs` and `src/provider_schema_contracts.rs`.

Compile scenarios without calling a backend:

```bash
scripts/appfw test
```

Run generated scenarios against a prepared backend:

```bash
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

Direct crate commands are also available:

```bash
cargo test --no-run
cargo test schemas:: -- --show-output
```

Use `scripts/appfw api-test` for product scenarios. It runs only tests whose
module path is under `schemas::`.

## Runtime Configuration

| Variable | Default | Description |
| --- | --- | --- |
| `API_TEST_BASE_URL` | `http://localhost:8080` | Backend API root. Schema names are appended to this URL. |
| `API_TEST_TIMEZONE` | `America/Denver` | Value sent in the `timezone` header. |
| `API_TEST_AUTH_MODE` | `bypass` | Use `bypass` locally or `token_files` to read tokens from disk. |
| `API_TEST_TOKEN_DIR` | `api_tests/test_tokens` | Directory containing `{token_name}.txt` files for token-file mode. |
| `API_TEST_PROVIDER` | unset | Optional provider hint for certification contracts: `postgres`, `mongo`, `mssql`, or `snowflake`. |
| `API_TEST_BASE_URL_POSTGRES` | unset | Provider-specific backend URL used by `scripts/appfw provider-test --all`. |
| `API_TEST_BASE_URL_MONGO` | unset | Provider-specific backend URL used by `scripts/appfw provider-test --all`. |
| `API_TEST_BASE_URL_MSSQL` | unset | Provider-specific backend URL used by `scripts/appfw provider-test --all`. |
| `API_TEST_BASE_URL_SNOWFLAKE` | unset | Provider-specific backend URL used by `scripts/appfw provider-test --all`. |

Provider certification is documented in `docs/PROVIDER_CERTIFICATION.md`. Run
the live provider suite once per configured provider before a release:

```bash
scripts/appfw provider-test --provider postgres
scripts/appfw provider-test --all --json
```

`provider-test` enables certification mode and defaults auth to `local_dev` so
policy-scoped users are exercised. Token-file certification environments must
provide equivalent `pdsh_admin`, `cc_tenant_user`, `tenant_one_crm_ops`, and
`west_sales_rep` tokens. `tenant_one_crm_ops` is the tenant-1 `crm_ops`
certification reader for governed AccountAudit evidence; it remains distinct
from the denied `cc_tenant_user` actor.
Compose-style CI can set `APP_ENABLE_LOCAL_TEST_AUTH=true` on the backend to
accept explicit `Bearer appfw-local:...` headers outside `ENV_NAME=local`; this
requires `APP_PROVIDER_CERTIFICATION_CI=true` and does not enable missing-auth
local admin behavior. Tenant-isolation certification also uses
`other_tenant_admin`.
In release CI, prefer:

```bash
scripts/appfw release-check --json
```

Direct provider-certification crate commands must opt in explicitly:

```bash
API_TEST_PROVIDER_CERTIFICATION=1 cargo test provider_contracts:: -- --show-output --test-threads=1
API_TEST_PROVIDER_CERTIFICATION=1 cargo test provider_semantic_contracts:: -- --show-output --test-threads=1
API_TEST_PROVIDER_CERTIFICATION=1 cargo test provider_schema_contracts:: -- --show-output --test-threads=1
```

## Scenario Source

Add test config under:

```text
app_gen/_config/schemas/<schema>/tests/
```

Generation writes Rust tests under:

```text
api_tests/src/schemas/<schema>/
```

Each YAML file becomes one ordered Rust scenario. Steps in a scenario share an
isolated result store, so references such as `create_account_result.id` do not
depend on Rust test ordering.

## YAML Shape

```yaml
- name: create_account
  description: create account
  auth_token: pdsh_admin
  graphql:
    schema: crm
    type: mutation
    name: createAccount
    variables:
    - name: input
      type: InputAccount!
      literal_value:
        name: Example Account
    select: [id, name]
  expect:
    data:
      createAccount:
        name: Example Account
  result_object_name: create_account_result
```
