pub const SERVICENOW_PROVIDER_KEY: &str = "servicenow";
pub const SERVICENOW_API_FAMILY: &str = "ServiceNow Table API / REST API Explorer export";
pub const SERVICENOW_RELEASE_BASIS: &str =
    "Australia public docs; authenticated instance export required";

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ServiceNowProviderDescriptor {
    pub provider_key: &'static str,
    pub data_source_name: String,
    pub api_family: &'static str,
    pub release_basis: &'static str,
}

impl ServiceNowProviderDescriptor {
    pub fn provider_key(&self) -> &'static str {
        self.provider_key
    }
}

pub trait ServiceNowProviderIdentity {
    fn external_api_data_source_name(&self) -> &str;

    fn external_api_provider_descriptor(&self) -> ServiceNowProviderDescriptor {
        ServiceNowProviderDescriptor {
            provider_key: SERVICENOW_PROVIDER_KEY,
            data_source_name: self.external_api_data_source_name().to_string(),
            api_family: SERVICENOW_API_FAMILY,
            release_basis: SERVICENOW_RELEASE_BASIS,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TestProvider;

    impl ServiceNowProviderIdentity for TestProvider {
        fn external_api_data_source_name(&self) -> &str {
            "servicenow_primary"
        }
    }

    #[test]
    fn descriptor_uses_servicenow_provider_key_and_evidence_basis() {
        let descriptor = TestProvider.external_api_provider_descriptor();

        assert_eq!(descriptor.provider_key(), SERVICENOW_PROVIDER_KEY);
        assert_eq!(descriptor.data_source_name, "servicenow_primary");
        assert_eq!(descriptor.api_family, SERVICENOW_API_FAMILY);
        assert!(descriptor
            .release_basis
            .contains("instance export required"));
    }
}
