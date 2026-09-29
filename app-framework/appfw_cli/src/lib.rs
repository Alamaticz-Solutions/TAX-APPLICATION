use std::{
    env,
    path::{Component, Path, PathBuf},
    process::{Command, ExitCode, ExitStatus},
};

pub mod instruction_routing;
pub mod lifecycle;

const INTROSPECT_COMMANDS: &[&str] = &[
    "explain",
    "handoff",
    "lock",
    "upgrade",
    "new",
    "analyze",
    "propose-model",
    "intake-proof",
    "golden-downstream",
    "boundary-check",
];

#[derive(Debug, Clone, Default)]
struct RootOverrides {
    app_root: Option<PathBuf>,
    framework_root: Option<PathBuf>,
    generator_root: Option<PathBuf>,
    config_root: Option<PathBuf>,
    templates_root: Option<PathBuf>,
    report_root: Option<PathBuf>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CliRoots {
    pub app_root: PathBuf,
    pub framework_root: PathBuf,
    pub generator_root: PathBuf,
    pub config_root: PathBuf,
    pub templates_root: PathBuf,
    pub report_root: PathBuf,
}

pub fn run_from_env() -> ExitCode {
    match run(env::args().skip(1)) {
        Ok(code) => code,
        Err(err) => {
            eprintln!("{err}");
            ExitCode::from(2)
        }
    }
}

pub fn run(args: impl IntoIterator<Item = String>) -> Result<ExitCode, String> {
    let mut args = args.into_iter().collect::<Vec<_>>();
    let overrides = parse_global_options(&mut args)?;

    if args.is_empty()
        || matches!(
            args.first().map(String::as_str),
            Some("help" | "--help" | "-h")
        )
    {
        print_usage();
        return Ok(ExitCode::SUCCESS);
    }

    if args.get(1).map(String::as_str) == Some("instructions") {
        let namespace = args.first().map(String::as_str).unwrap_or_default();
        let outcome = instruction_routing::resolve_arguments(namespace, &args[2..]);
        match outcome {
            Ok(envelope) => {
                print!("{}", envelope.to_json_line());
                return Ok(ExitCode::SUCCESS);
            }
            Err(error) => {
                print!("{}", error.to_json_line());
                return Ok(ExitCode::from(2));
            }
        }
    }

    let roots = resolve_roots(overrides)?;
    let command = args[0].as_str();
    if command == "lifecycle" {
        lifecycle::run(&args[1..])?;
        return Ok(ExitCode::SUCCESS);
    }
    let status = if INTROSPECT_COMMANDS.contains(&command) {
        run_introspect(&roots, &args)?
    } else {
        run_compat_wrapper(&roots, &args)?
    };

    Ok(exit_code_from_status(status))
}

fn parse_global_options(args: &mut Vec<String>) -> Result<RootOverrides, String> {
    let mut roots = RootOverrides::default();
    let mut idx = 0;
    while idx < args.len() {
        if args[idx] == "--" {
            break;
        }

        match args[idx].as_str() {
            "--repo-root" => {
                let path = take_path_arg(args, idx, "--repo-root")?;
                if roots.app_root.is_none() {
                    roots.app_root = Some(path.clone());
                }
                if roots.framework_root.is_none() {
                    roots.framework_root = Some(path);
                }
            }
            "--app-root" => {
                roots.app_root = Some(take_path_arg(args, idx, "--app-root")?);
            }
            "--framework-root" => {
                roots.framework_root = Some(take_path_arg(args, idx, "--framework-root")?);
            }
            "--generator-root" => {
                roots.generator_root = Some(take_path_arg(args, idx, "--generator-root")?);
            }
            "--config-root" => {
                roots.config_root = Some(take_path_arg(args, idx, "--config-root")?);
            }
            "--templates-root" => {
                roots.templates_root = Some(take_path_arg(args, idx, "--templates-root")?);
            }
            "--report-root" => {
                roots.report_root = Some(take_path_arg(args, idx, "--report-root")?);
            }
            _ => idx += 1,
        }
    }
    Ok(roots)
}

fn take_path_arg(args: &mut Vec<String>, idx: usize, flag: &str) -> Result<PathBuf, String> {
    let value = args
        .get(idx + 1)
        .ok_or_else(|| format!("{flag} requires a value"))?;
    let path = PathBuf::from(value);
    args.drain(idx..=idx + 1);
    Ok(path)
}

fn resolve_roots(overrides: RootOverrides) -> Result<CliRoots, String> {
    let cwd = env::current_dir().map_err(|err| err.to_string())?;
    let app_root = overrides
        .app_root
        .or_else(|| env_path("APPFW_APP_ROOT"))
        .unwrap_or_else(|| discover_app_root(&cwd));
    let app_root = absolutize(app_root, &cwd);

    let framework_root = overrides
        .framework_root
        .or_else(|| env_path("APPFW_FRAMEWORK_ROOT"))
        .or_else(installed_framework_root)
        .or_else(|| discover_framework_root(&cwd, &app_root))
        .unwrap_or_else(|| cwd.clone());
    let framework_root = absolutize(framework_root, &cwd);

    let generator_root = overrides
        .generator_root
        .or_else(|| env_path("APPFW_GENERATOR_ROOT"))
        .unwrap_or_else(|| framework_root.join("app_gen"));
    let config_root = overrides
        .config_root
        .or_else(|| env_path("APPFW_CONFIG_ROOT"))
        .unwrap_or_else(|| app_root.join(".appfw/model"));
    let templates_root = overrides
        .templates_root
        .or_else(|| env_path("APPFW_TEMPLATES_ROOT"))
        .unwrap_or_else(|| generator_root.join("_templates"));
    let report_root = overrides
        .report_root
        .or_else(|| env_path("APPFW_REPORT_ROOT"))
        .unwrap_or_else(|| app_root.join(".appfw/target/appfw"));

    Ok(CliRoots {
        app_root,
        framework_root,
        generator_root: absolutize(generator_root, &cwd),
        config_root: absolutize(config_root, &cwd),
        templates_root: absolutize(templates_root, &cwd),
        report_root: absolutize(report_root, &cwd),
    })
}

fn run_introspect(roots: &CliRoots, args: &[String]) -> Result<ExitStatus, String> {
    let invocation_cwd = env::current_dir().map_err(|err| err.to_string())?;
    if let Some(binary) = env_path("APPFW_INTROSPECT_BIN")
        .or_else(|| sibling_binary("appfw_introspect"))
        .or_else(|| packaged_binary(&roots.framework_root, "appfw_introspect"))
    {
        if binary.is_file() {
            return Command::new(binary)
                .current_dir(invocation_cwd)
                .arg("--app-root")
                .arg(&roots.app_root)
                .arg("--framework-root")
                .arg(&roots.framework_root)
                .arg("--generator-root")
                .arg(&roots.generator_root)
                .arg("--config-root")
                .arg(&roots.config_root)
                .arg("--templates-root")
                .arg(&roots.templates_root)
                .arg("--report-root")
                .arg(&roots.report_root)
                .args(args)
                .status()
                .map_err(|err| format!("failed to run packaged appfw_introspect: {err}"));
        }
    }

    ensure_dir(&roots.generator_root, "generator root")?;

    Command::new("cargo")
        .current_dir(invocation_cwd)
        .args(["run", "--locked", "--quiet", "--manifest-path"])
        .arg(roots.generator_root.join("Cargo.toml"))
        .args(["--bin", "appfw_introspect", "--", "--app-root"])
        .arg(&roots.app_root)
        .arg("--framework-root")
        .arg(&roots.framework_root)
        .arg("--generator-root")
        .arg(&roots.generator_root)
        .arg("--config-root")
        .arg(&roots.config_root)
        .arg("--templates-root")
        .arg(&roots.templates_root)
        .arg("--report-root")
        .arg(&roots.report_root)
        .args(args)
        .status()
        .map_err(|err| format!("failed to run appfw_introspect: {err}"))
}

fn run_compat_wrapper(roots: &CliRoots, args: &[String]) -> Result<ExitStatus, String> {
    let script = roots.framework_root.join("scripts/appfw");
    if !script.is_file() {
        return Err(format!(
            "could not find App Framework CLI wrapper at {}; pass --framework-root PATH or set APPFW_FRAMEWORK_ROOT",
            script.display()
        ));
    }

    let invocation_cwd = env::current_dir().map_err(|err| err.to_string())?;
    let command_cwd = if roots.app_root.is_dir() {
        roots.app_root.clone()
    } else {
        invocation_cwd
    };

    Command::new(script)
        .current_dir(command_cwd)
        .env("APPFW_APP_ROOT", &roots.app_root)
        .env("APPFW_FRAMEWORK_ROOT", &roots.framework_root)
        .env("APPFW_GENERATOR_ROOT", &roots.generator_root)
        .env("APPFW_CONFIG_ROOT", &roots.config_root)
        .env("APPFW_TEMPLATES_ROOT", &roots.templates_root)
        .env("APPFW_REPORT_ROOT", &roots.report_root)
        .envs(packaged_binary_env(&roots.framework_root))
        .args(args)
        .status()
        .map_err(|err| format!("failed to run scripts/appfw compatibility wrapper: {err}"))
}

fn discover_app_root(cwd: &Path) -> PathBuf {
    let default_product = cwd.join("examples/products/crm");
    if cwd.join("scripts/appfw").is_file() && default_product.join(".appfw/manifest.yaml").is_file()
    {
        return default_product;
    }
    if cwd.join(".appfw/manifest.yaml").is_file() || cwd.join(".appfw/model").is_dir() {
        return cwd.to_path_buf();
    }
    if cwd.file_name().and_then(|name| name.to_str()) == Some("model")
        && cwd
            .parent()
            .and_then(|parent| parent.file_name())
            .and_then(|name| name.to_str())
            == Some(".appfw")
    {
        return cwd
            .parent()
            .and_then(|parent| parent.parent())
            .unwrap_or(cwd)
            .to_path_buf();
    }
    cwd.to_path_buf()
}

fn discover_framework_root(cwd: &Path, app_root: &Path) -> Option<PathBuf> {
    if cwd.join("scripts/appfw").is_file() {
        return Some(cwd.to_path_buf());
    }
    if cwd.file_name().and_then(|name| name.to_str()) == Some("app_gen") {
        let parent = cwd.parent().unwrap_or(cwd);
        if parent.join("scripts/appfw").is_file() {
            return Some(parent.to_path_buf());
        }
    }
    if app_root.join("scripts/appfw").is_file() {
        return Some(app_root.to_path_buf());
    }
    if let Some(parent) = app_root.parent() {
        let adjacent = parent.join("app-framework");
        if adjacent.join("scripts/appfw").is_file() {
            return Some(adjacent);
        }
    }
    if let Some(parent) = cwd.parent() {
        let adjacent = parent.join("app-framework");
        if adjacent.join("scripts/appfw").is_file() {
            return Some(adjacent);
        }
    }
    None
}

fn installed_framework_root() -> Option<PathBuf> {
    let exe = env::current_exe().ok()?;
    let bin_dir = exe.parent()?;
    let root = bin_dir.parent()?;
    if root.join("scripts/appfw").is_file() && root.join("app_gen/_templates").is_dir() {
        Some(root.to_path_buf())
    } else {
        None
    }
}

fn sibling_binary(name: &str) -> Option<PathBuf> {
    let exe = env::current_exe().ok()?;
    let bin_dir = exe.parent()?;
    let candidate = bin_dir.join(binary_name(name));
    candidate.is_file().then_some(candidate)
}

fn packaged_binary(framework_root: &Path, name: &str) -> Option<PathBuf> {
    let candidate = framework_root.join("bin").join(binary_name(name));
    candidate.is_file().then_some(candidate)
}

fn packaged_binary_env(framework_root: &Path) -> Vec<(&'static str, PathBuf)> {
    let mut envs = Vec::new();
    for (variable, binary) in [
        ("APPFW_INTROSPECT_BIN", "appfw_introspect"),
        ("APPFW_APP_GEN_BIN", "app_gen"),
        ("APPFW_DATABASE_BIN", "database"),
    ] {
        if env::var_os(variable).is_none() {
            if let Some(path) =
                sibling_binary(binary).or_else(|| packaged_binary(framework_root, binary))
            {
                envs.push((variable, path));
            }
        }
    }
    envs
}

fn binary_name(name: &str) -> String {
    if cfg!(windows) {
        format!("{name}.exe")
    } else {
        name.to_string()
    }
}

fn ensure_dir(path: &Path, label: &str) -> Result<(), String> {
    if path.is_dir() {
        Ok(())
    } else {
        Err(format!(
            "{label} does not exist or is not a directory: {}",
            path.display()
        ))
    }
}

fn env_path(name: &str) -> Option<PathBuf> {
    env::var_os(name)
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
}

fn absolutize(path: PathBuf, cwd: &Path) -> PathBuf {
    let joined = if path.is_absolute() {
        path
    } else {
        cwd.join(path)
    };
    if let Ok(canonical) = joined.canonicalize() {
        canonical
    } else {
        normalize_components(&joined)
    }
}

fn normalize_components(path: &Path) -> PathBuf {
    let mut normalized = PathBuf::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                normalized.pop();
            }
            _ => normalized.push(component.as_os_str()),
        }
    }
    normalized
}

