use std::{
    env,
    path::{Component, Path, PathBuf},
};

use anyhow::{bail, Context, Result};
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct AppWorkspace {
    pub app_root: PathBuf,
    pub framework_root: PathBuf,
    pub generator_root: PathBuf,
    pub config_root: PathBuf,
    pub templates_root: PathBuf,
    pub report_root: PathBuf,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct AppGenOptions {
    pub validate_only: bool,
}

impl AppWorkspace {
    pub fn from_args(args: impl IntoIterator<Item = String>) -> Result<(Self, AppGenOptions)> {
        let cwd = env::current_dir().context("could not read current directory")?;
        let mut app_root = env_path("APPFW_APP_ROOT");
        let mut framework_root = env_path("APPFW_FRAMEWORK_ROOT");
        let mut generator_root = env_path("APPFW_GENERATOR_ROOT");
        let mut config_root = env_path("APPFW_CONFIG_ROOT");
        let mut templates_root = env_path("APPFW_TEMPLATES_ROOT");
        let mut report_root = env_path("APPFW_REPORT_ROOT");
        let mut options = AppGenOptions::default();

        let args = args.into_iter().collect::<Vec<_>>();
        let mut idx = 0;
        while idx < args.len() {
            match args[idx].as_str() {
                "--validate-only" => {
                    options.validate_only = true;
                    idx += 1;
                }
                "--app-root" => {
                    app_root = Some(required_path_arg(&args, idx, "--app-root")?);
                    idx += 2;
                }
                "--framework-root" => {
                    framework_root = Some(required_path_arg(&args, idx, "--framework-root")?);
                    idx += 2;
                }
                "--generator-root" => {
                    generator_root = Some(required_path_arg(&args, idx, "--generator-root")?);
                    idx += 2;
                }
                "--config-root" => {
                    config_root = Some(required_path_arg(&args, idx, "--config-root")?);
                    idx += 2;
                }
                "--templates-root" => {
                    templates_root = Some(required_path_arg(&args, idx, "--templates-root")?);
                    idx += 2;
                }
                "--report-root" => {
                    report_root = Some(required_path_arg(&args, idx, "--report-root")?);
                    idx += 2;
                }
                value => bail!("unknown app_gen option `{value}`"),
            }
        }

        let app_root = app_root.unwrap_or_else(|| discover_app_root(&cwd));
        let framework_root =
            framework_root.unwrap_or_else(|| discover_framework_root(&cwd, &app_root));
        let generator_root = generator_root.unwrap_or_else(|| framework_root.join("app_gen"));
        let config_root = config_root.unwrap_or_else(|| app_root.join(".appfw/model"));
        let templates_root = templates_root.unwrap_or_else(|| generator_root.join("_templates"));
        let report_root = report_root.unwrap_or_else(|| app_root.join(".appfw/target/appfw"));

        let workspace = Self::from_roots(
            app_root,
            framework_root,
            generator_root,
            config_root,
            templates_root,
            report_root,
            &cwd,
        )?;
        Ok((workspace, options))
    }

    pub(crate) fn from_roots(
        app_root: PathBuf,
        framework_root: PathBuf,
        generator_root: PathBuf,
        config_root: PathBuf,
        templates_root: PathBuf,
        report_root: PathBuf,
        cwd: &Path,
    ) -> Result<Self> {
        let workspace = Self {
            app_root: absolutize(app_root, cwd),
            framework_root: absolutize(framework_root, cwd),
            generator_root: absolutize(generator_root, cwd),
            // Preserve configured output-root spellings until output-safety
            // preflight. Canonicalizing here would erase a symlink leaf or
            // ancestor before `artifacts::begin_run` can reject it.
            config_root: absolutize_lexically(config_root, cwd),
            templates_root: absolutize(templates_root, cwd),
            report_root: absolutize_lexically(report_root, cwd),
        };
        workspace.validate()?;
        Ok(workspace)
    }

    pub fn schemas_root(&self) -> PathBuf {
        self.config_root.join("schemas")
    }

    pub fn target_file(&self, file_name: &str) -> PathBuf {
        self.report_root.join(file_name)
    }

    pub fn uses_split_roots(&self) -> bool {
        self.app_root != self.framework_root
    }

    fn validate(&self) -> Result<()> {
        ensure_dir(&self.app_root, "app root")?;
        ensure_dir(&self.framework_root, "framework root")?;
        ensure_dir(&self.generator_root, "generator root")?;
        ensure_dir(&self.config_root, "config root")?;
        ensure_dir(&self.templates_root, "templates root")?;
        ensure_dir(&self.generator_root.join("src"), "generator source root")?;
        ensure_dir(&self.config_root.join("schemas"), "schemas config root")?;
        Ok(())
    }
}

fn env_path(name: &str) -> Option<PathBuf> {
    env::var_os(name)
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
}

fn required_path_arg(args: &[String], idx: usize, flag: &str) -> Result<PathBuf> {
    args.get(idx + 1)
        .map(PathBuf::from)
        .with_context(|| format!("{flag} requires a path value"))
}

fn discover_app_root(cwd: &Path) -> PathBuf {
    if let Some(framework_root) = framework_checkout_root(cwd) {
        return framework_root.join("examples/products/crm");
    }
    if cwd.join(".appfw/manifest.yaml").is_file() || cwd.join(".appfw/model").is_dir() {
        return cwd.to_path_buf();
    }
    if cwd.file_name().and_then(|name| name.to_str()) == Some("model")
        && cwd
            .parent()
            .and_then(|parent| parent.file_name())
            .and_then(|name| name.to_str())
            == Some(".appfw")
    {
        return cwd
            .parent()
            .and_then(|parent| parent.parent())
            .unwrap_or(cwd)
            .to_path_buf();
    }
    cwd.to_path_buf()
}

fn discover_framework_root(cwd: &Path, app_root: &Path) -> PathBuf {
    if let Some(framework_root) = framework_checkout_root(cwd) {
        return framework_root;
    }
    if cwd.file_name().and_then(|name| name.to_str()) == Some("app_gen")
        && cwd.join("_templates").is_dir()
    {
        return cwd.parent().unwrap_or(cwd).to_path_buf();
    }
    if app_root.join("app_gen/_templates").is_dir() {
        return app_root.to_path_buf();
    }
    if let Some(parent) = app_root.parent() {
        let adjacent = parent.join("app-framework");
        if adjacent.join("app_gen/_templates").is_dir() {
            return adjacent;
        }
    }
    if let Some(parent) = cwd.parent() {
        let adjacent = parent.join("app-framework");
        if adjacent.join("app_gen/_templates").is_dir() {
            return adjacent;
        }
    }
    cwd.to_path_buf()
}

fn framework_checkout_root(cwd: &Path) -> Option<PathBuf> {
    let candidate = if cwd.file_name().and_then(|name| name.to_str()) == Some("app_gen") {
        cwd.parent().unwrap_or(cwd)
    } else {
        cwd
    };
    if candidate.join("app_gen/_templates").is_dir()
        && candidate
            .join("examples/products/crm/.appfw/manifest.yaml")
            .is_file()
    {
        Some(candidate.to_path_buf())
    } else {
        None
    }
}

fn ensure_dir(path: &Path, label: &str) -> Result<()> {
    if path.is_dir() {
        Ok(())
    } else {
        bail!(
            "{label} does not exist or is not a directory: {}",
            path.display()
        )
    }
}

fn absolutize(path: PathBuf, cwd: &Path) -> PathBuf {
    let joined = absolutize_lexically(path, cwd);
    if let Ok(canonical) = joined.canonicalize() {
        canonical
    } else {
        joined
    }
}

fn absolutize_lexically(path: PathBuf, cwd: &Path) -> PathBuf {
    let joined = if path.is_absolute() {
        path
    } else {
        cwd.join(path)
    };
    normalize_components(&joined)
}

fn normalize_components(path: &Path) -> PathBuf {
    let mut normalized = PathBuf::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                normalized.pop();
            }
            _ => normalized.push(component.as_os_str()),
        }
    }
    normalized
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{fs, time::SystemTime};

    #[test]
    fn discovers_adjacent_framework_root_for_product_checkout() {
        let base = temp_root("adjacent-framework");
        let product = base.join("od-equity-report");
        let framework = base.join("app-framework");
        fs::create_dir_all(product.join(".appfw")).expect("create product manifest dir");
        fs::write(product.join(".appfw/manifest.yaml"), "app:\n  name: od\n")
            .expect("write manifest");
        fs::create_dir_all(product.join(".appfw/model/schemas")).expect("create config root");
        fs::create_dir_all(framework.join("app_gen/_templates")).expect("create templates root");

        assert_eq!(discover_app_root(&product), product);
        assert_eq!(discover_framework_root(&product, &product), framework);

        let _ = fs::remove_dir_all(base);
    }

    #[test]
    fn framework_checkout_defaults_to_crm_sample_app_root() {
        let base = temp_root("framework-default-product-root");
        fs::create_dir_all(base.join("app_gen/_templates")).expect("create templates root");
        fs::create_dir_all(base.join("examples/products/crm/.appfw"))
            .expect("create crm manifest dir");
        fs::write(
            base.join("examples/products/crm/.appfw/manifest.yaml"),
            "app:\n  name: crm\n",
        )
        .expect("write crm manifest");

        assert_eq!(discover_app_root(&base), base.join("examples/products/crm"));
        assert_eq!(
            discover_app_root(&base.join("app_gen")),
            base.join("examples/products/crm")
        );
        assert_eq!(
            discover_framework_root(&base, &base.join("examples/products/crm")),
            base
        );

        let _ = fs::remove_dir_all(base);
    }

    fn temp_root(label: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .expect("system time")
            .as_nanos();
        env::temp_dir().join(format!(
            "appfw-workspace-{label}-{}-{nanos}",
            std::process::id()
        ))
    }
}
