extern crate tera;

use anyhow::{Context, Result};
use std::fs::{self};
use std::path::{Path, PathBuf};
use tera::Tera;

pub fn register_templates(tera: &mut Tera, path: &Path) -> Result<()> {
    if path.is_dir() {
        let mut entries = fs::read_dir(path)
            .with_context(|| format!("could not read {}", path.display()))?
            .map(|entry| {
                entry
                    .with_context(|| format!("could not read entry in {}", path.display()))
                    .map(|entry| entry.path())
            })
            .collect::<Result<Vec<PathBuf>>>()?;
        entries.sort();

        // First we have to ensure the child dirs are processed
        for entry_path in &entries {
            if entry_path.is_dir() {
                register_templates(tera, entry_path)?;
            }
        }
        // Then we handle the child files
        for entry_path in &entries {
            let is_j2 = entry_path
                .extension()
                .and_then(|extension| extension.to_str())
                .map(|extension| extension.eq_ignore_ascii_case("j2"))
                .unwrap_or(false);
            if entry_path.is_file() && is_j2 {
                register_template(tera, entry_path)?;
            }
        }
    }
    Ok(())
}

fn register_template(tera: &mut Tera, path: &Path) -> Result<()> {
    // println!("register_template: {:?}", path);
    let file_name = path
        .file_stem()
        .and_then(|value| value.to_str())
        .with_context(|| format!("template has invalid file name {}", path.display()))?;
    let dir_name = path
        .parent()
        .and_then(|parent| parent.file_stem())
        .and_then(|value| value.to_str())
        .with_context(|| format!("template has invalid parent {}", path.display()))?;
    let templ_name = if file_name.eq("_mod") {
        dir_name
    } else {
        file_name
    };
    tera.add_template_file(path, Some(templ_name))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn registers_macro_templates_in_stable_dependency_order() {
        let run_id = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time")
            .as_nanos();
        let root = env::temp_dir().join(format!("appfw-template-registry-test-{run_id}"));
        fs::create_dir_all(root.join("request")).expect("create request dir");
        fs::create_dir_all(root.join("assert")).expect("create assert dir");
        fs::write(
            root.join("request/_mod.j2"),
            r#"{%- import "assert" as assert -%}{% macro print() %}{{ assert::print() }}{% endmacro print %}"#,
        )
        .expect("write request template");
        fs::write(
            root.join("assert/_mod.j2"),
            r#"{% macro print() %}assert-ok{% endmacro print %}"#,
        )
        .expect("write assert template");
        fs::write(
            root.join("_mod.j2"),
            r#"{%- import "request" as request -%}{{ request::print() }}"#,
        )
        .expect("write root template");

        let mut tera = Tera::default();
        register_templates(&mut tera, &root).expect("register templates");
        let root_template_name = root
            .file_name()
            .and_then(|value| value.to_str())
            .expect("root template name");
        let rendered = tera
            .render(root_template_name, &tera::Context::new())
            .expect("render root template");
        assert_eq!(rendered, "assert-ok");

        fs::remove_dir_all(root).expect("remove temp");
    }
}
