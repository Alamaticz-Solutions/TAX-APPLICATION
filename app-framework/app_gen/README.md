# app_gen

`app_gen` is the source directory for the `appfw-codegen` package. It validates
product `.appfw/model` source and generates Rust backend, database package,
and API test artifacts.
The package exposes a stable crate-root API for downstream automation plus
compatibility binaries for the current CLI workflows.

Most developers and agents should run it through the root CLI:

```bash
scripts/appfw validate
scripts/appfw generate
```

Direct crate commands are still available from this directory:

```bash
cargo run --locked -- --validate-only
cargo run --locked
```

`app_gen` is root-aware. The wrapper passes explicit roots for the product app,
framework checkout, generator crate, app config, framework templates, and report
output. Product apps normally use a split-root layout: config and generated
artifacts stay in the product repo, while generator source and templates are
read from the framework checkout. Direct runs can be made explicit:

```bash
cargo run --locked --manifest-path app_gen/Cargo.toml --bin app_gen -- \
  --app-root . \
  --framework-root . \
  --generator-root app_gen \
  --config-root .appfw/model \
  --templates-root app_gen/_templates \
  --report-root .appfw/target/appfw \
  --validate-only
```

Equivalent environment variables are `APPFW_APP_ROOT`,
`APPFW_FRAMEWORK_ROOT`, `APPFW_GENERATOR_ROOT`, `APPFW_CONFIG_ROOT`,
`APPFW_TEMPLATES_ROOT`, and `APPFW_REPORT_ROOT`.

## Stable API

Downstream Rust automation should use the crate-root API instead of importing
generator internals:

```rust
use appfw_codegen::{generate, Codegen, CodegenRoots};

fn validate_product() -> anyhow::Result<()> {
    let roots = CodegenRoots::from_env_or_current_dir()?;
    Codegen::new(roots).validate_only().run()?;
    Ok(())
}

fn generate_product() -> anyhow::Result<()> {
    let roots = CodegenRoots::from_env_or_current_dir()?;
    let report = generate(roots)?;
    println!("{}", report.artifacts.artifact_manifest.display());
    Ok(())
}
```

Supported API details live in `docs/CODEGEN_API.md`.

## Responsibilities

- Validate application config before generation.
- Validate the app topology manifest against application config.
- Emit the config contract.
- Generate backend schemas, routes, and handler wiring.
- Generate local dev infra from app topology and data-source config.
- Create human-owned handler implementation files when missing.
- Generate database package artifacts.
- Generate API test modules.
- Emit an artifact ownership manifest.
- Emit performance/index recommendation artifacts from config.
- Carry golden downstream app profile overlays used by `scripts/appfw new`.
- Keep the first-class CRM sample product that proves the downstream contract.

## Important Outputs

By default, reports are written under `.appfw/target/appfw`. A root-aware run
may redirect them with `--report-root` or `APPFW_REPORT_ROOT`.

```text
.appfw/target/appfw/validation.json
.appfw/target/appfw/app_topology.json
.appfw/target/appfw/dev_infra.json
.appfw/target/appfw/config_contract.json
.appfw/target/appfw/config_contract.md
.appfw/target/appfw/generator_ir.json
.appfw/target/appfw/performance_recommendations.json
.appfw/target/appfw/performance_recommendations.md
.appfw/target/appfw/artifact_provenance.json
.appfw/target/appfw/artifacts.json
.appfw/model/_specs/CONFIG_CONTRACT.md
```

## Config Contract

Product `.appfw/model` is source code. Its canonical shape is owned by:

```text
src/config_contract.rs
src/validation.rs
```

Do not hand-maintain `.appfw/model/_specs/CONFIG_CONTRACT.md` as independent
docs. Change the Rust contract and run validation.

## Generated Ownership

The generator records artifact ownership:

- `generated`: controlled by config/templates and overwrite-safe.
- `human_owned`: created when missing, then preserved.

Inspect ownership from the repo root:

```bash
scripts/appfw manifest --json
```

The manifest is hash-bearing: generated and human-owned artifact records include
SHA-256 hashes reconciled from final regular-file bytes, and
`.appfw/target/appfw/artifact_provenance.json` records the config, template,
generator-source, and manifest hashes for release review. Generation fails when
a recorded artifact is missing, a directory, or a symlink at ledger-finalization
time.

Generated compose reads the `appfw_runtime` dependency source from
`backend/Cargo.toml`. Path dependencies retain the local Framework runtime bind.
A complete, validated sibling observability tree retains the observability
services; an absent sibling is omitted with a truthful rationale, while a
present incomplete or symlinked tree fails closed. Git and registry dependencies
omit checkout-bound runtime and observability assets.
`.appfw/target/appfw/dev_infra.json` records the resulting posture and rationale.

If the manifest does not exist, use `docs/start/generated-ownership.md` instead
of running generation solely to inspect ownership.

## Templates

Templates live in:

```text
_templates/
```

Golden downstream app profiles live in:

```text
_golden/downstream_apps/
```

These are app bootstrap overlays, not generator templates. Use them for
downstream onboarding defaults and profile metadata. First-class product
templates live outside `app_gen` under:

```text
../examples/products/
```

`scripts/appfw new` copies a product template, applies the selected overlay, and
rewrites local path dependencies so the generated product consumes the current
framework checkout instead of vendoring framework source.

When changing templates, run:

```bash
scripts/appfw validate --json
scripts/appfw generate
scripts/appfw generate --check --json
scripts/appfw test
```

`generate --check` is a drift diagnostic for generator/template work. If it
reports generated drift, preserve the diagnostic in your handoff.

## Console Output

Normal output is grouped by stage and uses stable labels such as `[step]`,
`[write]`, `[copy]`, `[skip]`, and `[warn]`.

Debug options:

```text
APP_GEN_VERBOSE=1
APP_GEN_COLOR=always
APP_GEN_COLOR=never
NO_COLOR=1
```
