use std::{
    env,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result};
use serde::Serialize;

use crate::app_workspace::AppWorkspace;

/// Stable generation modes exposed by the `appfw-codegen` crate.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[non_exhaustive]
pub enum CodegenMode {
    /// Validate product topology and config, then stop before writing generated outputs.
    ValidateOnly,
    /// Validate product topology and config, then write generated outputs.
    Generate,
}

/// Versionable options for invoking App Framework code generation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[non_exhaustive]
pub struct CodegenOptions {
    pub mode: CodegenMode,
}

impl CodegenOptions {
    pub fn validate_only() -> Self {
        Self {
            mode: CodegenMode::ValidateOnly,
        }
    }

    pub fn generate() -> Self {
        Self {
            mode: CodegenMode::Generate,
        }
    }
}

impl Default for CodegenOptions {
    fn default() -> Self {
        Self::generate()
    }
}

/// Explicit product/framework roots used by the stable codegen API.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[non_exhaustive]
pub struct CodegenRoots {
    pub app_root: PathBuf,
    pub framework_root: PathBuf,
    pub generator_root: PathBuf,
    pub config_root: PathBuf,
    pub templates_root: PathBuf,
    pub report_root: PathBuf,
}

impl CodegenRoots {
    pub fn new(
        app_root: impl Into<PathBuf>,
        framework_root: impl Into<PathBuf>,
        generator_root: impl Into<PathBuf>,
        config_root: impl Into<PathBuf>,
        templates_root: impl Into<PathBuf>,
        report_root: impl Into<PathBuf>,
    ) -> Self {
        Self {
            app_root: app_root.into(),
            framework_root: framework_root.into(),
            generator_root: generator_root.into(),
            config_root: config_root.into(),
            templates_root: templates_root.into(),
            report_root: report_root.into(),
        }
    }

    pub fn copied_layout(app_root: impl Into<PathBuf>) -> Self {
        let app_root = app_root.into();
        Self::split_layout(app_root.clone(), app_root)
    }

    pub fn split_layout(app_root: impl Into<PathBuf>, framework_root: impl Into<PathBuf>) -> Self {
        let app_root = app_root.into();
        let framework_root = framework_root.into();
        let generator_root = framework_root.join("app_gen");
        Self {
            app_root: app_root.clone(),
            framework_root,
            config_root: app_root.join(".appfw/model"),
            templates_root: generator_root.join("_templates"),
            report_root: app_root.join(".appfw/target/appfw"),
            generator_root,
        }
    }

    pub fn from_env_or_current_dir() -> Result<Self> {
        let (workspace, _) = AppWorkspace::from_args(std::iter::empty::<String>())?;
        Ok(Self::from_workspace(&workspace))
    }

    pub fn resolve(self) -> Result<Self> {
        let cwd = env::current_dir().context("could not read current directory")?;
        Ok(Self::from_workspace(&self.into_workspace(&cwd)?))
    }

    pub fn artifact_paths(&self) -> CodegenArtifactPaths {
        CodegenArtifactPaths::from_report_root(&self.report_root)
    }

    pub fn cargo_rerun_paths(&self) -> Vec<PathBuf> {
        vec![
            self.config_root.clone(),
            self.templates_root.clone(),
            self.generator_root.join("src"),
            self.app_root.join(".appfw/manifest.yaml"),
        ]
    }

    pub fn emit_cargo_rerun_if_changed(&self) {
        for path in self.cargo_rerun_paths() {
            println!("cargo:rerun-if-changed={}", path.display());
        }
    }

    pub(crate) fn into_workspace(self, cwd: &Path) -> Result<AppWorkspace> {
        AppWorkspace::from_roots(
            self.app_root,
            self.framework_root,
            self.generator_root,
            self.config_root,
            self.templates_root,
            self.report_root,
            cwd,
        )
    }

