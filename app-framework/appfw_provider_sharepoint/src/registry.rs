use std::collections::BTreeMap;

use crate::SharePointProviderError;

pub const SHAREPOINT_LIST_DRIVE_ITEMS_OPERATION: &str = "sharepoint.drive.list_items";
pub const SHAREPOINT_GET_DRIVE_ITEM_OPERATION: &str = "sharepoint.drive.get_item";
pub const SHAREPOINT_LIST_DRIVE_ITEM_VERSIONS_OPERATION: &str =
    "sharepoint.drive.list_item_versions";
pub const SHAREPOINT_UPLOAD_FILE_OPERATION: &str = "sharepoint.drive.upload_file";
pub const SHAREPOINT_UPLOAD_FILE_POLICY_SCOPE: &str = "sharepoint.drive.write";

/// Decision D2: 2 MB per file. Graph's simple-upload ceiling is far higher
/// (250 MB); this is the product's own, deliberately smaller transport limit,
/// enforced while the request plan is built so an oversized file never
/// reaches a gate, a ledger row or the network.
pub const SHAREPOINT_MAX_UPLOAD_BYTES: u64 = 2 * 1024 * 1024;

/// Parameter keys accepted by the named operations.
pub const PARAM_DRIVE_ID: &str = "drive_id";
pub const PARAM_ITEM_ID: &str = "item_id";
pub const PARAM_FOLDER_PATH: &str = "folder_path";
pub const PARAM_FILE_PATH: &str = "file_path";
pub const PARAM_CONTENT_TYPE: &str = "content_type";
pub const PARAM_CONTENT_LENGTH: &str = "content_length";

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SharePointNamedOperationRequest {
    pub name: String,
    pub params: BTreeMap<String, String>,
}

impl SharePointNamedOperationRequest {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            params: BTreeMap::new(),
        }
    }

    pub fn with_param(mut self, key: &str, value: impl Into<String>) -> Self {
        self.params.insert(key.to_string(), value.into());
        self
    }

    fn require(&self, key: &'static str) -> Result<&str, SharePointProviderError> {
        self.params
            .get(key)
            .map(String::as_str)
            .ok_or(SharePointProviderError::MissingParameter(key))
    }
}

/// A fully bound, validated, network-free description of one Graph call.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SharePointRequestPlan {
    pub method: &'static str,
    /// Path relative to `https://graph.microsoft.com/v1.0` (no host).
    pub path: String,
    pub query: Vec<(&'static str, &'static str)>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SharePointOperationKind {
    ListDriveItems,
    GetDriveItem,
    ListDriveItemVersions,
    UploadFile,
}

/// Unlike `appfw_provider_servicenow::ServiceNowOperationGate` (which mostly
/// waits on an authenticated tenant export), SharePoint's request shapes are
/// already public (Microsoft Learn). What this crate is actually waiting on is
/// framework-level wiring plus a real tenant grant - see each variant's
/// `reason()`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SharePointOperationGate {
    FrameworkProviderIdentityEvidence,
    TenantSiteGrantEvidence,
    DelegatedActorContextEvidence,
    TokenStoreIsolationEvidence,
    NamedMutationRegistryEvidence,
    MutationRequestBindingEvidence,
    IdempotencyReplayEvidence,
    WritePolicyScopeEvidence,
    WriteAuditEvidence,
    LiveCertification,
}

