use serde_json::Value;
use std::path::PathBuf;

use crate::app_workspace::AppWorkspace;
use crate::utils::{
    artifacts::{self, Artifact, OverwriteMode},
    console, context, engine, files,
};
use anyhow::Result;

pub fn run(
    workspace: &AppWorkspace,
    entry_path: &PathBuf,
    schema_name: String,
    test_group_name: String,
) -> Result<()> {
    let tests_yaml: Value = context::read_yaml(entry_path, serde_json::json!([]))?;
    // println!("      tests_yaml: {:?}", tests_yaml);

    let context = &mut context::create()?;
    context.insert("schemaName", &schema_name);
    context.insert("testGroupName", &test_group_name);
    context.insert("tests", &tests_yaml);

    let templates_dir = workspace.templates_root.join("tests");
    console::verbose_path("tests templates", &templates_dir);
    let tera = engine::create(workspace, &templates_dir)?;

    let target_test_file_path =
        &files::get_api_tests_src_rs_path(workspace, &schema_name, &test_group_name);
    // println!("      tera.render: {:?}", target_test_file_path);

    let output = tera.render("tests", &context)?;
    artifacts::emit(&Artifact::generated_text(
        target_test_file_path.clone(),
        output,
        OverwriteMode::Always,
    ))?;

    Ok(())
}

pub fn gen_schema_mod_rs(
    workspace: &AppWorkspace,
    schema_name: String,
    group_names: Vec<String>,
) -> Result<()> {
    console::step(format!("API test module: {schema_name}"));

    let context = &mut context::create()?;
    context.insert("schemaName", &schema_name);
    context.insert("groupNames", &group_names);

    let templates_dir = workspace.templates_root.join("tests").join("mod");
    let target_dir = &files::get_api_tests_src_dir(workspace, &schema_name);
    let target_path = &target_dir.join("mod.rs");

    let tera = engine::create(workspace, &templates_dir)?;
    let output = tera.render("mod", &context)?;
    artifacts::emit(&Artifact::generated_text(
        target_path.clone(),
        output,
        OverwriteMode::Always,
    ))?;

    Ok(())
}
