pub const SHAREPOINT_PROVIDER_KEY: &str = "sharepoint";
pub const SHAREPOINT_API_FAMILY: &str = "Microsoft Graph v1.0 (drives / sites resources)";
pub const SHAREPOINT_RELEASE_BASIS: &str =
    "Public Microsoft Learn documentation; no authenticated tenant export required for request shapes, \
     but a real tenant site/drive grant is required before any operation is executable";

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SharePointProviderDescriptor {
    pub provider_key: &'static str,
    pub data_source_name: String,
    pub api_family: &'static str,
    pub release_basis: &'static str,
}

impl SharePointProviderDescriptor {
    pub fn provider_key(&self) -> &'static str {
        self.provider_key
    }
}

pub trait SharePointProviderIdentity {
    fn external_api_data_source_name(&self) -> &str;

    fn external_api_provider_descriptor(&self) -> SharePointProviderDescriptor {
        SharePointProviderDescriptor {
            provider_key: SHAREPOINT_PROVIDER_KEY,
            data_source_name: self.external_api_data_source_name().to_string(),
            api_family: SHAREPOINT_API_FAMILY,
            release_basis: SHAREPOINT_RELEASE_BASIS,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TestProvider;

    impl SharePointProviderIdentity for TestProvider {
        fn external_api_data_source_name(&self) -> &str {
            "sharepoint_tax_documents"
        }
    }

    #[test]
    fn descriptor_uses_sharepoint_provider_key_and_release_basis() {
        let descriptor = TestProvider.external_api_provider_descriptor();

        assert_eq!(descriptor.provider_key(), SHAREPOINT_PROVIDER_KEY);
        assert_eq!(descriptor.data_source_name, "sharepoint_tax_documents");
        assert_eq!(descriptor.api_family, SHAREPOINT_API_FAMILY);
        assert!(descriptor.release_basis.contains("tenant site/drive grant"));
    }
}
