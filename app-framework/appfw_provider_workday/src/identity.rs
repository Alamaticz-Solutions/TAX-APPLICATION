pub const WORKDAY_PROVIDER_KEY: &str = "workday";

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum WorkdayFrameworkProvider {
    Workday,
}

impl WorkdayFrameworkProvider {
    pub fn key(self) -> &'static str {
        match self {
            Self::Workday => WORKDAY_PROVIDER_KEY,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WorkdayProviderDescriptor {
    pub provider: WorkdayFrameworkProvider,
    pub data_source_name: String,
    pub service_name: &'static str,
    pub service_version: &'static str,
}

impl WorkdayProviderDescriptor {
    pub fn new(data_source_name: impl Into<String>) -> Self {
        Self {
            provider: WorkdayFrameworkProvider::Workday,
            data_source_name: data_source_name.into(),
            service_name: "Human_ResourcesService",
            service_version: "v46.1",
        }
    }

    pub fn provider_key(&self) -> &'static str {
        self.provider.key()
    }
}

pub trait WorkdayProviderIdentity {
    fn external_api_data_source_name(&self) -> &str;

    fn external_api_framework_provider(&self) -> WorkdayFrameworkProvider {
        WorkdayFrameworkProvider::Workday
    }

    fn external_api_provider_descriptor(&self) -> WorkdayProviderDescriptor {
        WorkdayProviderDescriptor::new(self.external_api_data_source_name())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TestProvider;

    impl WorkdayProviderIdentity for TestProvider {
        fn external_api_data_source_name(&self) -> &str {
            "workday_hcm"
        }
    }

    #[test]
    fn descriptor_uses_workday_provider_key_and_version_pin() {
        let descriptor = TestProvider.external_api_provider_descriptor();

        assert_eq!(descriptor.provider, WorkdayFrameworkProvider::Workday);
        assert_eq!(descriptor.provider_key(), "workday");
        assert_eq!(descriptor.data_source_name, "workday_hcm");
        assert_eq!(descriptor.service_name, "Human_ResourcesService");
        assert_eq!(descriptor.service_version, "v46.1");
    }
}
