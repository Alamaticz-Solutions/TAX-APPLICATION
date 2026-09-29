use std::{collections::HashMap, fs, io::Read, path::Path};

use anyhow::{bail, Context, Result};

use super::config::{ApiTestConfig, AuthMode};

#[derive(Clone, Debug)]
pub struct AuthTokenProvider {
    mode: AuthMode,
    tokens: HashMap<String, String>,
}

impl AuthTokenProvider {
    pub fn new(config: &ApiTestConfig) -> Result<Self> {
        let tokens = match config.auth_mode {
            AuthMode::Bypass | AuthMode::LocalDev => HashMap::new(),
            AuthMode::TokenFiles => load_tokens(&config.token_dir)?,
        };

        Ok(Self {
            mode: config.auth_mode.clone(),
            tokens,
        })
    }

    pub fn authorization_header(&self, token_name: &str) -> Result<Option<String>> {
        match self.mode {
            AuthMode::Bypass => Ok(None),
            AuthMode::LocalDev => local_dev_authorization_header(token_name).map(Some),
            AuthMode::TokenFiles => self
                .tokens
                .get(token_name)
                .cloned()
                .filter(|token| !token.is_empty())
                .map(Some)
                .with_context(|| format!("auth token `{token_name}` was not found")),
        }
    }
}

fn local_dev_authorization_header(token_name: &str) -> Result<String> {
    let (user, tenant, roles): (&str, &str, &[&str]) = match token_name {
        "pdsh_admin" => ("pdsh_admin", "local", &["admin"]),
        "other_tenant_admin" => ("other_tenant_admin", "tenant-2", &["admin"]),
        "cc_tenant_user" => ("cc_tenant_user", "tenant-1", &["account_ca_reader"]),
        "tenant_one_crm_ops" => ("tenant_one_crm_ops", "tenant-1", &["crm_ops"]),
        "west_sales_rep" => ("west_sales_rep", "tenant-1", &["sales_rep"]),
        "pdsh_office_user" => ("pdsh_office_user", "tenant-2", &["office_user"]),
        other => bail!("no local dev auth mapping exists for token `{other}`"),
    };

    Ok(format!(
        "Bearer appfw-local:user={user};tenant={tenant};roles={}",
        roles.join(",")
    ))
}

fn load_tokens(dir_path: &Path) -> Result<HashMap<String, String>> {
    if !dir_path.exists() {
        bail!("token directory does not exist: {}", dir_path.display());
    }

    let mut tokens = HashMap::new();
    for entry in fs::read_dir(dir_path)
        .with_context(|| format!("could not read token directory {}", dir_path.display()))?
    {
        let path = entry?.path();
        if !is_token_file(&path) {
            continue;
        }

        let token_name = path
            .file_stem()
            .and_then(|name| name.to_str())
            .with_context(|| format!("token file has invalid name: {}", path.display()))?;
        let mut contents = String::new();
        fs::File::open(&path)
            .with_context(|| format!("could not open token file {}", path.display()))?
            .read_to_string(&mut contents)
            .with_context(|| format!("could not read token file {}", path.display()))?;

        tokens.insert(token_name.to_string(), contents.trim().to_string());
    }

    Ok(tokens)
}

fn is_token_file(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .map(|extension| extension.eq_ignore_ascii_case("txt"))
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::local_dev_authorization_header;

    #[test]
    fn tenant_one_crm_ops_is_distinct_from_denied_actor() {
        let reader = local_dev_authorization_header("tenant_one_crm_ops")
            .expect("tenant-one audit reader mapping");
        let actor = local_dev_authorization_header("cc_tenant_user").expect("denied actor mapping");

        assert!(reader.contains("user=tenant_one_crm_ops;tenant=tenant-1;roles=crm_ops"));
        assert!(actor.contains("user=cc_tenant_user;tenant=tenant-1;roles=account_ca_reader"));
        assert_ne!(reader, actor);
    }
}
