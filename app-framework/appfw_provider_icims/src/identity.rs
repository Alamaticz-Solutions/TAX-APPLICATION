pub const ICIMS_PROVIDER_KEY: &str = "icims";
pub const ICIMS_API_FAMILY: &str = "iCIMS Talent Cloud / Applicant Tracking APIs";
pub const ICIMS_RELEASE_BASIS: &str =
    "public Developer Community pages; authenticated API docs required";

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IcimsProviderDescriptor {
    pub provider_key: &'static str,
    pub data_source_name: String,
    pub api_family: &'static str,
    pub release_basis: &'static str,
}

impl IcimsProviderDescriptor {
    pub fn provider_key(&self) -> &'static str {
        self.provider_key
    }
}

pub trait IcimsProviderIdentity {
    fn external_api_data_source_name(&self) -> &str;

    fn external_api_provider_descriptor(&self) -> IcimsProviderDescriptor {
        IcimsProviderDescriptor {
            provider_key: ICIMS_PROVIDER_KEY,
            data_source_name: self.external_api_data_source_name().to_string(),
            api_family: ICIMS_API_FAMILY,
            release_basis: ICIMS_RELEASE_BASIS,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TestProvider;

    impl IcimsProviderIdentity for TestProvider {
        fn external_api_data_source_name(&self) -> &str {
            "icims_primary"
        }
    }

    #[test]
    fn descriptor_uses_icims_provider_key_and_evidence_basis() {
        let descriptor = TestProvider.external_api_provider_descriptor();

        assert_eq!(descriptor.provider_key(), ICIMS_PROVIDER_KEY);
        assert_eq!(descriptor.data_source_name, "icims_primary");
        assert_eq!(descriptor.api_family, ICIMS_API_FAMILY);
        assert!(descriptor.release_basis.contains("authenticated API docs"));
    }
}
