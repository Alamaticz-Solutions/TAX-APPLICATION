use crate::app_workspace::AppWorkspace;
use crate::utils::{
    console, constants::*, context, files, performance_recommendations, type_relationships,
};
use crate::{normalized_config, schema};
use anyhow::{Context, Result};

pub fn run(workspace: &AppWorkspace) -> Result<normalized_config::GeneratorIr> {
    console::stage("schemas");

    preprocess(workspace, true).context("preprocess system schema")?;
    preprocess(workspace, false).context("preprocess application schemas")?;

    let entity_types = type_relationships::run(workspace).context("resolve type relationships")?;
    let generator_ir =
        normalized_config::build(workspace, &entity_types).context("build generator IR")?;
    normalized_config::emit(workspace, &generator_ir).context("emit generator IR")?;
    performance_recommendations::emit(workspace, &entity_types)
        .context("emit performance recommendations")?;

    publish(workspace, true, &generator_ir).context("publish system schema")?;
    publish(workspace, false, &generator_ir).context("publish application schemas")?;

    console::done("schemas");

    Ok(generator_ir)
}

fn preprocess(workspace: &AppWorkspace, system_run: bool) -> Result<()> {
    for dir_name in schema_dir_names(workspace)? {
        if system_run && dir_name.ne(ctx_name::SYS_SCHEMA_NAME) {
            continue;
        }
        if !system_run && dir_name.eq(ctx_name::SYS_SCHEMA_NAME) {
            continue;
        }
        schema::preprocess(workspace, &dir_name)
            .with_context(|| format!("preprocess schema `{dir_name}`"))?;
    }
    Ok(())
}

fn publish(
    workspace: &AppWorkspace,
    system_run: bool,
    generator_ir: &normalized_config::GeneratorIr,
) -> Result<()> {
    let data_sources_src_file = files::get_data_sources_file(workspace);
    // Deserialize the data sources yaml. Ensure it is configured correctly
    let data_sources = context::read_data_sources(&data_sources_src_file)?;

    let target_backend_config_dir = files::get_backend_config_dir(workspace);
    schema::publish_data_sources(workspace, &target_backend_config_dir)
        .context("publish data sources to backend config")?;

    let target_database_pkg_dir = files::get_database_pkg_dir(workspace);
    schema::publish_data_sources(workspace, &target_database_pkg_dir)
        .context("publish data sources to database package")?;

    for schema_ir in &generator_ir.schemas {
        if schema_ir.is_system_schema != system_run {
            continue;
        }
        let schema_name = schema_ir.name.as_str();
        // The system schema is loaded in-memory by the backend from its YAML config;
        // it is NOT persisted to a database, so skip the database publish for it.
        let is_system_schema = schema_ir.is_system_schema;
        if !is_system_schema {
            schema::publish_database(workspace, schema_name, &data_sources)
                .with_context(|| format!("publish database artifacts for `{schema_name}`"))?;
        }
        schema::publish_schema(workspace, schema_name, is_system_schema)
            .with_context(|| format!("publish schema config for `{schema_name}`"))?;
        schema::publish_entity_types(workspace, schema_name)
            .with_context(|| format!("publish entity types for `{schema_name}`"))?;
        schema::publish_rego(workspace, schema_name)
            .with_context(|| format!("publish RBAC policies for `{schema_name}`"))?;
        schema::publish_tests(workspace, schema_name)
            .with_context(|| format!("publish API tests for `{schema_name}`"))?;
    }
    Ok(())
}

fn schema_dir_names(workspace: &AppWorkspace) -> Result<Vec<String>> {
    files::get_schema_dir_names(workspace).map_err(|err| anyhow::anyhow!("{err}"))
}
