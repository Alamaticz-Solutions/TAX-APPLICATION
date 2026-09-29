pub const ORACLE_FINANCIALS_PROVIDER_KEY: &str = "oracle_financials";
pub const ORACLE_FINANCIALS_API_FAMILY: &str = "Oracle Fusion Cloud Financials REST API";
pub const ORACLE_FINANCIALS_RELEASE: &str = "26B";
pub const ORACLE_FINANCIALS_OPENAPI_VERSION: &str = "2026.03.27";

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OracleFinancialsProviderDescriptor {
    pub provider_key: &'static str,
    pub data_source_name: String,
    pub api_family: &'static str,
    pub release: &'static str,
    pub openapi_version: &'static str,
}

impl OracleFinancialsProviderDescriptor {
    pub fn provider_key(&self) -> &'static str {
        self.provider_key
    }
}

pub trait OracleFinancialsProviderIdentity {
    fn external_api_data_source_name(&self) -> &str;

    fn external_api_provider_descriptor(&self) -> OracleFinancialsProviderDescriptor {
        OracleFinancialsProviderDescriptor {
            provider_key: ORACLE_FINANCIALS_PROVIDER_KEY,
            data_source_name: self.external_api_data_source_name().to_string(),
            api_family: ORACLE_FINANCIALS_API_FAMILY,
            release: ORACLE_FINANCIALS_RELEASE,
            openapi_version: ORACLE_FINANCIALS_OPENAPI_VERSION,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TestProvider;

    impl OracleFinancialsProviderIdentity for TestProvider {
        fn external_api_data_source_name(&self) -> &str {
            "oracle_financials_primary"
        }
    }

    #[test]
    fn descriptor_uses_oracle_financials_provider_key_and_api_pin() {
        let descriptor = TestProvider.external_api_provider_descriptor();

        assert_eq!(descriptor.provider_key(), ORACLE_FINANCIALS_PROVIDER_KEY);
        assert_eq!(descriptor.data_source_name, "oracle_financials_primary");
        assert_eq!(descriptor.api_family, ORACLE_FINANCIALS_API_FAMILY);
        assert_eq!(descriptor.release, "26B");
        assert_eq!(descriptor.openapi_version, "2026.03.27");
    }
}
