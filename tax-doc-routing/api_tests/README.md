# API Tests

`api_tests` contains product-generated API scenarios under `src/schemas`.
Framework-owned provider certification contracts live in the app-framework
checkout and run through `scripts/appfw provider-test`.

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

## Scenario Source

Add test config under:

```text
.appfw/model/schemas/<schema>/tests/
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
