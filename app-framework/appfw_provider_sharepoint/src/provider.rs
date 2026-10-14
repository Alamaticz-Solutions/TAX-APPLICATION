use crate::{
    SharePointNamedOperationRequest, SharePointOperationRegistry, SharePointProviderError,
    SharePointProviderIdentity, SharePointRequestPlan,
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SharePointProvider {
    data_source_name: String,
    registry: SharePointOperationRegistry,
}

impl SharePointProvider {
    pub fn new(data_source_name: impl Into<String>) -> Self {
        Self {
            data_source_name: data_source_name.into(),
            registry: SharePointOperationRegistry::default(),
        }
    }

    pub fn with_registry(
        data_source_name: impl Into<String>,
        registry: SharePointOperationRegistry,
    ) -> Self {
        Self {
            data_source_name: data_source_name.into(),
            registry,
        }
    }

    pub fn registry(&self) -> &SharePointOperationRegistry {
        &self.registry
    }

    pub fn health_check_plan(&self) -> Result<(), SharePointProviderError> {
        Ok(())
    }

    /// Validated request plan for a named read. Network-free.
    pub fn build_named_read_plan(
        &self,
        request: &SharePointNamedOperationRequest,
    ) -> Result<SharePointRequestPlan, SharePointProviderError> {
        let operation = self
            .registry
            .get(&request.name)
            .ok_or_else(|| SharePointProviderError::UnknownOperation(request.name.clone()))?;
        if !operation.is_named_read() {
            return Err(SharePointProviderError::UnsupportedOperation {
                name: operation.name.to_string(),
                reason: "operation is a named mutation and must use the governed-write path",
            });
        }
        self.registry.build_plan(request)
    }

    /// Validated request plan for the named mutation. Building the plan does
    /// not execute anything; execution stays behind the G1 gates.
    pub fn build_named_mutation_plan(
        &self,
        request: &SharePointNamedOperationRequest,
    ) -> Result<SharePointRequestPlan, SharePointProviderError> {
        let operation = self
            .registry
            .get(&request.name)
            .ok_or_else(|| SharePointProviderError::UnknownOperation(request.name.clone()))?;
        if !operation.is_named_mutation() {
            return Err(SharePointProviderError::UnsupportedMutation(
                "operation is a named read and must not use the governed-write path",
            ));
        }
        self.registry.build_plan(request)
    }

    /// Whether the framework may execute this operation itself. Always an
    /// error today: no `FrameworkProvider::SharePoint` exists, so only the
    /// product-owned module executes (see `framework-issues-for-pds.md`).
    pub fn ensure_framework_execution(
        &self,
        request: &SharePointNamedOperationRequest,
    ) -> Result<(), SharePointProviderError> {
        self.registry.ensure_operation_is_not_executable(request)
    }
}

impl SharePointProviderIdentity for SharePointProvider {
    fn external_api_data_source_name(&self) -> &str {
        &self.data_source_name
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        SharePointProviderDescriptor, PARAM_CONTENT_LENGTH, PARAM_CONTENT_TYPE, PARAM_DRIVE_ID,
        PARAM_FILE_PATH, SHAREPOINT_API_FAMILY, SHAREPOINT_LIST_DRIVE_ITEMS_OPERATION,
        SHAREPOINT_PROVIDER_KEY, SHAREPOINT_UPLOAD_FILE_OPERATION,
    };

    #[test]
    fn provider_identity_matches_sharepoint_descriptor() {
        let provider = SharePointProvider::new("sharepoint_tax_documents");
        let descriptor: SharePointProviderDescriptor = provider.external_api_provider_descriptor();

        assert_eq!(descriptor.provider_key(), SHAREPOINT_PROVIDER_KEY);
        assert_eq!(descriptor.data_source_name, "sharepoint_tax_documents");
        assert_eq!(descriptor.api_family, SHAREPOINT_API_FAMILY);
    }

    #[test]
    fn provider_builds_a_read_plan_but_keeps_framework_execution_gated() {
        let provider = SharePointProvider::new("sharepoint_tax_documents");
        let request = SharePointNamedOperationRequest::new(SHAREPOINT_LIST_DRIVE_ITEMS_OPERATION)
            .with_param(PARAM_DRIVE_ID, "d1");

        let plan = provider.build_named_read_plan(&request).unwrap();
        assert_eq!(plan.path, "/drives/d1/root/children");
        assert!(matches!(
            provider.ensure_framework_execution(&request).unwrap_err(),
            SharePointProviderError::UnsupportedOperation { reason, .. }
                if reason.contains("FrameworkProvider::SharePoint")
        ));
    }

    #[test]
    fn provider_builds_the_upload_plan_through_the_mutation_path_only() {
        let provider = SharePointProvider::new("sharepoint_tax_documents");
        let request = SharePointNamedOperationRequest::new(SHAREPOINT_UPLOAD_FILE_OPERATION)
            .with_param(PARAM_DRIVE_ID, "d1")
            .with_param(PARAM_FILE_PATH, "ENT-1/doc.pdf")
            .with_param(PARAM_CONTENT_TYPE, "application/pdf")
            .with_param(PARAM_CONTENT_LENGTH, "100");

        assert_eq!(provider.build_named_mutation_plan(&request).unwrap().method, "PUT");
        assert!(matches!(
            provider.build_named_read_plan(&request).unwrap_err(),
            SharePointProviderError::UnsupportedOperation { reason, .. }
                if reason.contains("named mutation")
        ));
    }
}
