pub const SALESFORCE_PROVIDER_KEY: &str = "salesforce";

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SalesforceFrameworkProvider {
    Salesforce,
}

impl SalesforceFrameworkProvider {
    pub fn key(self) -> &'static str {
        match self {
            Self::Salesforce => SALESFORCE_PROVIDER_KEY,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SalesforceProviderDescriptor {
    pub provider: SalesforceFrameworkProvider,
    pub data_source_name: String,
}

impl SalesforceProviderDescriptor {
    pub fn new(data_source_name: impl Into<String>) -> Self {
        Self {
            provider: SalesforceFrameworkProvider::Salesforce,
            data_source_name: data_source_name.into(),
        }
    }

    pub fn provider_key(&self) -> &'static str {
        self.provider.key()
    }
}

pub trait SalesforceProviderIdentity {
    fn external_api_data_source_name(&self) -> &str;

    fn external_api_framework_provider(&self) -> SalesforceFrameworkProvider {
        SalesforceFrameworkProvider::Salesforce
    }

    fn external_api_provider_descriptor(&self) -> SalesforceProviderDescriptor {
        SalesforceProviderDescriptor::new(self.external_api_data_source_name())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TestProvider;

    impl SalesforceProviderIdentity for TestProvider {
        fn external_api_data_source_name(&self) -> &str {
            "salesforce_primary"
        }
    }

    #[test]
    fn descriptor_uses_salesforce_provider_key() {
        let descriptor = TestProvider.external_api_provider_descriptor();

        assert_eq!(descriptor.provider, SalesforceFrameworkProvider::Salesforce);
        assert_eq!(descriptor.provider_key(), "salesforce");
        assert_eq!(descriptor.data_source_name, "salesforce_primary");
    }
}
