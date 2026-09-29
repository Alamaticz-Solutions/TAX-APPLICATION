extern crate tera;

use anyhow::{Context as AnyhowContext, Result};
use serde_json::Value;
use std::fs::{self};
use std::path::{Path, PathBuf};
use tera::Context;

use super::console;
use super::files;
use crate::app_workspace::AppWorkspace;
use crate::bootstrap_types::data_source::{DataSource, Schema};
use crate::bootstrap_types::entity_type::EntityType;

pub fn create() -> Result<Context> {
    Ok(Context::new())
}

pub fn create_for_schema(schema: &Schema) -> Result<Context> {
    let mut context = Context::new();
    context.insert("schema", schema);
    Ok(context)
}

pub fn load_array_yaml(context: &mut Context, src_file: &PathBuf, ctx_name: &str) -> Result<()> {
    console::verbose_path("load yaml array", src_file);
    let default = serde_json::json!([]);
    let res: Value = read_yaml(src_file, default)?;
    context.insert(ctx_name, &res);
    Ok(())
}

pub fn load_audit_entity_type(context: &mut Context, workspace: &AppWorkspace) -> Result<()> {
    // println!("    context::load_item_yaml: {:?}", src_file);
    let src_file = files::get_audit_entity_type_file(workspace);
    let default = serde_json::Value::Null;
    let res: Value = read_yaml(&src_file, default)?;
    let item = res
        .as_object()
        .with_context(|| format!("{} must be a YAML object", src_file.display()))?;
    context.insert("auditEntityType", &item);
    Ok(())
}

pub fn load_audit_property(context: &mut Context, workspace: &AppWorkspace) -> Result<()> {
    // println!("    context::load_item_yaml: {:?}", src_file);
    let src_file = files::get_audit_property_file(workspace);
    let default = serde_json::Value::Null;
    let res: Value = read_yaml(&src_file, default)?;
    let item = res
        .as_object()
        .with_context(|| format!("{} must be a YAML object", src_file.display()))?;
    context.insert("auditProperty", &item);
    Ok(())
}

pub fn read_data_sources(src_file: &PathBuf) -> Result<Vec<DataSource>> {
    console::verbose_path("read data sources", src_file);
    let f = std::fs::File::open(src_file)
        .with_context(|| format!("could not open data sources YAML {}", src_file.display()))?;
    let res = serde_yaml::from_reader(f)
        .with_context(|| format!("could not parse data sources YAML {}", src_file.display()))?;
    Ok(res)
}

pub fn read_schema(src_file: &PathBuf) -> Result<Schema> {
    console::verbose_path("read schema", src_file);
    let f = std::fs::File::open(src_file)
        .with_context(|| format!("could not open schema YAML {}", src_file.display()))?;
    let res = serde_yaml::from_reader(f)
        .with_context(|| format!("could not parse schema YAML {}", src_file.display()))?;
    Ok(res)
}

pub fn read_types(src_file: &PathBuf) -> Result<Vec<EntityType>> {
    console::verbose_path("read entity types", src_file);
    let f = std::fs::File::open(src_file)
        .with_context(|| format!("could not open entity types YAML {}", src_file.display()))?;
    let res = serde_yaml::from_reader(f)
        .with_context(|| format!("could not parse entity types YAML {}", src_file.display()))?;
    Ok(res)
}

pub fn read_yaml(src_file: &PathBuf, default: Value) -> Result<Value> {
    // println!("    context::read_yaml: {:?}", src_file);
    if !Path::new(src_file).exists() {
        Ok(default)
    } else {
        let f = std::fs::File::open(src_file)
            .with_context(|| format!("could not open YAML {}", src_file.display()))?;
        let mut res: Value = serde_yaml::from_reader(f)
            .with_context(|| format!("could not parse YAML {}", src_file.display()))?;
        if res.is_null() {
            res = default;
        }
        // println!("    context::read_yaml: {:?}", res);
        Ok(res)
    }
}

pub fn load_dir(context: &mut Context, src_dir: &PathBuf, ctx_name: &str) -> Result<()> {
    // println!("context::load_dir:: src_dir = {:?}, ctx_name = {:?}", src_dir, ctx_name);
    let res = get_dir_value(src_dir)?;
    // println!("context::load_dir:: res = {:?}", res);
    context.insert(ctx_name, &res);
    Ok(())
}

//
// NOTE: ONLY CONSUMES FIRST LEVEL FILES
//
fn get_dir_value(dir_path: &PathBuf) -> Result<serde_yaml::Value> {
    // Collect, sort by filename, then concatenate. Deterministic order matters
    // for seeds: a 01_/02_/... numeric prefix encodes FK-dependency layers, and
    // fs::read_dir on its own returns entries in unspecified order.
    let mut paths: Vec<PathBuf> = fs::read_dir(dir_path)?
        .filter_map(|e| e.ok().map(|d| d.path()))
        .filter(|p| {
            p.extension()
                .and_then(|e| e.to_str())
                .map_or(false, |e| e.eq_ignore_ascii_case("yaml"))
                && p.file_stem()
                    .and_then(|s| s.to_str())
                    .map_or(false, |s| s.ne("_res"))
        })
        .collect();
    paths.sort();

    let mut dir_items = serde_yaml::Sequence::new();
    for path in paths {
        let f = std::fs::File::open(&path)
            .with_context(|| format!("could not open YAML {}", path.display()))?;
        let mut file_items: serde_yaml::Sequence = serde_yaml::from_reader(f)
            .with_context(|| format!("{} must be a YAML array", path.display()))?;
        dir_items.append(&mut file_items);
    }
    Ok(dir_items.into())
}
