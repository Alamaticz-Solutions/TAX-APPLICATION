use std::fs;
use std::path::PathBuf;

use crate::app_workspace::AppWorkspace;
use crate::bootstrap_types::data_source::DataSourceType;

pub fn get_app_gen_src_dir(workspace: &AppWorkspace) -> PathBuf {
    workspace.generator_root.join("src")
}

pub fn get_audit_entity_type_file(workspace: &AppWorkspace) -> PathBuf {
    workspace.config_root.join("_facets/audit_entity_type.yaml")
}
pub fn get_audit_property_file(workspace: &AppWorkspace) -> PathBuf {
    workspace.config_root.join("_facets/audit_property.yaml")
}
pub fn get_soft_deleted_property_file(workspace: &AppWorkspace) -> PathBuf {
    workspace
        .config_root
        .join("_facets/soft_deleted_property.yaml")
}

/// Return the path to the version-property.yaml file in the app_gen project.
/// This file defines the facet that is used to mark a property as a version property.
pub fn get_version_property_file(workspace: &AppWorkspace) -> PathBuf {
    workspace.config_root.join("_facets/version_property.yaml")
}

// Paths to app_gen project (this project) locations
pub fn get_database_templates_dir(
    workspace: &AppWorkspace,
    data_source_type: DataSourceType,
) -> PathBuf {
    workspace
        .templates_root
        .join("database")
        .join(data_source_type.as_str())
}

pub fn get_backend_templates_dir(workspace: &AppWorkspace) -> PathBuf {
    workspace.templates_root.join("backend")
}

pub fn get_schema_dir(workspace: &AppWorkspace, schema_dir_name: &str) -> PathBuf {
    workspace.schemas_root().join(schema_dir_name)
}

pub fn get_schema_dir_names(
    workspace: &AppWorkspace,
) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let schemas_dir = workspace.schemas_root();
    let mut schema_dir_names: Vec<String> = Vec::new();

    for entry in fs::read_dir(schemas_dir)? {
        let entry = entry?;
        let path = entry.path();

        if path.is_dir() {
            if let Some(dir_name) = path.file_name() {
                if let Some(dir_name_str) = dir_name.to_str() {
                    // Skip hidden directories
                    if !dir_name_str.starts_with(".") {
                        schema_dir_names.push(dir_name_str.to_string());
                    }
                }
            }
        }
    }

    schema_dir_names.sort();
    Ok(schema_dir_names)
}

pub fn get_entity_types_file(workspace: &AppWorkspace, schema_dir_name: &str) -> PathBuf {
    get_schema_dir(workspace, schema_dir_name).join("entity_types/_res.yaml")
}

pub fn get_relationships_dir(workspace: &AppWorkspace, schema_dir_name: &str) -> PathBuf {
    get_schema_dir(workspace, schema_dir_name).join("relationships")
}

pub fn get_relationships_file(workspace: &AppWorkspace, schema_dir_name: &str) -> PathBuf {
    get_relationships_dir(workspace, schema_dir_name).join("_res.yaml")
}

pub fn get_seeds_dir(workspace: &AppWorkspace, schema_dir_name: &str) -> PathBuf {
    get_schema_dir(workspace, schema_dir_name).join("seeds")
}
pub fn get_gql_enum_types_file(workspace: &AppWorkspace, schema_dir_name: &str) -> PathBuf {
    get_schema_dir(workspace, schema_dir_name).join("gql_enum_types/_res.yaml")
}

pub fn get_data_sources_file(workspace: &AppWorkspace) -> PathBuf {
    workspace.config_root.join("data_sources").join("_res.yaml")
}

pub fn get_sync_descriptors_dir(workspace: &AppWorkspace) -> PathBuf {
    workspace.config_root.join("sync")
}

pub fn get_schema_file(workspace: &AppWorkspace, schema_dir_name: &str) -> PathBuf {
    get_schema_dir(workspace, schema_dir_name).join("_res.yaml")
}

// Paths to database project locations
pub fn get_database_pkg_dir(workspace: &AppWorkspace) -> PathBuf {
    workspace.app_root.join("database/_pkg")
}
pub fn get_database_pkg_schemas_dir(workspace: &AppWorkspace) -> PathBuf {
    get_database_pkg_dir(workspace).join("schemas")
}
pub fn get_database_sql_schema_dir(workspace: &AppWorkspace, schema_name: &str) -> PathBuf {
    get_database_pkg_schemas_dir(workspace).join(schema_name)
}
pub fn get_database_src_dir(workspace: &AppWorkspace) -> PathBuf {
    workspace.app_root.join("database/src")
}

// Paths to backend project locations
pub fn get_backend_config_dir(workspace: &AppWorkspace) -> PathBuf {
    workspace.app_root.join("backend/config/generated")
}
pub fn get_backend_sync_workers_config_file(workspace: &AppWorkspace) -> PathBuf {
    get_backend_config_dir(workspace).join("sync_workers.yaml")
}
pub fn get_backend_config_schema_dir(workspace: &AppWorkspace, schema_name: &str) -> PathBuf {
    get_backend_config_dir(workspace)
        .join("schemas")
        .join(schema_name)
}
pub fn get_backend_src_rs_path(
    workspace: &AppWorkspace,
    dir_name: &str,
    rs_file_name: &str,
) -> PathBuf {
    workspace
        .app_root
        .join("backend/src")
        .join(dir_name)
        .join(format!("{rs_file_name}.rs"))
}
pub fn get_backend_src_handlers_schema_dir_path(
    workspace: &AppWorkspace,
    schema_name: &str,
) -> PathBuf {
    workspace
        .app_root
        .join("backend/src/handlers")
        .join(schema_name)
}

// Paths to api_tests project locations
pub fn get_api_tests_src_dir(workspace: &AppWorkspace, schema_name: &str) -> PathBuf {
    workspace
        .app_root
        .join("api_tests/src/schemas")
        .join(schema_name)
}
pub fn get_api_tests_src_rs_path(
    workspace: &AppWorkspace,
    schema_name: &str,
    test_group_name: &str,
) -> PathBuf {
    get_api_tests_src_dir(workspace, schema_name).join(format!("{test_group_name}.rs"))
}
