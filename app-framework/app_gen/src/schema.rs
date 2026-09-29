use std::vec;
use std::{fs, path::PathBuf};

use anyhow::{Context, Result};

use crate::app_workspace::AppWorkspace;
use crate::bootstrap_types::data_source::DataSource;
use crate::utils::constants::{ctx_name, tmp_name};
use crate::utils::database::{Mongo, MsSql, Postgres, Snowflake};
use crate::utils::{
    artifacts::{self, Artifact, OverwriteMode},
    console, context, files, rego_gen, test_gen, yaml_gen,
};

pub fn preprocess(workspace: &AppWorkspace, schema_dir_name: &str) -> Result<()> {
    console::item("schema", schema_dir_name);
    console::step("preprocess schema config");

    let schema_src_file = files::get_schema_file(workspace, schema_dir_name);
    let schema = context::read_schema(&schema_src_file)
        .with_context(|| format!("read schema config {}", schema_src_file.display()))?;

    // Create the full configuration yaml from seed files
    let s = format!("schemas/{}", schema_dir_name); // Use let binding
    let types_src_sub_path = s.as_str();

    // Pre-process schema enums. Most product schemas start without enum types,
    // but downstream generation still expects the normalized collection file.
    let enum_types_src_dir = workspace
        .config_root
        .join(types_src_sub_path)
        .join(tmp_name::GQL_ENUM_TYPES);
    if enum_types_src_dir.exists() {
        yaml_gen::run(
            &workspace,
            types_src_sub_path,
            &schema,
            tmp_name::GQL_ENUM_TYPES,
            ctx_name::GQL_ENUM_TYPES,
            None,
            None,
        )?;
    } else {
        console::step(format!(
            "render YAML: {}.{}",
            schema.name,
            tmp_name::GQL_ENUM_TYPES
        ));
        artifacts::emit(&Artifact::generated_text(
            enum_types_src_dir.join("_res.yaml"),
            "[]\n".to_string(),
            OverwriteMode::Always,
        ))?;
    }

    // Generate {schema}/entity_types/_res.yaml.
    yaml_gen::run(
        &workspace,
        types_src_sub_path,
        &schema,
        tmp_name::ENTITY_TYPES,
        ctx_name::ENTITY_TYPES,
        None,
        None,
    )?;

    let relationships_dir = files::get_relationships_dir(workspace, schema_dir_name);
    if relationships_dir.exists() {
        yaml_gen::run(
            &workspace,
            types_src_sub_path,
            &schema,
            tmp_name::RELATIONSHIPS,
            ctx_name::RELATIONSHIPS,
            None,
            None,
        )?;
    }

    Ok(())
}

pub fn publish_database(
    workspace: &AppWorkspace,
    schema_dir_name: &str,
    _data_sources: &Vec<DataSource>,
) -> Result<()> {
    console::step(format!("publish database package: {schema_dir_name}"));

    if schema_dir_name == ctx_name::SYS_SCHEMA_NAME {
        let target_schema_dir = files::get_database_sql_schema_dir(workspace, schema_dir_name);
        console::skip(
            &target_schema_dir,
            "system schema is not persisted to database",
        );
        return Ok(());
    }

    let schema_src_file = files::get_schema_file(workspace, schema_dir_name);
    let schema = context::read_schema(&schema_src_file)
        .with_context(|| format!("read schema config {}", schema_src_file.display()))?;
    let target_schema_dir = files::get_database_sql_schema_dir(workspace, &schema.name);

    // DDL + seeds are emitted for every dialect every run. Mongo has no DDL
    // (collections are implicit), so only pg/mssql emit tables files. This lets
    // a schema be re-pointed to a different data source without regenerating.
    Postgres::generate(workspace, schema_dir_name, &schema, &target_schema_dir)?;
    MsSql::generate(workspace, schema_dir_name, &schema, &target_schema_dir)?;
    Snowflake::generate(workspace, schema_dir_name, &schema, &target_schema_dir)?;
    Mongo::generate(workspace, schema_dir_name, &schema, &target_schema_dir)?;

    Ok(())
}

pub fn publish_data_sources(workspace: &AppWorkspace, target_dir: &PathBuf) -> Result<()> {
    console::step("publish data sources");

    let data_sources_src_file = files::get_data_sources_file(workspace);
    let target_data_sources_file = target_dir.join("data_sources.yaml");
    emit_copy(data_sources_src_file, target_data_sources_file)?;

    Ok(())
}

