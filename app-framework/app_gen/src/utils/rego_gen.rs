use std::{fs, path::PathBuf};

use crate::app_workspace::AppWorkspace;
use crate::utils::{
    artifacts::{self, Artifact, OverwriteMode},
    context, engine,
};
use anyhow::Result;

pub fn run(
    workspace: &AppWorkspace,
    source_rego_file_path: &PathBuf,
    target_rego_file_path: &PathBuf,
    schema_name: String,
    entity_type_name: String,
) -> Result<()> {
    let templates_dir = workspace.templates_root.join("types").join("rbac");

    let tera = engine::create(workspace, &templates_dir)?;

    // Read the specific rules
    let entity_type_rego = fs::read_to_string(source_rego_file_path)?;

    let context = &mut context::create()?;
    context.insert("pds_tenant_id", "180000");
    context.insert("schema_name", &schema_name);
    context.insert("entity_type_name", &entity_type_name);
    context.insert("entity_type_rego", &entity_type_rego);

    // Generate the config yaml and write to the seed directory.
    let output = tera.render("rbac", &context)?;
    artifacts::emit(&Artifact::generated_text(
        target_rego_file_path.clone(),
        output,
        OverwriteMode::Always,
    ))?;

    Ok(())
}
