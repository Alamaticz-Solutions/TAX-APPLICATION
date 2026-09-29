use std::path::PathBuf;

use anyhow::Result;
use tera::Tera;

use crate::app_workspace::AppWorkspace;

use super::{filters, templates};

fn register_extra_tests(tera: &mut Tera) {
    tera.register_tester("bool", |value: Option<&tera::Value>, _: &[tera::Value]| {
        Ok(value.map_or(false, |v| v.is_boolean()))
    });
}

pub fn create(workspace: &AppWorkspace, templates_dir: &PathBuf) -> Result<Tera> {
    let tera = &mut Tera::default();
    filters::register_inflectors(tera)?;
    filters::register_hash_string(tera)?;
    filters::register_uuid(tera)?;
    filters::register_facets(tera, workspace)?;
    filters::register_seed_literals(tera)?;
    filters::register_backend_type_plans(tera)?;
    register_extra_tests(tera);
    templates::register_templates(tera, templates_dir.as_path())?;
    Ok(tera.to_owned())
}