pub fn publish_schema(
    workspace: &AppWorkspace,
    schema_dir_name: &str,
    is_system_schema: bool,
) -> Result<()> {
    console::step(format!("publish schema config: {schema_dir_name}"));

    let schema_src_file = files::get_schema_file(workspace, schema_dir_name);

    // backend
    let target_backend_schema_dir =
        &files::get_backend_config_schema_dir(workspace, schema_dir_name);
    let target_backend_schema_file = target_backend_schema_dir.join("schema.yaml");
    emit_copy(schema_src_file.clone(), target_backend_schema_file)?;

    // The system schema is not persisted to any database, so skip the database-side copy.
    if !is_system_schema {
        let target_database_schema_dir =
            &files::get_database_sql_schema_dir(workspace, schema_dir_name);
        let target_database_schema_file = target_database_schema_dir.join("schema.yaml");
        emit_copy(schema_src_file.clone(), target_database_schema_file)?;
    }

    // In copied/in-place layouts the database crate still needs the generated
    // bootstrap type mirror. Split-root product apps consume the framework-owned
    // database runner and keep only the generated database package under
    // database/_pkg.
    if !workspace.uses_split_roots() {
        let data_source_src_file = files::get_app_gen_src_dir(workspace)
            .join("bootstrap_types")
            .join("data_source.rs");
        let target_database_src_file =
            &files::get_database_src_dir(workspace).join("data_source.rs");
        emit_copy(data_source_src_file, target_database_src_file.clone())?;
    }

    Ok(())
}

pub fn publish_entity_types(workspace: &AppWorkspace, schema_dir_name: &str) -> Result<()> {
    console::step(format!("publish entity types: {schema_dir_name}"));

    let target_schema_dir = &files::get_backend_config_schema_dir(workspace, schema_dir_name);
    let source_entity_types_file = &files::get_entity_types_file(workspace, schema_dir_name);
    let target_entity_types_file = target_schema_dir.join("entity_types.yaml");
    emit_copy(source_entity_types_file.clone(), target_entity_types_file)?;

    Ok(())
}

fn emit_copy(source: PathBuf, target: PathBuf) -> Result<()> {
    artifacts::emit(&Artifact::generated_copy_from(
        target,
        source,
        OverwriteMode::Always,
    ))
}

pub fn publish_rego(workspace: &AppWorkspace, schema_name: &str) -> Result<()> {
    console::step(format!("publish RBAC policies: {schema_name}"));

    let rbac_dir = workspace.schemas_root().join(schema_name).join("rbac");
    if !rbac_dir.exists() {
        console::skip(&rbac_dir, "RBAC directory not found");
        return Ok(());
    }
    for dir_entry in fs::read_dir(&rbac_dir)
        .with_context(|| format!("could not read RBAC directory {}", rbac_dir.display()))?
    {
        let entry_path = dir_entry
            .with_context(|| format!("could not read entry in {}", rbac_dir.display()))?
            .path();
        if !entry_path.is_dir() {
            let is_rego = entry_path
                .extension()
                .and_then(|extension| extension.to_str())
                .map(|extension| extension.eq_ignore_ascii_case("rego"))
                .unwrap_or(false);
            if is_rego {
                // Get the name of the rego file
                let entity_type_name = entry_path
                    .file_stem()
                    .and_then(|stem| stem.to_str())
                    .with_context(|| format!("invalid RBAC file name {}", entry_path.display()))?;

                let target_schema_dir =
                    &files::get_backend_config_schema_dir(workspace, schema_name);
                let target_rego_file_path =
                    target_schema_dir.join(format!("{entity_type_name}.rego"));

                rego_gen::run(
                    workspace,
                    &entry_path,
                    &target_rego_file_path,
                    schema_name.to_string(),
                    entity_type_name.to_string(),
                )
                .with_context(|| format!("render RBAC policy {}", entry_path.display()))?;
            }
        }
    }

    Ok(())
}

pub fn publish_tests(workspace: &AppWorkspace, schema_name: &str) -> Result<()> {
    console::step(format!("publish API tests: {schema_name}"));

    let tests_dir = workspace.schemas_root().join(schema_name).join("tests");
    console::verbose_path("tests dir", &tests_dir);
    if !tests_dir.exists() {
        console::skip(&tests_dir, "API tests directory not found");
        return Ok(());
    }

    let mut test_files: Vec<PathBuf> = fs::read_dir(&tests_dir)
        .with_context(|| format!("could not read tests directory {}", tests_dir.display()))?
        .map(|entry| {
            entry
                .with_context(|| format!("could not read entry in {}", tests_dir.display()))
                .map(|entry| entry.path())
        })
        .collect::<Result<Vec<_>>>()?
        .into_iter()
        .filter(|path| {
            path.extension()
                .and_then(|extension| extension.to_str())
                .map(|extension| extension.eq_ignore_ascii_case("yaml"))
                .unwrap_or(false)
                && path
                    .file_stem()
                    .and_then(|stem| stem.to_str())
                    .map(|stem| stem.ne("_res"))
                    .unwrap_or(false)
        })
        .collect();
    test_files.sort();

    let mut group_names: Vec<String> = vec![];
    for entry_path in test_files {
        let test_group_name = entry_path
            .file_stem()
            .and_then(|stem| stem.to_str())
            .with_context(|| format!("invalid test file name {}", entry_path.display()))?;

        test_gen::run(
            workspace,
            &entry_path,
            schema_name.to_string(),
            test_group_name.to_string(),
        )
        .with_context(|| format!("render API test {}", entry_path.display()))?;

        group_names.push(test_group_name.to_string());
    }

    if group_names.is_empty() {
        console::skip(&tests_dir, "no API test groups found");
        return Ok(());
    }

    test_gen::gen_schema_mod_rs(workspace, schema_name.to_string(), group_names)
        .with_context(|| format!("render API test module for `{schema_name}`"))?;

    Ok(())
}
