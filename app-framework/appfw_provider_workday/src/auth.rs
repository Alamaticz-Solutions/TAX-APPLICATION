use crate::{soap::WORKDAY_WWS_VERSION, WorkdayProviderError};

pub const WORKDAY_HOST_ENV: &str = "WORKDAY_HOST";
pub const WORKDAY_TENANT_ENV: &str = "WORKDAY_TENANT";
pub const WORKDAY_ISU_USERNAME_ENV: &str = "WORKDAY_ISU_USERNAME";
pub const WORKDAY_ISU_SECRET_ENV: &str = "WORKDAY_ISU_SECRET";

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WorkdayTenantBinding {
    pub host: String,
    pub tenant: String,
}

impl WorkdayTenantBinding {
    pub fn new(
        host: impl Into<String>,
        tenant: impl Into<String>,
    ) -> Result<Self, WorkdayProviderError> {
        let host = host.into();
        let tenant = tenant.into();

        validate_workday_host(&host)?;
        validate_workday_tenant(&tenant)?;

        Ok(Self { host, tenant })
    }

    pub fn origin(&self) -> String {
        format!("https://{}", self.host)
    }

    pub fn human_resources_wws_path(&self) -> String {
        format!(
            "/ccx/service/{}/Human_Resources/{}",
            self.tenant, WORKDAY_WWS_VERSION
        )
    }
}

fn validate_workday_host(value: &str) -> Result<(), WorkdayProviderError> {
    if value.is_empty() {
        return Err(invalid_parameter(
            WORKDAY_HOST_ENV,
            "must be a non-empty HTTPS host",
        ));
    }

    if value
        .chars()
        .any(|character| character.is_control() || character.is_whitespace())
    {
        return Err(invalid_parameter(
            WORKDAY_HOST_ENV,
            "must be a single-line HTTPS host without whitespace",
        ));
    }

    if value.contains("://") {
        return Err(invalid_parameter(
            WORKDAY_HOST_ENV,
            "must be a host, not a URL",
        ));
    }

    if value.contains('/') || value.contains('\\') || value.contains('?') || value.contains('#') {
        return Err(invalid_parameter(
            WORKDAY_HOST_ENV,
            "must not include path, query, or fragment",
        ));
    }

    if value.contains('@') || value.contains(':') {
        return Err(invalid_parameter(
            WORKDAY_HOST_ENV,
            "must not include userinfo or port",
        ));
    }

    let labels = value.split('.').collect::<Vec<_>>();
    if labels.len() < 2 || labels.iter().any(|label| label.is_empty()) {
        return Err(invalid_parameter(
            WORKDAY_HOST_ENV,
            "must be a DNS host containing a dot",
        ));
    }

    if labels.iter().any(|label| !is_dns_label(label)) {
        return Err(invalid_parameter(
            WORKDAY_HOST_ENV,
            "must be a DNS host containing only ASCII letters, digits, hyphen, and dots",
        ));
    }

    Ok(())
}

fn is_dns_label(value: &str) -> bool {
    value
        .bytes()
        .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
        && !value.starts_with('-')
        && !value.ends_with('-')
}

fn validate_workday_tenant(value: &str) -> Result<(), WorkdayProviderError> {
    validate_workday_tenant_segment(WORKDAY_TENANT_ENV, value)
}

pub(crate) fn validate_workday_tenant_segment(
    name: &'static str,
    value: &str,
) -> Result<(), WorkdayProviderError> {
    if value.is_empty() {
        return Err(invalid_parameter(
            name,
            "must be a non-empty tenant path segment",
        ));
    }

    if value
        .chars()
        .any(|character| character.is_control() || character.is_whitespace())
    {
        return Err(invalid_parameter(
            name,
            "must be a single tenant path segment without whitespace",
        ));
    }

    if value.contains('/') || value.contains('\\') {
        return Err(invalid_parameter(
            name,
            "must be one tenant path segment without slash or backslash",
        ));
    }

    if value.contains('?') || value.contains('#') || value.contains('@') || value.contains(':') {
        return Err(invalid_parameter(
            name,
            "must not contain query, fragment, userinfo, or port punctuation",
        ));
    }

    if value.contains("..") {
        return Err(invalid_parameter(
            name,
            "must not contain parent-directory traversal",
        ));
    }

    if !value
        .bytes()
        .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_' || byte == b'-')
    {
        return Err(invalid_parameter(
            name,
            "must contain only ASCII letters, digits, underscore, and hyphen",
        ));
    }

    Ok(())
}

fn invalid_parameter(name: &'static str, reason: &'static str) -> WorkdayProviderError {
    WorkdayProviderError::InvalidParameter { name, reason }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tenant_binding_accepts_host_and_tenant_segment() {
        let binding = WorkdayTenantBinding::new("wd2-impl-services1.workday.com", "acme_01-prod")
            .expect("tenant binding");

        assert_eq!(binding.host, "wd2-impl-services1.workday.com");
        assert_eq!(binding.tenant, "acme_01-prod");
        assert_eq!(binding.origin(), "https://wd2-impl-services1.workday.com");
        assert_eq!(
            binding.human_resources_wws_path(),
            "/ccx/service/acme_01-prod/Human_Resources/v46.1"
        );
    }

    #[test]
    fn tenant_binding_rejects_url_and_path_shapes() {
        assert!(WorkdayTenantBinding::new("https://wd2.workday.com", "acme").is_err());
        assert!(WorkdayTenantBinding::new("wd2.workday.com/ccx", "acme").is_err());
        assert!(WorkdayTenantBinding::new("wd2.workday.com", "https://acme").is_err());
        assert!(WorkdayTenantBinding::new("wd2.workday.com", "acme/hr").is_err());
        assert!(WorkdayTenantBinding::new("wd2.workday.com", "acme\\hr").is_err());
    }

    #[test]
    fn tenant_binding_rejects_query_userinfo_and_port_shapes() {
        assert!(WorkdayTenantBinding::new("wd2.workday.com?tenant=evil", "acme").is_err());
        assert!(WorkdayTenantBinding::new("user@wd2.workday.com", "acme").is_err());
        assert!(WorkdayTenantBinding::new("wd2.workday.com:443", "acme").is_err());
        assert!(WorkdayTenantBinding::new("wd2.workday.com", "acme?env=evil").is_err());
        assert!(WorkdayTenantBinding::new("wd2.workday.com", "user@acme").is_err());
        assert!(WorkdayTenantBinding::new("wd2.workday.com", "acme:443").is_err());
        assert!(WorkdayTenantBinding::new("wd2.workday.com", "acme..prod").is_err());
    }

    #[test]
    fn tenant_binding_rejects_header_injection_shapes() {
        assert!(WorkdayTenantBinding::new("wd2.workday.com\r\nx-evil: yes", "acme").is_err());
        assert!(WorkdayTenantBinding::new("wd2.workday.com", "acme\nx-evil: yes").is_err());
    }
}