fn exit_code_from_status(status: ExitStatus) -> ExitCode {
    status
        .code()
        .map_or(ExitCode::FAILURE, |code| ExitCode::from(code as u8))
}

fn print_usage() {
    println!(
        r#"appfw - App Framework command-line interface

Usage:
  cargo run -p appfw-cli -- <command> [options]
  appfw <command> [options]
  appfw product <command> [options]
  appfw framework <command> [options]

Canonical namespaces:
  product     Build, run, validate, test, release, upgrade, and hand off a
              downstream product app.
  framework   Evolve, validate, certify, document, package, and release the
              App Framework itself.

Flat commands remain compatibility aliases for existing scripts. Prefer
`product` or `framework` in new docs, skills, prompts, and CI examples.

Mobile product work uses the product namespace. Start a React Native conversion
plan for an HTML mobile mockup with:
  appfw product mobile-plan --ui-artifact prototype.html --json

Resolve source-linked current-task instructions and independent review with:
  appfw framework instructions --task framework-docs-ia --role coding-agent --change-class C --json

Install:
  cargo install appfw-cli

Packaged distribution:
  appfw auto-discovers sibling tool binaries in bin/ when installed from the
  App Framework ProGet toolchain bundle. Use --framework-root only for custom
  layouts.

Root options:
  --repo-root PATH       Compatibility alias for copied framework layouts
  --app-root PATH        Downstream product root
  --framework-root PATH  App Framework checkout root
  --generator-root PATH  appfw-codegen package root
  --config-root PATH     Product .appfw/model root
  --templates-root PATH  Framework templates root
  --report-root PATH     Product .appfw/target/appfw report root

The CLI can run from the framework checkout, a copied-framework product, or a
product checkout adjacent to app-framework. Use APPFW_* environment variables or
explicit root options for other layouts.
"#
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn repo_root_sets_app_and_framework_roots() {
        let mut args = vec![
            "--repo-root".to_string(),
            "/repo".to_string(),
            "validate".to_string(),
        ];

        let overrides = parse_global_options(&mut args).expect("parse global options");

        assert_eq!(overrides.app_root, Some(PathBuf::from("/repo")));
        assert_eq!(overrides.framework_root, Some(PathBuf::from("/repo")));
        assert_eq!(args, vec!["validate"]);
    }

    #[test]
    fn root_options_are_removed_before_command_dispatch() {
        let mut args = vec![
            "--app-root".to_string(),
            "/product".to_string(),
            "--framework-root".to_string(),
            "/framework".to_string(),
            "handoff".to_string(),
            "--json".to_string(),
        ];

        let overrides = parse_global_options(&mut args).expect("parse global options");

        assert_eq!(overrides.app_root, Some(PathBuf::from("/product")));
        assert_eq!(overrides.framework_root, Some(PathBuf::from("/framework")));
        assert_eq!(args, vec!["handoff", "--json"]);
    }

    #[test]
    fn adjacent_framework_root_is_discovered_from_product_root() {
        let base = env::temp_dir().join(format!("appfw-cli-test-{}", std::process::id()));
        let product = base.join("operations-insight");
        let script = base.join("app-framework/scripts/appfw");
        let _ = std::fs::remove_dir_all(&base);
        std::fs::create_dir_all(script.parent().expect("script parent")).expect("create dirs");
        std::fs::write(&script, "#!/usr/bin/env bash\n").expect("write script");

        let framework = discover_framework_root(&product, &product);

        assert_eq!(framework, Some(base.join("app-framework")));
        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn framework_checkout_defaults_app_root_to_crm_sample() {
        let base = env::temp_dir().join(format!(
            "appfw-cli-framework-root-test-{}",
            std::process::id()
        ));
        let framework = base.join("app-framework");
        let script = framework.join("scripts/appfw");
        let crm_manifest = framework.join("examples/products/crm/.appfw/manifest.yaml");
        let _ = std::fs::remove_dir_all(&base);
        std::fs::create_dir_all(script.parent().expect("script parent")).expect("create scripts");
        std::fs::create_dir_all(crm_manifest.parent().expect("manifest parent"))
            .expect("create crm manifest parent");
        std::fs::write(&script, "#!/usr/bin/env bash\n").expect("write script");
        std::fs::write(&crm_manifest, "name: crm-sample\n").expect("write crm manifest");

        let app_root = discover_app_root(&framework);

        assert_eq!(app_root, framework.join("examples/products/crm"));
        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn non_existing_product_root_can_still_resolve_framework_root() {
        let cwd = env::temp_dir().join(format!(
            "appfw-cli-new-product-root-test-{}",
            std::process::id()
        ));
        let framework = cwd.join("app-framework");
        let script = framework.join("scripts/appfw");
        let package_marker = framework.join("app-framework-package.json");
        let templates = framework.join("app_gen/_templates");
        let app_root = cwd.join("new-product");
        let _ = std::fs::remove_dir_all(&cwd);
        std::fs::create_dir_all(script.parent().expect("script parent")).expect("create scripts");
        std::fs::create_dir_all(&templates).expect("create templates");
        std::fs::write(&script, "#!/usr/bin/env bash\n").expect("write script");
        std::fs::write(&package_marker, "{}\n").expect("write marker");

        let framework_root = discover_framework_root(&framework, &app_root);

        assert_eq!(framework_root, Some(framework));
        assert!(!app_root.exists());
        let _ = std::fs::remove_dir_all(&cwd);
    }

    #[test]
    fn packaged_binary_env_exports_all_packaged_helpers() {
        let root = env::temp_dir().join(format!(
            "appfw-cli-packaged-helpers-test-{}",
            std::process::id()
        ));
        let bin = root.join("bin");
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&bin).expect("create packaged bin directory");
        for binary in ["appfw_introspect", "app_gen", "database"] {
            std::fs::write(bin.join(binary_name(binary)), "").expect("create packaged helper");
        }

        let envs = packaged_binary_env(&root);

        assert_eq!(envs.len(), 3);
        for (variable, binary) in [
            ("APPFW_INTROSPECT_BIN", "appfw_introspect"),
            ("APPFW_APP_GEN_BIN", "app_gen"),
            ("APPFW_DATABASE_BIN", "database"),
        ] {
            assert!(envs.iter().any(|(actual_variable, path)| {
                *actual_variable == variable && path == &bin.join(binary_name(binary))
            }));
        }
        let _ = std::fs::remove_dir_all(&root);
    }
}