    pub(crate) fn from_workspace(workspace: &AppWorkspace) -> Self {
        Self {
            app_root: workspace.app_root.clone(),
            framework_root: workspace.framework_root.clone(),
            generator_root: workspace.generator_root.clone(),
            config_root: workspace.config_root.clone(),
            templates_root: workspace.templates_root.clone(),
            report_root: workspace.report_root.clone(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[non_exhaustive]
pub struct CodegenArtifactPaths {
    pub validation_report: PathBuf,
    pub app_topology_report: PathBuf,
    pub dev_infra_report: PathBuf,
    pub config_contract_json: PathBuf,
    pub config_contract_markdown: PathBuf,
    pub sync_descriptors: PathBuf,
    pub sync_worker_plan: PathBuf,
    pub generator_ir: PathBuf,
    pub performance_recommendations_json: PathBuf,
    pub performance_recommendations_markdown: PathBuf,
    pub artifact_manifest: PathBuf,
    pub artifact_provenance: PathBuf,
}

impl CodegenArtifactPaths {
    pub fn from_report_root(report_root: impl AsRef<Path>) -> Self {
        let report_root = report_root.as_ref();
        Self {
            validation_report: report_root.join("validation.json"),
            app_topology_report: report_root.join("app_topology.json"),
            dev_infra_report: report_root.join("dev_infra.json"),
            config_contract_json: report_root.join("config_contract.json"),
            config_contract_markdown: report_root.join("config_contract.md"),
            sync_descriptors: report_root.join("sync_descriptors.json"),
            sync_worker_plan: report_root.join("sync_worker_plan.json"),
            generator_ir: report_root.join("generator_ir.json"),
            performance_recommendations_json: report_root.join("performance_recommendations.json"),
            performance_recommendations_markdown: report_root
                .join("performance_recommendations.md"),
            artifact_manifest: report_root.join("artifacts.json"),
            artifact_provenance: report_root.join("artifact_provenance.json"),
        }
    }
}

/// Execution report returned by stable codegen API calls.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[non_exhaustive]
pub struct CodegenReport {
    pub mode: CodegenMode,
    pub generated: bool,
    pub roots: CodegenRoots,
    pub artifacts: CodegenArtifactPaths,
}

impl CodegenReport {
    pub(crate) fn from_workspace(workspace: &AppWorkspace, mode: CodegenMode) -> Self {
        let roots = CodegenRoots::from_workspace(workspace);
        let artifacts = roots.artifact_paths();
        Self {
            mode,
            generated: mode == CodegenMode::Generate,
            roots,
            artifacts,
        }
    }
}

/// Stable builder for invoking App Framework code generation from Rust.
#[derive(Debug, Clone)]
pub struct Codegen {
    roots: CodegenRoots,
    options: CodegenOptions,
}

impl Codegen {
    pub fn new(roots: CodegenRoots) -> Self {
        Self {
            roots,
            options: CodegenOptions::default(),
        }
    }

    pub fn with_options(mut self, options: CodegenOptions) -> Self {
        self.options = options;
        self
    }

    pub fn validate_only(mut self) -> Self {
        self.options = CodegenOptions::validate_only();
        self
    }

    pub fn generate(mut self) -> Self {
        self.options = CodegenOptions::generate();
        self
    }

    pub fn emit_cargo_rerun_if_changed(&self) {
        self.roots.emit_cargo_rerun_if_changed();
    }

    pub fn run(self) -> Result<CodegenReport> {
        let cwd = env::current_dir().context("could not read current directory")?;
        let workspace = self.roots.into_workspace(&cwd)?;
        crate::execute_codegen(&workspace, self.options.mode)
    }
}

pub fn validate(roots: CodegenRoots) -> Result<CodegenReport> {
    Codegen::new(roots).validate_only().run()
}

pub fn generate(roots: CodegenRoots) -> Result<CodegenReport> {
    Codegen::new(roots).generate().run()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn split_layout_keeps_product_config_and_framework_templates_separate() {
        let roots = CodegenRoots::split_layout("/product", "/framework");

        assert_eq!(roots.app_root, PathBuf::from("/product"));
        assert_eq!(roots.framework_root, PathBuf::from("/framework"));
        assert_eq!(roots.generator_root, PathBuf::from("/framework/app_gen"));
        assert_eq!(roots.config_root, PathBuf::from("/product/.appfw/model"));
        assert_eq!(
            roots.templates_root,
            PathBuf::from("/framework/app_gen/_templates")
        );
        assert_eq!(
            roots.report_root,
            PathBuf::from("/product/.appfw/target/appfw")
        );
    }

    #[test]
    fn validate_options_do_not_generate_outputs() {
        let options = CodegenOptions::validate_only();

        assert_eq!(options.mode, CodegenMode::ValidateOnly);
    }

    #[test]
    fn artifact_paths_are_report_root_relative() {
        let artifacts = CodegenArtifactPaths::from_report_root("/product/.appfw/target/appfw");

        assert_eq!(
            artifacts.validation_report,
            PathBuf::from("/product/.appfw/target/appfw/validation.json")
        );
        assert_eq!(
            artifacts.artifact_manifest,
            PathBuf::from("/product/.appfw/target/appfw/artifacts.json")
        );
        assert_eq!(
            artifacts.sync_descriptors,
            PathBuf::from("/product/.appfw/target/appfw/sync_descriptors.json")
        );
        assert_eq!(
            artifacts.sync_worker_plan,
            PathBuf::from("/product/.appfw/target/appfw/sync_worker_plan.json")
        );
    }
}
