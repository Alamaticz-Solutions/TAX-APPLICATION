pub const ANAPLAN_PROVIDER_KEY: &str = "anaplan";

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum AnaplanFrameworkProvider {
    Anaplan,
}

impl AnaplanFrameworkProvider {
    pub fn key(self) -> &'static str {
        match self {
            Self::Anaplan => ANAPLAN_PROVIDER_KEY,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AnaplanProviderDescriptor {
    pub provider: AnaplanFrameworkProvider,
    pub data_source_name: String,
    pub api_family: &'static str,
    pub api_base_path: &'static str,
}

impl AnaplanProviderDescriptor {
    pub fn new(data_source_name: impl Into<String>) -> Self {
        Self {
            provider: AnaplanFrameworkProvider::Anaplan,
            data_source_name: data_source_name.into(),
            api_family: "Integration API v2",
            api_base_path: "/2/0",
        }
    }

    pub fn provider_key(&self) -> &'static str {
        self.provider.key()
    }
}

pub trait AnaplanProviderIdentity {
    fn external_api_data_source_name(&self) -> &str;

    fn external_api_framework_provider(&self) -> AnaplanFrameworkProvider {
        AnaplanFrameworkProvider::Anaplan
    }

    fn external_api_provider_descriptor(&self) -> AnaplanProviderDescriptor {
        AnaplanProviderDescriptor::new(self.external_api_data_source_name())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TestProvider;

    impl AnaplanProviderIdentity for TestProvider {
        fn external_api_data_source_name(&self) -> &str {
            "anaplan_primary"
        }
    }

    #[test]
    fn descriptor_uses_anaplan_provider_key_and_api_pin() {
        let descriptor = TestProvider.external_api_provider_descriptor();

        assert_eq!(descriptor.provider, AnaplanFrameworkProvider::Anaplan);
        assert_eq!(descriptor.provider_key(), "anaplan");
        assert_eq!(descriptor.data_source_name, "anaplan_primary");
        assert_eq!(descriptor.api_family, "Integration API v2");
        assert_eq!(descriptor.api_base_path, "/2/0");
    }
}