impl SharePointOperationGate {
    pub fn reason(self) -> &'static str {
        match self {
            Self::FrameworkProviderIdentityEvidence => {
                "requires a FrameworkProvider::SharePoint identity and a DataSourceType entry \
                 in app_gen before named-operation contracts can be wired to the framework's \
                 data-source system (see tax-doc-routing/docs/architecture/framework-issues-for-pds.md)"
            }
            Self::TenantSiteGrantEvidence => {
                "requires a real tenant SharePoint site id, drive id, and Sites.Selected role \
                 grant before this operation can be executed (see \
                 tax-doc-routing/docs/architecture/sharepoint-graph-admin-handoff.md)"
            }
            // The following are the shared G1 write-area reasons, same wording
            // appfw_runtime::provider_capabilities uses for every SaaS write.
            Self::DelegatedActorContextEvidence => {
                "SaaS delegated/on-behalf-of actor context remains unsupported until per-user \
                 token storage, tenant binding, and impersonation audit evidence exist"
            }
            Self::TokenStoreIsolationEvidence => {
                "SaaS per-user token-store isolation remains unsupported until encrypted \
                 storage, rotation, tenant partitioning, and revocation evidence exist"
            }
            Self::NamedMutationRegistryEvidence => {
                "SaaS named mutations remain unsupported until a provider-specific mutation \
                 registry and deny-by-default exposure gate exist"
            }
            Self::MutationRequestBindingEvidence => {
                "SaaS mutation request binding remains unsupported until provider-specific \
                 request templates, parameter binding, and redaction evidence exist"
            }
            Self::IdempotencyReplayEvidence => {
                "SaaS write idempotency and replay protection remain unsupported until \
                 idempotency keys, retry classification, and duplicate-suppression evidence exist"
            }
            Self::WritePolicyScopeEvidence => {
                "SaaS write policy and scope enforcement remain unsupported until \
                 operation-level policy, MCP/Kafka/frontend exposure gates, and \
                 least-privilege scopes are proven"
            }
            Self::WriteAuditEvidence => {
                "SaaS write audit remains unsupported until before/after redaction, request \
                 correlation, undo/compensation posture, and retained evidence exist"
            }
            Self::LiveCertification => {
                "requires live connector certification evidence per docs/runtime/saas-certification.md"
            }
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RegisteredSharePointOperation {
    pub name: &'static str,
    pub kind: SharePointOperationKind,
    pub policy_scope: Option<&'static str>,
    pub gates: &'static [SharePointOperationGate],
    pub description: &'static str,
}

impl RegisteredSharePointOperation {
    pub fn list_drive_items() -> Self {
        read_operation(
            SHAREPOINT_LIST_DRIVE_ITEMS_OPERATION,
            SharePointOperationKind::ListDriveItems,
            "GET /drives/{drive-id}/root/children (or /root:/{path}:/children) - list files/folders one level deep.",
        )
    }

    pub fn get_drive_item() -> Self {
        read_operation(
            SHAREPOINT_GET_DRIVE_ITEM_OPERATION,
            SharePointOperationKind::GetDriveItem,
            "GET /drives/{drive-id}/items/{item-id} - file metadata, including the pre-authenticated download URL.",
        )
    }

    pub fn list_drive_item_versions() -> Self {
        read_operation(
            SHAREPOINT_LIST_DRIVE_ITEM_VERSIONS_OPERATION,
            SharePointOperationKind::ListDriveItemVersions,
            "GET /drives/{drive-id}/items/{item-id}/versions - prior version history.",
        )
    }

    pub fn upload_file() -> Self {
        governed_write_operation(
            SHAREPOINT_UPLOAD_FILE_OPERATION,
            SharePointOperationKind::UploadFile,
            "PUT /drives/{drive-id}/root:/{path}:/content?@microsoft.graph.conflictBehavior=fail - \
             simple (non-chunked) upload, max 2 MB. Non-executable until G1 live evidence proves \
             delegated auth, policy, idempotency, and audit, on top of the framework wiring gap \
             shared by every operation in this crate.",
        )
    }

    pub fn is_named_read(&self) -> bool {
        matches!(
            self.kind,
            SharePointOperationKind::ListDriveItems
                | SharePointOperationKind::GetDriveItem
                | SharePointOperationKind::ListDriveItemVersions
        )
    }

    pub fn is_named_mutation(&self) -> bool {
        matches!(self.kind, SharePointOperationKind::UploadFile)
    }

    pub fn unsupported_reason(&self) -> &'static str {
        self.gates
            .first()
            .map(|gate| gate.reason())
            .unwrap_or("operation is planned but not executable")
    }
}

const SHAREPOINT_READ_GATES: &[SharePointOperationGate] = &[
    SharePointOperationGate::FrameworkProviderIdentityEvidence,
    SharePointOperationGate::TenantSiteGrantEvidence,
    SharePointOperationGate::LiveCertification,
];
const SHAREPOINT_GOVERNED_WRITE_GATES: &[SharePointOperationGate] = &[
    SharePointOperationGate::FrameworkProviderIdentityEvidence,
    SharePointOperationGate::TenantSiteGrantEvidence,
    SharePointOperationGate::DelegatedActorContextEvidence,
    SharePointOperationGate::TokenStoreIsolationEvidence,
    SharePointOperationGate::NamedMutationRegistryEvidence,
    SharePointOperationGate::MutationRequestBindingEvidence,
    SharePointOperationGate::IdempotencyReplayEvidence,
    SharePointOperationGate::WritePolicyScopeEvidence,
    SharePointOperationGate::WriteAuditEvidence,
    SharePointOperationGate::LiveCertification,
];

fn read_operation(
    name: &'static str,
    kind: SharePointOperationKind,
    description: &'static str,
) -> RegisteredSharePointOperation {
    RegisteredSharePointOperation {
        name,
        kind,
        policy_scope: None,
        gates: SHAREPOINT_READ_GATES,
        description,
    }
}

fn governed_write_operation(
    name: &'static str,
    kind: SharePointOperationKind,
    description: &'static str,
) -> RegisteredSharePointOperation {
    RegisteredSharePointOperation {
        name,
        kind,
        policy_scope: Some(SHAREPOINT_UPLOAD_FILE_POLICY_SCOPE),
        gates: SHAREPOINT_GOVERNED_WRITE_GATES,
        description,
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SharePointOperationRegistry {
    operations: BTreeMap<&'static str, RegisteredSharePointOperation>,
}

impl SharePointOperationRegistry {
    pub fn new(operations: impl IntoIterator<Item = RegisteredSharePointOperation>) -> Self {
        let operations = operations
            .into_iter()
            .map(|operation| (operation.name, operation))
            .collect();
        Self { operations }
    }

    pub fn default_planned_operations() -> Self {
        Self::new([
            RegisteredSharePointOperation::list_drive_items(),
            RegisteredSharePointOperation::get_drive_item(),
            RegisteredSharePointOperation::list_drive_item_versions(),
            RegisteredSharePointOperation::upload_file(),
        ])
    }

    pub fn get(&self, name: &str) -> Option<&RegisteredSharePointOperation> {
        self.operations.get(name)
    }

    pub fn operation_names(&self) -> impl Iterator<Item = &'static str> + '_ {
        self.operations.keys().copied()
    }

    /// Build the validated, network-free request plan for a named operation.
    /// This is what `compiler_contracted` means for this crate: the plan is
    /// proven by unit contracts; executing it is the product's job and stays
    /// behind the gates above.
    pub fn build_plan(
        &self,
        request: &SharePointNamedOperationRequest,
    ) -> Result<SharePointRequestPlan, SharePointProviderError> {
        let operation = self
            .get(&request.name)
            .ok_or_else(|| SharePointProviderError::UnknownOperation(request.name.clone()))?;
        match operation.kind {
            SharePointOperationKind::ListDriveItems => {
                let drive = path_segment(PARAM_DRIVE_ID, request.require(PARAM_DRIVE_ID)?)?;
                let path = match request.params.get(PARAM_FOLDER_PATH) {
                    Some(folder) => {
                        let folder = drive_relative_path(PARAM_FOLDER_PATH, folder)?;
                        format!("/drives/{drive}/root:/{folder}:/children")
                    }
                    None => format!("/drives/{drive}/root/children"),
                };
                Ok(SharePointRequestPlan {
                    method: "GET",
                    path,
                    query: Vec::new(),
                })
            }
            SharePointOperationKind::GetDriveItem => {
                let drive = path_segment(PARAM_DRIVE_ID, request.require(PARAM_DRIVE_ID)?)?;
                let item = path_segment(PARAM_ITEM_ID, request.require(PARAM_ITEM_ID)?)?;
                Ok(SharePointRequestPlan {
                    method: "GET",
                    path: format!("/drives/{drive}/items/{item}"),
                    query: Vec::new(),
                })
            }
            SharePointOperationKind::ListDriveItemVersions => {
                let drive = path_segment(PARAM_DRIVE_ID, request.require(PARAM_DRIVE_ID)?)?;
                let item = path_segment(PARAM_ITEM_ID, request.require(PARAM_ITEM_ID)?)?;
                Ok(SharePointRequestPlan {
                    method: "GET",
                    path: format!("/drives/{drive}/items/{item}/versions"),
                    query: Vec::new(),
                })
            }
            SharePointOperationKind::UploadFile => {
                let drive = path_segment(PARAM_DRIVE_ID, request.require(PARAM_DRIVE_ID)?)?;
                let file = drive_relative_path(PARAM_FILE_PATH, request.require(PARAM_FILE_PATH)?)?;
                let length: u64 = request
                    .require(PARAM_CONTENT_LENGTH)?
                    .parse()
                    .map_err(|_| SharePointProviderError::InvalidParameter {
                        name: PARAM_CONTENT_LENGTH,
                        reason: "must be a non-negative integer byte count",
                    })?;
                if length == 0 {
                    return Err(SharePointProviderError::InvalidParameter {
                        name: PARAM_CONTENT_LENGTH,
                        reason: "an empty file is not uploaded",
                    });
                }
                if length > SHAREPOINT_MAX_UPLOAD_BYTES {
                    return Err(SharePointProviderError::InvalidParameter {
                        name: PARAM_CONTENT_LENGTH,
                        reason: "exceeds the 2 MB per-file upload limit (decision D2)",
                    });
                }
                let content_type = request.require(PARAM_CONTENT_TYPE)?;
                if content_type.is_empty()
                    || content_type.chars().any(|c| c.is_ascii_control())
                    || !content_type.contains('/')
                {
                    return Err(SharePointProviderError::InvalidParameter {
                        name: PARAM_CONTENT_TYPE,
                        reason: "must be a single-line media type such as application/pdf",
                    });
                }
                Ok(SharePointRequestPlan {
                    method: "PUT",
                    path: format!("/drives/{drive}/root:/{file}:/content"),
                    // conflictBehavior=fail: a retried upload either replays via
                    // the idempotency ledger or fails loudly - never overwrites.
                    query: vec![("@microsoft.graph.conflictBehavior", "fail")],
                })
            }
        }
    }

    pub fn ensure_operation_is_not_executable(
        &self,
        request: &SharePointNamedOperationRequest,
    ) -> Result<(), SharePointProviderError> {
        let operation = self
            .get(&request.name)
            .ok_or_else(|| SharePointProviderError::UnknownOperation(request.name.clone()))?;
        Err(SharePointProviderError::UnsupportedOperation {
            name: operation.name.to_string(),
            reason: operation.unsupported_reason(),
        })
    }

    pub fn ensure_named_read_is_not_executable(
        &self,
        request: &SharePointNamedOperationRequest,
    ) -> Result<(), SharePointProviderError> {
        let operation = self
            .get(&request.name)
            .ok_or_else(|| SharePointProviderError::UnknownOperation(request.name.clone()))?;
        if !operation.is_named_read() {
            return Err(SharePointProviderError::UnsupportedOperation {
                name: operation.name.to_string(),
                reason: "operation is a named mutation and must use the governed-write path",
            });
        }
        Err(SharePointProviderError::UnsupportedOperation {
            name: operation.name.to_string(),
            reason: operation.unsupported_reason(),
        })
    }

    pub fn ensure_named_mutation_is_not_executable(
        &self,
        request: &SharePointNamedOperationRequest,
    ) -> Result<(), SharePointProviderError> {
        let operation = self
            .get(&request.name)
            .ok_or_else(|| SharePointProviderError::UnknownOperation(request.name.clone()))?;
        if !operation.is_named_mutation() {
            return Err(SharePointProviderError::UnsupportedMutation(
                "operation is a named read and must not use the governed-write path",
            ));
        }
        Err(SharePointProviderError::UnsupportedOperation {
            name: operation.name.to_string(),
            reason: operation.unsupported_reason(),
        })
    }
}

impl Default for SharePointOperationRegistry {
    fn default() -> Self {
        Self::default_planned_operations()
    }
}

/// One URL path segment (an opaque Graph id). Rejects separators and control
/// characters instead of silently truncating a route.
fn path_segment(name: &'static str, value: &str) -> Result<String, SharePointProviderError> {
    let bad = value.is_empty()
        || value.trim() != value
        || value
            .chars()
            .any(|c| c.is_control() || matches!(c, '/' | '\\' | '?' | '#'));
    if bad {
        return Err(SharePointProviderError::InvalidParameter {
            name,
            reason: "must be a single opaque id with no path separators or control characters",
        });
    }
    Ok(value.to_string())
}

/// A drive-relative route (`{folder}/{file}`): every component is validated, so
/// no caller text can climb out of the project folder or inject a query.
fn drive_relative_path(name: &'static str, value: &str) -> Result<String, SharePointProviderError> {
    const FORBIDDEN: &[char] = &['\\', '?', '#', ':', '*', '"', '<', '>', '|', '%'];
    let invalid = |reason| SharePointProviderError::InvalidParameter { name, reason };
    if value.is_empty() || value.starts_with('/') || value.ends_with('/') {
        return Err(invalid("must be a non-empty relative path with no leading/trailing slash"));
    }
    for component in value.split('/') {
        if component.is_empty() || component == "." || component == ".." {
            return Err(invalid("must not contain empty, '.' or '..' components"));
        }
        if component.trim() != component || component.ends_with('.') {
            return Err(invalid("components must not be space-padded or end with a dot"));
        }
        if component
            .chars()
            .any(|c| c.is_control() || FORBIDDEN.contains(&c))
        {
            return Err(invalid("contains a character SharePoint or Graph does not allow"));
        }
    }
    Ok(value.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn upload(path: &str, length: &str) -> SharePointNamedOperationRequest {
        SharePointNamedOperationRequest::new(SHAREPOINT_UPLOAD_FILE_OPERATION)
            .with_param(PARAM_DRIVE_ID, "d1")
            .with_param(PARAM_FILE_PATH, path)
            .with_param(PARAM_CONTENT_TYPE, "application/pdf")
            .with_param(PARAM_CONTENT_LENGTH, length)
    }

    #[test]
    fn registry_exposes_stable_operation_names() {
        let registry = SharePointOperationRegistry::default();
        let names = registry.operation_names().collect::<Vec<_>>();
        // BTreeMap order: '_' (0x5F) sorts before 's' (0x73).
        assert_eq!(
            names,
            vec![
                SHAREPOINT_GET_DRIVE_ITEM_OPERATION,
                SHAREPOINT_LIST_DRIVE_ITEM_VERSIONS_OPERATION,
                SHAREPOINT_LIST_DRIVE_ITEMS_OPERATION,
                SHAREPOINT_UPLOAD_FILE_OPERATION,
            ]
        );
    }

    #[test]
    fn reads_are_not_executable_pending_framework_wiring() {
        let registry = SharePointOperationRegistry::default();
        for name in [
            SHAREPOINT_LIST_DRIVE_ITEMS_OPERATION,
            SHAREPOINT_GET_DRIVE_ITEM_OPERATION,
            SHAREPOINT_LIST_DRIVE_ITEM_VERSIONS_OPERATION,
        ] {
            let request = SharePointNamedOperationRequest::new(name);
            let error = registry
                .ensure_operation_is_not_executable(&request)
                .unwrap_err();
            assert!(matches!(
                error,
                SharePointProviderError::UnsupportedOperation { reason, .. }
                    if reason.contains("FrameworkProvider::SharePoint")
            ));
        }
    }

    #[test]
    fn upload_requires_g1_evidence_on_top_of_framework_wiring() {
        let registry = SharePointOperationRegistry::default();
        let operation = registry
            .get(SHAREPOINT_UPLOAD_FILE_OPERATION)
            .expect("upload operation should be registered");
        assert!(operation.is_named_mutation());
        assert_eq!(
            operation.policy_scope,
            Some(SHAREPOINT_UPLOAD_FILE_POLICY_SCOPE)
        );
        assert_eq!(operation.gates.len(), 10);
    }

    #[test]
    fn named_read_and_mutation_paths_do_not_cross() {
        let registry = SharePointOperationRegistry::default();
        let read = SharePointNamedOperationRequest::new(SHAREPOINT_LIST_DRIVE_ITEMS_OPERATION);
        let write = SharePointNamedOperationRequest::new(SHAREPOINT_UPLOAD_FILE_OPERATION);

        assert!(matches!(
            registry.ensure_named_read_is_not_executable(&write).unwrap_err(),
            SharePointProviderError::UnsupportedOperation { reason, .. }
                if reason.contains("named mutation")
        ));
        assert!(matches!(
            registry.ensure_named_mutation_is_not_executable(&read).unwrap_err(),
            SharePointProviderError::UnsupportedMutation(reason)
                if reason.contains("named read")
        ));
    }

    #[test]
    fn unknown_operation_is_rejected() {
        let registry = SharePointOperationRegistry::default();
        let request = SharePointNamedOperationRequest::new("sharepoint.drive.delete_everything");
        assert_eq!(
            registry.build_plan(&request).unwrap_err(),
            SharePointProviderError::UnknownOperation(
                "sharepoint.drive.delete_everything".to_string()
            )
        );
    }

    #[test]
    fn list_plan_targets_root_or_a_folder() {
        let registry = SharePointOperationRegistry::default();
        let root = SharePointNamedOperationRequest::new(SHAREPOINT_LIST_DRIVE_ITEMS_OPERATION)
            .with_param(PARAM_DRIVE_ID, "d1");
        assert_eq!(
            registry.build_plan(&root).unwrap().path,
            "/drives/d1/root/children"
        );
        let folder = root.with_param(PARAM_FOLDER_PATH, "ENT-100/2025");
        let plan = registry.build_plan(&folder).unwrap();
        assert_eq!(plan.method, "GET");
        assert_eq!(plan.path, "/drives/d1/root:/ENT-100/2025:/children");
    }

    #[test]
    fn get_item_and_versions_plans_use_the_item_id() {
        let registry = SharePointOperationRegistry::default();
        let item = SharePointNamedOperationRequest::new(SHAREPOINT_GET_DRIVE_ITEM_OPERATION)
            .with_param(PARAM_DRIVE_ID, "d1")
            .with_param(PARAM_ITEM_ID, "i1");
        assert_eq!(registry.build_plan(&item).unwrap().path, "/drives/d1/items/i1");
        let versions =
            SharePointNamedOperationRequest::new(SHAREPOINT_LIST_DRIVE_ITEM_VERSIONS_OPERATION)
                .with_param(PARAM_DRIVE_ID, "d1")
                .with_param(PARAM_ITEM_ID, "i1");
        assert_eq!(
            registry.build_plan(&versions).unwrap().path,
            "/drives/d1/items/i1/versions"
        );
    }

    #[test]
    fn upload_plan_is_a_put_to_content_that_fails_on_conflict() {
        let registry = SharePointOperationRegistry::default();
        let plan = registry
            .build_plan(&upload("ENT-100/doc-1__k1.pdf", "1024"))
            .unwrap();
        assert_eq!(plan.method, "PUT");
        assert_eq!(plan.path, "/drives/d1/root:/ENT-100/doc-1__k1.pdf:/content");
        assert_eq!(
            plan.query,
            vec![("@microsoft.graph.conflictBehavior", "fail")]
        );
    }

    #[test]
    fn upload_enforces_the_2_mb_limit_exactly() {
        let registry = SharePointOperationRegistry::default();
        assert!(registry
            .build_plan(&upload("a/b.pdf", &SHAREPOINT_MAX_UPLOAD_BYTES.to_string()))
            .is_ok());
        let too_big = (SHAREPOINT_MAX_UPLOAD_BYTES + 1).to_string();
        assert!(matches!(
            registry.build_plan(&upload("a/b.pdf", &too_big)).unwrap_err(),
            SharePointProviderError::InvalidParameter { name, reason }
                if name == PARAM_CONTENT_LENGTH && reason.contains("2 MB")
        ));
        assert!(registry.build_plan(&upload("a/b.pdf", "0")).is_err());
        assert!(registry.build_plan(&upload("a/b.pdf", "abc")).is_err());
    }

    #[test]
    fn upload_path_cannot_escape_or_inject() {
        let registry = SharePointOperationRegistry::default();
        for bad in [
            "../secrets.pdf",
            "a/../b.pdf",
            "/abs.pdf",
            "a//b.pdf",
            "a/b.pdf?x=1",
            "a/b#c.pdf",
            "a/b:c.pdf",
            "a\\b.pdf",
            "a/b.pdf/",
            "a/ b.pdf",
        ] {
            assert!(
                registry.build_plan(&upload(bad, "10")).is_err(),
                "{bad:?} must be rejected"
            );
        }
    }

    #[test]
    fn ids_must_be_single_segments() {
        let registry = SharePointOperationRegistry::default();
        let request = SharePointNamedOperationRequest::new(SHAREPOINT_GET_DRIVE_ITEM_OPERATION)
            .with_param(PARAM_DRIVE_ID, "d1/../d2")
            .with_param(PARAM_ITEM_ID, "i1");
        assert!(registry.build_plan(&request).is_err());
        let missing = SharePointNamedOperationRequest::new(SHAREPOINT_GET_DRIVE_ITEM_OPERATION)
            .with_param(PARAM_DRIVE_ID, "d1");
        assert_eq!(
            registry.build_plan(&missing).unwrap_err(),
            SharePointProviderError::MissingParameter(PARAM_ITEM_ID)
        );
    }
}
