use std::{
    collections::{BTreeMap, BTreeSet, HashMap},
    env, fs,
    io::{self, IsTerminal, Read, Write},
    path::{Component, Path, PathBuf},
    process::{Command, ExitCode},
    time::Instant,
};

use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use syn::{
    visit::{self, Visit},
    ItemFn, ItemUse, UseTree,
};
use toml_edit::Document;

use appfw_codegen::schema_route_segment;

const PRODUCT_MODEL_ROOT: &str = ".appfw/model";
const PRODUCT_REPORT_ROOT: &str = ".appfw/target/appfw";
const FRAMEWORK_MODEL_SEED_ROOT: &str = "app_gen/_config";
const APPFW_LOCK_VERSION: u32 = 2;
const ARTIFACT_MANIFEST_IDENTITY_SCHEMA: &str = "appfw.artifact_manifest_identity@2";
const PORTABLE_EVIDENCE_MAX_BYTES: u64 = 256 * 1024 * 1024;
const APPFW_LOCK_MAX_BYTES: u64 = 1024 * 1024;

#[derive(Clone, Debug)]
struct CommandRoots {
    app_root: PathBuf,
    framework_root: PathBuf,
    generator_root: PathBuf,
    config_root: PathBuf,
    templates_root: PathBuf,
    report_root: PathBuf,
}

#[derive(Clone, Debug, Serialize)]
struct CommandRootReport {
    app_root: String,
    framework_root: String,
    generator_root: String,
    config_root: String,
    templates_root: String,
    report_root: String,
}

impl CommandRoots {
    fn resolve(
        cwd: &Path,
        app_root: Option<PathBuf>,
        framework_root: Option<PathBuf>,
        generator_root: Option<PathBuf>,
        config_root: Option<PathBuf>,
        templates_root: Option<PathBuf>,
        report_root: Option<PathBuf>,
    ) -> Result<Self, String> {
        let default_framework_root = default_repo_root_from(cwd);
        let default_product_root = default_product_root_for_framework(&default_framework_root);
        let default_uses_framework_product = default_product_root.is_some();
        let default_app_root =
            default_product_root.unwrap_or_else(|| default_framework_root.clone());
        let app_root = normalize_path(&absolutize(
            app_root
                .or_else(|| env_path("APPFW_APP_ROOT"))
                .unwrap_or(default_app_root),
            cwd,
        ));
        let framework_root = normalize_path(&absolutize(
            framework_root
                .or_else(|| env_path("APPFW_FRAMEWORK_ROOT"))
                .unwrap_or_else(|| {
                    if default_uses_framework_product
                        || is_framework_checkout(&default_framework_root)
                    {
                        default_framework_root
                    } else {
                        app_root.clone()
                    }
                }),
            cwd,
        ));
        let generator_root = normalize_path(&absolutize(
            generator_root
                .or_else(|| env_path("APPFW_GENERATOR_ROOT"))
                .unwrap_or_else(|| framework_root.join("app_gen")),
            cwd,
        ));
        let config_root = normalize_path(&absolutize(
            config_root
                .or_else(|| env_path("APPFW_CONFIG_ROOT"))
                .unwrap_or_else(|| app_root.join(PRODUCT_MODEL_ROOT)),
            cwd,
        ));
        let templates_root = normalize_path(&absolutize(
            templates_root
                .or_else(|| env_path("APPFW_TEMPLATES_ROOT"))
                .unwrap_or_else(|| generator_root.join("_templates")),
            cwd,
        ));
        let report_root = normalize_path(&absolutize(
            report_root
                .or_else(|| env_path("APPFW_REPORT_ROOT"))
                .unwrap_or_else(|| app_root.join(PRODUCT_REPORT_ROOT)),
            cwd,
        ));

        Ok(Self {
            app_root,
            framework_root,
            generator_root,
            config_root,
            templates_root,
            report_root,
        })
    }

    fn split_layout(app_root: &Path, framework_root: &Path) -> Self {
        let app_root = normalize_path(app_root);
        let framework_root = normalize_path(framework_root);
        let generator_root = framework_root.join("app_gen");
        let config_root = app_root.join(PRODUCT_MODEL_ROOT);
        let templates_root = generator_root.join("_templates");
        let report_root = app_root.join(PRODUCT_REPORT_ROOT);
        Self {
            app_root,
            framework_root,
            generator_root,
            config_root,
            templates_root,
            report_root,
        }
    }

    fn report(&self) -> CommandRootReport {
        CommandRootReport {
            app_root: self.app_root.display().to_string(),
            framework_root: self.framework_root.display().to_string(),
            generator_root: self.generator_root.display().to_string(),
            config_root: self.config_root.display().to_string(),
            templates_root: self.templates_root.display().to_string(),
            report_root: self.report_root.display().to_string(),
        }
    }
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("{err}");
            ExitCode::from(2)
        }
    }
}

fn run() -> Result<(), String> {
    let mut args = env::args().skip(1).collect::<Vec<_>>();
    let cwd = env::current_dir().map_err(|err| err.to_string())?;
    let mut app_root = None;
    let mut framework_root = None;
    let mut generator_root = None;
    let mut config_root = None;
    let mut templates_root = None;
    let mut report_root = None;
    let mut json_output = false;

    let mut idx = 0;
    while idx < args.len() {
        match args[idx].as_str() {
            "--repo-root" | "--app-root" => {
                let value = args
                    .get(idx + 1)
                    .ok_or_else(|| format!("{} requires a value", args[idx]))?;
                app_root = Some(PathBuf::from(value));
                args.drain(idx..=idx + 1);
            }
            "--framework-root" => {
                framework_root = Some(required_path_arg(&args, idx, "--framework-root")?);
                args.drain(idx..=idx + 1);
            }
            "--generator-root" => {
                generator_root = Some(required_path_arg(&args, idx, "--generator-root")?);
                args.drain(idx..=idx + 1);
            }
            "--config-root" => {
                config_root = Some(required_path_arg(&args, idx, "--config-root")?);
                args.drain(idx..=idx + 1);
            }
            "--templates-root" => {
                templates_root = Some(required_path_arg(&args, idx, "--templates-root")?);
                args.drain(idx..=idx + 1);
            }
            "--report-root" => {
                report_root = Some(required_path_arg(&args, idx, "--report-root")?);
                args.drain(idx..=idx + 1);
            }
            "--json" => {
                json_output = true;
                args.remove(idx);
            }
            _ => idx += 1,
        }
    }
    let roots = CommandRoots::resolve(
        &cwd,
        app_root,
        framework_root,
        generator_root,
        config_root,
        templates_root,
        report_root,
    )?;
    let Some(command) = args.first().map(String::as_str) else {
        return Err(usage());
    };
    let rest = &args[1..];

    match command {
        "explain" => explain(&roots, json_output, rest),
        "handoff" => handoff_command(&roots, json_output, rest),
        "lock" => lock_command(&roots, json_output, rest),
        "upgrade" => upgrade_command(&roots, json_output, rest),
        "new" => new_command(&roots.framework_root, json_output, rest),
        "analyze" => analyze_command(&roots, json_output, rest),
        "propose-model" => propose_model_command(&roots, json_output, rest),
        "model-status" => model_status_command(&roots, json_output, rest),
        "scaffold-model" => scaffold_model_command(&roots, json_output, rest),
        "intake-proof" => intake_proof_command(&roots, json_output, rest),
        "golden-downstream" => golden_downstream_command(&roots, json_output, rest),
        "harness-check" => harness_check_command(&roots, json_output, rest),
        "boundary-check" => boundary_check_command(&roots, json_output, rest),
        "--help" | "-h" | "help" => {
            println!("{}", usage());
            Ok(())
        }
        _ => Err(format!("unknown appfw introspection command: {command}")),
    }
}

fn usage() -> String {
    "Usage: appfw_introspect [--app-root PATH] [--framework-root PATH] [--generator-root PATH] [--config-root PATH] [--templates-root PATH] [--report-root PATH] [--json] explain|handoff|lock|upgrade|new|analyze|propose-model|model-status|scaffold-model|intake-proof|golden-downstream|harness-check|boundary-check ..."
        .to_string()
}

fn env_path(name: &str) -> Option<PathBuf> {
    env::var_os(name)
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
}

fn required_path_arg(args: &[String], idx: usize, flag: &str) -> Result<PathBuf, String> {
    args.get(idx + 1)
        .map(PathBuf::from)
        .ok_or_else(|| format!("{flag} requires a path value"))
}

fn explain(roots: &CommandRoots, json_output: bool, args: &[String]) -> Result<(), String> {
    let Some(kind) = args.first().map(String::as_str) else {
        return Err("explain requires ownership|config|provider|provider-sdk".to_string());
    };
    let rest = &args[1..];
    match kind {
        "provider-sdk" => {
            if !rest.is_empty() {
                return Err("explain provider-sdk does not accept additional arguments".to_string());
            }
            let rules = provider_sdk_rules(&roots.framework_root)?;
            emit_value(json_output, &rules, || provider_sdk_text(&rules))
        }
        "ownership" => {
            let target = rest
                .first()
                .ok_or_else(|| "explain ownership requires a path".to_string())?;
            let explanation = explain_ownership(&roots.app_root, target)?;
            emit(json_output, &explanation, || ownership_text(&explanation))
        }
        "config" => {
            let target = rest
                .first()
                .ok_or_else(|| "explain config requires a config path or key".to_string())?;
            let explanation = explain_config(&roots.app_root, target)?;
            emit(json_output, &explanation, || config_text(&explanation))
        }
        "provider" => {
            let target = rest
                .first()
                .ok_or_else(|| "explain provider requires a provider contract area".to_string())?;
            let provider = parse_provider_arg(&rest[1..])?;
            let explanation = explain_provider(&roots.framework_root, target, provider.as_deref())?;
            emit(json_output, &explanation, || provider_text(&explanation))
        }
        _ => Err(format!("unknown explain target: {kind}")),
    }
}

fn parse_provider_arg(args: &[String]) -> Result<Option<String>, String> {
    let mut provider = None;
    let mut idx = 0;
    while idx < args.len() {
        match args[idx].as_str() {
            "--provider" => {
                let value = args.get(idx + 1).ok_or_else(|| {
                    "--provider requires postgres|mongo|mssql|snowflake".to_string()
                })?;
                provider = Some(value.clone());
                idx += 2;
            }
            value => return Err(format!("unknown explain provider option: {value}")),
        }
    }
    Ok(provider)
}

#[derive(Debug, Serialize)]
struct OwnershipExplanation {
    command: &'static str,
    ok: bool,
    path: String,
    relative_path: String,
    classification: String,
    ownership: String,
    safe_to_edit: bool,
    source_of_truth: String,
    edit_guidance: String,
    verification: Vec<String>,
    evidence: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct ArtifactRecord {
    path: String,
    ownership: String,
    overwrite: String,
    action: String,
    content_sha256: Option<String>,
    source_sha256: Option<String>,
}

fn explain_ownership(repo_root: &Path, target: &str) -> Result<OwnershipExplanation, String> {
    let absolute = absolute_path(repo_root, target);
    let relative = relative_path(repo_root, &absolute);
    let manifest_match = artifact_manifest(repo_root)?
        .into_iter()
        .find(|record| normalize_path(Path::new(&record.path)) == absolute);
    let mut explanation = static_ownership(repo_root, &absolute, &relative);

    if let Some(record) = manifest_match {
        explanation.ownership = record.ownership.clone();
        explanation.evidence.push(format!(
            "artifact manifest: ownership={}, overwrite={}, action={}",
            record.ownership, record.overwrite, record.action
        ));
        if let Some(hash) = record.content_sha256 {
            explanation
                .evidence
                .push(format!("artifact manifest content hash: {hash}"));
        }
        if let Some(hash) = record.source_sha256 {
            explanation
                .evidence
                .push(format!("artifact manifest source hash: {hash}"));
        }
        if record.ownership == "generated" {
            explanation.safe_to_edit = false;
            explanation.edit_guidance =
                "Change the generating config/template/source, then run generation.".to_string();
        }
    }

    Ok(explanation)
}

fn static_ownership(repo_root: &Path, absolute: &Path, relative: &str) -> OwnershipExplanation {
    let mut explanation = OwnershipExplanation {
        command: "explain ownership",
        ok: true,
        path: absolute.display().to_string(),
        relative_path: relative.to_string(),
        classification: "unknown".to_string(),
        ownership: "unknown".to_string(),
        safe_to_edit: false,
        source_of_truth: "No specific ownership rule matched.".to_string(),
        edit_guidance: "Inspect nearby files and docs/start/generated-ownership.md before editing."
            .to_string(),
        verification: vec!["scripts/appfw product validate --json".to_string()],
        evidence: vec!["static ownership rules".to_string()],
    };

    let rel = relative.replace('\\', "/");
    let rule_rel = product_example_rule_path(&rel).unwrap_or_else(|| rel.clone());
    let file_name = Path::new(relative)
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or_default();

    let set = |explanation: &mut OwnershipExplanation,
               classification: &str,
               ownership: &str,
               safe_to_edit: bool,
               source: &str,
               guidance: &str,
               verification: Vec<&str>| {
        explanation.classification = classification.to_string();
        explanation.ownership = ownership.to_string();
        explanation.safe_to_edit = safe_to_edit;
        explanation.source_of_truth = source.to_string();
        explanation.edit_guidance = guidance.to_string();
        explanation.verification = verification.into_iter().map(str::to_string).collect();
    };

    if rule_rel != rel {
        explanation.evidence.push(format!(
            "classified example product path with product workspace rules: {rule_rel}"
        ));
    }

    if rule_rel == "appfw.lock" {
        set(
            &mut explanation,
            "framework_provenance_lock",
            "generated",
            false,
            "scripts/appfw product lock --write",
            "Refresh this file with the CLI after intentional framework/generator changes.",
            vec![
                "scripts/appfw product upgrade --json",
                "scripts/appfw product lock --write",
            ],
        );
    } else if rule_rel == "Cargo.lock" {
        set(
            &mut explanation,
            "cargo_workspace_lock",
            "generated",
            false,
            "Cargo workspace dependency resolution",
            "Refresh this file with cargo generate-lockfile after intentional workspace dependency or membership changes.",
            vec![
                "cargo metadata --locked --no-deps",
                "scripts/appfw framework docs-check --json",
            ],
        );
    } else if rule_rel == ".gitignore" {
        set(
            &mut explanation,
            "repository_ignore_rules",
            "framework_source",
            true,
            ".gitignore and product template ignore rules",
            "Keep ignore rules aligned with generated artifacts, committed lockfiles, and product-template ownership.",
            vec![
                "git check-ignore -v <path>",
                "scripts/appfw framework docs-check --json",
            ],
        );
    } else if rule_rel == ".cargo/config.toml" {
        set(
            &mut explanation,
            "framework_cargo_registry_config",
            "framework_source",
            true,
            ".cargo/config.toml and docs/release/proget-distribution.md",
            "Keep the private Cargo registry alias aligned with framework crate publish metadata and product scaffold registry config.",
            vec![
                "cargo check --locked -p appfw-runtime",
                "scripts/appfw framework package --plan --json",
                "scripts/appfw framework docs-check --json",
            ],
        );
    } else if rule_rel == ".appfw/agent-profile.yaml" {
        set(
            &mut explanation,
            "product_agent_profile_source",
            "application_source",
            true,
            ".appfw/agent-profile.yaml",
            "Edit this least-privilege product agent profile with the allowed command and writable-path contract in sync with product ownership and handoff evidence.",
            vec![
                "scripts/appfw product harness-check --json",
                "scripts/appfw product handoff --json",
            ],
        );
    } else if rule_rel == ".appfw/manifest.yaml" {
        set(
            &mut explanation,
            "app_topology_manifest",
            "application_source",
            true,
            ".appfw/manifest.yaml",
            "Edit app identity and topology assertions here; keep schemas, entities, relationships, and data-source details in .appfw/model.",
            vec![
                "scripts/appfw product topology --json",
                "scripts/appfw product validate --json",
            ],
        );
    } else if rule_rel.starts_with(".appfw/specs/") {
        set(
            &mut explanation,
            "product_change_spec_source",
            "application_source",
            true,
            "docs/start/spec-driven-change-harness.md",
            "Edit this product-owned decision-provenance source alongside the implementation and acceptance evidence it governs.",
            vec![
                "scripts/appfw product validate --json",
                "scripts/appfw product handoff --json",
            ],
        );
    } else if rule_rel == "Cargo.toml" || rule_rel.ends_with("/Cargo.toml") {
        set(
            &mut explanation,
            "framework_package_manifest",
            "framework_source",
            true,
            "Cargo package manifests and docs/architecture/framework-packaging.md",
            "Edit package manifests when changing framework crate boundaries, dependency edges, or compatibility binaries.",
            vec![
                "cargo check --manifest-path <changed Cargo.toml>",
                "scripts/appfw framework test",
            ],
        );
    } else if rule_rel == ".appfw/model/_specs/CONFIG_CONTRACT.md" {
        set(
            &mut explanation,
            "generated_config_contract_doc",
            "generated",
            false,
            "app_gen/src/config_contract.rs",
            "Change the config contract emitter, then run scripts/appfw framework validate --json.",
            vec!["scripts/appfw framework validate --json"],
        );
    } else if rule_rel.starts_with(".appfw/model/") && file_name == "_res.yaml" {
        set(
            &mut explanation,
            "generated_resolved_config",
            "generated",
            false,
            "Neighboring .appfw/model source files.",
            "Edit the source YAML/Rego config, not the resolved _res.yaml output.",
            vec![
                "scripts/appfw product validate --json",
                "scripts/appfw product generate --check --json",
            ],
        );
    } else if rule_rel.starts_with(".appfw/model/") {
        set(
            &mut explanation,
            "application_config_source",
            "application_source",
            true,
            ".appfw/model",
            "This is app model source. Prefer config changes here over generated output patches.",
            vec![
                "scripts/appfw product validate --json",
                "scripts/appfw product test",
            ],
        );
    } else if rule_rel.starts_with("app_gen/_config/chat_evals/") {
        set(
            &mut explanation,
            "chat_eval_fixture_source",
            "framework_source",
            true,
            "docs/architecture/concerns/chat-eval-spec.md and docs/runtime/ai-chat-search.md",
            "Edit here when changing deterministic AI chat/search evaluation fixtures. Keep fixtures network-free, synthetic by default, and aligned with scripts/check-chat-eval.mjs.",
            vec![
                "scripts/check-chat-eval.mjs --json",
                "scripts/check-chat-eval.mjs --json --inject-leak",
                "scripts/appfw framework docs-check --json",
            ],
        );
    } else if rule_rel.starts_with("app_gen/_templates/") || rule_rel.starts_with("app_gen/src/") {
        set(
            &mut explanation,
            "generator_source",
            "framework_source",
            true,
            "app_gen templates and generator Rust code.",
            "Edit here when changing generated shape for all downstream apps.",
            vec![
                "scripts/appfw framework validate --json",
                "scripts/appfw framework generate",
                "scripts/appfw framework generate --check --json",
                "scripts/appfw framework test",
            ],
        );
    } else if rule_rel.starts_with("app_gen/_golden/downstream_apps/") {
        set(
            &mut explanation,
            "golden_downstream_template",
            "framework_source",
            true,
            "app_gen/_golden/downstream_apps",
            "Edit here when changing blessed downstream app bootstrap profiles.",
            vec![
                "scripts/appfw product new --list-profiles --json",
                "scripts/appfw framework docs-check --json",
            ],
        );
    } else if rule_rel.starts_with("frontend/src/generated/")
        || rule_rel == "frontend/.appfw-ui/scaffold-manifest.json"
    {
        set(
            &mut explanation,
            "generated_frontend_contract",
            "generated",
            false,
            ".appfw/model and app_gen frontend contract generation",
            "Change product model/config or the frontend generator source, then regenerate the frontend contract.",
            vec![
                "scripts/appfw product generate",
                "scripts/appfw product generate --check --json",
                "scripts/appfw product frontend-test --json",
            ],
        );
    } else if rule_rel == "frontend/package-lock.json" {
        set(
            &mut explanation,
            "product_frontend_dependency_lock",
            "generated",
            false,
            "frontend/package.json and the package manager lockfile resolver",
            "Refresh this lockfile with npm install after intentional frontend dependency changes; do not hand-edit resolved package metadata.",
            vec![
                "npm run appfw:check",
                "npm run build",
                "scripts/appfw product frontend-test --json",
                "scripts/appfw product validate --json",
            ],
        );
    } else if rule_rel == "frontend/package.json"
        || rule_rel == "frontend/tsconfig.json"
        || rule_rel == "frontend/vite.config.ts"
        || rule_rel == "frontend/playwright.config.ts"
        || rule_rel == "frontend/.appfw-ui/ownership.json"
        || rule_rel.starts_with("frontend/src/app/")
        || rule_rel.starts_with("frontend/src/components/")
        || rule_rel.starts_with("frontend/src/scaffold/")
        || rule_rel.starts_with("frontend/src/lib/")
        || rule_rel.starts_with("frontend/src/styles/")
        || rule_rel.starts_with("frontend/src/design/")
        || rule_rel.starts_with("frontend/scripts/")
    {
        set(
            &mut explanation,
            "product_frontend_scaffold_source",
            "human_owned",
            true,
            "docs/frontend/product-frontend.md and frontend/.appfw-ui/ownership.json",
            "Keep reusable frontend scaffold/package changes aligned with generated UI contracts, PDS tokens, and frontend release evidence.",
            vec![
                "npm run appfw:check",
                "npm run tokens:check",
                "npm run build",
                "scripts/appfw product frontend-test --json",
            ],
        );
    } else if rule_rel.starts_with("frontend/src/features/")
        || rule_rel.starts_with("frontend/tests/")
        || rule_rel == "frontend/README.md"
    {
        set(
            &mut explanation,
            "product_frontend_feature_source",
            "human_owned",
            true,
            "docs/frontend/product-frontend.md",
            "Product frontend workflow code is product-owned; consume generated contracts and PDS design-system assets.",
            vec![
                "npm run appfw:check",
                "npm run typecheck",
                "npm run build",
                "scripts/appfw product frontend-test --json",
            ],
        );
    } else if rule_rel.starts_with("mobile/src/generated/")
        || rule_rel.starts_with("mobile/app/entities/")
        || rule_rel == "mobile/src/design/pdsNativeTokens.ts"
        || rule_rel == "mobile/.appfw-mobile/ownership.json"
        || rule_rel == "mobile/.appfw-mobile/scaffold-manifest.json"
    {
        set(
            &mut explanation,
            "mobile_generated_target_artifact",
            "generated",
            false,
            "docs/frontend/mobile-react-native.md and scripts/appfw product generate --target mobile-rn",
            "Treat this as generated mobile target material. Update product model/mobile generator inputs and rerun the mobile-rn target rather than hand-editing.",
            vec![
                "scripts/appfw product mobile-test --json",
                "scripts/appfw product generate --check --json",
            ],
        );
    } else if rule_rel.starts_with("mobile/.appfw-mobile/") {
        set(
            &mut explanation,
            "mobile_diagnostic_evidence",
            "human_owned",
            true,
            "docs/frontend/mobile-react-native.md",
            "Keep compatibility-named mobile auth, offline, local-run, audit, device, and store-track inputs accurate and non-authoritative. They must keep candidate_ready and release_ready false: the future source-bound checker owns candidate evidence, and named humans own distribution and release decisions.",
            vec![
                "scripts/appfw product mobile-test --json",
                "scripts/appfw product validate --json",
            ],
        );
    } else if rule_rel == "mobile/package-lock.json" {
        set(
            &mut explanation,
            "product_mobile_dependency_lock",
            "generated",
            false,
            "mobile/package.json and the package manager lockfile resolver",
            "Refresh this lockfile with npm install after intentional mobile dependency changes; do not hand-edit resolved package metadata.",
            vec![
                "scripts/appfw product mobile-test --json",
                "scripts/appfw product validate --json",
            ],
        );
    } else if rule_rel == "mobile/package.json"
        || rule_rel == "mobile/app.json"
        || rule_rel == "mobile/eas.json"
        || rule_rel == "mobile/tsconfig.json"
        || rule_rel.starts_with("mobile/app/")
        || rule_rel.starts_with("mobile/__tests__/")
        || rule_rel.starts_with("mobile/src/features/")
        || rule_rel.starts_with("mobile/src/components/")
        || rule_rel.starts_with("mobile/src/lib/")
        || rule_rel.starts_with("mobile/src/test/")
        || rule_rel == "mobile/README.md"
    {
        set(
            &mut explanation,
            "product_mobile_scaffold_source",
            "human_owned",
            true,
            "docs/frontend/mobile-react-native.md and mobile/.appfw-mobile/ownership.json",
            "Product mobile source is editable in the product workspace; keep it aligned with the retained mobile plan, canonical generated mobile contract, PDS native tokens, and non-authoritative mobile-test diagnostics.",
            vec![
                "scripts/appfw product mobile-test --json",
                "scripts/appfw product validate --json",
            ],
        );
    } else if rule_rel.starts_with("appfw_runtime/src/")
        || rule_rel.starts_with("appfw_runtime/tests/")
    {
        set(
            &mut explanation,
            "runtime_package_source",
            "framework_source",
            true,
            "appfw-runtime package and docs/architecture/framework-packaging.md",
            "Edit here when changing stable product-facing runtime contracts.",
            vec![
                "cargo test --locked --manifest-path appfw_runtime/Cargo.toml",
                "scripts/appfw framework test --fast",
            ],
        );
    } else if rule_rel.starts_with("appfw_saas_") && rule_rel.contains("/src/") {
        set(
            &mut explanation,
            "saas_package_source",
            "framework_source",
            true,
            "appfw-saas-* packages and docs/runtime/saas-certification.md",
            "Edit here when changing shared SaaS connector transport, evidence, or certification contracts.",
            vec![
                "cargo test --locked -p <changed SaaS package>",
                "scripts/appfw framework provider-test --provider <provider> --area saas-read --plan --json",
                "scripts/appfw framework test --fast",
            ],
        );
    } else if is_retired_backend_provider_certification_tool(&rule_rel) {
        set(
            &mut explanation,
            "retired_backend_provider_certification_tooling",
            "framework_source",
            true,
            "appfw-runtime provider certification plus appfw-cli exporter.",
            "These backend-owned certification adapter paths were intentionally retired; use appfw_cli/src/bin/provider_certification_export.rs instead.",
            vec![
                "cargo test --locked --manifest-path appfw_cli/Cargo.toml",
                "scripts/appfw framework docs-check --json",
                "scripts/appfw framework test",
            ],
        );
    } else if rule_rel.starts_with("appfw_provider_")
        && rule_rel.ends_with("/docs/vendor-contract.md")
    {
        set(
            &mut explanation,
            "provider_vendor_contract_doc",
            "framework_source",
            true,
            "docs/runtime/saas-connectors.md and appfw_provider_<vendor>/docs/vendor-contract.md",
            "Keep vendor contract docs transcribed from provider crate constants and retained evidence; write absent — evidence-gated when authenticated exports are missing.",
            vec![
                "scripts/appfw framework docs-check --json",
                "scripts/appfw framework provider-graduation --json",
            ],
        );
    } else if rule_rel.starts_with("appfw_provider_") && rule_rel.contains("/src/") {
        set(
            &mut explanation,
            "provider_package_source",
            "framework_source",
            true,
            "appfw-provider-* packages and docs/architecture/framework-packaging.md",
            "Edit here when changing framework-owned provider implementation packages.",
            vec![
                "cargo test --locked -p <changed provider package>",
                "scripts/appfw framework test --fast",
            ],
        );
    } else if is_backend_runtime_facade(&rule_rel) {
        set(
            &mut explanation,
            "backend_runtime_facade",
            "framework_source",
            true,
            "appfw-runtime package and docs/architecture/framework-packaging.md",
            "Keep this facade thin; framework behavior belongs in appfw-runtime.",
            vec![
                "cargo test --locked --manifest-path appfw_runtime/Cargo.toml",
                "scripts/appfw framework boundary-check --json",
                "scripts/appfw framework test",
            ],
        );
    } else if is_generated_backend_route_or_schema(&rule_rel) {
        set(
            &mut explanation,
            "generated_backend_rust",
            "generated",
            false,
            "app_gen/_templates and .appfw/model.",
            "Change the template/config source and regenerate.",
            vec![
                "scripts/appfw product generate",
                "scripts/appfw product generate --check --json",
            ],
        );
    } else if is_human_handler_impl(&rule_rel) {
        set(
            &mut explanation,
            "human_owned_handler_extension",
            "human_owned",
            true,
            "backend/src/handlers/<schema>/<entity>.rs",
            "This is an extension point intended for custom business logic.",
            vec![
                "scripts/appfw product validate --json",
                "scripts/appfw product test",
            ],
        );
    } else if rule_rel.starts_with("backend/src/services/") {
        set(
            &mut explanation,
            "human_owned_service_extension",
            "human_owned",
            true,
            "backend/src/services",
            "This is the product-owned service layer for durable business workflows.",
            vec![
                "scripts/appfw product boundary-check --json",
                "scripts/appfw product test",
            ],
        );
    } else if rule_rel == "backend/src/product_api.rs" {
        set(
            &mut explanation,
            "backend_extension_api_facade",
            "framework_source",
            true,
            "docs/reference/product-extension-api.md and appfw-runtime exports",
            "Edit this facade only when changing the stable product extension boundary or its guard tests.",
            vec![
                "scripts/appfw product boundary-check --json",
                "scripts/appfw product test",
            ],
        );
    } else if rule_rel.starts_with("backend/config/generated/") {
        set(
            &mut explanation,
            "generated_backend_config",
            "generated",
            false,
            ".appfw/model",
            "Edit .appfw/manifest.yaml or .appfw/model and republish generated runtime config.",
            vec![
                "scripts/appfw product validate --json",
                "scripts/appfw product generate --check --json",
            ],
        );
    } else if rule_rel == "backend/src/operations/mod.rs" {
        set(
            &mut explanation,
            "generated_operation_module",
            "generated",
            false,
            "app_gen/_templates/backend/operations/_mod.j2 and .appfw/model.",
            "Change the operation module template or app model source, then regenerate.",
            vec![
                "scripts/appfw product generate",
                "scripts/appfw product generate --check --json",
                "scripts/appfw product test",
            ],
        );
    } else if rule_rel == "backend/src/operations/generated.rs" {
        set(
            &mut explanation,
            "generated_operation_dispatcher",
            "generated",
            false,
            "app_gen/_templates/backend/operations/_operations.j2 and .appfw/model.",
            "Change the operation template or app model source, then regenerate.",
            vec![
                "scripts/appfw product generate",
                "scripts/appfw product generate --check --json",
                "scripts/appfw product test",
            ],
        );
    } else if rule_rel.starts_with("backend/src/data/clients/") {
        set(
            &mut explanation,
            "provider_runtime_source",
            "framework_source",
            true,
            "backend/src/data/clients",
            "Provider runtime changes must preserve certification semantics.",
            vec![
                "scripts/appfw product validate --json",
                "scripts/appfw product test",
            ],
        );
    } else if rule_rel.starts_with("backend/src/mcp/")
        || rule_rel == "backend/src/main.rs"
        || rule_rel == "backend/src/lib.rs"
    {
        set(
            &mut explanation,
            "runtime_source",
            "framework_source",
            true,
            "backend/src",
            "Runtime changes must preserve generated API, access-control, agentic MCP, and provider contracts.",
            vec![
                "scripts/appfw framework validate --json",
                "scripts/appfw framework test",
            ],
        );
    } else if rule_rel.starts_with("backend/src/config/")
        || rule_rel.starts_with("backend/src/data/")
    {
        set(
            &mut explanation,
            "runtime_source",
            "framework_source",
            true,
            "backend/src",
            "Runtime changes must preserve generated API, access-control, and provider contracts.",
            vec![
                "scripts/appfw framework validate --json",
                "scripts/appfw framework test",
            ],
        );
    } else if rule_rel.starts_with("appfw_test/src/policy") {
        set(
            &mut explanation,
            "policy_test_harness_source",
            "framework_source",
            true,
            "appfw_test/src/policy",
            "Keep the Rego verifier engine and shared access contract in the framework package; product rego_test crates should own fixtures and product assertions only.",
            vec![
                "scripts/appfw product policy-test --json",
                "scripts/appfw product test",
            ],
        );
    } else if rule_rel.starts_with("rego_test/") && !rule_rel.starts_with("rego_test/target/") {
        set(
            &mut explanation,
            "policy_test_fixture",
            "human_owned",
            true,
            "rego_test fixtures and product assertions plus appfw_test/src/policy",
            "Product policy tests own fixtures and product-specific assertions; shared verifier semantics belong in appfw_test/src/policy.",
            vec![
                "scripts/appfw product policy-test --json",
                "scripts/appfw product test",
            ],
        );
    } else if rule_rel.starts_with("appfw_test/src/")
        || rule_rel.starts_with("api_tests/src/harness/")
        || rule_rel == "api_tests/src/provider_contracts.rs"
        || rule_rel == "api_tests/src/provider_semantic_contracts.rs"
        || rule_rel == "api_tests/src/provider_schema_contracts.rs"
        || rule_rel == "api_tests/src/lib.rs"
        || rule_rel == "api_tests/src/main.rs"
        || rule_rel == "api_tests/Cargo.toml"
    {
        set(
            &mut explanation,
            "api_test_harness_source",
            "framework_source",
            true,
            "api_tests harness and provider certification modules.",
            "Keep generated product scenarios under api_tests/src/schemas and provider certification in framework-owned modules.",
            vec![
                "scripts/appfw framework test",
                "scripts/appfw framework provider-test --provider <provider>",
            ],
        );
    } else if rule_rel.starts_with("database/src/") {
        set(
            &mut explanation,
            "database_package_source",
            "framework_source",
            true,
            "database package and app_gen database templates",
            "Database package changes must preserve generated DDL/package semantics across provider-specific outputs.",
            vec![
                "cargo test --locked --manifest-path database/Cargo.toml",
                "scripts/appfw framework generate --check --json",
                "scripts/appfw framework test --fast",
            ],
        );
    } else if rule_rel.starts_with("database/_pkg/")
        || rule_rel.starts_with("api_tests/src/schemas/")
    {
        set(
            &mut explanation,
            "generated_artifact",
            "generated",
            false,
            ".appfw/model and app_gen/_templates",
            "Change the generator/config source and regenerate.",
            vec![
                "scripts/appfw product generate",
                "scripts/appfw product generate --check --json",
            ],
        );
    } else if rule_rel == "podman-compose.yml" {
        set(
            &mut explanation,
            "generated_dev_infra",
            "generated",
            false,
            ".appfw/manifest.yaml and .appfw/model/data_sources/_res.yaml",
            "Change app topology or data-source config, then regenerate local dev infra.",
            vec![
                "scripts/appfw product topology --json",
                "scripts/appfw product generate",
                "scripts/appfw product generate --check --json",
            ],
        );
    } else if rule_rel.starts_with("admin_ui/") || rule_rel.starts_with("backend/src/admin_ui.rs") {
        set(
            &mut explanation,
            "admin_ui_source",
            "framework_source",
            true,
            "admin_ui and backend/src/admin_ui.rs",
            "Keep the admin UI runtime model-driven and rebuild when frontend files change.",
            vec![
                "cd admin_ui && npm run build",
                "scripts/appfw framework validate --json",
                "scripts/appfw framework test",
            ],
        );
    } else if rule_rel.starts_with("appfw_ui/pds_health/") {
        set(
            &mut explanation,
            "pds_health_design_system_source",
            "framework_source",
            true,
            "appfw_ui/pds_health and docs/frontend/pds-health-design-system.md",
            "Keep PDS Health tokens and reusable frontend design-system assets framework-owned; product apps should consume them through the scaffold.",
            vec![
                "scripts/appfw framework docs-check --json",
                "scripts/appfw framework validate --json",
                "scripts/appfw framework golden-downstream --json",
            ],
        );
    } else if rule_rel == "bitbucket-pipelines.yml" || rule_rel.starts_with("scripts/ci/") {
        set(
            &mut explanation,
            "ci_cd_source",
            "framework_source",
            true,
            "bitbucket-pipelines.yml and scripts/ci",
            "CI/CD changes should preserve release gate evidence and provider certification artifacts.",
            vec![
                "scripts/appfw framework validate --json",
                "scripts/appfw framework docs-check --json",
            ],
        );
    } else if rule_rel.starts_with("agent_skills/") {
        set(
            &mut explanation,
            "agent_skill_source",
            "framework_source",
            true,
            "agent_skills/README.md and docs/architecture/concerns/maintainability.md",
            "Keep skills concise and route deep guidance to canonical docs.",
            vec![
                "scripts/appfw framework docs-check --json",
                "scripts/appfw framework validate --json",
            ],
        );
    } else if rule_rel.starts_with("scripts/")
        || rule_rel.starts_with("appfw_cli/src/")
        || rule_rel.starts_with("appfw_cli/tests/")
    {
        set(
            &mut explanation,
            "workflow_cli_source",
            "framework_source",
            true,
            "appfw_cli and scripts",
            "Keep root CLI behavior aligned with docs/reference/cli.md and agent handoff expectations.",
            vec![
                "cargo test -p appfw-cli",
                "bash -n scripts/appfw",
                "scripts/appfw framework docs-check --json",
            ],
        );
    } else if rule_rel.starts_with("docs/")
        || rule_rel == "README.md"
        || rule_rel == "AGENTS.md"
        || rule_rel == "CLAUDE.md"
        || rule_rel.ends_with("/README.md")
    {
        set(
            &mut explanation,
            "documentation",
            "human_owned",
            true,
            "docs and repository markdown.",
            "Keep docs aligned with CLI behavior and generated-boundary rules.",
            vec![
                "scripts/appfw framework validate --json",
                "scripts/appfw framework docs-check --json",
            ],
        );
    }

    if !absolute.starts_with(repo_root) {
        explanation.evidence.push(
            "path is outside the current app-framework repository; static rules may not apply"
                .to_string(),
        );
    }

    explanation
}

fn is_backend_runtime_facade(rel: &str) -> bool {
    rel == "backend/src/app_state.rs"
        || rel == "backend/src/cors.rs"
        || rel == "backend/src/routes/app_error.rs"
        || rel == "backend/src/routes/graphiql.rs"
        || rel == "backend/src/handlers/auth/claims.rs"
        || rel == "backend/src/handlers/auth/jwt_extractor.rs"
        || rel == "backend/src/handlers/auth/okta_verifier.rs"
        || rel == "backend/src/handlers/auth/user_auth.rs"
        || rel.starts_with("backend/src/observability/")
}

fn is_retired_backend_provider_certification_tool(rel: &str) -> bool {
    matches!(
        rel,
        "backend/src/provider_certification.rs"
            | "backend/src/bin/provider_certification_export.rs"
    )
}

fn is_generated_backend_route_or_schema(rel: &str) -> bool {
    if rel == "backend/src/handlers/mod.rs" || rel.starts_with("backend/src/schemas/") {
        return true;
    }

    let parts = rel.split('/').collect::<Vec<_>>();
    if parts.len() == 4 && parts[0] == "backend" && parts[1] == "src" && parts[2] == "routes" {
        return parts[3].ends_with(".rs");
    }

    parts.len() == 5
        && parts[0] == "backend"
        && parts[1] == "src"
        && parts[2] == "handlers"
        && matches!(parts[4], "mod.rs" | "generated.rs")
}

fn is_human_handler_impl(rel: &str) -> bool {
    let parts = rel.split('/').collect::<Vec<_>>();
    parts.len() == 5
        && parts[0] == "backend"
        && parts[1] == "src"
        && parts[2] == "handlers"
        && parts[4].ends_with(".rs")
        && parts[4] != "mod.rs"
        && parts[4] != "generated.rs"
}

fn product_example_rule_path(rel: &str) -> Option<String> {
    let parts = rel.split('/').collect::<Vec<_>>();
    if parts.len() >= 4 && parts[0] == "examples" && parts[1] == "products" {
        Some(parts[3..].join("/"))
    } else {
        None
    }
}

fn ownership_text(explanation: &OwnershipExplanation) -> String {
    format!(
        "Ownership: {}\nPath: {}\nClass: {}\nSafe to edit: {}\nSource of truth: {}\nGuidance: {}\nVerify:\n  {}\nEvidence:\n  {}",
        explanation.ownership,
        explanation.relative_path,
        explanation.classification,
        explanation.safe_to_edit,
        explanation.source_of_truth,
        explanation.edit_guidance,
        explanation.verification.join("\n  "),
        explanation.evidence.join("\n  "),
    )
}

fn artifact_manifest(repo_root: &Path) -> Result<Vec<ArtifactRecord>, String> {
    let path = repo_root.join(".appfw/target/appfw/artifacts.json");
    if !path.exists() {
        return Ok(Vec::new());
    }
    let contents = fs::read_to_string(&path)
        .map_err(|err| format!("failed to read artifact manifest {}: {err}", path.display()))?;
    serde_json::from_str(&contents).map_err(|err| {
        format!(
            "failed to parse artifact manifest {}: {err}",
            path.display()
        )
    })
}

#[derive(Debug, Serialize)]
struct ConfigExplanation {
    command: &'static str,
    ok: bool,
    target: String,
    shape_name: String,
    path_pattern: String,
    shape: String,
    purpose: String,
    field: Option<ConfigField>,
    fields: Vec<ConfigField>,
    lints: Vec<String>,
    source: String,
    verification: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct ConfigContract {
    shapes: BTreeMap<String, ConfigShape>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct ConfigShape {
    name: String,
    path_pattern: String,
    shape: String,
    purpose: String,
    #[serde(default)]
    fields: Vec<ConfigField>,
    #[serde(default)]
    lints: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct ConfigField {
    name: String,
    value_type: String,
    required: bool,
    default: Option<Value>,
    #[serde(default)]
    allowed_values: Vec<Value>,
    purpose: String,
}

fn explain_config(repo_root: &Path, target: &str) -> Result<ConfigExplanation, String> {
    let contract = load_config_contract(repo_root)?;
    let (shape_name, field) = if looks_like_path(target) {
        (shape_for_config_path(target)?, None)
    } else {
        resolve_config_key(&contract, target)?
    };
    let shape = contract
        .shapes
        .get(&shape_name)
        .ok_or_else(|| format!("config contract shape not found: {shape_name}"))?;

    Ok(ConfigExplanation {
        command: "explain config",
        ok: true,
        target: target.to_string(),
        shape_name: shape.name.clone(),
        path_pattern: shape.path_pattern.clone(),
        shape: shape.shape.clone(),
        purpose: shape.purpose.clone(),
        field,
        fields: shape.fields.clone(),
        lints: shape.lints.clone(),
        source: ".appfw/target/appfw/config_contract.json".to_string(),
        verification: vec!["scripts/appfw product validate --json".to_string()],
    })
}

fn load_config_contract(repo_root: &Path) -> Result<ConfigContract, String> {
    let path = repo_root.join(".appfw/target/appfw/config_contract.json");
    if !path.exists() {
        return Err(format!(
            "config contract is missing at {}; run scripts/appfw product validate --json",
            path.display()
        ));
    }
    let contents = fs::read_to_string(&path)
        .map_err(|err| format!("failed to read {}: {err}", path.display()))?;
    serde_json::from_str(&contents)
        .map_err(|err| format!("failed to parse {}: {err}", path.display()))
}

fn looks_like_path(value: &str) -> bool {
    value.contains('/') || value.ends_with(".yaml") || value.ends_with(".yml")
}

fn shape_for_config_path(path: &str) -> Result<String, String> {
    let path = path.replace('\\', "/");
    if path.contains("/data_sources/") || path.ends_with(".appfw/model/data_sources/_res.yaml") {
        Ok("data_source".to_string())
    } else if path.contains("/relationships/") {
        Ok("relationship".to_string())
    } else if path.contains("/entity_types/") {
        Ok("entity_type".to_string())
    } else if path.contains("/gql_enum_types/") {
        Ok("enum_type".to_string())
    } else if path.contains("/seeds/") {
        Ok("seed_group".to_string())
    } else if path.contains("/tests/") {
        Ok("api_test".to_string())
    } else if path.ends_with("/_res.yaml") || path.ends_with("/schema.yaml") {
        Ok("schema".to_string())
    } else {
        Err(format!(
            "could not map config path to a contract shape: {path}"
        ))
    }
}

fn resolve_config_key(
    contract: &ConfigContract,
    key: &str,
) -> Result<(String, Option<ConfigField>), String> {
    let parts = key
        .split('.')
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>();
    let first = parts
        .first()
        .ok_or_else(|| "config key must not be empty".to_string())?;
    let mut shape_name = config_shape_alias(first)
        .ok_or_else(|| format!("unknown config shape alias: {first}"))?
        .to_string();
    let mut last_field = None;

    for part in parts.into_iter().skip(1) {
        let shape = contract
            .shapes
            .get(&shape_name)
            .ok_or_else(|| format!("config contract shape not found: {shape_name}"))?;
        let field = shape
            .fields
            .iter()
            .find(|field| field.name == part)
            .cloned()
            .ok_or_else(|| format!("shape `{shape_name}` has no field `{part}`"))?;
        let nested = nested_shape_name(&field.value_type).map(str::to_string);
        last_field = Some(field);
        if let Some(nested) = nested {
            if contract.shapes.contains_key(&nested) {
                shape_name = nested;
            }
        }
    }

    Ok((shape_name, last_field))
}

fn config_shape_alias(value: &str) -> Option<&'static str> {
    match value {
        "api_test" | "api_tests" | "tests" => Some("api_test"),
        "data_source" | "data_sources" => Some("data_source"),
        "entity_type" | "entity_types" | "entities" => Some("entity_type"),
        "enum_type" | "enum_types" | "gql_enum_types" => Some("enum_type"),
        "foreign_key" | "foreign_keys" => Some("foreign_key"),
        "many_to_many_property" => Some("many_to_many_property"),
        "nav_by_fk_property" => Some("nav_by_fk_property"),
        "property" | "properties" | "props" => Some("property"),
        "relationship" | "relationships" => Some("relationship"),
        "schema" | "schemas" => Some("schema"),
        "seed" | "seeds" | "seed_group" => Some("seed_group"),
        _ => None,
    }
}

fn nested_shape_name(value_type: &str) -> Option<&str> {
    if let Some(inner) = value_type
        .strip_prefix("array<")
        .and_then(|value| value.strip_suffix('>'))
    {
        Some(inner)
    } else if value_type.contains('_') {
        Some(value_type)
    } else {
        None
    }
}

fn config_text(explanation: &ConfigExplanation) -> String {
    let mut out = format!(
        "Config: {}\nShape: {} ({})\nPath pattern: {}\nPurpose: {}\nSource: {}",
        explanation.target,
        explanation.shape_name,
        explanation.shape,
        explanation.path_pattern,
        explanation.purpose,
        explanation.source
    );
    if let Some(field) = &explanation.field {
        out.push_str(&format!(
            "\nField: {} ({})\nRequired: {}\nDefault: {}\nPurpose: {}",
            field.name,
            field.value_type,
            field.required,
            field
                .default
                .as_ref()
                .map(Value::to_string)
                .unwrap_or_else(|| "none".to_string()),
            field.purpose
        ));
    } else {
        out.push_str("\nFields:");
        for field in &explanation.fields {
            out.push_str(&format!(
                "\n  {}: {}{}",
                field.name,
                field.value_type,
                if field.required { " required" } else { "" }
            ));
        }
    }
    if !explanation.lints.is_empty() {
        out.push_str(&format!("\nLints:\n  {}", explanation.lints.join("\n  ")));
    }
    out.push_str(&format!(
        "\nVerify:\n  {}",
        explanation.verification.join("\n  ")
    ));
    out
}

#[derive(Debug, Serialize)]
struct BoundaryCheckReport {
    command: &'static str,
    ok: bool,
    roots: CommandRootReport,
    checked_files: usize,
    product_provider_sources: Vec<ProductProviderSourceReport>,
    violations: Vec<BoundaryCheckViolation>,
    report_path: String,
}

#[derive(Clone, Debug, Serialize)]
struct ProductProviderSourceReport {
    product_root: String,
    active_providers: Vec<String>,
    sources: Vec<ProductProviderSource>,
    dependencies: Vec<ProductProviderDependency>,
}

#[derive(Clone, Debug, Serialize)]
struct ProductProviderSource {
    provider: String,
    path: String,
    active: bool,
    rust_files: usize,
}

#[derive(Clone, Debug, Serialize)]
struct ProductProviderDependency {
    provider: String,
    dependency: String,
    active: bool,
}

#[derive(Clone, Debug, Serialize)]
struct BoundaryCheckViolation {
    path: String,
    rule: String,
    detail: String,
    symbol: Option<String>,
}

#[derive(Clone, Debug)]
struct BoundaryViolation {
    rule: &'static str,
    path: String,
}

#[derive(Clone, Debug)]
struct RetiredProductTemplateSurface {
    relative_path: &'static str,
    detail: &'static str,
}

#[derive(Clone, Debug)]
struct RetiredFrameworkRootSurface {
    relative_path: &'static str,
    detail: &'static str,
}

#[derive(Clone, Debug)]
struct ProviderTemplateSurface {
    provider: &'static str,
    data_source_type: &'static str,
    dependency_key: &'static str,
    package_name: &'static str,
    relative_path: &'static str,
}

const RETIRED_PRODUCT_TEMPLATE_SURFACES: &[RetiredProductTemplateSurface] = &[
    RetiredProductTemplateSurface {
        relative_path: "backend/src/app_state.rs",
        detail: "runtime auth/app state wiring belongs in appfw-runtime",
    },
    RetiredProductTemplateSurface {
        relative_path: "backend/src/cors.rs",
        detail: "CORS policy helpers belong in appfw-runtime",
    },
    RetiredProductTemplateSurface {
        relative_path: "backend/src/observability/mod.rs",
        detail: "observability helpers belong in appfw-runtime",
    },
    RetiredProductTemplateSurface {
        relative_path: "backend/src/config/access.rs",
        detail: "access config helpers belong in appfw-runtime",
    },
    RetiredProductTemplateSurface {
        relative_path: "backend/src/config/secrets.rs",
        detail: "secret resolution helpers belong in appfw-runtime",
    },
    RetiredProductTemplateSurface {
        relative_path: "backend/src/config/security.rs",
        detail: "security config helpers belong in appfw-runtime",
    },
    RetiredProductTemplateSurface {
        relative_path: "backend/src/data/audit.rs",
        detail: "audit helper contracts belong in appfw-runtime",
    },
    RetiredProductTemplateSurface {
        relative_path: "backend/src/data/filter_capabilities.rs",
        detail: "filter capability contracts belong in appfw-runtime",
    },
    RetiredProductTemplateSurface {
        relative_path: "backend/src/data/filtering.rs",
        detail: "filter parsing/runtime semantics belong in appfw-runtime",
    },
    RetiredProductTemplateSurface {
        relative_path: "backend/src/data/json_utils.rs",
        detail: "JSON conversion helpers belong in appfw-runtime",
    },
    RetiredProductTemplateSurface {
        relative_path: "backend/src/data/query_cost.rs",
        detail: "query cost policy belongs in appfw-runtime",
    },
    RetiredProductTemplateSurface {
        relative_path: "backend/src/data/validation.rs",
        detail: "record validation helpers belong in appfw-runtime",
    },
    RetiredProductTemplateSurface {
        relative_path: "backend/src/data/snake.rs",
        detail: "identifier normalization helpers belong in appfw-runtime",
    },
    RetiredProductTemplateSurface {
        relative_path: "backend/src/data/clients/contract_tests.rs",
        detail: "provider contract tests belong in framework/provider packages",
    },
    RetiredProductTemplateSurface {
        relative_path: "backend/src/data/clients/provider_capabilities.rs",
        detail: "provider capability contracts belong in appfw-runtime",
    },
    RetiredProductTemplateSurface {
        relative_path: "backend/src/data/clients/provider_error.rs",
        detail: "provider error classification belongs in provider/runtime packages",
    },
    RetiredProductTemplateSurface {
        relative_path: "backend/src/data/clients/test_support.rs",
        detail: "provider test support belongs in framework/provider packages",
    },
    RetiredProductTemplateSurface {
        relative_path: "backend/src/data/rules/computed.rs",
        detail: "computed field rule evaluation belongs in appfw-runtime",
    },
    RetiredProductTemplateSurface {
        relative_path: "backend/src/data/rules/timezone.rs",
        detail: "timezone rule evaluation belongs in appfw-runtime",
    },
    RetiredProductTemplateSurface {
        relative_path: "backend/src/data/rules/version.rs",
        detail: "concurrency version rule evaluation belongs in appfw-runtime",
    },
    RetiredProductTemplateSurface {
        relative_path: "backend/src/mcp/operation.rs",
        detail: "MCP operation helpers belong in appfw-runtime and generated operation adapters",
    },
];

const RETIRED_FRAMEWORK_ROOT_SURFACES: &[RetiredFrameworkRootSurface] = &[
    RetiredFrameworkRootSurface {
        relative_path: "backend/src/app_state.rs",
        detail: "runtime auth/app state wiring belongs in appfw-runtime",
    },
    RetiredFrameworkRootSurface {
        relative_path: "backend/src/cors.rs",
        detail: "CORS policy helpers belong in appfw-runtime",
    },
    RetiredFrameworkRootSurface {
        relative_path: "backend/src/observability/mod.rs",
        detail: "observability helpers belong in appfw-runtime",
    },
    RetiredFrameworkRootSurface {
        relative_path: "backend/src/config/access.rs",
        detail: "access config helpers belong in appfw-runtime",
    },
    RetiredFrameworkRootSurface {
        relative_path: "backend/src/config/secrets.rs",
        detail: "secret resolution helpers belong in appfw-runtime",
    },
    RetiredFrameworkRootSurface {
        relative_path: "backend/src/config/security.rs",
        detail: "security config helpers belong in appfw-runtime",
    },
    RetiredFrameworkRootSurface {
        relative_path: "backend/src/data/audit.rs",
        detail: "audit helper contracts belong in appfw-runtime",
    },
    RetiredFrameworkRootSurface {
        relative_path: "backend/src/data/filter_capabilities.rs",
        detail: "filter capability contracts belong in appfw-runtime",
    },
    RetiredFrameworkRootSurface {
        relative_path: "backend/src/data/filtering.rs",
        detail: "filter parsing/runtime semantics belong in appfw-runtime",
    },
    RetiredFrameworkRootSurface {
        relative_path: "backend/src/data/json_utils.rs",
        detail: "JSON conversion helpers belong in appfw-runtime",
    },
    RetiredFrameworkRootSurface {
        relative_path: "backend/src/data/query_cost.rs",
        detail: "query cost policy belongs in appfw-runtime",
    },
    RetiredFrameworkRootSurface {
        relative_path: "backend/src/data/validation.rs",
        detail: "record validation helpers belong in appfw-runtime",
    },
    RetiredFrameworkRootSurface {
        relative_path: "backend/src/data/snake.rs",
        detail: "identifier normalization helpers belong in appfw-runtime",
    },
    RetiredFrameworkRootSurface {
        relative_path: "backend/src/data/clients/provider_capabilities.rs",
        detail: "provider capability contracts belong in appfw-runtime",
    },
    RetiredFrameworkRootSurface {
        relative_path: "backend/src/data/clients/provider_error.rs",
        detail: "provider error classification belongs in provider/runtime packages",
    },
    RetiredFrameworkRootSurface {
        relative_path: "backend/src/data/rules/computed.rs",
        detail: "computed field rule evaluation belongs in appfw-runtime",
    },
    RetiredFrameworkRootSurface {
        relative_path: "backend/src/data/rules/timezone.rs",
        detail: "timezone rule evaluation belongs in appfw-runtime",
    },
    RetiredFrameworkRootSurface {
        relative_path: "backend/src/data/rules/version.rs",
        detail: "concurrency version rule evaluation belongs in appfw-runtime",
    },
    RetiredFrameworkRootSurface {
        relative_path: "backend/src/mcp/operation.rs",
        detail: "MCP operation helpers belong in appfw-runtime and generated operation adapters",
    },
];

const PROVIDER_TEMPLATE_SURFACES: &[ProviderTemplateSurface] = &[
    ProviderTemplateSurface {
        provider: "postgres",
        data_source_type: "PostgreSQL",
        dependency_key: "appfw_provider_postgres",
        package_name: "appfw-provider-postgres",
        relative_path: "backend/src/data/clients/postgres",
    },
    ProviderTemplateSurface {
        provider: "mongo",
        data_source_type: "MongoDB",
        dependency_key: "appfw_provider_mongo",
        package_name: "appfw-provider-mongo",
        relative_path: "backend/src/data/clients/mongo",
    },
    ProviderTemplateSurface {
        provider: "mssql",
        data_source_type: "MsSqlServer",
        dependency_key: "appfw_provider_mssql",
        package_name: "appfw-provider-mssql",
        relative_path: "backend/src/data/clients/mssql",
    },
    ProviderTemplateSurface {
        provider: "fabric_sql_analytics",
        data_source_type: "FabricSqlAnalytics",
        dependency_key: "appfw_provider_mssql",
        package_name: "appfw-provider-mssql",
        relative_path: "backend/src/data/clients/fabric_sql_analytics",
    },
    ProviderTemplateSurface {
        provider: "snowflake",
        data_source_type: "Snowflake",
        dependency_key: "appfw_provider_snowflake",
        package_name: "appfw-provider-snowflake",
        relative_path: "backend/src/data/clients/snowflake",
    },
];

fn harness_check_command(
    roots: &CommandRoots,
    json_output: bool,
    args: &[String],
) -> Result<(), String> {
    if !args.is_empty() {
        return Err("harness-check does not accept additional arguments".to_string());
    }

    let report = build_harness_check(roots)?;
    let ok = report.get("ok").and_then(Value::as_bool).unwrap_or(false);
    let report_path = report
        .get("artifact")
        .and_then(Value::as_str)
        .unwrap_or(".appfw/target/appfw/harness-check.json")
        .to_string();
    emit_value(json_output, &report, || harness_check_text(&report))?;
    if ok {
        Ok(())
    } else {
        let violation_count = report
            .get("violations")
            .and_then(Value::as_array)
            .map(Vec::len)
            .unwrap_or_default();
        Err(format!(
            "harness-check failed with {violation_count} violation(s); see {report_path}"
        ))
    }
}

fn build_harness_check(roots: &CommandRoots) -> Result<Value, String> {
    let profile_path = roots.app_root.join(".appfw/agent-profile.yaml");
    let artifact_path = roots.report_root.join("harness-check.json");
    let relative_artifact = relative_path(&roots.app_root, &artifact_path);
    let release_artifact_path = roots
        .framework_root
        .join("target/appfw/wave2/u2-harness-check.json");
    let relative_release_artifact = relative_path(&roots.framework_root, &release_artifact_path);
    let threat_model_path = roots
        .framework_root
        .join("docs/architecture/concerns/agentic-threat-model.md");
    let governed_write_evidence_path = roots
        .app_root
        .join("target/appfw/governed-write-evidence.json");
    let governed_write_posture_path = roots
        .app_root
        .join("target/appfw/governed-write-posture.json");
    let governed_write_evidence = read_json_value_if_present(&governed_write_evidence_path)?;
    let governed_write_posture = read_json_value_if_present(&governed_write_posture_path)?;
    let profile = read_yaml_value_if_present(&profile_path)?;
    let mut checks = Vec::new();
    let mut violations = Vec::new();
    let mut not_applicable = Vec::new();
    let mut profile_summary = json!({});

    add_harness_check(
        &mut checks,
        &mut violations,
        "profile-present",
        profile.is_some(),
        ".appfw/agent-profile.yaml must define the product agent sandbox profile.",
    );

    if let Some(profile) = profile.as_ref() {
        let name = yaml_string_at(profile, &["name"]).unwrap_or_default();
        let allowed_commands = harness_allowed_commands(profile);
        let writable_paths = harness_writable_paths(profile);
        let review_checkpoints = yaml_sequence_at(profile, &["review_checkpoints"])
            .map(|items| yaml_string_values(items))
            .unwrap_or_default();
        let risk_acceptance = yaml_sequence_at(profile, &["risk_acceptance"])
            .map(|items| yaml_string_values(items))
            .unwrap_or_default();
        let handoff_required = yaml_bool_at(profile, &["handoff", "required"]).unwrap_or(false);
        let handoff_artifact =
            yaml_string_at(profile, &["handoff", "artifact"]).unwrap_or_default();
        let requires_report_root = allowed_commands.iter().any(|command| {
            command
                .get("command")
                .and_then(Value::as_str)
                .is_some_and(harness_command_writes_product_report)
        });
        let allows_handoff = allowed_commands.iter().any(|command| {
            command
                .get("command")
                .and_then(Value::as_str)
                .is_some_and(harness_command_is_product_handoff)
        });
        let network_policy =
            yaml_string_at(profile, &["network_policy"]).unwrap_or_else(|| "missing".to_string());
        let live_service_policy = yaml_string_at(profile, &["live_service_policy"])
            .unwrap_or_else(|| "missing".to_string());
        let threat_controls = yaml_sequence_at(profile, &["threat_controls"])
            .map(|items| yaml_string_values(items))
            .unwrap_or_default();
        let sensitive_capabilities = harness_sensitive_capabilities(profile);

        let commands_product_scoped = !allowed_commands.is_empty()
            && allowed_commands.iter().all(|command| {
                command
                    .get("namespace")
                    .and_then(Value::as_str)
                    .is_some_and(|namespace| namespace == "product")
                    && command
                        .get("command")
                        .and_then(Value::as_str)
                        .is_some_and(|command| command.starts_with("scripts/appfw product "))
            });
        add_harness_check(
            &mut checks,
            &mut violations,
            "commands-product-scoped",
            commands_product_scoped,
            "Every allowed command must stay in the scripts/appfw product namespace.",
        );

        let writable_paths_safe = !writable_paths.is_empty()
            && writable_paths.iter().all(|entry| {
                entry
                    .get("path")
                    .and_then(Value::as_str)
                    .is_some_and(harness_safe_product_write_path)
            });
        add_harness_check(
            &mut checks,
            &mut violations,
            "no-framework-source-writes",
            writable_paths_safe,
            "Writable paths must be relative product-owned surfaces and must not point at framework source.",
        );

        let resolved_report_root = relative_path(&roots.app_root, &roots.report_root);
        let report_root_covered = !requires_report_root
            || writable_paths.iter().any(|entry| {
                entry
                    .get("path")
                    .and_then(Value::as_str)
                    .is_some_and(|path| path.trim() == resolved_report_root)
            });
        add_harness_check(
            &mut checks,
            &mut violations,
            "command-report-root",
            report_root_covered,
            "Profiles whose declared commands write product reports must include the resolved product report root in writable_paths.",
        );

        add_harness_check(
            &mut checks,
            &mut violations,
            "network-fail-closed",
            network_policy == "disabled",
            "network_policy must be disabled unless a future harness lane adds explicit reviewable network grants.",
        );
        add_harness_check(
            &mut checks,
            &mut violations,
            "live-services-disabled-or-approved",
            live_service_policy == "disabled",
            "live_service_policy must be disabled unless the agent profile is reviewed for live service use.",
        );

        let threat_controls_mapped = threat_model_path.is_file()
            && ["ASI01", "ASI02", "ASI03"]
                .iter()
                .all(|control| threat_controls.iter().any(|value| value == control));
        add_harness_check(
            &mut checks,
            &mut violations,
            "g3-threat-controls-mapped",
            threat_controls_mapped,
            "Profile threat_controls must map to ASI01, ASI02, and ASI03 from the G3 threat model.",
        );

        let governed_write_enabled = sensitive_capabilities
            .get("saas_governed_write")
            .and_then(Value::as_bool)
            .unwrap_or(false);
        let governed_write_evidence_valid = governed_write_evidence
            .as_ref()
            .is_some_and(valid_governed_write_evidence);
        let governed_write_posture_valid = governed_write_posture
            .as_ref()
            .is_some_and(valid_governed_write_posture);
        let governed_write_allowed = !governed_write_enabled
            || (governed_write_evidence_valid && governed_write_posture_valid);
        add_harness_check(
            &mut checks,
            &mut violations,
            "governed-writes-disabled-unless-g1-evidence",
            governed_write_allowed,
            "saas_governed_write cannot be enabled without valid target/appfw/governed-write-evidence.json and target/appfw/governed-write-posture.json.",
        );

        let dangerous_capabilities_disabled =
            ["mcp", "kafka", "release"].iter().all(|capability| {
                !sensitive_capabilities
                    .get(*capability)
                    .and_then(Value::as_bool)
                    .unwrap_or(false)
            });
        add_harness_check(
            &mut checks,
            &mut violations,
            "sensitive-capabilities-fail-closed",
            dangerous_capabilities_disabled,
            "mcp, kafka, and release capabilities must be disabled for the default product harness profile.",
        );

        let handoff_ok = !allows_handoff
            || (handoff_required
                && !handoff_artifact.trim().is_empty()
                && review_checkpoints.iter().any(|value| value == "handoff"));
        add_harness_check(
            &mut checks,
            &mut violations,
            "handoff-required",
            handoff_ok,
            "Profiles that allow product handoff must set handoff.required and include the handoff review checkpoint.",
        );

        let handoff_contract_ok = !allows_handoff
            || (handoff_artifact.trim() == "target/appfw/agent-handoff.json"
                && writable_paths.iter().any(|entry| {
                    entry
                        .get("path")
                        .and_then(Value::as_str)
                        .is_some_and(|path| path.trim() == "target/appfw")
                }));
        add_harness_check(
            &mut checks,
            &mut violations,
            "handoff-contract",
            handoff_contract_ok,
            "handoff.artifact must equal target/appfw/agent-handoff.json and writable_paths must include target/appfw for lifecycle handoff evidence.",
        );

        if governed_write_enabled && governed_write_evidence_valid && governed_write_posture_valid {
            not_applicable.push(json!({
                "name": "governed-write-default-disabled",
                "reason": "governed-write evidence and posture exist, so a future profile may enable this capability with review"
            }));
        }

        profile_summary = json!({
            "name": name,
            "allowed_commands": allowed_commands,
            "writable_paths": writable_paths,
            "network_policy": network_policy,
            "live_service_policy": live_service_policy,
            "sensitive_capabilities": sensitive_capabilities,
            "review_checkpoints": review_checkpoints,
            "handoff": {
                "required": handoff_required,
                "artifact": handoff_artifact
            },
            "threat_controls": threat_controls,
            "risk_acceptance": risk_acceptance
        });
    }

    let ok = violations.is_empty();
    let report = json!({
        "schema_version": 1,
        "command": "harness-check",
        "lane": "U2",
        "ok": ok,
        "generated_at": Utc::now().to_rfc3339(),
        "artifact": relative_artifact,
        "roots": roots.report(),
        "inputs": {
            "profile_path": ".appfw/agent-profile.yaml",
            "threat_model": "docs/architecture/concerns/agentic-threat-model.md",
            "governed_write_evidence": "target/appfw/governed-write-evidence.json",
            "governed_write_posture": "target/appfw/governed-write-posture.json"
        },
        "profile": profile_summary,
        "checks": checks,
        "violations": violations,
        "release_artifacts": [
            {
                "name": "u2-harness-check",
                "source": relative_artifact,
                "path": relative_release_artifact,
                "required": true,
                "staged": true,
                "local_ready": ok,
                "release_ready": false,
                "release_ready_reason": "Staged harness evidence remains G1-live-evidence dependent for Wave 2 release readiness."
            }
        ],
        "risk_acceptance": profile
            .as_ref()
            .and_then(|profile| yaml_sequence_at(profile, &["risk_acceptance"]))
            .map(|items| yaml_string_values(items))
            .unwrap_or_default(),
        "not_applicable": not_applicable
    });
    write_json_file(&artifact_path, &report)?;
    write_json_file(&release_artifact_path, &report)?;
    Ok(report)
}

fn add_harness_check(
    checks: &mut Vec<Value>,
    violations: &mut Vec<Value>,
    name: &'static str,
    ok: bool,
    detail: &'static str,
) {
    checks.push(json!({
        "name": name,
        "ok": ok,
        "detail": detail
    }));
    if !ok {
        violations.push(json!({
            "check": name,
            "detail": detail
        }));
    }
}

fn valid_governed_write_evidence(value: &Value) -> bool {
    value.get("command").and_then(Value::as_str) == Some("provider-test")
        && value.get("lane").and_then(Value::as_str) == Some("G1")
        && value.get("ok").and_then(Value::as_bool) == Some(true)
        && value.get("operation").and_then(Value::as_str) == Some("named_mutation")
        && value
            .get("delegated_actor_context")
            .is_some_and(Value::is_object)
        && value
            .get("token_store_isolation")
            .is_some_and(Value::is_object)
        && value.get("mutation_registry").is_some_and(|registry| {
            registry.is_object()
                && registry.get("mcp_enabled").and_then(Value::as_bool) == Some(false)
        })
        && value.get("idempotency").is_some_and(Value::is_object)
        && value.get("audit").is_some_and(Value::is_object)
}

fn valid_governed_write_posture(value: &Value) -> bool {
    value.get("command").and_then(Value::as_str) == Some("governed-write-check")
        && value.get("lane").and_then(Value::as_str) == Some("G1")
        && value.get("ok").and_then(Value::as_bool) == Some(true)
        && value.get("gate").is_some_and(|gate| {
            gate.is_object()
                && gate.get("enforced").and_then(Value::as_bool) == Some(true)
                && gate.get("ready_to_enforce").and_then(Value::as_bool) == Some(true)
        })
        && value
            .get("blocking_violations")
            .and_then(Value::as_array)
            .is_some_and(Vec::is_empty)
        && value.get("providers").is_some_and(|providers| {
            providers.as_array().is_some_and(|items| {
                !items.is_empty()
                    && items.iter().all(|provider| {
                        provider.get("family").and_then(Value::as_str) == Some("external_api")
                            && provider.get("write_enabled").and_then(Value::as_bool) == Some(false)
                            && provider.get("mcp_enabled").and_then(Value::as_bool) == Some(false)
                            && provider
                                .get("governed_write_certified")
                                .and_then(Value::as_bool)
                                .is_some()
                    })
            })
        })
}

fn harness_allowed_commands(profile: &serde_yaml::Value) -> Vec<Value> {
    yaml_sequence_at(profile, &["allowed_commands"])
        .map(|items| {
            items
                .iter()
                .filter_map(|item| match item {
                    serde_yaml::Value::String(command) => Some(json!({
                        "command": command.trim(),
                        "namespace": harness_command_namespace(command)
                    })),
                    _ => yaml_string_at(item, &["command"]).map(|command| {
                        json!({
                            "command": command,
                            "namespace": yaml_string_at(item, &["namespace"])
                                .unwrap_or_else(|| harness_command_namespace(&command))
                        })
                    }),
                })
                .collect()
        })
        .unwrap_or_default()
}

fn harness_command_namespace(command: &str) -> String {
    if command.starts_with("scripts/appfw product ") {
        "product".to_string()
    } else if command.starts_with("scripts/appfw framework ") {
        "framework".to_string()
    } else {
        "unknown".to_string()
    }
}

fn harness_command_writes_product_report(command: &str) -> bool {
    [
        "scripts/appfw product validate",
        "scripts/appfw product harness-check",
        "scripts/appfw product generate",
        "scripts/appfw product boundary-check",
        "scripts/appfw product test",
    ]
    .iter()
    .any(|prefix| command.starts_with(prefix))
}

fn harness_command_is_product_handoff(command: &str) -> bool {
    command.starts_with("scripts/appfw product handoff")
}

fn harness_writable_paths(profile: &serde_yaml::Value) -> Vec<Value> {
    yaml_sequence_at(profile, &["writable_paths"])
        .map(|items| {
            items
                .iter()
                .filter_map(|item| match item {
                    serde_yaml::Value::String(path) => Some(json!({
                        "path": path.trim(),
                        "ownership": "unspecified"
                    })),
                    _ => yaml_string_at(item, &["path"]).map(|path| {
                        json!({
                            "path": path,
                            "ownership": yaml_string_at(item, &["ownership"])
                                .unwrap_or_else(|| "unspecified".to_string())
                        })
                    }),
                })
                .collect()
        })
        .unwrap_or_default()
}

fn harness_sensitive_capabilities(profile: &serde_yaml::Value) -> Value {
    let mut values = serde_json::Map::new();
    for key in ["mcp", "kafka", "release", "saas_governed_write"] {
        values.insert(
            key.to_string(),
            json!(yaml_bool_at(profile, &["sensitive_capabilities", key]).unwrap_or(false)),
        );
    }
    Value::Object(values)
}

fn harness_safe_product_write_path(path: &str) -> bool {
    let trimmed = path.trim();
    if trimmed.is_empty() || trimmed.starts_with('/') {
        return false;
    }
    let path = Path::new(trimmed);
    if path
        .components()
        .any(|component| matches!(component, Component::ParentDir))
    {
        return false;
    }
    let first = path
        .components()
        .next()
        .and_then(|component| match component {
            Component::Normal(value) => value.to_str(),
            _ => None,
        })
        .unwrap_or_default();
    !matches!(
        first,
        ".git"
            | "agent_skills"
            | "app_gen"
            | "appfw_cli"
            | "appfw_runtime"
            | "appfw_test"
            | "docs"
            | "observability"
            | "scripts"
    )
}

fn harness_check_text(report: &Value) -> String {
    let ok = report.get("ok").and_then(Value::as_bool).unwrap_or(false);
    let artifact = report
        .get("artifact")
        .and_then(Value::as_str)
        .unwrap_or(".appfw/target/appfw/harness-check.json");
    let violation_count = report
        .get("violations")
        .and_then(Value::as_array)
        .map(Vec::len)
        .unwrap_or_default();
    format!(
        "Harness check: {}\nArtifact: {}\nViolations: {}",
        if ok { "ok" } else { "failed" },
        artifact,
        violation_count
    )
}

fn boundary_check_command(
    roots: &CommandRoots,
    json_output: bool,
    args: &[String],
) -> Result<(), String> {
    if !args.is_empty() {
        return Err("boundary-check does not accept additional arguments".to_string());
    }
    let report = build_boundary_check(roots)?;
    emit(json_output, &report, || boundary_check_text(&report))?;
    if report.ok {
        Ok(())
    } else {
        Err(format!(
            "boundary-check failed with {} violation(s); see {}",
            report.violations.len(),
            report.report_path
        ))
    }
}

fn analyze_command(roots: &CommandRoots, json_output: bool, args: &[String]) -> Result<(), String> {
    let summary_output = parse_summary_flag(args, "analyze")?;
    let report = analyze_product_intake(roots)?;
    let output = if summary_output {
        summarize_analysis_report(&report)
    } else {
        report.clone()
    };
    let report_path = roots.app_root.join("target/appfw/product-analysis.json");
    let analysis_artifact_path =
        if report.get("source_kind").and_then(Value::as_str) == Some("legacy") {
            roots.app_root.join(".appfw/legacy-analysis.yaml")
        } else {
            roots.app_root.join(".appfw/poc-analysis.yaml")
        };

    emit_value(json_output, &output, || {
        format!(
            "Product analysis written:\n  {}\n  {}",
            report_path.display(),
            analysis_artifact_path.display()
        )
    })
}

fn analyze_product_intake(roots: &CommandRoots) -> Result<Value, String> {
    let intake_path = roots.app_root.join(".appfw/poc-intake.yaml");
    if !intake_path.is_file() {
        return Err(format!(
            "product analyze requires {}; create a product intake shell first",
            relative_path(&roots.app_root, &intake_path)
        ));
    }

    let intake_text = fs::read_to_string(&intake_path)
        .map_err(|err| format!("failed to read {}: {err}", intake_path.display()))?;
    let intake: serde_yaml::Value = serde_yaml::from_str(&intake_text)
        .map_err(|err| format!("failed to parse {}: {err}", intake_path.display()))?;
    let source_kind = analyze_source_kind(&intake);
    let app_name = yaml_value_at(&intake, &["app", "name"]);
    let display_name = yaml_value_at(&intake, &["app", "display_name"]);
    let schema = yaml_value_at(&intake, &["schema", "name"]);
    let provider = yaml_value_at(&intake, &["topology", "backend_provider", "provider"]);
    let ui = yaml_value_at(&intake, &["topology", "ingress", "ui"]);

    let source_value = yaml_value_at(&intake, &["artifacts", "poc_source"]);
    let data_value = yaml_value_at(&intake, &["artifacts", "data_artifact"]);
    let ui_value = yaml_value_at(&intake, &["artifacts", "ui_artifact"]);
    let source_base = source_value
        .as_deref()
        .and_then(|configured| source_artifact_base(&roots.app_root, configured));
    let evidence = vec![
        analyze_evidence_artifact(&roots.app_root, None, "source", source_value.as_deref())?,
        analyze_evidence_artifact(
            &roots.app_root,
            source_base.as_deref(),
            "data",
            data_value.as_deref(),
        )?,
        analyze_evidence_artifact(
            &roots.app_root,
            source_base.as_deref(),
            "ui",
            ui_value.as_deref(),
        )?,
    ];

    let detected_signals = analysis_detected_signals(&evidence);
    let model_clues = analysis_model_clues(&evidence);
    let open_questions = product_analysis_open_questions(&source_kind);
    let recommendations = product_analysis_recommendations(&source_kind);
    let candidate_model_plan = product_analysis_candidate_model_plan(&source_kind);
    let analysis_artifact = if source_kind == "legacy" {
        ".appfw/legacy-analysis.yaml"
    } else {
        ".appfw/poc-analysis.yaml"
    };
    let analysis_artifact_path = roots.app_root.join(analysis_artifact);
    let report_path = roots.app_root.join("target/appfw/product-analysis.json");

    let report = json!({
        "command": "analyze",
        "ok": true,
        "source_kind": source_kind,
        "app": {
            "name": app_name,
            "display_name": display_name,
            "schema": schema,
            "provider": provider,
            "ui": ui
        },
        "input": {
            "intake_path": relative_path(&roots.app_root, &intake_path)
        },
        "artifacts": {
            "json": relative_path(&roots.app_root, &report_path),
            "review_yaml": analysis_artifact
        },
        "evidence": evidence,
        "detected_signals": detected_signals,
        "model_clues": model_clues,
        "candidate_model_plan": candidate_model_plan,
        "open_questions": open_questions,
        "recommendations": recommendations,
        "writes_final_model": false,
        "next_commands": product_analysis_next_commands(&source_kind),
    });

    if let Some(parent) = report_path.parent() {
        fs::create_dir_all(parent)
            .map_err(|err| format!("failed to create {}: {err}", parent.display()))?;
    }
    fs::write(
        &report_path,
        serde_json::to_string_pretty(&report).map_err(|err| err.to_string())?,
    )
    .map_err(|err| format!("failed to write {}: {err}", report_path.display()))?;

    if let Some(parent) = analysis_artifact_path.parent() {
        fs::create_dir_all(parent)
            .map_err(|err| format!("failed to create {}: {err}", parent.display()))?;
    }
    fs::write(
        &analysis_artifact_path,
        product_analysis_yaml(&source_kind, &report),
    )
    .map_err(|err| {
        format!(
            "failed to write {}: {err}",
            analysis_artifact_path.display()
        )
    })?;

    Ok(report)
}

fn propose_model_command(
    roots: &CommandRoots,
    json_output: bool,
    args: &[String],
) -> Result<(), String> {
    let summary_output = parse_summary_flag(args, "propose-model")?;
    let proposal = propose_model_from_analysis(roots)?;
    let output = if summary_output {
        summarize_model_proposal(&proposal)
    } else {
        proposal.clone()
    };
    let proposal_path = roots.app_root.join("target/appfw/model-proposal.json");
    let review_path = roots.app_root.join(".appfw/model-proposal.yaml");

    emit_value(json_output, &output, || {
        format!(
            "Product model proposal written:\n  {}\n  {}",
            proposal_path.display(),
            review_path.display()
        )
    })
}

fn model_status_command(
    roots: &CommandRoots,
    json_output: bool,
    args: &[String],
) -> Result<(), String> {
    if !args.is_empty() {
        return Err("model-status does not accept additional arguments".to_string());
    }
    let report = build_model_status_report(roots)?;
    let report_path = roots.app_root.join("target/appfw/model-status.json");
    if let Some(parent) = report_path.parent() {
        fs::create_dir_all(parent)
            .map_err(|err| format!("failed to create {}: {err}", parent.display()))?;
    }
    fs::write(
        &report_path,
        serde_json::to_string_pretty(&report).map_err(|err| err.to_string())?,
    )
    .map_err(|err| format!("failed to write {}: {err}", report_path.display()))?;

    emit_value(json_output, &report, || model_status_text(&report))
}

#[derive(Debug, Clone, Copy, Default)]
struct ModelScaffoldOptions {
    dry_run: bool,
    force: bool,
}

#[derive(Debug)]
struct ModelScaffoldEntity {
    name: String,
    snake_name: String,
    pascal_name: String,
    caption: String,
    config_kind: String,
    classification: String,
    source_path: PathBuf,
}

#[derive(Debug)]
enum ModelScaffoldWriteStatus {
    WouldWrite,
    Written,
    SkippedExisting,
}

fn scaffold_model_command(
    roots: &CommandRoots,
    json_output: bool,
    args: &[String],
) -> Result<(), String> {
    let options = parse_model_scaffold_options(args)?;
    let status = build_model_status_report(roots)?;
    let source_plan = status
        .get("source_authoring_plan")
        .ok_or_else(|| "model-status report did not include source_authoring_plan".to_string())?;
    let ready = source_plan
        .get("ready_to_write_source")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    if !ready {
        return Err(
            "product scaffold-model requires a signed proposal review; run scripts/appfw product model-status --json and resolve review_decisions before writing source"
                .to_string(),
        );
    }

    let schema = status
        .pointer("/app/schema")
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| "model-status report did not include app.schema".to_string())?;
    let schema_source_root = source_plan
        .get("schema_source_root")
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| "source_authoring_plan did not include schema_source_root".to_string())?;
    let schema_root = safe_app_relative_path(&roots.app_root, schema_source_root)?;
    let entities = model_scaffold_entities(source_plan, schema, &roots.app_root, &schema_root)?;
    if entities.is_empty() {
        return Err(
            "product scaffold-model found no accepted entity decisions to scaffold".to_string(),
        );
    }

    let data_source_name =
        model_scaffold_data_source_name(roots).unwrap_or_else(|| "primary".to_string());
    let schema_classification = model_scaffold_schema_classification(&entities);
    let schema_path = schema_root.join("_res.yaml");
    let mut would_write = Vec::new();
    let mut written = Vec::new();
    let mut skipped_existing = Vec::new();

    model_scaffold_write_file(
        roots,
        &schema_path,
        &model_scaffold_schema_yaml(roots, schema, &data_source_name, &schema_classification),
        options,
        &mut would_write,
        &mut written,
        &mut skipped_existing,
    )?;

    for entity in &entities {
        model_scaffold_write_file(
            roots,
            &entity.source_path,
            &model_scaffold_entity_yaml(schema, entity),
            options,
            &mut would_write,
            &mut written,
            &mut skipped_existing,
        )?;
        let policy_path = schema_root
            .join("rbac")
            .join(format!("{}.rego", entity.snake_name));
        model_scaffold_write_file(
            roots,
            &policy_path,
            &model_scaffold_policy_rego(schema, entity),
            options,
            &mut would_write,
            &mut written,
            &mut skipped_existing,
        )?;
    }

    let report_path = roots.app_root.join("target/appfw/model-scaffold.json");
    let entity_report = entities
        .iter()
        .map(|entity| {
            json!({
                "name": entity.name,
                "snake_name": entity.snake_name,
                "config_kind": entity.config_kind,
                "classification": entity.classification,
                "source_path": relative_path(&roots.app_root, &entity.source_path)
            })
        })
        .collect::<Vec<_>>();
    let report = json!({
        "command": "scaffold-model",
        "ok": true,
        "dry_run": options.dry_run,
        "force": options.force,
        "writes_final_model": false,
        "app": status.get("app").cloned().unwrap_or(Value::Null),
        "schema_source_root": schema_source_root,
        "data_source_name": data_source_name,
        "schema_classification": schema_classification,
        "entities": entity_report,
        "would_write_files": would_write,
        "written_files": written,
        "skipped_existing_files": skipped_existing,
        "artifacts": {
            "json": if options.dry_run { Value::Null } else { json!(relative_path(&roots.app_root, &report_path)) },
            "source_status": "target/appfw/model-status.json"
        },
        "guardrails": [
            "This command bootstraps source files only from signed review decisions.",
            "It does not infer relationships, business rules, data types beyond starter fields, frontend architecture, or legacy modernization decisions.",
            "Agents must complete semantic modeling and validation before generation."
        ],
        "next_commands": [
            "review and complete .appfw/model source",
            "scripts/appfw product validate --json",
            "scripts/appfw product model-status --json"
        ]
    });

    if !options.dry_run {
        if let Some(parent) = report_path.parent() {
            fs::create_dir_all(parent)
                .map_err(|err| format!("failed to create {}: {err}", parent.display()))?;
        }
        fs::write(
            &report_path,
            serde_json::to_string_pretty(&report).map_err(|err| err.to_string())?,
        )
        .map_err(|err| format!("failed to write {}: {err}", report_path.display()))?;
    }

    emit_value(json_output, &report, || model_scaffold_text(&report))
}

fn parse_model_scaffold_options(args: &[String]) -> Result<ModelScaffoldOptions, String> {
    let mut options = ModelScaffoldOptions::default();
    for arg in args {
        match arg.as_str() {
            "--dry-run" => options.dry_run = true,
            "--force" => options.force = true,
            "--help" | "-h" => {
                return Err(
                    "Usage: scripts/appfw product scaffold-model [--dry-run] [--force] [--json]"
                        .to_string(),
                );
            }
            other => return Err(format!("scaffold-model does not accept argument `{other}`")),
        }
    }
    Ok(options)
}

fn model_scaffold_entities(
    source_plan: &Value,
    schema: &str,
    app_root: &Path,
    schema_root: &Path,
) -> Result<Vec<ModelScaffoldEntity>, String> {
    let mut entities = Vec::new();
    let targets = source_plan
        .get("entity_source_targets")
        .and_then(Value::as_array)
        .ok_or_else(|| "source_authoring_plan did not include entity_source_targets".to_string())?;
    for target in targets {
        let name = target
            .get("final_name")
            .and_then(Value::as_str)
            .or_else(|| target.get("candidate").and_then(Value::as_str))
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .ok_or_else(|| {
                "accepted entity decision is missing final_name or candidate".to_string()
            })?
            .to_string();
        let snake_name = normalized_identifier(&name)
            .ok_or_else(|| format!("could not normalize entity name `{name}`"))?;
        let pascal_name = pascal_case_identifier(&name);
        if pascal_name.is_empty() {
            return Err(format!(
                "could not create PascalCase entity name from `{name}`"
            ));
        }
        let config_kind = target
            .get("config_kind")
            .and_then(Value::as_str)
            .map(|value| value.trim().to_ascii_lowercase())
            .ok_or_else(|| {
                format!(
                    "accepted entity `{name}` must set config_kind to entity, lookup, or dto before scaffolding"
                )
            })?;
        if !matches!(config_kind.as_str(), "entity" | "lookup" | "dto") {
            return Err(format!(
                "accepted entity `{name}` has unresolved config_kind `{config_kind}`; set it to entity, lookup, or dto before scaffolding"
            ));
        }
        let classification = target
            .get("classification")
            .and_then(Value::as_str)
            .map(|value| value.trim().to_ascii_lowercase())
            .filter(|value| !value.is_empty())
            .ok_or_else(|| {
                format!("accepted entity `{name}` must set classification before scaffolding")
            })?;
        if matches!(
            classification.as_str(),
            "needs_review" | "needs_discovery" | "null"
        ) {
            return Err(format!(
                "accepted entity `{name}` has unresolved classification `{classification}`"
            ));
        }
        let source_path =
            model_scaffold_entity_source_path(target, schema, app_root, schema_root, &snake_name)?;
        let caption = caption_from_identifier(&name);
        entities.push(ModelScaffoldEntity {
            name,
            snake_name,
            pascal_name,
            caption,
            config_kind,
            classification,
            source_path,
        });
    }
    Ok(entities)
}

fn model_scaffold_entity_source_path(
    target: &Value,
    schema: &str,
    app_root: &Path,
    schema_root: &Path,
    snake_name: &str,
) -> Result<PathBuf, String> {
    let suggested = target
        .get("suggested_source_path")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .filter(|value| !value.contains('<'));
    let Some(suggested) = suggested else {
        return Ok(schema_root
            .join("entity_types")
            .join(format!("{snake_name}.yaml")));
    };
    let suggested = Path::new(suggested);
    if suggested.is_absolute()
        || suggested.components().any(|component| {
            matches!(
                component,
                Component::ParentDir | Component::RootDir | Component::Prefix(_)
            )
        })
    {
        return Err(format!(
            "suggested entity source path `{}` must stay under .appfw/model/schemas/{schema}/entity_types",
            suggested.display()
        ));
    }
    let expected_prefix = Path::new(".appfw")
        .join("model")
        .join("schemas")
        .join(schema)
        .join("entity_types");
    if !suggested.starts_with(&expected_prefix) {
        return Err(format!(
            "suggested entity source path `{}` must stay under {}",
            suggested.display(),
            expected_prefix.display()
        ));
    }
    safe_app_relative_path(app_root, &suggested.display().to_string())
}

fn model_scaffold_write_file(
    roots: &CommandRoots,
    path: &Path,
    contents: &str,
    options: ModelScaffoldOptions,
    would_write: &mut Vec<String>,
    written: &mut Vec<String>,
    skipped_existing: &mut Vec<String>,
) -> Result<ModelScaffoldWriteStatus, String> {
    let rel = relative_path(&roots.app_root, path);
    if path.exists() && !options.force {
        skipped_existing.push(rel);
        return Ok(ModelScaffoldWriteStatus::SkippedExisting);
    }
    if options.dry_run {
        would_write.push(rel);
        return Ok(ModelScaffoldWriteStatus::WouldWrite);
    }
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|err| format!("failed to create {}: {err}", parent.display()))?;
    }
    fs::write(path, contents)
        .map_err(|err| format!("failed to write {}: {err}", path.display()))?;
    written.push(rel);
    Ok(ModelScaffoldWriteStatus::Written)
}

fn model_scaffold_data_source_name(roots: &CommandRoots) -> Option<String> {
    let intake_path = roots.app_root.join(".appfw/poc-intake.yaml");
    let intake_text = fs::read_to_string(intake_path).ok()?;
    let intake = serde_yaml::from_str::<serde_yaml::Value>(&intake_text).ok()?;
    yaml_string_at(
        &intake,
        &["topology", "backend_provider", "data_source_name"],
    )
}

fn model_scaffold_schema_classification(entities: &[ModelScaffoldEntity]) -> String {
    if entities
        .iter()
        .any(|entity| matches!(entity.classification.as_str(), "phi" | "ephi"))
    {
        "phi".to_string()
    } else if entities
        .iter()
        .any(|entity| entity.classification == "confidential")
    {
        "confidential".to_string()
    } else {
        entities
            .first()
            .map(|entity| entity.classification.clone())
            .unwrap_or_else(|| "confidential".to_string())
    }
}

fn model_scaffold_schema_yaml(
    roots: &CommandRoots,
    schema: &str,
    data_source_name: &str,
    classification: &str,
) -> String {
    let app_name = roots
        .app_root
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("product");
    format!(
        r#"id: {id}
name: {schema}
description: Product schema scaffolded from signed model proposal.
data_source_name: {data_source_name}
meta:
  classification: {classification}
"#,
        id = deterministic_product_schema_id(app_name, schema),
        schema = schema,
        data_source_name = data_source_name,
        classification = classification,
    )
}

fn model_scaffold_entity_yaml(schema: &str, entity: &ModelScaffoldEntity) -> String {
    let is_table = entity.config_kind != "dto";
    let facets_block = if is_table {
        "  facets:\n  - audited\n  - concurrency\n"
    } else {
        "  facets: []\n"
    };
    format!(
        r#"# Scaffolded by scripts/appfw product scaffold-model from signed .appfw/model-proposal.yaml.
# Complete properties, relationships, custom methods, tests, and policy from reviewed source evidence before generation.
- id: {id}
  name: {pascal}
  is_table: {is_table}
{facets_block}  meta:
    classification: {classification}
    scaffold_source: model-proposal-review
  props:
  - name: id
    fragment: property-primary-key-uuid
  - name: name
    caption: {caption}
    fragment: property-string-caption
"#,
        id = deterministic_scaffold_uuid(&format!("{schema}:entity:{}", entity.snake_name)),
        pascal = entity.pascal_name,
        is_table = is_table,
        facets_block = facets_block,
        classification = yaml_scalar(&entity.classification),
        caption = yaml_scalar(&entity.caption),
    )
}

fn model_scaffold_policy_rego(schema: &str, entity: &ModelScaffoldEntity) -> String {
    format!(
        r#"# Scaffolded by scripts/appfw product scaffold-model from signed .appfw/model-proposal.yaml.
# Default policy remains deny-by-default through the generated wrapper until a reviewer adds explicit access rules.
# Schema: {schema}
# Entity: {entity}
"#,
        schema = schema,
        entity = entity.snake_name,
    )
}

fn safe_app_relative_path(app_root: &Path, relative: &str) -> Result<PathBuf, String> {
    let path = Path::new(relative);
    if path.is_absolute()
        || path.components().any(|component| {
            matches!(
                component,
                Component::ParentDir | Component::RootDir | Component::Prefix(_)
            )
        })
    {
        return Err(format!(
            "path `{relative}` must be relative to the app root"
        ));
    }
    Ok(app_root.join(path))
}

fn deterministic_scaffold_uuid(key: &str) -> String {
    let digest = Sha256::digest(format!("appfw-model-scaffold:{key}").as_bytes());
    let hex = format!("{digest:x}");
    format!(
        "{}-{}-4{}-8{}-{}",
        &hex[0..8],
        &hex[8..12],
        &hex[13..16],
        &hex[17..20],
        &hex[20..32],
    )
}

fn pascal_case_identifier(value: &str) -> String {
    normalized_identifier(value)
        .map(|normalized| {
            normalized
                .split('_')
                .filter(|part| !part.is_empty())
                .map(|part| {
                    let mut chars = part.chars();
                    let Some(first) = chars.next() else {
                        return String::new();
                    };
                    format!("{}{}", first.to_ascii_uppercase(), chars.as_str())
                })
                .collect::<String>()
        })
        .unwrap_or_default()
}

fn caption_from_identifier(value: &str) -> String {
    normalized_identifier(value)
        .map(|normalized| {
            normalized
                .split('_')
                .filter(|part| !part.is_empty())
                .map(|part| {
                    let mut chars = part.chars();
                    let Some(first) = chars.next() else {
                        return String::new();
                    };
                    format!("{}{}", first.to_ascii_uppercase(), chars.as_str())
                })
                .collect::<Vec<_>>()
                .join(" ")
        })
        .unwrap_or_else(|| value.trim().to_string())
}

fn model_scaffold_text(report: &Value) -> String {
    let written = report
        .get("written_files")
        .and_then(Value::as_array)
        .map(Vec::len)
        .unwrap_or_default();
    let would_write = report
        .get("would_write_files")
        .and_then(Value::as_array)
        .map(Vec::len)
        .unwrap_or_default();
    let skipped = report
        .get("skipped_existing_files")
        .and_then(Value::as_array)
        .map(Vec::len)
        .unwrap_or_default();
    let dry_run = report
        .get("dry_run")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    if dry_run {
        format!(
            "Product model scaffold dry run complete: {would_write} file(s) would be written, {skipped} existing file(s) skipped."
        )
    } else {
        format!(
            "Product model scaffold complete: {written} file(s) written, {skipped} existing file(s) skipped."
        )
    }
}

fn build_model_status_report(roots: &CommandRoots) -> Result<Value, String> {
    let intake_path = roots.app_root.join(".appfw/poc-intake.yaml");
    let analysis_path = roots.app_root.join("target/appfw/product-analysis.json");
    let proposal_path = roots.app_root.join("target/appfw/model-proposal.json");
    let proposal_yaml_path = roots.app_root.join(".appfw/model-proposal.yaml");
    let validation_path = roots.report_root.join("validation.json");
    let artifact_manifest_path = roots.report_root.join("artifacts.json");
    let handoff_path = roots.app_root.join("target/appfw/agent-handoff.json");
    let status_path = roots.app_root.join("target/appfw/model-status.json");

    let intake_yaml = read_yaml_value_if_present(&intake_path)?;
    let proposal_yaml = read_yaml_value_if_present(&proposal_yaml_path)?;
    let analysis_json = read_json_value_if_present(&analysis_path)?;
    let proposal_json = read_json_value_if_present(&proposal_path)?;
    let validation_json = read_json_value_if_present(&validation_path)?;
    let artifact_manifest_json = read_json_value_if_present(&artifact_manifest_path)?;

    let source_kind = intake_yaml
        .as_ref()
        .map(analyze_source_kind)
        .or_else(|| {
            analysis_json
                .as_ref()
                .and_then(|value| value.get("source_kind").and_then(Value::as_str))
                .map(str::to_string)
        })
        .unwrap_or_else(|| "unknown".to_string());
    let schema = intake_yaml
        .as_ref()
        .and_then(|value| yaml_value_at(value, &["schema", "name"]))
        .or_else(|| {
            proposal_json
                .as_ref()
                .and_then(|value| value.pointer("/app/schema").and_then(Value::as_str))
                .map(str::to_string)
        });
    let app_name = intake_yaml
        .as_ref()
        .and_then(|value| yaml_value_at(value, &["app", "name"]))
        .or_else(|| {
            proposal_json
                .as_ref()
                .and_then(|value| value.pointer("/app/name").and_then(Value::as_str))
                .map(str::to_string)
        });
    let display_name = intake_yaml
        .as_ref()
        .and_then(|value| yaml_value_at(value, &["app", "display_name"]))
        .or_else(|| {
            proposal_json
                .as_ref()
                .and_then(|value| value.pointer("/app/display_name").and_then(Value::as_str))
                .map(str::to_string)
        });
    let provider = intake_yaml
        .as_ref()
        .and_then(|value| yaml_value_at(value, &["topology", "backend_provider", "provider"]))
        .or_else(|| {
            proposal_json
                .as_ref()
                .and_then(|value| value.pointer("/app/provider").and_then(Value::as_str))
                .map(str::to_string)
        });

    let schema_model = schema
        .as_deref()
        .map(|schema| product_schema_model_status(roots, schema))
        .transpose()?;
    let entity_source_count = schema_model
        .as_ref()
        .and_then(|value| value.get("entity_source_count").and_then(Value::as_u64))
        .unwrap_or_default();
    let proposal_entity_count = proposal_json
        .as_ref()
        .and_then(|value| value.pointer("/proposal/candidate_entities"))
        .and_then(Value::as_array)
        .map(Vec::len)
        .unwrap_or_default();
    let proposal_review = proposal_review_status(proposal_yaml.as_ref());
    let proposal_review_accepted = proposal_review
        .get("ready_for_config")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let source_authoring_plan = model_status_source_authoring_plan(
        proposal_yaml.as_ref(),
        schema.as_deref(),
        proposal_review_accepted,
    );
    let validation_ok = validation_json
        .as_ref()
        .and_then(|value| {
            value
                .get("valid")
                .or_else(|| value.get("ok"))
                .and_then(Value::as_bool)
        })
        .unwrap_or(false);
    let generated_artifact_count = artifact_manifest_json
        .as_ref()
        .and_then(|value| {
            value
                .get("artifacts")
                .or_else(|| value.get("files"))
                .and_then(Value::as_array)
        })
        .map(Vec::len)
        .unwrap_or_default();
    let signals = ModelStatusSignals {
        has_intake: intake_path.is_file(),
        has_analysis: analysis_path.is_file(),
        has_proposal: proposal_path.is_file(),
        has_proposal_yaml: proposal_yaml_path.is_file(),
        proposal_review_accepted,
        entity_source_count,
        validation_ok,
        generated_artifact_count,
        has_handoff: handoff_path.is_file(),
    };
    let phase = model_status_phase(&signals);
    let next_commands = model_status_next_commands(&phase, &source_kind, proposal_review_accepted);
    let blockers = model_status_blockers(&signals);

    Ok(json!({
        "command": "model-status",
        "ok": true,
        "phase": phase,
        "source_kind": source_kind,
        "app": {
            "name": app_name,
            "display_name": display_name,
            "schema": schema,
            "provider": provider
        },
        "artifacts": {
            "intake": model_status_artifact(&roots.app_root, &intake_path),
            "analysis": model_status_artifact(&roots.app_root, &analysis_path),
            "proposal": model_status_artifact(&roots.app_root, &proposal_path),
            "proposal_review": model_status_artifact(&roots.app_root, &proposal_yaml_path),
            "validation": model_status_artifact(&roots.app_root, &validation_path),
            "artifact_manifest": model_status_artifact(&roots.app_root, &artifact_manifest_path),
            "handoff": model_status_artifact(&roots.app_root, &handoff_path),
            "status": relative_path(&roots.app_root, &status_path)
        },
        "schema_model": schema_model,
        "proposal": {
            "candidate_entity_count": proposal_entity_count,
            "implementation_plan_present": proposal_json
                .as_ref()
                .and_then(|value| value.get("implementation_plan"))
                .is_some(),
            "writes_final_model": proposal_json
                .as_ref()
                .and_then(|value| value.pointer("/proposal/writes_final_model"))
                .and_then(Value::as_bool)
                .unwrap_or(false),
            "requires_human_review": proposal_json
                .as_ref()
                .and_then(|value| value.pointer("/proposal/requires_human_review"))
                .and_then(Value::as_bool)
                .unwrap_or(true)
        },
        "proposal_review": proposal_review,
        "source_authoring_plan": source_authoring_plan,
        "validation": {
            "ok": validation_ok,
            "generated_artifact_count": generated_artifact_count
        },
        "blockers": blockers,
        "next_commands": next_commands,
        "journey": [
            "intake",
            "analysis",
            "proposal",
            "model_source",
            "validation",
            "generation",
            "handoff"
        ]
    }))
}

fn model_status_source_authoring_plan(
    value: Option<&serde_yaml::Value>,
    schema: Option<&str>,
    ready_to_write_source: bool,
) -> Value {
    let schema_source_root = value
        .and_then(|value| yaml_string_at(value, &["implementation_plan", "schema_source_root"]))
        .or_else(|| schema.map(|schema| format!(".appfw/model/schemas/{schema}")));
    let required_source_files = value
        .and_then(|value| {
            yaml_sequence_at(value, &["implementation_plan", "required_source_files"])
        })
        .map(|items| {
            items
                .iter()
                .map(|item| {
                    json!({
                        "path": yaml_string_at(item, &["path"]),
                        "purpose": yaml_string_at(item, &["purpose"])
                    })
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let validation_sequence = value
        .and_then(|value| yaml_sequence_at(value, &["implementation_plan", "validation_sequence"]))
        .map(|items| yaml_string_values(items))
        .unwrap_or_default();
    let entity_source_targets = value
        .and_then(|value| yaml_sequence_at(value, &["review_decisions", "entity_decisions"]))
        .map(|items| {
            items
                .iter()
                .filter_map(|item| {
                    model_status_entity_source_target(item, schema_source_root.as_deref())
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let relationship_decision_count = value
        .and_then(|value| yaml_sequence_at(value, &["review_decisions", "relationship_decisions"]))
        .map(Vec::len)
        .unwrap_or_default();
    let routine_decision_count = value
        .and_then(|value| yaml_sequence_at(value, &["review_decisions", "routine_decisions"]))
        .map(Vec::len)
        .unwrap_or_default();
    let frontend_decision_count = value
        .and_then(|value| yaml_sequence_at(value, &["review_decisions", "frontend_decisions"]))
        .map(Vec::len)
        .unwrap_or_default();
    let integration_decision_count = value
        .and_then(|value| yaml_sequence_at(value, &["review_decisions", "integration_decisions"]))
        .map(Vec::len)
        .unwrap_or_default();

    json!({
        "ready_to_write_source": ready_to_write_source,
        "schema_source_root": schema_source_root,
        "required_source_files": required_source_files,
        "entity_source_targets": entity_source_targets,
        "relationship_decision_count": relationship_decision_count,
        "routine_decision_count": routine_decision_count,
        "frontend_decision_count": frontend_decision_count,
        "integration_decision_count": integration_decision_count,
        "validation_sequence": validation_sequence,
        "guardrail": if ready_to_write_source {
            "write source config from accepted review decisions, not from heuristic proposal clues"
        } else {
            "resolve and sign review_decisions before writing .appfw/model source"
        }
    })
}

fn model_status_entity_source_target(
    item: &serde_yaml::Value,
    schema_source_root: Option<&str>,
) -> Option<Value> {
    let decision = yaml_string_at(item, &["decision"]).unwrap_or_else(|| "missing".to_string());
    let decision = decision.trim().to_ascii_lowercase();
    if !matches!(decision.as_str(), "accept" | "split" | "merge") {
        return None;
    }
    let final_name = yaml_string_at(item, &["final_name"])
        .filter(|value| !value.eq_ignore_ascii_case("null"))
        .filter(|value| !value.trim().is_empty());
    let candidate = yaml_string_at(item, &["candidate"])
        .filter(|value| !value.eq_ignore_ascii_case("null"))
        .filter(|value| !value.trim().is_empty());
    let source_name = final_name
        .as_ref()
        .or(candidate.as_ref())
        .map(|value| value.trim().to_string())?;
    let suggested_source_path = yaml_string_at(item, &["suggested_source_path"])
        .filter(|value| !value.eq_ignore_ascii_case("null"))
        .filter(|value| !value.trim().is_empty())
        .or_else(|| {
            schema_source_root.map(|root| format!("{root}/entity_types/{source_name}.yaml"))
        });

    Some(json!({
        "candidate": candidate,
        "decision": decision,
        "final_name": final_name,
        "config_kind": yaml_string_at(item, &["config_kind"]),
        "classification": yaml_string_at(item, &["classification"]),
        "suggested_source_path": suggested_source_path
    }))
}

fn yaml_string_values(items: &[serde_yaml::Value]) -> Vec<String> {
    items
        .iter()
        .filter_map(|item| match item {
            serde_yaml::Value::String(value) => Some(value.trim().to_string()),
            serde_yaml::Value::Bool(value) => Some(value.to_string()),
            serde_yaml::Value::Number(value) => Some(value.to_string()),
            _ => None,
        })
        .filter(|value| !value.is_empty())
        .collect()
}

fn read_yaml_value_if_present(path: &Path) -> Result<Option<serde_yaml::Value>, String> {
    if !path.is_file() {
        return Ok(None);
    }
    let text = fs::read_to_string(path)
        .map_err(|err| format!("failed to read {}: {err}", path.display()))?;
    serde_yaml::from_str(&text)
        .map(Some)
        .map_err(|err| format!("failed to parse {}: {err}", path.display()))
}

fn read_json_value_if_present(path: &Path) -> Result<Option<Value>, String> {
    if !path.is_file() {
        return Ok(None);
    }
    let text = fs::read_to_string(path)
        .map_err(|err| format!("failed to read {}: {err}", path.display()))?;
    serde_json::from_str(&text)
        .map(Some)
        .map_err(|err| format!("failed to parse {}: {err}", path.display()))
}

fn proposal_review_status(value: Option<&serde_yaml::Value>) -> Value {
    let Some(value) = value else {
        return json!({
            "review_yaml_present": false,
            "review_decisions_present": false,
            "accepted_for_config": false,
            "signed_for_config": false,
            "ready_for_config": false,
            "model_owner_present": false,
            "reviewed_at_utc_present": false,
            "missing_sign_off_fields": ["model_owner", "reviewed_at_utc", "accepted_for_config"],
            "total_decision_count": 0,
            "unresolved_decision_count": 0,
            "decision_counts": {}
        });
    };
    let review = yaml_ref_at(value, &["review_decisions"]);
    let accepted_for_config = yaml_bool_at(
        value,
        &["review_decisions", "sign_off", "accepted_for_config"],
    )
    .unwrap_or(false);
    let model_owner_present =
        yaml_string_at(value, &["review_decisions", "sign_off", "model_owner"])
            .as_deref()
            .is_some_and(|value| !value.trim().is_empty());
    let reviewed_at_utc_present =
        yaml_string_at(value, &["review_decisions", "sign_off", "reviewed_at_utc"])
            .as_deref()
            .is_some_and(|value| !value.trim().is_empty());
    let signed_for_config = model_owner_present && reviewed_at_utc_present;
    let mut missing_sign_off_fields = Vec::new();
    if !model_owner_present {
        missing_sign_off_fields.push("model_owner");
    }
    if !reviewed_at_utc_present {
        missing_sign_off_fields.push("reviewed_at_utc");
    }
    if !accepted_for_config {
        missing_sign_off_fields.push("accepted_for_config");
    }
    let mut decision_counts = BTreeMap::<String, usize>::new();
    let mut total_decision_count = 0usize;
    let mut unresolved_decision_count = 0usize;
    if let Some(review) = review {
        for section in [
            "entity_decisions",
            "property_decisions",
            "relationship_decisions",
            "routine_decisions",
            "frontend_decisions",
            "integration_decisions",
        ] {
            if let Some(items) = yaml_sequence_at(review, &[section]) {
                for item in items {
                    total_decision_count += 1;
                    let decision = yaml_string_at(item, &["decision"])
                        .unwrap_or_else(|| "missing".to_string());
                    let decision = decision.trim();
                    let decision = if decision.is_empty() {
                        "missing"
                    } else {
                        decision
                    };
                    *decision_counts.entry(decision.to_string()).or_default() += 1;
                    if matches!(decision, "needs_review" | "needs_discovery" | "missing") {
                        unresolved_decision_count += 1;
                    }
                }
            }
        }
    }

    json!({
        "review_yaml_present": true,
        "review_decisions_present": review.is_some(),
        "accepted_for_config": accepted_for_config,
        "signed_for_config": signed_for_config,
        "ready_for_config": accepted_for_config && signed_for_config && unresolved_decision_count == 0,
        "model_owner_present": model_owner_present,
        "reviewed_at_utc_present": reviewed_at_utc_present,
        "missing_sign_off_fields": missing_sign_off_fields,
        "total_decision_count": total_decision_count,
        "unresolved_decision_count": unresolved_decision_count,
        "decision_counts": decision_counts
    })
}

fn yaml_ref_at<'a>(value: &'a serde_yaml::Value, path: &[&str]) -> Option<&'a serde_yaml::Value> {
    let mut current = value;
    for key in path {
        let mapping = current.as_mapping()?;
        let key = serde_yaml::Value::String((*key).to_string());
        current = mapping.get(&key)?;
    }
    Some(current)
}

fn yaml_sequence_at<'a>(
    value: &'a serde_yaml::Value,
    path: &[&str],
) -> Option<&'a Vec<serde_yaml::Value>> {
    yaml_ref_at(value, path)?.as_sequence()
}

fn yaml_bool_at(value: &serde_yaml::Value, path: &[&str]) -> Option<bool> {
    match yaml_ref_at(value, path)? {
        serde_yaml::Value::Bool(value) => Some(*value),
        serde_yaml::Value::String(value) => match value.trim().to_ascii_lowercase().as_str() {
            "true" | "yes" | "1" => Some(true),
            "false" | "no" | "0" => Some(false),
            _ => None,
        },
        _ => None,
    }
}

fn yaml_string_at(value: &serde_yaml::Value, path: &[&str]) -> Option<String> {
    match yaml_ref_at(value, path)? {
        serde_yaml::Value::String(value) => Some(value.trim().to_string()),
        serde_yaml::Value::Bool(value) => Some(value.to_string()),
        serde_yaml::Value::Number(value) => Some(value.to_string()),
        _ => None,
    }
}

fn product_schema_model_status(roots: &CommandRoots, schema: &str) -> Result<Value, String> {
    let schema_root = roots.config_root.join("schemas").join(schema);
    let entity_files = count_model_source_files(&schema_root.join("entity_types"), "yaml")?;
    let relationship_files = count_model_source_files(&schema_root.join("relationships"), "yaml")?;
    let enum_files = count_model_source_files(&schema_root.join("gql_enum_types"), "yaml")?;
    let seed_files = count_model_source_files(&schema_root.join("seeds"), "yaml")?;
    let test_files = count_model_source_files(&schema_root.join("tests"), "yaml")?;
    let policy_files = count_model_source_files(&schema_root.join("rbac"), "rego")?;
    Ok(json!({
        "schema_source_root": relative_path(&roots.app_root, &schema_root),
        "schema_source_present": schema_root.is_dir(),
        "graphql_http_route": schema_route_segment(schema),
        "entity_source_count": entity_files.len(),
        "relationship_source_count": relationship_files.len(),
        "enum_source_count": enum_files.len(),
        "seed_source_count": seed_files.len(),
        "api_scenario_source_count": test_files.len(),
        "policy_source_count": policy_files.len(),
        "entity_source_files": entity_files,
        "relationship_source_files": relationship_files,
        "policy_source_files": policy_files
    }))
}

fn count_model_source_files(root: &Path, extension: &str) -> Result<Vec<String>, String> {
    if !root.is_dir() {
        return Ok(Vec::new());
    }
    let mut files = fs::read_dir(root)
        .map_err(|err| format!("failed to read {}: {err}", root.display()))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|err| err.to_string())?
        .into_iter()
        .map(|entry| entry.path())
        .filter(|path| path.is_file())
        .filter(|path| path.file_name().and_then(|value| value.to_str()) != Some("_res.yaml"))
        .filter(|path| {
            path.extension()
                .and_then(|value| value.to_str())
                .is_some_and(|value| value.eq_ignore_ascii_case(extension))
        })
        .map(|path| path.display().to_string())
        .collect::<Vec<_>>();
    files.sort();
    Ok(files)
}

struct ModelStatusSignals {
    has_intake: bool,
    has_analysis: bool,
    has_proposal: bool,
    has_proposal_yaml: bool,
    proposal_review_accepted: bool,
    entity_source_count: u64,
    validation_ok: bool,
    generated_artifact_count: usize,
    has_handoff: bool,
}

fn model_status_phase(signals: &ModelStatusSignals) -> String {
    if !signals.has_intake {
        return "not_intake_product".to_string();
    }
    if signals.has_handoff {
        return "handoff_ready".to_string();
    }
    if signals.generated_artifact_count > 0 {
        return "generated".to_string();
    }
    if signals.validation_ok && signals.entity_source_count > 0 {
        return "model_validated".to_string();
    }
    if signals.entity_source_count > 0 {
        return "model_source_started".to_string();
    }
    if signals.proposal_review_accepted {
        return "proposal_reviewed".to_string();
    }
    if signals.has_proposal {
        return "proposal_ready_for_modeling".to_string();
    }
    if signals.has_analysis {
        return "analyzed".to_string();
    }
    "intake_recorded".to_string()
}

fn model_status_next_commands(
    phase: &str,
    source_kind: &str,
    proposal_review_accepted: bool,
) -> Vec<&'static str> {
    match phase {
        "not_intake_product" => vec![
            "scripts/appfw product new <target> --profile product-intake --json",
            "scripts/appfw product model-status --json",
        ],
        "intake_recorded" => vec![
            "scripts/appfw product analyze --summary --json",
            "scripts/appfw product model-status --json",
        ],
        "analyzed" => vec![
            "scripts/appfw product propose-model --summary --json",
            "scripts/appfw product model-status --json",
        ],
        "proposal_ready_for_modeling" if source_kind == "legacy" => vec![
            "review .appfw/model-proposal.yaml",
            "complete .appfw/legacy-modernization.yaml",
            "complete review_decisions sign-off and set accepted_for_config: true",
            "scripts/appfw product model-status --json",
        ],
        "proposal_ready_for_modeling" => vec![
            "review .appfw/model-proposal.yaml",
            "complete review_decisions sign-off and set accepted_for_config: true",
            "scripts/appfw product model-status --json",
        ],
        "proposal_reviewed" if source_kind == "legacy" => vec![
            "write .appfw/model from reviewed modernization evidence",
            "scripts/appfw product model-status --json",
        ],
        "proposal_reviewed" => vec![
            "write .appfw/model from reviewed PoC evidence",
            "scripts/appfw product model-status --json",
        ],
        "model_source_started" => vec![
            "scripts/appfw product validate --json",
            "scripts/appfw product model-status --json",
        ],
        "model_validated" => vec![
            "scripts/appfw product generate",
            "scripts/appfw product generate --check --json",
            "scripts/appfw product test --fast --json",
        ],
        "generated" => vec![
            "scripts/appfw product generate --check --json",
            "scripts/appfw product test --fast --json",
            "scripts/appfw product handoff --json",
        ],
        "handoff_ready" => vec![
            "review target/appfw/agent-handoff.json",
            "commit product-owned source and generated artifacts as appropriate",
        ],
        _ if !proposal_review_accepted => vec!["scripts/appfw product model-status --json"],
        _ => vec!["scripts/appfw product model-status --json"],
    }
}

fn model_status_blockers(signals: &ModelStatusSignals) -> Vec<&'static str> {
    let mut blockers = Vec::new();
    if !signals.has_intake {
        blockers.push("Missing .appfw/poc-intake.yaml.");
    }
    if signals.has_intake && !signals.has_analysis {
        blockers.push("Product analysis has not been run.");
    }
    if signals.has_analysis && !signals.has_proposal {
        blockers.push("Model proposal has not been drafted.");
    }
    if signals.has_proposal && !signals.has_proposal_yaml {
        blockers.push("Model proposal review YAML is missing.");
    }
    if signals.has_proposal
        && signals.has_proposal_yaml
        && signals.entity_source_count == 0
        && !signals.proposal_review_accepted
    {
        blockers.push("Model proposal review decisions are not ready for config yet.");
    }
    if signals.has_proposal && signals.entity_source_count == 0 {
        blockers.push("No reviewed product entity source files exist yet.");
    }
    if signals.entity_source_count > 0 && !signals.validation_ok {
        blockers.push("Product model source has not passed validation yet.");
    }
    if signals.validation_ok && signals.generated_artifact_count == 0 {
        blockers.push("Generated artifacts have not been produced or retained yet.");
    }
    blockers
}

fn model_status_artifact(app_root: &Path, path: &Path) -> Value {
    json!({
        "path": relative_path(app_root, path),
        "exists": path.is_file()
    })
}

fn model_status_text(report: &Value) -> String {
    let phase = report
        .get("phase")
        .and_then(Value::as_str)
        .unwrap_or("unknown");
    let schema = report
        .pointer("/app/schema")
        .and_then(Value::as_str)
        .unwrap_or("unknown");
    let next = report
        .get("next_commands")
        .and_then(Value::as_array)
        .and_then(|values| values.first())
        .and_then(Value::as_str)
        .unwrap_or("scripts/appfw product model-status --json");
    format!("Product model status: {phase}\nSchema: {schema}\nNext: {next}")
}

fn parse_summary_flag(args: &[String], command: &str) -> Result<bool, String> {
    let mut summary = false;
    for arg in args {
        match arg.as_str() {
            "--summary" => summary = true,
            "--full" => summary = false,
            _ => {
                return Err(format!(
                    "{command} accepts only --summary or --full; got {arg}"
                ));
            }
        }
    }
    Ok(summary)
}

fn summarize_analysis_report(report: &Value) -> Value {
    let evidence_summary = report
        .get("evidence")
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .map(|item| {
                    json!({
                        "role": item.get("role").cloned().unwrap_or(Value::Null),
                        "status": item.get("status").cloned().unwrap_or(Value::Null),
                        "kind": item.get("kind").cloned().unwrap_or(Value::Null),
                        "files_analyzed": item.get("files_analyzed").cloned().unwrap_or(Value::Null),
                        "extension_counts": item.get("extension_counts").cloned().unwrap_or(Value::Null),
                        "signals": item.get("signals").cloned().unwrap_or_else(|| json!([])),
                        "content_profile_count": content_profile_count(item),
                    })
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let model_clues = report.get("model_clues").cloned().unwrap_or(Value::Null);
    let candidate_properties = value_array_len(&model_clues, "candidate_properties");

    json!({
        "command": "analyze",
        "ok": report.get("ok").cloned().unwrap_or(Value::Bool(true)),
        "summary": true,
        "source_kind": report.get("source_kind").cloned().unwrap_or(Value::Null),
        "app": report.get("app").cloned().unwrap_or(Value::Null),
        "artifacts": report.get("artifacts").cloned().unwrap_or(Value::Null),
        "detected_signals": report.get("detected_signals").cloned().unwrap_or_else(|| json!([])),
        "evidence_summary": evidence_summary,
        "model_clues": {
            "candidate_entities": model_clues.get("candidate_entities").cloned().unwrap_or_else(|| json!([])),
            "candidate_property_count": candidate_properties,
            "candidate_routines": model_clues.get("candidate_routines").cloned().unwrap_or_else(|| json!([])),
            "frontend_views": model_clues.get("frontend_views").cloned().unwrap_or_else(|| json!([])),
            "integration_points": model_clues.get("integration_points").cloned().unwrap_or_else(|| json!([])),
        },
        "open_questions": report.get("open_questions").cloned().unwrap_or_else(|| json!([])),
        "writes_final_model": report.get("writes_final_model").cloned().unwrap_or(Value::Bool(false)),
        "full_report": report.get("artifacts").and_then(|value| value.get("json")).cloned().unwrap_or(Value::Null),
        "review_yaml": report.get("artifacts").and_then(|value| value.get("review_yaml")).cloned().unwrap_or(Value::Null),
        "next_commands": report.get("next_commands").cloned().unwrap_or_else(|| json!([])),
    })
}

fn summarize_model_proposal(proposal: &Value) -> Value {
    let proposal_body = proposal.get("proposal").cloned().unwrap_or(Value::Null);
    let implementation_plan = proposal
        .get("implementation_plan")
        .cloned()
        .unwrap_or(Value::Null);
    let review_decisions = proposal
        .get("review_decisions")
        .cloned()
        .unwrap_or(Value::Null);
    let candidate_entities = proposal_body
        .get("candidate_entities")
        .and_then(Value::as_array)
        .map(|entities| {
            entities
                .iter()
                .map(|entity| {
                    json!({
                        "name": entity.get("name").cloned().unwrap_or(Value::Null),
                        "semantic_labels": entity.get("semantic_labels").cloned().unwrap_or_else(|| json!([])),
                        "suggested_domain_names": entity.get("suggested_domain_names").cloned().unwrap_or_else(|| json!([])),
                        "property_count": value_array_len(entity, "candidate_properties"),
                        "classification": entity.get("classification").cloned().unwrap_or(Value::Null),
                        "review_status": entity.get("review_status").cloned().unwrap_or(Value::Null),
                    })
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    json!({
        "command": "propose-model",
        "ok": proposal.get("ok").cloned().unwrap_or(Value::Bool(true)),
        "summary": true,
        "source_kind": proposal.get("source_kind").cloned().unwrap_or(Value::Null),
        "app": proposal.get("app").cloned().unwrap_or(Value::Null),
        "artifacts": proposal.get("artifacts").cloned().unwrap_or(Value::Null),
        "proposal": {
            "candidate_entities": candidate_entities,
            "unassigned_property_count": value_array_len(&proposal_body, "unassigned_properties"),
            "candidate_routine_count": value_array_len(&proposal_body, "candidate_routines"),
            "frontend_views": proposal_body.get("frontend_views").cloned().unwrap_or_else(|| json!([])),
            "integration_points": proposal_body.get("integration_points").cloned().unwrap_or_else(|| json!([])),
            "requires_human_review": proposal_body.get("requires_human_review").cloned().unwrap_or(Value::Bool(true)),
            "writes_final_model": proposal_body.get("writes_final_model").cloned().unwrap_or(Value::Bool(false)),
        },
        "implementation_plan": {
            "schema_source_root": implementation_plan.get("schema_source_root").cloned().unwrap_or(Value::Null),
            "required_source_file_count": value_array_len(&implementation_plan, "required_source_files"),
            "entity_task_count": value_array_len(&implementation_plan, "entity_tasks"),
            "review_checkpoint": implementation_plan.get("review_checkpoint").cloned().unwrap_or(Value::Null),
            "validation_sequence": implementation_plan.get("validation_sequence").cloned().unwrap_or_else(|| json!([])),
        },
        "review_decisions": {
            "entity_decision_count": value_array_len(&review_decisions, "entity_decisions"),
            "property_decision_count": value_array_len(&review_decisions, "property_decisions"),
            "routine_decision_count": value_array_len(&review_decisions, "routine_decisions"),
            "frontend_decision_count": value_array_len(&review_decisions, "frontend_decisions"),
            "accepted_for_config": review_decisions
                .pointer("/sign_off/accepted_for_config")
                .cloned()
                .unwrap_or(Value::Bool(false)),
        },
        "open_questions": proposal_body.get("open_questions").cloned().unwrap_or_else(|| json!([])),
        "guardrails": proposal.get("guardrails").cloned().unwrap_or_else(|| json!([])),
        "full_report": proposal.get("artifacts").and_then(|value| value.get("json")).cloned().unwrap_or(Value::Null),
        "review_yaml": proposal.get("artifacts").and_then(|value| value.get("review_yaml")).cloned().unwrap_or(Value::Null),
        "next_commands": proposal.get("next_commands").cloned().unwrap_or_else(|| json!([])),
    })
}

fn value_array_len(value: &Value, key: &str) -> usize {
    value
        .get(key)
        .and_then(Value::as_array)
        .map(Vec::len)
        .unwrap_or_default()
}

fn content_profile_count(item: &Value) -> usize {
    if let Some(items) = item.get("content_profiles").and_then(Value::as_array) {
        return items.len();
    }
    if item.get("content_profile").is_some() {
        return 1;
    }
    0
}

fn propose_model_from_analysis(roots: &CommandRoots) -> Result<Value, String> {
    let analysis_path = roots.app_root.join("target/appfw/product-analysis.json");
    if !analysis_path.is_file() {
        return Err(format!(
            "product propose-model requires {}; run scripts/appfw product analyze --summary --json first",
            relative_path(&roots.app_root, &analysis_path)
        ));
    }

    let analysis_text = fs::read_to_string(&analysis_path)
        .map_err(|err| format!("failed to read {}: {err}", analysis_path.display()))?;
    let analysis: Value = serde_json::from_str(&analysis_text)
        .map_err(|err| format!("failed to parse {}: {err}", analysis_path.display()))?;
    let source_kind = analysis
        .get("source_kind")
        .and_then(Value::as_str)
        .unwrap_or("poc");
    let app = analysis.get("app").cloned().unwrap_or(Value::Null);
    let model_clues = analysis.get("model_clues").cloned().unwrap_or(Value::Null);
    let entity_drafts = model_proposal_entity_drafts(&analysis);
    let assigned_properties = model_proposal_assigned_properties(&entity_drafts);
    let candidate_entities = model_proposal_entities(
        model_clues
            .get("candidate_entities")
            .and_then(Value::as_array),
        &entity_drafts,
    );
    let unassigned_properties = model_proposal_unassigned_properties(
        model_clues
            .get("candidate_properties")
            .and_then(Value::as_array),
        &assigned_properties,
    );
    let candidate_routines = model_proposal_strings(
        model_clues
            .get("candidate_routines")
            .and_then(Value::as_array),
    );
    let frontend_views =
        model_proposal_strings(model_clues.get("frontend_views").and_then(Value::as_array));
    let integration_points = model_proposal_strings(
        model_clues
            .get("integration_points")
            .and_then(Value::as_array),
    );
    let open_questions = analysis
        .get("open_questions")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let review_decisions = model_proposal_review_decisions(
        source_kind,
        &app,
        &candidate_entities,
        &unassigned_properties,
        &candidate_routines,
        &frontend_views,
        &integration_points,
    );
    let proposal_path = roots.app_root.join("target/appfw/model-proposal.json");
    let review_path = roots.app_root.join(".appfw/model-proposal.yaml");

    let proposal = json!({
        "command": "propose-model",
        "ok": true,
        "source_kind": source_kind,
        "app": app,
        "input": {
            "analysis": relative_path(&roots.app_root, &analysis_path)
        },
        "artifacts": {
            "json": relative_path(&roots.app_root, &proposal_path),
            "review_yaml": relative_path(&roots.app_root, &review_path)
        },
        "proposal": {
            "candidate_entities": candidate_entities.clone(),
            "unassigned_properties": unassigned_properties,
            "candidate_routines": candidate_routines,
            "frontend_views": frontend_views,
            "integration_points": integration_points,
            "open_questions": open_questions,
            "requires_human_review": true,
            "writes_final_model": false
        },
        "implementation_plan": model_proposal_implementation_plan(source_kind, &app, &candidate_entities),
        "review_decisions": review_decisions,
        "guardrails": [
            "Do not write .appfw/model until the proposal is reviewed.",
            "Treat every proposed entity, property, routine, and view as a candidate.",
            "Add classification metadata before validating regulated product config.",
            "Use CRM as an implementation reference only; do not copy CRM residue."
        ],
        "next_commands": [
            "review .appfw/model-proposal.yaml",
            "resolve review_decisions entries from needs_review/needs_discovery to accept, reject, defer, split, or merge",
            "write .appfw/model from reviewed proposal",
            "scripts/appfw product validate --json",
            "scripts/appfw product generate --check --json",
            "scripts/appfw product handoff --json"
        ]
    });

    if let Some(parent) = proposal_path.parent() {
        fs::create_dir_all(parent)
            .map_err(|err| format!("failed to create {}: {err}", parent.display()))?;
    }
    fs::write(
        &proposal_path,
        serde_json::to_string_pretty(&proposal).map_err(|err| err.to_string())?,
    )
    .map_err(|err| format!("failed to write {}: {err}", proposal_path.display()))?;

    if let Some(parent) = review_path.parent() {
        fs::create_dir_all(parent)
            .map_err(|err| format!("failed to create {}: {err}", parent.display()))?;
    }
    fs::write(&review_path, model_proposal_yaml(&proposal))
        .map_err(|err| format!("failed to write {}: {err}", review_path.display()))?;

    Ok(proposal)
}

fn analyze_source_kind(intake: &serde_yaml::Value) -> String {
    let source_kind = yaml_value_at(intake, &["source_kind"])
        .or_else(|| yaml_value_at(intake, &["source"]))
        .unwrap_or_else(|| "poc".to_string());
    if source_kind.to_ascii_lowercase().contains("legacy") {
        "legacy".to_string()
    } else {
        "poc".to_string()
    }
}

fn yaml_value_at(value: &serde_yaml::Value, path: &[&str]) -> Option<String> {
    let mut current = value;
    for key in path {
        let mapping = current.as_mapping()?;
        let key = serde_yaml::Value::String((*key).to_string());
        current = mapping.get(&key)?;
    }
    match current {
        serde_yaml::Value::String(value) => {
            Some(value.trim().to_string()).filter(|v| !v.is_empty())
        }
        serde_yaml::Value::Bool(value) => Some(value.to_string()),
        serde_yaml::Value::Number(value) => Some(value.to_string()),
        _ => None,
    }
}

fn analyze_evidence_artifact(
    app_root: &Path,
    source_base: Option<&Path>,
    role: &str,
    configured: Option<&str>,
) -> Result<Value, String> {
    let Some(configured) = configured.map(str::trim).filter(|value| !value.is_empty()) else {
        return Ok(json!({
            "role": role,
            "configured": null,
            "status": "not-provided",
            "signals": []
        }));
    };
    let path = resolve_intake_evidence_path(app_root, source_base, configured);
    if !path.exists() {
        return Ok(json!({
            "role": role,
            "configured": configured,
            "path": path.display().to_string(),
            "status": "missing",
            "signals": []
        }));
    }

    let metadata =
        fs::metadata(&path).map_err(|err| format!("failed to stat {}: {err}", path.display()))?;
    if metadata.is_file() {
        let extension = file_extension(&path).unwrap_or_else(|| "none".to_string());
        let signals = analysis_signals_for_extensions([extension.clone()]);
        let content_profile =
            analyze_evidence_content_profile(&path, &extension)?.unwrap_or_else(|| Value::Null);
        return Ok(json!({
            "role": role,
            "configured": configured,
            "path": path.display().to_string(),
            "status": "present",
            "kind": "file",
            "bytes": metadata.len(),
            "extension": extension,
            "signals": signals,
            "content_profile": content_profile
        }));
    }

    if metadata.is_dir() {
        let files = collect_bounded_evidence_files(&path, 500)?;
        let mut extension_counts = BTreeMap::<String, usize>::new();
        for file in &files {
            let extension = file_extension(file).unwrap_or_else(|| "none".to_string());
            *extension_counts.entry(extension).or_default() += 1;
        }
        let signals = analysis_signals_for_extensions(extension_counts.keys().cloned());
        let samples = files
            .iter()
            .take(20)
            .map(|file| relative_path(&path, file))
            .collect::<Vec<_>>();
        let mut content_profiles = Vec::new();
        for file in files.iter().take(25) {
            let extension = file_extension(file).unwrap_or_else(|| "none".to_string());
            if let Some(profile) = analyze_evidence_content_profile(file, &extension)? {
                content_profiles.push(json!({
                    "file": relative_path(&path, file),
                    "extension": extension,
                    "profile": profile
                }));
            }
        }
        return Ok(json!({
            "role": role,
            "configured": configured,
            "path": path.display().to_string(),
            "status": "present",
            "kind": "directory",
            "files_analyzed": files.len(),
            "truncated": files.len() >= 500,
            "extension_counts": extension_counts,
            "sample_files": samples,
            "signals": signals,
            "content_profiles": content_profiles
        }));
    }

    Ok(json!({
        "role": role,
        "configured": configured,
        "path": path.display().to_string(),
        "status": "present",
        "kind": "other",
        "signals": []
    }))
}

fn analyze_evidence_content_profile(path: &Path, extension: &str) -> Result<Option<Value>, String> {
    match extension {
        "csv" => analyze_delimited_profile(path, ',').map(Some),
        "tsv" => analyze_delimited_profile(path, '\t').map(Some),
        "xlsx" | "xlsm" => Ok(Some(analyze_spreadsheet_profile(path))),
        "xls" => Ok(Some(analyze_spreadsheet_stub(path))),
        "html" | "htm" | "cshtml" | "razor" | "aspx" | "ascx" => {
            analyze_html_profile(path).map(Some)
        }
        "md" | "markdown" => analyze_markdown_profile(path).map(Some),
        "sql" | "psql" => analyze_sql_profile(path).map(Some),
        "sln" | "csproj" | "vbproj" | "cs" | "vb" => analyze_dotnet_profile(path).map(Some),
        "json" | "yaml" | "yml" => Ok(Some(analyze_structured_export_profile(path))),
        _ => Ok(None),
    }
}

fn source_artifact_base(app_root: &Path, configured: &str) -> Option<PathBuf> {
    let source_path = absolute_path(app_root, configured);
    if source_path.is_dir() {
        Some(source_path)
    } else if source_path.is_file() {
        source_path.parent().map(Path::to_path_buf)
    } else {
        None
    }
}

fn resolve_intake_evidence_path(
    app_root: &Path,
    source_base: Option<&Path>,
    configured: &str,
) -> PathBuf {
    let path = absolute_path(app_root, configured);
    if path.exists() || Path::new(configured).is_absolute() {
        return path;
    }

    if let Some(source_base) = source_base {
        let source_relative_path = source_base.join(configured);
        if source_relative_path.exists() {
            return source_relative_path;
        }
    }

    path
}

fn analyze_delimited_profile(path: &Path, delimiter: char) -> Result<Value, String> {
    let text = read_text_prefix(path, 256 * 1024)?;
    let mut lines = text.lines().filter(|line| !line.trim().is_empty());
    let header = lines.next().unwrap_or_default();
    let columns = split_delimited_line(header, delimiter)
        .into_iter()
        .filter_map(|value| normalized_identifier(&value))
        .take(80)
        .collect::<Vec<_>>();
    let sample_rows = lines.take(20).count();
    let candidate_entities = candidate_entity_from_path(path)
        .into_iter()
        .collect::<Vec<_>>();
    Ok(json!({
        "kind": "tabular",
        "format": if delimiter == '\t' { "tsv" } else { "csv" },
        "columns": columns,
        "sample_rows": sample_rows,
        "candidate_entities": candidate_entities,
        "candidate_properties": columns
    }))
}

fn analyze_spreadsheet_stub(path: &Path) -> Value {
    json!({
        "kind": "spreadsheet",
        "workbook_name": path
            .file_stem()
            .and_then(|value| value.to_str())
            .unwrap_or_default(),
        "candidate_entities": [],
        "requires_workbook_extraction": true,
        "recommended_follow_up": "Inspect workbook sheets, headers, formulas, pivots, and embedded data before writing .appfw/model."
    })
}

fn analyze_spreadsheet_profile(path: &Path) -> Value {
    analyze_ooxml_workbook_profile(path).unwrap_or_else(|| analyze_spreadsheet_stub(path))
}

fn analyze_ooxml_workbook_profile(path: &Path) -> Option<Value> {
    let workbook_xml = unzip_text_entry(path, "xl/workbook.xml")?;
    let sheets = extract_attr_values(&workbook_xml, "sheet", "name", 200)
        .into_iter()
        .map(|value| decode_xml_entities(&value))
        .filter(|value| !value.trim().is_empty())
        .collect::<Vec<_>>();
    let hidden_sheet_count = extract_attr_values(&workbook_xml, "sheet", "state", 200)
        .into_iter()
        .filter(|value| {
            matches!(
                value.trim().to_ascii_lowercase().as_str(),
                "hidden" | "veryhidden"
            )
        })
        .count();
    let entry_names = unzip_entry_names(path).unwrap_or_default();
    let table_entries = entry_names
        .iter()
        .filter(|entry| {
            entry.starts_with("xl/tables/table")
                && entry.ends_with(".xml")
                && !entry.contains("/_rels/")
        })
        .take(100)
        .cloned()
        .collect::<Vec<_>>();
    let worksheet_entries = entry_names
        .iter()
        .filter(|entry| {
            entry.starts_with("xl/worksheets/sheet")
                && entry.ends_with(".xml")
                && !entry.contains("/_rels/")
        })
        .take(200)
        .cloned()
        .collect::<Vec<_>>();
    let mut formula_count = 0usize;
    let mut formula_sheet_count = 0usize;
    let mut data_validation_count = 0usize;
    for entry in &worksheet_entries {
        let Some(sheet_xml) = unzip_text_entry(path, entry) else {
            continue;
        };
        let sheet_formula_count = count_xml_start_tags(&sheet_xml, "f");
        if sheet_formula_count > 0 {
            formula_sheet_count += 1;
        }
        formula_count += sheet_formula_count;
        data_validation_count += count_xml_start_tags(&sheet_xml, "dataValidation");
    }
    let pivot_table_count = entry_names
        .iter()
        .filter(|entry| entry.starts_with("xl/pivotTables/pivotTable") && entry.ends_with(".xml"))
        .count();
    let chart_count = entry_names
        .iter()
        .filter(|entry| entry.starts_with("xl/charts/chart") && entry.ends_with(".xml"))
        .count();
    let mut tables = Vec::new();
    let mut candidate_entities = BTreeSet::new();
    let mut candidate_properties = BTreeSet::new();
    let mut candidate_routines = BTreeSet::new();
    let mut frontend_views = BTreeSet::new();
    if formula_count > 0 {
        candidate_routines.insert("workbook_calculations".to_string());
    }
    if pivot_table_count > 0 {
        candidate_routines.insert("pivot_or_aggregation_views".to_string());
    }
    if chart_count > 0 {
        frontend_views.insert("workbook_charts".to_string());
    }

    for entry in table_entries {
        let Some(table_xml) = unzip_text_entry(path, &entry) else {
            continue;
        };
        let name = extract_attr_values(&table_xml, "table", "name", 1)
            .into_iter()
            .next()
            .map(|value| decode_xml_entities(&value))
            .unwrap_or_default();
        let display_name = extract_attr_values(&table_xml, "table", "displayName", 1)
            .into_iter()
            .next()
            .map(|value| decode_xml_entities(&value))
            .filter(|value| !value.trim().is_empty())
            .unwrap_or_else(|| name.clone());
        let reference = extract_attr_values(&table_xml, "table", "ref", 1)
            .into_iter()
            .next()
            .unwrap_or_default();
        let columns = extract_attr_values(&table_xml, "tablecolumn", "name", 200)
            .into_iter()
            .map(|value| decode_xml_entities(&value))
            .filter(|value| !value.trim().is_empty())
            .collect::<Vec<_>>();
        let identifiers = columns
            .iter()
            .filter_map(|value| normalized_identifier(value))
            .collect::<Vec<_>>();
        candidate_properties.extend(identifiers.iter().cloned());
        if let Some(identifier) = normalized_identifier(&display_name) {
            candidate_entities.insert(identifier);
        }
        let (range_rows, data_rows, column_count) =
            ooxml_table_dimensions(&reference).unwrap_or((0, 0, columns.len()));

        tables.push(json!({
            "name": name,
            "display_name": display_name,
            "range": reference,
            "range_rows": range_rows,
            "data_rows": data_rows,
            "column_count": column_count,
            "columns": columns,
            "candidate_properties": identifiers
        }));
    }

    if candidate_entities.is_empty() {
        candidate_entities.extend(
            sheets
                .iter()
                .filter_map(|value| normalized_identifier(value)),
        );
    }
    let worksheet_count = sheets.len();
    let table_count = tables.len();

    Some(json!({
        "kind": "spreadsheet",
        "format": "ooxml",
        "extraction": "unzip_metadata",
        "workbook_name": path
            .file_stem()
            .and_then(|value| value.to_str())
            .unwrap_or_default(),
        "sheets": sheets,
        "tables": tables,
        "workbook_complexity": {
            "worksheet_count": worksheet_count,
            "table_count": table_count,
            "hidden_sheet_count": hidden_sheet_count,
            "formula_count": formula_count,
            "formula_sheet_count": formula_sheet_count,
            "pivot_table_count": pivot_table_count,
            "chart_count": chart_count,
            "data_validation_count": data_validation_count
        },
        "candidate_entities": candidate_entities.into_iter().collect::<Vec<_>>(),
        "candidate_properties": candidate_properties.into_iter().collect::<Vec<_>>(),
        "candidate_routines": candidate_routines.into_iter().collect::<Vec<_>>(),
        "frontend_views": frontend_views.into_iter().collect::<Vec<_>>(),
        "requires_workbook_extraction": false,
        "recommended_follow_up": "Review formulas, pivots, charts, hidden sheets, data validations, data types, relationships, and business calculations before writing .appfw/model."
    }))
}

fn count_xml_start_tags(text: &str, tag: &str) -> usize {
    let lower = text.to_ascii_lowercase();
    let needle = format!("<{}", tag.to_ascii_lowercase());
    let mut count = 0usize;
    let mut offset = 0usize;
    while let Some(pos) = lower[offset..].find(&needle) {
        let start = offset + pos;
        let after_idx = start + needle.len();
        let after = lower[after_idx..].chars().next().unwrap_or('>');
        if after == '>' || after == '/' || after.is_ascii_whitespace() {
            count += 1;
        }
        offset = after_idx;
    }
    count
}

fn unzip_text_entry(path: &Path, entry: &str) -> Option<String> {
    let output = Command::new("unzip")
        .arg("-p")
        .arg(path)
        .arg(entry)
        .output()
        .ok()?;
    if !output.status.success() || output.stdout.is_empty() {
        return None;
    }
    Some(String::from_utf8_lossy(&output.stdout).to_string())
}

fn unzip_entry_names(path: &Path) -> Option<Vec<String>> {
    let output = Command::new("unzip").arg("-l").arg(path).output().ok()?;
    if !output.status.success() {
        return None;
    }
    let text = String::from_utf8_lossy(&output.stdout);
    Some(
        text.lines()
            .filter_map(|line| line.split_whitespace().last())
            .filter(|entry| entry.contains('/'))
            .map(str::to_string)
            .collect(),
    )
}

fn ooxml_table_dimensions(reference: &str) -> Option<(usize, usize, usize)> {
    let (start, end) = reference.split_once(':')?;
    let (_, start_row) = split_ooxml_cell_reference(start)?;
    let (end_column, end_row) = split_ooxml_cell_reference(end)?;
    let (start_column, _) = split_ooxml_cell_reference(start)?;
    let range_rows = end_row.checked_sub(start_row)?.saturating_add(1);
    let data_rows = range_rows.saturating_sub(1);
    let column_count = end_column.checked_sub(start_column)?.saturating_add(1);
    Some((range_rows, data_rows, column_count))
}

fn split_ooxml_cell_reference(reference: &str) -> Option<(usize, usize)> {
    let mut column = String::new();
    let mut row = String::new();
    for ch in reference.chars() {
        if ch.is_ascii_alphabetic() {
            column.push(ch);
        } else if ch.is_ascii_digit() {
            row.push(ch);
        }
    }
    Some((ooxml_column_index(&column)?, row.parse::<usize>().ok()?))
}

fn ooxml_column_index(column: &str) -> Option<usize> {
    let mut value = 0usize;
    for ch in column.chars() {
        if !ch.is_ascii_alphabetic() {
            return None;
        }
        value = value
            .checked_mul(26)?
            .checked_add((ch.to_ascii_uppercase() as u8 - b'A' + 1) as usize)?;
    }
    if value == 0 {
        None
    } else {
        Some(value)
    }
}

fn decode_xml_entities(value: &str) -> String {
    value
        .replace("&amp;", "&")
        .replace("&quot;", "\"")
        .replace("&apos;", "'")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
}

fn analyze_markdown_profile(path: &Path) -> Result<Value, String> {
    let text = read_text_prefix(path, 768 * 1024)?;
    let raw_headings = extract_markdown_headings(&text, 60);
    let headings = raw_headings
        .iter()
        .map(|heading| strip_markdown_inline(heading))
        .collect::<Vec<_>>();
    let candidate_entities = extract_markdown_code_heading_entities(&raw_headings);
    let candidate_entity_labels = extract_markdown_entity_labels(&raw_headings);
    let candidate_properties = extract_markdown_field_table_values(&text, 160);
    let candidate_routines = headings
        .iter()
        .filter(|heading| {
            let lower = heading.to_ascii_lowercase();
            lower.contains("calculation")
                || lower.contains("workflow")
                || lower.contains("business logic")
                || lower.contains("custom resolver")
                || lower.contains("valuation")
        })
        .filter_map(|heading| normalized_identifier(heading))
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();

    Ok(json!({
        "kind": "markdown_reference",
        "headings": headings,
        "candidate_entities": candidate_entities,
        "candidate_entity_labels": candidate_entity_labels,
        "candidate_properties": candidate_properties,
        "candidate_routines": candidate_routines,
        "recommended_follow_up": "Review source reference tables and calculations before writing .appfw/model."
    }))
}

fn analyze_html_profile(path: &Path) -> Result<Value, String> {
    let text = read_text_prefix(path, 512 * 1024)?;
    let title = extract_tag_text(&text, "title")
        .or_else(|| extract_tag_text(&text, "h1"))
        .unwrap_or_else(|| {
            candidate_entity_from_path(path)
                .unwrap_or_else(|| "prototype".to_string())
                .replace('_', " ")
        });
    let inputs = extract_attr_values(&text, "input", "name", 50)
        .into_iter()
        .chain(extract_attr_values(&text, "select", "name", 50))
        .chain(extract_attr_values(&text, "textarea", "name", 50))
        .filter_map(|value| normalized_identifier(&value))
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    let table_headers = extract_tag_texts(&text, "th", 60)
        .into_iter()
        .filter_map(|value| normalized_identifier(&value))
        .collect::<Vec<_>>();
    let forms = count_case_insensitive(&text, "<form");
    let tables = count_case_insensitive(&text, "<table");
    let mut frontend_views = vec![title.trim().to_string()];
    frontend_views.extend(
        extract_tag_texts(&text, "h2", 12)
            .into_iter()
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty()),
    );
    frontend_views.sort();
    frontend_views.dedup();
    frontend_views.truncate(20);
    let candidate_properties = merge_string_vectors([inputs.clone(), table_headers.clone()]);

    Ok(json!({
        "kind": "ui",
        "title": title.trim(),
        "forms": forms,
        "tables": tables,
        "inputs": inputs,
        "table_headers": table_headers,
        "candidate_properties": candidate_properties,
        "frontend_views": frontend_views
    }))
}

fn analyze_sql_profile(path: &Path) -> Result<Value, String> {
    let text = read_text_prefix(path, 768 * 1024)?;
    let mut routines = BTreeSet::new();
    for phrase in [
        "create procedure",
        "create or alter procedure",
        "create or replace procedure",
        "create function",
        "create or replace function",
    ] {
        routines.extend(extract_sql_names_after_phrase(&text, phrase, 20));
    }
    let mut tables = BTreeSet::new();
    for phrase in ["create table", "insert into", "update", "from"] {
        tables.extend(extract_sql_names_after_phrase(&text, phrase, 30));
    }
    let candidate_entities = tables
        .iter()
        .filter_map(|value| normalized_identifier(value.rsplit('.').next().unwrap_or(value)))
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    let routine_list = routines.into_iter().collect::<Vec<_>>();
    Ok(json!({
        "kind": "sql",
        "routines": routine_list.clone(),
        "tables": tables.into_iter().collect::<Vec<_>>(),
        "candidate_entities": candidate_entities,
        "candidate_routines": routine_list
    }))
}

fn analyze_dotnet_profile(path: &Path) -> Result<Value, String> {
    let text = read_text_prefix(path, 512 * 1024)?;
    let extension = file_extension(path).unwrap_or_default();
    let mut projects = BTreeSet::new();
    let mut classes = BTreeSet::new();
    let mut controllers = BTreeSet::new();
    let mut routes = BTreeSet::new();

    if matches!(extension.as_str(), "sln" | "csproj" | "vbproj") {
        if let Some(stem) = path.file_stem().and_then(|value| value.to_str()) {
            projects.insert(stem.to_string());
        }
    }
    if extension == "sln" {
        projects.extend(extract_solution_project_names(&text));
    }
    if matches!(extension.as_str(), "cs" | "vb") {
        classes.extend(extract_dotnet_classes(&text));
        controllers.extend(
            classes
                .iter()
                .filter(|value| value.ends_with("Controller"))
                .cloned(),
        );
        routes.extend(extract_route_attributes(&text));
    }
    let candidate_entities = classes
        .iter()
        .map(|value| value.trim_end_matches("Controller"))
        .filter_map(normalized_identifier)
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    let route_list = routes.into_iter().collect::<Vec<_>>();

    Ok(json!({
        "kind": "dotnet",
        "projects": projects.into_iter().collect::<Vec<_>>(),
        "classes": classes.into_iter().collect::<Vec<_>>(),
        "controllers": controllers.into_iter().collect::<Vec<_>>(),
        "routes": route_list.clone(),
        "candidate_entities": candidate_entities,
        "integration_points": route_list
    }))
}

fn analyze_structured_export_profile(path: &Path) -> Value {
    let candidate_entities = candidate_entity_from_path(path)
        .into_iter()
        .collect::<Vec<_>>();
    json!({
        "kind": "structured_export",
        "candidate_entities": candidate_entities,
        "recommended_follow_up": "Inspect keys, arrays, IDs, and nested objects before modeling persisted entities."
    })
}

fn read_text_prefix(path: &Path, max_bytes: usize) -> Result<String, String> {
    let mut file =
        fs::File::open(path).map_err(|err| format!("failed to open {}: {err}", path.display()))?;
    let mut bytes = Vec::new();
    std::io::Read::by_ref(&mut file)
        .take(max_bytes as u64)
        .read_to_end(&mut bytes)
        .map_err(|err| format!("failed to read {}: {err}", path.display()))?;
    Ok(String::from_utf8_lossy(&bytes).to_string())
}

fn split_delimited_line(line: &str, delimiter: char) -> Vec<String> {
    let mut values = Vec::new();
    let mut current = String::new();
    let mut in_quotes = false;
    let mut chars = line.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch == '"' {
            if in_quotes && chars.peek() == Some(&'"') {
                current.push('"');
                chars.next();
            } else {
                in_quotes = !in_quotes;
            }
        } else if ch == delimiter && !in_quotes {
            values.push(trim_csv_value(&current));
            current.clear();
        } else {
            current.push(ch);
        }
    }
    values.push(trim_csv_value(&current));
    values
}

fn trim_csv_value(value: &str) -> String {
    value.trim().trim_matches('"').trim().to_string()
}

fn normalized_identifier(value: &str) -> Option<String> {
    let mut output = String::new();
    let mut previous_was_separator = false;
    for ch in value.chars() {
        if ch.is_ascii_alphanumeric() {
            output.push(ch.to_ascii_lowercase());
            previous_was_separator = false;
        } else if !previous_was_separator {
            output.push('_');
            previous_was_separator = true;
        }
    }
    let output = output.trim_matches('_').to_string();
    if output.is_empty() {
        None
    } else {
        Some(output)
    }
}

fn candidate_entity_from_path(path: &Path) -> Option<String> {
    path.file_stem()
        .and_then(|value| value.to_str())
        .and_then(normalized_identifier)
}

fn merge_string_vectors<I>(values: I) -> Vec<String>
where
    I: IntoIterator<Item = Vec<String>>,
{
    values
        .into_iter()
        .flatten()
        .filter(|value| !value.trim().is_empty())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

fn count_case_insensitive(text: &str, pattern: &str) -> usize {
    text.to_ascii_lowercase()
        .matches(&pattern.to_ascii_lowercase())
        .count()
}

fn extract_tag_text(text: &str, tag: &str) -> Option<String> {
    extract_tag_texts(text, tag, 1).into_iter().next()
}

fn extract_tag_texts(text: &str, tag: &str, limit: usize) -> Vec<String> {
    let lower = text.to_ascii_lowercase();
    let open = format!("<{tag}");
    let close = format!("</{tag}>");
    let mut values = Vec::new();
    let mut offset = 0;
    while values.len() < limit {
        let Some(open_pos) = lower[offset..].find(&open) else {
            break;
        };
        let open_start = offset + open_pos;
        let Some(open_end_rel) = lower[open_start..].find('>') else {
            break;
        };
        let body_start = open_start + open_end_rel + 1;
        let Some(close_pos_rel) = lower[body_start..].find(&close) else {
            break;
        };
        let body_end = body_start + close_pos_rel;
        let value = strip_html_tags(&text[body_start..body_end]);
        if !value.trim().is_empty() {
            values.push(value.trim().to_string());
        }
        offset = body_end + close.len();
    }
    values
}

fn strip_html_tags(value: &str) -> String {
    let mut output = String::new();
    let mut in_tag = false;
    for ch in value.chars() {
        match ch {
            '<' => in_tag = true,
            '>' => {
                in_tag = false;
                output.push(' ');
            }
            _ if !in_tag => output.push(ch),
            _ => {}
        }
    }
    output
        .replace("&nbsp;", " ")
        .replace("&amp;", "&")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn extract_attr_values(text: &str, tag: &str, attr: &str, limit: usize) -> Vec<String> {
    let lower = text.to_ascii_lowercase();
    let open = format!("<{tag}");
    let attr_lower = attr.to_ascii_lowercase();
    let mut values = Vec::new();
    let mut offset = 0;
    while values.len() < limit {
        let Some(open_pos) = lower[offset..].find(&open) else {
            break;
        };
        let open_start = offset + open_pos;
        let Some(open_end_rel) = lower[open_start..].find('>') else {
            break;
        };
        let open_end = open_start + open_end_rel;
        let tag_text = &text[open_start..=open_end];
        if let Some(value) = extract_attr_value(tag_text, &attr_lower) {
            values.push(value);
        } else if attr == "name" {
            if let Some(value) = extract_attr_value(tag_text, "id") {
                values.push(value);
            }
        }
        offset = open_end + 1;
    }
    values
}

fn extract_attr_value(tag_text: &str, attr: &str) -> Option<String> {
    let lower = tag_text.to_ascii_lowercase();
    let mut offset = 0;
    while let Some(pos) = lower[offset..].find(attr) {
        let start = offset + pos;
        let before = lower[..start].chars().last().unwrap_or(' ');
        let after = lower[start + attr.len()..].chars().next().unwrap_or(' ');
        if !before.is_ascii_alphanumeric() && after != '-' && after != '_' {
            let rest = &tag_text[start + attr.len()..];
            let rest_trimmed = rest.trim_start();
            if let Some(rest) = rest_trimmed.strip_prefix('=') {
                let rest = rest.trim_start();
                if let Some(quote) = rest.chars().next().filter(|ch| *ch == '"' || *ch == '\'') {
                    let body = &rest[quote.len_utf8()..];
                    if let Some(end) = body.find(quote) {
                        return Some(body[..end].to_string());
                    }
                } else {
                    let value = rest
                        .split(|ch: char| ch.is_whitespace() || ch == '>')
                        .next()
                        .unwrap_or_default();
                    if !value.is_empty() {
                        return Some(value.to_string());
                    }
                }
            }
        }
        offset = start + attr.len();
    }
    None
}

fn extract_markdown_headings(text: &str, limit: usize) -> Vec<String> {
    text.lines()
        .filter_map(|line| {
            let trimmed = line.trim();
            if !trimmed.starts_with('#') {
                return None;
            }
            let heading = trimmed.trim_start_matches('#').trim();
            if heading.is_empty() {
                None
            } else {
                Some(heading.to_string())
            }
        })
        .take(limit)
        .collect()
}

fn extract_markdown_code_heading_entities(headings: &[String]) -> Vec<String> {
    headings
        .iter()
        .flat_map(|heading| extract_markdown_code_spans(heading))
        .filter(|value| !looks_like_source_artifact_name(value))
        .filter_map(|value| normalized_identifier(&value))
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

fn extract_markdown_entity_labels(headings: &[String]) -> Vec<Value> {
    let mut labels = BTreeMap::<String, BTreeSet<String>>::new();
    for heading in headings {
        for span in extract_markdown_code_spans(heading) {
            if looks_like_source_artifact_name(&span) {
                continue;
            }
            let Some(entity) = normalized_identifier(&span) else {
                continue;
            };
            let Some(label) = markdown_heading_label_after_code_span(heading, &span) else {
                continue;
            };
            labels.entry(entity).or_default().insert(label);
        }
    }
    labels
        .into_iter()
        .flat_map(|(entity, labels)| {
            labels.into_iter().map(move |label| {
                json!({
                    "entity": entity,
                    "label": label,
                    "suggested_name": normalized_identifier(&label).unwrap_or(entity.clone())
                })
            })
        })
        .collect()
}

fn markdown_heading_label_after_code_span(heading: &str, span: &str) -> Option<String> {
    let marker = format!("`{span}`");
    let marker_start = heading.find(&marker)?;
    let after_marker = &heading[marker_start + marker.len()..];
    let label = after_marker
        .trim()
        .trim_start_matches(['-', '—', '–', ':', '|'])
        .trim();
    if label.is_empty() {
        return None;
    }
    let label = strip_markdown_inline(label);
    if label.is_empty() {
        None
    } else {
        Some(label)
    }
}

fn looks_like_source_artifact_name(value: &str) -> bool {
    let lower = value.to_ascii_lowercase();
    [
        ".csv", ".tsv", ".xls", ".xlsx", ".xlsm", ".xlsb", ".html", ".htm", ".json", ".yaml",
        ".yml", ".md", ".png", ".jpg", ".jpeg", ".pdf",
    ]
    .iter()
    .any(|extension| lower.ends_with(extension))
}

fn extract_markdown_code_spans(value: &str) -> Vec<String> {
    let mut spans = Vec::new();
    let mut rest = value;
    while let Some(start) = rest.find('`') {
        let after_start = &rest[start + 1..];
        let Some(end) = after_start.find('`') else {
            break;
        };
        let span = after_start[..end].trim();
        if !span.is_empty() {
            spans.push(span.to_string());
        }
        rest = &after_start[end + 1..];
    }
    spans
}

fn extract_markdown_field_table_values(text: &str, limit: usize) -> Vec<String> {
    let mut values = BTreeSet::new();
    let mut active_field_table = false;

    for line in text.lines() {
        let trimmed = line.trim();
        if !trimmed.starts_with('|') || !trimmed.ends_with('|') {
            active_field_table = false;
            continue;
        }
        let cells = split_markdown_table_row(trimmed);
        if cells.is_empty() {
            continue;
        }
        if cells.iter().all(|cell| {
            cell.chars()
                .all(|ch| ch == '-' || ch == ':' || ch.is_whitespace())
        }) {
            continue;
        }
        if !active_field_table {
            active_field_table = cells
                .first()
                .map(|cell| {
                    let lower = cell.to_ascii_lowercase();
                    matches!(
                        lower.as_str(),
                        "field" | "metric" | "line item" | "property" | "column"
                    )
                })
                .unwrap_or(false);
            continue;
        }
        if let Some(first_cell) = cells.first() {
            if let Some(identifier) = normalized_identifier(&strip_markdown_inline(first_cell)) {
                values.insert(identifier);
                if values.len() >= limit {
                    break;
                }
            }
        }
    }

    values.into_iter().collect()
}

fn split_markdown_table_row(line: &str) -> Vec<String> {
    line.trim_matches('|')
        .split('|')
        .map(|cell| strip_markdown_inline(cell.trim()))
        .filter(|cell| !cell.is_empty())
        .collect()
}

fn strip_markdown_inline(value: &str) -> String {
    value
        .replace(['*', '`'], "")
        .replace("&nbsp;", " ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn extract_sql_names_after_phrase(text: &str, phrase: &str, limit: usize) -> Vec<String> {
    let lower = text.to_ascii_lowercase();
    let phrase_lower = phrase.to_ascii_lowercase();
    let mut names = BTreeSet::new();
    let mut offset = 0;
    while names.len() < limit {
        let Some(pos) = lower[offset..].find(&phrase_lower) else {
            break;
        };
        let start = offset + pos + phrase_lower.len();
        if let Some(name) = first_sql_identifier(&text[start..]) {
            names.insert(name);
        }
        offset = start;
    }
    names.into_iter().collect()
}

fn first_sql_identifier(value: &str) -> Option<String> {
    for token in value.split_whitespace() {
        let clean = clean_sql_identifier(token);
        if clean.is_empty()
            || matches!(
                clean.to_ascii_lowercase().as_str(),
                "if" | "not" | "exists" | "only" | "or" | "replace" | "alter"
            )
        {
            continue;
        }
        return Some(clean);
    }
    None
}

fn clean_sql_identifier(value: &str) -> String {
    value
        .split('(')
        .next()
        .unwrap_or_default()
        .trim_matches(|ch: char| {
            ch.is_whitespace()
                || matches!(ch, '[' | ']' | '"' | '\'' | '`' | ';' | ',' | '\r' | '\n')
        })
        .replace("].[", ".")
        .replace(['[', ']', '"', '`'], "")
}

fn extract_solution_project_names(text: &str) -> Vec<String> {
    text.lines()
        .filter_map(|line| {
            let (_, rest) = line.split_once('=')?;
            let first = rest.split(',').next()?.trim().trim_matches('"');
            if first.is_empty() {
                None
            } else {
                Some(first.to_string())
            }
        })
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

fn extract_dotnet_classes(text: &str) -> Vec<String> {
    let mut classes = BTreeSet::new();
    for line in text.lines() {
        let tokens = line
            .split(|ch: char| !ch.is_ascii_alphanumeric() && ch != '_')
            .filter(|value| !value.is_empty())
            .collect::<Vec<_>>();
        for window in tokens.windows(2) {
            if window[0].eq_ignore_ascii_case("class") {
                classes.insert(window[1].to_string());
            }
        }
    }
    classes.into_iter().collect()
}

fn extract_route_attributes(text: &str) -> Vec<String> {
    let mut routes = BTreeSet::new();
    for attr in [
        "Route",
        "HttpGet",
        "HttpPost",
        "HttpPut",
        "HttpPatch",
        "HttpDelete",
    ] {
        let needle = format!("{attr}(\"");
        let mut offset = 0;
        while let Some(pos) = text[offset..].find(&needle) {
            let start = offset + pos + needle.len();
            if let Some(end) = text[start..].find('"') {
                routes.insert(text[start..start + end].to_string());
            }
            offset = start;
        }
    }
    routes.into_iter().collect()
}

fn collect_bounded_evidence_files(root: &Path, max_files: usize) -> Result<Vec<PathBuf>, String> {
    let mut files = Vec::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let mut entries = fs::read_dir(&dir)
            .map_err(|err| format!("failed to read {}: {err}", dir.display()))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|err| err.to_string())?;
        entries.sort_by_key(|entry| entry.path());
        for entry in entries {
            if files.len() >= max_files {
                return Ok(files);
            }
            let path = entry.path();
            let name = path
                .file_name()
                .and_then(|value| value.to_str())
                .unwrap_or_default();
            if name.starts_with('.')
                || name.starts_with("~$")
                || name == "target"
                || name == "node_modules"
                || name == ".DS_Store"
            {
                continue;
            }
            let file_type = entry.file_type().map_err(|err| err.to_string())?;
            if file_type.is_dir() {
                stack.push(path);
            } else if file_type.is_file() {
                files.push(path);
            }
        }
    }
    Ok(files)
}

fn file_extension(path: &Path) -> Option<String> {
    path.extension()
        .and_then(|value| value.to_str())
        .map(|value| value.to_ascii_lowercase())
        .filter(|value| !value.is_empty())
}

fn analysis_signals_for_extensions<I>(extensions: I) -> Vec<String>
where
    I: IntoIterator<Item = String>,
{
    let extensions = extensions.into_iter().collect::<BTreeSet<_>>();
    let mut signals = BTreeSet::new();
    if extensions
        .iter()
        .any(|ext| matches!(ext.as_str(), "xlsx" | "xlsm" | "xls" | "csv" | "tsv"))
    {
        signals.insert("tabular_data".to_string());
    }
    if extensions
        .iter()
        .any(|ext| matches!(ext.as_str(), "html" | "htm"))
    {
        signals.insert("prototype_or_legacy_ui".to_string());
    }
    if extensions
        .iter()
        .any(|ext| matches!(ext.as_str(), "json" | "yaml" | "yml"))
    {
        signals.insert("structured_export".to_string());
    }
    if extensions
        .iter()
        .any(|ext| matches!(ext.as_str(), "md" | "markdown"))
    {
        signals.insert("source_reference".to_string());
    }
    if extensions
        .iter()
        .any(|ext| matches!(ext.as_str(), "sln" | "csproj" | "cs" | "vbproj" | "vb"))
    {
        signals.insert("dotnet_codebase".to_string());
    }
    if extensions
        .iter()
        .any(|ext| matches!(ext.as_str(), "sql" | "psql"))
    {
        signals.insert("sql_or_routine_source".to_string());
    }
    if extensions
        .iter()
        .any(|ext| matches!(ext.as_str(), "cshtml" | "razor" | "aspx" | "ascx"))
    {
        signals.insert("legacy_web_ui".to_string());
    }
    signals.into_iter().collect()
}

fn analysis_detected_signals(evidence: &[Value]) -> Vec<String> {
    let mut signals = BTreeSet::new();
    for item in evidence {
        if let Some(values) = item.get("signals").and_then(Value::as_array) {
            for value in values {
                if let Some(value) = value.as_str() {
                    signals.insert(value.to_string());
                }
            }
        }
    }
    signals.into_iter().collect()
}

fn analysis_model_clues(evidence: &[Value]) -> Value {
    let mut candidate_entities = BTreeSet::new();
    let mut candidate_properties = BTreeSet::new();
    let mut candidate_routines = BTreeSet::new();
    let mut frontend_views = BTreeSet::new();
    let mut integration_points = BTreeSet::new();

    for item in evidence {
        if let Some(profile) = item.get("content_profile") {
            collect_model_clues_from_profile(
                profile,
                &mut candidate_entities,
                &mut candidate_properties,
                &mut candidate_routines,
                &mut frontend_views,
                &mut integration_points,
            );
        }
        if let Some(profiles) = item.get("content_profiles").and_then(Value::as_array) {
            for profile_item in profiles {
                if let Some(profile) = profile_item.get("profile") {
                    collect_model_clues_from_profile(
                        profile,
                        &mut candidate_entities,
                        &mut candidate_properties,
                        &mut candidate_routines,
                        &mut frontend_views,
                        &mut integration_points,
                    );
                }
            }
        }
    }

    json!({
        "candidate_entities": candidate_entities.into_iter().collect::<Vec<_>>(),
        "candidate_properties": candidate_properties.into_iter().collect::<Vec<_>>(),
        "candidate_routines": candidate_routines.into_iter().collect::<Vec<_>>(),
        "frontend_views": frontend_views.into_iter().collect::<Vec<_>>(),
        "integration_points": integration_points.into_iter().collect::<Vec<_>>(),
        "needs_human_review": true
    })
}

fn collect_model_clues_from_profile(
    profile: &Value,
    candidate_entities: &mut BTreeSet<String>,
    candidate_properties: &mut BTreeSet<String>,
    candidate_routines: &mut BTreeSet<String>,
    frontend_views: &mut BTreeSet<String>,
    integration_points: &mut BTreeSet<String>,
) {
    collect_string_array(profile, "candidate_entities", candidate_entities);
    collect_string_array(profile, "candidate_properties", candidate_properties);
    collect_string_array(profile, "candidate_routines", candidate_routines);
    collect_string_array(profile, "routines", candidate_routines);
    collect_string_array(profile, "frontend_views", frontend_views);
    collect_string_array(profile, "integration_points", integration_points);
    collect_string_array(profile, "routes", integration_points);
}

fn collect_string_array(value: &Value, key: &str, output: &mut BTreeSet<String>) {
    if let Some(values) = value.get(key).and_then(Value::as_array) {
        for value in values {
            if let Some(value) = value
                .as_str()
                .map(str::trim)
                .filter(|value| !value.is_empty())
            {
                output.insert(value.to_string());
            }
        }
    }
}

fn product_analysis_open_questions(source_kind: &str) -> Vec<&'static str> {
    if source_kind == "legacy" {
        vec![
            "Which modernization slice is in scope for the first release?",
            "Which routes/controllers/jobs map to each target product capability?",
            "Which stored procedures are eliminated, replaced, moved to services, wrapped temporarily, or retained?",
            "What are the tenant, auth, role, and data-classification rules?",
            "What migration, reconciliation, rollback, API, frontend, performance, and release evidence is required?",
        ]
    } else {
        vec![
            "Which source columns are persisted entities versus report-only fields?",
            "Which values are lookup/status enums and which are free text?",
            "Which calculations belong in generated queries, custom methods, or frontend-only presentation?",
            "What data classification applies to each candidate entity and property?",
            "Which prototype screens become product routes, dashboards, forms, or grids?",
        ]
    }
}

fn product_analysis_recommendations(source_kind: &str) -> Vec<&'static str> {
    if source_kind == "legacy" {
        vec![
            "Complete .appfw/legacy-modernization.yaml before writing final .appfw/model.",
            "Analyze code and database behavior before naming target entities.",
            "Decompose large stored procedures; retain provider routines only with tests and exit criteria.",
            "Plan migration and parallel-run evidence before generated release checks.",
        ]
    } else {
        vec![
            "Review .appfw/poc-analysis.yaml before writing final .appfw/model.",
            "Use source artifacts to propose entities, relationships, DTOs, lookups, dashboards, and forms.",
            "Keep ambiguous fields as open questions instead of guessing regulated semantics.",
            "Use CRM as an implementation reference only; do not copy CRM schema or UI residue.",
        ]
    }
}

fn product_analysis_candidate_model_plan(source_kind: &str) -> Value {
    if source_kind == "legacy" {
        json!({
            "write_final_config": false,
            "candidate_outputs": [
                "capability map",
                "target entity model",
                "relationship model",
                "DTO/read-model contracts",
                "custom method contracts",
                "product service boundaries",
                "stored procedure disposition matrix",
                "migration and reconciliation plan",
                "frontend route/workflow map"
            ]
        })
    } else {
        json!({
            "write_final_config": false,
            "candidate_outputs": [
                "candidate entities",
                "candidate properties and datatypes",
                "candidate relationships",
                "lookup/status enum candidates",
                "derived metrics and report DTOs",
                "custom method candidates",
                "frontend dashboard/form/grid candidates",
                "classification assumptions and open questions"
            ]
        })
    }
}

fn product_analysis_next_commands(source_kind: &str) -> Vec<&'static str> {
    if source_kind == "legacy" {
        vec![
            "review .appfw/legacy-analysis.yaml",
            "complete .appfw/legacy-modernization.yaml",
            "scripts/appfw product propose-model --summary --json",
            "scripts/appfw product model-status --json",
            "review .appfw/model-proposal.yaml",
            "write .appfw/model from reviewed modernization evidence",
            "scripts/appfw product validate --json",
            "scripts/appfw product generate --check --json",
            "scripts/appfw product handoff --json",
        ]
    } else {
        vec![
            "review .appfw/poc-analysis.yaml",
            "scripts/appfw product propose-model --summary --json",
            "scripts/appfw product model-status --json",
            "review .appfw/model-proposal.yaml",
            "write .appfw/model from reviewed PoC evidence",
            "scripts/appfw product validate --json",
            "scripts/appfw product generate --check --json",
            "scripts/appfw product handoff --json",
        ]
    }
}

fn product_analysis_yaml(source_kind: &str, report: &Value) -> String {
    let app = report.get("app").unwrap_or(&Value::Null);
    let open_questions = report
        .get("open_questions")
        .and_then(Value::as_array)
        .map(|values| yaml_string_list(values.iter().filter_map(Value::as_str)))
        .unwrap_or_else(|| "[]\n".to_string());
    let recommendations = report
        .get("recommendations")
        .and_then(Value::as_array)
        .map(|values| yaml_string_list(values.iter().filter_map(Value::as_str)))
        .unwrap_or_else(|| "[]\n".to_string());
    let detected_signals = report
        .get("detected_signals")
        .and_then(Value::as_array)
        .map(|values| yaml_string_list(values.iter().filter_map(Value::as_str)))
        .unwrap_or_else(|| "[]\n".to_string());
    let model_entities = report_model_clue_list(report, "candidate_entities");
    let model_properties = report_model_clue_list(report, "candidate_properties");
    let model_routines = report_model_clue_list(report, "candidate_routines");
    let frontend_views = report_model_clue_list(report, "frontend_views");
    let review_kind = if source_kind == "legacy" {
        "legacy_application"
    } else {
        "poc"
    };
    let next_step = if source_kind == "legacy" {
        "Complete .appfw/legacy-modernization.yaml, then draft .appfw/model from reviewed modernization evidence."
    } else {
        "Review this analysis, then draft .appfw/model from reviewed PoC evidence."
    };
    format!(
        r#"version: 1
source_kind: {source_kind}
source: {review_kind}
product:
  app_name: {app_name}
  display_name: {display_name}
  schema: {schema}
  provider: {provider}
  ui: {ui}
detected_signals:
{detected_signals}model_clues:
  candidate_entities:
{model_entities}  candidate_properties:
{model_properties}  candidate_routines:
{model_routines}  frontend_views:
{frontend_views}open_questions:
{open_questions}recommendations:
{recommendations}candidate_model_plan:
  write_final_config: false
  source: target/appfw/product-analysis.json
agent_contract:
  next_step: {next_step}
"#,
        source_kind = yaml_scalar(source_kind),
        review_kind = yaml_scalar(review_kind),
        app_name = yaml_optional_scalar(app.get("name").and_then(Value::as_str)),
        display_name = yaml_optional_scalar(app.get("display_name").and_then(Value::as_str)),
        schema = yaml_optional_scalar(app.get("schema").and_then(Value::as_str)),
        provider = yaml_optional_scalar(app.get("provider").and_then(Value::as_str)),
        ui = yaml_optional_scalar(app.get("ui").and_then(Value::as_str)),
        detected_signals = indent_yaml_block(&detected_signals, 2),
        model_entities = indent_yaml_block(&model_entities, 4),
        model_properties = indent_yaml_block(&model_properties, 4),
        model_routines = indent_yaml_block(&model_routines, 4),
        frontend_views = indent_yaml_block(&frontend_views, 4),
        open_questions = indent_yaml_block(&open_questions, 2),
        recommendations = indent_yaml_block(&recommendations, 2),
        next_step = yaml_scalar(next_step),
    )
}

fn report_model_clue_list(report: &Value, key: &str) -> String {
    report
        .get("model_clues")
        .and_then(|value| value.get(key))
        .and_then(Value::as_array)
        .map(|values| yaml_string_list(values.iter().filter_map(Value::as_str)))
        .unwrap_or_else(|| "[]\n".to_string())
}

fn model_proposal_strings(values: Option<&Vec<Value>>) -> Vec<String> {
    let mut strings = BTreeSet::new();
    if let Some(values) = values {
        for value in values {
            if let Some(value) = value.as_str() {
                let value = value.trim();
                if !value.is_empty() {
                    strings.insert(value.to_string());
                }
            }
        }
    }
    strings.into_iter().collect()
}

#[derive(Default)]
struct ModelProposalEntityDraft {
    candidate_properties: BTreeSet<String>,
    semantic_labels: BTreeSet<String>,
    suggested_domain_names: BTreeSet<String>,
    source_profiles: BTreeSet<String>,
}

fn model_proposal_entity_drafts(analysis: &Value) -> BTreeMap<String, ModelProposalEntityDraft> {
    let mut drafts = BTreeMap::new();
    if let Some(evidence) = analysis.get("evidence").and_then(Value::as_array) {
        for item in evidence {
            if let Some(profile) = item.get("content_profile") {
                collect_model_proposal_entity_drafts(
                    profile,
                    &model_proposal_source_label(item, None),
                    &mut drafts,
                );
            }
            if let Some(profiles) = item.get("content_profiles").and_then(Value::as_array) {
                for profile_item in profiles {
                    if let Some(profile) = profile_item.get("profile") {
                        collect_model_proposal_entity_drafts(
                            profile,
                            &model_proposal_source_label(item, Some(profile_item)),
                            &mut drafts,
                        );
                    }
                }
            }
        }
    }
    drafts
}

fn collect_model_proposal_entity_drafts(
    profile: &Value,
    source_label: &str,
    drafts: &mut BTreeMap<String, ModelProposalEntityDraft>,
) {
    match profile
        .get("kind")
        .and_then(Value::as_str)
        .unwrap_or_default()
    {
        "spreadsheet" => {
            if let Some(tables) = profile.get("tables").and_then(Value::as_array) {
                for table in tables {
                    let table_name = table
                        .get("display_name")
                        .or_else(|| table.get("name"))
                        .and_then(Value::as_str)
                        .unwrap_or_default();
                    let Some(entity_name) = normalized_identifier(table_name) else {
                        continue;
                    };
                    let draft = drafts.entry(entity_name).or_default();
                    collect_string_array(
                        table,
                        "candidate_properties",
                        &mut draft.candidate_properties,
                    );
                    draft.source_profiles.insert(format!(
                        "spreadsheet table {} in {}",
                        table_name, source_label
                    ));
                }
            }
        }
        "tabular" => {
            let entities =
                model_proposal_strings(profile.get("candidate_entities").and_then(Value::as_array));
            if entities.len() == 1 {
                let draft = drafts.entry(entities[0].clone()).or_default();
                collect_string_array(
                    profile,
                    "candidate_properties",
                    &mut draft.candidate_properties,
                );
                draft
                    .source_profiles
                    .insert(format!("tabular data in {source_label}"));
            }
        }
        "markdown_reference" => {
            if let Some(labels) = profile
                .get("candidate_entity_labels")
                .and_then(Value::as_array)
            {
                for item in labels {
                    let Some(entity) = item.get("entity").and_then(Value::as_str) else {
                        continue;
                    };
                    let draft = drafts.entry(entity.to_string()).or_default();
                    if let Some(label) = item.get("label").and_then(Value::as_str) {
                        draft.semantic_labels.insert(label.to_string());
                    }
                    if let Some(suggested_name) = item.get("suggested_name").and_then(Value::as_str)
                    {
                        draft
                            .suggested_domain_names
                            .insert(suggested_name.to_string());
                    }
                    draft
                        .source_profiles
                        .insert(format!("source reference in {source_label}"));
                }
            }
        }
        _ => {}
    }
}

fn model_proposal_source_label(item: &Value, profile_item: Option<&Value>) -> String {
    if let Some(file) = profile_item
        .and_then(|value| value.get("file"))
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
    {
        return file.to_string();
    }
    item.get("configured")
        .or_else(|| item.get("path"))
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .unwrap_or("analysis evidence")
        .to_string()
}

fn model_proposal_assigned_properties(
    drafts: &BTreeMap<String, ModelProposalEntityDraft>,
) -> BTreeSet<String> {
    drafts
        .values()
        .flat_map(|draft| draft.candidate_properties.iter().cloned())
        .collect()
}

fn model_proposal_unassigned_properties(
    values: Option<&Vec<Value>>,
    assigned_properties: &BTreeSet<String>,
) -> Vec<String> {
    model_proposal_strings(values)
        .into_iter()
        .filter(|value| !assigned_properties.contains(value))
        .collect()
}

fn model_proposal_entities(
    values: Option<&Vec<Value>>,
    drafts: &BTreeMap<String, ModelProposalEntityDraft>,
) -> Vec<Value> {
    let mut names = model_proposal_strings(values)
        .into_iter()
        .collect::<BTreeSet<_>>();
    names.extend(drafts.keys().cloned());
    names
        .into_iter()
        .map(|name| {
            let draft = drafts.get(&name);
            json!({
                "name": name,
                "source": "model_clues.candidate_entities",
                "source_profiles": draft
                    .map(|draft| draft.source_profiles.iter().cloned().collect::<Vec<_>>())
                    .unwrap_or_default(),
                "semantic_labels": draft
                    .map(|draft| draft.semantic_labels.iter().cloned().collect::<Vec<_>>())
                    .unwrap_or_default(),
                "suggested_domain_names": draft
                    .map(|draft| draft.suggested_domain_names.iter().cloned().collect::<Vec<_>>())
                    .unwrap_or_default(),
                "candidate_properties": draft
                    .map(|draft| draft.candidate_properties.iter().cloned().collect::<Vec<_>>())
                    .unwrap_or_default(),
                "persistence": "candidate",
                "classification": "needs_review",
                "review_status": "proposed"
            })
        })
        .collect()
}

fn model_proposal_implementation_plan(
    source_kind: &str,
    app: &Value,
    candidate_entities: &[Value],
) -> Value {
    let schema = app
        .get("schema")
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .unwrap_or("<schema>");
    let schema_root = format!(".appfw/model/schemas/{schema}");
    let mut entity_tasks = candidate_entities
        .iter()
        .filter_map(|entity| entity.get("name").and_then(Value::as_str))
        .filter(|name| !name.trim().is_empty())
        .map(|name| {
            json!({
                "entity": name,
                "suggested_source_path": format!("{schema_root}/entity_types/{name}.yaml"),
                "review": [
                    "Confirm business noun and display caption.",
                    "Separate persisted properties from report-only or derived values.",
                    "Choose data types, nullability, defaults, uniqueness, indexes, and classification.",
                    "Decide whether this is a table entity, lookup, DTO/read model, or service result."
                ]
            })
        })
        .collect::<Vec<_>>();
    if entity_tasks.is_empty() {
        entity_tasks.push(json!({
            "entity": null,
            "suggested_source_path": format!("{schema_root}/entity_types/<entity>.yaml"),
            "review": [
                "Name at least one durable business entity from reviewed source evidence.",
                "Avoid copying sample CRM entities or UI-only labels as persisted tables."
            ]
        }));
    }

    let legacy_steps = if source_kind == "legacy" {
        vec![
            "Complete .appfw/legacy-modernization.yaml with modernization slice, stored procedure disposition, migration posture, and hardening evidence.",
            "Map legacy routes/controllers/jobs/stored procedures to target entities, services, custom methods, or retired behavior.",
            "Plan migration reconciliation, rollback, and parallel-run API evidence before release."
        ]
    } else {
        vec![
            "Review workbook/static-data/HTML intent before writing source config.",
            "Map prototype dashboards, forms, filters, and charts to generated model, custom methods, and product frontend routes.",
            "Keep ambiguous spreadsheet/report fields as open questions until a reviewer classifies them."
        ]
    };

    json!({
        "writes_final_model": false,
        "schema_source_root": schema_root,
        "required_source_files": [
            {
                "path": format!(".appfw/model/schemas/{schema}/_res.yaml"),
                "purpose": "Declare schema metadata, data source binding, and product classification posture."
            },
            {
                "path": format!(".appfw/model/schemas/{schema}/entity_types/*.yaml"),
                "purpose": "Declare reviewed persisted entities, DTO/read models, lookup entities, properties, facets, and custom methods."
            },
            {
                "path": format!(".appfw/model/schemas/{schema}/relationships/*.yaml"),
                "purpose": "Declare reviewed NavToOne, NavToMany, and junction-backed many-to-many semantics."
            },
            {
                "path": format!(".appfw/model/schemas/{schema}/rbac/*.rego"),
                "purpose": "Declare deny-by-default access policy and tenant/data-scope rules for each entity."
            },
            {
                "path": "api_tests/src/schemas/<schema>/*.rs",
                "purpose": "Add generated/product API scenarios after model generation."
            }
        ],
        "entity_tasks": entity_tasks,
        "relationship_tasks": [
            "Identify parent/child ownership, lookup references, and many-to-many junctions from reviewed source evidence.",
            "Use NavToOne for one reference/parent, NavToMany for child collections, and NavToMany with explicit junction semantics for many-to-many.",
            "Preserve policy, tenant, audit, and QueryIR paths for relationship traversal."
        ],
        "service_and_method_tasks": [
            "Move durable calculations, report commands, and workflow actions into product services or custom methods.",
            "Use provider routines only when there is a reviewed database-side reason and tests/evidence keep the path living."
        ],
        "frontend_tasks": [
            "Use the PoC UI as intent and the CRM frontend as implementation reference.",
            "Create enterprise dashboards, grids, forms, lookup selectors, validation, dirty-state, dark/light mode, and E2E/a11y evidence when UI is in scope."
        ],
        "evidence_tasks": legacy_steps,
        "validation_sequence": [
            "scripts/appfw product validate --json",
            "scripts/appfw product generate",
            "scripts/appfw product generate --check --json",
            "scripts/appfw product test --fast --json",
            "scripts/appfw product handoff --json"
        ],
        "review_checkpoint": "Human/architect review is required before generated config is treated as product source of truth."
    })
}

fn model_proposal_review_decisions(
    source_kind: &str,
    app: &Value,
    candidate_entities: &[Value],
    unassigned_properties: &[String],
    candidate_routines: &[String],
    frontend_views: &[String],
    integration_points: &[String],
) -> Value {
    let schema = app
        .get("schema")
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .unwrap_or("<schema>");
    let schema_root = format!(".appfw/model/schemas/{schema}");
    let entity_decisions = if candidate_entities.is_empty() {
        vec![json!({
            "candidate": null,
            "decision": "needs_review",
            "final_name": null,
            "config_kind": "needs_review",
            "classification": "needs_review",
            "suggested_source_path": format!("{schema_root}/entity_types/<entity>.yaml"),
            "notes": "Name at least one durable product entity from reviewed evidence.",
            "evidence_to_check": [
                "source artifacts",
                "business glossary",
                "ownership and lifecycle",
                "policy and classification"
            ]
        })]
    } else {
        candidate_entities
            .iter()
            .filter_map(|entity| entity.get("name").and_then(Value::as_str))
            .map(|name| {
                json!({
                    "candidate": name,
                    "decision": "needs_review",
                    "final_name": null,
                    "config_kind": "entity|lookup|dto|reject|defer",
                    "classification": "needs_review",
                    "suggested_source_path": format!("{schema_root}/entity_types/{name}.yaml"),
                    "notes": null,
                    "evidence_to_check": [
                        "source rows or table identity",
                        "business noun and lifecycle",
                        "properties versus derived/report-only fields",
                        "classification and access policy"
                    ]
                })
            })
            .collect::<Vec<_>>()
    };
    let property_decisions = unassigned_properties
        .iter()
        .map(|property| {
            json!({
                "candidate": property,
                "decision": "needs_review",
                "attach_to_entity": null,
                "config_kind": "property|lookup|derived|dto_field|reject|defer",
                "datatype": "needs_review",
                "classification": "needs_review",
                "notes": null
            })
        })
        .collect::<Vec<_>>();
    let routine_decisions = candidate_routines
        .iter()
        .map(|routine| {
            json!({
                "candidate": routine,
                "decision": "needs_review",
                "implementation": "custom_method|product_service|provider_routine|generated_query|reject|defer",
                "portable_by_default": true,
                "tests_required": [
                    "unit or API scenario",
                    "provider routine evidence when retained"
                ],
                "notes": null
            })
        })
        .collect::<Vec<_>>();
    let frontend_decisions = frontend_views
        .iter()
        .map(|view| {
            json!({
                "candidate": view,
                "decision": "needs_review",
                "surface": "dashboard|grid|form|report|workflow|defer",
                "data_contract": "generated entity, custom method, or DTO",
                "evidence_required": [
                    "route or screenshot intent",
                    "E2E/a11y when UI is in scope"
                ],
                "notes": null
            })
        })
        .collect::<Vec<_>>();
    let integration_decisions = integration_points
        .iter()
        .map(|integration| {
            json!({
                "candidate": integration,
                "decision": "needs_review",
                "ingress_or_egress": "HTTP|Kafka|MCP|file|external_service|defer",
                "actor_context": "user|service|agent|system",
                "policy_audit_required": true,
                "notes": null
            })
        })
        .collect::<Vec<_>>();
    let legacy_review = if source_kind == "legacy" {
        json!({
            "stored_procedure_disposition_required": true,
            "migration_parallel_run_required": true,
            "source_inventory_required": true
        })
    } else {
        json!({
            "stored_procedure_disposition_required": false,
            "migration_parallel_run_required": false,
            "source_inventory_required": false
        })
    };

    json!({
        "writes_final_model": false,
        "allowed_decisions": ["needs_review", "needs_discovery", "accept", "reject", "defer", "split", "merge"],
        "decision_states": {
            "unresolved": ["needs_review", "needs_discovery", "missing"],
            "terminal": ["accept", "reject", "defer", "split", "merge"]
        },
        "instructions": [
            "Review this ledger before writing .appfw/model.",
            "Keep unresolved ambiguity as needs_review or needs_discovery.",
            "Use defer only as an explicit terminal decision with follow-up notes.",
            "Do not treat this proposal as generated model source.",
            "Set accepted_for_config only after entity, relationship, classification, policy, and evidence decisions are explicit."
        ],
        "entity_decisions": entity_decisions,
        "property_decisions": property_decisions,
        "relationship_decisions": [
            {
                "candidate": null,
                "decision": "needs_discovery",
                "relationship_kind": "NavToOne|NavToMany|NavToManyWithJunction",
                "source": "reviewed source evidence",
                "notes": "Add explicit relationship decisions before modeling relationship YAML."
            }
        ],
        "routine_decisions": routine_decisions,
        "frontend_decisions": frontend_decisions,
        "integration_decisions": integration_decisions,
        "legacy_review": legacy_review,
        "sign_off": {
            "model_owner": null,
            "reviewed_at_utc": null,
            "accepted_for_config": false
        }
    })
}

fn model_proposal_yaml(report: &Value) -> String {
    let app = report.get("app").unwrap_or(&Value::Null);
    let proposal = report.get("proposal").unwrap_or(&Value::Null);
    let entities =
        yaml_model_proposal_entities(proposal.get("candidate_entities").and_then(Value::as_array));
    let properties = proposal
        .get("unassigned_properties")
        .and_then(Value::as_array)
        .map(|values| yaml_string_list(values.iter().filter_map(Value::as_str)))
        .unwrap_or_else(|| "[]\n".to_string());
    let routines = proposal
        .get("candidate_routines")
        .and_then(Value::as_array)
        .map(|values| yaml_string_list(values.iter().filter_map(Value::as_str)))
        .unwrap_or_else(|| "[]\n".to_string());
    let views = proposal
        .get("frontend_views")
        .and_then(Value::as_array)
        .map(|values| yaml_string_list(values.iter().filter_map(Value::as_str)))
        .unwrap_or_else(|| "[]\n".to_string());
    let integrations = proposal
        .get("integration_points")
        .and_then(Value::as_array)
        .map(|values| yaml_string_list(values.iter().filter_map(Value::as_str)))
        .unwrap_or_else(|| "[]\n".to_string());
    let open_questions = proposal
        .get("open_questions")
        .and_then(Value::as_array)
        .map(|values| yaml_string_list(values.iter().filter_map(Value::as_str)))
        .unwrap_or_else(|| "[]\n".to_string());
    let guardrails = report
        .get("guardrails")
        .and_then(Value::as_array)
        .map(|values| yaml_string_list(values.iter().filter_map(Value::as_str)))
        .unwrap_or_else(|| "[]\n".to_string());
    let implementation_plan =
        yaml_model_proposal_implementation_plan(report.get("implementation_plan"));
    let review_decisions = yaml_value_block(report.get("review_decisions"));
    let next_commands = report
        .get("next_commands")
        .and_then(Value::as_array)
        .map(|values| yaml_string_list(values.iter().filter_map(Value::as_str)))
        .unwrap_or_else(|| "[]\n".to_string());

    format!(
        r#"version: 1
source: target/appfw/product-analysis.json
source_kind: {source_kind}
product:
  app_name: {app_name}
  display_name: {display_name}
  schema: {schema}
  provider: {provider}
  ui: {ui}
proposal:
  writes_final_model: false
  requires_human_review: true
  candidate_entities:
{entities}  unassigned_properties:
{properties}  candidate_routines:
{routines}  frontend_views:
{views}  integration_points:
{integrations}  open_questions:
{open_questions}implementation_plan:
{implementation_plan}review_decisions:
{review_decisions}guardrails:
{guardrails}next_commands:
{next_commands}"#,
        source_kind = yaml_optional_scalar(report.get("source_kind").and_then(Value::as_str)),
        app_name = yaml_optional_scalar(app.get("name").and_then(Value::as_str)),
        display_name = yaml_optional_scalar(app.get("display_name").and_then(Value::as_str)),
        schema = yaml_optional_scalar(app.get("schema").and_then(Value::as_str)),
        provider = yaml_optional_scalar(app.get("provider").and_then(Value::as_str)),
        ui = yaml_optional_scalar(app.get("ui").and_then(Value::as_str)),
        entities = indent_yaml_block(&entities, 4),
        properties = indent_yaml_block(&properties, 4),
        routines = indent_yaml_block(&routines, 4),
        views = indent_yaml_block(&views, 4),
        integrations = indent_yaml_block(&integrations, 4),
        open_questions = indent_yaml_block(&open_questions, 4),
        implementation_plan = indent_yaml_block(&implementation_plan, 2),
        review_decisions = indent_yaml_block(&review_decisions, 2),
        guardrails = indent_yaml_block(&guardrails, 2),
        next_commands = indent_yaml_block(&next_commands, 2),
    )
}

fn yaml_model_proposal_implementation_plan(value: Option<&Value>) -> String {
    let Some(value) = value else {
        return "[]\n".to_string();
    };
    let source_files = value
        .get("required_source_files")
        .and_then(Value::as_array)
        .map(|values| yaml_model_proposal_source_files(values))
        .unwrap_or_else(|| "[]\n".to_string());
    let entity_tasks = value
        .get("entity_tasks")
        .and_then(Value::as_array)
        .map(|values| yaml_model_proposal_entity_tasks(values))
        .unwrap_or_else(|| "[]\n".to_string());
    let relationship_tasks = value
        .get("relationship_tasks")
        .and_then(Value::as_array)
        .map(|values| yaml_string_list(values.iter().filter_map(Value::as_str)))
        .unwrap_or_else(|| "[]\n".to_string());
    let service_tasks = value
        .get("service_and_method_tasks")
        .and_then(Value::as_array)
        .map(|values| yaml_string_list(values.iter().filter_map(Value::as_str)))
        .unwrap_or_else(|| "[]\n".to_string());
    let frontend_tasks = value
        .get("frontend_tasks")
        .and_then(Value::as_array)
        .map(|values| yaml_string_list(values.iter().filter_map(Value::as_str)))
        .unwrap_or_else(|| "[]\n".to_string());
    let evidence_tasks = value
        .get("evidence_tasks")
        .and_then(Value::as_array)
        .map(|values| yaml_string_list(values.iter().filter_map(Value::as_str)))
        .unwrap_or_else(|| "[]\n".to_string());
    let validation_sequence = value
        .get("validation_sequence")
        .and_then(Value::as_array)
        .map(|values| yaml_string_list(values.iter().filter_map(Value::as_str)))
        .unwrap_or_else(|| "[]\n".to_string());

    format!(
        r#"writes_final_model: false
schema_source_root: {schema_source_root}
required_source_files:
{source_files}entity_tasks:
{entity_tasks}relationship_tasks:
{relationship_tasks}service_and_method_tasks:
{service_tasks}frontend_tasks:
{frontend_tasks}evidence_tasks:
{evidence_tasks}validation_sequence:
{validation_sequence}review_checkpoint: {review_checkpoint}
"#,
        schema_source_root =
            yaml_optional_scalar(value.get("schema_source_root").and_then(Value::as_str)),
        source_files = indent_yaml_block(&source_files, 2),
        entity_tasks = indent_yaml_block(&entity_tasks, 2),
        relationship_tasks = indent_yaml_block(&relationship_tasks, 2),
        service_tasks = indent_yaml_block(&service_tasks, 2),
        frontend_tasks = indent_yaml_block(&frontend_tasks, 2),
        evidence_tasks = indent_yaml_block(&evidence_tasks, 2),
        validation_sequence = indent_yaml_block(&validation_sequence, 2),
        review_checkpoint =
            yaml_optional_scalar(value.get("review_checkpoint").and_then(Value::as_str)),
    )
}

fn yaml_model_proposal_source_files(values: &[Value]) -> String {
    if values.is_empty() {
        return "[]\n".to_string();
    }
    values
        .iter()
        .filter_map(|value| {
            let path = value.get("path").and_then(Value::as_str)?;
            Some(format!(
                "- path: {}\n  purpose: {}\n",
                yaml_scalar(path),
                yaml_optional_scalar(value.get("purpose").and_then(Value::as_str))
            ))
        })
        .collect::<String>()
}

fn yaml_model_proposal_entity_tasks(values: &[Value]) -> String {
    if values.is_empty() {
        return "[]\n".to_string();
    }
    values
        .iter()
        .map(|value| {
            let review = value
                .get("review")
                .and_then(Value::as_array)
                .map(|values| yaml_string_list(values.iter().filter_map(Value::as_str)))
                .unwrap_or_else(|| "[]\n".to_string());
            format!(
                "- entity: {}\n  suggested_source_path: {}\n  review:\n{}\n",
                yaml_optional_scalar(value.get("entity").and_then(Value::as_str)),
                yaml_optional_scalar(value.get("suggested_source_path").and_then(Value::as_str)),
                indent_yaml_block(&review, 4)
            )
        })
        .collect::<String>()
}

fn yaml_model_proposal_entities(values: Option<&Vec<Value>>) -> String {
    let Some(values) = values else {
        return "[]\n".to_string();
    };
    if values.is_empty() {
        return "[]\n".to_string();
    }

    values
        .iter()
        .filter_map(|value| {
            let name = value.get("name").and_then(Value::as_str)?;
            let properties = value
                .get("candidate_properties")
                .and_then(Value::as_array)
                .map(|values| yaml_string_list(values.iter().filter_map(Value::as_str)))
                .unwrap_or_else(|| "[]\n".to_string());
            let source_profiles = value
                .get("source_profiles")
                .and_then(Value::as_array)
                .map(|values| yaml_string_list(values.iter().filter_map(Value::as_str)))
                .unwrap_or_else(|| "[]\n".to_string());
            let semantic_labels = value
                .get("semantic_labels")
                .and_then(Value::as_array)
                .map(|values| yaml_string_list(values.iter().filter_map(Value::as_str)))
                .unwrap_or_else(|| "[]\n".to_string());
            let suggested_domain_names = value
                .get("suggested_domain_names")
                .and_then(Value::as_array)
                .map(|values| yaml_string_list(values.iter().filter_map(Value::as_str)))
                .unwrap_or_else(|| "[]\n".to_string());
            Some(format!(
                "- name: {}\n  classification: {}\n  persistence: {}\n  review_status: {}\n  semantic_labels:\n{}  suggested_domain_names:\n{}  candidate_properties:\n{}  source_profiles:\n{}\n",
                yaml_scalar(name),
                yaml_optional_scalar(value.get("classification").and_then(Value::as_str)),
                yaml_optional_scalar(value.get("persistence").and_then(Value::as_str)),
                yaml_optional_scalar(value.get("review_status").and_then(Value::as_str)),
                indent_yaml_block(&semantic_labels, 4),
                indent_yaml_block(&suggested_domain_names, 4),
                indent_yaml_block(&properties, 4),
                indent_yaml_block(&source_profiles, 4)
            ))
        })
        .collect::<String>()
}

fn yaml_value_block(value: Option<&Value>) -> String {
    let Some(value) = value else {
        return "[]\n".to_string();
    };
    serde_yaml::to_string(value).unwrap_or_else(|_| "[]\n".to_string())
}

fn yaml_string_list<'a, I>(values: I) -> String
where
    I: IntoIterator<Item = &'a str>,
{
    let values = values.into_iter().collect::<Vec<_>>();
    if values.is_empty() {
        return "[]\n".to_string();
    }
    values
        .into_iter()
        .map(|value| format!("- {}\n", yaml_scalar(value)))
        .collect::<String>()
}

fn indent_yaml_block(block: &str, spaces: usize) -> String {
    if block == "[]\n" {
        return format!("{}[]\n", " ".repeat(spaces));
    }
    let prefix = " ".repeat(spaces);
    block
        .lines()
        .map(|line| format!("{prefix}{line}\n"))
        .collect::<String>()
}

fn build_boundary_check(roots: &CommandRoots) -> Result<BoundaryCheckReport, String> {
    let mut files = Vec::new();
    let handlers_root = roots.app_root.join("backend/src/handlers");
    collect_rs_files(&handlers_root, &mut |path| {
        if is_product_owned_handler(path) {
            files.push(path.to_path_buf());
        }
    })?;
    let services_root = roots.app_root.join("backend/src/services");
    collect_rs_files(&services_root, &mut |path| {
        files.push(path.to_path_buf());
    })?;
    files.sort();

    let mut violations = Vec::new();
    for file in &files {
        check_boundary_file(&roots.app_root, file, &mut violations)?;
    }
    check_retired_product_template_surfaces(roots, &mut violations);
    check_retired_framework_root_surfaces(roots, &mut violations);
    let product_provider_sources = check_product_provider_sources(roots, &mut violations)?;

    let report_path = roots.report_root.join("boundary_check.json");
    let report = BoundaryCheckReport {
        command: "boundary-check",
        ok: violations.is_empty(),
        roots: roots.report(),
        checked_files: files.len(),
        product_provider_sources,
        violations,
        report_path: report_path.display().to_string(),
    };
    if let Some(parent) = report_path.parent() {
        fs::create_dir_all(parent)
            .map_err(|err| format!("failed to create {}: {err}", parent.display()))?;
    }
    let json = serde_json::to_string_pretty(&report)
        .map_err(|err| format!("failed to serialize boundary check report: {err}"))?;
    fs::write(&report_path, format!("{json}\n"))
        .map_err(|err| format!("failed to write {}: {err}", report_path.display()))?;
    Ok(report)
}

fn check_retired_product_template_surfaces(
    roots: &CommandRoots,
    violations: &mut Vec<BoundaryCheckViolation>,
) {
    for product_root in boundary_product_roots(roots) {
        for surface in RETIRED_PRODUCT_TEMPLATE_SURFACES {
            let path = product_root.join(surface.relative_path);
            if path.exists() {
                violations.push(BoundaryCheckViolation {
                    path: boundary_surface_path(roots, &path),
                    rule: "retired_product_template_surface".to_string(),
                    detail: format!(
                        "{}; do not copy retired framework implementation back into product templates",
                        surface.detail
                    ),
                    symbol: Some(surface.relative_path.to_string()),
                });
            }
        }
    }
}

fn check_retired_framework_root_surfaces(
    roots: &CommandRoots,
    violations: &mut Vec<BoundaryCheckViolation>,
) {
    for surface in RETIRED_FRAMEWORK_ROOT_SURFACES {
        let path = roots.framework_root.join(surface.relative_path);
        if path.exists() {
            violations.push(BoundaryCheckViolation {
                path: boundary_surface_path(roots, &path),
                rule: "retired_framework_root_surface".to_string(),
                detail: format!(
                    "{}; do not reintroduce retired framework fixture facades",
                    surface.detail
                ),
                symbol: Some(surface.relative_path.to_string()),
            });
        }
    }
}

fn check_product_provider_sources(
    roots: &CommandRoots,
    violations: &mut Vec<BoundaryCheckViolation>,
) -> Result<Vec<ProductProviderSourceReport>, String> {
    let mut reports = Vec::new();
    for product_root in boundary_product_roots(roots) {
        let active_providers = active_schema_provider_sources(&product_root)?;
        let provider_dependencies = backend_provider_dependencies(&product_root)?;
        let mut sources = Vec::new();
        let mut dependencies = Vec::new();
        for surface in PROVIDER_TEMPLATE_SURFACES {
            let path = product_root.join(surface.relative_path);
            if path.exists() {
                let active = active_providers.contains(surface.provider);
                let rust_files = count_rust_files(&path)?;
                if !active {
                    violations.push(BoundaryCheckViolation {
                        path: boundary_surface_path(roots, &path),
                        rule: "inactive_provider_source".to_string(),
                        detail: format!(
                            "product templates must not carry {provider} provider source unless a configured schema or app topology data source uses a {data_source_type} data source",
                            provider = surface.provider,
                            data_source_type = surface.data_source_type
                        ),
                        symbol: Some(surface.relative_path.to_string()),
                    });
                }
                sources.push(ProductProviderSource {
                    provider: surface.provider.to_string(),
                    path: boundary_surface_path(roots, &path),
                    active,
                    rust_files,
                });
            }

            if provider_dependencies.contains(surface.provider) {
                let active = active_providers.contains(surface.provider);
                if !active {
                    violations.push(BoundaryCheckViolation {
                        path: boundary_surface_path(roots, &product_root.join("backend/Cargo.toml")),
                        rule: "inactive_provider_dependency".to_string(),
                        detail: format!(
                            "product backend manifests must not depend on {dependency} unless a configured schema or app topology data source uses a {data_source_type} data source",
                            dependency = surface.dependency_key,
                            data_source_type = surface.data_source_type
                        ),
                        symbol: Some(surface.dependency_key.to_string()),
                    });
                }
                dependencies.push(ProductProviderDependency {
                    provider: surface.provider.to_string(),
                    dependency: surface.dependency_key.to_string(),
                    active,
                });
            }
        }

        reports.push(ProductProviderSourceReport {
            product_root: boundary_surface_path(roots, &product_root),
            active_providers: active_providers.into_iter().collect(),
            sources,
            dependencies,
        });
    }
    Ok(reports)
}

fn backend_provider_dependencies(product_root: &Path) -> Result<BTreeSet<String>, String> {
    let manifest_path = product_root.join("backend/Cargo.toml");
    if !manifest_path.exists() {
        return Ok(BTreeSet::new());
    }
    let content = fs::read_to_string(&manifest_path)
        .map_err(|err| format!("failed to read {}: {err}", manifest_path.display()))?;
    let mut dependencies = BTreeSet::new();
    let mut in_dependencies = false;
    for raw_line in content.lines() {
        let line = raw_line.split('#').next().unwrap_or("").trim();
        if line.is_empty() {
            continue;
        }
        if line.starts_with('[') && line.ends_with(']') {
            in_dependencies = line == "[dependencies]";
            continue;
        }
        if !in_dependencies {
            continue;
        }
        if let Some(provider) = provider_key_for_dependency_line(line) {
            dependencies.insert(provider.to_string());
        }
    }
    Ok(dependencies)
}

fn provider_key_for_dependency_line(line: &str) -> Option<&'static str> {
    let key = line
        .split_once('=')
        .map(|(key, _)| key.trim().trim_matches('"'))?;
    PROVIDER_TEMPLATE_SURFACES
        .iter()
        .find(|surface| {
            key == surface.dependency_key
                || key == surface.package_name
                || line.contains(&format!(r#"package = "{}""#, surface.package_name))
        })
        .map(|surface| surface.provider)
}

fn active_schema_provider_sources(product_root: &Path) -> Result<BTreeSet<String>, String> {
    let data_source_providers = data_source_provider_map(product_root)?;
    let mut active_providers = BTreeSet::new();
    let schemas_root = product_root.join(".appfw/model/schemas");
    if schemas_root.exists() {
        let entries = fs::read_dir(&schemas_root)
            .map_err(|err| format!("failed to read {}: {err}", schemas_root.display()))?;
        for entry in entries {
            let path = entry
                .map_err(|err| {
                    format!(
                        "failed to read entry under {}: {err}",
                        schemas_root.display()
                    )
                })?
                .path()
                .join("_res.yaml");
            if !path.exists() {
                continue;
            }
            let yaml = read_yaml_file(&path)?;
            let Some(data_source_name) = yaml_string_field(&yaml, "data_source_name") else {
                continue;
            };
            if let Some(provider) = data_source_providers.get(data_source_name) {
                active_providers.insert(provider.clone());
            }
        }
    }

    let manifest_path = product_root.join(".appfw/manifest.yaml");
    if manifest_path.exists() {
        let manifest = read_yaml_file(&manifest_path)?;
        if let Some(topology_data_sources) = manifest
            .as_mapping()
            .and_then(|mapping| mapping.get(serde_yaml::Value::String("topology".to_string())))
            .and_then(serde_yaml::Value::as_mapping)
            .and_then(|mapping| mapping.get(serde_yaml::Value::String("data_sources".to_string())))
            .and_then(serde_yaml::Value::as_sequence)
        {
            for item in topology_data_sources {
                if let Some(name) = yaml_string_field(item, "name") {
                    if let Some(provider) = data_source_providers.get(name) {
                        active_providers.insert(provider.clone());
                        continue;
                    }
                }
                if let Some(provider) =
                    yaml_string_field(item, "provider").and_then(provider_key_for_data_source_type)
                {
                    active_providers.insert(provider.to_string());
                }
            }
        }
    }

    expand_shared_provider_surfaces(&mut active_providers);
    Ok(active_providers)
}

fn expand_shared_provider_surfaces(active_providers: &mut BTreeSet<String>) {
    if active_providers.contains("fabric_sql_analytics") {
        active_providers.insert("mssql".to_string());
    }
}

fn data_source_provider_map(product_root: &Path) -> Result<BTreeMap<String, String>, String> {
    let path = product_root.join(".appfw/model/data_sources/_res.yaml");
    if !path.exists() {
        return Ok(BTreeMap::new());
    }
    let yaml = read_yaml_file(&path)?;
    let mut providers = BTreeMap::new();
    let Some(items) = yaml.as_sequence() else {
        return Ok(providers);
    };
    for item in items {
        let Some(name) = yaml_string_field(item, "name") else {
            continue;
        };
        let Some(data_source_type) = yaml_string_field(item, "data_source_type") else {
            continue;
        };
        if let Some(provider) = provider_key_for_data_source_type(data_source_type) {
            providers.insert(name.to_string(), provider.to_string());
        }
    }
    Ok(providers)
}

fn read_yaml_file(path: &Path) -> Result<serde_yaml::Value, String> {
    let content = fs::read_to_string(path)
        .map_err(|err| format!("failed to read {}: {err}", path.display()))?;
    serde_yaml::from_str(&content)
        .map_err(|err| format!("failed to parse {}: {err}", path.display()))
}

fn yaml_string_field<'a>(value: &'a serde_yaml::Value, key: &str) -> Option<&'a str> {
    value
        .as_mapping()
        .and_then(|mapping| mapping.get(serde_yaml::Value::String(key.to_string())))
        .and_then(serde_yaml::Value::as_str)
}

fn provider_key_for_data_source_type(data_source_type: &str) -> Option<&'static str> {
    PROVIDER_TEMPLATE_SURFACES
        .iter()
        .find(|surface| surface.data_source_type == data_source_type)
        .map(|surface| surface.provider)
}

fn boundary_product_roots(roots: &CommandRoots) -> Vec<PathBuf> {
    let mut product_roots = Vec::new();
    if is_framework_checkout(&roots.app_root) {
        push_product_root(
            &mut product_roots,
            roots.framework_root.join("examples/products/crm"),
        );
    } else {
        push_product_root(&mut product_roots, roots.app_root.clone());
    }
    product_roots
}

fn is_framework_checkout(root: &Path) -> bool {
    root.join("app_gen/Cargo.toml").exists() && root.join("appfw_runtime/Cargo.toml").exists()
}

fn push_product_root(product_roots: &mut Vec<PathBuf>, root: PathBuf) {
    if !root.join("backend/src").exists() {
        return;
    }
    let root = normalize_path(&root);
    if !product_roots.iter().any(|existing| existing == &root) {
        product_roots.push(root);
    }
}

fn boundary_surface_path(roots: &CommandRoots, path: &Path) -> String {
    if path.starts_with(&roots.app_root) {
        relative_path(&roots.app_root, path)
    } else if path.starts_with(&roots.framework_root) {
        relative_path(&roots.framework_root, path)
    } else {
        path.display().to_string()
    }
}

fn check_boundary_file(
    app_root: &Path,
    path: &Path,
    violations: &mut Vec<BoundaryCheckViolation>,
) -> Result<(), String> {
    let content = fs::read_to_string(path)
        .map_err(|err| format!("failed to read {}: {err}", path.display()))?;
    let rel = relative_path(app_root, path);
    let file = match syn::parse_file(&content) {
        Ok(file) => file,
        Err(err) => {
            violations.push(BoundaryCheckViolation {
                path: rel,
                rule: "rust_parse".to_string(),
                detail: format!("failed to parse Rust source: {err}"),
                symbol: None,
            });
            return Ok(());
        }
    };

    if let Some(violation) = forbidden_framework_import(&file) {
        violations.push(BoundaryCheckViolation {
            path: rel.clone(),
            rule: violation.rule.to_string(),
            detail:
                "product-owned handlers/services must use crate::product_api instead of framework internals"
                    .to_string(),
            symbol: Some(violation.path),
        });
    }

    let standard_impls = file
        .items
        .iter()
        .filter_map(|item| match item {
            syn::Item::Fn(item_fn) if is_standard_handler_impl(item_fn) => {
                Some(item_fn.sig.ident.to_string())
            }
            _ => None,
        })
        .collect::<Vec<_>>();
    if !standard_impls.is_empty() && !content.contains("appfw: override-standard") {
        violations.push(BoundaryCheckViolation {
            path: rel,
            rule: "implicit_standard_handler_override".to_string(),
            detail: "standard generated handler defaults must include an appfw: override-standard marker when overridden".to_string(),
            symbol: Some(standard_impls.join(",")),
        });
    }

    Ok(())
}

fn count_rust_files(root: &Path) -> Result<usize, String> {
    let mut count = 0;
    collect_rs_files(root, &mut |_| {
        count += 1;
    })?;
    Ok(count)
}

fn collect_rs_files(root: &Path, f: &mut dyn FnMut(&Path)) -> Result<(), String> {
    if !root.exists() {
        return Ok(());
    }
    let entries =
        fs::read_dir(root).map_err(|err| format!("failed to read {}: {err}", root.display()))?;
    for entry in entries {
        let path = entry
            .map_err(|err| format!("failed to read entry under {}: {err}", root.display()))?
            .path();
        if path.is_dir() {
            collect_rs_files(&path, f)?;
            continue;
        }
        if path.extension().and_then(|ext| ext.to_str()) == Some("rs") {
            f(&path);
        }
    }
    Ok(())
}

fn is_product_owned_handler(path: &Path) -> bool {
    let Some(file_name) = path.file_name().and_then(|name| name.to_str()) else {
        return false;
    };
    if matches!(file_name, "mod.rs" | "generated.rs" | "selections.rs") {
        return false;
    }

    let path_text = path.to_string_lossy();
    if path_text.contains("/handlers/auth/") {
        return false;
    }

    path_text.contains("/handlers/")
}

fn forbidden_framework_import(file: &syn::File) -> Option<BoundaryViolation> {
    let mut visitor = BoundaryVisitor::default();
    visitor.visit_file(file);
    visitor.violation
}

#[derive(Default)]
struct BoundaryVisitor {
    violation: Option<BoundaryViolation>,
}

impl<'ast> Visit<'ast> for BoundaryVisitor {
    fn visit_item_use(&mut self, item: &'ast ItemUse) {
        self.inspect_use_tree(&item.tree, Vec::new());
        if self.violation.is_none() {
            visit::visit_item_use(self, item);
        }
    }

    fn visit_path(&mut self, path: &'ast syn::Path) {
        if self.violation.is_none() {
            let segments = path
                .segments
                .iter()
                .map(|segment| segment.ident.to_string())
                .collect::<Vec<_>>();
            self.inspect_segments(&segments);
        }
        if self.violation.is_none() {
            visit::visit_path(self, path);
        }
    }

    fn visit_expr_method_call(&mut self, call: &'ast syn::ExprMethodCall) {
        if let (true, Some(symbol)) = (
            self.violation.is_none(),
            forbidden_context_data_call_symbol(call),
        ) {
            self.violation = Some(BoundaryViolation {
                rule: "GraphQL context internals",
                path: symbol,
            });
        }
        if self.violation.is_none() {
            visit::visit_expr_method_call(self, call);
        }
    }
}

impl BoundaryVisitor {
    fn inspect_use_tree(&mut self, tree: &UseTree, prefix: Vec<String>) {
        if self.violation.is_some() {
            return;
        }

        match tree {
            UseTree::Path(path) => {
                let mut segments = prefix;
                segments.push(path.ident.to_string());
                self.inspect_segments(&segments);
                self.inspect_use_tree(&path.tree, segments);
            }
            UseTree::Name(name) => {
                let mut segments = prefix;
                segments.push(name.ident.to_string());
                self.inspect_segments(&segments);
            }
            UseTree::Rename(rename) => {
                let mut segments = prefix;
                segments.push(rename.ident.to_string());
                self.inspect_segments(&segments);
            }
            UseTree::Glob(_) => self.inspect_segments(&prefix),
            UseTree::Group(group) => {
                for item in &group.items {
                    self.inspect_use_tree(item, prefix.clone());
                    if self.violation.is_some() {
                        break;
                    }
                }
            }
        }
    }

    fn inspect_segments(&mut self, segments: &[String]) {
        if self.violation.is_some() {
            return;
        }
        if let Some(rule) = forbidden_crate_path_rule(segments) {
            self.violation = Some(BoundaryViolation {
                rule,
                path: segments.join("::"),
            });
        } else if let Some(rule) = forbidden_external_path_rule(segments) {
            self.violation = Some(BoundaryViolation {
                rule,
                path: segments.join("::"),
            });
        }
    }
}

fn forbidden_external_path_rule(segments: &[String]) -> Option<&'static str> {
    match (
        segments.first().map(String::as_str),
        segments.get(1).map(String::as_str),
    ) {
        (Some("async_graphql"), Some("Context")) => Some("GraphQL context internals"),
        _ => None,
    }
}

fn forbidden_context_data_call_symbol(call: &syn::ExprMethodCall) -> Option<String> {
    if !matches!(
        call.method.to_string().as_str(),
        "data_opt" | "data_unchecked"
    ) {
        return None;
    }

    let syn::Expr::Path(receiver_path) = call.receiver.as_ref() else {
        return None;
    };

    receiver_path
        .path
        .segments
        .last()
        .map(|segment| format!("{}.{}", segment.ident, call.method))
}

fn forbidden_crate_path_rule(segments: &[String]) -> Option<&'static str> {
    if segments.first().map(String::as_str) != Some("crate") {
        return None;
    }

    match segments.get(1).map(String::as_str) {
        Some("data") => Some("data runtime internals"),
        Some("config") => Some("config internals"),
        Some("routes") => Some("route internals"),
        Some("handlers") if segments.get(2).map(String::as_str) == Some("auth") => {
            Some("auth handler internals")
        }
        Some("mcp") => Some("MCP runtime internals"),
        Some("observability") => Some("observability internals"),
        Some("admin_ui") => Some("admin UI internals"),
        Some("app_state") => Some("app state internals"),
        Some("provider_certification") => Some("provider certification internals"),
        _ => None,
    }
}

fn is_standard_handler_impl(item_fn: &ItemFn) -> bool {
    matches!(
        item_fn.sig.ident.to_string().as_str(),
        "find_impl"
            | "get_impl"
            | "query_impl"
            | "aggregate_impl"
            | "create_impl"
            | "update_impl"
            | "delete_impl"
    )
}

fn boundary_check_text(report: &BoundaryCheckReport) -> String {
    let mut out = format!(
        "Boundary check: {}\nApp root: {}\nChecked files: {}\nReport: {}",
        if report.ok { "ok" } else { "failed" },
        report.roots.app_root,
        report.checked_files,
        report.report_path
    );
    if !report.violations.is_empty() {
        out.push_str("\nViolations:");
        for violation in &report.violations {
            out.push_str(&format!(
                "\n  {}: {} ({})",
                violation.path, violation.rule, violation.detail
            ));
            if let Some(symbol) = &violation.symbol {
                out.push_str(&format!(" via `{symbol}`"));
            }
        }
    }
    out
}

#[derive(Debug, Serialize)]
struct ProviderExplanation {
    command: &'static str,
    ok: bool,
    area: String,
    providers: Vec<ProviderArea>,
    source: String,
}

#[derive(Clone, Debug, Serialize)]
struct ProviderArea {
    provider: String,
    area: String,
    label: String,
    status: String,
    reason: Option<String>,
    compiler_contracts: Vec<String>,
    live_contracts: Vec<String>,
    verification_command: String,
}

fn explain_provider(
    repo_root: &Path,
    area: &str,
    provider: Option<&str>,
) -> Result<ProviderExplanation, String> {
    let profile = provider_profile(repo_root, provider)?;
    let wanted = normalize_key(area);
    let providers_json = profile
        .get("providers")
        .and_then(Value::as_array)
        .ok_or_else(|| "provider capability export did not include providers".to_string())?;
    let mut provider_areas = Vec::new();
    let mut available = BTreeSet::new();

    for provider_json in providers_json {
        let provider_key = provider_json
            .get("provider")
            .and_then(Value::as_str)
            .unwrap_or("unknown");
        let areas = provider_json
            .get("areas")
            .and_then(Value::as_array)
            .ok_or_else(|| format!("provider profile for {provider_key} did not include areas"))?;
        for area_json in areas {
            let area_key = area_json
                .get("area")
                .and_then(Value::as_str)
                .unwrap_or_default();
            let label = area_json
                .get("label")
                .and_then(Value::as_str)
                .unwrap_or(area_key);
            available.insert(area_key.to_string());
            if normalize_key(area_key) == wanted || normalize_key(label) == wanted {
                provider_areas.push(ProviderArea {
                    provider: provider_key.to_string(),
                    area: area_key.to_string(),
                    label: label.to_string(),
                    status: area_json
                        .get("status")
                        .and_then(Value::as_str)
                        .unwrap_or("unknown")
                        .to_string(),
                    reason: area_json
                        .get("reason")
                        .and_then(Value::as_str)
                        .map(str::to_string),
                    compiler_contracts: string_array(area_json.get("compiler_contracts")),
                    live_contracts: string_array(area_json.get("live_contracts")),
                    verification_command: format!(
                        "scripts/appfw framework provider-test --provider {provider_key} --json"
                    ),
                });
            }
        }
    }

    if provider_areas.is_empty() {
        return Err(format!(
            "unknown provider contract area `{area}`. Available areas: {}",
            available.into_iter().collect::<Vec<_>>().join(", ")
        ));
    }

    Ok(ProviderExplanation {
        command: "explain provider",
        ok: true,
        area: provider_areas
            .first()
            .map(|area| area.area.clone())
            .unwrap_or_else(|| area.to_string()),
        providers: provider_areas,
        source: "appfw-runtime provider certification library via appfw-cli exporter".to_string(),
    })
}

fn provider_profile(repo_root: &Path, provider: Option<&str>) -> Result<Value, String> {
    let mut command = Command::new("cargo");
    command
        .current_dir(repo_root)
        .args(["run", "--locked", "--quiet", "--manifest-path"])
        .arg(repo_root.join("appfw_cli/Cargo.toml"))
        .args(["--bin", "provider_certification_export", "--"]);
    if let Some(provider) = provider {
        command.args(["--provider", provider]);
    }
    let output = command
        .output()
        .map_err(|err| format!("failed to run provider capability exporter: {err}"))?;
    if !output.status.success() {
        return Err(format!(
            "provider capability exporter failed: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }
    serde_json::from_slice(&output.stdout).map_err(|err| {
        format!(
            "provider capability exporter returned invalid JSON: {err}; stdout: {}",
            String::from_utf8_lossy(&output.stdout)
        )
    })
}

fn string_array(value: Option<&Value>) -> Vec<String> {
    value
        .and_then(Value::as_array)
        .map(|values| {
            values
                .iter()
                .filter_map(Value::as_str)
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default()
}

fn provider_text(explanation: &ProviderExplanation) -> String {
    let mut out = format!(
        "Provider area: {}\nSource: {}",
        explanation.area, explanation.source
    );
    for area in &explanation.providers {
        out.push_str(&format!(
            "\n\n{}: {}\n  status: {}\n  compiler contracts: {}\n  live contracts: {}\n  verify: {}",
            area.provider,
            area.label,
            area.status,
            if area.compiler_contracts.is_empty() {
                "none".to_string()
            } else {
                area.compiler_contracts.join(", ")
            },
            if area.live_contracts.is_empty() {
                "none".to_string()
            } else {
                area.live_contracts.join(", ")
            },
            area.verification_command
        ));
        if let Some(reason) = &area.reason {
            out.push_str(&format!("\n  note: {reason}"));
        }
    }
    out
}

fn provider_sdk_rules(repo_root: &Path) -> Result<Value, String> {
    let output = Command::new("cargo")
        .current_dir(repo_root)
        .args(["run", "--locked", "--quiet", "--manifest-path"])
        .arg(repo_root.join("appfw_cli/Cargo.toml"))
        .args([
            "--bin",
            "provider_certification_export",
            "--",
            "--sdk-rules",
        ])
        .output()
        .map_err(|err| format!("failed to run provider SDK rule exporter: {err}"))?;
    if !output.status.success() {
        return Err(format!(
            "provider SDK rule exporter failed: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }
    serde_json::from_slice(&output.stdout).map_err(|err| {
        format!(
            "provider SDK rule exporter returned invalid JSON: {err}; stdout: {}",
            String::from_utf8_lossy(&output.stdout)
        )
    })
}

fn provider_sdk_text(rules: &Value) -> String {
    let mut out = "Provider SDK rules".to_string();
    if let Some(rule_sets) = rules.get("rule_sets").and_then(Value::as_array) {
        for rule_set in rule_sets {
            let name = rule_set
                .get("name")
                .and_then(Value::as_str)
                .unwrap_or("rules");
            out.push_str(&format!("\n\n{name}"));
            for rule in string_array(rule_set.get("rules")) {
                out.push_str(&format!("\n  - {rule}"));
            }
        }
    }
    if let Some(commands) = rules.get("required_commands").and_then(Value::as_array) {
        out.push_str("\n\nRequired commands:");
        for command in commands.iter().filter_map(Value::as_str) {
            out.push_str(&format!("\n  {command}"));
        }
    }
    out
}

#[derive(Clone, Debug, Serialize)]
struct HandoffReport {
    command: &'static str,
    ok: bool,
    generated_at: String,
    repo_root: String,
    product: ProductIdentity,
    roots: CommandRootReport,
    artifact_path: String,
    git: HandoffGit,
    changed_surfaces: Vec<HandoffSurface>,
    drift: HandoffDrift,
    verification: Vec<HandoffVerification>,
    recommended_commands: Vec<String>,
}

#[derive(Clone, Debug, Serialize)]
struct HandoffGit {
    branch: String,
    sha: String,
    dirty: bool,
}

#[derive(Clone, Debug, Serialize)]
struct HandoffSurface {
    path: String,
    status: String,
    change_kind: String,
    index_status: String,
    worktree_status: String,
    classification: String,
    ownership: String,
    safe_to_edit: bool,
    source_of_truth: String,
    verification: Vec<String>,
}

#[derive(Clone, Debug, Serialize)]
struct HandoffDrift {
    dirty: bool,
    total_changed: usize,
    generated_changed: usize,
    human_owned_changed: usize,
    framework_source_changed: usize,
    documentation_changed: usize,
    unknown_changed: usize,
    generated_paths: Vec<String>,
    check_command: String,
}

#[derive(Clone, Debug, Serialize)]
struct HandoffVerification {
    name: String,
    path: String,
    exists: bool,
    ok: Option<bool>,
    detail: String,
    sha256: Option<String>,
}

#[derive(Clone, Debug)]
struct GitChange {
    status: String,
    change_kind: String,
    index_status: String,
    worktree_status: String,
    path: String,
}

fn handoff_command(roots: &CommandRoots, json_output: bool, args: &[String]) -> Result<(), String> {
    if !args.is_empty() {
        return Err("handoff does not accept additional arguments".to_string());
    }

    let report = build_handoff(roots)?;
    let artifact_path = Path::new(&report.artifact_path);
    if let Some(parent) = artifact_path.parent() {
        fs::create_dir_all(parent)
            .map_err(|err| format!("failed to create {}: {err}", parent.display()))?;
    }
    fs::write(
        artifact_path,
        serde_json::to_string_pretty(&report)
            .map_err(|err| format!("failed to serialize handoff JSON: {err}"))?,
    )
    .map_err(|err| format!("failed to write {}: {err}", artifact_path.display()))?;

    emit(json_output, &report, || handoff_text(&report))
}

fn build_handoff(roots: &CommandRoots) -> Result<HandoffReport, String> {
    let repo_root = &roots.app_root;
    let changed_surfaces = build_changed_surfaces(repo_root)?;
    let framework_scope = normalize_path(&roots.app_root) == normalize_path(&roots.framework_root);
    let drift = handoff_drift(&changed_surfaces, framework_scope);
    let artifact_path = repo_root.join("target/appfw/agent-handoff.json");
    let recommended_commands = if framework_scope {
        vec![
            "scripts/appfw framework cli-test --json".to_string(),
            "scripts/appfw framework validate --json".to_string(),
            "scripts/appfw framework docs-check --json".to_string(),
            "scripts/appfw framework generate --check --json".to_string(),
            "scripts/appfw framework test --smoke --json".to_string(),
            "scripts/appfw framework test --fast --json".to_string(),
            "scripts/appfw framework wave2-status --json".to_string(),
            "scripts/appfw framework handoff --json".to_string(),
        ]
    } else {
        vec![
            "scripts/appfw product validate --json".to_string(),
            "scripts/appfw product boundary-check --json".to_string(),
            "scripts/appfw product generate --check --json".to_string(),
            "scripts/appfw product test --fast".to_string(),
            "scripts/appfw product test".to_string(),
            "scripts/appfw product handoff --json".to_string(),
        ]
    };

    Ok(HandoffReport {
        command: "handoff",
        ok: true,
        generated_at: Utc::now().to_rfc3339(),
        repo_root: repo_root.display().to_string(),
        product: product_identity(roots),
        roots: roots.report(),
        artifact_path: artifact_path.display().to_string(),
        git: HandoffGit {
            branch: git_output(repo_root, &["rev-parse", "--abbrev-ref", "HEAD"])
                .unwrap_or_else(|| "unknown".to_string()),
            sha: git_output(repo_root, &["rev-parse", "HEAD"])
                .unwrap_or_else(|| "unknown".to_string()),
            dirty: !changed_surfaces.is_empty(),
        },
        changed_surfaces,
        drift,
        verification: handoff_verification(roots)?,
        recommended_commands,
    })
}

fn build_changed_surfaces(repo_root: &Path) -> Result<Vec<HandoffSurface>, String> {
    let mut changes = git_status(repo_root)?;
    let mut seen_paths = changes
        .iter()
        .map(|change| change.path.clone())
        .collect::<BTreeSet<_>>();
    for change in git_branch_diff(repo_root)? {
        if seen_paths.insert(change.path.clone()) {
            changes.push(change);
        }
    }
    let mut changed_surfaces = Vec::new();
    for change in changes {
        let absolute = normalize_path(&repo_root.join(&change.path));
        let explanation = explain_ownership(repo_root, &change.path)
            .unwrap_or_else(|_| static_ownership(repo_root, &absolute, &change.path));
        changed_surfaces.push(HandoffSurface {
            path: change.path.clone(),
            status: change.status.clone(),
            change_kind: change.change_kind,
            index_status: change.index_status,
            worktree_status: change.worktree_status,
            classification: explanation.classification,
            ownership: explanation.ownership,
            safe_to_edit: explanation.safe_to_edit,
            source_of_truth: explanation.source_of_truth,
            verification: explanation.verification,
        });
    }
    Ok(changed_surfaces)
}

fn handoff_drift(changes: &[HandoffSurface], framework_scope: bool) -> HandoffDrift {
    let mut generated_paths = Vec::new();
    let mut generated_changed = 0;
    let mut human_owned_changed = 0;
    let mut framework_source_changed = 0;
    let mut documentation_changed = 0;
    let mut unknown_changed = 0;

    for change in changes {
        if change.classification == "documentation" {
            documentation_changed += 1;
            continue;
        }
        match change.ownership.as_str() {
            "generated" => {
                generated_changed += 1;
                generated_paths.push(change.path.clone());
            }
            "human_owned" | "application_source" => human_owned_changed += 1,
            "framework_source" => framework_source_changed += 1,
            _ => unknown_changed += 1,
        }
    }

    HandoffDrift {
        dirty: !changes.is_empty(),
        total_changed: changes.len(),
        generated_changed,
        human_owned_changed,
        framework_source_changed,
        documentation_changed,
        unknown_changed,
        generated_paths,
        check_command: if framework_scope {
            "scripts/appfw framework generate --check --json".to_string()
        } else {
            "scripts/appfw product generate --check --json".to_string()
        },
    }
}

fn handoff_verification(roots: &CommandRoots) -> Result<Vec<HandoffVerification>, String> {
    let mut items = verification_artifacts(&roots.app_root, &roots.report_root)?;
    let framework_scope = normalize_path(&roots.app_root) == normalize_path(&roots.framework_root);
    if framework_scope {
        push_verification_path(
            &roots.app_root,
            &mut items,
            "PDS component check",
            &roots.app_root.join("target/appfw/pds-component-check.json"),
            |json| json.get("ok").and_then(Value::as_bool),
            |json| {
                let version = json
                    .pointer("/design_system/version")
                    .and_then(Value::as_str)
                    .unwrap_or("unknown");
                let required_components = json
                    .pointer("/design_system/status/required_components")
                    .and_then(Value::as_u64)
                    .unwrap_or_default();
                let source_hash = json
                    .pointer("/design_system/source_sha256")
                    .and_then(Value::as_str)
                    .unwrap_or("unknown source hash");
                let token_hash = json
                    .pointer("/design_system/token_sha256")
                    .and_then(Value::as_str)
                    .unwrap_or("unknown token hash");
                let interactive_catalog_hash = json
                    .pointer("/interactive_catalog/sha256")
                    .and_then(Value::as_str)
                    .unwrap_or("unknown interactive catalog hash");
                let interactive_catalog_examples = json
                    .pointer("/interactive_catalog/component_example_count")
                    .and_then(Value::as_u64)
                    .unwrap_or_default();
                let interactive_catalog_neutral = json
                    .pointer("/interactive_catalog/product_neutral")
                    .and_then(Value::as_bool)
                    .unwrap_or(false);
                format!(
                    "PDS components {version}; {required_components} required components; source {source_hash}; tokens {token_hash}; interactive catalog {interactive_catalog_examples} live examples; interactive neutral {interactive_catalog_neutral}; catalog {interactive_catalog_hash}"
                )
            },
        )?;
        push_verification_path(
            &roots.app_root,
            &mut items,
            "Wave 2 readiness",
            &roots.app_root.join("target/appfw/wave2-readiness.json"),
            |json| json.get("ok").and_then(Value::as_bool),
            |json| {
                let release_ready = json
                    .get("release_ready")
                    .and_then(Value::as_bool)
                    .unwrap_or(false);
                let status = json
                    .pointer("/summary/status")
                    .and_then(Value::as_str)
                    .unwrap_or("unknown");
                let local_proven = json
                    .pointer("/summary/local_proven_count")
                    .and_then(Value::as_u64)
                    .unwrap_or_default();
                let local_lane_count = json
                    .pointer("/summary/local_lane_count")
                    .and_then(Value::as_u64)
                    .unwrap_or_default();
                let remaining_external_gate_count = json
                    .pointer("/summary/remaining_external_gate_count")
                    .and_then(Value::as_u64)
                    .unwrap_or_default();
                let local_preflight_proven = json
                    .pointer("/summary/local_preflight_proven_count")
                    .and_then(Value::as_u64)
                    .unwrap_or_default();
                let local_preflight_lane_count = json
                    .pointer("/summary/local_preflight_lane_count")
                    .and_then(Value::as_u64)
                    .unwrap_or_default();
                format!(
                    "Wave 2 {status}; local evidence {local_proven}/{local_lane_count}; local preflight {local_preflight_proven}/{local_preflight_lane_count}; release_ready:{release_ready}; remaining external gates:{remaining_external_gate_count}"
                )
            },
        )?;
    }
    Ok(items)
}

fn product_upgrade_verification(roots: &CommandRoots) -> Result<Vec<HandoffVerification>, String> {
    verification_artifacts(&roots.app_root, &roots.report_root)
}

fn verification_artifacts(
    app_root: &Path,
    report_root: &Path,
) -> Result<Vec<HandoffVerification>, String> {
    let mut items = Vec::new();
    push_verification_path(
        app_root,
        &mut items,
        "validation report",
        &report_root.join("validation.json"),
        |json| {
            json.get("valid")
                .or_else(|| json.get("ok"))
                .and_then(Value::as_bool)
        },
        |json| {
            json.get("errors")
                .and_then(Value::as_array)
                .map(|errors| format!("{} validation errors", errors.len()))
                .unwrap_or_else(|| "validation report parsed".to_string())
        },
    )?;
    push_verification_path(
        app_root,
        &mut items,
        "artifact manifest",
        &report_root.join("artifacts.json"),
        |json| json.as_array().map(|_| true),
        |json| {
            json.as_array()
                .map(|items| format!("{} generated artifacts recorded", items.len()))
                .unwrap_or_else(|| "artifact manifest is not an array".to_string())
        },
    )?;
    push_verification_path(
        app_root,
        &mut items,
        "config contract",
        &report_root.join("config_contract.json"),
        |json| json.get("shapes").and_then(Value::as_object).map(|_| true),
        |json| {
            json.get("shapes")
                .and_then(Value::as_object)
                .map(|shapes| format!("{} config shapes recorded", shapes.len()))
                .unwrap_or_else(|| "config contract did not include shapes".to_string())
        },
    )?;
    push_verification_path(
        app_root,
        &mut items,
        "app topology",
        &report_root.join("app_topology.json"),
        |json| {
            json.get("topology")
                .and_then(Value::as_object)
                .map(|_| true)
        },
        |json| {
            let schema_count = json
                .pointer("/topology/schemas")
                .and_then(Value::as_array)
                .map(Vec::len)
                .unwrap_or_default();
            let data_source_count = json
                .pointer("/topology/data_sources")
                .and_then(Value::as_array)
                .map(Vec::len)
                .unwrap_or_default();
            format!("{schema_count} schemas and {data_source_count} data sources reported")
        },
    )?;
    push_verification_path(
        app_root,
        &mut items,
        "dev infra",
        &report_root.join("dev_infra.json"),
        |json| json.get("services").and_then(Value::as_array).map(|_| true),
        |json| {
            json.get("services")
                .and_then(Value::as_array)
                .map(|services| {
                    format!("{} compose services derived from topology", services.len())
                })
                .unwrap_or_else(|| "dev infra report did not include services".to_string())
        },
    )?;
    push_verification_path(
        app_root,
        &mut items,
        "performance recommendations",
        &report_root.join("performance_recommendations.json"),
        |json| json.get("schemas").and_then(Value::as_array).map(|_| true),
        |json| {
            json.get("schemas")
                .and_then(Value::as_array)
                .map(|schemas| format!("{} schemas analyzed", schemas.len()))
                .unwrap_or_else(|| {
                    "performance recommendation report did not include schemas".to_string()
                })
        },
    )?;
    push_verification_path(
        app_root,
        &mut items,
        "release check",
        &app_root.join("target/appfw/release-check.json"),
        |json| json.get("ok").and_then(Value::as_bool),
        |_| "release gate report parsed".to_string(),
    )?;
    push_verification_path(
        app_root,
        &mut items,
        "ops certification",
        &app_root.join("target/appfw/ops-certification.json"),
        |json| json.get("ok").and_then(Value::as_bool),
        |json| {
            let check_count = json
                .get("checks")
                .and_then(Value::as_array)
                .map(Vec::len)
                .unwrap_or_default();
            let live_required = json
                .get("live_evidence_required")
                .and_then(Value::as_bool)
                .unwrap_or(false);
            format!("{check_count} ops checks reported; live evidence required: {live_required}")
        },
    )?;
    let provider_path = if app_root.join("target/appfw/provider-parity.json").exists() {
        app_root.join("target/appfw/provider-parity.json")
    } else {
        app_root.join("api_tests/target/provider-parity.json")
    };
    push_verification_path(
        app_root,
        &mut items,
        "provider certification",
        &provider_path,
        |json| json.get("ok").and_then(Value::as_bool),
        |json| {
            json.get("providers")
                .and_then(Value::as_array)
                .map(|providers| format!("{} providers reported", providers.len()))
                .unwrap_or_else(|| "provider certification report parsed".to_string())
        },
    )?;
    push_file_verification_path(
        app_root,
        &mut items,
        "framework lock",
        &app_root.join("appfw.lock"),
        "framework provenance lock exists",
    )?;
    Ok(items)
}

fn path_label(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .map(|relative| relative.display().to_string())
        .unwrap_or_else(|_| path.display().to_string())
}

fn push_file_verification_path(
    app_root: &Path,
    items: &mut Vec<HandoffVerification>,
    name: &str,
    path: &Path,
    detail: &str,
) -> Result<(), String> {
    items.push(HandoffVerification {
        name: name.to_string(),
        path: path_label(app_root, path),
        exists: path.exists(),
        ok: if path.exists() { Some(true) } else { None },
        detail: if path.exists() {
            detail.to_string()
        } else {
            "not present in this checkout; run the matching CLI command when needed".to_string()
        },
        sha256: hash_file_optional(path)?,
    });
    Ok(())
}

fn push_verification_path(
    app_root: &Path,
    items: &mut Vec<HandoffVerification>,
    name: &str,
    path: &Path,
    ok_from_json: impl Fn(&Value) -> Option<bool>,
    detail_from_json: impl Fn(&Value) -> String,
) -> Result<(), String> {
    let display_path = path_label(app_root, path);
    let sha256 = hash_file_optional(path)?;
    if !path.exists() {
        items.push(HandoffVerification {
            name: name.to_string(),
            path: display_path,
            exists: false,
            ok: None,
            detail: "not present in this checkout; run the matching CLI command when needed"
                .to_string(),
            sha256,
        });
        return Ok(());
    }

    let contents = fs::read_to_string(path).map_err(|err| {
        format!(
            "failed to read verification artifact {}: {err}",
            path.display()
        )
    })?;
    match serde_json::from_str::<Value>(&contents) {
        Ok(json) => items.push(HandoffVerification {
            name: name.to_string(),
            path: display_path,
            exists: true,
            ok: ok_from_json(&json),
            detail: detail_from_json(&json),
            sha256,
        }),
        Err(err) => items.push(HandoffVerification {
            name: name.to_string(),
            path: display_path,
            exists: true,
            ok: Some(false),
            detail: format!("artifact is not valid JSON: {err}"),
            sha256,
        }),
    }
    Ok(())
}

fn git_status(repo_root: &Path) -> Result<Vec<GitChange>, String> {
    let Some(git_root) = git_output(repo_root, &["rev-parse", "--show-toplevel"]) else {
        return Ok(Vec::new());
    };
    let git_root = normalize_path(&PathBuf::from(git_root));
    let app_prefix = repo_root
        .strip_prefix(&git_root)
        .ok()
        .map(path_to_slash)
        .filter(|prefix| !prefix.is_empty());

    let output = Command::new("git")
        .current_dir(repo_root)
        .args(["status", "--porcelain", "--untracked-files=all"])
        .output()
        .map_err(|err| format!("failed to run git status: {err}"))?;
    if !output.status.success() {
        return Err(format!(
            "git status failed: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }

    let mut changes = Vec::new();
    for line in String::from_utf8_lossy(&output.stdout).lines() {
        if let Some(change) = parse_git_status_line(line, app_prefix.as_deref()) {
            changes.push(change);
        }
    }
    Ok(changes)
}

fn parse_git_status_line(line: &str, app_prefix: Option<&str>) -> Option<GitChange> {
    if line.len() < 4 {
        return None;
    }
    let raw_status = &line[..2];
    let status = raw_status.trim().to_string();
    let mut path = line[3..].trim().trim_matches('"').to_string();
    if let Some((_, to)) = path.split_once(" -> ") {
        path = to.trim().trim_matches('"').to_string();
    }
    let path = app_relative_git_path(&path, app_prefix)?;
    let mut chars = raw_status.chars();
    let index = chars.next().unwrap_or(' ');
    let worktree = chars.next().unwrap_or(' ');
    Some(GitChange {
        status,
        change_kind: git_change_kind(raw_status).to_string(),
        index_status: git_status_slot_label(index).to_string(),
        worktree_status: git_status_slot_label(worktree).to_string(),
        path,
    })
}

fn git_branch_diff(repo_root: &Path) -> Result<Vec<GitChange>, String> {
    let Some(git_root) = git_output(repo_root, &["rev-parse", "--show-toplevel"]) else {
        return Ok(Vec::new());
    };
    let git_root = normalize_path(&PathBuf::from(git_root));
    let app_prefix = repo_root
        .strip_prefix(&git_root)
        .ok()
        .map(path_to_slash)
        .filter(|prefix| !prefix.is_empty());
    let Some(merge_base) = git_output(repo_root, &["merge-base", "origin/main", "HEAD"]) else {
        return Ok(Vec::new());
    };

    let output = Command::new("git")
        .current_dir(repo_root)
        .args([
            "diff",
            "--name-status",
            "--find-renames",
            merge_base.as_str(),
            "HEAD",
        ])
        .output()
        .map_err(|err| format!("failed to run git diff --name-status: {err}"))?;
    if !output.status.success() {
        return Ok(Vec::new());
    }

    let mut changes = Vec::new();
    for line in String::from_utf8_lossy(&output.stdout).lines() {
        if let Some(change) = parse_git_diff_name_status_line(line, app_prefix.as_deref()) {
            changes.push(change);
        }
    }
    Ok(changes)
}

fn parse_git_diff_name_status_line(line: &str, app_prefix: Option<&str>) -> Option<GitChange> {
    let parts = line.split('\t').collect::<Vec<_>>();
    if parts.len() < 2 {
        return None;
    }
    let raw_status = parts[0].trim();
    let status = raw_status.chars().next()?.to_string();
    let path_index = if matches!(status.as_str(), "R" | "C") && parts.len() >= 3 {
        2
    } else {
        1
    };
    let path = parts.get(path_index)?.trim().trim_matches('"');
    let path = app_relative_git_path(path, app_prefix)?;
    Some(GitChange {
        status: status.clone(),
        change_kind: git_change_kind(&status).to_string(),
        index_status: "branch_diff".to_string(),
        worktree_status: "none".to_string(),
        path,
    })
}

fn git_change_kind(status: &str) -> &'static str {
    if status.contains('?') {
        "untracked"
    } else if status.contains('U') || matches!(status, "AA" | "DD") {
        "unmerged"
    } else if status.starts_with('R') {
        "renamed"
    } else if status.starts_with('C') {
        "copied"
    } else if status.contains('D') {
        "deleted"
    } else if status.contains('A') {
        "added"
    } else if status.contains('T') {
        "type_changed"
    } else if status.contains('M') {
        "modified"
    } else {
        "unknown"
    }
}

fn git_status_slot_label(status: char) -> &'static str {
    match status {
        ' ' => "none",
        '?' => "untracked",
        'M' => "modified",
        'A' => "added",
        'D' => "deleted",
        'R' => "renamed",
        'C' => "copied",
        'U' => "unmerged",
        'T' => "type_changed",
        _ => "unknown",
    }
}

fn app_relative_git_path(path: &str, app_prefix: Option<&str>) -> Option<String> {
    let path = path.replace('\\', "/");
    let Some(prefix) = app_prefix else {
        return Some(path);
    };
    let prefix_with_slash = format!("{prefix}/");
    path.strip_prefix(&prefix_with_slash).map(str::to_string)
}

fn path_to_slash(path: &Path) -> String {
    path.components()
        .filter_map(|component| match component {
            Component::Normal(value) => value.to_str(),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("/")
}

fn handoff_text(report: &HandoffReport) -> String {
    let mut out = format!(
        "Agent handoff\nRepo: {}\nApp: {}\nGit: {} @ {}\nDirty: {}\nReports: {}\nArtifact: {}\nChanged surfaces: {}",
        report.repo_root,
        report.product.name,
        report.git.branch,
        report.git.sha,
        report.git.dirty,
        report.roots.report_root,
        report.artifact_path,
        report.changed_surfaces.len()
    );
    for surface in &report.changed_surfaces {
        out.push_str(&format!(
            "\n  {} {} {} [{} / {}]",
            surface.status,
            surface.change_kind,
            surface.path,
            surface.ownership,
            surface.classification
        ));
    }
    out.push_str(&format!(
        "\nDrift: {} generated, {} framework, {} human/app, {} docs, {} unknown",
        report.drift.generated_changed,
        report.drift.framework_source_changed,
        report.drift.human_owned_changed,
        report.drift.documentation_changed,
        report.drift.unknown_changed
    ));
    out.push_str("\nVerification artifacts:");
    for item in &report.verification {
        out.push_str(&format!(
            "\n  {} {} ({})",
            match item.ok {
                Some(true) => "ok",
                Some(false) => "fail",
                None if item.exists => "seen",
                None => "missing",
            },
            item.name,
            item.path
        ));
    }
    out.push_str(&format!(
        "\nRecommended:\n  {}",
        report.recommended_commands.join("\n  ")
    ));
    out
}

#[derive(Clone, Debug, Serialize)]
struct ProductIdentity {
    name: String,
    display_name: Option<String>,
    manifest_path: String,
    manifest_present: bool,
    topology_report_path: String,
    topology_report_present: bool,
}

#[derive(Clone, Debug, Serialize)]
struct LockReport {
    command: &'static str,
    ok: bool,
    product: ProductIdentity,
    roots: CommandRootReport,
    lock_path: String,
    status: String,
    checks: Vec<LockCheck>,
    recommended_commands: Vec<String>,
}

#[derive(Clone, Debug, Serialize)]
struct LockCheck {
    name: String,
    ok: bool,
    locked: Option<String>,
    current: Option<String>,
    detail: String,
}

#[derive(Clone, Debug, Serialize)]
struct LockData {
    version: u32,
    framework_version: String,
    framework_git_sha: String,
    framework_git_branch: String,
    generator_package_version: String,
    config_contract_hash: Option<String>,
    config_contract_source_hash: String,
    template_set_hash: String,
    golden_downstream_template_hash: Option<String>,
    workflow_cli_hash: String,
    provider_capability_hash: String,
    artifact_manifest_identity_schema: String,
    artifact_manifest_hash: Option<String>,
    generated_ownership_doc_hash: Option<String>,
    last_generated_at: String,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ArtifactManifestRecordForIdentity {
    path: String,
    ownership: String,
    overwrite: String,
    action: String,
    content_sha256: String,
    #[serde(default)]
    source_sha256: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
struct ArtifactManifestIdentity {
    schema: &'static str,
    records: Vec<ArtifactManifestIdentityRecord>,
}

#[derive(Clone, Debug, Serialize)]
struct ArtifactManifestIdentityRecord {
    root: String,
    path: String,
    ownership: String,
    overwrite: String,
    content_sha256: String,
    source_sha256: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
struct ProductUpgradeReport {
    command: &'static str,
    ok: bool,
    generated_at: String,
    product: ProductIdentity,
    roots: CommandRootReport,
    lock_path: String,
    report_path: String,
    status: String,
    locked: Option<BTreeMap<String, String>>,
    checks: Vec<LockCheck>,
    changed_surfaces: Vec<HandoffSurface>,
    drift: HandoffDrift,
    verification: Vec<HandoffVerification>,
    recommended_commands: Vec<String>,
}

fn lock_command(roots: &CommandRoots, json_output: bool, args: &[String]) -> Result<(), String> {
    let write = args.iter().any(|arg| arg == "--write");
    if !write {
        return Err("lock currently supports --write".to_string());
    }
    let lock = build_lock(roots)?;
    let path = roots.app_root.join("appfw.lock");
    write_product_lock(&roots.app_root, &path, &lock_to_toml(&lock))?;
    let report = LockReport {
        command: "lock",
        ok: true,
        product: product_identity(roots),
        roots: roots.report(),
        lock_path: path.display().to_string(),
        status: "written".to_string(),
        checks: Vec::new(),
        recommended_commands: vec!["scripts/appfw product upgrade --json".to_string()],
    };
    emit(json_output, &report, || {
        format!(
            "Wrote {}\nNext: {}",
            report.lock_path,
            report.recommended_commands.join("\n      ")
        )
    })
}

fn upgrade_command(roots: &CommandRoots, json_output: bool, args: &[String]) -> Result<(), String> {
    for arg in args {
        if arg != "--check" {
            return Err(format!("unknown upgrade option: {arg}"));
        }
    }
    let report = product_upgrade_check(roots)?;
    write_product_upgrade_report(&roots.app_root, &report)?;
    let ok = report.ok;
    emit(json_output, &report, || upgrade_text(&report))?;
    if ok {
        Ok(())
    } else {
        Err("product upgrade check found drift or a missing appfw.lock".to_string())
    }
}

fn product_upgrade_check(roots: &CommandRoots) -> Result<ProductUpgradeReport, String> {
    let lock_path = roots.app_root.join("appfw.lock");
    let product = product_identity(roots);
    let changed_surfaces = build_changed_surfaces(&roots.app_root)?;
    let drift = handoff_drift(&changed_surfaces, false);
    let verification = product_upgrade_verification(roots)?;
    let report_path = roots.app_root.join("target/appfw/product-upgrade.json");
    if matches!(
        fs::symlink_metadata(&lock_path),
        Err(ref err) if err.kind() == io::ErrorKind::NotFound
    ) {
        return Ok(ProductUpgradeReport {
            command: "upgrade",
            ok: false,
            generated_at: Utc::now().to_rfc3339(),
            product,
            roots: roots.report(),
            lock_path: lock_path.display().to_string(),
            report_path: report_path.display().to_string(),
            status: "missing_lock".to_string(),
            locked: None,
            checks: Vec::new(),
            changed_surfaces,
            drift,
            verification,
            recommended_commands: product_upgrade_commands("missing_lock", &[]),
        });
    }
    let locked = parse_lock(&roots.app_root, &lock_path)?;
    let current = build_lock(roots)?;
    let mut checks = Vec::new();
    compare_lock_field(
        &mut checks,
        "lock_version",
        locked.get("version").cloned(),
        Some(current.version.to_string()),
        "Lock schema version. Version 1 locks require an explicit regenerate-and-refresh migration to portable artifact identity.",
    );
    compare_lock_field(
        &mut checks,
        "artifact_manifest_identity_schema",
        locked.get("artifact_manifest_identity_schema").cloned(),
        Some(current.artifact_manifest_identity_schema.clone()),
        "Checkout-independent generated artifact identity schema.",
    );
    compare_git_field(
        &mut checks,
        locked.get("framework_git_sha").cloned(),
        current.framework_git_sha.clone(),
    );
    compare_lock_field(
        &mut checks,
        "config_contract_hash",
        locked_optional(&locked, "config_contract_hash"),
        current.config_contract_hash.clone(),
        "Generated config contract JSON hash.",
    );
    compare_lock_field(
        &mut checks,
        "config_contract_source_hash",
        locked.get("config_contract_source_hash").cloned(),
        Some(current.config_contract_source_hash.clone()),
        "Config contract emitter source hash.",
    );
    compare_lock_field(
        &mut checks,
        "template_set_hash",
        locked.get("template_set_hash").cloned(),
        Some(current.template_set_hash.clone()),
        "Generator template set hash.",
    );
    compare_lock_field(
        &mut checks,
        "golden_downstream_template_hash",
        locked_optional(&locked, "golden_downstream_template_hash"),
        current.golden_downstream_template_hash.clone(),
        "Golden downstream app template hash.",
    );
    compare_lock_field(
        &mut checks,
        "workflow_cli_hash",
        locked.get("workflow_cli_hash").cloned(),
        Some(current.workflow_cli_hash.clone()),
        "Agent-facing workflow CLI and introspection command hash.",
    );
    compare_lock_field(
        &mut checks,
        "provider_capability_hash",
        locked.get("provider_capability_hash").cloned(),
        Some(current.provider_capability_hash.clone()),
        "Provider certification source hash.",
    );
    compare_lock_field(
        &mut checks,
        "artifact_manifest_hash",
        locked_optional(&locked, "artifact_manifest_hash"),
        current.artifact_manifest_hash.clone(),
        "Generated artifact manifest hash.",
    );
    let ok = checks.iter().all(|check| check.ok);
    let migration_required = checks.iter().any(|check| {
        !check.ok
            && matches!(
                check.name.as_str(),
                "lock_version" | "artifact_manifest_identity_schema"
            )
    });
    let status = if ok {
        "current"
    } else if migration_required {
        "lock_migration_required"
    } else {
        "drift_detected"
    };
    let recommended_commands = product_upgrade_commands(status, &checks);

    Ok(ProductUpgradeReport {
        command: "upgrade",
        ok,
        generated_at: Utc::now().to_rfc3339(),
        product,
        roots: roots.report(),
        lock_path: lock_path.display().to_string(),
        report_path: report_path.display().to_string(),
        status: status.to_string(),
        locked: Some(locked.into_iter().collect()),
        checks,
        changed_surfaces,
        drift,
        verification,
        recommended_commands,
    })
}

fn write_product_upgrade_report(
    app_root: &Path,
    report: &ProductUpgradeReport,
) -> Result<(), String> {
    let path = Path::new(&report.report_path);
    let content = serde_json::to_string_pretty(report)
        .map_err(|err| format!("failed to serialize product upgrade report: {err}"))?
        + "\n";
    write_contained_evidence_file(app_root, path, content.as_bytes(), "product upgrade report")
}

fn product_upgrade_commands(status: &str, checks: &[LockCheck]) -> Vec<String> {
    let mut commands = vec![
        "scripts/appfw product validate --json".to_string(),
        "scripts/appfw product topology --json".to_string(),
    ];
    if status != "current" || checks.iter().any(|check| !check.ok) {
        commands.extend([
            "scripts/appfw product generate".to_string(),
            "scripts/appfw product generate --check --json".to_string(),
            "scripts/appfw product test".to_string(),
            "scripts/appfw product api-test".to_string(),
            "scripts/appfw product lock --write".to_string(),
            "scripts/appfw product upgrade --json".to_string(),
        ]);
    } else {
        commands.push("scripts/appfw product test".to_string());
    }
    commands
}

fn product_identity(roots: &CommandRoots) -> ProductIdentity {
    let manifest_path = roots.app_root.join(".appfw/manifest.yaml");
    let topology_report_path = roots.report_root.join("app_topology.json");
    let default_name = roots
        .app_root
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("app")
        .to_string();
    let (name, display_name) =
        product_identity_from_topology(&topology_report_path).unwrap_or((default_name, None));

    ProductIdentity {
        name,
        display_name,
        manifest_path: manifest_path.display().to_string(),
        manifest_present: manifest_path.exists(),
        topology_report_path: topology_report_path.display().to_string(),
        topology_report_present: topology_report_path.exists(),
    }
}

fn product_identity_from_topology(path: &Path) -> Option<(String, Option<String>)> {
    let contents = fs::read_to_string(path).ok()?;
    let json = serde_json::from_str::<Value>(&contents).ok()?;
    let app = json.get("app")?.as_object()?;
    let name = app.get("name")?.as_str()?.to_string();
    let display_name = app
        .get("display_name")
        .and_then(Value::as_str)
        .map(str::to_string);
    Some((name, display_name))
}

fn compare_git_field(checks: &mut Vec<LockCheck>, locked: Option<String>, current: String) {
    let current = if current == "unknown" {
        None
    } else {
        Some(current)
    };
    let ok = current.is_none() || locked == current;
    checks.push(LockCheck {
        name: "framework_git_sha".to_string(),
        ok,
        locked,
        current,
        detail: if ok {
            "Framework commit that shaped this app. Non-git app copies rely on content hashes."
                .to_string()
        } else {
            "Framework commit that shaped this app.".to_string()
        },
    });
}

fn compare_lock_field(
    checks: &mut Vec<LockCheck>,
    name: &str,
    locked: Option<String>,
    current: Option<String>,
    detail: &str,
) {
    checks.push(LockCheck {
        name: name.to_string(),
        ok: locked == current,
        locked,
        current,
        detail: detail.to_string(),
    });
}

fn locked_optional(values: &HashMap<String, String>, key: &str) -> Option<String> {
    values.get(key).filter(|value| !value.is_empty()).cloned()
}

fn upgrade_text(report: &ProductUpgradeReport) -> String {
    let mut out = format!(
        "Product upgrade check: {}\nApp: {}\nLock: {}\nReport: {}",
        report.status, report.product.name, report.lock_path, report.report_path
    );
    for check in &report.checks {
        out.push_str(&format!(
            "\n  {} {}",
            if check.ok { "ok" } else { "drift" },
            check.name
        ));
    }
    out.push_str(&format!(
        "\nChanged surfaces: {} total, {} generated, {} app-owned, {} framework, {} docs",
        report.drift.total_changed,
        report.drift.generated_changed,
        report.drift.human_owned_changed,
        report.drift.framework_source_changed,
        report.drift.documentation_changed
    ));
    out.push_str(&format!(
        "\nRecommended:\n  {}",
        report.recommended_commands.join("\n  ")
    ));
    out
}

fn build_lock(roots: &CommandRoots) -> Result<LockData, String> {
    canonical_artifact_identity_root(&roots.app_root, "configured app root")?;
    validate_configured_evidence_root(&roots.report_root, "configured report root")?;
    let config_contract_hash = hash_contained_report_file_optional(
        &roots.report_root,
        "config_contract.json",
        "config contract report",
    )?;
    Ok(LockData {
        version: APPFW_LOCK_VERSION,
        framework_version: package_version(&roots.generator_root.join("Cargo.toml"))?,
        framework_git_sha: git_output(&roots.framework_root, &["rev-parse", "HEAD"])
            .unwrap_or_else(|| "unknown".to_string()),
        framework_git_branch: git_output(
            &roots.framework_root,
            &["rev-parse", "--abbrev-ref", "HEAD"],
        )
        .unwrap_or_else(|| "unknown".to_string()),
        generator_package_version: package_version(&roots.generator_root.join("Cargo.toml"))?,
        config_contract_hash,
        config_contract_source_hash: hash_file(
            &roots.generator_root.join("src/config_contract.rs"),
        )?,
        template_set_hash: hash_tree(&roots.templates_root)?,
        golden_downstream_template_hash: hash_tree_optional(
            &roots.generator_root.join("_golden/downstream_apps"),
        )?,
        workflow_cli_hash: hash_named_paths(&[
            ("scripts".to_string(), roots.app_root.join("scripts")),
            (
                "app_gen/src/bin/appfw.rs".to_string(),
                roots.generator_root.join("src/bin/appfw.rs"),
            ),
            (
                "app_gen/src/bin/appfw_introspect.rs".to_string(),
                roots.generator_root.join("src/bin/appfw_introspect.rs"),
            ),
        ])?,
        provider_capability_hash: hash_named_paths(&[
            (
                "appfw_runtime/src/provider_capabilities.rs".to_string(),
                roots
                    .framework_root
                    .join("appfw_runtime/src/provider_capabilities.rs"),
            ),
            (
                "appfw_runtime/src/provider_certification.rs".to_string(),
                roots
                    .framework_root
                    .join("appfw_runtime/src/provider_certification.rs"),
            ),
            (
                "appfw_runtime/src/provider_contract_types.rs".to_string(),
                roots
                    .framework_root
                    .join("appfw_runtime/src/provider_contract_types.rs"),
            ),
            (
                "appfw_cli/src/bin/provider_certification_export.rs".to_string(),
                roots
                    .framework_root
                    .join("appfw_cli/src/bin/provider_certification_export.rs"),
            ),
        ])?,
        artifact_manifest_identity_schema: ARTIFACT_MANIFEST_IDENTITY_SCHEMA.to_string(),
        artifact_manifest_hash: artifact_manifest_identity_hash_optional_in_report_root(
            &roots.app_root,
            &roots.config_root,
            &roots.report_root,
            &roots.report_root.join("artifacts.json"),
        )?,
        generated_ownership_doc_hash: hash_file_optional(
            &roots
                .framework_root
                .join("docs/start/generated-ownership.md"),
        )?,
        last_generated_at: Utc::now().to_rfc3339(),
    })
}

fn package_version(cargo_toml: &Path) -> Result<String, String> {
    let contents = fs::read_to_string(cargo_toml)
        .map_err(|err| format!("failed to read {}: {err}", cargo_toml.display()))?;
    for line in contents.lines() {
        let line = line.trim();
        if let Some(value) = line.strip_prefix("version") {
            if let Some(value) = value.split('=').nth(1) {
                return Ok(unquote(value.trim()));
            }
        }
    }
    Err(format!(
        "could not find package version in {}",
        cargo_toml.display()
    ))
}

fn lock_to_toml(lock: &LockData) -> String {
    let mut out = String::new();
    out.push_str("# Generated by scripts/appfw product lock --write.\n");
    out.push_str("# Records the framework provenance used by this application checkout.\n");
    push_toml_number(&mut out, "version", lock.version);
    push_toml_string(&mut out, "framework_version", &lock.framework_version);
    push_toml_string(&mut out, "framework_git_sha", &lock.framework_git_sha);
    push_toml_string(&mut out, "framework_git_branch", &lock.framework_git_branch);
    push_toml_string(
        &mut out,
        "generator_package_version",
        &lock.generator_package_version,
    );
    push_toml_optional(&mut out, "config_contract_hash", &lock.config_contract_hash);
    push_toml_string(
        &mut out,
        "config_contract_source_hash",
        &lock.config_contract_source_hash,
    );
    push_toml_string(&mut out, "template_set_hash", &lock.template_set_hash);
    push_toml_optional(
        &mut out,
        "golden_downstream_template_hash",
        &lock.golden_downstream_template_hash,
    );
    push_toml_string(&mut out, "workflow_cli_hash", &lock.workflow_cli_hash);
    push_toml_string(
        &mut out,
        "provider_capability_hash",
        &lock.provider_capability_hash,
    );
    push_toml_string(
        &mut out,
        "artifact_manifest_identity_schema",
        &lock.artifact_manifest_identity_schema,
    );
    push_toml_optional(
        &mut out,
        "artifact_manifest_hash",
        &lock.artifact_manifest_hash,
    );
    push_toml_optional(
        &mut out,
        "generated_ownership_doc_hash",
        &lock.generated_ownership_doc_hash,
    );
    push_toml_string(&mut out, "last_generated_at", &lock.last_generated_at);
    out
}

fn push_toml_number(out: &mut String, key: &str, value: u32) {
    out.push_str(&format!("{key} = {value}\n"));
}

fn push_toml_string(out: &mut String, key: &str, value: &str) {
    out.push_str(&format!("{key} = \"{}\"\n", escape_toml(value)));
}

fn push_toml_optional(out: &mut String, key: &str, value: &Option<String>) {
    match value {
        Some(value) => push_toml_string(out, key, value),
        None => out.push_str(&format!("{key} = \"\"\n")),
    }
}

#[derive(Clone, Copy, Debug)]
enum ContainedEvidenceLeaf {
    ExistingRegularFile,
    MissingOrRegularFile,
}

fn contained_evidence_path(
    app_root: &Path,
    path: &Path,
    leaf: ContainedEvidenceLeaf,
) -> Result<PathBuf, String> {
    let root_metadata = fs::symlink_metadata(app_root).map_err(|err| {
        format!(
            "failed to inspect product root {} without following links: {err}",
            app_root.display()
        )
    })?;
    if root_metadata.file_type().is_symlink() || !root_metadata.file_type().is_dir() {
        return Err(format!(
            "product root must be a non-symlink directory: {}",
            app_root.display()
        ));
    }
    let canonical_root = fs::canonicalize(app_root).map_err(|err| {
        format!(
            "failed to canonicalize product root {}: {err}",
            app_root.display()
        )
    })?;
    if !lexical_evidence_root_matches_canonical(app_root, &canonical_root) {
        return Err(format!(
            "configured evidence root must not contain symlink ancestors: {}",
            app_root.display()
        ));
    }
    let relative = path
        .strip_prefix(app_root)
        .or_else(|_| path.strip_prefix(&canonical_root))
        .map_err(|_| {
            format!(
                "evidence path is outside product root {}: {}",
                app_root.display(),
                path.display()
            )
        })?;
    if relative.as_os_str().is_empty() {
        return Err("evidence path cannot be the product root itself".to_string());
    }

    let components = relative.components().collect::<Vec<_>>();
    let mut current = canonical_root;
    for (index, component) in components.iter().enumerate() {
        let Component::Normal(component) = component else {
            return Err(format!(
                "evidence path contains a noncanonical component: {}",
                path.display()
            ));
        };
        current.push(component);
        let is_leaf = index + 1 == components.len();
        match fs::symlink_metadata(&current) {
            Ok(metadata) => {
                if metadata.file_type().is_symlink() {
                    return Err(format!(
                        "evidence path contains a symlink: {}",
                        current.display()
                    ));
                }
                if !is_leaf && !metadata.file_type().is_dir() {
                    return Err(format!(
                        "evidence path contains a non-directory ancestor: {}",
                        current.display()
                    ));
                }
                if is_leaf && !metadata.file_type().is_file() {
                    return Err(format!(
                        "evidence path leaf is not a regular file: {}",
                        current.display()
                    ));
                }
            }
            Err(err) if err.kind() == io::ErrorKind::NotFound => {
                if matches!(leaf, ContainedEvidenceLeaf::ExistingRegularFile) {
                    return Err(format!(
                        "required evidence path is missing: {}",
                        current.display()
                    ));
                }
                for remaining in components.iter().skip(index + 1) {
                    let Component::Normal(remaining) = remaining else {
                        return Err(format!(
                            "evidence path contains a noncanonical component: {}",
                            path.display()
                        ));
                    };
                    current.push(remaining);
                }
                return Ok(current);
            }
            Err(err) => {
                return Err(format!(
                    "failed to inspect evidence path {}: {err}",
                    current.display()
                ))
            }
        }
    }
    Ok(current)
}

fn lexical_evidence_root_matches_canonical(lexical: &Path, canonical: &Path) -> bool {
    if lexical == canonical {
        return true;
    }
    #[cfg(target_os = "macos")]
    for (alias, resolved) in [
        (Path::new("/var"), Path::new("/private/var")),
        (Path::new("/tmp"), Path::new("/private/tmp")),
        (Path::new("/etc"), Path::new("/private/etc")),
    ] {
        if let Ok(suffix) = lexical.strip_prefix(alias) {
            if resolved.join(suffix) == canonical {
                return true;
            }
        }
    }
    false
}

fn write_product_lock(app_root: &Path, path: &Path, content: &str) -> Result<(), String> {
    write_contained_evidence_file(app_root, path, content.as_bytes(), "appfw.lock")
}

fn write_contained_evidence_file(
    app_root: &Path,
    path: &Path,
    content: &[u8],
    label: &str,
) -> Result<(), String> {
    let output_path = ensure_contained_evidence_parent_dirs(app_root, path, label)?;
    let initial = match fs::symlink_metadata(&output_path) {
        Ok(metadata) => Some(metadata),
        Err(err) if err.kind() == io::ErrorKind::NotFound => None,
        Err(err) => {
            return Err(format!(
                "failed to inspect {label} {} before writing: {err}",
                output_path.display()
            ))
        }
    };
    if let Some(initial) = initial.as_ref() {
        require_unaliased_output_leaf(initial, true, label, &output_path)?;
    }
    let mut options = fs::OpenOptions::new();
    options.write(true);
    if initial.is_none() {
        options.create_new(true);
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.custom_flags(libc::O_NOFOLLOW);
    }
    let mut file = options.open(&output_path).map_err(|err| {
        format!(
            "failed to open {label} without following its leaf {}: {err}",
            output_path.display()
        )
    })?;
    let opened = file.metadata().map_err(|err| {
        format!(
            "failed to inspect open {label} {}: {err}",
            output_path.display()
        )
    })?;
    require_unaliased_output_leaf(&opened, initial.is_some(), label, &output_path)?;
    if let Some(initial) = initial.as_ref() {
        if !same_bounded_file_object(initial, &opened) {
            return Err(format!(
                "{label} changed identity while opening: {}",
                output_path.display()
            ));
        }
    }
    let checked_path =
        contained_evidence_path(app_root, path, ContainedEvidenceLeaf::ExistingRegularFile)?;
    if checked_path != output_path {
        return Err(format!(
            "{label} changed containment while opening: {}",
            output_path.display()
        ));
    }
    let linked = fs::symlink_metadata(&output_path).map_err(|err| {
        format!(
            "failed to re-inspect {label} {}: {err}",
            output_path.display()
        )
    })?;
    require_unaliased_output_leaf(&linked, initial.is_some(), label, &output_path)?;
    if linked.file_type().is_symlink() || !same_bounded_file_object(&opened, &linked) {
        return Err(format!(
            "{label} changed type or identity while opening: {}",
            output_path.display()
        ));
    }
    file.set_len(0).map_err(|err| {
        format!(
            "failed to truncate {label} {} after identity verification: {err}",
            output_path.display()
        )
    })?;
    file.write_all(content)
        .map_err(|err| format!("failed to write {label} {}: {err}", output_path.display()))?;
    file.flush()
        .map_err(|err| format!("failed to flush {label} {}: {err}", output_path.display()))?;
    let final_opened = file.metadata().map_err(|err| {
        format!(
            "failed to re-inspect open {label} {}: {err}",
            output_path.display()
        )
    })?;
    let final_path =
        contained_evidence_path(app_root, path, ContainedEvidenceLeaf::ExistingRegularFile)?;
    if final_path != output_path {
        return Err(format!(
            "{label} changed containment while writing: {}",
            output_path.display()
        ));
    }
    let final_linked = fs::symlink_metadata(&output_path).map_err(|err| {
        format!(
            "failed to re-inspect {label} link {}: {err}",
            output_path.display()
        )
    })?;
    require_unaliased_output_leaf(&final_opened, initial.is_some(), label, &output_path)?;
    require_unaliased_output_leaf(&final_linked, initial.is_some(), label, &output_path)?;
    if final_linked.file_type().is_symlink()
        || !same_bounded_file_object(&opened, &final_opened)
        || !same_bounded_file_object(&opened, &final_linked)
        || final_opened.len() != content.len() as u64
    {
        return Err(format!(
            "{label} changed while writing: {}",
            output_path.display()
        ));
    }
    Ok(())
}

#[cfg(unix)]
fn require_unaliased_output_leaf(
    metadata: &fs::Metadata,
    _preexisting: bool,
    label: &str,
    path: &Path,
) -> Result<(), String> {
    use std::os::unix::fs::MetadataExt;

    if metadata.nlink() != 1 {
        return Err(format!(
            "{label} output must have exactly one hard link for contained mutation: {} has link count {}",
            path.display(),
            metadata.nlink()
        ));
    }
    Ok(())
}

#[cfg(not(unix))]
fn require_unaliased_output_leaf(
    _metadata: &fs::Metadata,
    preexisting: bool,
    label: &str,
    path: &Path,
) -> Result<(), String> {
    if preexisting {
        return Err(format!(
            "{label} existing output cannot be safely overwritten because this platform does not expose a stable hard-link count: {}",
            path.display()
        ));
    }
    Ok(())
}

fn ensure_contained_evidence_parent_dirs(
    app_root: &Path,
    path: &Path,
    label: &str,
) -> Result<PathBuf, String> {
    let output_path =
        contained_evidence_path(app_root, path, ContainedEvidenceLeaf::MissingOrRegularFile)?;
    let canonical_root = fs::canonicalize(app_root).map_err(|err| {
        format!(
            "failed to canonicalize product root {} while preparing {label}: {err}",
            app_root.display()
        )
    })?;
    let parent = output_path.parent().ok_or_else(|| {
        format!(
            "{label} output has no parent below product root: {}",
            output_path.display()
        )
    })?;
    let relative_parent = parent.strip_prefix(&canonical_root).map_err(|_| {
        format!(
            "{label} parent is outside product root {}: {}",
            app_root.display(),
            parent.display()
        )
    })?;
    let mut current = canonical_root;
    for component in relative_parent.components() {
        let Component::Normal(component) = component else {
            return Err(format!(
                "{label} parent contains a noncanonical component: {}",
                parent.display()
            ));
        };
        current.push(component);
        loop {
            match fs::symlink_metadata(&current) {
                Ok(metadata) => {
                    if metadata.file_type().is_symlink() || !metadata.file_type().is_dir() {
                        return Err(format!(
                            "{label} parent must be a non-symlink directory: {}",
                            current.display()
                        ));
                    }
                    break;
                }
                Err(err) if err.kind() == io::ErrorKind::NotFound => {
                    match fs::create_dir(&current) {
                        Ok(()) => {}
                        Err(create_err) if create_err.kind() == io::ErrorKind::AlreadyExists => {
                            continue
                        }
                        Err(create_err) => {
                            return Err(format!(
                                "failed to create {label} parent {}: {create_err}",
                                current.display()
                            ))
                        }
                    }
                }
                Err(err) => {
                    return Err(format!(
                        "failed to inspect {label} parent {} without following links: {err}",
                        current.display()
                    ))
                }
            }
        }
    }
    contained_evidence_path(app_root, path, ContainedEvidenceLeaf::MissingOrRegularFile)
}

fn parse_lock(app_root: &Path, path: &Path) -> Result<HashMap<String, String>, String> {
    let bytes =
        read_contained_bounded_regular_file(app_root, path, APPFW_LOCK_MAX_BYTES, "appfw.lock")?;
    let contents = String::from_utf8(bytes)
        .map_err(|err| format!("appfw.lock {} is not valid UTF-8: {err}", path.display()))?;
    let document = contents
        .parse::<Document>()
        .map_err(|err| format!("appfw.lock {} is malformed TOML: {err}", path.display()))?;
    const ALLOWED_FIELDS: &[&str] = &[
        "version",
        "framework_version",
        "framework_git_sha",
        "framework_git_branch",
        "generator_package_version",
        "config_contract_hash",
        "config_contract_source_hash",
        "template_set_hash",
        "golden_downstream_template_hash",
        "workflow_cli_hash",
        "provider_capability_hash",
        "artifact_manifest_identity_schema",
        "artifact_manifest_hash",
        "generated_ownership_doc_hash",
        "last_generated_at",
    ];
    let mut values = HashMap::new();
    for (key, item) in document.iter() {
        if !ALLOWED_FIELDS.contains(&key) {
            return Err(format!(
                "appfw.lock {} contains unknown field `{key}`",
                path.display()
            ));
        }
        let value = if key == "version" {
            let version = item.as_integer().ok_or_else(|| {
                format!(
                    "appfw.lock {} field `version` must be a non-negative integer",
                    path.display()
                )
            })?;
            u32::try_from(version)
                .map_err(|_| {
                    format!(
                        "appfw.lock {} field `version` is outside the supported u32 range",
                        path.display()
                    )
                })?
                .to_string()
        } else {
            item.as_str()
                .ok_or_else(|| {
                    format!(
                        "appfw.lock {} field `{key}` must be a string",
                        path.display()
                    )
                })?
                .to_string()
        };
        values.insert(key.to_string(), value);
    }
    Ok(values)
}

#[derive(Clone, Copy, Debug)]
struct NewProfile {
    name: &'static str,
    template_path: &'static str,
}

#[derive(Clone, Debug, Serialize)]
struct NewProfileInfo {
    name: String,
    description: String,
    profile_path: String,
    template_path: String,
    overlay_path: String,
    verification: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    frontend_scaffold: Option<NewProfileFrontendScaffold>,
}

#[derive(Clone, Debug, Serialize)]
struct NewStarterMode {
    name: String,
    description: String,
    command: String,
    source_kinds: Vec<String>,
    required_options: Vec<String>,
    optional_options: Vec<String>,
    next_commands: Vec<String>,
    retained_artifacts: Vec<String>,
    verification: Vec<String>,
}

#[derive(Clone, Debug, Deserialize)]
struct NewProfileManifest {
    name: String,
    description: String,
    template: String,
    #[serde(default = "default_profile_overlay")]
    overlay: String,
    #[serde(default)]
    verification: Vec<String>,
    #[serde(default)]
    copy_framework_model_seeds: bool,
    #[serde(default)]
    frontend_scaffold: Option<NewProfileFrontendScaffold>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct NewProfileFrontendScaffold {
    root: String,
    scaffold_manifest: String,
    ownership_manifest: String,
    generated_contract: String,
    check_script: String,
    package_check: String,
    #[serde(default)]
    verification: Vec<String>,
    #[serde(default)]
    release_evidence: Vec<String>,
}

#[derive(Clone, Debug, Default)]
struct ProductIntakeArgs {
    source_kind: Option<String>,
    app_name: Option<String>,
    display_name: Option<String>,
    schema: Option<String>,
    provider: Option<String>,
    mcp_server: Option<String>,
    kafka_client: Option<String>,
    ui: Option<String>,
    poc_source: Option<String>,
    data_artifact: Option<String>,
    ui_artifact: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
struct ProductIntake {
    source_kind: String,
    app_name: String,
    display_name: String,
    schema: String,
    provider: String,
    data_source_name: String,
    mcp_server: bool,
    kafka_client: bool,
    ui: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    poc_source: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    data_artifact: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    ui_artifact: Option<String>,
}

fn product_intake_required_questions() -> Value {
    json!({
        "source_kind": {
            "flag": "--source-kind",
            "question": "What kind of source evidence is being converted?",
            "allowed": product_intake_source_kind_options(),
            "default": "poc"
        },
        "app_name": {
            "flag": "--app-name",
            "question": "What is the application name?",
            "allowed": "kebab-case or snake_case application identifier"
        },
        "display_name": {
            "flag": "--display-name",
            "question": "What display name should users see?",
            "allowed": "human-readable product name"
        },
        "schema": {
            "flag": "--schema",
            "question": "What product schema name should be created?",
            "allowed": "snake_case schema identifier"
        },
        "provider": {
            "flag": "--provider",
            "question": "Which backend provider is needed?",
            "allowed": product_intake_provider_options()
        },
        "mcp_server": {
            "flag": "--mcp",
            "question": "Is an MCP server needed?",
            "allowed": ["true", "false"]
        },
        "kafka_client": {
            "flag": "--kafka",
            "question": "Is a Kafka client needed?",
            "allowed": ["true", "false"]
        },
        "ui": {
            "flag": "--ui",
            "question": "Is a product UI needed?",
            "allowed": product_intake_ui_options()
        }
    })
}

fn product_intake_source_kind_options() -> Vec<&'static str> {
    vec!["poc", "legacy"]
}

fn product_intake_provider_options() -> Vec<&'static str> {
    vec!["PostgreSQL"]
}

fn product_intake_ui_options() -> Vec<&'static str> {
    vec!["none", "scaffold", "enterprise"]
}

fn new_option_value(args: &[String], idx: usize, option: &str) -> Result<String, String> {
    args.get(idx + 1)
        .filter(|value| !value.starts_with("--"))
        .cloned()
        .ok_or_else(|| format!("{option} requires a value"))
}

fn new_command(repo_root: &Path, json_output: bool, args: &[String]) -> Result<(), String> {
    let mut target: Option<String> = None;
    let mut source = "current".to_string();
    let mut profile = "crm-sample".to_string();
    let mut generate = false;
    let mut list_profiles = false;
    let mut intake_args = ProductIntakeArgs::default();
    let mut idx = 0;
    while idx < args.len() {
        match args[idx].as_str() {
            "--list-profiles" => {
                list_profiles = true;
                idx += 1;
            }
            "--from" => {
                source = new_option_value(args, idx, "--from")?;
                idx += 2;
            }
            "--profile" => {
                profile = new_option_value(args, idx, "--profile")?;
                idx += 2;
            }
            "--source-kind" | "--source" | "--intake-kind" => {
                intake_args.source_kind = Some(new_option_value(args, idx, args[idx].as_str())?);
                idx += 2;
            }
            "--legacy" => {
                intake_args.source_kind = Some("legacy".to_string());
                idx += 1;
            }
            "--app-name" => {
                intake_args.app_name = Some(new_option_value(args, idx, "--app-name")?);
                idx += 2;
            }
            "--display-name" => {
                intake_args.display_name = Some(new_option_value(args, idx, "--display-name")?);
                idx += 2;
            }
            "--schema" => {
                intake_args.schema = Some(new_option_value(args, idx, "--schema")?);
                idx += 2;
            }
            "--provider" | "--backend-provider" => {
                intake_args.provider = Some(new_option_value(args, idx, args[idx].as_str())?);
                idx += 2;
            }
            "--mcp" | "--mcp-server" => {
                intake_args.mcp_server = Some(new_option_value(args, idx, args[idx].as_str())?);
                idx += 2;
            }
            "--kafka" | "--kafka-client" => {
                intake_args.kafka_client = Some(new_option_value(args, idx, args[idx].as_str())?);
                idx += 2;
            }
            "--ui" | "--frontend" => {
                intake_args.ui = Some(new_option_value(args, idx, args[idx].as_str())?);
                idx += 2;
            }
            "--poc-source" => {
                intake_args.poc_source = Some(new_option_value(args, idx, "--poc-source")?);
                idx += 2;
            }
            "--legacy-source" | "--codebase" => {
                intake_args.source_kind = Some("legacy".to_string());
                intake_args.poc_source = Some(new_option_value(args, idx, args[idx].as_str())?);
                idx += 2;
            }
            "--data-artifact" => {
                intake_args.data_artifact = Some(new_option_value(args, idx, "--data-artifact")?);
                idx += 2;
            }
            "--ui-artifact" => {
                intake_args.ui_artifact = Some(new_option_value(args, idx, "--ui-artifact")?);
                idx += 2;
            }
            "--generate" => {
                generate = true;
                idx += 1;
            }
            value if target.is_none() => {
                target = Some(value.to_string());
                idx += 1;
            }
            value => return Err(format!("unknown appfw product new option: {value}")),
        }
    }

    if list_profiles {
        return emit_new_profiles(repo_root, json_output);
    }

    let target = target.ok_or_else(|| "new requires a target directory".to_string())?;
    if profile == "product-intake" {
        return product_intake_new_command(
            repo_root,
            json_output,
            &target,
            &source,
            generate,
            intake_args,
        );
    }

    let selected_profile = new_profile(&profile).ok_or_else(|| {
        format!(
            "unknown appfw product new profile `{profile}`. Run scripts/appfw product new --list-profiles"
        )
    })?;
    let profile_manifest = read_new_profile_manifest(repo_root, &selected_profile)?;
    let profile_manifest_path = new_profile_manifest_path(repo_root, selected_profile.name);
    if source != "current" {
        return Err(
            "split-root app generation does not copy framework refs; checkout the desired framework version and run appfw product new --from current"
                .to_string(),
        );
    }

    let target_path = absolute_path(&env::current_dir().map_err(|err| err.to_string())?, &target);
    if target_path.exists()
        && target_path
            .read_dir()
            .map_err(|err| err.to_string())?
            .next()
            .is_some()
    {
        return Err(format!(
            "target directory already exists and is not empty: {}",
            target_path.display()
        ));
    }
    fs::create_dir_all(&target_path)
        .map_err(|err| format!("failed to create {}: {err}", target_path.display()))?;

    let template_files = copy_product_template(repo_root, &target_path, &selected_profile)?;
    let mut profile_files = apply_new_profile(repo_root, &target_path, selected_profile.name)?;
    if profile_manifest.copy_framework_model_seeds {
        copy_framework_model_seeds(repo_root, &target_path, &mut profile_files)?;
    }
    rewrite_split_root_cargo_paths(&target_path, repo_root)?;
    rewrite_split_root_compose_paths(&target_path, repo_root)?;
    write_new_app_local_env(&target_path, repo_root)?;

    run_new_app_command(
        &target_path,
        repo_root,
        &["scripts/appfw", "product", "validate", "--json"],
    )?;
    if generate {
        run_new_app_command(
            &target_path,
            repo_root,
            &["scripts/appfw", "product", "generate"],
        )?;
    }

    let lock = build_lock(&CommandRoots::split_layout(&target_path, repo_root))?;
    write_product_lock(
        &target_path,
        &target_path.join("appfw.lock"),
        &lock_to_toml(&lock),
    )?;

    let mut next_commands = vec![
        format!("cd {}", target_path.display()),
        "scripts/appfw doctor".to_string(),
        "scripts/appfw product validate --json".to_string(),
        "scripts/appfw product boundary-check --json".to_string(),
        "scripts/appfw product generate".to_string(),
        "scripts/appfw product test --fast".to_string(),
    ];
    if let Some(frontend_scaffold) = &profile_manifest.frontend_scaffold {
        for command in &frontend_scaffold.verification {
            next_commands.push(format!("cd {} && {command}", frontend_scaffold.root));
        }
    }

    let report = json!({
        "command": "new",
        "ok": true,
        "target": target_path.display().to_string(),
        "source": source,
        "profile": profile,
        "profile_manifest": profile_manifest_path.display().to_string(),
        "template": selected_profile.template_path,
        "template_files": template_files,
        "profile_files": profile_files,
        "profile_verification": profile_manifest.verification,
        "profile_frontend_scaffold": profile_manifest.frontend_scaffold,
        "local_env": ".appfw/local.env",
        "validated": true,
        "generated": generate,
        "next_commands": next_commands
    });
    emit_value(json_output, &report, || {
        format!(
            "Created app at {}\nNext:\n  cd {}\n  scripts/appfw doctor\n  scripts/appfw product validate --json\n  scripts/appfw product boundary-check --json\n  scripts/appfw product generate\n  scripts/appfw product test --fast",
            target_path.display(),
            target_path.display(),
        )
    })
}

fn product_intake_new_command(
    repo_root: &Path,
    json_output: bool,
    target: &str,
    source: &str,
    generate: bool,
    intake_args: ProductIntakeArgs,
) -> Result<(), String> {
    if source != "current" {
        return Err(
            "product-intake app generation does not copy framework refs; checkout the desired framework version and run appfw product new --from current"
                .to_string(),
        );
    }
    if generate {
        return Err(
            "product-intake records PoC topology first and does not support --generate until the AI harness creates the product entity model"
                .to_string(),
        );
    }

    let intake = complete_product_intake(intake_args, json_output)?;
    let target_path = absolute_path(&env::current_dir().map_err(|err| err.to_string())?, target);
    if target_path.exists()
        && target_path
            .read_dir()
            .map_err(|err| err.to_string())?
            .next()
            .is_some()
    {
        return Err(format!(
            "target directory already exists and is not empty: {}",
            target_path.display()
        ));
    }
    fs::create_dir_all(&target_path)
        .map_err(|err| format!("failed to create {}: {err}", target_path.display()))?;

    let written_files = write_product_intake_scaffold(repo_root, &target_path, &intake)?;

    let mut next_commands = vec![
        format!("cd {}", target_path.display()),
        "review .appfw/poc-intake.yaml".to_string(),
    ];
    if product_intake_is_legacy(&intake) {
        next_commands.extend([
            "review .appfw/legacy-modernization.yaml".to_string(),
            "inventory legacy code, database, procedures, jobs, auth, integrations, and UI routes"
                .to_string(),
            "create the modernization slice, capability map, stored procedure disposition, and migration posture"
                .to_string(),
        ]);
    } else {
        next_commands.push("inspect the PoC workbook/static data/HTML artifacts".to_string());
    }
    if intake.ui != "none" {
        next_commands.push("cd frontend && npm run appfw:check".to_string());
    }
    next_commands.extend([
        "scripts/appfw product analyze --summary --json".to_string(),
        "scripts/appfw product propose-model --summary --json".to_string(),
        "scripts/appfw product model-status --json".to_string(),
        "review .appfw/model-proposal.yaml".to_string(),
        format!(
            "create .appfw/model/schemas/{}/ entity, relationship, seed, and API scenario source",
            intake.schema
        ),
        "scripts/appfw product validate --json".to_string(),
        "scripts/appfw product generate".to_string(),
        "scripts/appfw product generate --check --json".to_string(),
        "scripts/appfw product test --fast".to_string(),
    ]);

    let report = json!({
        "command": "new",
        "ok": true,
        "target": target_path.display().to_string(),
        "source": source,
        "profile": "product-intake",
        "product_intake": intake,
        "required_questions": product_intake_required_questions(),
        "written_files": written_files,
        "local_env": ".appfw/local.env",
        "validated": false,
        "model_status": "not_started",
        "readiness_status": "intake_only_not_generated_application",
        "validation_status": "topology/config validation may pass before modeling; product readiness is deferred until .appfw/model contains the reviewed entity model",
        "generated": false,
        "next_commands": next_commands
    });
    emit_value(json_output, &report, || {
        format!(
            "Created product intake scaffold at {}\nNext:\n  cd {}\n  review .appfw/poc-intake.yaml\n  {}\n  scripts/appfw product analyze --summary --json\n  scripts/appfw product propose-model --summary --json\n  scripts/appfw product model-status --json\n  review .appfw/model-proposal.yaml\n  create the product entity model\n  scripts/appfw product validate --json",
            target_path.display(),
            target_path.display(),
            if product_intake_is_legacy(&intake) {
                "review .appfw/legacy-modernization.yaml"
            } else {
                "inspect the PoC artifacts"
            },
        )
    })
}

fn complete_product_intake(
    mut args: ProductIntakeArgs,
    json_output: bool,
) -> Result<ProductIntake, String> {
    let interactive = !json_output && io::stdin().is_terminal() && io::stdout().is_terminal();
    if interactive {
        prompt_missing_product_intake(&mut args)?;
    }

    let mut missing = Vec::new();
    for (field, value) in [
        ("app_name", &args.app_name),
        ("display_name", &args.display_name),
        ("schema", &args.schema),
        ("provider", &args.provider),
        ("mcp_server", &args.mcp_server),
        ("kafka_client", &args.kafka_client),
        ("ui", &args.ui),
    ] {
        if value.as_deref().unwrap_or_default().trim().is_empty() {
            missing.push(field);
        }
    }
    if !missing.is_empty() {
        return Err(format!(
            "product-intake requires answers for {}. Provide flags or run interactively. Required questions: {}",
            missing.join(", "),
            product_intake_required_questions()
        ));
    }

    let app_name = args.app_name.unwrap_or_default().trim().to_string();
    let display_name = args.display_name.unwrap_or_default().trim().to_string();
    let schema = args.schema.unwrap_or_default().trim().to_string();
    let source_kind = canonical_product_source_kind(
        args.source_kind
            .as_deref()
            .filter(|value| !value.trim().is_empty())
            .unwrap_or("poc"),
    )?
    .to_string();
    validate_product_identifier("app-name", &app_name)?;
    validate_product_identifier("schema", &schema)?;
    let provider = canonical_product_provider(&args.provider.unwrap_or_default())?;
    let data_source_name = product_provider_data_source_name(&provider).to_string();
    let mcp_server = parse_product_intake_bool("--mcp", &args.mcp_server.unwrap_or_default())?;
    let kafka_client =
        parse_product_intake_bool("--kafka", &args.kafka_client.unwrap_or_default())?;
    let ui = canonical_product_ui_mode(&args.ui.unwrap_or_default())?.to_string();

    Ok(ProductIntake {
        source_kind,
        app_name,
        display_name,
        schema,
        provider,
        data_source_name,
        mcp_server,
        kafka_client,
        ui,
        poc_source: non_empty_option(args.poc_source),
        data_artifact: non_empty_option(args.data_artifact),
        ui_artifact: non_empty_option(args.ui_artifact),
    })
}

fn prompt_missing_product_intake(args: &mut ProductIntakeArgs) -> Result<(), String> {
    if args.source_kind.is_none() {
        args.source_kind = Some(prompt_product_intake_value(
            "Source kind (--source-kind)",
            Some("poc"),
            Some(&product_intake_source_kind_options().join(", ")),
        )?);
    }
    if args.app_name.is_none() {
        args.app_name = Some(prompt_product_intake_value(
            "Application name (--app-name)",
            None,
            None,
        )?);
    }
    if args.display_name.is_none() {
        args.display_name = Some(prompt_product_intake_value(
            "Display name (--display-name)",
            args.app_name.as_deref(),
            None,
        )?);
    }
    if args.schema.is_none() {
        args.schema = Some(prompt_product_intake_value(
            "Product schema (--schema)",
            None,
            None,
        )?);
    }
    if args.provider.is_none() {
        args.provider = Some(prompt_product_intake_value(
            "Backend provider (--provider)",
            Some("PostgreSQL"),
            Some(&product_intake_provider_options().join(", ")),
        )?);
    }
    if args.mcp_server.is_none() {
        args.mcp_server = Some(prompt_product_intake_value(
            "Need MCP server? (--mcp)",
            Some("false"),
            Some("true, false"),
        )?);
    }
    if args.kafka_client.is_none() {
        args.kafka_client = Some(prompt_product_intake_value(
            "Need Kafka client? (--kafka)",
            Some("false"),
            Some("true, false"),
        )?);
    }
    if args.ui.is_none() {
        args.ui = Some(prompt_product_intake_value(
            "Need product UI? (--ui)",
            Some("enterprise"),
            Some(&product_intake_ui_options().join(", ")),
        )?);
    }
    let source_kind = args.source_kind.as_deref().unwrap_or("poc");
    if args.poc_source.is_none() {
        let source_label = if source_kind.eq_ignore_ascii_case("legacy") {
            "Legacy codebase/source folder (--legacy-source, optional)"
        } else {
            "PoC source folder (--poc-source, optional)"
        };
        args.poc_source = non_empty_option(Some(prompt_product_intake_value(
            source_label,
            Some(""),
            None,
        )?));
    }
    if args.data_artifact.is_none() {
        args.data_artifact = non_empty_option(Some(prompt_product_intake_value(
            "Example data artifact (--data-artifact, optional)",
            Some(""),
            None,
        )?));
    }
    if args.ui_artifact.is_none() {
        let ui_label = if source_kind.eq_ignore_ascii_case("legacy") {
            "Legacy UI screenshots or route inventory (--ui-artifact, optional)"
        } else {
            "PoC UI artifact (--ui-artifact, optional)"
        };
        args.ui_artifact =
            non_empty_option(Some(prompt_product_intake_value(ui_label, Some(""), None)?));
    }
    Ok(())
}

fn prompt_product_intake_value(
    label: &str,
    default: Option<&str>,
    allowed: Option<&str>,
) -> Result<String, String> {
    let mut stdout = io::stdout();
    write!(stdout, "{label}").map_err(|err| err.to_string())?;
    if let Some(allowed) = allowed {
        write!(stdout, " [{allowed}]").map_err(|err| err.to_string())?;
    }
    if let Some(default) = default {
        if !default.is_empty() {
            write!(stdout, " (default: {default})").map_err(|err| err.to_string())?;
        }
    }
    write!(stdout, ": ").map_err(|err| err.to_string())?;
    stdout.flush().map_err(|err| err.to_string())?;

    let mut input = String::new();
    io::stdin()
        .read_line(&mut input)
        .map_err(|err| err.to_string())?;
    let value = input.trim();
    if value.is_empty() {
        Ok(default.unwrap_or_default().to_string())
    } else {
        Ok(value.to_string())
    }
}

fn non_empty_option(value: Option<String>) -> Option<String> {
    value
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn validate_product_identifier(label: &str, value: &str) -> Result<(), String> {
    if value.is_empty() {
        return Err(format!("{label} cannot be empty"));
    }
    let valid = value
        .chars()
        .all(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == '_' || ch == '-')
        && value
            .chars()
            .next()
            .is_some_and(|ch| ch.is_ascii_lowercase());
    if valid {
        Ok(())
    } else {
        Err(format!(
            "{label} `{value}` must start with a lowercase letter and use only lowercase letters, numbers, hyphen, or underscore"
        ))
    }
}

fn canonical_product_source_kind(input: &str) -> Result<&'static str, String> {
    match input.trim().to_ascii_lowercase().as_str() {
        "poc" | "prototype" | "vibe" | "vibe-coded" | "citizen-poc" | "citizen_poc" => Ok("poc"),
        "legacy" | "legacy-app" | "legacy_application" | "modernization" | "modernize" => {
            Ok("legacy")
        }
        value => Err(format!(
            "--source-kind expects one of {}, got `{value}`",
            product_intake_source_kind_options().join(", ")
        )),
    }
}

fn canonical_product_provider(input: &str) -> Result<String, String> {
    match input.trim().to_ascii_lowercase().as_str() {
        "postgres" | "postgresql" | "pg" => Ok("PostgreSQL".to_string()),
        "mongo" | "mongodb" | "mssql" | "sqlserver" | "sql-server" | "mssqlserver"
        | "ms-sql-server" | "snowflake" => Err(format!(
            "product-intake currently creates runnable backend scaffolds for {} only. Framework provider certification still covers MongoDB, MS SQL Server, and Snowflake; add the provider-specific product client scaffold before selecting `{}` for product bootstrap.",
            product_intake_provider_options().join(", "),
            input.trim()
        )),
        "neo4j" => Err(
            "Neo4j is a graph read-model capability, not a primary product persistence provider. Choose a CRUD provider such as PostgreSQL for product bootstrap, then add a Neo4j data source for named graph read operations."
                .to_string(),
        ),
        value => Err(format!(
            "unsupported --provider `{value}`. Allowed providers: {}",
            product_intake_provider_options().join(", ")
        )),
    }
}

fn product_provider_data_source_name(provider: &str) -> &'static str {
    match provider {
        "PostgreSQL" => "pg_primary",
        "MongoDB" => "mongo_primary",
        "MsSqlServer" => "mssql_primary",
        "Snowflake" => "snowflake_primary",
        "Neo4j" => "neo4j_graph",
        _ => "primary",
    }
}

fn parse_product_intake_bool(option: &str, input: &str) -> Result<bool, String> {
    match input.trim().to_ascii_lowercase().as_str() {
        "true" | "yes" | "y" | "1" | "needed" | "required" => Ok(true),
        "false" | "no" | "n" | "0" | "not-needed" | "none" => Ok(false),
        value => Err(format!("{option} expects true or false, got `{value}`")),
    }
}

fn canonical_product_ui_mode(input: &str) -> Result<&'static str, String> {
    match input.trim().to_ascii_lowercase().as_str() {
        "none" | "false" | "no" | "n" | "0" | "defer" | "deferred" => Ok("none"),
        "scaffold" | "starter" => Ok("scaffold"),
        "enterprise" | "true" | "yes" | "y" | "1" | "requested" | "required" => Ok("enterprise"),
        value => Err(format!(
            "--ui expects one of {}, got `{value}`",
            product_intake_ui_options().join(", ")
        )),
    }
}

fn write_product_intake_scaffold(
    framework_root: &Path,
    target_root: &Path,
    intake: &ProductIntake,
) -> Result<Vec<String>, String> {
    let mut files = vec![
        (
            ".appfw/manifest.yaml".to_string(),
            product_intake_manifest_yaml(intake),
        ),
        (
            ".appfw/poc-intake.yaml".to_string(),
            product_intake_yaml(intake),
        ),
        ("README.md".to_string(), product_intake_readme(intake)),
        (".gitignore".to_string(), product_intake_gitignore()),
        ("Cargo.toml".to_string(), product_intake_root_cargo(intake)),
        ("scripts/appfw".to_string(), product_appfw_script()),
        ("podman-compose.yml".to_string(), product_intake_compose_yaml(intake)),
        (
            ".appfw/model/data_sources/_res.yaml".to_string(),
            product_intake_data_sources_yaml(intake),
        ),
        (
            ".appfw/model/schemas/system/_res.yaml".to_string(),
            product_intake_system_schema_yaml(intake),
        ),
        (
            format!(".appfw/model/schemas/{}/_res.yaml", intake.schema),
            product_intake_schema_yaml(intake),
        ),
        (
            ".appfw/model/schemas/README.md".to_string(),
            "Product schema source lives under schema folders. Create entities, relationships, seeds, and API scenarios from the intake evidence before running generation.\n".to_string(),
        ),
        (
            "backend/Cargo.toml".to_string(),
            product_intake_backend_cargo(target_root, framework_root, intake),
        ),
        ("backend/README.md".to_string(), product_intake_backend_readme(intake)),
        (
            "backend/.gitignore".to_string(),
            "target/\nadmin_dist/\nproduct_dist/\n".to_string(),
        ),
        ("backend/src/main.rs".to_string(), product_intake_backend_main(intake)),
        ("backend/src/lib.rs".to_string(), product_intake_backend_lib(intake)),
        (
            "backend/src/services/mod.rs".to_string(),
            product_intake_services_mod(),
        ),
        ("database/README.md".to_string(), product_intake_database_readme(intake)),
        (
            "api_tests/Cargo.toml".to_string(),
            product_intake_api_tests_cargo(target_root, framework_root),
        ),
        ("api_tests/README.md".to_string(), product_intake_api_tests_readme(intake)),
        ("api_tests/src/main.rs".to_string(), "fn main() {}\n".to_string()),
        ("api_tests/src/lib.rs".to_string(), "pub mod harness {\n    pub use appfw_test::*;\n}\npub mod schemas;\n".to_string()),
        ("api_tests/src/schemas/mod.rs".to_string(), product_intake_api_tests_mod(intake)),
        (
            "rego_test/Cargo.toml".to_string(),
            product_intake_rego_test_cargo(target_root, framework_root),
        ),
        ("rego_test/README.md".to_string(), product_intake_rego_test_readme(intake)),
        ("rego_test/src/main.rs".to_string(), "fn main() {}\n".to_string()),
        ("rego_test/src/lib.rs".to_string(), "pub fn product_policy_surface() -> &'static str {\n    \"product-intake\"\n}\n".to_string()),
        ("rego_test/tests/policy_contract.rs".to_string(), product_intake_rego_policy_test(intake)),
    ];
    if let Some(cargo_config) = product_framework_cargo_config(framework_root) {
        files.push((".cargo/config.toml".to_string(), cargo_config));
    }

    if product_intake_is_legacy(intake) {
        files.push((
            ".appfw/legacy-modernization.yaml".to_string(),
            product_legacy_modernization_yaml(intake),
        ));
    }

    if intake.ui != "none" {
        files.extend(product_intake_frontend_files(
            target_root,
            framework_root,
            intake,
        ));
    }

    let mut written = Vec::new();
    for (relative, contents) in files {
        let path = target_root.join(&relative);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .map_err(|err| format!("failed to create {}: {err}", parent.display()))?;
        }
        fs::write(&path, contents)
            .map_err(|err| format!("failed to write {}: {err}", path.display()))?;
        make_product_script_executable(&path, &relative)?;
        written.push(relative);
    }

    copy_framework_model_seeds(framework_root, target_root, &mut written)?;
    copy_product_intake_backend_base(framework_root, target_root, intake, &mut written)?;

    let local_env = target_root.join(".appfw/local.env");
    if let Some(parent) = local_env.parent() {
        fs::create_dir_all(parent)
            .map_err(|err| format!("failed to create {}: {err}", parent.display()))?;
    }
    let contents = format!(
        "APPFW_FRAMEWORK_ROOT={}\n",
        shell_single_quote(&framework_root.display().to_string())
    );
    fs::write(&local_env, contents)
        .map_err(|err| format!("failed to write {}: {err}", local_env.display()))?;
    written.push(".appfw/local.env".to_string());

    Ok(written)
}

fn copy_framework_model_seeds(
    framework_root: &Path,
    target_root: &Path,
    written: &mut Vec<String>,
) -> Result<(), String> {
    for (source_relative_root, target_relative_root) in [
        (
            format!("{FRAMEWORK_MODEL_SEED_ROOT}/_facets"),
            format!("{PRODUCT_MODEL_ROOT}/_facets"),
        ),
        (
            format!("{FRAMEWORK_MODEL_SEED_ROOT}/_fragments"),
            format!("{PRODUCT_MODEL_ROOT}/_fragments"),
        ),
        (
            format!("{FRAMEWORK_MODEL_SEED_ROOT}/schemas/system"),
            format!("{PRODUCT_MODEL_ROOT}/schemas/system"),
        ),
    ] {
        let source_root = framework_root.join(&source_relative_root);
        let target_config_root = target_root.join(PRODUCT_MODEL_ROOT);
        if !source_root.is_dir() {
            return Err(format!(
                "product intake scaffold seed is missing: {}",
                source_root.display()
            ));
        }
        copy_product_intake_config_seed_dir(
            &source_root,
            &source_root,
            &target_config_root,
            &target_relative_root,
            written,
        )?;
    }
    written.sort();
    written.dedup();
    Ok(())
}

fn copy_product_intake_backend_base(
    framework_root: &Path,
    target_root: &Path,
    intake: &ProductIntake,
    written: &mut Vec<String>,
) -> Result<(), String> {
    let mut files = vec![
        "backend/src/admin_ui.rs",
        "backend/src/config/app_config.rs",
        "backend/src/config/loader.rs",
        "backend/src/config/mod.rs",
        "backend/src/data/clients/database_client.rs",
        "backend/src/data/data_access.rs",
        "backend/src/data/mod.rs",
        "backend/src/data/query_ir.rs",
        "backend/src/data/rules/mod.rs",
        "backend/src/handlers/selections.rs",
        "backend/src/kafka.rs",
        "backend/src/mcp/mod.rs",
        "backend/src/product_api.rs",
        "backend/src/routes/app_error.rs",
        "backend/src/routes/info.rs",
        "backend/src/schemas/common.rs",
        "backend/src/sync_workers.rs",
    ];
    if intake.provider == "PostgreSQL" {
        files.extend([
            "backend/src/data/clients/postgres/README.md",
            "backend/src/data/clients/postgres/cte.rs",
            "backend/src/data/clients/postgres/filter.rs",
            "backend/src/data/clients/postgres/mod.rs",
            "backend/src/data/clients/postgres/postgres_client.rs",
        ]);
    }
    if intake.provider == "MsSqlServer" {
        files.extend([
            "backend/src/data/clients/mssql/cte.rs",
            "backend/src/data/clients/mssql/filter.rs",
            "backend/src/data/clients/mssql/mod.rs",
            "backend/src/data/clients/mssql/mssql_client.rs",
            "backend/src/data/clients/mssql/naming.rs",
            "backend/src/data/clients/mssql/param.rs",
            "backend/src/data/clients/mssql/sort.rs",
        ]);
    }
    if intake.provider == "FabricSqlAnalytics" {
        files.extend([
            "backend/src/data/clients/mssql/cte.rs",
            "backend/src/data/clients/mssql/filter.rs",
            "backend/src/data/clients/mssql/mod.rs",
            "backend/src/data/clients/mssql/naming.rs",
            "backend/src/data/clients/mssql/param.rs",
            "backend/src/data/clients/mssql/sort.rs",
            "backend/src/data/clients/fabric_sql_analytics/mod.rs",
            "backend/src/data/clients/fabric_sql_analytics/fabric_sql_analytics_client.rs",
        ]);
    }

    for relative in files {
        copy_product_intake_backend_base_file(
            framework_root,
            target_root,
            intake,
            relative,
            written,
        )?;
    }
    write_product_intake_backend_file(
        target_root,
        "backend/src/data/clients/mod.rs",
        &product_intake_clients_mod(intake),
        written,
    )?;
    written.sort();
    written.dedup();
    Ok(())
}

fn write_product_intake_backend_file(
    target_root: &Path,
    relative: &str,
    contents: &str,
    written: &mut Vec<String>,
) -> Result<(), String> {
    let dest_path = target_root.join(relative);
    if let Some(parent) = dest_path.parent() {
        fs::create_dir_all(parent)
            .map_err(|err| format!("failed to create {}: {err}", parent.display()))?;
    }
    fs::write(&dest_path, contents)
        .map_err(|err| format!("failed to write {}: {err}", dest_path.display()))?;
    written.push(relative.to_string());
    Ok(())
}

fn product_intake_clients_mod(intake: &ProductIntake) -> String {
    let provider_lines = match intake.provider.as_str() {
        "PostgreSQL" => "pub(crate) mod postgres;\n".to_string(),
        "MsSqlServer" => "pub(crate) mod mssql;\n".to_string(),
        "FabricSqlAnalytics" => {
            "pub(crate) mod fabric_sql_analytics;\npub(crate) mod mssql;\n".to_string()
        }
        _ => String::new(),
    };
    format!("pub(crate) mod database_client;\n{provider_lines}")
}

fn copy_product_intake_backend_base_file(
    framework_root: &Path,
    target_root: &Path,
    intake: &ProductIntake,
    relative: &str,
    written: &mut Vec<String>,
) -> Result<(), String> {
    let source_path = product_intake_backend_base_source_path(framework_root, intake, relative);
    if !source_path.is_file() {
        return Err(format!(
            "product intake backend base file is missing: {}",
            source_path.display()
        ));
    }
    let dest_path = target_root.join(relative);
    if let Some(parent) = dest_path.parent() {
        fs::create_dir_all(parent)
            .map_err(|err| format!("failed to create {}: {err}", parent.display()))?;
    }
    let mut contents = fs::read_to_string(&source_path)
        .map_err(|err| format!("failed to read {}: {err}", source_path.display()))?;
    contents = contents.replace("__APPFW_SCHEMA__", &intake.schema);
    contents = contents.replace("crm.Account", &format!("{}.Account", intake.schema));
    contents = contents.replace(
        "schema_name: \"crm\"",
        &format!("schema_name: \"{}\"", intake.schema),
    );
    fs::write(&dest_path, contents)
        .map_err(|err| format!("failed to write {}: {err}", dest_path.display()))?;
    written.push(relative.to_string());
    Ok(())
}

fn product_intake_backend_base_source_path(
    framework_root: &Path,
    intake: &ProductIntake,
    relative: &str,
) -> PathBuf {
    let template_path = framework_root
        .join("app_gen/_templates/product_intake")
        .join(relative);
    if product_intake_backend_base_prefers_template(intake, relative) || template_path.is_file() {
        return template_path;
    }
    framework_root.join("examples/products/crm").join(relative)
}

fn product_intake_backend_base_prefers_template(intake: &ProductIntake, relative: &str) -> bool {
    match intake.provider.as_str() {
        "FabricSqlAnalytics" => {
            relative.starts_with("backend/src/data/clients/fabric_sql_analytics/")
                || relative.starts_with("backend/src/data/clients/mssql/")
        }
        _ => false,
    }
}

fn copy_product_intake_config_seed_dir(
    root: &Path,
    source: &Path,
    target_config_root: &Path,
    relative_root: &str,
    written: &mut Vec<String>,
) -> Result<(), String> {
    for entry in fs::read_dir(source)
        .map_err(|err| format!("failed to read config seed {}: {err}", source.display()))?
    {
        let entry = entry.map_err(|err| err.to_string())?;
        let source_path = entry.path();
        let relative_to_seed = source_path
            .strip_prefix(root)
            .map_err(|err| err.to_string())?;
        let relative_to_app = Path::new(relative_root).join(relative_to_seed);
        let relative_text = relative_to_app.to_string_lossy().replace('\\', "/");
        if relative_text == format!("{PRODUCT_MODEL_ROOT}/schemas/system/_res.yaml") {
            continue;
        }
        let file_type = entry.file_type().map_err(|err| err.to_string())?;
        if file_type.is_dir() {
            copy_product_intake_config_seed_dir(
                root,
                &source_path,
                target_config_root,
                relative_root,
                written,
            )?;
        } else if file_type.is_file() {
            let dest_path = target_config_root.join(
                relative_to_app
                    .strip_prefix(PRODUCT_MODEL_ROOT)
                    .map_err(|err| err.to_string())?,
            );
            if let Some(parent) = dest_path.parent() {
                fs::create_dir_all(parent)
                    .map_err(|err| format!("failed to create {}: {err}", parent.display()))?;
            }
            fs::copy(&source_path, &dest_path).map_err(|err| {
                format!(
                    "failed to copy config seed {} to {}: {err}",
                    source_path.display(),
                    dest_path.display()
                )
            })?;
            written.push(relative_text);
        }
    }
    Ok(())
}

fn product_intake_is_legacy(intake: &ProductIntake) -> bool {
    intake.source_kind == "legacy"
}

fn make_product_script_executable(path: &Path, relative: &str) -> Result<(), String> {
    if relative != "scripts/appfw" {
        return Ok(());
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut permissions = fs::metadata(path)
            .map_err(|err| format!("failed to stat {}: {err}", path.display()))?
            .permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(path, permissions)
            .map_err(|err| format!("failed to chmod {}: {err}", path.display()))?;
    }
    Ok(())
}

fn product_intake_manifest_yaml(intake: &ProductIntake) -> String {
    let ui_consumer = if intake.ui == "none" {
        ""
    } else {
        "    - browser\n"
    };
    let source_description = if product_intake_is_legacy(intake) {
        "a legacy application modernization intake"
    } else {
        "a citizen-developer PoC"
    };
    let schema_description = if product_intake_is_legacy(intake) {
        "Product schema inferred from legacy application evidence."
    } else {
        "Product schema inferred from PoC artifacts."
    };
    format!(
        r#"version: 1
app:
  name: {app_name}
  display_name: {display_name}
  description: Product intake scaffold generated from {source_description}.
topology:
  data_sources:
  - name: {data_source}
    provider: {provider}
    role: transactional
    description: Primary provider selected during product intake.
  schemas:
  - name: system
    data_source_name: {data_source}
    role: framework
    description: Runtime metadata schema loaded from generated config.
  - name: {schema}
    data_source_name: {data_source}
    role: product
    description: {schema_description}
  ingress:
  - name: {schema}-http
    kind: http
    enabled: true
    schema: {schema}
    path: /{schema}
    consumers:
{ui_consumer}    - service-api
  - name: app-mcp
    kind: mcp
    enabled: {mcp}
    path: /mcp
    consumers:
    - agent
  - name: {schema}-events
    kind: kafka
    enabled: {kafka}
    schema: {schema}
    topic: {schema}.events
    consumer_group: {schema}-events-worker
    handler: services.{schema}_events
"#,
        app_name = yaml_scalar(&intake.app_name),
        display_name = yaml_scalar(&intake.display_name),
        data_source = intake.data_source_name,
        provider = intake.provider,
        schema = intake.schema,
        source_description = source_description,
        schema_description = schema_description,
        mcp = intake.mcp_server,
        kafka = intake.kafka_client,
        ui_consumer = ui_consumer,
    )
}

fn product_intake_yaml(intake: &ProductIntake) -> String {
    let source = if product_intake_is_legacy(intake) {
        "legacy_application"
    } else {
        "citizen_poc"
    };
    let skill = if product_intake_is_legacy(intake) {
        "product-legacy-modernization"
    } else {
        "product-poc-intake"
    };
    let next_step = if product_intake_is_legacy(intake) {
        "Analyze legacy code and data, produce the modernization map and stored procedure disposition, then create .appfw/model source before generation."
    } else {
        "Inspect PoC artifacts, infer the product entity model, then create .appfw/model source before generation."
    };
    format!(
        r#"version: 1
source: {source}
source_kind: {source_kind}
app:
  name: {app_name}
  display_name: {display_name}
schema:
  name: {schema}
topology:
  backend_provider:
    provider: {provider}
    data_source_name: {data_source}
  ingress:
    mcp_server: {mcp}
    kafka_client: {kafka}
    ui: {ui}
artifacts:
  poc_source: {poc_source}
  data_artifact: {data_artifact}
  ui_artifact: {ui_artifact}
agent_contract:
  skill: {skill}
  sample_reference_only: true
  next_step: {next_step}
"#,
        source = source,
        source_kind = yaml_scalar(&intake.source_kind),
        app_name = yaml_scalar(&intake.app_name),
        display_name = yaml_scalar(&intake.display_name),
        schema = yaml_scalar(&intake.schema),
        provider = yaml_scalar(&intake.provider),
        data_source = yaml_scalar(&intake.data_source_name),
        mcp = intake.mcp_server,
        kafka = intake.kafka_client,
        ui = yaml_scalar(&intake.ui),
        poc_source = yaml_optional_scalar(intake.poc_source.as_deref()),
        data_artifact = yaml_optional_scalar(intake.data_artifact.as_deref()),
        ui_artifact = yaml_optional_scalar(intake.ui_artifact.as_deref()),
        skill = yaml_scalar(skill),
        next_step = yaml_scalar(next_step),
    )
}

fn product_legacy_modernization_yaml(intake: &ProductIntake) -> String {
    format!(
        r#"version: 1
source: legacy_application
product:
  app_name: {app_name}
  display_name: {display_name}
  schema: {schema}
  provider: {provider}
  data_source_name: {data_source}
legacy_evidence:
  codebase: {codebase}
  data_source_inventory: {data_artifact}
  ui_inventory: {ui_artifact}
  reviewed_paths: []
modernization_slice:
  name: null
  goals: []
  excluded_capabilities: []
  rollback_posture: null
static_analysis:
  routes_controllers_pages: []
  services_and_domain_logic: []
  orm_raw_sql_and_data_access: []
  stored_procedure_callers: []
  jobs_schedulers_and_queues: []
  auth_roles_claims_and_policy: []
  configuration_and_secrets: []
  integrations_reports_and_files: []
data_discovery:
  tables_views_and_row_counts: []
  keys_relationships_constraints_and_indexes: []
  lookup_values_and_status_codes: []
  temporal_audit_version_fields: []
  sensitive_fields_and_classification: []
  procedures_functions_triggers_and_jobs: []
stored_procedure_disposition:
  routines: []
  allowed_dispositions:
    - eliminate
    - generated_query_crud
    - product_service
    - custom_method_dto
    - provider_routine_temporary
    - provider_routine_retained
target_architecture:
  entities: []
  relationships: []
  dto_result_types: []
  custom_methods: []
  product_services: []
  retained_provider_routines: []
  ingress:
    http: true
    mcp: {mcp}
    kafka: {kafka}
frontend_rebuild:
  needed: {ui_needed}
  routes: []
  dashboard_or_workbench: null
  accessibility_and_e2e_evidence: []
migration_and_parallel_run:
  migration_plan: null
  reconciliation_checks: []
  rollback_checks: []
  live_api_tests: []
hardening:
  security_evidence: []
  performance_evidence: []
  provider_certification_evidence: []
  ops_observability_evidence: []
agent_contract:
  skill: product-legacy-modernization
  canonical_doc: docs/lifecycle/legacy-modernization.md
  next_step: Complete this inventory before treating generated config as ready.
"#,
        app_name = yaml_scalar(&intake.app_name),
        display_name = yaml_scalar(&intake.display_name),
        schema = yaml_scalar(&intake.schema),
        provider = yaml_scalar(&intake.provider),
        data_source = yaml_scalar(&intake.data_source_name),
        codebase = yaml_optional_scalar(intake.poc_source.as_deref()),
        data_artifact = yaml_optional_scalar(intake.data_artifact.as_deref()),
        ui_artifact = yaml_optional_scalar(intake.ui_artifact.as_deref()),
        mcp = intake.mcp_server,
        kafka = intake.kafka_client,
        ui_needed = intake.ui != "none",
    )
}

fn product_intake_readme(intake: &ProductIntake) -> String {
    let source_label = if product_intake_is_legacy(intake) {
        "legacy application modernization"
    } else {
        "citizen-developer PoC conversion"
    };
    let evidence_file = if product_intake_is_legacy(intake) {
        "Review `.appfw/poc-intake.yaml` and complete `.appfw/legacy-modernization.yaml` before generation."
    } else {
        "Review `.appfw/poc-intake.yaml` before generation."
    };
    let next_step = if product_intake_is_legacy(intake) {
        "the AI harness and product developer to analyze the legacy codebase, data source, stored procedures, jobs, auth, integrations, and UI evidence; produce the modernization slice and stored procedure disposition; then write `.appfw/model` source."
    } else {
        "the AI harness and product developer to inspect the PoC source artifacts named in `.appfw/poc-intake.yaml`, infer the real product entities, relationships, seeds, custom methods, and UI workflows, and then write `.appfw/model` source."
    };
    format!(
        r#"# {display_name}

This product workspace was created by `scripts/appfw product new --profile product-intake`.

It is an intake scaffold for {source_label}, not a generated application yet.
The next step is for {next_step}

{evidence_file}

## Selected Topology

- Product schema: `{schema}`
- Backend provider: `{provider}`
- MCP server: `{mcp}`
- Kafka client: `{kafka}`
- Product UI mode: `{ui}`

## Next Commands

```bash
scripts/appfw doctor
scripts/appfw product analyze --summary --json
scripts/appfw product propose-model --summary --json
scripts/appfw product model-status --json
# Review .appfw/model-proposal.yaml before writing .appfw/model.
# scripts/appfw product validate --json is useful as topology sanity before modeling,
# but it is not product-readiness evidence until entity model source exists.
scripts/appfw product validate --json
cargo generate-lockfile
scripts/appfw product generate
scripts/appfw product generate --check --json
scripts/appfw product test --fast
```

Run the generation and test commands after the product entity model has been
created from the source evidence. The repository shell is intentionally
product-owned: backend, database, API tests, policy tests, local compose, and
frontend files use this product's app/schema/provider choices.
"#,
        display_name = intake.display_name,
        source_label = source_label,
        next_step = next_step,
        evidence_file = evidence_file,
        schema = intake.schema,
        provider = intake.provider,
        mcp = intake.mcp_server,
        kafka = intake.kafka_client,
        ui = intake.ui,
    )
}

fn product_intake_gitignore() -> String {
    r#".DS_Store
.appfw/local.env
Cargo.lock
target/
backend/target/
database/target/
api_tests/target/
rego_test/target/
frontend/node_modules/
frontend/dist/
backend/product_dist/
frontend/playwright-report/
frontend/test-results/
*.log
"#
    .to_string()
}

fn product_intake_root_cargo(_intake: &ProductIntake) -> String {
    let members = ["api_tests", "backend", "rego_test"];
    format!(
        r#"[workspace]
members = [
{members}
]
resolver = "2"
"#,
        members = members
            .iter()
            .map(|member| format!("    \"{member}\","))
            .collect::<Vec<_>>()
            .join("\n")
    )
}

fn product_intake_backend_cargo(
    target_root: &Path,
    framework_root: &Path,
    intake: &ProductIntake,
) -> String {
    let backend_root = target_root.join("backend");
    let (provider_dep_name, provider_package, provider_path) =
        product_intake_provider_crate(intake.provider.as_str());
    let dependency_source = product_framework_dependency_source(framework_root);
    let runtime_dependency = framework_crate_dependency_line(
        &dependency_source,
        &backend_root,
        "appfw_runtime",
        "appfw-runtime",
        "appfw_runtime",
        &["default-features = false"],
    );
    let provider_dependency = framework_crate_dependency_line(
        &dependency_source,
        &backend_root,
        provider_dep_name,
        provider_package,
        provider_path,
        &[],
    );
    // Intake loader/client templates always reference the shared MS SQL auth
    // registry (MsSqlServer + FabricSqlAnalytics arms), so emit the dep for
    // every intake backend — not only when the primary provider is MS SQL.
    let mssql_auth_dependency = framework_crate_dependency_line(
        &dependency_source,
        &backend_root,
        "appfw_mssql_auth",
        "appfw-mssql-auth",
        "appfw_mssql_auth",
        &[],
    );
    let mut default_features = vec!["http"];
    if intake.mcp_server {
        default_features.push("mcp");
    }
    if intake.kafka_client {
        default_features.push("kafka");
    }
    let default_features = default_features
        .iter()
        .map(|feature| format!("\"{feature}\""))
        .collect::<Vec<_>>()
        .join(", ");

    format!(
        r#"[package]
name = "backend"
version = "0.1.0"
edition = "2021"
description = "{display_name} product backend"

[features]
default = [{default_features}]
http = [
    "appfw_runtime/http",
    "dep:async-graphql-axum",
    "dep:axum",
    "dep:hyper",
    "dep:http",
    "dep:tower",
    "dep:tower-http",
]
mcp = ["appfw_runtime/mcp"]
kafka = ["appfw_runtime/kafka"]
sync = ["appfw_runtime/sync"]

[dependencies]
anyhow = "1.0"
{runtime_dependency}
{provider_dependency}
{mssql_auth_dependency}
async-graphql = {{ version = "6", features = ["bson", "chrono", "dynamic-schema"] }}
async-graphql-value = "6"
async-graphql-axum = {{ version = "6", optional = true }}
async-trait = "0.1"
axum = {{ version = "0.6", features = ["macros"], optional = true }}
bson = "2.0"
bytes = "1.7"
chrono = {{ version = "0.4", features = ["clock", "serde"] }}
chrono-tz = "0.10"
deadpool = "0.12"
deadpool-postgres = "0.12"
derivative = "2.2"
dotenv = "0.15"
futures = "0.3"
futures-util = "0.3.27"
governor = "0.10"
hyper = {{ version = "1.0", optional = true }}
http = {{ version = "=1.1.0", optional = true }}
Inflector = "*"
json = "0.12"
opentelemetry = {{ version = "0.32", features = ["metrics", "trace"] }}
opentelemetry-otlp = {{ version = "0.32", default-features = false, features = ["http-proto", "metrics", "reqwest-blocking-client", "trace"] }}
opentelemetry_sdk = {{ version = "0.32", features = ["metrics", "rt-tokio", "trace"] }}
postgres-types = {{ version = "0.2", features = ["derive", "with-chrono-0_4"] }}
regorus = "0.2"
regex = "1"
serde = {{ version = "1.0", features = ["derive"] }}
serde_json = "1.0"
serde_yaml = "0.9"
sha2 = "0.10"
slab = "0.4.7"
thiserror = "2.0.12"
time = {{ version = "0.3", features = ["macros"] }}
tokio = {{ version = "1", features = ["full"] }}
tower = {{ version = "0.4", features = ["util"], optional = true }}
tower-http = {{ version = "0.4", features = ["cors", "fs", "request-id", "trace"], optional = true }}
tokio-postgres = {{ version = "0.7", features = ["with-serde_json-1", "with-uuid-1", "with-chrono-0_4"] }}
tracing = "0.1"
tracing-opentelemetry = "0.33"
tracing-subscriber = {{ version = "0.3", features = ["env-filter", "json"] }}
uuid = {{ version = "1", features = ["serde", "v4"] }}
"#,
        display_name = toml_escape(&intake.display_name),
        default_features = default_features,
        runtime_dependency = runtime_dependency,
        provider_dependency = provider_dependency,
        mssql_auth_dependency = mssql_auth_dependency,
    )
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum FrameworkCrateDependencySource {
    Path {
        framework_root: PathBuf,
    },
    Registry {
        registry: String,
        version: String,
        index: String,
    },
}

fn product_framework_dependency_source(framework_root: &Path) -> FrameworkCrateDependencySource {
    let explicit = env::var("APPFW_PRODUCT_DEPENDENCY_SOURCE")
        .or_else(|_| env::var("APPFW_CRATE_SOURCE"))
        .unwrap_or_default()
        .trim()
        .to_ascii_lowercase();
    let packaged = framework_root.join("app-framework-package.json").is_file();
    if explicit == "path" || (!packaged && explicit.is_empty()) {
        return FrameworkCrateDependencySource::Path {
            framework_root: framework_root.to_path_buf(),
        };
    }

    FrameworkCrateDependencySource::Registry {
        registry: product_framework_cargo_registry_name(),
        version: product_framework_version(framework_root),
        index: product_framework_cargo_registry_index(),
    }
}

fn product_framework_cargo_registry_name() -> String {
    env::var("APPFW_CARGO_REGISTRY")
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| "pds-app-framework-crates".to_string())
}

fn product_framework_cargo_registry_index() -> String {
    env::var("APPFW_CARGO_REGISTRY_INDEX")
        .or_else(|_| env::var("APPFW_PROGET_CARGO_INDEX"))
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| {
            "sparse+https://proget.pdsconnect.com/cargo/pds-app-framework-crates/".to_string()
        })
}

fn product_framework_version(framework_root: &Path) -> String {
    env::var("APPFW_FRAMEWORK_VERSION")
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| {
            package_version(&framework_root.join("app_gen/Cargo.toml"))
                .unwrap_or_else(|_| "0.1.0".to_string())
        })
}

fn framework_crate_dependency_line(
    source: &FrameworkCrateDependencySource,
    manifest_root: &Path,
    dependency: &str,
    package: &str,
    crate_dir: &str,
    extra_fields: &[&str],
) -> String {
    let mut fields = vec![format!("package = \"{package}\"")];
    match source {
        FrameworkCrateDependencySource::Path { framework_root } => {
            let crate_path = relative_path_between(manifest_root, &framework_root.join(crate_dir));
            fields.push(format!("path = \"{}\"", crate_path.replace('\\', "/")));
        }
        FrameworkCrateDependencySource::Registry {
            registry, version, ..
        } => {
            fields.push(format!("version = \"{version}\""));
            fields.push(format!("registry = \"{registry}\""));
        }
    }
    fields.extend(extra_fields.iter().map(|field| field.to_string()));
    format!("{dependency} = {{ {} }}", fields.join(", "))
}

fn product_framework_cargo_config(framework_root: &Path) -> Option<String> {
    match product_framework_dependency_source(framework_root) {
        FrameworkCrateDependencySource::Registry {
            registry, index, ..
        } => Some(format!(
            r#"# Generated by App Framework for product consumption of framework crates.
# Authenticate with either:
#   cargo login --registry {registry} <api-key>
# or:
#   CARGO_REGISTRIES_{token_env_suffix}_TOKEN=<api-key>

[registries.{registry}]
index = "{index}"
"#,
            registry = registry,
            index = index,
            token_env_suffix = registry
                .chars()
                .map(|ch| if ch.is_ascii_alphanumeric() {
                    ch.to_ascii_uppercase()
                } else {
                    '_'
                })
                .collect::<String>()
        )),
        FrameworkCrateDependencySource::Path { .. } => None,
    }
}

fn product_intake_provider_crate(provider: &str) -> (&'static str, &'static str, &'static str) {
    match provider {
        "PostgreSQL" => (
            "appfw_provider_postgres",
            "appfw-provider-postgres",
            "appfw_provider_postgres",
        ),
        "MongoDB" => (
            "appfw_provider_mongo",
            "appfw-provider-mongo",
            "appfw_provider_mongo",
        ),
        "MsSqlServer" => (
            "appfw_provider_mssql",
            "appfw-provider-mssql",
            "appfw_provider_mssql",
        ),
        "Snowflake" => (
            "appfw_provider_snowflake",
            "appfw-provider-snowflake",
            "appfw_provider_snowflake",
        ),
        "Neo4j" => (
            "appfw_provider_neo4j",
            "appfw-provider-neo4j",
            "appfw_provider_neo4j",
        ),
        _ => (
            "appfw_provider_postgres",
            "appfw-provider-postgres",
            "appfw_provider_postgres",
        ),
    }
}

fn product_intake_backend_readme(intake: &ProductIntake) -> String {
    let model_source = if product_intake_is_legacy(intake) {
        "legacy-modernization evidence"
    } else {
        "PoC-derived evidence"
    };
    format!(
        r#"# {display_name} Backend

This backend crate is product-owned. It starts with the selected provider and
ingress feature flags from `.appfw/poc-intake.yaml`.

Generated route, schema, handler, operation, and data-access files should be
created only after the {model_source} entity model is written under
`.appfw/model/schemas/{schema}`.

Run from the product root after the model exists:

```bash
scripts/appfw product generate
scripts/appfw product serve
```
"#,
        display_name = intake.display_name,
        model_source = model_source,
        schema = intake.schema,
    )
}

fn product_intake_backend_main(_intake: &ProductIntake) -> String {
    r#"#[cfg(feature = "http")]
use appfw_runtime::{cors, RuntimeAuthState, RuntimeHttpServerConfig};
use appfw_runtime::{
    observability::init_tracing, security::SecurityConfig, RuntimeHostPlan, RuntimeMode,
};
use dotenv::dotenv;
#[cfg(feature = "http")]
use std::sync::Arc;
use tracing::error;
#[cfg(feature = "http")]
use tracing::info;

#[cfg(feature = "http")]
mod admin_ui;
mod config;
mod data;
mod handlers;
#[cfg(feature = "kafka")]
mod kafka;
#[cfg(all(feature = "http", feature = "mcp"))]
mod mcp;
#[cfg(any(feature = "mcp", feature = "kafka"))]
mod operations;
mod product_api;
mod routes;
mod schemas;
mod services;
#[cfg(feature = "sync")]
mod sync_workers;

#[cfg(feature = "http")]
use config::app_config::AppConfig;
#[cfg(feature = "http")]
use routes::get_routes;

#[tokio::main]
async fn main() {
    dotenv().ok();
    let observability_guard = init_tracing();
    let security = SecurityConfig::from_env();
    if let Err(e) = security.validate_runtime_safety() {
        error!(error = %e, "unsafe security configuration");
        std::process::exit(1);
    }

    let runtime_mode = match RuntimeMode::from_env() {
        Ok(mode) => mode,
        Err(e) => {
            error!(error = %e, "invalid runtime mode configuration");
            std::process::exit(1);
        }
    };
    let host_plan = RuntimeHostPlan::new(runtime_mode);

    #[cfg(feature = "sync")]
    if host_plan.has_sync_worker_with_listener_modules() {
        error!(
            modules = ?host_plan.module_names(),
            workers = ?host_plan.worker_module_names(),
            "SaaS sync workers must run in a worker-only process; deploy APPFW_RUNTIME_MODE=sync separately from the HTTP backend"
        );
        observability_guard.shutdown();
        std::process::exit(1);
    }

    if host_plan.has_multiple_worker_modules() {
        error!(
            modules = ?host_plan.module_names(),
            workers = ?host_plan.worker_module_names(),
            "multiple runtime worker modules are selected; run one worker mode per process until a worker supervisor is available"
        );
        observability_guard.shutdown();
        std::process::exit(1);
    }

    #[cfg(feature = "kafka")]
    if host_plan.runs_kafka_workers() {
        if let Err(e) = kafka::run_workers(&host_plan).await {
            error!(error = %e, "Kafka runtime worker host error");
            observability_guard.shutdown();
            std::process::exit(1);
        }

        observability_guard.shutdown();
        return;
    }

    #[cfg(feature = "sync")]
    if host_plan.runs_sync_workers() {
        if let Err(e) = sync_workers::run_workers(&host_plan).await {
            error!(error = %e, "SaaS sync runtime worker host error");
            observability_guard.shutdown();
            std::process::exit(1);
        }

        observability_guard.shutdown();
        return;
    }

    if host_plan.has_unsupported_worker_modules() {
        error!(
            modules = ?host_plan.module_names(),
            workers = ?host_plan.worker_module_names(),
            "this product backend does not yet provide runtime worker modules"
        );
        observability_guard.shutdown();
        std::process::exit(1);
    }

    #[cfg(feature = "http")]
    if host_plan.serves_http_listener() {
        let app_config = match AppConfig::init().await {
            Ok(config) => Arc::new(config),
            Err(e) => {
                error!(error = %e, "failed to initialize app config");
                std::process::exit(1);
            }
        };
        let app_state = match RuntimeAuthState::from_env() {
            Ok(state) => state,
            Err(e) => {
                error!(error = %e, "failed to initialize app state");
                std::process::exit(1);
            }
        };

        info!(auth_configured = true, "Okta configuration loaded");

        let cors = cors::get();

        let routes = match get_routes(
            cors,
            app_config.clone(),
            app_state.clone(),
            security,
            host_plan.mode().clone(),
        )
        .await
        {
            Ok(routes) => routes,
            Err(e) => {
                error!(error = %e, "failed to build routes");
                std::process::exit(1);
            }
        };

        let http_config = match RuntimeHttpServerConfig::from_env() {
            Ok(config) => config,
            Err(e) => {
                error!(error = %e, "invalid HTTP server configuration");
                std::process::exit(1);
            }
        };

        if let Err(e) = appfw_runtime::serve_http_router(routes, &http_config).await {
            error!(error = %e, "backend runtime host error");
            std::process::exit(1);
        }

        observability_guard.shutdown();
        return;
    }

    error!(
        modules = ?host_plan.module_names(),
        "this product backend has no enabled runtime ingress modules it can host"
    );
    observability_guard.shutdown();
    std::process::exit(1);
}
"#
    .to_string()
}

fn product_intake_backend_lib(intake: &ProductIntake) -> String {
    format!(
        r#"pub const APP_NAME: &str = "{app_name}";
pub const PRODUCT_SCHEMA: &str = "{schema}";
pub const BACKEND_PROVIDER: &str = "{provider}";
"#,
        app_name = rust_string_escape(&intake.app_name),
        schema = rust_string_escape(&intake.schema),
        provider = rust_string_escape(&intake.provider),
    )
}

fn product_intake_services_mod() -> String {
    r#"//! Product-owned service layer.
//!
//! Add durable business workflows here and call them from product-owned
//! handlers. Keep framework/runtime access behind `crate::product_api`.
"#
    .to_string()
}

fn product_intake_database_readme(intake: &ProductIntake) -> String {
    format!(
        r#"# {display_name} Database Package

This directory is the product-owned generated database package surface.

The selected provider is `{provider}` through data source `{data_source}`.
Generated package artifacts and product migrations belong under:

```text
database/_pkg/
```

Run database workflows from the product root after the model exists:

```bash
scripts/appfw product migrate doctor
scripts/appfw product migrate plan --json
scripts/appfw product migrate lint --phase all --json
scripts/appfw product migrate rollback-guide --json
```
"#,
        display_name = intake.display_name,
        provider = intake.provider,
        data_source = intake.data_source_name,
    )
}

fn product_intake_api_tests_cargo(target_root: &Path, framework_root: &Path) -> String {
    let api_tests_root = target_root.join("api_tests");
    let dependency_source = product_framework_dependency_source(framework_root);
    let appfw_test_dependency = framework_crate_dependency_line(
        &dependency_source,
        &api_tests_root,
        "appfw_test",
        "appfw-test",
        "appfw_test",
        &[],
    );
    format!(
        r#"[package]
name = "api_tests"
version = "0.1.0"
edition = "2021"

[dependencies]
anyhow = "1.0"
{appfw_test_dependency}
async-graphql = "7.0"
graphql_client = "0.14"
once_cell = "1.20"
reqwest = {{ version = "0.11", features = ["json"] }}
serde = {{ version = "1.0", features = ["derive"] }}
serde_json = "1.0"
tokio = {{ version = "1.28", features = ["full"] }}
"#,
        appfw_test_dependency = appfw_test_dependency,
    )
}

fn product_intake_api_tests_readme(intake: &ProductIntake) -> String {
    let model_source = if product_intake_is_legacy(intake) {
        "legacy-modernization"
    } else {
        "PoC-derived"
    };
    format!(
        r#"# {display_name} API Tests

Generated API scenario modules belong under `api_tests/src/schemas/{schema}`.
Create the {model_source} entity model first, then run:

```bash
scripts/appfw product generate
scripts/appfw product api-test
```
"#,
        display_name = intake.display_name,
        model_source = model_source,
        schema = intake.schema,
    )
}

fn product_intake_api_tests_mod(intake: &ProductIntake) -> String {
    format!(
        r#"// Generated scenario modules for `{schema}` appear here after the product model exists.
"#,
        schema = intake.schema,
    )
}

fn product_intake_rego_test_cargo(target_root: &Path, framework_root: &Path) -> String {
    let rego_root = target_root.join("rego_test");
    let dependency_source = product_framework_dependency_source(framework_root);
    let appfw_test_dependency = framework_crate_dependency_line(
        &dependency_source,
        &rego_root,
        "appfw_test",
        "appfw-test",
        "appfw_test",
        &[],
    );
    format!(
        r#"[package]
name = "rego_test"
version = "0.1.0"
edition = "2021"

[dependencies]
anyhow = "1.0"
{appfw_test_dependency}
serde_json = "1.0"
"#,
        appfw_test_dependency = appfw_test_dependency,
    )
}

fn product_intake_rego_test_readme(intake: &ProductIntake) -> String {
    let evidence = if product_intake_is_legacy(intake) {
        "legacy application analysis"
    } else {
        "PoC artifacts"
    };
    format!(
        r#"# {display_name} Policy Tests

Policy fixtures should be created from the product users, roles, workflows, and
data classifications discovered in the {evidence}.

Keep deny-by-default behavior explicit and add row-scope fixtures before release.
"#,
        display_name = intake.display_name,
        evidence = evidence,
    )
}

fn product_intake_rego_policy_test(intake: &ProductIntake) -> String {
    format!(
        r#"#[test]
fn product_policy_contract_is_named_for_schema() {{
    assert_eq!("{schema}", "{schema}");
}}
"#,
        schema = rust_string_escape(&intake.schema),
    )
}

fn product_appfw_script() -> String {
    r#"#!/usr/bin/env bash
set -euo pipefail

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
app_root="$(cd "$script_dir/.." && pwd)"

if [[ -z "${APPFW_FRAMEWORK_ROOT:-}" && -f "$app_root/.appfw/local.env" ]]; then
  # shellcheck disable=SC1091
  source "$app_root/.appfw/local.env"
fi

if [[ -n "${APPFW_FRAMEWORK_ROOT:-}" ]]; then
  framework_root="$APPFW_FRAMEWORK_ROOT"
elif [[ -f "$app_root/../app-framework/Cargo.toml" ]]; then
  framework_root="$(cd "$app_root/../app-framework" && pwd)"
elif [[ -f "$app_root/../../../Cargo.toml" && -d "$app_root/../../../app_gen/_templates" ]]; then
  framework_root="$(cd "$app_root/../../.." && pwd)"
else
  echo "Could not find app-framework; set APPFW_FRAMEWORK_ROOT." >&2
  exit 2
fi

export CARGO_TARGET_DIR="${APPFW_CARGO_TARGET_DIR:-$app_root/target/appfw-cargo}"

if [[ ! -f "$framework_root/Cargo.lock" ]]; then
  cat >&2 <<EOF
appfw: missing required framework Cargo.lock:
  $framework_root/Cargo.lock

The product wrapper runs appfw-cli with --locked for deterministic workflow
evidence. Generate and commit the framework workspace lockfile before running
this command:
  cd "$framework_root" && cargo generate-lockfile
EOF
  exit 2
fi

exec cargo run --locked --quiet --manifest-path "$framework_root/Cargo.toml" -p appfw-cli -- \
  --app-root "$app_root" \
  --framework-root "$framework_root" \
  "$@"
"#
    .to_string()
}

fn product_intake_compose_yaml(intake: &ProductIntake) -> String {
    let mut services = vec![product_intake_provider_compose_service(intake)];
    if intake.kafka_client {
        services.push(product_intake_kafka_compose_service(intake));
    }
    services.push(product_intake_backend_compose_service(intake));
    let mut volumes = vec![format!(
        "  {}-data:",
        product_intake_compose_service_name(intake)
    )];
    if intake.kafka_client {
        volumes.push("  kafka-data:".to_string());
    }
    format!(
        r#"version: '3.8'

# Product-intake local development topology.
# Generated from .appfw/manifest.yaml and .appfw/model/data_sources/_res.yaml.
# Edit product topology/config, then rerun generation after the entity model exists.
services:
{services}

volumes:
{volumes}

networks:
  app-network:
    driver: bridge
"#,
        services = services.join("\n"),
        volumes = volumes.join("\n"),
    )
}

fn product_intake_provider_compose_service(intake: &ProductIntake) -> String {
    let service = product_intake_compose_service_name(intake);
    match intake.provider.as_str() {
        "PostgreSQL" => format!(
            r#"  {service}:
    image: postgres:14
    container_name: ${{APP_STACK_NAME:-{app}}}-{service}
    restart: always
    ports:
      - "${{APP_POSTGRES_HOST_PORT:-5432}}:5432"
    environment:
      POSTGRES_USER: postgres
      POSTGRES_PASSWORD: postgres
      POSTGRES_DB: {db}
    volumes:
      - type: volume
        source: {service}-data
        target: /var/lib/postgresql/data
    healthcheck:
      test: ["CMD-SHELL", "pg_isready -U postgres"]
      interval: 5s
      timeout: 5s
      retries: 5
    networks:
      - app-network
"#,
            app = intake.app_name,
            db = intake.app_name.replace('-', "_"),
        ),
        "MongoDB" => format!(
            r#"  {service}:
    image: mongo:7
    container_name: ${{APP_STACK_NAME:-{app}}}-{service}
    restart: always
    ports:
      - "${{APP_MONGO_HOST_PORT:-27017}}:27017"
    environment:
      MONGO_INITDB_ROOT_USERNAME: "${{MONGO_SERVICE_ACCOUNT_NAME:-mongo}}"
      MONGO_INITDB_ROOT_PASSWORD: "${{MONGO_SERVICE_ACCOUNT_PASS:-mongo}}"
    volumes:
      - type: volume
        source: {service}-data
        target: /data/db
    healthcheck:
      test: ["CMD", "mongosh", "--quiet", "--eval", "db.runCommand({{ ping: 1 }}).ok"]
      interval: 10s
      timeout: 5s
      retries: 10
      start_period: 10s
    networks:
      - app-network
"#,
            app = intake.app_name,
        ),
        "MsSqlServer" => format!(
            r#"  {service}:
    image: mcr.microsoft.com/mssql/server:2022-latest
    container_name: ${{APP_STACK_NAME:-{app}}}-{service}
    restart: always
    ports:
      - "${{APP_MSSQL_HOST_PORT:-1433}}:1433"
    environment:
      ACCEPT_EULA: "Y"
      MSSQL_SA_PASSWORD: "${{MSSQL_SERVICE_ACCOUNT_PASS:-YourStrong!Passw0rd}}"
      MSSQL_PID: Developer
    volumes:
      - type: volume
        source: {service}-data
        target: /var/opt/mssql
    healthcheck:
      test: ["CMD-SHELL", "/opt/mssql-tools18/bin/sqlcmd -S localhost -U sa -P \"$$MSSQL_SA_PASSWORD\" -C -Q 'SELECT 1' || exit 1"]
      interval: 10s
      timeout: 5s
      retries: 10
      start_period: 30s
    networks:
      - app-network
"#,
            app = intake.app_name,
        ),
        "Snowflake" => format!(
            r#"  {service}:
    image: localstack/snowflake:${{LOCALSTACK_SNOWFLAKE_TAG:-stable}}
    container_name: ${{APP_STACK_NAME:-{app}}}-{service}
    profiles: ["snowflake"]
    ports:
      - "127.0.0.1:4566:4566"
      - "127.0.0.1:4510-4559:4510-4559"
      - "127.0.0.1:443:443"
    environment:
      - LOCALSTACK_AUTH_TOKEN=${{LOCALSTACK_AUTH_TOKEN:?Set LOCALSTACK_AUTH_TOKEN to start LocalStack Snowflake}}
      - SF_DEFAULT_USER=${{SNOWFLAKE_SERVICE_ACCOUNT_NAME:-test}}
      - SF_DEFAULT_PASSWORD=${{SNOWFLAKE_SERVICE_ACCOUNT_PASS:-test}}
    volumes:
      - type: volume
        source: {service}-data
        target: /var/lib/localstack
    networks:
      app-network:
        aliases:
          - snowflake.localhost.localstack.cloud
"#,
            app = intake.app_name,
        ),
        _ => String::new(),
    }
}

fn product_intake_compose_service_name(intake: &ProductIntake) -> &'static str {
    match intake.provider.as_str() {
        "PostgreSQL" => "postgres",
        "MongoDB" => "mongo",
        "MsSqlServer" => "mssql",
        "Snowflake" => "snowflake",
        _ => "data",
    }
}

fn product_intake_kafka_compose_service(intake: &ProductIntake) -> String {
    format!(
        r#"  kafka:
    image: bitnami/kafka:3.7
    container_name: ${{APP_STACK_NAME:-{app}}}-kafka
    restart: always
    ports:
      - "${{APP_KAFKA_HOST_PORT:-9092}}:9092"
    environment:
      - KAFKA_ENABLE_KRAFT=yes
      - KAFKA_CFG_NODE_ID=1
      - KAFKA_CFG_PROCESS_ROLES=broker,controller
      - KAFKA_CFG_CONTROLLER_QUORUM_VOTERS=1@kafka:9093
      - KAFKA_CFG_LISTENERS=PLAINTEXT://:9092,CONTROLLER://:9093
      - KAFKA_CFG_ADVERTISED_LISTENERS=PLAINTEXT://localhost:9092
      - KAFKA_CFG_CONTROLLER_LISTENER_NAMES=CONTROLLER
      - ALLOW_PLAINTEXT_LISTENER=yes
    volumes:
      - type: volume
        source: kafka-data
        target: /bitnami/kafka
    networks:
      - app-network
"#,
        app = intake.app_name,
    )
}

fn product_intake_backend_compose_service(intake: &ProductIntake) -> String {
    let service = product_intake_compose_service_name(intake);
    let kafka_dep = if intake.kafka_client {
        "      - kafka\n"
    } else {
        ""
    };
    let command = if intake.provider == "MsSqlServer" {
        r#"sh -c 'apt-get update &&
             apt-get install -y --no-install-recommends ca-certificates curl gnupg unixodbc unixodbc-dev tdsodbc freetds-bin freetds-common &&
             curl -fsSL https://packages.microsoft.com/keys/microsoft.asc | gpg --dearmor -o /usr/share/keyrings/microsoft-prod.gpg &&
             echo "deb [arch=amd64 signed-by=/usr/share/keyrings/microsoft-prod.gpg] https://packages.microsoft.com/debian/12/prod bookworm main" > /etc/apt/sources.list.d/mssql-release.list &&
             apt-get update &&
             ACCEPT_EULA=Y apt-get install -y --no-install-recommends msodbcsql18 &&
             MS_DRIVER_PATH="$(ls /opt/microsoft/msodbcsql18/lib64/libmsodbcsql-18*.so.* 2>/dev/null | head -1)" &&
             printf "%s\n" "[FreeTDS]" "Description=FreeTDS Driver" "Driver=/usr/lib/x86_64-linux-gnu/odbc/libtdsodbc.so" "Setup=/usr/lib/x86_64-linux-gnu/odbc/libtdsodbc.so" "UsageCount=1" "" "[ODBC Driver 18 for SQL Server]" "Description=Microsoft ODBC Driver 18 for SQL Server" "Driver=${MS_DRIVER_PATH}" "UsageCount=1" > /etc/odbcinst.ini &&
             cargo run --bin backend'"#
    } else {
        "cargo run --bin backend"
    };
    format!(
        r#"  backend:
    image: rust:latest
    container_name: ${{APP_STACK_NAME:-{app}}}-backend
    working_dir: /app
    volumes:
      - type: bind
        source: ./backend
        target: /app
    ports:
      - "127.0.0.1:${{API_PORT:-8080}}:${{API_PORT:-8080}}"
    command: {command}
    depends_on:
      - {service}
{kafka_dep}    environment:
      - API_HOST=${{API_HOST:-0.0.0.0}}
      - API_PORT=${{API_PORT:-8080}}
      - ENV_NAME=${{ENV_NAME:-compose}}
      - APP_DATA_SOURCE_NAME={data_source}
      - APP_MCP_ENABLED={mcp}
      - APP_KAFKA_ENABLED={kafka}
    networks:
      - app-network
"#,
        app = intake.app_name,
        service = service,
        kafka_dep = kafka_dep,
        command = command,
        data_source = intake.data_source_name,
        mcp = intake.mcp_server,
        kafka = intake.kafka_client,
    )
}

fn product_intake_data_sources_yaml(intake: &ProductIntake) -> String {
    let (local_host, compose_host, port, security_profile, tls_mode) =
        match intake.provider.as_str() {
            "PostgreSQL" => ("localhost", "postgres", 5432, "local_dev", "disabled"),
            "MongoDB" => ("localhost", "mongo", 27017, "local_dev", "disabled"),
            "MsSqlServer" => ("127.0.0.1", "mssql", 1433, "local_dev", "disabled"),
            "Snowflake" => (
                "account_identifier.snowflakecomputing.com",
                "http://snowflake.localhost.localstack.cloud:4566",
                443,
                "managed",
                "verify_full",
            ),
            _ => ("localhost", "data", 0, "local_dev", "disabled"),
        };
    format!(
        r#"- name: {data_source}
  data_source_type: {provider}
  is_system_schema_host: true
  description: Primary provider selected during product intake.
  environments:
  - name: local
    db_host: {local_host}
    db_name: {app_name}
    db_port: {port}
    security_profile: {security_profile}
    tls_mode: {tls_mode}
  - name: compose
    db_host: {compose_host}
    db_name: {app_name}
    db_port: {port}
    security_profile: local_dev
    tls_mode: disabled
"#,
        data_source = intake.data_source_name,
        provider = intake.provider,
        local_host = local_host,
        compose_host = compose_host,
        app_name = intake.app_name.replace('-', "_"),
        port = port,
        security_profile = security_profile,
        tls_mode = tls_mode,
    )
}

fn product_intake_system_schema_yaml(intake: &ProductIntake) -> String {
    format!(
        r#"id: ff252a31-08c1-41be-addb-9aab41082a2e
name: system
description: System schema
data_source_name: {data_source}
"#,
        data_source = intake.data_source_name,
    )
}

fn product_intake_schema_yaml(intake: &ProductIntake) -> String {
    let description = if product_intake_is_legacy(intake) {
        "Product schema inferred from legacy application evidence."
    } else {
        "Product schema inferred from PoC artifacts."
    };
    format!(
        r#"id: {id}
name: {schema}
description: {description}
data_source_name: {data_source}
"#,
        id = deterministic_product_schema_id(&intake.app_name, &intake.schema),
        schema = intake.schema,
        description = description,
        data_source = intake.data_source_name,
    )
}

fn product_intake_frontend_files(
    target_root: &Path,
    framework_root: &Path,
    intake: &ProductIntake,
) -> Vec<(String, String)> {
    let frontend_root = target_root.join("frontend");
    let pds_components_source = relative_path_between(
        &frontend_root,
        &framework_root.join("appfw_ui/pds_health/components/src"),
    )
    .replace('\\', "/");
    let repo_root = relative_path_between(&frontend_root, framework_root).replace('\\', "/");

    vec![
        (
            "frontend/README.md".to_string(),
            product_intake_frontend_readme(intake),
        ),
        (
            "frontend/.gitignore".to_string(),
            "node_modules/\ndist/\nplaywright-report/\ntest-results/\n".to_string(),
        ),
        (
            "frontend/package.json".to_string(),
            product_intake_frontend_package(intake),
        ),
        (
            "frontend/index.html".to_string(),
            product_intake_frontend_index(intake),
        ),
        (
            "frontend/tsconfig.json".to_string(),
            product_intake_frontend_tsconfig(&pds_components_source),
        ),
        (
            "frontend/.appfw-ui/scaffold-manifest.json".to_string(),
            product_intake_frontend_scaffold_manifest(intake),
        ),
        (
            "frontend/.appfw-ui/ownership.json".to_string(),
            product_intake_frontend_ownership_manifest(intake),
        ),
        (
            "frontend/vite.config.ts".to_string(),
            product_intake_frontend_vite_config(&repo_root, &pds_components_source, &intake.schema),
        ),
        (
            "frontend/src/generated/appfw-ui-contract.ts".to_string(),
            product_intake_frontend_generated_contract(intake),
        ),
        (
            "frontend/src/main.tsx".to_string(),
            product_intake_frontend_main(intake),
        ),
        (
            "frontend/src/styles.css".to_string(),
            product_intake_frontend_styles(),
        ),
        (
            "frontend/scripts/check-scaffold.mjs".to_string(),
            product_intake_frontend_check_script(intake),
        ),
    ]
}

fn product_intake_frontend_readme(intake: &ProductIntake) -> String {
    let evidence = if product_intake_is_legacy(intake) {
        "legacy UI screenshots, route inventory, and workflow evidence"
    } else {
        "PoC UI artifact"
    };
    let model_source = if product_intake_is_legacy(intake) {
        "legacy-modernization"
    } else {
        "PoC-derived"
    };
    format!(
        r#"# {display_name} Frontend

This frontend was created because product intake selected UI mode `{ui}`.

Use the {evidence} for workflow intent and App Framework frontend
standards for implementation shape. Replace the placeholder screen after the
{model_source} entity model and generated UI contract exist.

```bash
npm install
npm run appfw:check
npm run typecheck
npm run build
```

`npm run build` emits the deployable SPA bundle to `../backend/product_dist`.
The generated backend serves that bundle at `/` in the default one-image
deployment topology when `APP_PRODUCT_UI_ENABLED=true`.
"#,
        display_name = intake.display_name,
        evidence = evidence,
        model_source = model_source,
        ui = intake.ui,
    )
}

fn product_intake_frontend_package(intake: &ProductIntake) -> String {
    format!(
        r#"{{
  "name": "{name}-frontend",
  "version": "0.1.0",
  "private": true,
  "type": "module",
  "scripts": {{
    "dev": "vite",
    "build": "tsc --noEmit && vite build",
    "typecheck": "tsc --noEmit",
    "appfw:check": "node scripts/check-scaffold.mjs"
  }},
  "dependencies": {{
    "react": "^18.3.1",
    "react-dom": "^18.3.1"
  }},
  "devDependencies": {{
    "@types/react": "^18.3.5",
    "@types/react-dom": "^18.3.0",
    "@vitejs/plugin-react": "^4.7.0",
    "typescript": "^5.5.4",
    "vite": "^6.4.3"
  }},
  "overrides": {{
    "esbuild": "^0.28.1"
  }}
}}
"#,
        name = intake.app_name,
    )
}

fn product_intake_frontend_index(intake: &ProductIntake) -> String {
    format!(
        r#"<!doctype html>
<html lang="en">
  <head>
    <meta charset="UTF-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1.0" />
    <title>{title}</title>
  </head>
  <body>
    <div id="root"></div>
    <script type="module" src="/src/main.tsx"></script>
  </body>
</html>
"#,
        title = html_escape(&intake.display_name),
    )
}

fn product_intake_frontend_tsconfig(pds_components_source: &str) -> String {
    r#"{
  "compilerOptions": {
    "target": "ES2020",
    "useDefineForClassFields": true,
    "lib": ["DOM", "DOM.Iterable", "ES2020"],
    "allowJs": false,
    "skipLibCheck": true,
    "esModuleInterop": true,
    "allowSyntheticDefaultImports": true,
    "strict": true,
    "forceConsistentCasingInFileNames": true,
    "module": "ESNext",
    "moduleResolution": "Node",
    "resolveJsonModule": true,
    "isolatedModules": true,
    "noEmit": true,
    "jsx": "react-jsx",
    "baseUrl": ".",
    "paths": {
      "@appfw/pds-health-components": ["__PDS_COMPONENTS_SOURCE__/index.ts"],
      "@appfw/pds-health-components/styles.css": ["__PDS_COMPONENTS_SOURCE__/styles.css"],
      "react-dom": ["node_modules/@types/react-dom/index.d.ts"]
    }
  },
  "include": ["src"]
}
"#
    .replace(
        "__PDS_COMPONENTS_SOURCE__",
        &js_string_escape(pds_components_source),
    )
}

fn product_intake_frontend_scaffold_manifest(intake: &ProductIntake) -> String {
    let manifest = json!({
        "version": 1,
        "profile": "product-intake",
        "ui_mode": &intake.ui,
        "app": {
            "name": &intake.app_name,
            "display_name": &intake.display_name,
            "schema": &intake.schema,
            "provider": &intake.provider,
        },
        "design_system": {
            "brand": "PDS Health",
            "source": "appfw_ui/pds_health",
            "tokens": "appfw_ui/pds_health/tokens",
            "components": "appfw_ui/pds_health/components",
            "package": "@appfw/pds-health-components",
            "style_import": "@appfw/pds-health-components/styles.css",
            "provenance": "framework-owned design-system source; product-owned frontend consumes local framework aliases for starter UI",
            "starter_examples": [
                "overlay",
                "analytics",
                "data-grid",
                "generated-form"
            ]
        },
        "generated_contract": "src/generated/appfw-ui-contract.ts",
        "checks": {
            "offline": "npm run appfw:check",
            "evidence": "target/appfw/frontend-scaffold-check.json"
        }
    });
    serde_json::to_string_pretty(&manifest).unwrap_or_else(|_| "{}".to_string()) + "\n"
}

fn product_intake_frontend_ownership_manifest(intake: &ProductIntake) -> String {
    let manifest = json!({
        "version": 1,
        "profile": "product-intake",
        "app_name": &intake.app_name,
        "owners": [
            {
                "path": "src/generated/**",
                "owner": "app-framework-generator",
                "edit_policy": "regenerate, do not hand-edit"
            },
            {
                "path": "src/**",
                "owner": "product-frontend",
                "edit_policy": "product-owned implementation"
            },
            {
                "path": ".appfw-ui/**",
                "owner": "app-framework",
                "edit_policy": "update through framework scaffold upgrades"
            }
        ]
    });
    serde_json::to_string_pretty(&manifest).unwrap_or_else(|_| "{}".to_string()) + "\n"
}

fn product_intake_frontend_generated_contract(intake: &ProductIntake) -> String {
    let schema_label = titleize_identifier(&intake.schema);
    format!(
        r#"export const appUiContract = {{
  appName: '{app_name}',
  displayName: '{display_name}',
  schema: '{schema}',
  schemaLabel: '{schema_label}',
  provider: '{provider}',
  modelStatus: 'intake'
}} as const;

export type AppUiContract = typeof appUiContract;
"#,
        app_name = js_string_escape(&intake.app_name),
        display_name = js_string_escape(&intake.display_name),
        schema = js_string_escape(&intake.schema),
        schema_label = js_string_escape(&schema_label),
        provider = js_string_escape(&intake.provider),
    )
}

fn product_intake_frontend_vite_config(
    repo_root: &str,
    pds_components_source: &str,
    schema_name: &str,
) -> String {
    r#"import { defineConfig } from 'vite';
import react from '@vitejs/plugin-react';
import { fileURLToPath } from 'node:url';

const repoRoot = fileURLToPath(new URL('__REPO_ROOT__/', import.meta.url));
const frontendRoot = fileURLToPath(new URL('./', import.meta.url));
const pdsComponentsRoot = fileURLToPath(new URL('__PDS_COMPONENTS_SOURCE__/', import.meta.url));

export default defineConfig({
  plugins: [react()],
  build: {
    target: 'esnext',
    outDir: '../backend/product_dist',
    emptyOutDir: true
  },
  resolve: {
    alias: {
      '@appfw/pds-health-components/styles.css': `${pdsComponentsRoot}styles.css`,
      '@appfw/pds-health-components': `${pdsComponentsRoot}index.ts`
    },
    dedupe: ['react', 'react-dom']
  },
  server: {
    fs: {
      allow: [frontendRoot, repoRoot]
    },
    port: 5173,
    proxy: {
      '/admin': 'http://127.0.0.1:8080',
      '/system': 'http://127.0.0.1:8080',
      '/__SCHEMA_ROUTE__': 'http://127.0.0.1:8080'
    }
  }
});
"#
    .replace("__REPO_ROOT__", &js_string_escape(repo_root))
    .replace(
        "__PDS_COMPONENTS_SOURCE__",
        &js_string_escape(pds_components_source),
    )
    .replace("__SCHEMA_ROUTE__", &js_string_escape(schema_name))
}

fn product_intake_frontend_main(intake: &ProductIntake) -> String {
    let model_source = if product_intake_is_legacy(intake) {
        "legacy-modernization"
    } else {
        "PoC-derived"
    };
    format!(
        r##"import React, {{ useState }} from 'react';
import {{ createRoot }} from 'react-dom/client';
import {{
  AppShell,
  Badge,
  Button,
  ChartLegend,
  ChartShell,
  CommandPalette,
  DataGridDensityControl,
  DataGridPagination,
  DataGridShell,
  DataGridToolbar,
  DateField,
  Dialog,
  FormLayout,
  KpiTile,
  MetricTrend,
  PageHeader,
  SelectField,
  Surface,
  SwitchField,
  TextField,
  ValidationSummary,
  type CommandPaletteItem,
  type PdsDataGridColumn,
  type PdsDensity
}} from '@appfw/pds-health-components';
import {{ appUiContract }} from './generated/appfw-ui-contract';
import './styles.css';

type StarterRow = Record<string, unknown> & {{
  id: string;
  request: string;
  owner: string;
  state: string;
  due: string;
}};

const starterRows: StarterRow[] = [
  {{ id: 'REQ-101', request: 'Model review', owner: 'Platform team', state: 'Ready', due: 'Jun 18' }},
  {{ id: 'REQ-102', request: 'Policy mapping', owner: 'Security team', state: 'Needs review', due: 'Jun 20' }},
  {{ id: 'REQ-103', request: 'Release evidence', owner: 'Product team', state: 'Draft', due: 'Jun 24' }}
];

const starterColumns: PdsDataGridColumn<StarterRow>[] = [
  {{ key: 'request', header: 'Request', width: '32%' }},
  {{ key: 'owner', header: 'Owner', width: '26%' }},
  {{ key: 'state', header: 'State', width: '22%', render: (row) => <Badge tone={{row.state === 'Ready' ? 'success' : row.state === 'Needs review' ? 'warning' : 'neutral'}}>{{row.state}}</Badge> }},
  {{ key: 'due', header: 'Due', width: '20%', align: 'end' }}
];

const requestTypeOptions = [
  {{ value: 'intake', label: 'Intake review' }},
  {{ value: 'model', label: 'Model decision' }},
  {{ value: 'release', label: 'Release evidence' }}
];

const starterValidation = {{
  requestType: ['Select a generated type before submit.']
}};

const starterLegend = [
  {{ id: 'ready', label: 'Ready', tone: 'success' as const, value: '72%' }},
  {{ id: 'review', label: 'Needs review', tone: 'warning' as const, value: '18%' }},
  {{ id: 'draft', label: 'Draft', tone: 'neutral' as const, value: '10%' }}
];

const workspaceCommands: CommandPaletteItem[] = [
  {{
    id: 'workspace:model',
    group: 'Workspace',
    label: 'Review model',
    detail: appUiContract.schema,
    href: '#model'
  }},
  {{
    id: 'workspace:contract',
    group: 'Workspace',
    label: 'Open generated contract',
    detail: 'src/generated/appfw-ui-contract.ts',
    href: '#contract'
  }},
  {{
    id: 'workspace:evidence',
    group: 'Release',
    label: 'Review frontend evidence',
    detail: 'target/appfw/frontend-scaffold-check.json',
    href: '#evidence'
  }},
  {{
    id: 'workspace:components',
    group: 'Design system',
    label: 'Review starter components',
    detail: 'Forms, grid, overlays, and analytics',
    href: '#components'
  }}
];

function App() {{
  const [reviewOpen, setReviewOpen] = useState(false);
  const [density, setDensity] = useState<PdsDensity>('compact');
  const [approvalRequired, setApprovalRequired] = useState(true);

  return (
    <AppShell
      brand={{(
        <div className="app-brand">
          <strong>{{appUiContract.displayName}}</strong>
          <span>{{appUiContract.schemaLabel}} workspace</span>
        </div>
      )}}
      navigation={{(
        <div className="app-nav" aria-label="Workspace sections">
          <a href="#model">Model</a>
          <a href="#contract">Contract</a>
          <a href="#evidence">Evidence</a>
          <a href="#components">Components</a>
        </div>
      )}}
      topBar={{(
        <div className="app-topbar">
          <CommandPalette
            items={{workspaceCommands}}
            triggerLabel="Search workspace"
            searchPlaceholder="Search workspace commands"
          />
        </div>
      )}}
      footer={{(
        <div className="app-shell-footer">
          <span>Generated scaffold</span>
          <strong>{{appUiContract.provider}}</strong>
        </div>
      )}}
    >
      <PageHeader
        eyebrow={{appUiContract.schemaLabel}}
        title={{appUiContract.displayName}}
        subtitle="{model_source} product workspace"
        actions={{<Badge tone="accent">{{appUiContract.provider}}</Badge>}}
      />
      <div className="app-status-grid" aria-label="Workspace status">
        <Surface id="model" title="Model" subtitle="Application data model" density="compact">
          <p>{{appUiContract.schema}}</p>
          <Badge>{{appUiContract.modelStatus}}</Badge>
        </Surface>
        <Surface id="contract" title="Contract" subtitle="Generated UI boundary" density="compact">
          <p>src/generated/appfw-ui-contract.ts</p>
          <Badge tone="success">Ready</Badge>
        </Surface>
        <Surface id="evidence" title="Evidence" subtitle="Frontend scaffold check" density="compact">
          <p>target/appfw/frontend-scaffold-check.json</p>
          <Badge>Local</Badge>
        </Surface>
      </div>
      <div className="app-kpi-grid" id="components" aria-label="Starter component examples">
        <KpiTile
          label="Design system"
          value="PDS"
          detail="Framework-owned source"
          tone="accent"
          trend={{<MetricTrend value="Ready" label="starter" tone="positive" direction="up" />}}
        />
        <KpiTile
          label="Generated contract"
          value={{appUiContract.schemaLabel}}
          detail="Replace after model generation"
          tone="success"
          trend={{<MetricTrend value="v1" label="UI boundary" tone="accent" direction="flat" />}}
        />
        <KpiTile
          label="Release evidence"
          value="Local"
          detail="Retain scaffold check output"
          tone="neutral"
          trend={{<MetricTrend value="4" label="starter examples" tone="neutral" direction="flat" />}}
        />
      </div>

      <div className="app-work-grid">
        <Surface title="Generated form starter" subtitle="Token-backed field states and validation" density="compact">
          <ValidationSummary validation={{starterValidation}} />
          <FormLayout
            columns="two"
            footer={{(
              <>
                <Button variant="quiet">Reset</Button>
                <Button variant="primary" onClick={{() => setReviewOpen(true)}}>Review draft</Button>
              </>
            )}}
          >
            <SelectField
              label="Request type"
              required
              error="Select a generated type before submit."
              placeholder="Select a type"
              options={{requestTypeOptions}}
            />
            <TextField label="Owner" defaultValue="Product team" hint="Use product-owned copy and generated field hints." />
            <DateField label="Due date" defaultValue="2026-06-24" hint="Native date controls preserve keyboard and mobile behavior." />
            <SwitchField
              id="approval-required"
              label="Require approval"
              checked={{approvalRequired}}
              onCheckedChange={{setApprovalRequired}}
              detail="Use explicit state for governed workflow choices."
            />
          </FormLayout>
        </Surface>

        <Surface title="Work queue starter" subtitle="Compact grid with user-selectable density" density="compact">
          <DataGridToolbar
            ariaLabel="Starter queue controls"
            search={{<input className="app-search" type="search" aria-label="Search starter queue" placeholder="Search queue" />}}
            summary={{(
              <>
                <strong>{{starterRows.length}} rows</strong>
                <span>{{density}} density</span>
              </>
            )}}
            actions={{<DataGridDensityControl value={{density}} onChange={{setDensity}} label="Density" />}}
            density={{density}}
          />
          <DataGridShell
            columns={{starterColumns}}
            rows={{starterRows}}
            rowKey="id"
            ariaLabel="Starter work queue"
            density={{density}}
          />
          <DataGridPagination pageSize={{25}} pageIndex={{1}} startRow={{1}} endRow={{starterRows.length}} totalRows={{starterRows.length}} responseMs={{32}} />
        </Surface>
      </div>

      <ChartShell
        title="Readiness starter"
        subtitle="Chart-engine-neutral analytics chrome"
        footer={{<ChartLegend items={{starterLegend}} />}}
      >
        <div className="app-chart-bars" aria-label="Starter readiness distribution">
          <span className="app-chart-bar is-ready"><b>Ready</b></span>
          <span className="app-chart-bar is-review"><b>Needs review</b></span>
          <span className="app-chart-bar is-draft"><b>Draft</b></span>
        </div>
      </ChartShell>

      <Dialog
        open={{reviewOpen}}
        title="Review starter draft"
        description="Use shared overlays for confirmation, review, and supporting workflow panels."
        onClose={{() => setReviewOpen(false)}}
        closeLabel="Close review dialog"
        footer={{<Button variant="primary" onClick={{() => setReviewOpen(false)}}>Close</Button>}}
      >
        <p>This placeholder is product-owned. Replace it with workflow-specific review content once the model and generated UI contract are ready.</p>
      </Dialog>
    </AppShell>
  );
}}

createRoot(document.getElementById('root') as HTMLElement).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>
);
"##,
        model_source = jsx_text_escape(model_source),
    )
}

fn titleize_identifier(value: &str) -> String {
    let words = value
        .split(|ch: char| !ch.is_ascii_alphanumeric())
        .filter(|word| !word.is_empty())
        .map(|word| {
            let mut chars = word.chars();
            let Some(first) = chars.next() else {
                return String::new();
            };
            format!(
                "{}{}",
                first.to_ascii_uppercase(),
                chars.as_str().to_ascii_lowercase()
            )
        })
        .filter(|word| !word.is_empty())
        .collect::<Vec<_>>();
    if words.is_empty() {
        "Product".to_string()
    } else {
        words.join(" ")
    }
}

fn product_intake_frontend_styles() -> String {
    r#"@import "@appfw/pds-health-components/styles.css";

body {
  margin: 0;
  background: var(--pds-gradient-canvas);
  color: var(--pds-color-text-default);
  font-family: var(--pds-font-family-sans);
}

.pds-app-shell__main {
  display: grid;
  align-content: start;
  gap: var(--pds-space-5);
}

.app-brand,
.app-nav,
.app-shell-footer {
  display: grid;
  gap: var(--pds-space-2);
}

.app-brand strong {
  color: var(--pds-color-text-default);
  font-size: var(--pds-font-size-base);
}

.app-brand span,
.app-shell-footer {
  color: var(--pds-color-text-muted);
  font-size: var(--pds-font-size-sm);
}

.app-nav a {
  padding: var(--pds-space-2) var(--pds-space-3);
  border-radius: var(--pds-radius-md);
  color: var(--pds-color-text-muted);
  font-size: var(--pds-font-size-md);
  font-weight: var(--pds-font-weight-semibold);
  text-decoration: none;
}

.app-nav a:hover,
.app-nav a:focus-visible {
  background: var(--pds-color-state-accent-soft);
  color: var(--pds-color-brand-blue-deep);
}

.app-topbar {
  width: min(100%, 420px);
}

.app-topbar .pds-command-palette,
.app-topbar .pds-command-palette__trigger {
  width: 100%;
}

.app-status-grid {
  display: grid;
  grid-template-columns: repeat(3, minmax(0, 1fr));
  gap: var(--pds-space-4);
}

.app-status-grid p {
  min-height: 40px;
  margin: 0 0 var(--pds-space-3);
  color: var(--pds-color-text-muted);
  font-size: var(--pds-font-size-md);
  overflow-wrap: anywhere;
}

.app-kpi-grid,
.app-work-grid {
  display: grid;
  gap: var(--pds-space-4);
}

.app-kpi-grid {
  grid-template-columns: repeat(3, minmax(0, 1fr));
}

.app-work-grid {
  grid-template-columns: minmax(0, 1fr) minmax(0, 1.2fr);
  align-items: start;
}

.app-search {
  width: 100%;
  min-height: 36px;
  padding: 0 var(--pds-space-3);
  border: 1px solid var(--pds-color-border-default);
  border-radius: var(--pds-radius-md);
  background: var(--pds-gradient-control);
  color: var(--pds-color-text-default);
  font: inherit;
}

.app-search:focus {
  border-color: transparent;
  background:
    var(--pds-gradient-control) padding-box,
    var(--pds-gradient-border-accent) border-box;
  box-shadow: var(--pds-shadow-control);
  outline: 3px solid var(--pds-color-state-focus);
  outline-offset: 2px;
}

.app-chart-bars {
  display: grid;
  gap: var(--pds-space-3);
}

.app-chart-bar {
  min-height: 34px;
  display: flex;
  align-items: center;
  padding-inline: var(--pds-space-3);
  border: 1px solid var(--pds-color-border-default);
  border-radius: var(--pds-radius-md);
  background: var(--pds-color-surface-panel-soft);
  color: var(--pds-color-text-default);
}

.app-chart-bar b {
  font-size: var(--pds-font-size-sm);
}

.app-chart-bar.is-ready {
  width: 72%;
  border-color: var(--pds-color-state-success);
  background: var(--pds-color-state-success-bg);
}

.app-chart-bar.is-review {
  width: 44%;
  background: color-mix(in srgb, var(--pds-color-state-gold) 14%, var(--pds-color-surface-panel-soft));
}

.app-chart-bar.is-draft {
  width: 28%;
}

@media (max-width: 860px) {
  .app-topbar .pds-command-palette,
  .app-topbar .pds-command-palette__trigger {
    width: 100%;
    max-width: none;
  }

  .app-status-grid,
  .app-kpi-grid,
  .app-work-grid {
    grid-template-columns: minmax(0, 1fr);
  }

  .app-chart-bar {
    width: 100%;
  }
}
"#
    .to_string()
}

fn product_intake_frontend_check_script(intake: &ProductIntake) -> String {
    let script = r#"import fs from 'node:fs';

const displayName = '__DISPLAY_NAME__';
const allowedProductIdentity = [
  '__APP_NAME__',
  '__DISPLAY_NAME__',
  '__SCHEMA__',
  '__SCHEMA_LABEL__'
].map((value) => value.toLowerCase()).filter(Boolean);
const uniqueAllowedProductIdentity = [...new Set(allowedProductIdentity)];
const required = [
  '.appfw-ui/ownership.json',
  '.appfw-ui/scaffold-manifest.json',
  'src/generated/appfw-ui-contract.ts',
  'src/main.tsx',
  'src/styles.css',
  'vite.config.ts',
  'tsconfig.json'
];

const residueScanFiles = [
  'README.md',
  'package.json',
  'index.html',
  'vite.config.ts',
  'tsconfig.json',
  '.appfw-ui/ownership.json',
  '.appfw-ui/scaffold-manifest.json',
  'src/generated/appfw-ui-contract.ts',
  'src/main.tsx',
  'src/styles.css'
];

const residueWords = new Set([
  'crm',
  'account',
  'accounts',
  'activity',
  'activities',
  'contact',
  'contacts',
  'lead',
  'leads',
  'opportunity',
  'opportunities',
  'pipeline',
  'pipelines',
  'quote',
  'quotes'
]);
const residuePhrases = ['customer relationship management'];
const ignoredTechnicalFragments = [
  'atlassian/pipelines/agent/build',
  'atlassian\\pipelines\\agent\\build'
];

function scaffoldUrl(relativePath) {
  return new URL(`../${relativePath}`, import.meta.url);
}

function normalizeResidueLine(line) {
  let normalized = line.toLowerCase();
  normalized = removePathLikeResidueFragments(normalized);
  for (const allowed of uniqueAllowedProductIdentity) {
    normalized = normalized.split(allowed).join(' ');
  }
  for (const fragment of ignoredTechnicalFragments) {
    normalized = normalized.split(fragment).join(' ');
  }
  return normalized;
}

function removePathLikeResidueFragments(line) {
  return line
    .split(/\s+/)
    .map((token) => {
      const trimmed = token.replace(/^[\\"'([{]+|[\\\\"')},;]+$/g, '');
      if (
        trimmed.includes('../') ||
        trimmed.includes('..\\\\') ||
        trimmed.startsWith('/') ||
        trimmed.includes('/private/') ||
        trimmed.includes('/users/') ||
        trimmed.includes('appfw_ui/pds_health') ||
        trimmed.includes('@appfw/pds-health-components')
      ) {
        return ' ';
      }
      return token;
    })
    .join(' ');
}

function residueMatches(relativePath, text) {
  const matches = [];
  const lines = text.split(/\r?\n/);
  lines.forEach((line, index) => {
    const normalized = normalizeResidueLine(line);
    const words = new Set(normalized.split(/[^a-z0-9]+/).filter(Boolean));
    for (const term of residueWords) {
      if (words.has(term)) {
        matches.push({
          path: relativePath,
          line: index + 1,
          term,
          excerpt: line.trim().slice(0, 160)
        });
      }
    }
    for (const phrase of residuePhrases) {
      if (normalized.includes(phrase)) {
        matches.push({
          path: relativePath,
          line: index + 1,
          term: phrase,
          excerpt: line.trim().slice(0, 160)
        });
      }
    }
  });
  return matches;
}

const missing = required.filter((path) => !fs.existsSync(scaffoldUrl(path)));
const pdsChecks = [
  {
    id: 'pds-component-import',
    path: 'src/main.tsx',
    pattern: '@appfw/pds-health-components'
  },
  {
    id: 'pds-command-palette',
    path: 'src/main.tsx',
    pattern: 'CommandPalette'
  },
  {
    id: 'pds-app-shell',
    path: 'src/main.tsx',
    pattern: 'AppShell'
  },
  {
    id: 'pds-overlay-example',
    path: 'src/main.tsx',
    pattern: 'Dialog'
  },
  {
    id: 'pds-analytics-example',
    path: 'src/main.tsx',
    pattern: 'KpiTile'
  },
  {
    id: 'pds-data-grid-example',
    path: 'src/main.tsx',
    pattern: 'DataGridShell'
  },
  {
    id: 'pds-generated-form-example',
    path: 'src/main.tsx',
    pattern: 'FormLayout'
  },
  {
    id: 'pds-style-import',
    path: 'src/styles.css',
    pattern: '@appfw/pds-health-components/styles.css'
  },
  {
    id: 'pds-tsconfig-alias',
    path: 'tsconfig.json',
    pattern: '@appfw/pds-health-components'
  },
  {
    id: 'pds-vite-alias',
    path: 'vite.config.ts',
    pattern: '@appfw/pds-health-components'
  },
  {
    id: 'pds-manifest-package',
    path: '.appfw-ui/scaffold-manifest.json',
    pattern: '@appfw/pds-health-components'
  }
];
const missingPdsChecks = pdsChecks.filter((check) => {
  const url = scaffoldUrl(check.path);
  return !fs.existsSync(url) || !fs.readFileSync(url, 'utf8').includes(check.pattern);
});
const residue = [];
for (const relativePath of residueScanFiles) {
  const url = scaffoldUrl(relativePath);
  if (!fs.existsSync(url)) {
    continue;
  }
  residue.push(...residueMatches(relativePath, fs.readFileSync(url, 'utf8')));
}

const artifactUrl = new URL('../target/appfw/frontend-scaffold-check.json', import.meta.url);
fs.mkdirSync(new URL('../target/appfw/', import.meta.url), { recursive: true });

const report = {
  command: 'appfw:check',
  ok: missing.length === 0 && missingPdsChecks.length === 0 && residue.length === 0,
  generated_at_utc: new Date().toISOString(),
  display_name: displayName,
  required_files: required,
  missing_files: missing,
  design_system: {
    ok: missingPdsChecks.length === 0,
    checks: pdsChecks,
    missing: missingPdsChecks
  },
  residue_check: {
    ok: residue.length === 0,
    scan_files: residueScanFiles,
    terms: [...residueWords, ...residuePhrases],
    allowed_product_identity: uniqueAllowedProductIdentity,
    matches: residue
  },
  artifact: 'target/appfw/frontend-scaffold-check.json'
};

fs.writeFileSync(artifactUrl, `${JSON.stringify(report, null, 2)}\n`);

if (!report.ok) {
  if (missing.length > 0) {
    console.error(`Missing frontend scaffold files: ${missing.join(', ')}`);
  }
  if (residue.length > 0) {
    console.error(`Frontend scaffold contains sample residue; see ${artifactUrl.pathname}`);
  }
  if (missingPdsChecks.length > 0) {
    console.error(`Frontend scaffold is missing PDS design-system wiring; see ${artifactUrl.pathname}`);
  }
  process.exit(1);
}

console.log(`${displayName} frontend scaffold OK`);
console.log(`Evidence: ${artifactUrl.pathname}`);
"#;
    script
        .replace("__APP_NAME__", &js_string_escape(&intake.app_name))
        .replace("__DISPLAY_NAME__", &js_string_escape(&intake.display_name))
        .replace("__SCHEMA__", &js_string_escape(&intake.schema))
        .replace(
            "__SCHEMA_LABEL__",
            &js_string_escape(&titleize_identifier(&intake.schema)),
        )
}

fn product_intake_frontend_residue_report(
    product_root: &Path,
    written_files: &[String],
    intake: &ProductIntake,
) -> Result<Value, String> {
    let allowed_identity = product_frontend_allowed_identity_terms(intake);
    let scan_files = written_files
        .iter()
        .filter(|relative| product_frontend_residue_scannable(relative))
        .cloned()
        .collect::<Vec<_>>();
    let mut matches = Vec::new();
    for relative in &scan_files {
        let path = product_root.join(relative);
        if !path.is_file() {
            continue;
        }
        let contents = fs::read_to_string(&path)
            .map_err(|err| format!("failed to read {}: {err}", path.display()))?;
        for (line_index, line) in contents.lines().enumerate() {
            for term in product_frontend_residue_matches(line, &allowed_identity) {
                matches.push(json!({
                    "path": relative,
                    "line": line_index + 1,
                    "term": term,
                    "excerpt": line.trim().chars().take(160).collect::<String>(),
                }));
            }
        }
    }

    let design_checks = product_frontend_design_system_checks();
    let mut missing_design_checks = Vec::new();
    for (id, relative, pattern) in &design_checks {
        let path = product_root.join(relative);
        let present = path
            .is_file()
            .then(|| fs::read_to_string(&path).ok())
            .flatten()
            .is_some_and(|contents| contents.contains(pattern));
        if !present {
            missing_design_checks.push(json!({
                "id": id,
                "path": relative,
                "pattern": pattern,
            }));
        }
    }
    let design_ok = missing_design_checks.is_empty();

    let terms = product_frontend_residue_terms()
        .iter()
        .chain(product_frontend_residue_phrases().iter())
        .copied()
        .collect::<Vec<_>>();

    Ok(json!({
        "command": "frontend-residue-check",
        "ok": matches.is_empty() && design_ok,
        "generated_at_utc": Utc::now().to_rfc3339(),
        "scan_root": "frontend",
        "files_scanned": scan_files.len(),
        "scan_files": scan_files,
        "design_system": {
            "ok": design_ok,
            "checks": design_checks
                .iter()
                .map(|(id, path, pattern)| json!({
                    "id": id,
                    "path": path,
                    "pattern": pattern,
                }))
                .collect::<Vec<_>>(),
            "missing": missing_design_checks
        },
        "terms": terms,
        "allowed_product_identity": allowed_identity,
        "matches": matches,
        "artifact": "target/appfw/frontend-residue-check.json"
    }))
}

fn product_frontend_design_system_checks() -> Vec<(&'static str, &'static str, &'static str)> {
    vec![
        (
            "pds-component-import",
            "frontend/src/main.tsx",
            "@appfw/pds-health-components",
        ),
        (
            "pds-command-palette",
            "frontend/src/main.tsx",
            "CommandPalette",
        ),
        ("pds-app-shell", "frontend/src/main.tsx", "AppShell"),
        ("pds-overlay-example", "frontend/src/main.tsx", "Dialog"),
        ("pds-analytics-example", "frontend/src/main.tsx", "KpiTile"),
        (
            "pds-data-grid-example",
            "frontend/src/main.tsx",
            "DataGridShell",
        ),
        (
            "pds-generated-form-example",
            "frontend/src/main.tsx",
            "FormLayout",
        ),
        (
            "pds-style-import",
            "frontend/src/styles.css",
            "@appfw/pds-health-components/styles.css",
        ),
        (
            "pds-tsconfig-alias",
            "frontend/tsconfig.json",
            "@appfw/pds-health-components",
        ),
        (
            "pds-vite-alias",
            "frontend/vite.config.ts",
            "@appfw/pds-health-components",
        ),
        (
            "pds-manifest-package",
            "frontend/.appfw-ui/scaffold-manifest.json",
            "@appfw/pds-health-components",
        ),
    ]
}

fn product_frontend_residue_scannable(relative: &str) -> bool {
    relative.starts_with("frontend/")
        && relative != "frontend/scripts/check-scaffold.mjs"
        && [
            ".css", ".html", ".js", ".json", ".md", ".mjs", ".ts", ".tsx",
        ]
        .iter()
        .any(|suffix| relative.ends_with(suffix))
}

fn product_frontend_allowed_identity_terms(intake: &ProductIntake) -> Vec<String> {
    let mut allowed = vec![
        intake.app_name.to_ascii_lowercase(),
        intake.display_name.to_ascii_lowercase(),
        intake.schema.to_ascii_lowercase(),
        titleize_identifier(&intake.schema).to_ascii_lowercase(),
    ];
    allowed.retain(|value| !value.trim().is_empty());
    allowed.sort();
    allowed.dedup();
    allowed
}

fn product_frontend_residue_matches(line: &str, allowed_identity: &[String]) -> Vec<&'static str> {
    let normalized = product_frontend_normalized_residue_line(line, allowed_identity);
    let words = normalized
        .split(|ch: char| !ch.is_ascii_alphanumeric())
        .filter(|word| !word.is_empty())
        .collect::<BTreeSet<_>>();

    let mut matches = Vec::new();
    for term in product_frontend_residue_terms() {
        if words.contains(term) {
            matches.push(*term);
        }
    }
    for phrase in product_frontend_residue_phrases() {
        if normalized.contains(phrase) {
            matches.push(*phrase);
        }
    }
    matches
}

fn product_frontend_normalized_residue_line(line: &str, allowed_identity: &[String]) -> String {
    let mut normalized = line.to_ascii_lowercase();
    normalized = product_frontend_remove_path_like_residue_fragments(&normalized);
    for allowed in allowed_identity {
        normalized = normalized.replace(allowed, " ");
    }
    for fragment in product_frontend_ignored_technical_residue_fragments() {
        normalized = normalized.replace(fragment, " ");
    }
    normalized
}

fn product_frontend_remove_path_like_residue_fragments(line: &str) -> String {
    line.split_whitespace()
        .map(|token| {
            let trimmed = token.trim_matches(|ch: char| {
                matches!(
                    ch,
                    '"' | '\'' | '(' | ')' | '[' | ']' | '{' | '}' | ',' | ';'
                )
            });
            if trimmed.contains("../")
                || trimmed.contains("..\\")
                || trimmed.starts_with('/')
                || trimmed.contains("/private/")
                || trimmed.contains("/users/")
                || trimmed.contains("appfw_ui/pds_health")
                || trimmed.contains("@appfw/pds-health-components")
            {
                " "
            } else {
                token
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn product_frontend_ignored_technical_residue_fragments() -> &'static [&'static str] {
    &[
        "atlassian/pipelines/agent/build",
        "atlassian\\pipelines\\agent\\build",
    ]
}

fn product_frontend_residue_terms() -> &'static [&'static str] {
    &[
        "crm",
        "account",
        "accounts",
        "activity",
        "activities",
        "contact",
        "contacts",
        "lead",
        "leads",
        "opportunity",
        "opportunities",
        "pipeline",
        "pipelines",
        "quote",
        "quotes",
    ]
}

fn product_frontend_residue_phrases() -> &'static [&'static str] {
    &["customer relationship management"]
}

fn deterministic_product_schema_id(app_name: &str, schema: &str) -> String {
    let digest = Sha256::digest(format!("appfw-product-intake:{app_name}:{schema}").as_bytes());
    let hex = format!("{digest:x}");
    format!(
        "{}-{}-4{}-8{}-{}",
        &hex[0..8],
        &hex[8..12],
        &hex[13..16],
        &hex[17..20],
        &hex[20..32],
    )
}

fn yaml_scalar(value: &str) -> String {
    if value
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || ch == '_' || ch == '-' || ch == '.' || ch == '/')
    {
        value.to_string()
    } else {
        format!("'{}'", value.replace('\'', "''"))
    }
}

fn yaml_optional_scalar(value: Option<&str>) -> String {
    value.map(yaml_scalar).unwrap_or_else(|| "null".to_string())
}

fn toml_escape(value: &str) -> String {
    value.replace('\\', "\\\\").replace('"', "\\\"")
}

fn rust_string_escape(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
}

fn js_string_escape(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace('\'', "\\'")
        .replace('\n', "\\n")
}

fn html_escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

fn jsx_text_escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('{', "&#123;")
        .replace('}', "&#125;")
}

fn emit_new_profiles(repo_root: &Path, json_output: bool) -> Result<(), String> {
    let mut profiles = Vec::new();
    for profile in new_profiles() {
        let manifest = read_new_profile_manifest(repo_root, profile)?;
        let profile_root = new_profile_root(repo_root, profile.name);
        profiles.push(NewProfileInfo {
            name: manifest.name,
            description: manifest.description,
            profile_path: new_profile_manifest_path(repo_root, profile.name)
                .display()
                .to_string(),
            template_path: repo_root.join(&manifest.template).display().to_string(),
            overlay_path: profile_root.join(&manifest.overlay).display().to_string(),
            verification: manifest.verification,
            frontend_scaffold: manifest.frontend_scaffold,
        });
    }
    let starter_modes = new_starter_modes();
    let report = json!({
        "command": "new --list-profiles",
        "ok": true,
        "profiles": profiles,
        "starter_modes": starter_modes,
    });
    emit_value(json_output, &report, || {
        let mut out = "Available appfw product new profiles:".to_string();
        for profile in &profiles {
            out.push_str(&format!("\n  {} - {}", profile.name, profile.description));
            out.push_str(&format!("\n      manifest: {}", profile.profile_path));
            if !profile.verification.is_empty() {
                out.push_str("\n      verification:");
                for command in &profile.verification {
                    out.push_str(&format!("\n        {command}"));
                }
            }
        }
        out.push_str("\n\nAvailable appfw product new starter modes:");
        for starter in &starter_modes {
            out.push_str(&format!("\n  {} - {}", starter.name, starter.description));
            out.push_str(&format!("\n      command: {}", starter.command));
            if !starter.next_commands.is_empty() {
                out.push_str("\n      next commands:");
                for command in &starter.next_commands {
                    out.push_str(&format!("\n        {command}"));
                }
            }
        }
        out
    })
}

fn new_starter_modes() -> Vec<NewStarterMode> {
    vec![NewStarterMode {
        name: "product-intake".to_string(),
        description:
            "Product-neutral PoC or legacy intake shell with governed analysis and modeling evidence."
                .to_string(),
        command: "scripts/appfw product new <target> --profile product-intake --json".to_string(),
        source_kinds: vec!["poc".to_string(), "legacy".to_string()],
        required_options: vec![
            "--app-name".to_string(),
            "--display-name".to_string(),
            "--schema".to_string(),
        ],
        optional_options: vec![
            "--source-kind poc|legacy".to_string(),
            "--provider PostgreSQL".to_string(),
            "--mcp yes|no".to_string(),
            "--kafka yes|no".to_string(),
            "--ui none|scaffold|enterprise".to_string(),
            "--poc-source <path>".to_string(),
            "--legacy-source <path>".to_string(),
            "--data-artifact <path>".to_string(),
            "--ui-artifact <path>".to_string(),
        ],
        next_commands: vec![
            "scripts/appfw product analyze --summary --json".to_string(),
            "scripts/appfw product propose-model --summary --json".to_string(),
            "scripts/appfw product model-status --json".to_string(),
            "scripts/appfw product scaffold-model --dry-run --json".to_string(),
            "scripts/appfw product validate --json".to_string(),
        ],
        retained_artifacts: vec![
            ".appfw/poc-intake.yaml".to_string(),
            ".appfw/poc-analysis.yaml or .appfw/legacy-analysis.yaml".to_string(),
            ".appfw/model-proposal.yaml".to_string(),
            "target/appfw/product-analysis.json".to_string(),
            "target/appfw/model-proposal.json".to_string(),
            "target/appfw/model-status.json".to_string(),
            "target/appfw/model-scaffold.json".to_string(),
        ],
        verification: vec![
            "scripts/appfw framework intake-proof --json".to_string(),
            "scripts/appfw product validate --json".to_string(),
            "scripts/appfw product handoff --json".to_string(),
        ],
    }]
}

fn intake_proof_command(
    roots: &CommandRoots,
    json_output: bool,
    args: &[String],
) -> Result<(), String> {
    let mut source_kind = "poc".to_string();
    let mut keep = false;
    let mut idx = 0;
    while idx < args.len() {
        match args[idx].as_str() {
            "--source-kind" => {
                source_kind = canonical_product_source_kind(
                    args.get(idx + 1)
                        .ok_or_else(|| "--source-kind requires a value".to_string())?,
                )?
                .to_string();
                idx += 2;
            }
            "--legacy" => {
                source_kind = "legacy".to_string();
                idx += 1;
            }
            "--keep" => {
                keep = true;
                idx += 1;
            }
            value => return Err(format!("unknown intake-proof option: {value}")),
        }
    }

    let run_id = Utc::now().timestamp_nanos_opt().unwrap_or_default();
    let temp_root = env::temp_dir().join(format!("appfw-product-intake-proof-{run_id}"));
    let product_root = temp_root.join("claims-insight");
    let source_root = temp_root.join(if source_kind == "legacy" {
        "legacy-source"
    } else {
        "poc-source"
    });
    fs::create_dir_all(&source_root)
        .map_err(|err| format!("failed to create {}: {err}", source_root.display()))?;

    let (data_artifact, ui_artifact, fixture_files) = if source_kind == "legacy" {
        write_intake_proof_legacy_source(&source_root)?
    } else {
        write_intake_proof_poc_source(&source_root)?
    };

    let intake = complete_product_intake(
        ProductIntakeArgs {
            source_kind: Some(source_kind.clone()),
            app_name: Some("claims-insight".to_string()),
            display_name: Some("Claims Insight".to_string()),
            schema: Some("claims".to_string()),
            provider: Some("PostgreSQL".to_string()),
            mcp_server: Some("false".to_string()),
            kafka_client: Some("false".to_string()),
            ui: Some("enterprise".to_string()),
            poc_source: Some(if source_kind == "legacy" {
                "../legacy-source".to_string()
            } else {
                "../poc-source".to_string()
            }),
            data_artifact: Some(data_artifact),
            ui_artifact: Some(ui_artifact),
        },
        true,
    )?;
    fs::create_dir_all(&product_root)
        .map_err(|err| format!("failed to create {}: {err}", product_root.display()))?;
    let written_files =
        write_product_intake_scaffold(&roots.framework_root, &product_root, &intake)?;
    let frontend_residue =
        product_intake_frontend_residue_report(&product_root, &written_files, &intake)?;
    let product_residue_path = product_root.join("target/appfw/frontend-residue-check.json");
    write_json_file(&product_residue_path, &frontend_residue)?;
    let scaffold_check_result = run_product_frontend_scaffold_check(&product_root);
    let (frontend_scaffold_command, frontend_scaffold_error) = match scaffold_check_result {
        Ok(command) => (command, None),
        Err(err) => ("node scripts/check-scaffold.mjs".to_string(), Some(err)),
    };
    let product_scaffold_check_path =
        product_root.join("frontend/target/appfw/frontend-scaffold-check.json");
    let mut frontend_scaffold_check =
        read_json_file(&product_scaffold_check_path).unwrap_or_else(|err| {
            json!({
                "command": "appfw:check",
                "ok": false,
                "error": err,
                "artifact": "target/appfw/frontend-scaffold-check.json"
            })
        });
    if let Some(error) = &frontend_scaffold_error {
        if let Some(report) = frontend_scaffold_check.as_object_mut() {
            report
                .entry("error".to_string())
                .or_insert_with(|| json!(error));
        }
    }
    let frontend_scaffold_check_ok = frontend_scaffold_check
        .get("ok")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let frontend_scaffold_design_system_ok = frontend_scaffold_check
        .get("design_system")
        .and_then(|value| value.get("ok"))
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let frontend_residue_ok = frontend_residue
        .get("ok")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let frontend_residue_matches = frontend_residue
        .get("matches")
        .and_then(Value::as_array)
        .map(Vec::len)
        .unwrap_or(0);
    let frontend_design_system_ok = frontend_residue
        .get("design_system")
        .and_then(|value| value.get("ok"))
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let product_roots = CommandRoots::split_layout(&product_root, &roots.framework_root);
    let analysis = analyze_product_intake(&product_roots)?;
    let proposal = propose_model_from_analysis(&product_roots)?;
    let status = build_model_status_report(&product_roots)?;
    let product_status_path = product_root.join("target/appfw/model-status.json");
    write_json_file(&product_status_path, &status)?;

    let artifact_dir = roots
        .framework_root
        .join("target/appfw/product-intake-proof");
    fs::create_dir_all(&artifact_dir)
        .map_err(|err| format!("failed to create {}: {err}", artifact_dir.display()))?;
    let report_path = roots
        .framework_root
        .join("target/appfw/product-intake-proof.json");
    let analysis_copy = artifact_dir.join("product-analysis.json");
    let proposal_copy = artifact_dir.join("model-proposal.json");
    let status_copy = artifact_dir.join("model-status.json");
    let proposal_yaml_copy = artifact_dir.join("model-proposal.yaml");
    let frontend_residue_copy = artifact_dir.join("frontend-residue-check.json");
    let frontend_scaffold_check_copy = artifact_dir.join("frontend-scaffold-check.json");
    write_json_file(&analysis_copy, &analysis)?;
    write_json_file(&proposal_copy, &proposal)?;
    write_json_file(&status_copy, &status)?;
    write_json_file(&frontend_scaffold_check_copy, &frontend_scaffold_check)?;
    fs::copy(&product_residue_path, &frontend_residue_copy).map_err(|err| {
        format!(
            "failed to copy frontend residue check {} to {}: {err}",
            product_residue_path.display(),
            frontend_residue_copy.display()
        )
    })?;
    fs::copy(
        product_root.join(".appfw/model-proposal.yaml"),
        &proposal_yaml_copy,
    )
    .map_err(|err| format!("failed to copy {}: {err}", proposal_yaml_copy.display()))?;

    let retained = keep;
    let cleanup_status = if keep {
        "kept"
    } else {
        fs::remove_dir_all(&temp_root)
            .map_err(|err| format!("failed to remove {}: {err}", temp_root.display()))?;
        "removed"
    };

    let report = json!({
        "command": "intake-proof",
        "ok": frontend_residue_ok && frontend_scaffold_check_ok,
        "source_kind": source_kind,
        "generated_at_utc": Utc::now().to_rfc3339(),
        "artifact": relative_path(&roots.framework_root, &report_path),
        "artifact_dir": relative_path(&roots.framework_root, &artifact_dir),
        "retained_temp_workspace": retained,
        "temp_workspace_status": cleanup_status,
        "temp_workspace": temp_root.display().to_string(),
        "product_root": product_root.display().to_string(),
        "source_root": source_root.display().to_string(),
        "fixture_files": fixture_files,
        "written_files_count": written_files.len(),
        "steps": [
            {
                "name": "source-fixture",
                "ok": true
            },
            {
                "name": "product-intake-scaffold",
                "ok": true,
                "profile": "product-intake"
            },
            {
                "name": "frontend-residue-check",
                "ok": frontend_residue_ok,
                "artifact": "target/appfw/product-intake-proof/frontend-residue-check.json",
                "matches": frontend_residue_matches,
                "design_system_ok": frontend_design_system_ok
            },
            {
                "name": "frontend-scaffold-check",
                "ok": frontend_scaffold_check_ok,
                "artifact": "target/appfw/product-intake-proof/frontend-scaffold-check.json",
                "command": frontend_scaffold_command,
                "design_system_ok": frontend_scaffold_design_system_ok,
                "error": frontend_scaffold_error
            },
            {
                "name": "analyze",
                "ok": analysis.get("ok").and_then(Value::as_bool).unwrap_or(false),
                "artifact": "target/appfw/product-analysis.json"
            },
            {
                "name": "propose-model",
                "ok": proposal.get("ok").and_then(Value::as_bool).unwrap_or(false),
                "artifact": "target/appfw/model-proposal.json",
                "writes_final_model": false
            },
            {
                "name": "model-status",
                "ok": status.get("ok").and_then(Value::as_bool).unwrap_or(false),
                "artifact": "target/appfw/model-status.json",
                "phase": status.get("phase").cloned().unwrap_or_else(|| json!("unknown"))
            }
        ],
        "retained_artifacts": {
            "analysis_json": relative_path(&roots.framework_root, &analysis_copy),
            "proposal_json": relative_path(&roots.framework_root, &proposal_copy),
            "status_json": relative_path(&roots.framework_root, &status_copy),
            "proposal_yaml": relative_path(&roots.framework_root, &proposal_yaml_copy),
            "frontend_residue_check": relative_path(&roots.framework_root, &frontend_residue_copy),
            "frontend_scaffold_check": relative_path(&roots.framework_root, &frontend_scaffold_check_copy)
        },
        "frontend_residue_check": {
            "ok": frontend_residue_ok,
            "artifact": relative_path(&roots.framework_root, &frontend_residue_copy),
            "files_scanned": frontend_residue
                .get("files_scanned")
                .cloned()
                .unwrap_or_else(|| json!(0)),
            "matches": frontend_residue_matches
        },
        "frontend_scaffold_check": {
            "ok": frontend_scaffold_check_ok,
            "artifact": relative_path(&roots.framework_root, &frontend_scaffold_check_copy),
            "design_system_ok": frontend_scaffold_design_system_ok,
            "error": frontend_scaffold_error
        },
        "proposal_summary": {
            "candidate_entities": proposal
                .get("proposal")
                .and_then(|value| value.get("candidate_entities"))
                .cloned()
                .unwrap_or_else(|| json!([])),
            "unassigned_properties": proposal
                .get("proposal")
                .and_then(|value| value.get("unassigned_properties"))
                .cloned()
                .unwrap_or_else(|| json!([])),
            "frontend_views": proposal
                .get("proposal")
                .and_then(|value| value.get("frontend_views"))
                .cloned()
                .unwrap_or_else(|| json!([])),
            "requires_human_review": true,
            "writes_final_model": false
        },
        "next_commands": [
            "scripts/appfw product new <target> --profile product-intake ... --json",
            "cd <target>",
            "scripts/appfw product analyze --summary --json",
            "scripts/appfw product propose-model --summary --json",
            "scripts/appfw product model-status --json",
            "review .appfw/model-proposal.yaml before writing .appfw/model"
        ]
    });

    write_json_file(&report_path, &report)?;
    if !frontend_residue_ok {
        return Err(format!(
            "product-intake frontend residue check failed; see {}",
            report_path.display()
        ));
    }
    if !frontend_scaffold_check_ok {
        return Err(format!(
            "product-intake frontend scaffold check failed; see {}",
            report_path.display()
        ));
    }

    emit_value(json_output, &report, || {
        format!(
            "Product intake proof: OK\nArtifact: {}",
            report_path.display()
        )
    })
}

fn write_intake_proof_poc_source(
    source_root: &Path,
) -> Result<(String, String, Vec<String>), String> {
    let csv = source_root.join("claims.csv");
    let html = source_root.join("prototype.html");
    fs::write(
        &csv,
        "Claim ID,Member Name,Claim Status,Total Paid\n1,Ada,Open,42.50\n",
    )
    .map_err(|err| format!("failed to write {}: {err}", csv.display()))?;
    fs::write(
        &html,
        r#"<html><head><title>Claims Dashboard</title></head><body><h2>Claim Work Queue</h2><form><input name="claim_id"><select name="claim_status"></select></form><table><tr><th>Total Paid</th></tr></table></body></html>"#,
    )
    .map_err(|err| format!("failed to write {}: {err}", html.display()))?;
    Ok((
        "../poc-source/claims.csv".to_string(),
        "../poc-source/prototype.html".to_string(),
        vec!["claims.csv".to_string(), "prototype.html".to_string()],
    ))
}

fn write_intake_proof_legacy_source(
    source_root: &Path,
) -> Result<(String, String, Vec<String>), String> {
    let sln = source_root.join("Claims.sln");
    let controller = source_root.join("ClaimsController.cs");
    let sql = source_root.join("procedures.sql");
    fs::write(&sln, "Microsoft Visual Studio Solution File\n")
        .map_err(|err| format!("failed to write {}: {err}", sln.display()))?;
    fs::write(
        &controller,
        "public class ClaimsController { public void GetClaimQueue() {} }\n",
    )
    .map_err(|err| format!("failed to write {}: {err}", controller.display()))?;
    fs::write(
        &sql,
        "CREATE PROCEDURE claims_refresh AS SELECT claim_id, claim_status FROM claims;\n",
    )
    .map_err(|err| format!("failed to write {}: {err}", sql.display()))?;
    Ok((
        "../legacy-source/procedures.sql".to_string(),
        "../legacy-source/ClaimsController.cs".to_string(),
        vec![
            "Claims.sln".to_string(),
            "ClaimsController.cs".to_string(),
            "procedures.sql".to_string(),
        ],
    ))
}

fn write_json_file(path: &Path, value: &Value) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|err| format!("failed to create {}: {err}", parent.display()))?;
    }
    fs::write(
        path,
        serde_json::to_string_pretty(value).map_err(|err| err.to_string())? + "\n",
    )
    .map_err(|err| format!("failed to write {}: {err}", path.display()))
}

fn read_json_file(path: &Path) -> Result<Value, String> {
    let contents = fs::read_to_string(path)
        .map_err(|err| format!("failed to read {}: {err}", path.display()))?;
    serde_json::from_str(&contents)
        .map_err(|err| format!("failed to parse {}: {err}", path.display()))
}

fn run_product_frontend_scaffold_check(product_root: &Path) -> Result<String, String> {
    let frontend_root = product_root.join("frontend");
    let command_display = "node scripts/check-scaffold.mjs".to_string();
    let output = Command::new("node")
        .current_dir(&frontend_root)
        .arg("scripts/check-scaffold.mjs")
        .output()
        .map_err(|err| {
            format!(
                "failed to run frontend scaffold check in {}: {err}",
                frontend_root.display()
            )
        })?;
    if output.status.success() {
        Ok(command_display)
    } else {
        Err(format!(
            "frontend scaffold check failed in {}\nstdout:\n{}\nstderr:\n{}",
            frontend_root.display(),
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        ))
    }
}

fn golden_downstream_command(
    roots: &CommandRoots,
    json_output: bool,
    args: &[String],
) -> Result<(), String> {
    let mut profile_filter: Option<String> = None;
    let mut execute = false;
    let mut workspace_root: Option<PathBuf> = None;
    let mut idx = 0;
    while idx < args.len() {
        match args[idx].as_str() {
            "--profile" => {
                profile_filter = Some(
                    args.get(idx + 1)
                        .ok_or_else(|| "--profile requires a profile name".to_string())?
                        .clone(),
                );
                idx += 2;
            }
            "--execute" => {
                execute = true;
                idx += 1;
            }
            "--workspace" => {
                workspace_root = Some(PathBuf::from(
                    args.get(idx + 1)
                        .ok_or_else(|| "--workspace requires a path".to_string())?,
                ));
                idx += 2;
            }
            value => return Err(format!("unknown golden-downstream option: {value}")),
        }
    }
    if workspace_root.is_some() && !execute {
        return Err("--workspace requires --execute".to_string());
    }

    let report_dir = roots.app_root.join("target/appfw");
    let artifact_path = report_dir.join("golden-downstream.json");
    let execution_root = workspace_root
        .map(|path| normalize_path(&absolutize(path, &roots.app_root)))
        .unwrap_or_else(|| report_dir.join("golden-downstream-work"));
    fs::create_dir_all(&report_dir)
        .map_err(|err| format!("failed to create {}: {err}", report_dir.display()))?;
    if execute {
        fs::create_dir_all(&execution_root).map_err(|err| {
            format!(
                "failed to create golden downstream workspace {}: {err}",
                execution_root.display()
            )
        })?;
    }

    let required_frontend_commands = ["npm run appfw:check", "npm run typecheck", "npm run build"];
    let required_profile_verification = new_profiles()
        .iter()
        .map(|profile| {
            (
                profile.name,
                golden_downstream_required_profile_commands(profile.name),
            )
        })
        .collect::<BTreeMap<_, _>>();
    let mut profile_reports = Vec::new();
    let mut failed_checks = Vec::new();

    for profile in new_profiles() {
        if profile_filter
            .as_deref()
            .is_some_and(|name| name != profile.name)
        {
            continue;
        }

        let manifest = read_new_profile_manifest(&roots.framework_root, profile)?;
        let profile_root = new_profile_root(&roots.framework_root, profile.name);
        let profile_path = new_profile_manifest_path(&roots.framework_root, profile.name);
        let template_path = roots.framework_root.join(&manifest.template);
        let overlay_path = profile_root.join(&manifest.overlay);

        let required_profile_commands = golden_downstream_required_profile_commands(profile.name);
        let missing_profile_commands = required_profile_commands
            .iter()
            .filter(|command| {
                !manifest
                    .verification
                    .iter()
                    .any(|item| item.as_str() == **command)
            })
            .map(|command| command.to_string())
            .collect::<Vec<_>>();
        for command in &missing_profile_commands {
            failed_checks.push(format!(
                "profile `{}` missing verification command `{command}`",
                profile.name
            ));
        }

        let frontend_report = manifest.frontend_scaffold.as_ref().map(|frontend| {
            let missing_frontend_commands = required_frontend_commands
                .iter()
                .filter(|command| {
                    !frontend
                        .verification
                        .iter()
                        .any(|item| item.as_str() == **command)
                })
                .map(|command| command.to_string())
                .collect::<Vec<_>>();
            for command in &missing_frontend_commands {
                failed_checks.push(format!(
                    "profile `{}` missing frontend verification command `{command}`",
                    profile.name
                ));
            }

            json!({
                "root": frontend.root,
                "scaffold_manifest": frontend.scaffold_manifest,
                "ownership_manifest": frontend.ownership_manifest,
                "generated_contract": frontend.generated_contract,
                "check_script": frontend.check_script,
                "package_check": frontend.package_check,
                "verification": frontend.verification,
                "release_evidence": frontend.release_evidence,
                "missing_required_verification": missing_frontend_commands,
            })
        });

        let disposable_execution = if execute {
            let execution =
                execute_golden_downstream_profile(roots, profile.name, &execution_root)?;
            if execution.get("ok").and_then(Value::as_bool) != Some(true) {
                failed_checks.push(format!(
                    "profile `{}` disposable downstream execution failed",
                    profile.name
                ));
            }
            Some(execution)
        } else {
            None
        };
        let disposable_target = format!("$TMPDIR/appfw-golden-{}", profile.name);
        let mut disposable_ci_lane = vec![
            format!(
                "scripts/appfw product new \"{disposable_target}\" --from current --profile {} --generate --json",
                profile.name
            ),
            format!("cd \"{disposable_target}\""),
            "scripts/appfw product validate --json".to_string(),
            "scripts/appfw product harness-check --json".to_string(),
            "scripts/appfw product generate".to_string(),
            "scripts/appfw product generate --check --json".to_string(),
            if profile.name == "second-consumer" {
                "scripts/appfw product test --fast".to_string()
            } else {
                "scripts/appfw product test".to_string()
            },
        ];
        if manifest.frontend_scaffold.is_some() {
            disposable_ci_lane.extend([
                "scripts/appfw product frontend-test --json".to_string(),
                "scripts/appfw product release-check --json".to_string(),
                "scripts/appfw product upgrade --check --json".to_string(),
            ]);
        }
        disposable_ci_lane.push("scripts/appfw product handoff --json".to_string());

        profile_reports.push(json!({
            "name": manifest.name,
            "description": manifest.description,
            "profile_path": relative_path(&roots.framework_root, &profile_path),
            "template_path": relative_path(&roots.framework_root, &template_path),
            "overlay_path": relative_path(&roots.framework_root, &overlay_path),
            "verification": manifest.verification,
            "required_verification": required_profile_commands,
            "missing_required_verification": missing_profile_commands,
            "frontend_scaffold": frontend_report,
            "disposable_ci_lane": {
                "status": "specified",
                "commands": disposable_ci_lane,
                "note": "Run this lane in CI or an approved disposable workspace when write-heavy downstream proof is in scope."
            },
            "disposable_execution": disposable_execution
        }));
    }

    if profile_reports.is_empty() {
        return Err(format!(
            "no golden downstream profile matched `{}`",
            profile_filter.unwrap_or_else(|| "<all>".to_string())
        ));
    }

    let execution_summary = if execute {
        json!({
            "mode": "disposable-ci-execution",
            "destructive": true,
            "workspace_root": relative_path(&roots.app_root, &execution_root),
            "full_disposable_ci_status": if failed_checks.is_empty() { "passed" } else { "failed" },
            "full_disposable_ci_reason": "The disposable downstream lane was executed because --execute was supplied."
        })
    } else {
        json!({
            "mode": "metadata-and-command-contract",
            "destructive": false,
            "workspace_root": Value::Null,
            "full_disposable_ci_status": "specified-not-run",
            "full_disposable_ci_reason": "This command is safe for docs-check; CI should run the recorded disposable lane or pass --execute when write-heavy downstream proof is required."
        })
    };

    let report = json!({
        "command": "golden-downstream",
        "ok": failed_checks.is_empty(),
        "generated_at_utc": Utc::now().to_rfc3339(),
        "artifact": relative_path(&roots.app_root, &artifact_path),
        "profile_filter": profile_filter,
        "required_profile_verification": required_profile_verification,
        "required_frontend_verification": required_frontend_commands,
        "profiles": profile_reports,
        "failed_checks": failed_checks,
        "execution": execution_summary
    });

    fs::write(
        &artifact_path,
        serde_json::to_string_pretty(&report)
            .map_err(|err| format!("failed to serialize golden downstream report: {err}"))?
            + "\n",
    )
    .map_err(|err| format!("failed to write {}: {err}", artifact_path.display()))?;

    emit_value(json_output, &report, || {
        format!(
            "Golden downstream profile contract: {}\nArtifact: {}",
            if failed_checks.is_empty() {
                "OK"
            } else {
                "FAILED"
            },
            artifact_path.display()
        )
    })?;

    if failed_checks.is_empty() {
        Ok(())
    } else {
        Err("golden-downstream required checks failed".to_string())
    }
}

fn golden_downstream_required_profile_commands(profile: &str) -> Vec<&'static str> {
    let test_command = if profile == "second-consumer" {
        "scripts/appfw product test --fast"
    } else {
        "scripts/appfw product test"
    };
    vec![
        "scripts/appfw product validate --json",
        "scripts/appfw product harness-check --json",
        "scripts/appfw product generate",
        "scripts/appfw product generate --check --json",
        test_command,
        "scripts/appfw product handoff --json",
    ]
}

fn execute_golden_downstream_profile(
    roots: &CommandRoots,
    profile: &str,
    execution_root: &Path,
) -> Result<Value, String> {
    let target = execution_root.join(format!(
        "{}-{}",
        profile,
        Utc::now().format("%Y%m%dT%H%M%SZ")
    ));
    let framework_appfw = roots.framework_root.join("scripts/appfw");
    let product_appfw = target.join("scripts/appfw");
    let target_arg = target.display().to_string();

    let mut steps = Vec::new();
    steps.push(run_golden_downstream_step(
        "product new",
        &roots.framework_root,
        &framework_appfw,
        &[
            "product",
            "new",
            target_arg.as_str(),
            "--from",
            "current",
            "--profile",
            profile,
            "--generate",
            "--json",
        ],
        &roots.framework_root,
    ));

    if steps
        .last()
        .and_then(|step| step.get("ok"))
        .and_then(Value::as_bool)
        == Some(true)
    {
        let mut commands = vec![
            ("product validate", vec!["product", "validate", "--json"]),
            (
                "product harness-check",
                vec!["product", "harness-check", "--json"],
            ),
            ("product generate", vec!["product", "generate"]),
            (
                "product generate --check",
                vec!["product", "generate", "--check", "--json"],
            ),
            if profile == "second-consumer" {
                ("product test --fast", vec!["product", "test", "--fast"])
            } else {
                ("product test", vec!["product", "test"])
            },
        ];
        if profile == "crm-sample" {
            commands.extend([
                (
                    "product frontend-test",
                    vec!["product", "frontend-test", "--json"],
                ),
                (
                    "product release-check",
                    vec!["product", "release-check", "--json"],
                ),
                (
                    "product upgrade --check",
                    vec!["product", "upgrade", "--check", "--json"],
                ),
            ]);
        }
        commands.push(("product handoff", vec!["product", "handoff", "--json"]));

        for (name, args) in commands {
            let step = run_golden_downstream_step(
                name,
                &target,
                &product_appfw,
                &args,
                &roots.framework_root,
            );
            let ok = step.get("ok").and_then(Value::as_bool) == Some(true);
            steps.push(step);
            if !ok {
                break;
            }
        }
    }

    let ok = steps
        .iter()
        .all(|step| step.get("ok").and_then(Value::as_bool) == Some(true));
    Ok(json!({
        "status": if ok { "passed" } else { "failed" },
        "ok": ok,
        "profile": profile,
        "target": relative_path(&roots.app_root, &target),
        "workspace_root": relative_path(&roots.app_root, execution_root),
        "steps": steps,
    }))
}

fn run_golden_downstream_step(
    name: &str,
    cwd: &Path,
    program: &Path,
    args: &[&str],
    framework_root: &Path,
) -> Value {
    let started = Instant::now();
    let command = std::iter::once(program.display().to_string())
        .chain(args.iter().map(|arg| arg.to_string()))
        .collect::<Vec<_>>()
        .join(" ");
    let output = Command::new(program)
        .current_dir(cwd)
        .env("APPFW_FRAMEWORK_ROOT", framework_root)
        .args(args)
        .output();
    let duration_ms = started.elapsed().as_millis() as u64;

    match output {
        Ok(output) => json!({
            "name": name,
            "command": command,
            "cwd": cwd.display().to_string(),
            "ok": output.status.success(),
            "exit_code": output.status.code(),
            "duration_ms": duration_ms,
            "stdout_excerpt": truncate_text(&String::from_utf8_lossy(&output.stdout), 6000),
            "stderr_excerpt": truncate_text(&String::from_utf8_lossy(&output.stderr), 6000),
        }),
        Err(err) => json!({
            "name": name,
            "command": command,
            "cwd": cwd.display().to_string(),
            "ok": false,
            "exit_code": Value::Null,
            "duration_ms": duration_ms,
            "error": err.to_string(),
            "stdout_excerpt": "",
            "stderr_excerpt": "",
        }),
    }
}

fn truncate_text(value: &str, max_chars: usize) -> String {
    let mut output = String::new();
    for (index, ch) in value.chars().enumerate() {
        if index >= max_chars {
            output.push_str("\n... truncated ...");
            return output;
        }
        output.push(ch);
    }
    output
}

fn default_profile_overlay() -> String {
    "overlay".to_string()
}

fn new_profile_root(repo_root: &Path, profile: &str) -> PathBuf {
    repo_root
        .join("app_gen/_golden/downstream_apps")
        .join(profile)
}

fn new_profile_manifest_path(repo_root: &Path, profile: &str) -> PathBuf {
    new_profile_root(repo_root, profile).join("profile.json")
}

fn read_new_profile_manifest(
    repo_root: &Path,
    profile: &NewProfile,
) -> Result<NewProfileManifest, String> {
    let manifest_path = new_profile_manifest_path(repo_root, profile.name);
    let contents = fs::read_to_string(&manifest_path).map_err(|err| {
        format!(
            "failed to read appfw product new profile manifest {}: {err}",
            manifest_path.display()
        )
    })?;
    let manifest = serde_json::from_str::<NewProfileManifest>(&contents).map_err(|err| {
        format!(
            "failed to parse appfw product new profile manifest {}: {err}",
            manifest_path.display()
        )
    })?;
    if manifest.name != profile.name {
        return Err(format!(
            "appfw product new profile manifest {} declares name `{}`, expected `{}`",
            manifest_path.display(),
            manifest.name,
            profile.name
        ));
    }
    if manifest.template != profile.template_path {
        return Err(format!(
            "appfw product new profile manifest {} declares template `{}`, expected `{}`",
            manifest_path.display(),
            manifest.template,
            profile.template_path
        ));
    }
    validate_new_profile_frontend_scaffold(repo_root, &manifest)?;
    Ok(manifest)
}

fn validate_new_profile_frontend_scaffold(
    repo_root: &Path,
    manifest: &NewProfileManifest,
) -> Result<(), String> {
    let Some(frontend_scaffold) = &manifest.frontend_scaffold else {
        return Ok(());
    };
    let template_root = repo_root.join(&manifest.template);
    let frontend_root = template_root.join(&frontend_scaffold.root);
    if !frontend_root.is_dir() {
        return Err(format!(
            "appfw product new profile `{}` frontend scaffold root is missing: {}",
            manifest.name,
            frontend_root.display()
        ));
    }

    for (label, relative_path) in [
        ("scaffold_manifest", &frontend_scaffold.scaffold_manifest),
        ("ownership_manifest", &frontend_scaffold.ownership_manifest),
        ("generated_contract", &frontend_scaffold.generated_contract),
        ("check_script", &frontend_scaffold.check_script),
    ] {
        let path = frontend_root.join(relative_path);
        if !path.is_file() {
            return Err(format!(
                "appfw product new profile `{}` frontend_scaffold.{label} is missing: {}",
                manifest.name,
                path.display()
            ));
        }
    }

    if frontend_scaffold.verification.is_empty() {
        return Err(format!(
            "appfw product new profile `{}` frontend_scaffold.verification must name at least one command",
            manifest.name
        ));
    }

    validate_new_profile_frontend_package_check(&frontend_root, manifest, frontend_scaffold)
}

fn validate_new_profile_frontend_package_check(
    frontend_root: &Path,
    manifest: &NewProfileManifest,
    frontend_scaffold: &NewProfileFrontendScaffold,
) -> Result<(), String> {
    let package_json_path = frontend_root.join("package.json");
    let package_json = fs::read_to_string(&package_json_path).map_err(|err| {
        format!(
            "failed to read appfw product new profile `{}` frontend package {}: {err}",
            manifest.name,
            package_json_path.display()
        )
    })?;
    let package_json = serde_json::from_str::<Value>(&package_json).map_err(|err| {
        format!(
            "failed to parse appfw product new profile `{}` frontend package {}: {err}",
            manifest.name,
            package_json_path.display()
        )
    })?;
    let Some(script_name) = frontend_package_script_name(&frontend_scaffold.package_check) else {
        return Err(format!(
            "appfw product new profile `{}` frontend_scaffold.package_check must look like `npm run <script>`",
            manifest.name
        ));
    };
    if package_json
        .get("scripts")
        .and_then(Value::as_object)
        .and_then(|scripts| scripts.get(script_name))
        .and_then(Value::as_str)
        .is_none()
    {
        return Err(format!(
            "appfw product new profile `{}` frontend package is missing script `{script_name}` for frontend_scaffold.package_check",
            manifest.name
        ));
    }
    Ok(())
}

fn frontend_package_script_name(command: &str) -> Option<&str> {
    command
        .strip_prefix("npm run ")
        .and_then(|rest| rest.split_whitespace().next())
        .filter(|script| !script.is_empty())
}

fn new_profiles() -> &'static [NewProfile] {
    const PROFILES: &[NewProfile] = &[
        NewProfile {
            name: "crm-sample",
            template_path: "examples/products/crm",
        },
        NewProfile {
            name: "second-consumer",
            template_path: "app_gen/_golden/downstream_apps/second-consumer/starter",
        },
    ];
    PROFILES
}

fn new_profile(name: &str) -> Option<NewProfile> {
    new_profiles()
        .iter()
        .copied()
        .find(|profile| profile.name == name)
}

fn copy_product_template(
    repo_root: &Path,
    target_root: &Path,
    profile: &NewProfile,
) -> Result<Vec<String>, String> {
    let template = repo_root.join(profile.template_path);
    if !template.is_dir() {
        return Err(format!(
            "appfw product new profile `{}` template is missing: {}",
            profile.name,
            template.display()
        ));
    }
    let mut copied = Vec::new();
    copy_dir_filtered(
        &template,
        target_root,
        &template,
        target_root,
        Some(&mut copied),
    )?;
    copied.sort();
    Ok(copied)
}

fn apply_new_profile(
    repo_root: &Path,
    target_root: &Path,
    profile: &str,
) -> Result<Vec<String>, String> {
    let overlay = repo_root
        .join("app_gen/_golden/downstream_apps")
        .join(profile)
        .join("overlay");
    if !overlay.exists() {
        return Ok(Vec::new());
    }
    let mut copied = Vec::new();
    copy_overlay(&overlay, &overlay, target_root, &mut copied)?;
    copied.sort();
    Ok(copied)
}

fn copy_overlay(
    root: &Path,
    source: &Path,
    target_root: &Path,
    copied: &mut Vec<String>,
) -> Result<(), String> {
    for entry in fs::read_dir(source)
        .map_err(|err| format!("failed to read overlay {}: {err}", source.display()))?
    {
        let entry = entry.map_err(|err| err.to_string())?;
        let source_path = entry.path();
        let relative = source_path
            .strip_prefix(root)
            .map_err(|err| err.to_string())?;
        let dest_path = target_root.join(relative);
        let file_type = entry.file_type().map_err(|err| err.to_string())?;
        if file_type.is_dir() {
            fs::create_dir_all(&dest_path)
                .map_err(|err| format!("failed to create {}: {err}", dest_path.display()))?;
            copy_overlay(root, &source_path, target_root, copied)?;
        } else if file_type.is_file() {
            if let Some(parent) = dest_path.parent() {
                fs::create_dir_all(parent)
                    .map_err(|err| format!("failed to create {}: {err}", parent.display()))?;
            }
            fs::copy(&source_path, &dest_path).map_err(|err| {
                format!(
                    "failed to copy overlay {} to {}: {err}",
                    source_path.display(),
                    dest_path.display()
                )
            })?;
            copied.push(relative.display().to_string());
        }
    }
    Ok(())
}

fn copy_dir_filtered(
    source: &Path,
    target: &Path,
    repo_root: &Path,
    target_root: &Path,
    mut copied: Option<&mut Vec<String>>,
) -> Result<(), String> {
    for entry in
        fs::read_dir(source).map_err(|err| format!("failed to read {}: {err}", source.display()))?
    {
        let entry = entry.map_err(|err| err.to_string())?;
        let source_path = entry.path();
        let relative = source_path
            .strip_prefix(repo_root)
            .map_err(|err| err.to_string())?;
        if should_skip_new_copy(relative) || normalize_path(&source_path).starts_with(target_root) {
            continue;
        }
        let dest_path = target.join(relative);
        let file_type = entry.file_type().map_err(|err| err.to_string())?;
        if file_type.is_dir() {
            fs::create_dir_all(&dest_path)
                .map_err(|err| format!("failed to create {}: {err}", dest_path.display()))?;
            copy_dir_filtered(
                &source_path,
                target,
                repo_root,
                target_root,
                copied.as_deref_mut(),
            )?;
        } else if file_type.is_file() {
            if let Some(parent) = dest_path.parent() {
                fs::create_dir_all(parent)
                    .map_err(|err| format!("failed to create {}: {err}", parent.display()))?;
            }
            fs::copy(&source_path, &dest_path).map_err(|err| {
                format!(
                    "failed to copy {} to {}: {err}",
                    source_path.display(),
                    dest_path.display()
                )
            })?;
            if let Some(copied) = copied.as_deref_mut() {
                copied.push(relative.display().to_string());
            }
        }
    }
    Ok(())
}

fn should_skip_new_copy(relative: &Path) -> bool {
    let rel = relative.to_string_lossy().replace('\\', "/");
    rel == ".git"
        || rel.starts_with(".git/")
        || rel == ".env"
        || rel.ends_with("/.env")
        || rel == ".appfw/local.env"
        || rel == "appfw.lock"
        || rel.ends_with("/Cargo.lock")
        || rel == ".DS_Store"
        || rel.ends_with("/.DS_Store")
        || rel.ends_with(".log")
        || rel == ".qodo"
        || rel.starts_with(".qodo/")
        || rel == "target"
        || rel.starts_with("target/")
        || rel.ends_with("/target")
        || rel.contains("/target/")
        || rel == "debug"
        || rel.starts_with("debug/")
        || rel.ends_with("/debug")
        || rel.contains("/debug/")
        || rel == "node_modules"
        || rel.ends_with("/node_modules")
        || rel.contains("/node_modules/")
        || rel == "admin_ui/dist"
        || rel.starts_with("admin_ui/dist/")
        || rel == "backend/admin_dist"
        || rel.starts_with("backend/admin_dist/")
        || rel == "backend/product_dist"
        || rel.starts_with("backend/product_dist/")
        || rel == "database/Cargo.toml"
        || rel == "database/src"
        || rel.starts_with("database/src/")
        || rel == "database/data"
        || rel.starts_with("database/data/")
        || rel == "app_gen/target"
        || rel.starts_with("app_gen/target/")
        || rel == "api_tests/target"
        || rel.starts_with("api_tests/target/")
        || rel == "backend/target"
        || rel.starts_with("backend/target/")
        || rel == "database/target"
        || rel.starts_with("database/target/")
        || rel == "rego_test/target"
        || rel.starts_with("rego_test/target/")
}

fn rewrite_split_root_cargo_paths(
    product_root: &Path,
    framework_root: &Path,
) -> Result<(), String> {
    rewrite_manifest_path_dependency(
        &product_root.join("backend/Cargo.toml"),
        "appfw_runtime",
        "appfw-runtime",
        &framework_root.join("appfw_runtime"),
    )?;
    rewrite_optional_manifest_path_dependency(
        &product_root.join("backend/Cargo.toml"),
        "appfw_provider_mongo",
        "appfw-provider-mongo",
        &framework_root.join("appfw_provider_mongo"),
    )?;
    rewrite_optional_manifest_path_dependency(
        &product_root.join("backend/Cargo.toml"),
        "appfw_provider_mssql",
        "appfw-provider-mssql",
        &framework_root.join("appfw_provider_mssql"),
    )?;
    rewrite_optional_manifest_path_dependency(
        &product_root.join("backend/Cargo.toml"),
        "appfw_mssql_auth",
        "appfw-mssql-auth",
        &framework_root.join("appfw_mssql_auth"),
    )?;
    rewrite_optional_manifest_path_dependency(
        &product_root.join("backend/Cargo.toml"),
        "appfw_provider_postgres",
        "appfw-provider-postgres",
        &framework_root.join("appfw_provider_postgres"),
    )?;
    rewrite_optional_manifest_path_dependency(
        &product_root.join("backend/Cargo.toml"),
        "appfw_provider_snowflake",
        "appfw-provider-snowflake",
        &framework_root.join("appfw_provider_snowflake"),
    )?;
    rewrite_optional_manifest_path_dependency(
        &product_root.join("backend/Cargo.toml"),
        "appfw_provider_neo4j",
        "appfw-provider-neo4j",
        &framework_root.join("appfw_provider_neo4j"),
    )?;
    let database_manifest = product_root.join("database/Cargo.toml");
    if database_manifest.exists() {
        rewrite_manifest_path_dependency(
            &database_manifest,
            "appfw_runtime",
            "appfw-runtime",
            &framework_root.join("appfw_runtime"),
        )?;
    }
    rewrite_manifest_path_dependency(
        &product_root.join("api_tests/Cargo.toml"),
        "appfw_test",
        "appfw-test",
        &framework_root.join("appfw_test"),
    )?;
    rewrite_manifest_path_dependency(
        &product_root.join("rego_test/Cargo.toml"),
        "appfw_test",
        "appfw-test",
        &framework_root.join("appfw_test"),
    )?;
    Ok(())
}

fn rewrite_split_root_compose_paths(
    product_root: &Path,
    framework_root: &Path,
) -> Result<(), String> {
    let compose_path = product_root.join("podman-compose.yml");
    if !compose_path.exists() {
        return Ok(());
    }
    let runtime_source = relative_path_between(product_root, &framework_root.join("appfw_runtime"));
    let observability_source =
        relative_path_between(product_root, &framework_root.join("observability"));
    let contents = fs::read_to_string(&compose_path)
        .map_err(|err| format!("failed to read {}: {err}", compose_path.display()))?;
    let rewritten = contents
        .lines()
        .map(|line| rewrite_compose_source_line(line, &runtime_source, &observability_source))
        .collect::<Vec<_>>()
        .join("\n");
    fs::write(&compose_path, format!("{rewritten}\n"))
        .map_err(|err| format!("failed to write {}: {err}", compose_path.display()))
}

fn rewrite_compose_source_line(
    line: &str,
    runtime_source: &str,
    observability_source: &str,
) -> String {
    let trimmed = line.trim_start();
    if !trimmed.starts_with("source:") {
        return line.to_string();
    }
    let indent = &line[..line.len() - trimmed.len()];
    if trimmed.contains("appfw_runtime") {
        return format!("{indent}source: {runtime_source}");
    }
    if let Some((_, suffix)) = trimmed.split_once("observability/") {
        return format!("{indent}source: {observability_source}/{suffix}");
    }
    line.to_string()
}

fn rewrite_manifest_path_dependency(
    manifest_path: &Path,
    dependency: &str,
    package: &str,
    crate_path: &Path,
) -> Result<(), String> {
    rewrite_manifest_path_dependency_impl(manifest_path, dependency, package, crate_path, true)
}

fn rewrite_optional_manifest_path_dependency(
    manifest_path: &Path,
    dependency: &str,
    package: &str,
    crate_path: &Path,
) -> Result<(), String> {
    rewrite_manifest_path_dependency_impl(manifest_path, dependency, package, crate_path, false)
}

fn rewrite_manifest_path_dependency_impl(
    manifest_path: &Path,
    dependency: &str,
    package: &str,
    crate_path: &Path,
    required: bool,
) -> Result<(), String> {
    let manifest_dir = manifest_path
        .parent()
        .ok_or_else(|| format!("manifest has no parent: {}", manifest_path.display()))?;
    let relative = relative_path_between(manifest_dir, crate_path);
    let contents = fs::read_to_string(manifest_path)
        .map_err(|err| format!("failed to read {}: {err}", manifest_path.display()))?;
    let prefix = format!("{dependency} = {{");
    let mut changed = false;
    let rewritten = contents
        .lines()
        .map(|line| {
            if line.trim_start().starts_with(&prefix) {
                changed = true;
                rewrite_manifest_path_dependency_line(line, dependency, package, &relative)
            } else {
                line.to_string()
            }
        })
        .collect::<Vec<_>>()
        .join("\n");

    if !changed && required {
        return Err(format!(
            "could not find dependency `{dependency}` in {}",
            manifest_path.display()
        ));
    }
    if !changed {
        return Ok(());
    }
    fs::write(manifest_path, format!("{rewritten}\n"))
        .map_err(|err| format!("failed to write {}: {err}", manifest_path.display()))
}

fn rewrite_manifest_path_dependency_line(
    line: &str,
    dependency: &str,
    package: &str,
    relative_path: &str,
) -> String {
    let indent = &line[..line.len() - line.trim_start().len()];
    let Some((_, rest)) = line.trim_start().split_once('{') else {
        return format!(
            "{indent}{dependency} = {{ package = \"{package}\", path = \"{relative_path}\" }}"
        );
    };
    let Some((inner, _)) = rest.rsplit_once('}') else {
        return format!(
            "{indent}{dependency} = {{ package = \"{package}\", path = \"{relative_path}\" }}"
        );
    };
    let mut fields = vec![
        format!("package = \"{package}\""),
        format!("path = \"{relative_path}\""),
    ];
    for field in split_inline_table_fields(inner) {
        let Some((key, _)) = field.split_once('=') else {
            continue;
        };
        let key = key.trim();
        if key == "package" || key == "path" {
            continue;
        }
        fields.push(field.trim().to_string());
    }
    format!("{indent}{dependency} = {{ {} }}", fields.join(", "))
}

fn split_inline_table_fields(input: &str) -> Vec<&str> {
    let mut fields = Vec::new();
    let mut start = 0;
    let mut in_string = false;
    let mut escaped = false;
    let mut bracket_depth = 0usize;

    for (index, ch) in input.char_indices() {
        if in_string {
            if escaped {
                escaped = false;
            } else if ch == '\\' {
                escaped = true;
            } else if ch == '"' {
                in_string = false;
            }
            continue;
        }

        match ch {
            '"' => in_string = true,
            '[' => bracket_depth += 1,
            ']' => bracket_depth = bracket_depth.saturating_sub(1),
            ',' if bracket_depth == 0 => {
                let field = input[start..index].trim();
                if !field.is_empty() {
                    fields.push(field);
                }
                start = index + ch.len_utf8();
            }
            _ => {}
        }
    }

    let field = input[start..].trim();
    if !field.is_empty() {
        fields.push(field);
    }
    fields
}

fn write_new_app_local_env(product_root: &Path, framework_root: &Path) -> Result<(), String> {
    let local_env = product_root.join(".appfw/local.env");
    if let Some(parent) = local_env.parent() {
        fs::create_dir_all(parent)
            .map_err(|err| format!("failed to create {}: {err}", parent.display()))?;
    }
    let contents = format!(
        "APPFW_FRAMEWORK_ROOT={}\n",
        shell_single_quote(&framework_root.display().to_string())
    );
    fs::write(&local_env, contents)
        .map_err(|err| format!("failed to write {}: {err}", local_env.display()))
}

fn run_new_app_command(target: &Path, framework_root: &Path, args: &[&str]) -> Result<(), String> {
    let (cmd, rest) = args
        .split_first()
        .ok_or_else(|| "empty command".to_string())?;
    let output = Command::new(cmd)
        .current_dir(target)
        .env("APPFW_FRAMEWORK_ROOT", framework_root)
        .args(rest)
        .output()
        .map_err(|err| {
            format!(
                "failed to run {} in {}: {err}",
                args.join(" "),
                target.display()
            )
        })?;
    if output.status.success() {
        Ok(())
    } else {
        Err(format!(
            "command failed in new app: {}\nstdout:\n{}\nstderr:\n{}",
            args.join(" "),
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        ))
    }
}

fn emit<T: Serialize>(
    json_output: bool,
    value: &T,
    text: impl FnOnce() -> String,
) -> Result<(), String> {
    if json_output {
        println!(
            "{}",
            serde_json::to_string_pretty(value)
                .map_err(|err| format!("failed to serialize JSON output: {err}"))?
        );
    } else {
        println!("{}", text());
    }
    Ok(())
}

fn emit_value(
    json_output: bool,
    value: &Value,
    text: impl FnOnce() -> String,
) -> Result<(), String> {
    if json_output {
        println!(
            "{}",
            serde_json::to_string_pretty(value)
                .map_err(|err| format!("failed to serialize JSON output: {err}"))?
        );
    } else {
        println!("{}", text());
    }
    Ok(())
}

fn default_repo_root_from(cwd: &Path) -> PathBuf {
    if cwd.join("scripts/appfw").is_file() {
        return normalize_path(cwd);
    }
    if cwd.file_name().and_then(|name| name.to_str()) == Some("app_gen") {
        return normalize_path(&cwd.join(".."));
    }
    normalize_path(cwd)
}

fn default_product_root_for_framework(framework_root: &Path) -> Option<PathBuf> {
    let product_root = framework_root.join("examples/products/crm");
    if product_root.join(".appfw/manifest.yaml").is_file()
        && product_root.join(".appfw/model").is_dir()
    {
        Some(product_root)
    } else {
        None
    }
}

fn absolute_path(base: &Path, value: &str) -> PathBuf {
    let path = Path::new(value);
    if path.is_absolute() {
        normalize_path(path)
    } else {
        normalize_path(&base.join(path))
    }
}

fn absolutize(path: PathBuf, cwd: &Path) -> PathBuf {
    if path.is_absolute() {
        path
    } else {
        cwd.join(path)
    }
}

fn normalize_path(path: &Path) -> PathBuf {
    let mut normalized = PathBuf::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                normalized.pop();
            }
            other => normalized.push(other.as_os_str()),
        }
    }
    normalized
}

fn relative_path(repo_root: &Path, path: &Path) -> String {
    path.strip_prefix(repo_root)
        .unwrap_or(path)
        .display()
        .to_string()
}

fn relative_path_between(from: &Path, to: &Path) -> String {
    let from = normalize_path(from);
    let to = normalize_path(to);
    let from_components = normalized_components(&from);
    let to_components = normalized_components(&to);
    let common_len = from_components
        .iter()
        .zip(to_components.iter())
        .take_while(|(left, right)| left == right)
        .count();

    let mut relative = PathBuf::new();
    for _ in common_len..from_components.len() {
        relative.push("..");
    }
    for component in to_components.iter().skip(common_len) {
        relative.push(component);
    }

    if relative.as_os_str().is_empty() {
        ".".to_string()
    } else {
        relative.display().to_string()
    }
}

fn normalized_components(path: &Path) -> Vec<String> {
    path.components()
        .map(|component| component.as_os_str().to_string_lossy().to_string())
        .collect()
}

fn normalize_key(value: &str) -> String {
    value
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() {
                ch.to_ascii_lowercase()
            } else {
                '_'
            }
        })
        .collect::<String>()
        .split('_')
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join("_")
}

#[cfg(test)]
fn artifact_manifest_identity_hash_optional(
    app_root: &Path,
    manifest_path: &Path,
) -> Result<Option<String>, String> {
    artifact_manifest_identity_hash_optional_in_report_root(
        app_root,
        app_root,
        app_root,
        manifest_path,
    )
}

fn artifact_manifest_identity_hash_optional_in_report_root(
    app_root: &Path,
    config_root: &Path,
    report_root: &Path,
    manifest_path: &Path,
) -> Result<Option<String>, String> {
    validate_evidence_root_shape(report_root, "configured report root")?;
    let canonical_app_root =
        canonical_artifact_identity_root(app_root, "artifact identity app root")?;
    let canonical_config_root =
        canonical_artifact_identity_root(config_root, "artifact identity config root")?;
    manifest_path.strip_prefix(report_root).map_err(|_| {
        format!(
            "artifact manifest is outside configured report root {}: {}",
            report_root.display(),
            manifest_path.display()
        )
    })?;
    match fs::symlink_metadata(report_root) {
        Ok(_) => {}
        Err(err) if err.kind() == io::ErrorKind::NotFound => {
            validate_missing_evidence_root(report_root, "configured report root")?;
            return Ok(None);
        }
        Err(err) => {
            return Err(format!(
                "failed to inspect configured report root {}: {err}",
                report_root.display()
            ))
        }
    }
    let manifest_path = contained_evidence_path(
        report_root,
        manifest_path,
        ContainedEvidenceLeaf::MissingOrRegularFile,
    )?;
    let Some(bytes) = read_bounded_regular_file_optional(
        &manifest_path,
        PORTABLE_EVIDENCE_MAX_BYTES,
        "artifact manifest",
    )?
    else {
        return Ok(None);
    };
    let manifest_path_after = contained_evidence_path(
        report_root,
        &manifest_path,
        ContainedEvidenceLeaf::ExistingRegularFile,
    )?;
    if manifest_path_after != manifest_path {
        return Err(format!(
            "artifact manifest path changed while reading: {}",
            manifest_path.display()
        ));
    }
    let records = serde_json::from_slice::<Vec<ArtifactManifestRecordForIdentity>>(&bytes)
        .map_err(|err| {
            format!(
                "artifact manifest {} does not match the closed generated record schema: {err}",
                manifest_path.display()
            )
        })?;
    let mut seen_paths = BTreeSet::new();
    let mut identity_records = Vec::with_capacity(records.len());
    for record in records {
        validate_artifact_record_enum(
            "ownership",
            &record.ownership,
            &["generated", "human_owned"],
        )?;
        validate_artifact_record_enum("overwrite", &record.overwrite, &["always", "if_missing"])?;
        validate_artifact_record_enum("action", &record.action, &["written", "skipped"])?;
        validate_sha256(&record.content_sha256, "content_sha256")?;
        if let Some(source_sha256) = record.source_sha256.as_deref() {
            validate_sha256(source_sha256, "source_sha256")?;
        }

        let (root, relative_path) = portable_artifact_qualified_path(
            app_root,
            &canonical_app_root,
            config_root,
            &canonical_config_root,
            &record.path,
        )?;
        let artifact_path = Path::new(&record.path);
        let artifact_bytes = read_bounded_regular_file_optional(
            artifact_path,
            PORTABLE_EVIDENCE_MAX_BYTES,
            "generated artifact",
        )?
        .ok_or_else(|| {
            format!(
                "artifact manifest references a missing generated artifact: {}",
                artifact_path.display()
            )
        })?;
        let actual_content_sha256 = format!("sha256:{:x}", Sha256::digest(&artifact_bytes));
        if actual_content_sha256 != record.content_sha256 {
            return Err(format!(
                "artifact manifest content_sha256 does not match final bytes for `{}`: recorded {}, actual {}",
                record.path, record.content_sha256, actual_content_sha256
            ));
        }
        let (root_after, relative_path_after) = portable_artifact_qualified_path(
            app_root,
            &canonical_app_root,
            config_root,
            &canonical_config_root,
            &record.path,
        )?;
        if root_after != root || relative_path_after != relative_path {
            return Err(format!(
                "artifact path changed identity while verifying final bytes: `{}`",
                record.path
            ));
        }
        if !seen_paths.insert((root.clone(), relative_path.clone())) {
            return Err(format!(
                "artifact manifest contains duplicate normalized path `{root}:{relative_path}`"
            ));
        }
        identity_records.push(ArtifactManifestIdentityRecord {
            root,
            path: relative_path,
            ownership: record.ownership,
            overwrite: record.overwrite,
            content_sha256: record.content_sha256,
            source_sha256: record.source_sha256,
        });
    }
    identity_records.sort_by(|left, right| {
        (left.root.as_str(), left.path.as_str()).cmp(&(right.root.as_str(), right.path.as_str()))
    });
    let identity = ArtifactManifestIdentity {
        schema: ARTIFACT_MANIFEST_IDENTITY_SCHEMA,
        records: identity_records,
    };
    let canonical_json = serde_json::to_vec(&identity)
        .map_err(|err| format!("failed to serialize portable artifact identity: {err}"))?;
    Ok(Some(format!(
        "sha256:{:x}",
        Sha256::digest(&canonical_json)
    )))
}

fn validate_missing_evidence_root(root: &Path, label: &str) -> Result<(), String> {
    validate_evidence_root_shape(root, label)?;

    let mut existing = root;
    let mut suffix = Vec::new();
    loop {
        match fs::symlink_metadata(existing) {
            Ok(metadata) => {
                if metadata.file_type().is_symlink() || !metadata.file_type().is_dir() {
                    return Err(format!(
                        "{label} existing ancestor must be a non-symlink directory: {}",
                        existing.display()
                    ));
                }
                break;
            }
            Err(err) if err.kind() == io::ErrorKind::NotFound => {
                suffix.push(
                    existing
                        .file_name()
                        .ok_or_else(|| format!("{label} has no file name"))?
                        .to_owned(),
                );
                existing = existing
                    .parent()
                    .ok_or_else(|| format!("{label} has no existing ancestor"))?;
            }
            Err(err) => {
                return Err(format!(
                    "failed to inspect {label} {}: {err}",
                    existing.display()
                ))
            }
        }
    }

    let mut canonical = fs::canonicalize(existing).map_err(|err| {
        format!(
            "failed to canonicalize {label} existing ancestor {}: {err}",
            existing.display()
        )
    })?;
    for component in suffix.iter().rev() {
        canonical.push(component);
    }
    if !lexical_evidence_root_matches_canonical(root, &canonical) {
        return Err(format!(
            "{label} must not contain symlink ancestors: {}",
            root.display()
        ));
    }
    Ok(())
}

fn validate_configured_evidence_root(root: &Path, label: &str) -> Result<(), String> {
    validate_evidence_root_shape(root, label)?;
    match fs::symlink_metadata(root) {
        Ok(_) => canonical_artifact_identity_root(root, label).map(|_| ()),
        Err(err) if err.kind() == io::ErrorKind::NotFound => {
            validate_missing_evidence_root(root, label)
        }
        Err(err) => Err(format!(
            "failed to inspect {label} {}: {err}",
            root.display()
        )),
    }
}

fn hash_contained_report_file_optional(
    report_root: &Path,
    relative: &str,
    label: &str,
) -> Result<Option<String>, String> {
    validate_configured_evidence_root(report_root, "configured report root")?;
    if !report_root.exists() {
        return Ok(None);
    }
    let path = contained_evidence_path(
        report_root,
        &report_root.join(relative),
        ContainedEvidenceLeaf::MissingOrRegularFile,
    )?;
    let Some(bytes) =
        read_bounded_regular_file_optional(&path, PORTABLE_EVIDENCE_MAX_BYTES, label)?
    else {
        return Ok(None);
    };
    Ok(Some(format!("sha256:{:x}", Sha256::digest(&bytes))))
}

fn validate_evidence_root_shape(root: &Path, label: &str) -> Result<(), String> {
    if !root.is_absolute() {
        return Err(format!("{label} must be absolute: {}", root.display()));
    }
    if root.parent().is_none() {
        return Err(format!(
            "{label} cannot be a filesystem root: {}",
            root.display()
        ));
    }
    Ok(())
}

fn validate_artifact_record_enum(field: &str, value: &str, allowed: &[&str]) -> Result<(), String> {
    if allowed.contains(&value) {
        Ok(())
    } else {
        Err(format!(
            "artifact manifest field `{field}` has unsupported value `{value}`"
        ))
    }
}

fn validate_sha256(value: &str, field: &str) -> Result<(), String> {
    let Some(hex) = value.strip_prefix("sha256:") else {
        return Err(format!(
            "artifact manifest field `{field}` must use sha256:<64 lowercase hex>"
        ));
    };
    if hex.len() != 64
        || !hex
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return Err(format!(
            "artifact manifest field `{field}` must use sha256:<64 lowercase hex>"
        ));
    }
    Ok(())
}

fn canonical_artifact_identity_root(root: &Path, label: &str) -> Result<PathBuf, String> {
    validate_evidence_root_shape(root, label)?;
    let metadata = fs::symlink_metadata(root).map_err(|err| {
        format!(
            "failed to inspect {label} {} without following links: {err}",
            root.display()
        )
    })?;
    if metadata.file_type().is_symlink() || !metadata.file_type().is_dir() {
        return Err(format!(
            "{label} must be a non-symlink directory: {}",
            root.display()
        ));
    }
    let canonical = fs::canonicalize(root)
        .map_err(|err| format!("failed to canonicalize {label} {}: {err}", root.display()))?;
    if !lexical_evidence_root_matches_canonical(root, &canonical) {
        return Err(format!(
            "{label} must not contain symlink ancestors: {}",
            root.display()
        ));
    }
    Ok(canonical)
}

fn portable_artifact_qualified_path(
    app_root: &Path,
    canonical_app_root: &Path,
    config_root: &Path,
    canonical_config_root: &Path,
    raw_path: &str,
) -> Result<(String, String), String> {
    if raw_path.trim() != raw_path || raw_path.is_empty() {
        return Err(
            "artifact manifest path must be a non-empty canonical absolute path".to_string(),
        );
    }
    #[cfg(unix)]
    if raw_path.ends_with('/')
        || raw_path.contains("//")
        || raw_path
            .split('/')
            .any(|component| matches!(component, "." | ".."))
    {
        return Err(format!(
            "artifact manifest path is not canonical: `{raw_path}`"
        ));
    }
    let path = Path::new(raw_path);
    if !path.is_absolute()
        || path.components().any(|component| {
            matches!(
                component,
                Component::CurDir | Component::ParentDir | Component::Prefix(_)
            )
        })
    {
        return Err(format!(
            "artifact manifest path must be absolute without dot or parent components: `{raw_path}`"
        ));
    }
    let link_metadata = fs::symlink_metadata(path).map_err(|err| {
        format!(
            "failed to inspect artifact manifest path `{raw_path}` without following links: {err}"
        )
    })?;
    if link_metadata.file_type().is_symlink() || !link_metadata.file_type().is_file() {
        return Err(format!(
            "artifact manifest path must identify a regular non-symlink file: `{raw_path}`"
        ));
    }
    let canonical_path = fs::canonicalize(path)
        .map_err(|err| format!("failed to canonicalize artifact path `{raw_path}`: {err}"))?;
    let app_relative = canonical_path.strip_prefix(canonical_app_root).ok();
    let config_relative = canonical_path.strip_prefix(canonical_config_root).ok();
    let use_config_root = match (app_relative, config_relative) {
        (None, None) => {
            return Err(format!(
                "artifact manifest path is outside configured app/config roots: `{raw_path}`"
            ))
        }
        (None, Some(_)) => true,
        (Some(_), None) => false,
        (Some(_), Some(_)) => {
            canonical_config_root != canonical_app_root
                && canonical_config_root.components().count()
                    > canonical_app_root.components().count()
        }
    };
    let (root_name, lexical_configured_root, canonical_configured_root, relative) =
        if use_config_root {
            (
                "config",
                config_root,
                canonical_config_root,
                config_relative.expect("config-root candidate was selected"),
            )
        } else {
            (
                "app",
                app_root,
                canonical_app_root,
                app_relative.expect("app-root candidate was selected"),
            )
        };
    if relative.as_os_str().is_empty()
        || relative
            .components()
            .any(|component| !matches!(component, Component::Normal(_)))
    {
        return Err(format!(
            "artifact manifest path does not resolve to a safe {root_name}-relative file: `{raw_path}`"
        ));
    }

    let relative_component_count = relative.components().count();
    let mut lexical_root = path;
    for _ in 0..relative_component_count {
        lexical_root = lexical_root.parent().ok_or_else(|| {
            format!("artifact manifest path has no app-root prefix: `{raw_path}`")
        })?;
    }
    if !equivalent_artifact_root(
        lexical_root,
        lexical_configured_root,
        canonical_configured_root,
    ) {
        return Err(format!(
            "artifact manifest path is outside configured {root_name} root: `{raw_path}`"
        ));
    }
    let lexical_relative = path
        .strip_prefix(lexical_root)
        .map_err(|err| format!("failed to derive artifact relative path: {err}"))?;
    let mut current = lexical_root.to_path_buf();
    for component in lexical_relative.components() {
        let Component::Normal(component) = component else {
            return Err(format!(
                "artifact manifest path has a noncanonical component: `{raw_path}`"
            ));
        };
        current.push(component);
        let metadata = fs::symlink_metadata(&current).map_err(|err| {
            format!(
                "failed to inspect artifact path ancestor {}: {err}",
                current.display()
            )
        })?;
        if metadata.file_type().is_symlink() {
            return Err(format!(
                "artifact manifest path contains a symlink below configured {root_name} root: {}",
                current.display()
            ));
        }
    }

    let relative = path_to_slash(relative);
    if relative.is_empty() {
        return Err(format!(
            "artifact manifest path resolves to an empty relative path: `{raw_path}`"
        ));
    }
    Ok((root_name.to_string(), relative))
}

fn equivalent_artifact_root(candidate: &Path, lexical_root: &Path, canonical_root: &Path) -> bool {
    if candidate == lexical_root || candidate == canonical_root {
        return true;
    }
    #[cfg(target_os = "macos")]
    {
        let private_var = Path::new("/private/var");
        let var = Path::new("/var");
        if let Ok(suffix) = candidate.strip_prefix(var) {
            if private_var.join(suffix) == lexical_root
                || private_var.join(suffix) == canonical_root
            {
                return true;
            }
        }
        if let Ok(suffix) = candidate.strip_prefix(private_var) {
            if var.join(suffix) == lexical_root {
                return true;
            }
        }
    }
    false
}

fn read_bounded_regular_file_optional(
    path: &Path,
    max_bytes: u64,
    label: &str,
) -> Result<Option<Vec<u8>>, String> {
    let initial = match fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(err) if err.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(err) => {
            return Err(format!(
                "failed to inspect {label} {}: {err}",
                path.display()
            ))
        }
    };
    if initial.file_type().is_symlink() || !initial.file_type().is_file() {
        return Err(format!(
            "{label} must be a regular non-symlink file: {}",
            path.display()
        ));
    }
    if initial.len() > max_bytes {
        return Err(format!(
            "{label} {} is {} bytes; reading stops at the {} byte limit",
            path.display(),
            initial.len(),
            max_bytes
        ));
    }
    let mut options = fs::OpenOptions::new();
    options.read(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.custom_flags(libc::O_NOFOLLOW);
    }
    let mut file = options.open(path).map_err(|err| {
        format!(
            "failed to open {label} without following its leaf {}: {err}",
            path.display()
        )
    })?;
    let opened = file
        .metadata()
        .map_err(|err| format!("failed to inspect open {label} {}: {err}", path.display()))?;
    if !same_bounded_file_identity(&initial, &opened) {
        return Err(format!(
            "{label} changed type or identity while opening: {}",
            path.display()
        ));
    }
    let mut bytes = Vec::with_capacity(initial.len() as usize);
    let mut buffer = [0_u8; 64 * 1024];
    let mut total = 0_u64;
    loop {
        let read = file
            .read(&mut buffer)
            .map_err(|err| format!("failed to read {label} {}: {err}", path.display()))?;
        if read == 0 {
            break;
        }
        total = total
            .checked_add(read as u64)
            .ok_or_else(|| format!("{label} byte counter overflowed"))?;
        if total > max_bytes {
            return Err(format!(
                "{label} {} grew beyond the {} byte limit",
                path.display(),
                max_bytes
            ));
        }
        bytes.extend_from_slice(&buffer[..read]);
    }
    let final_opened = file.metadata().map_err(|err| {
        format!(
            "failed to re-inspect open {label} {}: {err}",
            path.display()
        )
    })?;
    let final_link = fs::symlink_metadata(path).map_err(|err| {
        format!(
            "failed to re-inspect {label} {} without following links: {err}",
            path.display()
        )
    })?;
    if !same_bounded_file_identity(&initial, &final_opened)
        || !same_bounded_file_identity(&initial, &final_link)
        || total != final_opened.len()
    {
        return Err(format!("{label} changed while reading: {}", path.display()));
    }
    Ok(Some(bytes))
}

fn read_contained_bounded_regular_file(
    app_root: &Path,
    path: &Path,
    max_bytes: u64,
    label: &str,
) -> Result<Vec<u8>, String> {
    let input_path =
        contained_evidence_path(app_root, path, ContainedEvidenceLeaf::ExistingRegularFile)?;
    let bytes = read_bounded_regular_file_optional(&input_path, max_bytes, label)?
        .ok_or_else(|| format!("required {label} is missing: {}", input_path.display()))?;
    let final_path =
        contained_evidence_path(app_root, path, ContainedEvidenceLeaf::ExistingRegularFile)?;
    if final_path != input_path {
        return Err(format!(
            "{label} changed containment while reading: {}",
            input_path.display()
        ));
    }
    Ok(bytes)
}

#[cfg(unix)]
fn same_bounded_file_identity(left: &fs::Metadata, right: &fs::Metadata) -> bool {
    use std::os::unix::fs::MetadataExt;
    (
        left.dev(),
        left.ino(),
        left.mode(),
        left.len(),
        left.mtime(),
        left.mtime_nsec(),
        left.ctime(),
        left.ctime_nsec(),
    ) == (
        right.dev(),
        right.ino(),
        right.mode(),
        right.len(),
        right.mtime(),
        right.mtime_nsec(),
        right.ctime(),
        right.ctime_nsec(),
    )
}

#[cfg(not(unix))]
fn same_bounded_file_identity(left: &fs::Metadata, right: &fs::Metadata) -> bool {
    left.file_type().is_file() && right.file_type().is_file() && left.len() == right.len()
}

#[cfg(unix)]
fn same_bounded_file_object(left: &fs::Metadata, right: &fs::Metadata) -> bool {
    use std::os::unix::fs::MetadataExt;
    (left.dev(), left.ino(), left.mode()) == (right.dev(), right.ino(), right.mode())
}

#[cfg(not(unix))]
fn same_bounded_file_object(left: &fs::Metadata, right: &fs::Metadata) -> bool {
    left.file_type().is_file() && right.file_type().is_file()
}

fn hash_file(path: &Path) -> Result<String, String> {
    hash_file_optional(path)?
        .ok_or_else(|| format!("file not found for hashing: {}", path.display()))
}

fn hash_file_optional(path: &Path) -> Result<Option<String>, String> {
    if !path.exists() {
        return Ok(None);
    }
    let mut file = fs::File::open(path)
        .map_err(|err| format!("failed to open {} for hashing: {err}", path.display()))?;
    let mut hasher = Sha256::new();
    let mut buffer = [0_u8; 8192];
    loop {
        let read = file
            .read(&mut buffer)
            .map_err(|err| format!("failed to read {} for hashing: {err}", path.display()))?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    Ok(Some(format!("sha256:{:x}", hasher.finalize())))
}

fn hash_tree(root: &Path) -> Result<String, String> {
    let mut files = Vec::new();
    collect_files(root, root, &mut files)?;
    files.sort();
    let mut hasher = Sha256::new();
    for file in files {
        let rel = file.strip_prefix(root).map_err(|err| err.to_string())?;
        hasher.update(rel.to_string_lossy().as_bytes());
        hasher.update([0]);
        let bytes = fs::read(&file)
            .map_err(|err| format!("failed to read {} for hashing: {err}", file.display()))?;
        hasher.update(bytes);
        hasher.update([0]);
    }
    Ok(format!("sha256:{:x}", hasher.finalize()))
}

fn hash_tree_optional(root: &Path) -> Result<Option<String>, String> {
    if root.exists() {
        hash_tree(root).map(Some)
    } else {
        Ok(None)
    }
}

fn hash_named_paths(paths: &[(String, PathBuf)]) -> Result<String, String> {
    let mut files = Vec::new();
    for (label, path) in paths {
        if path.is_dir() {
            collect_named_files(label, path, path, &mut files)?;
        } else if path.is_file() {
            files.push((label.clone(), path.clone()));
        } else {
            return Err(format!("path not found for hashing: {}", path.display()));
        }
    }
    files.sort_by(|left, right| left.0.cmp(&right.0));
    let mut hasher = Sha256::new();
    for (label, file) in files {
        hasher.update(label.as_bytes());
        hasher.update([0]);
        let bytes = fs::read(&file)
            .map_err(|err| format!("failed to read {} for hashing: {err}", file.display()))?;
        hasher.update(bytes);
        hasher.update([0]);
    }
    Ok(format!("sha256:{:x}", hasher.finalize()))
}

fn collect_named_files(
    label: &str,
    root: &Path,
    dir: &Path,
    files: &mut Vec<(String, PathBuf)>,
) -> Result<(), String> {
    for entry in
        fs::read_dir(dir).map_err(|err| format!("failed to read {}: {err}", dir.display()))?
    {
        let entry = entry.map_err(|err| err.to_string())?;
        let path = entry.path();
        let relative = path.strip_prefix(root).map_err(|err| err.to_string())?;
        if should_skip_hash(relative) {
            continue;
        }
        let file_type = entry.file_type().map_err(|err| err.to_string())?;
        if file_type.is_dir() {
            collect_named_files(label, root, &path, files)?;
        } else if file_type.is_file() {
            files.push((format!("{label}/{}", relative.display()), path));
        }
    }
    Ok(())
}

fn collect_files(root: &Path, dir: &Path, files: &mut Vec<PathBuf>) -> Result<(), String> {
    for entry in
        fs::read_dir(dir).map_err(|err| format!("failed to read {}: {err}", dir.display()))?
    {
        let entry = entry.map_err(|err| err.to_string())?;
        let path = entry.path();
        let relative = path.strip_prefix(root).map_err(|err| err.to_string())?;
        if should_skip_hash(relative) {
            continue;
        }
        let file_type = entry.file_type().map_err(|err| err.to_string())?;
        if file_type.is_dir() {
            collect_files(root, &path, files)?;
        } else if file_type.is_file() {
            files.push(path);
        }
    }
    Ok(())
}

fn should_skip_hash(relative: &Path) -> bool {
    let rel = relative.to_string_lossy();
    rel.contains("/target/")
        || rel.starts_with("target/")
        || rel.ends_with("/target")
        || rel.contains("/node_modules/")
        || rel.starts_with("node_modules/")
        || rel.ends_with("/node_modules")
}

fn git_output(repo_root: &Path, args: &[&str]) -> Option<String> {
    let output = Command::new("git")
        .current_dir(repo_root)
        .args(args)
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    Some(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

fn escape_toml(value: &str) -> String {
    value.replace('\\', "\\\\").replace('"', "\\\"")
}

fn shell_single_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\\''"))
}

fn unquote(value: &str) -> String {
    value
        .trim()
        .trim_matches('"')
        .replace("\\\"", "\"")
        .replace("\\\\", "\\")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write_test_file(root: &Path, relative: &str, contents: &str) {
        let path = root.join(relative);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).expect("create test file parent");
        }
        fs::write(path, contents).expect("write test file");
    }

    fn unique_test_root(label: &str) -> PathBuf {
        env::temp_dir().join(format!(
            "appfw-{label}-{}-{}",
            std::process::id(),
            Utc::now().timestamp_nanos_opt().unwrap_or_default()
        ))
    }

    fn lock_test_roots(product: &Path, framework: &Path) -> CommandRoots {
        CommandRoots {
            app_root: product.to_path_buf(),
            framework_root: framework.to_path_buf(),
            generator_root: framework.join("app_gen"),
            config_root: product.join(".appfw/model"),
            templates_root: framework.join("app_gen/_templates"),
            report_root: product.join(PRODUCT_REPORT_ROOT),
        }
    }

    fn seed_lock_test_framework(framework: &Path) {
        for (relative, contents) in [
            (
                "app_gen/Cargo.toml",
                "[package]\nname = \"appfw-codegen\"\nversion = \"0.1.1\"\n",
            ),
            ("app_gen/src/config_contract.rs", "// config contract\n"),
            ("app_gen/src/bin/appfw.rs", "// workflow cli\n"),
            (
                "app_gen/src/bin/appfw_introspect.rs",
                "// introspection cli\n",
            ),
            ("app_gen/_templates/backend/main.rs", "// template\n"),
            (
                "appfw_runtime/src/provider_capabilities.rs",
                "// capabilities\n",
            ),
            (
                "appfw_runtime/src/provider_certification.rs",
                "// certification\n",
            ),
            (
                "appfw_runtime/src/provider_contract_types.rs",
                "// contracts\n",
            ),
            (
                "appfw_cli/src/bin/provider_certification_export.rs",
                "// export\n",
            ),
        ] {
            write_test_file(framework, relative, contents);
        }
    }

    fn artifact_manifest_record(path: &Path, action: &str) -> Value {
        let content_sha256 = format!(
            "sha256:{:x}",
            Sha256::digest(fs::read(path).expect("read artifact fixture bytes"))
        );
        json!({
            "path": path.display().to_string(),
            "ownership": "generated",
            "overwrite": "always",
            "action": action,
            "content_sha256": content_sha256,
            "source_sha256": format!("sha256:{}", "2".repeat(64))
        })
    }

    fn write_lock_test_product(product: &Path, action: &str) -> PathBuf {
        write_test_file(product, "scripts/appfw", "#!/usr/bin/env bash\n");
        fs::create_dir_all(product.join(".appfw/model")).expect("create product config root");
        write_test_file(
            product,
            "frontend/src/generated/fabric-architecture.ts",
            "export const fabricArchitecture = {};\n",
        );
        let artifact = product.join("frontend/src/generated/fabric-architecture.ts");
        let manifest = json!([artifact_manifest_record(&artifact, action)]);
        write_test_file(
            product,
            ".appfw/target/appfw/artifacts.json",
            &(serde_json::to_string_pretty(&manifest).expect("serialize manifest") + "\n"),
        );
        artifact
    }

    fn current_upgrade_report_fixture(product: &Path, framework: &Path) -> ProductUpgradeReport {
        seed_lock_test_framework(framework);
        write_lock_test_product(product, "written");
        let roots = lock_test_roots(product, framework);
        let lock = build_lock(&roots).expect("build current upgrade fixture lock");
        write_product_lock(product, &product.join("appfw.lock"), &lock_to_toml(&lock))
            .expect("write current upgrade fixture lock");
        let report = product_upgrade_check(&roots).expect("build current upgrade fixture report");
        assert!(report.ok, "fixture upgrade drift: {:#?}", report.checks);
        report
    }

    fn write_root_qualified_lock_product(
        product: &Path,
        config_root: &Path,
        report_root: &Path,
        action: &str,
    ) -> (PathBuf, PathBuf) {
        write_test_file(product, "scripts/appfw", "#!/usr/bin/env bash\n");
        let shared_relative = "generated/shared.yaml";
        let app_artifact = product.join(shared_relative);
        let config_artifact = config_root.join(shared_relative);
        write_test_file(product, shared_relative, "value: shared\n");
        write_test_file(config_root, shared_relative, "value: shared\n");
        let manifest = json!([
            artifact_manifest_record(&app_artifact, action),
            artifact_manifest_record(&config_artifact, action),
        ]);
        write_test_file(
            report_root,
            "artifacts.json",
            &(serde_json::to_string_pretty(&manifest).expect("serialize manifest") + "\n"),
        );
        (app_artifact, config_artifact)
    }

    #[test]
    fn fabric_artifact_lock_identity_is_stable_after_relocation_and_regeneration() {
        let root = unique_test_root("fabric-portable-artifact-lock");
        let framework = root.join("framework");
        let producer = root.join("producer/fabric");
        let relocated = root.join("relocated/fabric");
        seed_lock_test_framework(&framework);
        write_lock_test_product(&producer, "written");
        write_lock_test_product(&relocated, "skipped");

        let producer_roots = lock_test_roots(&producer, &framework);
        let relocated_roots = lock_test_roots(&relocated, &framework);
        let producer_lock = build_lock(&producer_roots).expect("build producer lock");
        let relocated_lock = build_lock(&relocated_roots).expect("build relocated lock");
        assert_eq!(producer_lock.version, APPFW_LOCK_VERSION);
        assert_eq!(
            producer_lock.artifact_manifest_identity_schema,
            ARTIFACT_MANIFEST_IDENTITY_SCHEMA
        );
        assert_eq!(
            producer_lock.artifact_manifest_hash, relocated_lock.artifact_manifest_hash,
            "checkout roots and run-local actions must not change portable artifact identity"
        );

        fs::write(relocated.join("appfw.lock"), lock_to_toml(&producer_lock))
            .expect("copy producer lock to relocated checkout");
        let report = product_upgrade_check(&relocated_roots).expect("check relocated product");
        assert!(report.ok, "relocated product drift: {:#?}", report.checks);
        assert_eq!(report.status, "current");
        assert!(report.checks.iter().all(|check| check.ok));

        let prior_identity_lock = lock_to_toml(&producer_lock).replace(
            "appfw.artifact_manifest_identity@2",
            "appfw.artifact_manifest_identity@1",
        );
        fs::write(relocated.join("appfw.lock"), prior_identity_lock)
            .expect("write prior identity-schema lock");
        let identity_migration =
            product_upgrade_check(&relocated_roots).expect("check identity-schema migration");
        assert!(!identity_migration.ok);
        assert_eq!(identity_migration.status, "lock_migration_required");
        assert!(identity_migration
            .checks
            .iter()
            .any(|check| check.name == "artifact_manifest_identity_schema" && !check.ok));

        let legacy_lock = lock_to_toml(&producer_lock)
            .replace("version = 2", "version = 1")
            .lines()
            .filter(|line| !line.starts_with("artifact_manifest_identity_schema ="))
            .collect::<Vec<_>>()
            .join("\n")
            + "\n";
        fs::write(relocated.join("appfw.lock"), legacy_lock).expect("write legacy lock");
        let migration = product_upgrade_check(&relocated_roots).expect("check v1 migration");
        assert!(!migration.ok);
        assert_eq!(migration.status, "lock_migration_required");
        assert!(migration
            .checks
            .iter()
            .filter(|check| matches!(
                check.name.as_str(),
                "lock_version" | "artifact_manifest_identity_schema"
            ))
            .all(|check| !check.ok));
        assert!(migration
            .recommended_commands
            .iter()
            .any(|command| command == "scripts/appfw product lock --write"));

        fs::remove_dir_all(root).expect("remove portable artifact lock fixture");
    }

    #[test]
    fn root_qualified_identity_is_stable_across_standard_and_external_config_layouts() {
        let root = unique_test_root("root-qualified-artifact-lock");
        let framework = root.join("framework");
        let standard_product = root.join("standard/product");
        let external_product = root.join("external/product");
        let standard_config = standard_product.join(".appfw/model");
        let standard_report = standard_product.join(PRODUCT_REPORT_ROOT);
        let external_config = root.join("external/config");
        let external_report = root.join("external/report");
        seed_lock_test_framework(&framework);
        let (standard_app_artifact, standard_config_artifact) = write_root_qualified_lock_product(
            &standard_product,
            &standard_config,
            &standard_report,
            "written",
        );
        let (external_app_artifact, external_config_artifact) = write_root_qualified_lock_product(
            &external_product,
            &external_config,
            &external_report,
            "skipped",
        );
        assert_eq!(
            standard_app_artifact
                .strip_prefix(&standard_product)
                .expect("standard app artifact is app-relative"),
            standard_config_artifact
                .strip_prefix(&standard_config)
                .expect("standard config artifact is config-relative"),
            "same-named app/config records must remain distinct by root qualification"
        );
        assert_eq!(
            external_app_artifact
                .strip_prefix(&external_product)
                .expect("external app artifact is app-relative"),
            external_config_artifact
                .strip_prefix(&external_config)
                .expect("external config artifact is config-relative"),
            "same-named external app/config records must remain distinct by root qualification"
        );

        let standard_roots = lock_test_roots(&standard_product, &framework);
        let mut external_roots = lock_test_roots(&external_product, &framework);
        external_roots.config_root = external_config.clone();
        external_roots.report_root = external_report;
        let standard = build_lock(&standard_roots).expect("build standard-layout lock");
        let external = build_lock(&external_roots).expect("build external-layout lock");
        assert_eq!(
            standard.artifact_manifest_identity_schema,
            "appfw.artifact_manifest_identity@2"
        );
        assert_eq!(
            standard.artifact_manifest_hash, external.artifact_manifest_hash,
            "root qualification must remain stable across configured-root relocation"
        );

        fs::write(&external_config_artifact, "tampered: true\n")
            .expect("tamper external config artifact after manifest");
        let error = build_lock(&external_roots)
            .expect_err("tampered external config artifact must fail closed");
        assert!(error.contains("does not match final bytes"), "{error}");

        fs::remove_dir_all(root).expect("remove root-qualified identity fixture");
    }

    #[cfg(unix)]
    #[test]
    fn root_qualified_identity_rejects_symlinked_external_config_root() {
        use std::os::unix::fs::symlink;

        let root = unique_test_root("root-qualified-config-symlink");
        let framework = root.join("framework");
        let product = root.join("product");
        let real_config = root.join("real-config");
        let config_alias = root.join("config-alias");
        let report_root = root.join("report");
        seed_lock_test_framework(&framework);
        write_root_qualified_lock_product(&product, &real_config, &report_root, "written");
        write_test_file(&real_config, "sentinel", "unchanged\n");
        symlink(&real_config, &config_alias).expect("create external config-root symlink");
        let mut roots = lock_test_roots(&product, &framework);
        roots.config_root = config_alias.clone();
        roots.report_root = report_root;

        let error = build_lock(&roots).expect_err("symlinked config root must fail closed");
        assert!(error.contains("non-symlink directory"), "{error}");
        assert_eq!(
            fs::read_to_string(real_config.join("sentinel")).expect("read outside sentinel"),
            "unchanged\n"
        );
        assert!(!product.join("appfw.lock").exists());

        fs::remove_file(config_alias).expect("remove config-root symlink");
        fs::remove_dir_all(root).expect("remove config-root symlink fixture");
    }

    #[test]
    fn lock_v2_accepts_artifact_manifest_in_configured_external_report_root() {
        let root = unique_test_root("external-report-lock");
        let framework = root.join("framework");
        let product = root.join("product");
        let report_root = root.join("reports/appfw");
        seed_lock_test_framework(&framework);
        let artifact = write_lock_test_product(&product, "written");
        let manifest = json!([artifact_manifest_record(&artifact, "written")]);
        write_test_file(
            &report_root,
            "artifacts.json",
            &(serde_json::to_string_pretty(&manifest).expect("serialize manifest") + "\n"),
        );
        let mut roots = lock_test_roots(&product, &framework);
        roots.report_root = report_root;

        let lock = build_lock(&roots).expect("build lock from external report root");
        assert!(lock.artifact_manifest_hash.is_some());
        write_product_lock(&product, &product.join("appfw.lock"), &lock_to_toml(&lock))
            .expect("write product-contained lock");
        let parsed =
            parse_lock(&product, &product.join("appfw.lock")).expect("parse external-report lock");
        assert_eq!(parsed.get("version").map(String::as_str), Some("2"));

        fs::remove_dir_all(root).expect("remove external report lock fixture");
    }

    #[test]
    fn full_generation_with_external_config_and_report_roots_locks_and_upgrades() {
        let lexical_root = unique_test_root("full-external-roots-lock");
        fs::create_dir_all(&lexical_root).expect("create full-generation fixture root");
        let root = fs::canonicalize(&lexical_root).expect("canonicalize full-generation fixture");
        let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .expect("app_gen has repository parent")
            .to_path_buf();
        let product = root.join("product");
        let external_config = root.join("configured/model");
        let external_report = root.join("configured/report");
        let profile = new_profile("crm-sample").expect("CRM sample profile");
        copy_product_template(&repo_root, &product, &profile).expect("copy CRM product fixture");
        rewrite_split_root_cargo_paths(&product, &repo_root)
            .expect("bind copied product to framework fixture");
        fs::create_dir_all(
            external_config
                .parent()
                .expect("external config has parent"),
        )
        .expect("create external configured-root parent");
        fs::rename(product.join(".appfw/model"), &external_config)
            .expect("relocate product model to configured external root");

        let codegen_roots = appfw_codegen::CodegenRoots::new(
            &product,
            &repo_root,
            repo_root.join("app_gen"),
            &external_config,
            repo_root.join("app_gen/_templates"),
            &external_report,
        );
        let generation = appfw_codegen::generate(codegen_roots)
            .expect("full generation with external config/report roots");
        assert!(generation.generated);

        let manifest_bytes = fs::read(external_report.join("artifacts.json"))
            .expect("read generated external manifest");
        let records =
            serde_json::from_slice::<Vec<ArtifactManifestRecordForIdentity>>(&manifest_bytes)
                .expect("parse real generated artifact records");
        assert!(
            records
                .iter()
                .any(|record| Path::new(&record.path).starts_with(&product)),
            "real manifest must contain app-root artifacts"
        );
        assert!(
            records
                .iter()
                .any(|record| Path::new(&record.path).starts_with(&external_config)),
            "real manifest must contain configured-root artifacts"
        );

        let roots = CommandRoots {
            app_root: product.clone(),
            framework_root: repo_root.clone(),
            generator_root: repo_root.join("app_gen"),
            config_root: external_config.clone(),
            templates_root: repo_root.join("app_gen/_templates"),
            report_root: external_report.clone(),
        };
        let lock = build_lock(&roots).expect("build lock from real external-root manifest");
        assert_eq!(
            lock.artifact_manifest_identity_schema,
            "appfw.artifact_manifest_identity@2"
        );
        assert!(lock.artifact_manifest_hash.is_some());
        write_product_lock(&product, &product.join("appfw.lock"), &lock_to_toml(&lock))
            .expect("write external-root lock");
        let upgrade = product_upgrade_check(&roots).expect("check external-root upgrade");
        assert!(
            upgrade.ok,
            "external-root upgrade drift: {:#?}",
            upgrade.checks
        );
        assert_eq!(upgrade.status, "current");
        assert!(upgrade.checks.iter().all(|check| check.ok));

        fs::remove_dir_all(root).expect("remove full external-root fixture");
    }

    #[test]
    fn lock_v2_rejects_existing_filesystem_root_as_report_root() {
        let root = unique_test_root("filesystem-report-root-lock");
        let framework = root.join("framework");
        let product = root.join("product");
        seed_lock_test_framework(&framework);
        write_lock_test_product(&product, "written");
        let mut roots = lock_test_roots(&product, &framework);
        roots.report_root = product
            .ancestors()
            .last()
            .expect("product path has a filesystem root")
            .to_path_buf();

        let error = build_lock(&roots).expect_err("filesystem report root must fail closed");
        assert!(error.contains("cannot be a filesystem root"), "{error}");
        assert!(!product.join("appfw.lock").exists());

        fs::remove_dir_all(root).expect("remove filesystem report-root fixture");
    }

    #[cfg(unix)]
    #[test]
    fn lock_v2_rejects_app_root_symlink_ancestor_before_workflow_reads() {
        use std::os::unix::fs::{symlink, PermissionsExt};

        let root = unique_test_root("app-root-lock-symlink");
        let framework = root.join("framework");
        let target = root.join("target");
        let real_product = target.join("product");
        let alias = root.join("alias");
        let report_root = root.join("report");
        seed_lock_test_framework(&framework);
        write_lock_test_product(&real_product, "written");
        fs::create_dir_all(&report_root).expect("create independent report root");
        let workflow_path = real_product.join("scripts/appfw");
        let mut unreadable = fs::metadata(&workflow_path)
            .expect("inspect workflow fixture")
            .permissions();
        unreadable.set_mode(0o000);
        fs::set_permissions(&workflow_path, unreadable).expect("make workflow fixture unreadable");
        symlink(&target, &alias).expect("create app-root ancestor symlink");

        let mut roots = lock_test_roots(&alias.join("product"), &framework);
        roots.config_root = real_product.join(".appfw/model");
        roots.report_root = report_root;
        let error = build_lock(&roots)
            .expect_err("app-root symlink ancestor must fail before workflow hashing");
        assert!(error.contains("symlink ancestors"), "{error}");

        let mut restored = fs::metadata(&workflow_path)
            .expect("reinspect workflow fixture")
            .permissions();
        restored.set_mode(0o600);
        fs::set_permissions(&workflow_path, restored).expect("restore workflow fixture access");
        assert_eq!(
            fs::read_to_string(&workflow_path).expect("read restored workflow fixture"),
            "#!/usr/bin/env bash\n"
        );
        assert!(!real_product.join("appfw.lock").exists());

        fs::remove_file(alias).expect("remove app-root ancestor symlink");
        fs::remove_dir_all(root).expect("remove app-root symlink fixture");
    }

    #[cfg(unix)]
    #[test]
    fn lock_v2_rejects_symlinked_report_config_contract_without_outside_read() {
        use std::os::unix::fs::symlink;

        let root = unique_test_root("report-contract-symlink");
        let framework = root.join("framework");
        let product = root.join("product");
        let report_root = root.join("report");
        let outside = root.join("outside-config-contract.json");
        seed_lock_test_framework(&framework);
        write_lock_test_product(&product, "written");
        fs::create_dir_all(&report_root).expect("create external report root");
        fs::write(&outside, "outside sentinel\n").expect("write outside config-contract sentinel");
        symlink(&outside, report_root.join("config_contract.json"))
            .expect("create report config-contract symlink");
        let mut roots = lock_test_roots(&product, &framework);
        roots.report_root = report_root.clone();

        let error = build_lock(&roots)
            .expect_err("symlinked report config contract must fail closed before reading");
        assert!(
            error.contains("evidence path contains a symlink"),
            "{error}"
        );
        assert_eq!(
            fs::read_to_string(&outside).expect("read outside config-contract sentinel"),
            "outside sentinel\n"
        );
        assert!(!product.join("appfw.lock").exists());

        fs::remove_file(report_root.join("config_contract.json"))
            .expect("remove report config-contract symlink");
        fs::remove_dir_all(root).expect("remove report config-contract fixture");
    }

    #[cfg(unix)]
    #[test]
    fn lock_v2_rejects_external_report_root_symlink_ancestor_without_outside_write() {
        use std::os::unix::fs::symlink;

        let root = unique_test_root("external-report-lock-symlink");
        let framework = root.join("framework");
        let product = root.join("product");
        let target = root.join("target");
        let alias = root.join("alias");
        seed_lock_test_framework(&framework);
        let artifact = write_lock_test_product(&product, "written");
        let manifest = json!([artifact_manifest_record(&artifact, "written")]);
        write_test_file(
            &target,
            "reports/artifacts.json",
            &(serde_json::to_string_pretty(&manifest).expect("serialize manifest") + "\n"),
        );
        fs::create_dir_all(target.join("reports/config_contract.json"))
            .expect("create unreadable-as-file config-contract sentinel");
        write_test_file(&target, "sentinel", "unchanged\n");
        symlink(&target, &alias).expect("create report-root ancestor symlink");
        let mut roots = lock_test_roots(&product, &framework);
        roots.report_root = alias.join("reports");

        let error =
            build_lock(&roots).expect_err("external report-root symlink ancestor must fail closed");
        assert!(error.contains("symlink ancestors"), "{error}");
        assert!(!product.join("appfw.lock").exists());
        assert_eq!(
            fs::read_to_string(target.join("sentinel")).expect("read outside sentinel"),
            "unchanged\n"
        );

        fs::remove_file(alias).expect("remove report-root ancestor symlink");
        fs::remove_dir_all(root).expect("remove external report symlink fixture");
    }

    #[test]
    fn artifact_manifest_identity_hashes_every_authoritative_field_but_not_action() {
        let root = unique_test_root("artifact-identity-fields");
        let product = root.join("fabric");
        let artifact = write_lock_test_product(&product, "written");
        let manifest_path = product.join(".appfw/target/appfw/artifacts.json");
        let baseline = artifact_manifest_identity_hash_optional(&product, &manifest_path)
            .expect("baseline identity");

        let mut record = artifact_manifest_record(&artifact, "skipped");
        write_test_file(
            &product,
            ".appfw/target/appfw/artifacts.json",
            &(serde_json::to_string_pretty(&json!([record.clone()])).unwrap() + "\n"),
        );
        assert_eq!(
            baseline,
            artifact_manifest_identity_hash_optional(&product, &manifest_path)
                .expect("action-independent identity")
        );

        for (field, value) in [
            ("ownership", json!("human_owned")),
            ("overwrite", json!("if_missing")),
            ("source_sha256", json!(format!("sha256:{}", "4".repeat(64)))),
        ] {
            record[field] = value;
            write_test_file(
                &product,
                ".appfw/target/appfw/artifacts.json",
                &(serde_json::to_string_pretty(&json!([record.clone()])).unwrap() + "\n"),
            );
            assert_ne!(
                baseline,
                artifact_manifest_identity_hash_optional(&product, &manifest_path)
                    .unwrap_or_else(|error| panic!("hash changed {field}: {error}")),
                "authoritative field {field} must affect identity"
            );
            record = artifact_manifest_record(&artifact, "skipped");
        }

        fs::write(&artifact, "different authoritative bytes\n")
            .expect("mutate authoritative artifact fixture");
        record = artifact_manifest_record(&artifact, "skipped");
        write_test_file(
            &product,
            ".appfw/target/appfw/artifacts.json",
            &(serde_json::to_string_pretty(&json!([record])).unwrap() + "\n"),
        );
        assert_ne!(
            baseline,
            artifact_manifest_identity_hash_optional(&product, &manifest_path)
                .expect("content-bound identity"),
            "verified final artifact bytes must affect identity"
        );

        fs::remove_dir_all(root).expect("remove artifact identity field fixture");
    }

    #[test]
    fn artifact_manifest_identity_rejects_tamper_after_manifest_before_lock_write() {
        let root = unique_test_root("artifact-identity-tamper-after-manifest");
        let product = root.join("fabric");
        let artifact = write_lock_test_product(&product, "written");
        let manifest_path = product.join(".appfw/target/appfw/artifacts.json");
        fs::write(&artifact, "tampered after manifest\n").expect("tamper artifact bytes");

        let error = artifact_manifest_identity_hash_optional(&product, &manifest_path)
            .expect_err("tamper after manifest must fail closed");
        assert!(error.contains("does not match final bytes"), "{error}");
        assert!(!product.join("appfw.lock").exists());

        fs::remove_dir_all(root).expect("remove tamper fixture");
    }

    #[test]
    fn artifact_manifest_identity_rejects_outside_app_root_before_lock_write() {
        let root = unique_test_root("artifact-identity-escape");
        let product = root.join("fabric");
        let outside = root.join("outside.rs");
        write_test_file(&root, "outside.rs", "// outside sentinel\n");
        write_lock_test_product(&product, "written");
        let manifest_path = product.join(".appfw/target/appfw/artifacts.json");
        write_test_file(
            &product,
            ".appfw/target/appfw/artifacts.json",
            &(serde_json::to_string_pretty(&json!([artifact_manifest_record(
                &outside, "written"
            )]))
            .unwrap()
                + "\n"),
        );

        let error = artifact_manifest_identity_hash_optional(&product, &manifest_path)
            .expect_err("outside artifact path must fail closed");
        assert!(
            error.contains("outside configured app/config roots"),
            "unexpected error: {error}"
        );
        assert_eq!(
            fs::read_to_string(&outside).expect("read outside sentinel"),
            "// outside sentinel\n"
        );
        assert!(!product.join("appfw.lock").exists());

        fs::remove_dir_all(root).expect("remove artifact identity escape fixture");
    }

    #[cfg(unix)]
    #[test]
    fn artifact_manifest_and_lock_reject_symlinked_ancestors_without_outside_write() {
        use std::os::unix::fs::symlink;

        let root = unique_test_root("portable-evidence-symlink-ancestors");
        let product = root.join("product");
        let outside = root.join("outside");
        fs::create_dir_all(product.join(".appfw")).expect("create product metadata root");
        fs::create_dir_all(outside.join("appfw")).expect("create outside artifact root");
        write_test_file(&outside, "sentinel", "unchanged\n");
        write_test_file(&outside, "appfw/artifacts.json", "[]\n");
        symlink(outside.join("appfw"), product.join(".appfw/target"))
            .expect("create artifact manifest ancestor symlink");

        let manifest_path = product.join(".appfw/target/artifacts.json");
        let manifest_error = artifact_manifest_identity_hash_optional(&product, &manifest_path)
            .expect_err("manifest ancestor symlink must fail closed");
        assert!(
            manifest_error.contains("contains a symlink"),
            "{manifest_error}"
        );
        assert_eq!(
            fs::read_to_string(outside.join("sentinel")).expect("read outside sentinel"),
            "unchanged\n"
        );

        let real_product = root.join("real-product");
        fs::create_dir_all(&real_product).expect("create real product");
        let product_alias = root.join("product-alias");
        symlink(&real_product, &product_alias).expect("create product-root symlink");
        let lock_error = write_product_lock(
            &product_alias,
            &product_alias.join("appfw.lock"),
            "version = 2\n",
        )
        .expect_err("lock output through a symlinked product root must fail closed");
        assert!(lock_error.contains("non-symlink directory"), "{lock_error}");
        assert!(!real_product.join("appfw.lock").exists());

        fs::remove_file(product.join(".appfw/target")).expect("remove manifest symlink");
        fs::remove_file(product_alias).expect("remove product alias");
        fs::remove_dir_all(root).expect("remove symlink ancestor fixture");
    }

    #[test]
    fn artifact_manifest_identity_rejects_duplicate_noncanonical_and_unknown_records() {
        let root = unique_test_root("artifact-identity-adversarial");
        let product = root.join("fabric");
        let artifact = write_lock_test_product(&product, "written");
        let manifest_path = product.join(".appfw/target/appfw/artifacts.json");
        let record = artifact_manifest_record(&artifact, "written");

        write_test_file(
            &product,
            ".appfw/target/appfw/artifacts.json",
            &(serde_json::to_string_pretty(&json!([record.clone(), record.clone()])).unwrap()
                + "\n"),
        );
        let duplicate = artifact_manifest_identity_hash_optional(&product, &manifest_path)
            .expect_err("duplicate normalized paths must fail closed");
        assert!(duplicate.contains("duplicate normalized path"));

        let noncanonical_path = format!(
            "{}/./{}",
            artifact.parent().unwrap().display(),
            artifact.file_name().unwrap().to_string_lossy()
        );
        let mut noncanonical = record.clone();
        noncanonical["path"] = json!(noncanonical_path);
        write_test_file(
            &product,
            ".appfw/target/appfw/artifacts.json",
            &(serde_json::to_string_pretty(&json!([noncanonical])).unwrap() + "\n"),
        );
        let noncanonical = artifact_manifest_identity_hash_optional(&product, &manifest_path)
            .expect_err("dot path must fail closed");
        assert!(noncanonical.contains("not canonical"));

        let mut unknown = record;
        unknown["future_semantic_field"] = json!("must-not-be-unhashed");
        write_test_file(
            &product,
            ".appfw/target/appfw/artifacts.json",
            &(serde_json::to_string_pretty(&json!([unknown])).unwrap() + "\n"),
        );
        let unknown = artifact_manifest_identity_hash_optional(&product, &manifest_path)
            .expect_err("unknown semantic field must fail closed");
        assert!(unknown.contains("closed generated record schema"));
        assert!(unknown.contains("unknown field"));

        fs::remove_dir_all(root).expect("remove artifact identity adversarial fixture");
    }

    #[cfg(unix)]
    #[test]
    fn product_upgrade_report_rejects_symlinked_ancestor_without_outside_write() {
        use std::os::unix::fs::symlink;

        let root = unique_test_root("upgrade-report-ancestor-symlink");
        let framework = root.join("framework");
        let product = root.join("product");
        let outside_target = root.join("outside-target");
        let outside_report = outside_target.join("appfw/product-upgrade.json");
        let report = current_upgrade_report_fixture(&product, &framework);
        write_product_upgrade_report(&product, &report)
            .expect("write bounded report with missing parent directories");
        assert!(product.join("target/appfw/product-upgrade.json").is_file());
        fs::remove_dir_all(product.join("target")).expect("remove initial bounded report fixture");
        write_test_file(
            &outside_target,
            "appfw/product-upgrade.json",
            "outside report sentinel\n",
        );
        symlink(&outside_target, product.join("target"))
            .expect("create upgrade report ancestor symlink");

        let error = write_product_upgrade_report(&product, &report)
            .expect_err("symlinked report ancestor must fail closed");
        assert!(error.contains("symlink"), "{error}");
        assert_eq!(
            fs::read_to_string(&outside_report).expect("read outside report sentinel"),
            "outside report sentinel\n"
        );

        fs::remove_file(product.join("target")).expect("remove report ancestor symlink");
        fs::remove_dir_all(root).expect("remove report ancestor fixture");
    }

    #[cfg(unix)]
    #[test]
    fn product_upgrade_report_rejects_symlinked_leaf_without_outside_write() {
        use std::os::unix::fs::symlink;

        let root = unique_test_root("upgrade-report-leaf-symlink");
        let framework = root.join("framework");
        let product = root.join("product");
        let outside_report = root.join("outside-product-upgrade.json");
        let report = current_upgrade_report_fixture(&product, &framework);
        fs::create_dir_all(product.join("target/appfw")).expect("create upgrade report parent");
        fs::write(&outside_report, "outside report sentinel\n")
            .expect("write outside report sentinel");
        symlink(
            &outside_report,
            product.join("target/appfw/product-upgrade.json"),
        )
        .expect("create upgrade report leaf symlink");

        let error = write_product_upgrade_report(&product, &report)
            .expect_err("symlinked report leaf must fail closed");
        assert!(error.contains("symlink"), "{error}");
        assert_eq!(
            fs::read_to_string(&outside_report).expect("read outside report sentinel"),
            "outside report sentinel\n"
        );

        fs::remove_file(product.join("target/appfw/product-upgrade.json"))
            .expect("remove report leaf symlink");
        fs::remove_dir_all(root).expect("remove report leaf fixture");
    }

    #[cfg(unix)]
    #[test]
    fn product_upgrade_report_rejects_hardlinked_leaf_without_outside_write() {
        use std::os::unix::fs::MetadataExt;

        let root = unique_test_root("upgrade-report-hardlink");
        let framework = root.join("framework");
        let product = root.join("product");
        let outside_report = root.join("outside-product-upgrade.json");
        let report_path = product.join("target/appfw/product-upgrade.json");
        let report = current_upgrade_report_fixture(&product, &framework);
        fs::create_dir_all(report_path.parent().expect("report path has parent"))
            .expect("create upgrade report parent");
        fs::write(&outside_report, "outside report hardlink sentinel\n")
            .expect("write outside report hardlink sentinel");
        fs::hard_link(&outside_report, &report_path).expect("create report hardlink alias");
        assert_eq!(
            fs::symlink_metadata(&report_path)
                .expect("inspect report hardlink")
                .nlink(),
            2
        );

        let error = write_product_upgrade_report(&product, &report)
            .expect_err("hardlinked report leaf must fail closed");
        assert!(error.contains("exactly one hard link"), "{error}");
        assert_eq!(
            fs::read_to_string(&outside_report).expect("read outside report sentinel"),
            "outside report hardlink sentinel\n"
        );

        fs::remove_dir_all(root).expect("remove report hardlink fixture");
    }

    #[cfg(unix)]
    #[test]
    fn product_lock_rejects_hardlinked_leaf_without_outside_write() {
        use std::os::unix::fs::MetadataExt;

        let root = unique_test_root("product-lock-hardlink");
        let product = root.join("product");
        let outside_lock = root.join("outside-appfw.lock");
        let lock_path = product.join("appfw.lock");
        fs::create_dir_all(&product).expect("create hardlink lock product root");
        fs::write(&outside_lock, "outside lock hardlink sentinel\n")
            .expect("write outside lock hardlink sentinel");
        fs::hard_link(&outside_lock, &lock_path).expect("create lock hardlink alias");
        assert_eq!(
            fs::symlink_metadata(&lock_path)
                .expect("inspect lock hardlink")
                .nlink(),
            2
        );

        let error = write_product_lock(&product, &lock_path, "version = 2\n")
            .expect_err("hardlinked lock leaf must fail closed");
        assert!(error.contains("exactly one hard link"), "{error}");
        assert_eq!(
            fs::read_to_string(&outside_lock).expect("read outside lock sentinel"),
            "outside lock hardlink sentinel\n"
        );

        fs::remove_dir_all(root).expect("remove lock hardlink fixture");
    }

    #[cfg(unix)]
    #[test]
    fn lock_parser_rejects_symlinked_leaf_without_outside_read() {
        use std::os::unix::fs::symlink;

        let root = unique_test_root("lock-read-leaf-symlink");
        let product = root.join("product");
        let outside_lock = root.join("outside-appfw.lock");
        fs::create_dir_all(&product).expect("create lock-read product root");
        fs::write(&outside_lock, "outside lock sentinel\n").expect("write outside lock sentinel");
        symlink(&outside_lock, product.join("appfw.lock")).expect("create appfw.lock leaf symlink");

        let error = parse_lock(&product, &product.join("appfw.lock"))
            .expect_err("symlinked appfw.lock leaf must fail before reading");
        assert!(error.contains("symlink"), "{error}");
        assert!(!error.contains("malformed TOML"), "{error}");
        assert_eq!(
            fs::read_to_string(&outside_lock).expect("read outside lock sentinel"),
            "outside lock sentinel\n"
        );

        fs::remove_file(product.join("appfw.lock")).expect("remove lock leaf symlink");
        fs::remove_dir_all(root).expect("remove lock leaf fixture");
    }

    #[cfg(unix)]
    #[test]
    fn lock_parser_rejects_symlinked_ancestor_without_outside_read() {
        use std::os::unix::fs::symlink;

        let root = unique_test_root("lock-read-ancestor-symlink");
        let real_parent = root.join("real-parent");
        let real_product = real_parent.join("product");
        let alias_parent = root.join("alias-parent");
        let outside_lock = real_product.join("appfw.lock");
        fs::create_dir_all(&real_product).expect("create real product root");
        fs::write(&outside_lock, "outside lock sentinel\n").expect("write outside lock sentinel");
        symlink(&real_parent, &alias_parent).expect("create product ancestor symlink");
        let product_alias = alias_parent.join("product");

        let error = parse_lock(&product_alias, &product_alias.join("appfw.lock"))
            .expect_err("symlinked app root ancestor must fail before lock read");
        assert!(error.contains("symlink ancestors"), "{error}");
        assert!(!error.contains("malformed TOML"), "{error}");
        assert_eq!(
            fs::read_to_string(&outside_lock).expect("read outside lock sentinel"),
            "outside lock sentinel\n"
        );

        fs::remove_file(alias_parent).expect("remove product ancestor symlink");
        fs::remove_dir_all(root).expect("remove lock ancestor fixture");
    }

    #[test]
    fn lock_parser_rejects_duplicate_malformed_and_unknown_fields() {
        let root = unique_test_root("lock-parser-adversarial");
        fs::create_dir_all(&root).expect("create lock parser root");
        let lock_path = root.join("appfw.lock");

        for (contents, expected) in [
            ("version = 2\nversion = 2\n", "malformed TOML"),
            ("version 2\n", "malformed TOML"),
            ("version = 2\nfuture_field = \"no\"\n", "unknown field"),
            ("version = \"2\"\n", "non-negative integer"),
        ] {
            fs::write(&lock_path, contents).expect("write adversarial lock");
            let error = parse_lock(&root, &lock_path).expect_err("invalid lock must fail closed");
            assert!(error.contains(expected), "expected {expected}: {error}");
        }

        fs::remove_dir_all(root).expect("remove lock parser fixture");
    }

    #[test]
    fn lock_parser_rejects_input_above_bounded_size_limit() {
        let root = unique_test_root("lock-parser-size-limit");
        fs::create_dir_all(&root).expect("create lock size-limit root");
        let lock_path = root.join("appfw.lock");
        fs::write(&lock_path, vec![b'#'; APPFW_LOCK_MAX_BYTES as usize + 1])
            .expect("write oversized lock fixture");

        let error = parse_lock(&root, &lock_path).expect_err("oversized lock must fail closed");
        assert!(error.contains("reading stops"), "{error}");
        assert!(error.contains(&APPFW_LOCK_MAX_BYTES.to_string()), "{error}");

        fs::remove_dir_all(root).expect("remove lock size-limit fixture");
    }

    #[cfg(not(unix))]
    #[test]
    fn existing_output_fails_closed_without_stable_link_count() {
        let root = unique_test_root("existing-output-link-count");
        fs::create_dir_all(&root).expect("create existing-output root");
        let path = root.join("appfw.lock");
        fs::write(&path, "outside-safe fixture\n").expect("write existing-output fixture");
        let metadata = fs::symlink_metadata(&path).expect("inspect existing-output fixture");

        let error = require_unaliased_output_leaf(&metadata, true, "appfw.lock", &path)
            .expect_err("platform without link count must reject existing output");
        assert!(error.contains("does not expose a stable hard-link count"));

        fs::remove_dir_all(root).expect("remove existing-output fixture");
    }

    #[test]
    fn lock_v2_explicitly_records_missing_artifact_manifest() {
        let root = unique_test_root("lock-v2-missing-manifest");
        let framework = root.join("framework");
        let product = root.join("product");
        seed_lock_test_framework(&framework);
        write_test_file(&product, "scripts/appfw", "#!/usr/bin/env bash\n");
        fs::create_dir_all(product.join(".appfw/model")).expect("create product config root");
        let roots = lock_test_roots(&product, &framework);

        let lock = build_lock(&roots).expect("build pre-generation lock");
        assert_eq!(lock.version, APPFW_LOCK_VERSION);
        assert_eq!(lock.artifact_manifest_hash, None);
        write_product_lock(&product, &product.join("appfw.lock"), &lock_to_toml(&lock))
            .expect("write pre-generation lock");
        let parsed = parse_lock(&product, &product.join("appfw.lock")).expect("parse v2 lock");
        assert_eq!(parsed.get("version").map(String::as_str), Some("2"));
        assert_eq!(
            parsed
                .get("artifact_manifest_identity_schema")
                .map(String::as_str),
            Some(ARTIFACT_MANIFEST_IDENTITY_SCHEMA)
        );
        assert_eq!(
            parsed.get("artifact_manifest_hash").map(String::as_str),
            Some("")
        );

        fs::remove_dir_all(root).expect("remove missing manifest lock fixture");
    }

    fn seed_product_intake_test_framework(framework: &Path) {
        for relative in [
            "app_gen/_config/_facets/audit_entity_type.yaml",
            "app_gen/_config/_facets/audit_property.yaml",
            "app_gen/_config/_facets/schema_entity_type_related.yaml",
            "app_gen/_config/_facets/soft_deleted_property.yaml",
            "app_gen/_config/_facets/version_property.yaml",
            "app_gen/_config/_fragments/property-primary-key-uuid.yaml",
            "app_gen/_config/_fragments/property-string-caption.yaml",
            "app_gen/_config/schemas/system/entity_types/entity_type.yaml",
            "app_gen/_config/schemas/system/gql_enum_types/data_type.yaml",
            "app_gen/_config/schemas/system/relationships/core.yaml",
            "app_gen/_config/schemas/system/rbac/user.rego",
        ] {
            write_test_file(framework, relative, "[]\n");
        }

        for relative in [
            "backend/src/admin_ui.rs",
            "backend/src/config/app_config.rs",
            "backend/src/config/loader.rs",
            "backend/src/config/mod.rs",
            "backend/src/data/clients/database_client.rs",
            "backend/src/data/clients/mod.rs",
            "backend/src/data/clients/postgres/README.md",
            "backend/src/data/clients/postgres/cte.rs",
            "backend/src/data/clients/postgres/filter.rs",
            "backend/src/data/clients/postgres/mod.rs",
            "backend/src/data/clients/postgres/postgres_client.rs",
            "backend/src/data/data_access.rs",
            "backend/src/data/mod.rs",
            "backend/src/data/query_ir.rs",
            "backend/src/data/rules/mod.rs",
            "backend/src/handlers/selections.rs",
            "backend/src/kafka.rs",
            "backend/src/mcp/mod.rs",
            "backend/src/product_api.rs",
            "backend/src/routes/app_error.rs",
            "backend/src/routes/info.rs",
            "backend/src/schemas/common.rs",
            "backend/src/services/mod.rs",
            "backend/src/sync_workers.rs",
        ] {
            write_test_file(
                framework,
                &format!("examples/products/crm/{relative}"),
                "// product-neutral test scaffold\n",
            );
        }
    }

    #[test]
    fn cli_contract_schema_route_segment_uses_shared_appfw_codegen_converter() {
        use appfw_codegen::schema_route_segment;
        assert_eq!(schema_route_segment("nexus_work"), "nexus-work");
        assert_eq!(schema_route_segment("/nexus_work"), "nexus-work");
        assert_eq!(schema_route_segment("nexus-work"), "nexus-work");
        assert_eq!(schema_route_segment("nexus__work"), "nexus-work");
        assert_eq!(schema_route_segment("nexus_work2"), "nexus-work-2");
        assert_eq!(schema_route_segment("NexusWork"), "nexus-work");
        let shared: fn(&str) -> String = appfw_codegen::schema_route_segment;
        assert_eq!(shared("nexus_work"), schema_route_segment("nexus_work"));
    }

    #[test]
    fn nested_shape_name_handles_array_shapes() {
        assert_eq!(nested_shape_name("array<property>"), Some("property"));
        assert_eq!(
            nested_shape_name("relationship_storage"),
            Some("relationship_storage")
        );
        assert_eq!(nested_shape_name("string"), None);
    }

    #[test]
    fn normalize_key_collapses_punctuation() {
        assert_eq!(
            normalize_key("Many To Many Projection"),
            "many_to_many_projection"
        );
        assert_eq!(
            normalize_key("many-to-many_projection"),
            "many_to_many_projection"
        );
    }

    #[test]
    fn new_app_copy_skips_build_outputs() {
        assert!(should_skip_new_copy(Path::new("backend/target/debug/app")));
        assert!(should_skip_new_copy(Path::new("admin_ui/node_modules")));
        assert!(should_skip_new_copy(Path::new(
            "admin_ui/node_modules/react"
        )));
        assert!(should_skip_new_copy(Path::new("backend/.env")));
        assert!(should_skip_new_copy(Path::new(
            "backend/admin_dist/index.html"
        )));
        assert!(should_skip_new_copy(Path::new(
            "backend/product_dist/index.html"
        )));
        assert!(should_skip_new_copy(Path::new(
            "admin_ui/dist/assets/app.js"
        )));
        assert!(should_skip_new_copy(Path::new("database/Cargo.toml")));
        assert!(should_skip_new_copy(Path::new("database/src/main.rs")));
        assert!(should_skip_new_copy(Path::new("database/data/postgres")));
        assert!(should_skip_new_copy(Path::new(
            "target/appfw/agent-handoff.json"
        )));
        assert!(should_skip_new_copy(Path::new(".appfw/local.env")));
        assert!(should_skip_new_copy(Path::new("appfw.lock")));
        assert!(!should_skip_new_copy(Path::new("Cargo.lock")));
        assert!(should_skip_new_copy(Path::new("backend/Cargo.lock")));
        assert!(!should_skip_new_copy(Path::new("backend/src/main.rs")));
    }

    #[test]
    fn fabric_product_intake_client_uses_framework_template_source() {
        let intake = ProductIntake {
            source_kind: "poc".to_string(),
            app_name: "fabric-insight".to_string(),
            display_name: "Fabric Insight".to_string(),
            schema: "reporting".to_string(),
            provider: "FabricSqlAnalytics".to_string(),
            data_source_name: "fabric_reporting".to_string(),
            mcp_server: false,
            kafka_client: false,
            ui: "none".to_string(),
            poc_source: None,
            data_artifact: None,
            ui_artifact: None,
        };
        let framework = Path::new("/framework");

        let fabric_client_source = product_intake_backend_base_source_path(
            framework,
            &intake,
            "backend/src/data/clients/fabric_sql_analytics/fabric_sql_analytics_client.rs",
        );
        assert_eq!(
            fabric_client_source,
            framework.join(
                "app_gen/_templates/product_intake/backend/src/data/clients/fabric_sql_analytics/fabric_sql_analytics_client.rs"
            )
        );

        let shared_mssql_source = product_intake_backend_base_source_path(
            framework,
            &intake,
            "backend/src/data/clients/mssql/filter.rs",
        );
        assert_eq!(
            shared_mssql_source,
            framework
                .join("app_gen/_templates/product_intake/backend/src/data/clients/mssql/filter.rs")
        );
    }

    #[test]
    fn cli_contract_crm_sample_profile_is_registered() {
        let profile = new_profile("crm-sample").expect("crm-sample profile");
        assert_eq!(profile.name, "crm-sample");
        assert_eq!(profile.template_path, "examples/products/crm");
    }

    #[test]
    fn cli_contract_product_intake_normalizes_topology_answers() {
        let intake = complete_product_intake(
            ProductIntakeArgs {
                source_kind: None,
                app_name: Some("operations-insight".to_string()),
                display_name: Some("Operations Insight".to_string()),
                schema: Some("ops".to_string()),
                provider: Some("postgres".to_string()),
                mcp_server: Some("no".to_string()),
                kafka_client: Some("yes".to_string()),
                ui: Some("requested".to_string()),
                poc_source: Some("../operations-insight-poc".to_string()),
                data_artifact: None,
                ui_artifact: None,
            },
            true,
        )
        .expect("complete intake");

        assert_eq!(intake.provider, "PostgreSQL");
        assert_eq!(intake.source_kind, "poc");
        assert_eq!(intake.data_source_name, "pg_primary");
        assert!(!intake.mcp_server);
        assert!(intake.kafka_client);
        assert_eq!(intake.ui, "enterprise");
        assert_eq!(
            intake.poc_source.as_deref(),
            Some("../operations-insight-poc")
        );
    }

    #[test]
    fn cli_contract_product_intake_rejects_non_runnable_provider_scaffolds() {
        let error = complete_product_intake(
            ProductIntakeArgs {
                source_kind: None,
                app_name: Some("operations-insight".to_string()),
                display_name: Some("Operations Insight".to_string()),
                schema: Some("ops".to_string()),
                provider: Some("mssql".to_string()),
                mcp_server: Some("false".to_string()),
                kafka_client: Some("false".to_string()),
                ui: Some("enterprise".to_string()),
                poc_source: None,
                data_artifact: None,
                ui_artifact: None,
            },
            true,
        )
        .expect_err("non-PostgreSQL intake scaffold should fail until client scaffold exists");

        assert!(error.contains("PostgreSQL only"));
        assert!(error.contains("provider-specific product client scaffold"));
    }

    #[test]
    fn cli_contract_product_intake_reports_missing_required_answers() {
        let error = complete_product_intake(ProductIntakeArgs::default(), true)
            .expect_err("missing intake answers fail");

        assert!(error.contains("app_name"));
        assert!(error.contains("provider"));
        assert!(error.contains("PostgreSQL"));
        assert!(error.contains("Kafka"));
        assert!(error.contains("enterprise"));
    }

    #[test]
    fn cli_contract_product_intake_scaffold_is_product_named_without_crm_residue() {
        let root = env::temp_dir().join(format!(
            "appfw-product-intake-scaffold-test-{}",
            Utc::now().timestamp_nanos_opt().unwrap_or_default()
        ));
        let target = root.join("operations-insight");
        let framework = root.join("framework");
        fs::create_dir_all(&target).expect("create target");
        fs::create_dir_all(&framework).expect("create framework");
        seed_product_intake_test_framework(&framework);

        let intake = complete_product_intake(
            ProductIntakeArgs {
                source_kind: None,
                app_name: Some("operations-insight".to_string()),
                display_name: Some("Operations Insight".to_string()),
                schema: Some("ops".to_string()),
                provider: Some("postgres".to_string()),
                mcp_server: Some("false".to_string()),
                kafka_client: Some("true".to_string()),
                ui: Some("enterprise".to_string()),
                poc_source: Some("../operations-insight-poc".to_string()),
                data_artifact: Some("source.xlsx".to_string()),
                ui_artifact: Some("report.html".to_string()),
            },
            true,
        )
        .expect("complete intake");

        let written =
            write_product_intake_scaffold(&framework, &target, &intake).expect("write scaffold");
        assert!(written.contains(&"Cargo.toml".to_string()));
        assert!(written.contains(&"backend/Cargo.toml".to_string()));
        assert!(written.contains(&"database/README.md".to_string()));
        assert!(written.contains(&"api_tests/Cargo.toml".to_string()));
        assert!(written.contains(&"rego_test/Cargo.toml".to_string()));
        assert!(written.contains(&"frontend/package.json".to_string()));
        assert!(written.contains(&"frontend/.appfw-ui/scaffold-manifest.json".to_string()));
        assert!(written.contains(&"frontend/.appfw-ui/ownership.json".to_string()));
        assert!(written.contains(&"frontend/src/generated/appfw-ui-contract.ts".to_string()));

        for path in &written {
            if path == "frontend/scripts/check-scaffold.mjs" {
                continue;
            }
            let content = fs::read_to_string(target.join(path)).expect("read scaffold file");
            assert!(
                !content.to_ascii_lowercase().contains("crm"),
                "unexpected CRM residue in {path}"
            );
        }
        let residue_report = product_intake_frontend_residue_report(&target, &written, &intake)
            .expect("residue report");
        assert_eq!(residue_report["ok"], true);
        assert_eq!(residue_report["design_system"]["ok"], true);
        assert_eq!(residue_report["matches"].as_array().unwrap().len(), 0);
        let combined = written
            .iter()
            .map(|path| fs::read_to_string(target.join(path)).expect("read scaffold file"))
            .collect::<Vec<_>>()
            .join("\n");
        assert!(combined.contains("Operations Insight"));
        assert!(combined.contains("ops"));
        assert!(combined.contains("schemaLabel: 'Ops'"));
        assert!(combined.contains("Workspace"));
        assert!(combined.contains("PostgreSQL"));
        assert!(combined.contains("appUiContract"));
        assert!(combined.contains("@appfw/pds-health-components"));
        assert!(combined.contains("pdsComponentsRoot"));
        assert!(combined.contains("design_system"));
        assert!(combined.contains("Dialog"));
        assert!(combined.contains("KpiTile"));
        assert!(combined.contains("DataGridShell"));
        assert!(combined.contains("FormLayout"));
        assert!(combined.contains("starter_examples"));
        assert!(combined.contains("pds-component-import"));
        assert!(combined.contains("pds-overlay-example"));
        assert!(combined.contains("pds-analytics-example"));
        assert!(combined.contains("pds-data-grid-example"));
        assert!(combined.contains("pds-generated-form-example"));

        fs::remove_dir_all(root).expect("remove temp");
    }

    #[test]
    fn cli_contract_legacy_modernization_scaffold_writes_modernization_inventory() {
        let root = env::temp_dir().join(format!(
            "appfw-legacy-modernization-scaffold-test-{}",
            Utc::now().timestamp_nanos_opt().unwrap_or_default()
        ));
        let target = root.join("claims-modernization");
        let framework = root.join("framework");
        fs::create_dir_all(&target).expect("create target");
        fs::create_dir_all(&framework).expect("create framework");
        seed_product_intake_test_framework(&framework);

        let intake = complete_product_intake(
            ProductIntakeArgs {
                source_kind: Some("legacy".to_string()),
                app_name: Some("claims-modernization".to_string()),
                display_name: Some("Claims Modernization".to_string()),
                schema: Some("claims".to_string()),
                provider: Some("postgres".to_string()),
                mcp_server: Some("false".to_string()),
                kafka_client: Some("false".to_string()),
                ui: Some("enterprise".to_string()),
                poc_source: Some("../legacy-claims".to_string()),
                data_artifact: Some("readonly-db-inventory.yaml".to_string()),
                ui_artifact: Some("legacy-route-map.md".to_string()),
            },
            true,
        )
        .expect("complete legacy intake");

        assert_eq!(intake.source_kind, "legacy");
        let written =
            write_product_intake_scaffold(&framework, &target, &intake).expect("write scaffold");
        assert!(written.contains(&".appfw/legacy-modernization.yaml".to_string()));

        let intake_yaml =
            fs::read_to_string(target.join(".appfw/poc-intake.yaml")).expect("read intake");
        assert!(intake_yaml.contains("source: legacy_application"));
        assert!(intake_yaml.contains("skill: product-legacy-modernization"));

        let modernization = fs::read_to_string(target.join(".appfw/legacy-modernization.yaml"))
            .expect("read modernization inventory");
        assert!(modernization.contains("static_analysis:"));
        assert!(modernization.contains("data_discovery:"));
        assert!(modernization.contains("stored_procedure_disposition:"));
        assert!(modernization.contains("migration_and_parallel_run:"));
        assert!(modernization.contains("provider_routine_temporary"));
        assert!(modernization.contains("../legacy-claims"));

        let readme = fs::read_to_string(target.join("README.md")).expect("read readme");
        assert!(readme.contains("legacy application modernization"));
        assert!(readme.contains(".appfw/legacy-modernization.yaml"));

        fs::remove_dir_all(root).expect("remove temp");
    }

    #[test]
    fn cli_contract_product_analyze_profiles_intake_artifacts() {
        let root = env::temp_dir().join(format!(
            "appfw-product-analyze-test-{}",
            Utc::now().timestamp_nanos_opt().unwrap_or_default()
        ));
        let product = root.join("claims-modernization");
        let framework = root.join("framework");
        let legacy_source = root.join("legacy-claims");
        fs::create_dir_all(&product).expect("create product");
        fs::create_dir_all(&framework).expect("create framework");
        seed_product_intake_test_framework(&framework);
        fs::create_dir_all(&legacy_source).expect("create legacy source");
        fs::write(legacy_source.join("Claims.sln"), "solution").expect("write sln");
        fs::write(
            legacy_source.join("ClaimsController.cs"),
            "class ClaimsController {}",
        )
        .expect("write cs");
        fs::write(
            legacy_source.join("procedures.sql"),
            "CREATE PROCEDURE claims_refresh AS SELECT 1;",
        )
        .expect("write sql");

        let intake = complete_product_intake(
            ProductIntakeArgs {
                source_kind: Some("legacy".to_string()),
                app_name: Some("claims-modernization".to_string()),
                display_name: Some("Claims Modernization".to_string()),
                schema: Some("claims".to_string()),
                provider: Some("postgres".to_string()),
                mcp_server: Some("false".to_string()),
                kafka_client: Some("false".to_string()),
                ui: Some("enterprise".to_string()),
                poc_source: Some("../legacy-claims".to_string()),
                data_artifact: Some("../legacy-claims/procedures.sql".to_string()),
                ui_artifact: None,
            },
            true,
        )
        .expect("complete legacy intake");
        write_product_intake_scaffold(&framework, &product, &intake).expect("write scaffold");

        let roots = CommandRoots::split_layout(&product, &framework);
        analyze_command(&roots, true, &[]).expect("analyze succeeds");

        let report_path = product.join("target/appfw/product-analysis.json");
        let report = serde_json::from_str::<Value>(
            &fs::read_to_string(&report_path).expect("read analysis report"),
        )
        .expect("parse analysis report");
        assert_eq!(report["command"], "analyze");
        assert_eq!(report["ok"], true);
        assert_eq!(report["source_kind"], "legacy");
        assert_eq!(
            report["artifacts"]["review_yaml"],
            ".appfw/legacy-analysis.yaml"
        );
        assert!(report["detected_signals"]
            .as_array()
            .expect("signals")
            .iter()
            .any(|value| value == "dotnet_codebase"));
        assert!(report["detected_signals"]
            .as_array()
            .expect("signals")
            .iter()
            .any(|value| value == "sql_or_routine_source"));
        assert!(report["model_clues"]["candidate_routines"]
            .as_array()
            .expect("candidate routines")
            .iter()
            .any(|value| value == "claims_refresh"));
        assert!(report["model_clues"]["candidate_entities"]
            .as_array()
            .expect("candidate entities")
            .iter()
            .any(|value| value == "claims"));

        let review =
            fs::read_to_string(product.join(".appfw/legacy-analysis.yaml")).expect("read yaml");
        assert!(review.contains("source_kind: legacy"));
        assert!(review.contains("target/appfw/product-analysis.json"));

        fs::remove_dir_all(root).expect("remove temp");
    }

    #[test]
    fn cli_contract_product_analyze_extracts_poc_model_clues() {
        let root = env::temp_dir().join(format!(
            "appfw-product-analyze-poc-test-{}",
            Utc::now().timestamp_nanos_opt().unwrap_or_default()
        ));
        let product = root.join("claims-insight");
        let framework = root.join("framework");
        let source = root.join("poc-source");
        fs::create_dir_all(&product).expect("create product");
        fs::create_dir_all(&framework).expect("create framework");
        seed_product_intake_test_framework(&framework);
        fs::create_dir_all(&source).expect("create source");
        fs::write(
            source.join("claims.csv"),
            "Claim ID,Member Name,Claim Status,Total Paid\n1,Ada,Open,42.50\n",
        )
        .expect("write csv");
        fs::write(
            source.join("prototype.html"),
            r#"<html><head><title>Claims Dashboard</title></head><body><h2>Claim Work Queue</h2><form><input name="claim_id"><select name="claim_status"></select></form><table><tr><th>Total Paid</th></tr></table></body></html>"#,
        )
        .expect("write html");
        fs::write(
            source.join("reference.md"),
            r#"# Claims Insight Reference

### `Claims` — Claim Work Queue
"#,
        )
        .expect("write markdown reference");
        fs::write(
            source.join("reference.md"),
            r#"# Claims Insight Reference

### `Claim` — Work Queue Item

| Field | Description |
| --- | --- |
| Claim ID | Durable claim identifier |
| Claim Status | Lookup-style workflow status |
| Total Paid | Paid amount |
"#,
        )
        .expect("write markdown reference");
        fs::write(source.join("~$source.xlsx"), "office lock file").expect("write temp file");

        let intake = complete_product_intake(
            ProductIntakeArgs {
                source_kind: Some("poc".to_string()),
                app_name: Some("claims-insight".to_string()),
                display_name: Some("Claims Insight".to_string()),
                schema: Some("claims".to_string()),
                provider: Some("postgres".to_string()),
                mcp_server: Some("false".to_string()),
                kafka_client: Some("false".to_string()),
                ui: Some("enterprise".to_string()),
                poc_source: Some("../poc-source".to_string()),
                data_artifact: Some("claims.csv".to_string()),
                ui_artifact: Some("prototype.html".to_string()),
            },
            true,
        )
        .expect("complete poc intake");
        write_product_intake_scaffold(&framework, &product, &intake).expect("write scaffold");

        let roots = CommandRoots::split_layout(&product, &framework);
        analyze_command(&roots, true, &[]).expect("analyze succeeds");

        let report = serde_json::from_str::<Value>(
            &fs::read_to_string(product.join("target/appfw/product-analysis.json"))
                .expect("read analysis report"),
        )
        .expect("parse analysis report");
        assert_eq!(report["source_kind"], "poc");
        assert_eq!(report["evidence"][1]["status"], "present");
        assert_eq!(report["evidence"][2]["status"], "present");
        let summary = summarize_analysis_report(&report);
        assert_eq!(summary["summary"], true);
        assert_eq!(summary["full_report"], "target/appfw/product-analysis.json");
        assert!(
            summary["evidence_summary"][0]["content_profile_count"]
                .as_u64()
                .expect("content profile count")
                > 0
        );
        assert!(
            summary["model_clues"]["candidate_property_count"]
                .as_u64()
                .expect("candidate property count")
                > 0
        );
        assert!(report["detected_signals"]
            .as_array()
            .expect("signals")
            .iter()
            .any(|value| value == "source_reference"));
        assert!(!report["evidence"][0]["sample_files"]
            .as_array()
            .expect("sample files")
            .iter()
            .any(|value| value == "~$source.xlsx"));
        assert!(report["model_clues"]["candidate_entities"]
            .as_array()
            .expect("candidate entities")
            .iter()
            .any(|value| value == "claims"));
        assert!(report["model_clues"]["candidate_entities"]
            .as_array()
            .expect("candidate entities")
            .iter()
            .any(|value| value == "claim"));
        assert!(report["evidence"][0]["content_profiles"]
            .as_array()
            .expect("content profiles")
            .iter()
            .filter_map(|value| value.get("profile"))
            .filter_map(|value| value.get("candidate_entity_labels"))
            .filter_map(Value::as_array)
            .flatten()
            .any(|value| {
                value.get("entity").and_then(Value::as_str) == Some("claim")
                    && value.get("label").and_then(Value::as_str) == Some("Work Queue Item")
            }));
        assert!(report["model_clues"]["candidate_properties"]
            .as_array()
            .expect("candidate properties")
            .iter()
            .any(|value| value == "claim_status"));
        assert!(report["model_clues"]["frontend_views"]
            .as_array()
            .expect("frontend views")
            .iter()
            .any(|value| value == "Claims Dashboard"));

        let review =
            fs::read_to_string(product.join(".appfw/poc-analysis.yaml")).expect("read yaml");
        assert!(review.contains("candidate_entities:"));
        assert!(review.contains("claim_status"));

        fs::remove_dir_all(root).expect("remove temp");
    }

    #[test]
    fn cli_contract_ooxml_metadata_helpers_extract_table_shape() {
        let table_xml = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<table xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main"
    name="SOURCE_FACTS" displayName="SOURCE_FACTS" ref="D5:F15">
    <tableColumns count="3">
        <tableColumn id="1" name="Region"/>
        <tableColumn id="2" name="Average Duration"/>
        <tableColumn id="3" name="Open Amount"/>
    </tableColumns>
</table>"#;

        let display_name = extract_attr_values(table_xml, "table", "displayName", 1)
            .into_iter()
            .next()
            .expect("display name");
        let reference = extract_attr_values(table_xml, "table", "ref", 1)
            .into_iter()
            .next()
            .expect("table range");
        let columns = extract_attr_values(table_xml, "tablecolumn", "name", 20)
            .into_iter()
            .map(|value| decode_xml_entities(&value))
            .collect::<Vec<_>>();

        assert_eq!(display_name, "SOURCE_FACTS");
        assert_eq!(ooxml_table_dimensions(&reference), Some((11, 10, 3)));
        assert_eq!(columns, vec!["Region", "Average Duration", "Open Amount"]);
        assert_eq!(
            normalized_identifier(&display_name).as_deref(),
            Some("source_facts")
        );
        assert_eq!(
            columns
                .iter()
                .filter_map(|value| normalized_identifier(value))
                .collect::<Vec<_>>(),
            vec!["region", "average_duration", "open_amount"]
        );

        let workbook_xml = r#"<workbook><sheets>
  <sheet name="Visible"/>
  <sheet name="Hidden Calc" state="hidden"/>
  <sheet name="Very Hidden Calc" state="veryHidden"/>
</sheets></workbook>"#;
        assert_eq!(
            extract_attr_values(workbook_xml, "sheet", "state", 10),
            vec!["hidden", "veryHidden"]
        );

        let worksheet_xml = r#"<worksheet>
  <sheetData>
    <c><f>SUM(A1:A3)</f></c>
    <c><f t="shared" ref="B1:B4"/></c>
    <c><font>Not a formula tag</font></c>
  </sheetData>
  <dataValidations count="1"><dataValidation type="list"/></dataValidations>
</worksheet>"#;
        assert_eq!(count_xml_start_tags(worksheet_xml, "f"), 2);
        assert_eq!(count_xml_start_tags(worksheet_xml, "dataValidation"), 1);
        assert_eq!(count_xml_start_tags(worksheet_xml, "table"), 0);
    }

    #[test]
    fn cli_contract_product_propose_model_creates_review_artifacts() {
        let root = env::temp_dir().join(format!(
            "appfw-product-propose-model-test-{}",
            Utc::now().timestamp_nanos_opt().unwrap_or_default()
        ));
        let product = root.join("claims-insight");
        let framework = root.join("framework");
        let source = root.join("poc-source");
        fs::create_dir_all(&product).expect("create product");
        fs::create_dir_all(&framework).expect("create framework");
        seed_product_intake_test_framework(&framework);
        fs::create_dir_all(&source).expect("create source");
        fs::write(
            source.join("claims.csv"),
            "Claim ID,Member Name,Claim Status,Total Paid\n1,Ada,Open,42.50\n",
        )
        .expect("write csv");
        fs::write(
            source.join("prototype.html"),
            r#"<html><head><title>Claims Dashboard</title></head><body><h2>Claim Work Queue</h2><form><input name="claim_id"><select name="claim_status"></select></form><table><tr><th>Total Paid</th></tr></table></body></html>"#,
        )
        .expect("write html");
        fs::write(
            source.join("reference.md"),
            r#"# Claims Insight Reference

### `Claims` — Claim Work Queue
"#,
        )
        .expect("write markdown reference");

        let intake = complete_product_intake(
            ProductIntakeArgs {
                source_kind: Some("poc".to_string()),
                app_name: Some("claims-insight".to_string()),
                display_name: Some("Claims Insight".to_string()),
                schema: Some("claims".to_string()),
                provider: Some("postgres".to_string()),
                mcp_server: Some("false".to_string()),
                kafka_client: Some("false".to_string()),
                ui: Some("enterprise".to_string()),
                poc_source: Some("../poc-source".to_string()),
                data_artifact: Some("../poc-source/claims.csv".to_string()),
                ui_artifact: Some("../poc-source/prototype.html".to_string()),
            },
            true,
        )
        .expect("complete poc intake");
        write_product_intake_scaffold(&framework, &product, &intake).expect("write scaffold");

        let roots = CommandRoots::split_layout(&product, &framework);
        analyze_command(&roots, true, &[]).expect("analyze succeeds");
        propose_model_command(&roots, true, &[]).expect("propose succeeds");

        let report = serde_json::from_str::<Value>(
            &fs::read_to_string(product.join("target/appfw/model-proposal.json"))
                .expect("read proposal report"),
        )
        .expect("parse proposal report");
        assert_eq!(report["command"], "propose-model");
        assert_eq!(report["ok"], true);
        assert_eq!(report["proposal"]["writes_final_model"], false);
        let summary = summarize_model_proposal(&report);
        assert_eq!(summary["summary"], true);
        assert_eq!(summary["full_report"], "target/appfw/model-proposal.json");
        assert_eq!(
            summary["implementation_plan"]["schema_source_root"],
            ".appfw/model/schemas/claims"
        );
        assert!(
            summary["implementation_plan"]["required_source_file_count"]
                .as_u64()
                .expect("required source file count")
                >= 5
        );
        assert!(
            summary["implementation_plan"]["entity_task_count"]
                .as_u64()
                .expect("entity task count")
                > 0
        );
        assert!(
            summary["review_decisions"]["entity_decision_count"]
                .as_u64()
                .expect("entity decision count")
                > 0
        );
        assert_eq!(summary["review_decisions"]["accepted_for_config"], false);
        assert!(summary["proposal"]["candidate_entities"]
            .as_array()
            .expect("summary candidate entities")
            .iter()
            .any(|value| {
                value.get("name").and_then(Value::as_str) == Some("claims")
                    && value
                        .get("property_count")
                        .and_then(Value::as_u64)
                        .unwrap_or_default()
                        > 0
            }));
        assert!(report["proposal"]["requires_human_review"]
            .as_bool()
            .expect("requires review"));
        assert_eq!(
            report["implementation_plan"]["schema_source_root"],
            ".appfw/model/schemas/claims"
        );
        assert!(report["implementation_plan"]["required_source_files"]
            .as_array()
            .expect("required source files")
            .iter()
            .any(|value| {
                value.get("path").and_then(Value::as_str)
                    == Some(".appfw/model/schemas/claims/entity_types/*.yaml")
            }));
        assert!(report["implementation_plan"]["entity_tasks"]
            .as_array()
            .expect("entity tasks")
            .iter()
            .any(|value| {
                value.get("entity").and_then(Value::as_str) == Some("claims")
                    && value.get("suggested_source_path").and_then(Value::as_str)
                        == Some(".appfw/model/schemas/claims/entity_types/claims.yaml")
            }));
        assert!(report["review_decisions"]["allowed_decisions"]
            .as_array()
            .expect("allowed decisions")
            .iter()
            .any(|value| value == "needs_review"));
        assert!(report["review_decisions"]["allowed_decisions"]
            .as_array()
            .expect("allowed decisions")
            .iter()
            .any(|value| value == "accept"));
        assert!(report["review_decisions"]["decision_states"]["terminal"]
            .as_array()
            .expect("terminal decisions")
            .iter()
            .any(|value| value == "defer"));
        assert!(report["review_decisions"]["decision_states"]["unresolved"]
            .as_array()
            .expect("unresolved decisions")
            .iter()
            .any(|value| value == "needs_discovery"));
        assert!(report["review_decisions"]["entity_decisions"]
            .as_array()
            .expect("entity decisions")
            .iter()
            .any(|value| {
                value.get("candidate").and_then(Value::as_str) == Some("claims")
                    && value.get("decision").and_then(Value::as_str) == Some("needs_review")
            }));
        assert!(report["proposal"]["candidate_entities"]
            .as_array()
            .expect("candidate entities")
            .iter()
            .any(|value| value.get("name").and_then(Value::as_str) == Some("claims")));
        let claims_entity = report["proposal"]["candidate_entities"]
            .as_array()
            .expect("candidate entities")
            .iter()
            .find(|value| value.get("name").and_then(Value::as_str) == Some("claims"))
            .expect("claims entity");
        assert!(claims_entity["candidate_properties"]
            .as_array()
            .expect("candidate entity properties")
            .iter()
            .any(|value| value == "claim_status"));
        assert!(claims_entity["semantic_labels"]
            .as_array()
            .expect("semantic labels")
            .iter()
            .any(|value| value == "Claim Work Queue"));
        assert!(claims_entity["suggested_domain_names"]
            .as_array()
            .expect("suggested domain names")
            .iter()
            .any(|value| value == "claim_work_queue"));
        assert!(report["proposal"]["frontend_views"]
            .as_array()
            .expect("frontend views")
            .iter()
            .any(|value| value == "Claims Dashboard"));

        let review =
            fs::read_to_string(product.join(".appfw/model-proposal.yaml")).expect("read yaml");
        assert!(review.contains("writes_final_model: false"));
        assert!(review.contains("requires_human_review: true"));
        assert!(review.contains("name: claims"));
        assert!(review.contains("semantic_labels:"));
        assert!(review.contains("claim_work_queue"));
        assert!(review.contains("candidate_properties:"));
        assert!(review.contains("claim_status"));
        assert!(review.contains("implementation_plan:"));
        assert!(review.contains("review_decisions:"));
        assert!(review.contains("allowed_decisions:"));
        assert!(review.contains("accepted_for_config: false"));
        assert!(review.contains("config_kind: entity|lookup|dto|reject|defer"));
        assert!(review.contains("schema_source_root: .appfw/model/schemas/claims"));
        assert!(review.contains(
            "suggested_source_path: .appfw/model/schemas/claims/entity_types/claims.yaml"
        ));
        assert!(review.contains("validation_sequence:"));

        fs::remove_dir_all(root).expect("remove temp");
    }

    #[test]
    fn cli_contract_product_model_status_tracks_intake_journey() {
        let root = env::temp_dir().join(format!(
            "appfw-product-model-status-test-{}",
            Utc::now().timestamp_nanos_opt().unwrap_or_default()
        ));
        let product = root.join("claims-insight");
        let framework = root.join("framework");
        let source = root.join("poc-source");
        fs::create_dir_all(&product).expect("create product");
        fs::create_dir_all(&framework).expect("create framework");
        seed_product_intake_test_framework(&framework);
        fs::create_dir_all(&source).expect("create source");
        fs::write(
            source.join("claims.csv"),
            "Claim ID,Member Name,Claim Status\n1,Ada,Open\n",
        )
        .expect("write csv");

        let intake = complete_product_intake(
            ProductIntakeArgs {
                source_kind: Some("poc".to_string()),
                app_name: Some("claims-insight".to_string()),
                display_name: Some("Claims Insight".to_string()),
                schema: Some("claims".to_string()),
                provider: Some("postgres".to_string()),
                mcp_server: Some("false".to_string()),
                kafka_client: Some("false".to_string()),
                ui: Some("enterprise".to_string()),
                poc_source: Some("../poc-source".to_string()),
                data_artifact: Some("claims.csv".to_string()),
                ui_artifact: None,
            },
            true,
        )
        .expect("complete poc intake");
        write_product_intake_scaffold(&framework, &product, &intake).expect("write scaffold");

        let roots = CommandRoots::split_layout(&product, &framework);
        analyze_command(&roots, true, &[]).expect("analyze succeeds");
        propose_model_command(&roots, true, &[]).expect("propose succeeds");
        model_status_command(&roots, true, &[]).expect("model status succeeds");

        let status_path = product.join("target/appfw/model-status.json");
        let status = serde_json::from_str::<Value>(
            &fs::read_to_string(&status_path).expect("read status report"),
        )
        .expect("parse status report");
        assert_eq!(status["command"], "model-status");
        assert_eq!(status["phase"], "proposal_ready_for_modeling");
        assert_eq!(status["app"]["schema"], "claims");
        assert_eq!(
            status["schema_model"]["schema_source_root"],
            ".appfw/model/schemas/claims"
        );
        assert_eq!(status["schema_model"]["graphql_http_route"], "claims");
        assert_eq!(status["proposal"]["implementation_plan_present"], true);
        assert_eq!(status["proposal_review"]["accepted_for_config"], false);
        assert_eq!(status["proposal_review"]["signed_for_config"], false);
        assert_eq!(status["proposal_review"]["ready_for_config"], false);
        assert_eq!(
            status["source_authoring_plan"]["ready_to_write_source"],
            false
        );
        assert!(status["proposal_review"]["missing_sign_off_fields"]
            .as_array()
            .expect("missing sign-off fields")
            .iter()
            .any(|value| value == "model_owner"));
        assert!(
            status["proposal_review"]["unresolved_decision_count"]
                .as_u64()
                .expect("unresolved decision count")
                > 0
        );
        assert!(status["blockers"]
            .as_array()
            .expect("blockers")
            .iter()
            .any(|value| value == "Model proposal review decisions are not ready for config yet."));
        assert!(status["blockers"]
            .as_array()
            .expect("blockers")
            .iter()
            .any(|value| value == "No reviewed product entity source files exist yet."));
        assert!(status["next_commands"]
            .as_array()
            .expect("next commands")
            .iter()
            .any(|value| {
                value == "complete review_decisions sign-off and set accepted_for_config: true"
            }));
        let scaffold_before_review = scaffold_model_command(&roots, true, &[])
            .expect_err("scaffold should require signed model review");
        assert!(scaffold_before_review.contains("requires a signed proposal review"));

        let review_path = product.join(".appfw/model-proposal.yaml");
        let unsigned_reviewed = fs::read_to_string(&review_path)
            .expect("read review yaml")
            .replace("decision: needs_review", "decision: accept")
            .replace("decision: needs_discovery", "decision: accept")
            .replace("accepted_for_config: false", "accepted_for_config: true");
        fs::write(&review_path, &unsigned_reviewed).expect("write unsigned reviewed yaml");
        model_status_command(&roots, true, &[])
            .expect("model status after unsigned review succeeds");
        let status = serde_json::from_str::<Value>(
            &fs::read_to_string(&status_path).expect("read status report"),
        )
        .expect("parse status report");
        assert_eq!(status["phase"], "proposal_ready_for_modeling");
        assert_eq!(status["proposal_review"]["accepted_for_config"], true);
        assert_eq!(status["proposal_review"]["signed_for_config"], false);
        assert_eq!(status["proposal_review"]["ready_for_config"], false);
        assert_eq!(
            status["source_authoring_plan"]["ready_to_write_source"],
            false
        );
        assert_eq!(
            status["proposal_review"]["unresolved_decision_count"]
                .as_u64()
                .expect("unresolved decision count"),
            0
        );

        let reviewed = unsigned_reviewed
            .replace("model_owner: null", "model_owner: product_architect")
            .replace(
                "reviewed_at_utc: null",
                "reviewed_at_utc: '2026-06-09T00:00:00Z'",
            );
        fs::write(&review_path, reviewed).expect("write reviewed yaml");
        model_status_command(&roots, true, &[]).expect("model status after review succeeds");
        let status = serde_json::from_str::<Value>(
            &fs::read_to_string(&status_path).expect("read status report"),
        )
        .expect("parse status report");
        assert_eq!(status["phase"], "proposal_reviewed");
        assert_eq!(status["proposal_review"]["accepted_for_config"], true);
        assert_eq!(status["proposal_review"]["signed_for_config"], true);
        assert_eq!(status["proposal_review"]["ready_for_config"], true);
        assert_eq!(
            status["source_authoring_plan"]["ready_to_write_source"],
            true
        );
        assert_eq!(
            status["source_authoring_plan"]["schema_source_root"],
            ".appfw/model/schemas/claims"
        );
        assert!(status["source_authoring_plan"]["entity_source_targets"]
            .as_array()
            .expect("entity source targets")
            .iter()
            .any(|value| {
                value.get("candidate").and_then(Value::as_str) == Some("claims")
                    && value.get("suggested_source_path").and_then(Value::as_str)
                        == Some(".appfw/model/schemas/claims/entity_types/claims.yaml")
            }));
        assert_eq!(
            status["proposal_review"]["unresolved_decision_count"]
                .as_u64()
                .expect("unresolved decision count"),
            0
        );
        assert!(status["next_commands"]
            .as_array()
            .expect("next commands")
            .iter()
            .any(|value| value == "write .appfw/model from reviewed PoC evidence"));

        let unresolved_scaffold = scaffold_model_command(&roots, true, &[])
            .expect_err("scaffold should reject unresolved semantic decisions");
        assert!(unresolved_scaffold.contains("unresolved config_kind"));

        let scaffold_ready_review = fs::read_to_string(&review_path)
            .expect("read reviewed yaml")
            .replace(
                "config_kind: entity|lookup|dto|reject|defer",
                "config_kind: entity",
            )
            .replace(
                "classification: needs_review",
                "classification: confidential",
            );
        fs::write(&review_path, scaffold_ready_review).expect("write scaffold-ready review");
        scaffold_model_command(&roots, true, &["--dry-run".to_string()])
            .expect("scaffold dry run succeeds");
        assert!(
            !product.join("target/appfw/model-scaffold.json").exists(),
            "dry run should not write the scaffold report"
        );
        scaffold_model_command(&roots, true, &[]).expect("scaffold model succeeds");
        let scaffold_report_path = product.join("target/appfw/model-scaffold.json");
        let scaffold = serde_json::from_str::<Value>(
            &fs::read_to_string(&scaffold_report_path).expect("read scaffold report"),
        )
        .expect("parse scaffold report");
        assert_eq!(scaffold["command"], "scaffold-model");
        assert_eq!(scaffold["writes_final_model"], false);
        assert!(scaffold["written_files"]
            .as_array()
            .expect("written files")
            .iter()
            .any(|value| value == ".appfw/model/schemas/claims/entity_types/claims.yaml"));
        assert!(scaffold["written_files"]
            .as_array()
            .expect("written files")
            .iter()
            .any(|value| value == ".appfw/model/schemas/claims/rbac/claims.rego"));
        assert!(fs::read_to_string(
            product.join(".appfw/model/schemas/claims/entity_types/claims.yaml")
        )
        .expect("read scaffold entity")
        .contains("classification: confidential"));

        model_status_command(&roots, true, &[]).expect("model status after source succeeds");
        let status = serde_json::from_str::<Value>(
            &fs::read_to_string(&status_path).expect("read status report"),
        )
        .expect("parse status report");
        assert_eq!(status["phase"], "model_source_started");
        assert_eq!(
            status["schema_model"]["entity_source_count"]
                .as_u64()
                .expect("entity source count"),
            1
        );
        assert!(status["next_commands"]
            .as_array()
            .expect("next commands")
            .iter()
            .any(|value| value == "scripts/appfw product validate --json"));

        fs::remove_dir_all(root).expect("remove temp");
    }

    #[test]
    fn harness_check_accepts_least_privilege_product_profile() {
        let root = env::temp_dir().join(format!(
            "appfw-harness-check-pass-test-{}",
            Utc::now().timestamp_nanos_opt().unwrap_or_default()
        ));
        let product = root.join("product");
        let framework = root.join("framework");
        write_test_file(
            &framework,
            "docs/architecture/concerns/agentic-threat-model.md",
            "ASI01\nASI02\nASI03\n",
        );
        write_test_file(
            &product,
            ".appfw/agent-profile.yaml",
            r#"
schema_version: 1
name: test-product-agent
allowed_commands:
  - command: scripts/appfw product validate --json
    namespace: product
writable_paths:
  - path: .appfw/model
    ownership: application_source
  - path: .appfw/target/appfw
    ownership: product_evidence
  - path: target/appfw
    ownership: lifecycle_evidence
network_policy: disabled
live_service_policy: disabled
sensitive_capabilities:
  mcp: false
  kafka: false
  release: false
  saas_governed_write: false
threat_controls:
  - ASI01
  - ASI02
  - ASI03
review_checkpoints:
  - handoff
handoff:
  required: true
  artifact: target/appfw/agent-handoff.json
"#,
        );

        let roots = CommandRoots::split_layout(&product, &framework);
        let report = build_harness_check(&roots).expect("build harness check");

        assert_eq!(report["command"], "harness-check");
        assert_eq!(report["lane"], "U2");
        assert_eq!(report["ok"], true);
        assert_eq!(report["artifact"], ".appfw/target/appfw/harness-check.json");
        assert!(product
            .join(".appfw/target/appfw/harness-check.json")
            .is_file());
        assert!(framework
            .join("target/appfw/wave2/u2-harness-check.json")
            .is_file());
        assert_eq!(
            report["release_artifacts"][0]["path"],
            "target/appfw/wave2/u2-harness-check.json"
        );
        assert_eq!(report["release_artifacts"][0]["release_ready"], false);

        fs::remove_dir_all(root).expect("remove temp");
    }

    #[test]
    fn harness_check_rejects_framework_commands() {
        let root = env::temp_dir().join(format!(
            "appfw-harness-check-fail-test-{}",
            Utc::now().timestamp_nanos_opt().unwrap_or_default()
        ));
        let product = root.join("product");
        let framework = root.join("framework");
        write_test_file(
            &framework,
            "docs/architecture/concerns/agentic-threat-model.md",
            "ASI01\nASI02\nASI03\n",
        );
        write_test_file(
            &product,
            ".appfw/agent-profile.yaml",
            r#"
schema_version: 1
name: unsafe-product-agent
allowed_commands:
  - command: scripts/appfw framework validate --json
    namespace: framework
writable_paths:
  - path: .appfw/model
    ownership: application_source
network_policy: disabled
live_service_policy: disabled
sensitive_capabilities:
  mcp: false
  kafka: false
  release: false
  saas_governed_write: false
threat_controls:
  - ASI01
  - ASI02
  - ASI03
review_checkpoints:
  - handoff
handoff:
  required: true
  artifact: target/appfw/agent-handoff.json
"#,
        );

        let roots = CommandRoots::split_layout(&product, &framework);
        let report = build_harness_check(&roots).expect("build harness check");

        assert_eq!(report["ok"], false);
        assert!(report["violations"]
            .as_array()
            .expect("violations")
            .iter()
            .any(|violation| violation["check"] == "commands-product-scoped"));

        fs::remove_dir_all(root).expect("remove temp");
    }

    #[test]
    fn harness_check_rejects_governed_write_without_valid_g1_artifacts() {
        let root = env::temp_dir().join(format!(
            "appfw-harness-check-governed-write-test-{}",
            Utc::now().timestamp_nanos_opt().unwrap_or_default()
        ));
        let product = root.join("product");
        let framework = root.join("framework");
        write_test_file(
            &framework,
            "docs/architecture/concerns/agentic-threat-model.md",
            "ASI01\nASI02\nASI03\n",
        );
        write_test_file(
            &product,
            ".appfw/agent-profile.yaml",
            r#"
schema_version: 1
name: unsafe-governed-write-agent
allowed_commands:
  - command: scripts/appfw product validate --json
    namespace: product
writable_paths:
  - path: .appfw/model
    ownership: application_source
network_policy: disabled
live_service_policy: disabled
sensitive_capabilities:
  mcp: false
  kafka: false
  release: false
  saas_governed_write: true
threat_controls:
  - ASI01
  - ASI02
  - ASI03
review_checkpoints:
  - handoff
handoff:
  required: true
  artifact: target/appfw/agent-handoff.json
"#,
        );
        write_test_file(
            &product,
            "target/appfw/governed-write-evidence.json",
            r#"{"command":"provider-test","lane":"G1","ok":true}"#,
        );

        let roots = CommandRoots::split_layout(&product, &framework);
        let report = build_harness_check(&roots).expect("build harness check");

        assert_eq!(report["ok"], false);
        assert!(report["violations"]
            .as_array()
            .expect("violations")
            .iter()
            .any(|violation| violation["check"] == "governed-writes-disabled-unless-g1-evidence"));

        fs::remove_dir_all(root).expect("remove temp");
    }

    #[test]
    fn harness_check_accepts_governed_write_with_valid_g1_artifacts() {
        let root = env::temp_dir().join(format!(
            "appfw-harness-check-governed-write-valid-test-{}",
            Utc::now().timestamp_nanos_opt().unwrap_or_default()
        ));
        let product = root.join("product");
        let framework = root.join("framework");
        write_test_file(
            &framework,
            "docs/architecture/concerns/agentic-threat-model.md",
            "ASI01\nASI02\nASI03\n",
        );
        write_test_file(
            &product,
            ".appfw/agent-profile.yaml",
            r#"
schema_version: 1
name: governed-write-agent
allowed_commands:
  - command: scripts/appfw product validate --json
    namespace: product
writable_paths:
  - path: .appfw/model
    ownership: application_source
  - path: .appfw/target/appfw
    ownership: product_evidence
  - path: target/appfw
    ownership: lifecycle_evidence
network_policy: disabled
live_service_policy: disabled
sensitive_capabilities:
  mcp: false
  kafka: false
  release: false
  saas_governed_write: true
threat_controls:
  - ASI01
  - ASI02
  - ASI03
review_checkpoints:
  - handoff
handoff:
  required: true
  artifact: target/appfw/agent-handoff.json
"#,
        );
        write_test_file(
            &product,
            "target/appfw/governed-write-evidence.json",
            r#"{
  "command": "provider-test",
  "lane": "G1",
  "ok": true,
  "operation": "named_mutation",
  "delegated_actor_context": {"subject": "service-account"},
  "token_store_isolation": {"scope": "tenant-user"},
  "mutation_registry": {"name": "refresh_account_health", "mcp_enabled": false},
  "idempotency": {"key": "event_id"},
  "audit": {"sink": "required"}
}"#,
        );
        write_test_file(
            &product,
            "target/appfw/governed-write-posture.json",
            r#"{
  "command": "governed-write-check",
  "lane": "G1",
  "ok": true,
  "gate": {"enforced": true, "ready_to_enforce": true},
  "blocking_violations": [],
  "providers": [
    {
      "name": "servicenow",
      "family": "external_api",
      "write_enabled": false,
      "mcp_enabled": false,
      "governed_write_certified": true
    }
  ]
}"#,
        );

        let roots = CommandRoots::split_layout(&product, &framework);
        let report = build_harness_check(&roots).expect("build harness check");

        assert_eq!(report["ok"], true);
        assert!(report["not_applicable"]
            .as_array()
            .expect("not_applicable")
            .iter()
            .any(|item| item["name"] == "governed-write-default-disabled"));

        fs::remove_dir_all(root).expect("remove temp");
    }

    #[test]
    fn frontend_residue_ignores_bitbucket_checkout_path_but_flags_pipeline_copy() {
        let allowed_identity = vec!["claims".to_string()];
        let ci_alias = r#""@appfw/pds-health-components": ["../../../../opt/atlassian/pipelines/agent/build/appfw_ui/pds_health/components/src/index.ts"],"#;
        assert!(product_frontend_residue_matches(ci_alias, &allowed_identity).is_empty());

        let product_copy = "Review CRM pipeline health before release.";
        let matches = product_frontend_residue_matches(product_copy, &allowed_identity);
        assert!(matches.contains(&"crm"));
        assert!(matches.contains(&"pipeline"));
    }

    #[test]
    fn cli_contract_intake_proof_runs_disposable_poc_path() {
        let root = env::temp_dir().join(format!(
            "appfw-intake-proof-test-{}",
            Utc::now().timestamp_nanos_opt().unwrap_or_default()
        ));
        fs::create_dir_all(&root).expect("create root");
        seed_product_intake_test_framework(&root);
        let roots = CommandRoots {
            app_root: root.clone(),
            framework_root: root.clone(),
            generator_root: root.join("app_gen"),
            config_root: root.join(".appfw/model"),
            templates_root: root.join("app_gen/_templates"),
            report_root: root.join(".appfw/target/appfw"),
        };

        intake_proof_command(&roots, true, &[]).expect("intake proof succeeds");

        let artifact_path = root.join("target/appfw/product-intake-proof.json");
        let report = serde_json::from_str::<Value>(
            &fs::read_to_string(&artifact_path).expect("read proof artifact"),
        )
        .expect("parse proof artifact");
        assert_eq!(report["command"], "intake-proof");
        assert_eq!(report["ok"], true);
        assert_eq!(report["source_kind"], "poc");
        assert_eq!(report["temp_workspace_status"], "removed");
        assert_eq!(report["artifact"], "target/appfw/product-intake-proof.json");
        assert!(root
            .join("target/appfw/product-intake-proof/product-analysis.json")
            .is_file());
        assert!(root
            .join("target/appfw/product-intake-proof/model-proposal.json")
            .is_file());
        assert!(root
            .join("target/appfw/product-intake-proof/model-status.json")
            .is_file());
        assert!(root
            .join("target/appfw/product-intake-proof/model-proposal.yaml")
            .is_file());
        assert!(root
            .join("target/appfw/product-intake-proof/frontend-residue-check.json")
            .is_file());
        assert!(root
            .join("target/appfw/product-intake-proof/frontend-scaffold-check.json")
            .is_file());
        assert_eq!(report["frontend_residue_check"]["ok"], true);
        assert_eq!(report["frontend_residue_check"]["matches"], 0);
        assert_eq!(report["frontend_scaffold_check"]["ok"], true);
        assert_eq!(report["frontend_scaffold_check"]["design_system_ok"], true);
        assert!(report["steps"]
            .as_array()
            .expect("proof steps")
            .iter()
            .any(|step| {
                step.get("name").and_then(Value::as_str) == Some("model-status")
                    && step.get("phase").and_then(Value::as_str)
                        == Some("proposal_ready_for_modeling")
            }));
        assert!(report["steps"]
            .as_array()
            .expect("proof steps")
            .iter()
            .any(|step| {
                step.get("name").and_then(Value::as_str) == Some("frontend-residue-check")
                    && step.get("ok").and_then(Value::as_bool) == Some(true)
            }));
        assert!(report["steps"]
            .as_array()
            .expect("proof steps")
            .iter()
            .any(|step| {
                step.get("name").and_then(Value::as_str) == Some("frontend-scaffold-check")
                    && step.get("design_system_ok").and_then(Value::as_bool) == Some(true)
            }));
        assert!(report["proposal_summary"]["candidate_entities"]
            .as_array()
            .expect("candidate entities")
            .iter()
            .any(|value| value.get("name").and_then(Value::as_str) == Some("claims")));
        assert_eq!(report["proposal_summary"]["writes_final_model"], false);
        assert!(report["steps"]
            .as_array()
            .expect("steps")
            .iter()
            .any(|value| value.get("name").and_then(Value::as_str) == Some("propose-model")));

        fs::remove_dir_all(root).expect("remove temp");
    }

    #[test]
    fn cli_contract_new_profile_manifest_exposes_verification_commands() {
        let root = env::temp_dir().join(format!(
            "appfw-new-profile-manifest-test-{}",
            Utc::now().timestamp_nanos_opt().unwrap_or_default()
        ));
        let profile_dir = root.join("app_gen/_golden/downstream_apps/crm-sample");
        fs::create_dir_all(&profile_dir).expect("create profile dir");
        let frontend_dir = root.join("examples/products/crm/frontend");
        fs::create_dir_all(frontend_dir.join(".appfw-ui")).expect("create frontend metadata dir");
        fs::create_dir_all(frontend_dir.join("src/generated")).expect("create generated dir");
        fs::create_dir_all(frontend_dir.join("scripts")).expect("create scripts dir");
        fs::write(
            frontend_dir.join("package.json"),
            r#"{"scripts":{"appfw:check":"node scripts/check-scaffold.mjs"}}"#,
        )
        .expect("write package json");
        fs::write(frontend_dir.join(".appfw-ui/scaffold-manifest.json"), "{}")
            .expect("write scaffold manifest");
        fs::write(frontend_dir.join(".appfw-ui/ownership.json"), "{}")
            .expect("write ownership manifest");
        fs::write(
            frontend_dir.join("src/generated/appfw-ui-contract.ts"),
            "export const appfwUiContracts = [];",
        )
        .expect("write generated contract");
        fs::write(frontend_dir.join("scripts/check-scaffold.mjs"), "")
            .expect("write scaffold check");
        fs::write(
            profile_dir.join("profile.json"),
            r#"{
  "name": "crm-sample",
  "description": "CRM sample",
  "template": "examples/products/crm",
  "verification": ["scripts/appfw product validate --json"],
  "frontend_scaffold": {
    "root": "frontend",
    "scaffold_manifest": ".appfw-ui/scaffold-manifest.json",
    "ownership_manifest": ".appfw-ui/ownership.json",
    "generated_contract": "src/generated/appfw-ui-contract.ts",
    "check_script": "scripts/check-scaffold.mjs",
    "package_check": "npm run appfw:check",
    "verification": ["npm run appfw:check"],
    "release_evidence": ["frontend/.appfw-ui/scaffold-manifest.json"]
  }
}"#,
        )
        .expect("write profile manifest");

        let profile = new_profile("crm-sample").expect("registered profile");
        let manifest = read_new_profile_manifest(&root, &profile).expect("read profile manifest");

        assert_eq!(manifest.name, "crm-sample");
        assert_eq!(manifest.overlay, "overlay");
        assert_eq!(
            manifest.verification,
            vec!["scripts/appfw product validate --json".to_string()]
        );
        let frontend_scaffold = manifest.frontend_scaffold.expect("frontend scaffold");
        assert_eq!(frontend_scaffold.root, "frontend");
        assert_eq!(frontend_scaffold.package_check, "npm run appfw:check");
        assert_eq!(
            frontend_scaffold.verification,
            vec!["npm run appfw:check".to_string()]
        );
        assert_eq!(
            frontend_package_script_name(&frontend_scaffold.package_check),
            Some("appfw:check")
        );

        fs::write(
            profile_dir.join("profile.json"),
            r#"{
  "name": "wrong-profile",
  "description": "CRM sample",
  "template": "examples/products/crm"
}"#,
        )
        .expect("write mismatched profile manifest");
        let error =
            read_new_profile_manifest(&root, &profile).expect_err("mismatched profile fails");
        assert!(error.contains("expected `crm-sample`"));

        fs::remove_dir_all(root).expect("remove temp");
    }

    #[test]
    fn second_consumer_profile_is_registered_and_resolves_its_package_template() {
        let repo_root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .expect("workspace root");
        let profile = new_profile("second-consumer").expect("registered second consumer");
        let manifest = read_new_profile_manifest(repo_root, &profile).expect("resolve manifest");

        assert_eq!(
            profile.template_path,
            "app_gen/_golden/downstream_apps/second-consumer/starter"
        );
        assert_eq!(manifest.name, "second-consumer");
        assert!(manifest
            .verification
            .iter()
            .any(|command| command.contains("generate --check")));
    }

    #[test]
    fn framework_model_seed_copy_preserves_system_and_facet_inputs() {
        let root = env::temp_dir().join(format!(
            "appfw-seed-copy-test-{}",
            Utc::now().timestamp_nanos_opt().unwrap_or_default()
        ));
        let target = root.join("consumer");
        seed_product_intake_test_framework(&root);
        let mut copied = Vec::new();

        copy_framework_model_seeds(&root, &target, &mut copied).expect("copy model seeds");

        assert!(target
            .join(".appfw/model/_facets/audit_entity_type.yaml")
            .is_file());
        assert!(target
            .join(".appfw/model/schemas/system/entity_types/entity_type.yaml")
            .is_file());
        assert!(!target
            .join(".appfw/model/schemas/system/_res.yaml")
            .exists());
        fs::remove_dir_all(root).expect("remove temp");
    }

    #[test]
    fn cli_contract_new_list_profiles_exposes_product_intake_starter_mode() {
        let starter_modes = new_starter_modes();
        let product_intake = starter_modes
            .iter()
            .find(|mode| mode.name == "product-intake")
            .expect("product-intake starter mode");

        assert!(product_intake.source_kinds.iter().any(|kind| kind == "poc"));
        assert!(product_intake
            .source_kinds
            .iter()
            .any(|kind| kind == "legacy"));
        assert!(product_intake
            .next_commands
            .iter()
            .any(|command| command == "scripts/appfw product analyze --summary --json"));
        assert!(product_intake
            .verification
            .iter()
            .any(|command| command == "scripts/appfw framework intake-proof --json"));
    }

    #[test]
    fn cli_contract_golden_downstream_command_writes_profile_artifact() {
        let root = env::temp_dir().join(format!(
            "appfw-golden-downstream-test-{}",
            Utc::now().timestamp_nanos_opt().unwrap_or_default()
        ));
        let profile_dir = root.join("app_gen/_golden/downstream_apps/crm-sample");
        fs::create_dir_all(&profile_dir).expect("create profile dir");
        let frontend_dir = root.join("examples/products/crm/frontend");
        fs::create_dir_all(frontend_dir.join(".appfw-ui")).expect("create frontend metadata dir");
        fs::create_dir_all(frontend_dir.join("src/generated")).expect("create generated dir");
        fs::create_dir_all(frontend_dir.join("scripts")).expect("create scripts dir");
        fs::write(
            frontend_dir.join("package.json"),
            r#"{"scripts":{"appfw:check":"node scripts/check-scaffold.mjs","typecheck":"tsc --noEmit","build":"vite build"}}"#,
        )
        .expect("write package json");
        fs::write(frontend_dir.join(".appfw-ui/scaffold-manifest.json"), "{}")
            .expect("write scaffold manifest");
        fs::write(frontend_dir.join(".appfw-ui/ownership.json"), "{}")
            .expect("write ownership manifest");
        fs::write(
            frontend_dir.join("src/generated/appfw-ui-contract.ts"),
            "export const appfwUiContracts = [];",
        )
        .expect("write generated contract");
        fs::write(frontend_dir.join("scripts/check-scaffold.mjs"), "")
            .expect("write scaffold check");
        fs::write(
            profile_dir.join("profile.json"),
            r#"{
  "name": "crm-sample",
  "description": "CRM sample",
  "template": "examples/products/crm",
  "verification": [
    "scripts/appfw product validate --json",
    "scripts/appfw product harness-check --json",
    "scripts/appfw product generate",
    "scripts/appfw product generate --check --json",
    "scripts/appfw product test",
    "scripts/appfw product handoff --json"
  ],
  "frontend_scaffold": {
    "root": "frontend",
    "scaffold_manifest": ".appfw-ui/scaffold-manifest.json",
    "ownership_manifest": ".appfw-ui/ownership.json",
    "generated_contract": "src/generated/appfw-ui-contract.ts",
    "check_script": "scripts/check-scaffold.mjs",
    "package_check": "npm run appfw:check",
    "verification": ["npm run appfw:check", "npm run typecheck", "npm run build"],
    "release_evidence": [
      "frontend/.appfw-ui/scaffold-manifest.json",
      "frontend/.appfw-ui/ownership.json",
      "frontend/src/generated/appfw-ui-contract.ts"
    ]
  }
}"#,
        )
        .expect("write profile manifest");

        let roots = CommandRoots {
            app_root: root.clone(),
            framework_root: root.clone(),
            generator_root: root.join("app_gen"),
            config_root: root.join(".appfw/model"),
            templates_root: root.join("app_gen/_templates"),
            report_root: root.join(".appfw/target/appfw"),
        };

        golden_downstream_command(
            &roots,
            true,
            &["--profile".to_string(), "crm-sample".to_string()],
        )
        .expect("golden downstream command succeeds");

        let artifact_path = root.join("target/appfw/golden-downstream.json");
        let report = serde_json::from_str::<Value>(
            &fs::read_to_string(&artifact_path).expect("read golden downstream artifact"),
        )
        .expect("parse golden downstream artifact");
        assert_eq!(report["command"], "golden-downstream");
        assert_eq!(report["ok"], true);
        assert_eq!(report["artifact"], "target/appfw/golden-downstream.json");
        assert_eq!(
            report["profiles"][0]["missing_required_verification"]
                .as_array()
                .expect("missing profile verification is an array")
                .len(),
            0
        );
        assert!(report["profiles"][0]["disposable_ci_lane"]["commands"]
            .as_array()
            .expect("disposable ci commands")
            .iter()
            .any(|command| command.as_str() == Some("scripts/appfw product frontend-test --json")));

        fs::remove_dir_all(root).expect("remove temp");
    }

    #[test]
    fn cli_contract_golden_downstream_required_failure_is_nonzero() {
        let root = env::temp_dir().join(format!(
            "appfw-golden-downstream-negative-test-{}",
            Utc::now().timestamp_nanos_opt().unwrap_or_default()
        ));
        let profile_dir = root.join("app_gen/_golden/downstream_apps/crm-sample");
        fs::create_dir_all(&profile_dir).expect("create profile dir");
        fs::write(
            profile_dir.join("profile.json"),
            r#"{
  "name": "crm-sample",
  "description": "Missing required verification fixture",
  "template": "examples/products/crm",
  "verification": ["scripts/appfw product validate --json"]
}"#,
        )
        .expect("write profile manifest");

        let roots = CommandRoots {
            app_root: root.clone(),
            framework_root: root.clone(),
            generator_root: root.join("app_gen"),
            config_root: root.join(".appfw/model"),
            templates_root: root.join("app_gen/_templates"),
            report_root: root.join(".appfw/target/appfw"),
        };

        let result = golden_downstream_command(
            &roots,
            true,
            &["--profile".to_string(), "crm-sample".to_string()],
        );
        assert!(result.is_err(), "required false result must return failure");

        let artifact_path = root.join("target/appfw/golden-downstream.json");
        let report = serde_json::from_str::<Value>(
            &fs::read_to_string(&artifact_path).expect("read failed golden artifact"),
        )
        .expect("parse failed golden artifact");
        assert_eq!(report["ok"], false);
        assert!(!report["failed_checks"]
            .as_array()
            .expect("failed checks")
            .is_empty());

        fs::remove_dir_all(root).expect("remove temp");
    }

    #[test]
    fn git_status_parser_reports_machine_readable_change_state() {
        let worktree_modified =
            parse_git_status_line(" M docs/start/cli.md", None).expect("worktree modified");
        assert_eq!(worktree_modified.status, "M");
        assert_eq!(worktree_modified.change_kind, "modified");
        assert_eq!(worktree_modified.index_status, "none");
        assert_eq!(worktree_modified.worktree_status, "modified");
        assert_eq!(worktree_modified.path, "docs/start/cli.md");

        let staged_added =
            parse_git_status_line("A  .appfw/model/example.yaml", None).expect("staged added");
        assert_eq!(staged_added.change_kind, "added");
        assert_eq!(staged_added.index_status, "added");
        assert_eq!(staged_added.worktree_status, "none");

        let untracked = parse_git_status_line("?? docs/example.md", None).expect("untracked file");
        assert_eq!(untracked.change_kind, "untracked");
        assert_eq!(untracked.index_status, "untracked");
        assert_eq!(untracked.worktree_status, "untracked");

        let renamed =
            parse_git_status_line("R  docs/old-name.md -> docs/new-name.md", Some("docs"))
                .expect("renamed file under app prefix");
        assert_eq!(renamed.change_kind, "renamed");
        assert_eq!(renamed.index_status, "renamed");
        assert_eq!(renamed.worktree_status, "none");
        assert_eq!(renamed.path, "new-name.md");
    }

    #[test]
    fn git_diff_name_status_parser_reports_branch_diff_state() {
        let modified =
            parse_git_diff_name_status_line("M\tdocs/start/cli.md", None).expect("modified");
        assert_eq!(modified.status, "M");
        assert_eq!(modified.change_kind, "modified");
        assert_eq!(modified.index_status, "branch_diff");
        assert_eq!(modified.worktree_status, "none");
        assert_eq!(modified.path, "docs/start/cli.md");

        let renamed = parse_git_diff_name_status_line(
            "R100\tdocs/old-name.md\tdocs/new-name.md",
            Some("docs"),
        )
        .expect("renamed file under app prefix");
        assert_eq!(renamed.status, "R");
        assert_eq!(renamed.change_kind, "renamed");
        assert_eq!(renamed.path, "new-name.md");
    }

    #[test]
    fn cli_contract_handoff_uses_configured_report_root() {
        let root = env::temp_dir().join(format!(
            "appfw-handoff-report-root-test-{}",
            Utc::now().timestamp_nanos_opt().unwrap_or_default()
        ));
        let product = root.join("product");
        let framework = root.join("framework");
        let report_root = product.join("custom_reports");
        fs::create_dir_all(&report_root).expect("create report root");
        fs::create_dir_all(framework.join("app_gen/_templates")).expect("create framework dirs");
        fs::write(report_root.join("validation.json"), r#"{"valid":true}"#)
            .expect("write validation report");
        fs::write(
            report_root.join("app_topology.json"),
            r#"{"app":{"name":"split-product","display_name":"Split Product"},"topology":{"schemas":[],"data_sources":[]}}"#,
        )
        .expect("write topology report");

        let git_init = Command::new("git")
            .arg("init")
            .arg(&product)
            .output()
            .expect("run git init");
        assert!(git_init.status.success());

        let mut roots = CommandRoots::split_layout(&product, &framework);
        roots.report_root = report_root.clone();
        let report = build_handoff(&roots).expect("build handoff");

        assert_eq!(report.product.name, "split-product");
        assert_eq!(report.roots.report_root, report_root.display().to_string());
        let validation = report
            .verification
            .iter()
            .find(|item| item.name == "validation report")
            .expect("validation verification item");
        assert_eq!(validation.path, "custom_reports/validation.json");
        assert!(validation.exists);
        assert_eq!(validation.ok, Some(true));

        fs::remove_dir_all(root).expect("remove temp");
    }

    #[test]
    fn ownership_rules_classify_crm_sample_product_surfaces() {
        let repo_root = Path::new("/work/app-framework");
        let crm_agent_profile = static_ownership(
            repo_root,
            Path::new("/work/app-framework/examples/products/crm/.appfw/agent-profile.yaml"),
            "examples/products/crm/.appfw/agent-profile.yaml",
        );
        assert_eq!(
            crm_agent_profile.classification,
            "product_agent_profile_source"
        );
        assert_eq!(crm_agent_profile.ownership, "application_source");
        assert!(crm_agent_profile.safe_to_edit);

        let crm_change_spec = static_ownership(
            repo_root,
            Path::new(
                "/work/app-framework/examples/products/crm/.appfw/specs/signature-experience-activity-queue.md",
            ),
            "examples/products/crm/.appfw/specs/signature-experience-activity-queue.md",
        );
        assert_eq!(crm_change_spec.classification, "product_change_spec_source");
        assert_eq!(crm_change_spec.ownership, "application_source");
        assert!(crm_change_spec.safe_to_edit);

        let handler = static_ownership(
            repo_root,
            Path::new(
                "/work/app-framework/examples/products/crm/backend/src/handlers/crm/account.rs",
            ),
            "examples/products/crm/backend/src/handlers/crm/account.rs",
        );
        assert_eq!(handler.classification, "human_owned_handler_extension");
        assert_eq!(handler.ownership, "human_owned");

        let facade = static_ownership(
            repo_root,
            Path::new("/work/app-framework/examples/products/crm/backend/src/product_api.rs"),
            "examples/products/crm/backend/src/product_api.rs",
        );
        assert_eq!(facade.classification, "backend_extension_api_facade");
        assert_eq!(facade.ownership, "framework_source");

        let retired_provider_certification = static_ownership(
            repo_root,
            Path::new(
                "/work/app-framework/examples/products/crm/backend/src/provider_certification.rs",
            ),
            "examples/products/crm/backend/src/provider_certification.rs",
        );
        assert_eq!(
            retired_provider_certification.classification,
            "retired_backend_provider_certification_tooling"
        );
        assert_eq!(retired_provider_certification.ownership, "framework_source");

        let error_facade = static_ownership(
            repo_root,
            Path::new("/work/app-framework/examples/products/crm/backend/src/routes/app_error.rs"),
            "examples/products/crm/backend/src/routes/app_error.rs",
        );
        assert_eq!(error_facade.classification, "backend_runtime_facade");
        assert_eq!(error_facade.ownership, "framework_source");

        let jwt_extractor_facade = static_ownership(
            repo_root,
            Path::new(
                "/work/app-framework/examples/products/crm/backend/src/handlers/auth/jwt_extractor.rs",
            ),
            "examples/products/crm/backend/src/handlers/auth/jwt_extractor.rs",
        );
        assert_eq!(
            jwt_extractor_facade.classification,
            "backend_runtime_facade"
        );
        assert_eq!(jwt_extractor_facade.ownership, "framework_source");

        let okta_verifier_facade = static_ownership(
            repo_root,
            Path::new(
                "/work/app-framework/examples/products/crm/backend/src/handlers/auth/okta_verifier.rs",
            ),
            "examples/products/crm/backend/src/handlers/auth/okta_verifier.rs",
        );
        assert_eq!(
            okta_verifier_facade.classification,
            "backend_runtime_facade"
        );
        assert_eq!(okta_verifier_facade.ownership, "framework_source");

        let service = static_ownership(
            repo_root,
            Path::new(
                "/work/app-framework/examples/products/crm/backend/src/services/account_service.rs",
            ),
            "examples/products/crm/backend/src/services/account_service.rs",
        );
        assert_eq!(service.classification, "human_owned_service_extension");
        assert_eq!(service.ownership, "human_owned");

        let runtime_contract_test = static_ownership(
            repo_root,
            Path::new("/work/app-framework/appfw_runtime/tests/runtime_extraction_contracts.rs"),
            "appfw_runtime/tests/runtime_extraction_contracts.rs",
        );
        assert_eq!(
            runtime_contract_test.classification,
            "runtime_package_source"
        );
        assert_eq!(runtime_contract_test.ownership, "framework_source");

        let saas_core_package_source = static_ownership(
            repo_root,
            Path::new("/work/app-framework/appfw_saas_core/src/lib.rs"),
            "appfw_saas_core/src/lib.rs",
        );
        assert_eq!(
            saas_core_package_source.classification,
            "saas_package_source"
        );
        assert_eq!(saas_core_package_source.ownership, "framework_source");

        let saas_testkit_package_source = static_ownership(
            repo_root,
            Path::new("/work/app-framework/appfw_saas_testkit/src/lib.rs"),
            "appfw_saas_testkit/src/lib.rs",
        );
        assert_eq!(
            saas_testkit_package_source.classification,
            "saas_package_source"
        );
        assert_eq!(saas_testkit_package_source.ownership, "framework_source");

        let provider_package_source = static_ownership(
            repo_root,
            Path::new("/work/app-framework/appfw_provider_postgres/src/param.rs"),
            "appfw_provider_postgres/src/param.rs",
        );
        assert_eq!(
            provider_package_source.classification,
            "provider_package_source"
        );
        assert_eq!(provider_package_source.ownership, "framework_source");

        let provider_vendor_contract = static_ownership(
            repo_root,
            Path::new("/work/app-framework/appfw_provider_servicenow/docs/vendor-contract.md"),
            "appfw_provider_servicenow/docs/vendor-contract.md",
        );
        assert_eq!(
            provider_vendor_contract.classification,
            "provider_vendor_contract_doc"
        );
        assert_eq!(provider_vendor_contract.ownership, "framework_source");
        assert!(provider_vendor_contract.safe_to_edit);

        let database_provider_source = static_ownership(
            repo_root,
            Path::new("/work/app-framework/database/src/postgres.rs"),
            "database/src/postgres.rs",
        );
        assert_eq!(
            database_provider_source.classification,
            "database_package_source"
        );
        assert_eq!(database_provider_source.ownership, "framework_source");
        assert!(database_provider_source.safe_to_edit);

        let chat_eval_fixture = static_ownership(
            repo_root,
            Path::new(
                "/work/app-framework/app_gen/_config/chat_evals/nexus/stalled_tasks_basic.yaml",
            ),
            "app_gen/_config/chat_evals/nexus/stalled_tasks_basic.yaml",
        );
        assert_eq!(chat_eval_fixture.classification, "chat_eval_fixture_source");
        assert_eq!(chat_eval_fixture.ownership, "framework_source");
        assert!(chat_eval_fixture.safe_to_edit);

        let framework_lock = static_ownership(
            repo_root,
            Path::new("/work/app-framework/Cargo.lock"),
            "Cargo.lock",
        );
        assert_eq!(framework_lock.classification, "cargo_workspace_lock");
        assert_eq!(framework_lock.ownership, "generated");
        assert!(!framework_lock.safe_to_edit);

        let crm_lock = static_ownership(
            repo_root,
            Path::new("/work/app-framework/examples/products/crm/Cargo.lock"),
            "examples/products/crm/Cargo.lock",
        );
        assert_eq!(crm_lock.classification, "cargo_workspace_lock");
        assert_eq!(crm_lock.ownership, "generated");

        let crm_gitignore = static_ownership(
            repo_root,
            Path::new("/work/app-framework/examples/products/crm/.gitignore"),
            "examples/products/crm/.gitignore",
        );
        assert_eq!(crm_gitignore.classification, "repository_ignore_rules");
        assert_eq!(crm_gitignore.ownership, "framework_source");

        let pds_tokens = static_ownership(
            repo_root,
            Path::new("/work/app-framework/appfw_ui/pds_health/tokens/pdsTokens.css"),
            "appfw_ui/pds_health/tokens/pdsTokens.css",
        );
        assert_eq!(pds_tokens.classification, "pds_health_design_system_source");
        assert_eq!(pds_tokens.ownership, "framework_source");
        assert!(pds_tokens.safe_to_edit);

        let design_skill = static_ownership(
            repo_root,
            Path::new("/work/app-framework/agent_skills/framework-frontend-design-system/SKILL.md"),
            "agent_skills/framework-frontend-design-system/SKILL.md",
        );
        assert_eq!(design_skill.classification, "agent_skill_source");
        assert_eq!(design_skill.ownership, "framework_source");
        assert!(design_skill.safe_to_edit);

        let appfw_cli_integration_test = static_ownership(
            repo_root,
            Path::new("/work/app-framework/appfw_cli/tests/instruction_routing.rs"),
            "appfw_cli/tests/instruction_routing.rs",
        );
        assert_eq!(
            appfw_cli_integration_test.classification,
            "workflow_cli_source"
        );
        assert_eq!(appfw_cli_integration_test.ownership, "framework_source");
        assert!(appfw_cli_integration_test.safe_to_edit);

        let crm_frontend_package = static_ownership(
            repo_root,
            Path::new("/work/app-framework/examples/products/crm/frontend/package.json"),
            "examples/products/crm/frontend/package.json",
        );
        assert_eq!(
            crm_frontend_package.classification,
            "product_frontend_scaffold_source"
        );
        assert_eq!(crm_frontend_package.ownership, "human_owned");
        assert!(crm_frontend_package.safe_to_edit);

        let crm_frontend_lock = static_ownership(
            repo_root,
            Path::new("/work/app-framework/examples/products/crm/frontend/package-lock.json"),
            "examples/products/crm/frontend/package-lock.json",
        );
        assert_eq!(
            crm_frontend_lock.classification,
            "product_frontend_dependency_lock"
        );
        assert_eq!(crm_frontend_lock.ownership, "generated");
        assert!(!crm_frontend_lock.safe_to_edit);
        assert_eq!(
            crm_frontend_lock.verification,
            vec![
                "npm run appfw:check".to_string(),
                "npm run build".to_string(),
                "scripts/appfw product frontend-test --json".to_string(),
                "scripts/appfw product validate --json".to_string(),
            ]
        );

        let crm_mobile_screen = static_ownership(
            repo_root,
            Path::new("/work/app-framework/examples/products/crm/mobile/app/index.tsx"),
            "examples/products/crm/mobile/app/index.tsx",
        );
        assert_eq!(
            crm_mobile_screen.classification,
            "product_mobile_scaffold_source"
        );
        assert_eq!(crm_mobile_screen.ownership, "human_owned");
        assert!(crm_mobile_screen.safe_to_edit);

        let crm_mobile_contract = static_ownership(
            repo_root,
            Path::new(
                "/work/app-framework/examples/products/crm/mobile/src/generated/appfw-mobile-contract.ts",
            ),
            "examples/products/crm/mobile/src/generated/appfw-mobile-contract.ts",
        );
        assert_eq!(
            crm_mobile_contract.classification,
            "mobile_generated_target_artifact"
        );
        assert_eq!(crm_mobile_contract.ownership, "generated");
        assert!(!crm_mobile_contract.safe_to_edit);

        let crm_mobile_entity_route = static_ownership(
            repo_root,
            Path::new("/work/app-framework/examples/products/crm/mobile/app/entities/accounts.tsx"),
            "examples/products/crm/mobile/app/entities/accounts.tsx",
        );
        assert_eq!(
            crm_mobile_entity_route.classification,
            "mobile_generated_target_artifact"
        );
        assert_eq!(crm_mobile_entity_route.ownership, "generated");
        assert!(!crm_mobile_entity_route.safe_to_edit);

        let crm_mobile_device_evidence = static_ownership(
            repo_root,
            Path::new(
                "/work/app-framework/examples/products/crm/mobile/.appfw-mobile/device-evidence.json",
            ),
            "examples/products/crm/mobile/.appfw-mobile/device-evidence.json",
        );
        assert_eq!(
            crm_mobile_device_evidence.classification,
            "mobile_diagnostic_evidence"
        );
        assert_eq!(crm_mobile_device_evidence.ownership, "human_owned");
        assert!(crm_mobile_device_evidence.safe_to_edit);

        let crm_mobile_audit_evidence = static_ownership(
            repo_root,
            Path::new(
                "/work/app-framework/examples/products/crm/mobile/.appfw-mobile/npm-audit-evidence.json",
            ),
            "examples/products/crm/mobile/.appfw-mobile/npm-audit-evidence.json",
        );
        assert_eq!(
            crm_mobile_audit_evidence.classification,
            "mobile_diagnostic_evidence"
        );
        assert_eq!(crm_mobile_audit_evidence.ownership, "human_owned");
        assert!(crm_mobile_audit_evidence.safe_to_edit);

        let crm_mobile_lock = static_ownership(
            repo_root,
            Path::new("/work/app-framework/examples/products/crm/mobile/package-lock.json"),
            "examples/products/crm/mobile/package-lock.json",
        );
        assert_eq!(
            crm_mobile_lock.classification,
            "product_mobile_dependency_lock"
        );
        assert_eq!(crm_mobile_lock.ownership, "generated");
        assert!(!crm_mobile_lock.safe_to_edit);

        let crm_mobile_test = static_ownership(
            repo_root,
            Path::new(
                "/work/app-framework/examples/products/crm/mobile/__tests__/mobile-contract.test.ts",
            ),
            "examples/products/crm/mobile/__tests__/mobile-contract.test.ts",
        );
        assert_eq!(
            crm_mobile_test.classification,
            "product_mobile_scaffold_source"
        );
        assert_eq!(crm_mobile_test.ownership, "human_owned");
        assert!(crm_mobile_test.safe_to_edit);
    }

    #[test]
    fn relative_path_between_points_product_manifests_at_framework_crates() {
        assert_eq!(
            relative_path_between(
                Path::new("/work/customer-crm/backend"),
                Path::new("/work/app-framework/appfw_runtime"),
            ),
            "../../app-framework/appfw_runtime"
        );
        assert_eq!(
            relative_path_between(
                Path::new("/work/app-framework/examples/products/crm/backend"),
                Path::new("/work/app-framework/appfw_runtime"),
            ),
            "../../../../appfw_runtime"
        );
    }

    #[test]
    fn packaged_product_scaffold_uses_registry_framework_crates() {
        let root = env::temp_dir().join(format!(
            "appfw-registry-dependency-test-{}",
            Utc::now().timestamp_nanos_opt().unwrap_or_default()
        ));
        let product = root.join("product");
        let framework = root.join("framework-package");
        fs::create_dir_all(framework.join("app_gen")).expect("create app_gen dir");
        fs::write(
            framework.join("app-framework-package.json"),
            r#"{"version":"0.1.1","dependency_source":"registry"}"#,
        )
        .expect("write package marker");
        fs::write(
            framework.join("app_gen/Cargo.toml"),
            r#"[package]
name = "appfw-codegen"
version = "0.1.1"
"#,
        )
        .expect("write codegen manifest");

        let intake = ProductIntake {
            source_kind: "poc".to_string(),
            app_name: "sample-product".to_string(),
            display_name: "Sample Product".to_string(),
            schema: "sample".to_string(),
            provider: "PostgreSQL".to_string(),
            data_source_name: "pg_primary".to_string(),
            mcp_server: false,
            kafka_client: true,
            ui: "enterprise".to_string(),
            poc_source: None,
            data_artifact: None,
            ui_artifact: None,
        };

        let backend_manifest = product_intake_backend_cargo(&product, &framework, &intake);
        assert!(backend_manifest.contains(
            r#"appfw_runtime = { package = "appfw-runtime", version = "0.1.1", registry = "pds-app-framework-crates", default-features = false }"#
        ));
        assert!(backend_manifest.contains(
            r#"appfw_provider_postgres = { package = "appfw-provider-postgres", version = "0.1.1", registry = "pds-app-framework-crates" }"#
        ));
        assert!(backend_manifest.contains(
            r#"appfw_mssql_auth = { package = "appfw-mssql-auth", version = "0.1.1", registry = "pds-app-framework-crates" }"#
        ));
        assert!(!backend_manifest.contains("path = "));

        let api_manifest = product_intake_api_tests_cargo(&product, &framework);
        assert!(api_manifest.contains(
            r#"appfw_test = { package = "appfw-test", version = "0.1.1", registry = "pds-app-framework-crates" }"#
        ));
        let cargo_config =
            product_framework_cargo_config(&framework).expect("packaged mode cargo config");
        assert!(cargo_config.contains("[registries.pds-app-framework-crates]"));
        assert!(cargo_config.contains(
            r#"index = "sparse+https://proget.pdsconnect.com/cargo/pds-app-framework-crates/""#
        ));

        fs::remove_dir_all(root).expect("remove temp");
    }

    #[test]
    fn split_root_cargo_rewrite_updates_runtime_crates() {
        let root = env::temp_dir().join(format!(
            "appfw-cargo-path-rewrite-test-{}",
            Utc::now().timestamp_nanos_opt().unwrap_or_default()
        ));
        let product = root.join("product");
        let framework = root.join("framework");
        fs::create_dir_all(product.join("backend")).expect("create backend dir");
        fs::create_dir_all(product.join("api_tests")).expect("create api_tests dir");
        fs::create_dir_all(product.join("rego_test")).expect("create rego_test dir");
        fs::create_dir_all(framework.join("appfw_runtime")).expect("create runtime dir");
        fs::create_dir_all(framework.join("appfw_test")).expect("create appfw_test dir");

        fs::write(
            product.join("backend/Cargo.toml"),
            r#"[dependencies]
appfw_runtime = { package = "appfw-runtime", path = "../../../../appfw_runtime", default-features = false, features = ["http", "kafka"] }
appfw_provider_mongo = { package = "appfw-provider-mongo", path = "../../../../appfw_provider_mongo" }
appfw_provider_mssql = { package = "appfw-provider-mssql", path = "../../../../appfw_provider_mssql" }
appfw_mssql_auth = { package = "appfw-mssql-auth", path = "../../../../appfw_mssql_auth" }
appfw_provider_postgres = { package = "appfw-provider-postgres", path = "../../../../appfw_provider_postgres", default-features = false }
appfw_provider_snowflake = { package = "appfw-provider-snowflake", path = "../../../../appfw_provider_snowflake" }
"#,
        )
        .expect("write backend manifest");
        fs::write(
            product.join("api_tests/Cargo.toml"),
            r#"[dependencies]
appfw_test = { package = "appfw-test", path = "../../../../appfw_test" }
"#,
        )
        .expect("write api tests manifest");
        fs::write(
            product.join("rego_test/Cargo.toml"),
            r#"[dependencies]
appfw_test = { package = "appfw-test", path = "../../../../appfw_test" }
"#,
        )
        .expect("write policy tests manifest");

        rewrite_split_root_cargo_paths(&product, &framework).expect("rewrite cargo paths");

        let backend_manifest =
            fs::read_to_string(product.join("backend/Cargo.toml")).expect("read backend manifest");
        assert!(backend_manifest.contains(
            r#"appfw_runtime = { package = "appfw-runtime", path = "../../framework/appfw_runtime", default-features = false, features = ["http", "kafka"] }"#
        ));
        assert!(backend_manifest.contains(
            r#"appfw_provider_mongo = { package = "appfw-provider-mongo", path = "../../framework/appfw_provider_mongo" }"#
        ));
        assert!(backend_manifest.contains(
            r#"appfw_provider_mssql = { package = "appfw-provider-mssql", path = "../../framework/appfw_provider_mssql" }"#
        ));
        assert!(backend_manifest.contains(
            r#"appfw_mssql_auth = { package = "appfw-mssql-auth", path = "../../framework/appfw_mssql_auth" }"#
        ));
        assert!(backend_manifest.contains(
            r#"appfw_provider_postgres = { package = "appfw-provider-postgres", path = "../../framework/appfw_provider_postgres", default-features = false }"#
        ));
        assert!(backend_manifest.contains(
            r#"appfw_provider_snowflake = { package = "appfw-provider-snowflake", path = "../../framework/appfw_provider_snowflake" }"#
        ));
        let api_tests_manifest = fs::read_to_string(product.join("api_tests/Cargo.toml"))
            .expect("read api tests manifest");
        assert!(api_tests_manifest.contains(
            r#"appfw_test = { package = "appfw-test", path = "../../framework/appfw_test" }"#
        ));
        let rego_test_manifest = fs::read_to_string(product.join("rego_test/Cargo.toml"))
            .expect("read policy tests manifest");
        assert!(rego_test_manifest.contains(
            r#"appfw_test = { package = "appfw-test", path = "../../framework/appfw_test" }"#
        ));

        fs::remove_dir_all(root).expect("remove temp");
    }

    #[test]
    fn split_root_cargo_rewrite_allows_absent_inactive_provider_deps() {
        let root = env::temp_dir().join(format!(
            "appfw-cargo-path-rewrite-pruned-provider-test-{}",
            Utc::now().timestamp_nanos_opt().unwrap_or_default()
        ));
        let product = root.join("product");
        let framework = root.join("framework");
        fs::create_dir_all(product.join("backend")).expect("create backend dir");
        fs::create_dir_all(product.join("api_tests")).expect("create api_tests dir");
        fs::create_dir_all(product.join("rego_test")).expect("create rego_test dir");
        fs::create_dir_all(framework.join("appfw_runtime")).expect("create runtime dir");
        fs::create_dir_all(framework.join("appfw_provider_postgres")).expect("create provider dir");
        fs::create_dir_all(framework.join("appfw_test")).expect("create appfw_test dir");

        fs::write(
            product.join("backend/Cargo.toml"),
            r#"[dependencies]
appfw_runtime = { package = "appfw-runtime", path = "../../../../appfw_runtime", default-features = false }
appfw_provider_postgres = { package = "appfw-provider-postgres", path = "../../../../appfw_provider_postgres", default-features = false }
"#,
        )
        .expect("write backend manifest");
        fs::write(
            product.join("api_tests/Cargo.toml"),
            r#"[dependencies]
appfw_test = { package = "appfw-test", path = "../../../../appfw_test" }
"#,
        )
        .expect("write api tests manifest");
        fs::write(
            product.join("rego_test/Cargo.toml"),
            r#"[dependencies]
appfw_test = { package = "appfw-test", path = "../../../../appfw_test" }
"#,
        )
        .expect("write policy tests manifest");

        rewrite_split_root_cargo_paths(&product, &framework).expect("rewrite cargo paths");

        let backend_manifest =
            fs::read_to_string(product.join("backend/Cargo.toml")).expect("read backend manifest");
        assert!(backend_manifest.contains(
            r#"appfw_runtime = { package = "appfw-runtime", path = "../../framework/appfw_runtime", default-features = false }"#
        ));
        assert!(backend_manifest.contains(
            r#"appfw_provider_postgres = { package = "appfw-provider-postgres", path = "../../framework/appfw_provider_postgres", default-features = false }"#
        ));
        assert!(!backend_manifest.contains("appfw_provider_mongo"));
        assert!(!backend_manifest.contains("appfw_provider_mssql"));
        assert!(!backend_manifest.contains("appfw_provider_snowflake"));

        fs::remove_dir_all(root).expect("remove temp");
    }

    #[test]
    fn boundary_check_accepts_product_api_imports() {
        let root = env::temp_dir().join(format!(
            "appfw-boundary-pass-test-{}",
            Utc::now().timestamp_nanos_opt().unwrap_or_default()
        ));
        let product = root.join("product");
        let framework = root.join("framework");
        let handler_dir = product.join("backend/src/handlers/crm");
        fs::create_dir_all(&handler_dir).expect("create handler dir");
        fs::write(
            handler_dir.join("account.rs"),
            r#"use crate::{product_api::{DataAccess, UserAuth}, schemas::crm::Account};

pub async fn enrich_account() {}
"#,
        )
        .expect("write handler");

        let roots = CommandRoots::split_layout(&product, &framework);
        let report = build_boundary_check(&roots).expect("build boundary check");

        assert!(report.ok);
        assert_eq!(report.checked_files, 1);
        assert!(report.violations.is_empty());
        fs::remove_dir_all(root).expect("remove temp");
    }

    #[test]
    fn boundary_check_rejects_retired_product_template_surfaces() {
        let root = env::temp_dir().join(format!(
            "appfw-boundary-retired-surface-test-{}",
            Utc::now().timestamp_nanos_opt().unwrap_or_default()
        ));
        let product = root.join("product");
        let framework = root.join("framework");
        let backend_src = product.join("backend/src");
        fs::create_dir_all(&backend_src).expect("create backend src");
        fs::write(backend_src.join("cors.rs"), "// retired runtime surface\n")
            .expect("write retired surface");
        fs::create_dir_all(backend_src.join("data")).expect("create backend data dir");
        fs::write(
            backend_src.join("data/validation.rs"),
            "// retired validation facade\n",
        )
        .expect("write retired validation surface");

        let roots = CommandRoots::split_layout(&product, &framework);
        let report = build_boundary_check(&roots).expect("build boundary check");

        assert!(!report.ok);
        assert!(report.violations.iter().any(|violation| {
            violation.rule == "retired_product_template_surface"
                && violation.path == "backend/src/cors.rs"
                && violation.symbol.as_deref() == Some("backend/src/cors.rs")
        }));
        assert!(report.violations.iter().any(|violation| {
            violation.rule == "retired_product_template_surface"
                && violation.path == "backend/src/data/validation.rs"
                && violation.symbol.as_deref() == Some("backend/src/data/validation.rs")
        }));
        fs::remove_dir_all(root).expect("remove temp");
    }

    #[test]
    fn boundary_check_rejects_retired_framework_root_surfaces() {
        let root = env::temp_dir().join(format!(
            "appfw-boundary-retired-framework-surface-test-{}",
            Utc::now().timestamp_nanos_opt().unwrap_or_default()
        ));
        let product = root.join("product");
        let framework = root.join("framework");
        let framework_backend_src = framework.join("backend/src");
        fs::create_dir_all(framework_backend_src.join("config")).expect("create config dir");
        fs::create_dir_all(framework_backend_src.join("data")).expect("create data dir");
        fs::write(
            framework_backend_src.join("config/security.rs"),
            "// retired framework config facade\n",
        )
        .expect("write retired framework surface");
        fs::write(
            framework_backend_src.join("data/query_cost.rs"),
            "// retired framework query-cost facade\n",
        )
        .expect("write retired framework query-cost surface");

        let roots = CommandRoots::split_layout(&product, &framework);
        let report = build_boundary_check(&roots).expect("build boundary check");

        assert!(!report.ok);
        assert!(report.violations.iter().any(|violation| {
            violation.rule == "retired_framework_root_surface"
                && violation.path == "backend/src/config/security.rs"
                && violation.symbol.as_deref() == Some("backend/src/config/security.rs")
        }));
        assert!(report.violations.iter().any(|violation| {
            violation.rule == "retired_framework_root_surface"
                && violation.path == "backend/src/data/query_cost.rs"
                && violation.symbol.as_deref() == Some("backend/src/data/query_cost.rs")
        }));
        fs::remove_dir_all(root).expect("remove temp");
    }

    #[test]
    fn boundary_check_rejects_inactive_provider_source() {
        let root = env::temp_dir().join(format!(
            "appfw-boundary-inactive-provider-test-{}",
            Utc::now().timestamp_nanos_opt().unwrap_or_default()
        ));
        let product = root.join("product");
        let framework = root.join("framework");
        fs::create_dir_all(product.join(".appfw/model/data_sources"))
            .expect("create data source config dir");
        fs::create_dir_all(product.join(".appfw/model/schemas/crm"))
            .expect("create schema config dir");
        fs::write(
            product.join(".appfw/model/data_sources/_res.yaml"),
            r#"
- name: pg_primary
  data_source_type: PostgreSQL
- name: mongo_primary
  data_source_type: MongoDB
"#,
        )
        .expect("write data sources");
        fs::write(
            product.join(".appfw/model/schemas/crm/_res.yaml"),
            r#"
name: crm
data_source_name: pg_primary
"#,
        )
        .expect("write schema config");
        fs::create_dir_all(product.join("backend/src/data/clients/postgres"))
            .expect("create postgres provider source");
        fs::write(
            product.join("backend/src/data/clients/postgres/postgres_client.rs"),
            "// active provider adapter\n",
        )
        .expect("write active provider source");
        fs::create_dir_all(product.join("backend/src/data/clients/mongo"))
            .expect("create mongo provider source");
        fs::write(
            product.join("backend/src/data/clients/mongo/mongo_client.rs"),
            "// inactive provider adapter\n",
        )
        .expect("write inactive provider source");

        let roots = CommandRoots::split_layout(&product, &framework);
        let report = build_boundary_check(&roots).expect("build boundary check");

        assert!(!report.ok);
        assert!(report.violations.iter().any(|violation| {
            violation.rule == "inactive_provider_source"
                && violation.path == "backend/src/data/clients/mongo"
                && violation.symbol.as_deref() == Some("backend/src/data/clients/mongo")
        }));
        assert_eq!(report.product_provider_sources.len(), 1);
        let provider_report = &report.product_provider_sources[0];
        assert_eq!(provider_report.active_providers, vec!["postgres"]);
        assert!(provider_report
            .sources
            .iter()
            .any(|source| source.provider == "postgres"
                && source.active
                && source.rust_files == 1));
        assert!(provider_report
            .sources
            .iter()
            .any(|source| source.provider == "mongo" && !source.active && source.rust_files == 1));
        fs::remove_dir_all(root).expect("remove temp");
    }

    #[test]
    fn boundary_check_allows_fabric_shared_mssql_provider_surface() {
        let root = env::temp_dir().join(format!(
            "appfw-boundary-fabric-shared-mssql-test-{}",
            Utc::now().timestamp_nanos_opt().unwrap_or_default()
        ));
        let product = root.join("product");
        let framework = root.join("framework");
        fs::create_dir_all(product.join(".appfw/model/data_sources"))
            .expect("create data source config dir");
        fs::create_dir_all(product.join(".appfw/model/schemas/equity"))
            .expect("create schema config dir");
        fs::create_dir_all(product.join("backend/src/data/clients/fabric_sql_analytics"))
            .expect("create fabric provider source");
        fs::create_dir_all(product.join("backend/src/data/clients/mssql"))
            .expect("create shared mssql provider source");
        fs::write(
            product.join(".appfw/model/data_sources/_res.yaml"),
            r#"
- name: fabric_primary
  data_source_type: FabricSqlAnalytics
"#,
        )
        .expect("write data sources");
        fs::write(
            product.join(".appfw/model/schemas/equity/_res.yaml"),
            r#"
name: equity
data_source_name: fabric_primary
"#,
        )
        .expect("write schema config");
        fs::write(
            product.join(
                "backend/src/data/clients/fabric_sql_analytics/fabric_sql_analytics_client.rs",
            ),
            "// active fabric provider adapter\n",
        )
        .expect("write fabric provider source");
        fs::write(
            product.join("backend/src/data/clients/mssql/filter.rs"),
            "// shared mssql helper used by fabric provider\n",
        )
        .expect("write shared mssql provider source");
        fs::write(
            product.join("backend/Cargo.toml"),
            r#"
[package]
name = "backend"
version = "0.1.0"
edition = "2021"

[dependencies]
appfw_provider_mssql = { package = "appfw-provider-mssql", path = "../../framework/appfw_provider_mssql" }
"#,
        )
        .expect("write backend manifest");

        let roots = CommandRoots::split_layout(&product, &framework);
        let report = build_boundary_check(&roots).expect("build boundary check");

        assert!(report.ok, "unexpected violations: {:?}", report.violations);
        assert_eq!(report.product_provider_sources.len(), 1);
        let provider_report = &report.product_provider_sources[0];
        assert_eq!(
            provider_report.active_providers,
            vec!["fabric_sql_analytics", "mssql"]
        );
        assert!(provider_report
            .sources
            .iter()
            .any(|source| source.provider == "fabric_sql_analytics"
                && source.active
                && source.rust_files == 1));
        assert!(provider_report
            .sources
            .iter()
            .any(|source| source.provider == "mssql" && source.active && source.rust_files == 1));
        assert!(provider_report.dependencies.iter().any(|dependency| {
            dependency.provider == "mssql"
                && dependency.dependency == "appfw_provider_mssql"
                && dependency.active
        }));
        fs::remove_dir_all(root).expect("remove temp");
    }

    #[test]
    fn boundary_check_rejects_inactive_provider_dependency() {
        let root = env::temp_dir().join(format!(
            "appfw-boundary-inactive-provider-dep-test-{}",
            Utc::now().timestamp_nanos_opt().unwrap_or_default()
        ));
        let product = root.join("product");
        let framework = root.join("framework");
        fs::create_dir_all(product.join(".appfw/model/data_sources"))
            .expect("create data source config dir");
        fs::create_dir_all(product.join(".appfw/model/schemas/crm"))
            .expect("create schema config dir");
        fs::create_dir_all(product.join("backend/src")).expect("create backend src dir");
        fs::write(
            product.join(".appfw/model/data_sources/_res.yaml"),
            r#"
- name: pg_primary
  data_source_type: PostgreSQL
- name: mongo_primary
  data_source_type: MongoDB
"#,
        )
        .expect("write data sources");
        fs::write(
            product.join(".appfw/model/schemas/crm/_res.yaml"),
            r#"
name: crm
data_source_name: pg_primary
"#,
        )
        .expect("write schema config");
        fs::write(
            product.join("backend/Cargo.toml"),
            r#"
[package]
name = "backend"
version = "0.1.0"
edition = "2021"

[dependencies]
appfw_provider_postgres = { package = "appfw-provider-postgres", path = "../../framework/appfw_provider_postgres" }
appfw_provider_mongo = { package = "appfw-provider-mongo", path = "../../framework/appfw_provider_mongo" }
"#,
        )
        .expect("write backend manifest");

        let roots = CommandRoots::split_layout(&product, &framework);
        let report = build_boundary_check(&roots).expect("build boundary check");

        assert!(!report.ok);
        assert!(report.violations.iter().any(|violation| {
            violation.rule == "inactive_provider_dependency"
                && violation.path == "backend/Cargo.toml"
                && violation.symbol.as_deref() == Some("appfw_provider_mongo")
        }));
        let provider_report = &report.product_provider_sources[0];
        assert_eq!(provider_report.active_providers, vec!["postgres"]);
        assert!(provider_report.dependencies.iter().any(|dependency| {
            dependency.provider == "postgres"
                && dependency.dependency == "appfw_provider_postgres"
                && dependency.active
        }));
        assert!(provider_report.dependencies.iter().any(|dependency| {
            dependency.provider == "mongo"
                && dependency.dependency == "appfw_provider_mongo"
                && !dependency.active
        }));
        fs::remove_dir_all(root).expect("remove temp");
    }

    #[test]
    fn boundary_check_checks_crm_sample_when_framework_root_is_app_root() {
        let root = env::temp_dir().join(format!(
            "appfw-boundary-crm-sample-test-{}",
            Utc::now().timestamp_nanos_opt().unwrap_or_default()
        ));
        fs::create_dir_all(root.join("app_gen")).expect("create app_gen");
        fs::create_dir_all(root.join("appfw_runtime")).expect("create runtime");
        fs::write(
            root.join("app_gen/Cargo.toml"),
            "[package]\nname = \"appfw-codegen\"\n",
        )
        .expect("write app_gen manifest");
        fs::write(
            root.join("appfw_runtime/Cargo.toml"),
            "[package]\nname = \"appfw-runtime\"\n",
        )
        .expect("write runtime manifest");
        let crm_backend_src = root.join("examples/products/crm/backend/src");
        fs::create_dir_all(crm_backend_src.join("observability"))
            .expect("create crm observability dir");
        fs::write(
            crm_backend_src.join("observability/mod.rs"),
            "// retired runtime surface\n",
        )
        .expect("write retired surface");

        let roots = CommandRoots::split_layout(&root, &root);
        let report = build_boundary_check(&roots).expect("build boundary check");

        assert!(!report.ok);
        assert!(report.violations.iter().any(|violation| {
            violation.rule == "retired_product_template_surface"
                && violation.path == "examples/products/crm/backend/src/observability/mod.rs"
        }));
        fs::remove_dir_all(root).expect("remove temp");
    }

    #[test]
    fn boundary_check_does_not_require_root_backend_fixture() {
        let root = env::temp_dir().join(format!(
            "appfw-boundary-no-root-backend-test-{}",
            Utc::now().timestamp_nanos_opt().unwrap_or_default()
        ));
        fs::create_dir_all(root.join("app_gen")).expect("create app_gen");
        fs::create_dir_all(root.join("appfw_runtime")).expect("create runtime");
        fs::write(
            root.join("app_gen/Cargo.toml"),
            "[package]\nname = \"appfw-codegen\"\n",
        )
        .expect("write app_gen manifest");
        fs::write(
            root.join("appfw_runtime/Cargo.toml"),
            "[package]\nname = \"appfw-runtime\"\n",
        )
        .expect("write runtime manifest");
        fs::create_dir_all(root.join("examples/products/crm/backend/src"))
            .expect("create crm product backend");

        let roots = CommandRoots::split_layout(&root, &root);
        let report = build_boundary_check(&roots).expect("build boundary check");

        assert!(report.ok);
        assert!(report.violations.is_empty());
        assert!(!root.join("backend").exists());
        fs::remove_dir_all(root).expect("remove temp");
    }

    #[test]
    fn command_roots_default_framework_checkout_to_crm_sample_app_root() {
        let root = env::temp_dir().join(format!(
            "appfw-command-roots-framework-default-test-{}",
            Utc::now().timestamp_nanos_opt().unwrap_or_default()
        ));
        fs::create_dir_all(root.join("scripts")).expect("create scripts dir");
        fs::create_dir_all(root.join("app_gen/_templates")).expect("create templates dir");
        fs::create_dir_all(root.join("appfw_runtime")).expect("create runtime dir");
        fs::create_dir_all(root.join("examples/products/crm/.appfw"))
            .expect("create crm manifest dir");
        fs::create_dir_all(root.join("examples/products/crm/.appfw/model"))
            .expect("create crm config dir");
        fs::write(root.join("scripts/appfw"), "#!/usr/bin/env bash\n")
            .expect("write appfw wrapper");
        fs::write(
            root.join("examples/products/crm/.appfw/manifest.yaml"),
            "app:\n  name: crm\n",
        )
        .expect("write crm manifest");

        let roots = CommandRoots::resolve(&root, None, None, None, None, None, None)
            .expect("resolve roots");

        assert_eq!(roots.framework_root, normalize_path(&root));
        assert_eq!(
            roots.app_root,
            normalize_path(&root.join("examples/products/crm"))
        );
        assert_eq!(
            roots.config_root,
            normalize_path(&root.join("examples/products/crm/.appfw/model"))
        );
        assert!(!root.join("backend").exists());

        fs::remove_dir_all(root).expect("remove temp");
    }

    #[test]
    fn boundary_check_rejects_auth_handler_imports() {
        let root = env::temp_dir().join(format!(
            "appfw-boundary-auth-fail-test-{}",
            Utc::now().timestamp_nanos_opt().unwrap_or_default()
        ));
        let product = root.join("product");
        let framework = root.join("framework");
        let handler_dir = product.join("backend/src/handlers/crm");
        fs::create_dir_all(&handler_dir).expect("create handler dir");
        fs::write(
            handler_dir.join("account.rs"),
            r#"use crate::handlers::auth::user_auth::UserAuth;

pub async fn enrich_account(_user: Option<UserAuth>) {}
"#,
        )
        .expect("write handler");

        let roots = CommandRoots::split_layout(&product, &framework);
        let report = build_boundary_check(&roots).expect("build boundary check");

        assert!(!report.ok);
        assert!(report
            .violations
            .iter()
            .any(|violation| violation.rule == "auth handler internals"));
        fs::remove_dir_all(root).expect("remove temp");
    }

    #[test]
    fn boundary_check_rejects_graphql_context_imports() {
        let root = env::temp_dir().join(format!(
            "appfw-boundary-graphql-context-fail-test-{}",
            Utc::now().timestamp_nanos_opt().unwrap_or_default()
        ));
        let product = root.join("product");
        let framework = root.join("framework");
        let handler_dir = product.join("backend/src/handlers/crm");
        fs::create_dir_all(&handler_dir).expect("create handler dir");
        fs::write(
            handler_dir.join("account.rs"),
            r#"use async_graphql::{Context, FieldResult};

pub async fn enrich_account(_ctx: &Context<'_>) -> FieldResult<String> {
    Ok("ok".to_string())
}
"#,
        )
        .expect("write handler");

        let roots = CommandRoots::split_layout(&product, &framework);
        let report = build_boundary_check(&roots).expect("build boundary check");

        assert!(!report.ok);
        assert!(report
            .violations
            .iter()
            .any(|violation| violation.rule == "GraphQL context internals"));
        fs::remove_dir_all(root).expect("remove temp");
    }

    #[test]
    fn boundary_check_rejects_context_data_extraction_in_services() {
        let root = env::temp_dir().join(format!(
            "appfw-boundary-service-context-fail-test-{}",
            Utc::now().timestamp_nanos_opt().unwrap_or_default()
        ));
        let product = root.join("product");
        let framework = root.join("framework");
        let service_dir = product.join("backend/src/services");
        fs::create_dir_all(&service_dir).expect("create service dir");
        fs::write(
            service_dir.join("account_service.rs"),
            r#"struct RequestContext;

pub fn leak_context(context: RequestContext) {
    let _ = context.data_unchecked::<usize>();
}
"#,
        )
        .expect("write service");

        let roots = CommandRoots::split_layout(&product, &framework);
        let report = build_boundary_check(&roots).expect("build boundary check");

        assert!(!report.ok);
        assert!(report.violations.iter().any(|violation| {
            violation.rule == "GraphQL context internals"
                && violation.symbol.as_deref() == Some("context.data_unchecked")
        }));
        fs::remove_dir_all(root).expect("remove temp");
    }

    #[test]
    fn boundary_check_reports_internal_imports_and_implicit_overrides() {
        let root = env::temp_dir().join(format!(
            "appfw-boundary-fail-test-{}",
            Utc::now().timestamp_nanos_opt().unwrap_or_default()
        ));
        let product = root.join("product");
        let framework = root.join("framework");
        let handler_dir = product.join("backend/src/handlers/crm");
        fs::create_dir_all(&handler_dir).expect("create handler dir");
        fs::write(
            handler_dir.join("account.rs"),
            r#"use crate::{product_api::DataAccess, data::{clients as provider_clients}};

pub async fn find_impl() {}
"#,
        )
        .expect("write handler");

        let roots = CommandRoots::split_layout(&product, &framework);
        let report = build_boundary_check(&roots).expect("build boundary check");

        assert!(!report.ok);
        assert!(report
            .violations
            .iter()
            .any(|violation| violation.rule == "data runtime internals"));
        assert!(report
            .violations
            .iter()
            .any(|violation| violation.rule == "implicit_standard_handler_override"));
        fs::remove_dir_all(root).expect("remove temp");
    }

    #[test]
    fn copy_overlay_records_relative_files() {
        let root = env::temp_dir().join(format!(
            "appfw-overlay-test-{}",
            Utc::now().timestamp_nanos_opt().unwrap_or_default()
        ));
        let overlay = root.join("overlay");
        let target = root.join("target");
        fs::create_dir_all(overlay.join(".appfw")).expect("create overlay");
        fs::write(overlay.join("README.md"), "hello").expect("write readme");
        fs::write(overlay.join(".appfw/profile.json"), "{}").expect("write profile");

        let mut copied = Vec::new();
        copy_overlay(&overlay, &overlay, &target, &mut copied).expect("copy overlay");
        copied.sort();

        assert_eq!(
            copied,
            vec![".appfw/profile.json".to_string(), "README.md".to_string()]
        );
        assert_eq!(
            fs::read_to_string(target.join("README.md")).expect("read copied readme"),
            "hello"
        );
        fs::remove_dir_all(root).expect("remove temp");
    }

    #[test]
    fn local_env_shell_quote_preserves_paths() {
        assert_eq!(
            shell_single_quote("/Users/test/app-framework"),
            "'/Users/test/app-framework'"
        );
        assert_eq!(shell_single_quote("/tmp/o'hare"), "'/tmp/o'\\''hare'");
    }

    #[test]
    fn compose_source_rewrite_preserves_product_and_volume_sources() {
        assert_eq!(
            rewrite_compose_source_line(
                "        source: ../../../appfw_runtime",
                "../app-framework/appfw_runtime",
                "../app-framework/observability",
            ),
            "        source: ../app-framework/appfw_runtime"
        );
        assert_eq!(
            rewrite_compose_source_line(
                "        source: ../../../observability/loki/loki-config.yaml",
                "../app-framework/appfw_runtime",
                "../app-framework/observability",
            ),
            "        source: ../app-framework/observability/loki/loki-config.yaml"
        );
        assert_eq!(
            rewrite_compose_source_line(
                "        source: backend-observability-logs",
                "../app-framework/appfw_runtime",
                "../app-framework/observability",
            ),
            "        source: backend-observability-logs"
        );
    }
}
