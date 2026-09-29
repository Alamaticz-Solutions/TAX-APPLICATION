# Schema Config

Schema config is documented by the generated config contract:

```text
.appfw/model/_specs/CONFIG_CONTRACT.md
.appfw/target/appfw/config_contract.json
```

Run from the repository root:

```bash
scripts/appfw validate
```

Typical layout:

```text
schemas/<schema>/
|-- _res.yaml
|-- entity_types/*.yaml
|-- gql_enum_types/*.yaml
|-- rbac/*.rego
|-- seeds/*.yaml
`-- tests/*.yaml
```

Use `docs/02_SCHEMA_DESIGN.md` for authoring guidance.
