use serde_json::Value;

use crate::app_workspace::AppWorkspace;
use crate::utils::{
    artifacts::{self, Artifact, OverwriteMode},
    console, context, engine, filters,
};
use anyhow::Result;

use crate::bootstrap_types::data_source::Schema;

pub fn run(
    workspace: &AppWorkspace,
    types_src_sub_path: &str,
    schema: &Schema,
    template_name: &str,
    ctx_name: &str,
    additional_ctx_name: Option<String>,
    additional_ctx_value: Option<Value>,
) -> Result<()> {
    let config_dir = &workspace.config_root;
    let templates_dir = workspace.templates_root.join("types").join(template_name);
    let types_src_dir = config_dir.join(types_src_sub_path).join(template_name);

    let mut tera = engine::create(workspace, &templates_dir)?;

    let fragments_dir = config_dir.join("_fragments");
    filters::register_fragment(&mut tera, &fragments_dir)?;

    // println!("      yaml_gen::run: {:?}", schema_yaml);
    let context = &mut context::create_for_schema(schema)?;
    // println!("      loading types from: {:?}", types_src_dir);
    context::load_dir(context, &types_src_dir, ctx_name)?;
    context::load_audit_entity_type(context, workspace)?;
    context::load_audit_property(context, workspace)?;

    if let (Some(var_name), Some(var_value)) = (additional_ctx_name, additional_ctx_value) {
        let var_name = var_name.replace("\"", "");
        // println!("      additional context -- name: {}; value: {:?}", &var_name, &var_value);
        context.insert(&var_name, &var_value);
    }

    // Generate the config yaml and write to the seed directory.
    console::step(format!("render YAML: {}.{template_name}", schema.name));
    let output = tera.render(template_name, &context)?;
    let target_file = types_src_dir.join("_res.yaml");
    artifacts::emit(&Artifact::generated_text(
        target_file,
        output,
        OverwriteMode::Always,
    ))?;

    Ok(())
}
