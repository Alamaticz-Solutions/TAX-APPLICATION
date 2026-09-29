# Codegen API

`appfw-codegen` exposes a small Rust API for product automation. This is the
supported package boundary for downstream build tooling and product workflow
commands. Generator internals under `app_gen/src` remain framework
implementation details unless they are re-exported from the crate root.

## Stable Surface

Use the crate-root exports:

```rust
use appfw_codegen::{generate, validate, Codegen, CodegenOptions, CodegenRoots};
```

The supported types are:

| Type | Purpose |
| --- | --- |
| `CodegenRoots` | Explicit product, framework, generator, config, template, and report roots. |
| `CodegenOptions` | Versionable generation options. |
| `CodegenMode` | `ValidateOnly` or `Generate`. |
| `Codegen` | Builder for running validation or generation. |
| `CodegenReport` | Returned mode, resolved roots, and standard report artifact paths. |
| `CodegenArtifactPaths` | Standard report paths under the configured report root. |

These structs are `#[non_exhaustive]`; downstream code should construct them
with provided constructors and avoid matching them exhaustively.

## Product Layouts

Copied-framework products can use:

```rust
let roots = CodegenRoots::copied_layout("/path/to/product");
```

Dependency-based products should separate product source from framework source:

```rust
let roots = CodegenRoots::split_layout("/path/to/product", "/path/to/app-framework");
```

Use `CodegenRoots::new(...)` for explicit root overrides.

Generation always consumes product source from the product root and framework
source from the framework root:

| Root | Consumed/Written Content |
| --- | --- |
| `app_root` | Product-owned backend, database, API tests, `.appfw/manifest.yaml`, `podman-compose.yml`, and checked-in generated outputs. |
| `config_root` | Product-owned schema, entity, relationship, data-source, seed, policy, and API scenario config. |
| `report_root` | Product-owned validation, topology, dev-infra, config-contract, IR, performance, and artifact reports. |
| `framework_root` | Framework checkout containing codegen/runtime/test packages and workflow scripts. |
| `generator_root` | Framework-owned `appfw-codegen` package source. |
| `templates_root` | Framework-owned generator templates. |

The generator must not require templates or generator source to exist in the
product root once split roots are provided.

## Build Script Pattern

When a product uses `appfw-codegen` as a build dependency, keep the build script
thin and explicit:

```rust
use appfw_codegen::{Codegen, CodegenRoots};

fn main() -> anyhow::Result<()> {
    let roots = CodegenRoots::split_layout(
        env!("CARGO_MANIFEST_DIR"),
        "/path/to/app-framework",
    );

    Codegen::new(roots.clone()).emit_cargo_rerun_if_changed();
    Codegen::new(roots).validate_only().run()?;

    Ok(())
}
```

Prefer validation in `build.rs`. Use generation from an explicit product
workflow command so source-writing changes remain reviewable.

## Workflow Command Pattern

Product-owned commands can generate artifacts without parsing CLI flags:

```rust
use appfw_codegen::{generate, CodegenRoots};

fn main() -> anyhow::Result<()> {
    let roots = CodegenRoots::from_env_or_current_dir()?;
    let report = generate(roots)?;

    println!("{}", report.artifacts.artifact_manifest.display());
    Ok(())
}
```

The existing `app_gen`, `appfw`, and `appfw_introspect` binaries remain
compatibility entry points. New downstream automation should prefer the stable
API above.
