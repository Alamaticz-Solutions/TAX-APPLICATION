#![allow(
    clippy::collapsible_match,
    clippy::manual_contains,
    clippy::needless_borrow,
    clippy::needless_borrows_for_generic_args,
    clippy::print_literal,
    clippy::ptr_arg,
    clippy::question_mark,
    clippy::too_many_arguments,
    clippy::unnecessary_lazy_evaluations,
    clippy::unnecessary_map_or
)]

extern crate serde_derive;
extern crate serde_json;
extern crate tera;

use anyhow::Context;
use dotenv::dotenv;

mod api;
pub use api::{
    generate, validate, Codegen, CodegenArtifactPaths, CodegenMode, CodegenOptions, CodegenReport,
    CodegenRoots,
};

mod app_manifest;
mod app_workspace;
mod backend;
mod bootstrap_types;
mod config_contract;
mod dev_infra;
mod frontend;
mod normalized_config;
mod relationship_config;
mod schema;
pub mod schema_route;
pub use schema_route::schema_route_segment;
mod schemas;
pub mod sync_descriptor;
mod sync_worker;
mod utils;
mod validation;

use utils::console;

pub fn run_from_args(args: impl IntoIterator<Item = String>) -> anyhow::Result<()> {
    dotenv().ok();

    let process_cwd = std::env::current_dir()?;
    let (workspace, options) = app_workspace::AppWorkspace::from_args(args)?;
    console::cwd(&process_cwd);
    console::verbose_path("app root", &workspace.app_root);
    console::verbose_path("framework root", &workspace.framework_root);
    console::verbose_path("generator root", &workspace.generator_root);
    console::verbose_path("config root", &workspace.config_root);
    console::verbose_path("templates root", &workspace.templates_root);
    console::verbose_path("report root", &workspace.report_root);

    // Bootstrap-only: uncomment this when the system metadata shape changes
    // and app_gen/src/bootstrap_types/*.rs needs to be regenerated.
    // utils::bootstrap_types_gen::run(&workspace)?;

    execute_codegen(
        &workspace,
        if options.validate_only {
            CodegenMode::ValidateOnly
        } else {
            CodegenMode::Generate
        },
    )?;

    Ok(())
}

pub(crate) fn execute_codegen(
    workspace: &app_workspace::AppWorkspace,
    mode: CodegenMode,
) -> anyhow::Result<CodegenReport> {
    console::app_start("app_gen");

    utils::artifacts::begin_run(workspace)?;

    // Dependency and output-path safety are generation preconditions. Resolve
    // them before validation writes reports or any generated output changes.
    let framework_assets = if mode == CodegenMode::Generate {
        Some(dev_infra::preflight(workspace)?)
    } else {
        None
    };

    // Fail before generation mutates outputs. The report is always emitted at
    // the configured report root so agents and editor integrations can consume it.
    validation::run(&workspace)?;
    sync_descriptor::emit_sync_descriptor_report(&workspace)?;
    if mode == CodegenMode::ValidateOnly {
        console::app_done("app_gen");
        return Ok(CodegenReport::from_workspace(workspace, mode));
    }

    // Process each schema.
    let generator_ir = schemas::run(&workspace)?;
    frontend::run(&workspace, &generator_ir)?;

    // Generate backend code from _res.yaml files in system sub-directories.
    backend::run(&workspace)?;
    sync_worker::run(&workspace)?;
    dev_infra::run(
        &workspace,
        framework_assets
            .as_ref()
            .context("generated dev infra dependency preflight is missing")?,
    )?;
    utils::artifacts::write_manifest(&workspace)?;

    console::app_done("app_gen");
    Ok(CodegenReport::from_workspace(workspace, mode))
}
