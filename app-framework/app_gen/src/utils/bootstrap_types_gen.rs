use std::path::PathBuf;

use anyhow::Result;

use crate::app_workspace::AppWorkspace;
use crate::utils::{
    artifacts::{self, Artifact, OverwriteMode},
    console, context, engine, files,
};

#[allow(dead_code)]
pub fn run(workspace: &AppWorkspace) -> Result<()> {
    console::step("generate app_gen bootstrap types");

    let templates_dir = workspace.templates_root.join("app_gen/bootstrap_types");
    let context = context::create()?;
    let tera = engine::create(workspace, &templates_dir)?;
    let target_dir = files::get_app_gen_src_dir(workspace).join("bootstrap_types");

    emit_bootstrap_type(&tera, &context, &target_dir, "entity_type")?;
    emit_bootstrap_type(&tera, &context, &target_dir, "data_source")
}

fn emit_bootstrap_type(
    tera: &tera::Tera,
    context: &tera::Context,
    target_dir: &PathBuf,
    template_name: &str,
) -> Result<()> {
    let target_file = target_dir.join(format!("{template_name}.rs"));
    let output = tera.render(template_name, context)?;
    artifacts::emit(&Artifact::generated_text(
        target_file,
        output,
        OverwriteMode::Always,
    ))
}
