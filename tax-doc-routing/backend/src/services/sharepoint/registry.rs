//! The allow-listed named READ operations against one SharePoint drive.
//! Callers select an operation and supply bound value parameters - they cannot
//! choose an arbitrary drive, site, or path. Request-plan construction and
//! validation live in the framework crate (`appfw-provider-sharepoint`); this
//! file binds typed operations to it and converts the result into the shared
//! `RequestPlan`.

use appfw_provider_sharepoint::{
    SharePointNamedOperationRequest, SharePointOperationRegistry, SharePointProviderError,
    SharePointRequestPlan, PARAM_DRIVE_ID, PARAM_FOLDER_PATH, PARAM_ITEM_ID,
    SHAREPOINT_GET_DRIVE_ITEM_OPERATION, SHAREPOINT_LIST_DRIVE_ITEMS_OPERATION,
    SHAREPOINT_LIST_DRIVE_ITEM_VERSIONS_OPERATION,
};

use super::request::RequestPlan;

#[derive(Debug, Clone)]
pub enum ReadOperation {
    /// Root of the library, or one entity folder. One level deep, not recursive.
    ListDriveItems {
        drive_id: String,
        folder_path: Option<String>,
    },
    /// The default Graph response already includes
    /// `@microsoft.graph.downloadUrl` - a short-lived, pre-authenticated URL -
    /// so a user can download without bytes streaming through this backend.
    GetDriveItem { drive_id: String, item_id: String },
    /// Prior versions SharePoint keeps for a replaced file.
    ListDriveItemVersions { drive_id: String, item_id: String },
}

impl ReadOperation {
    pub fn name(&self) -> &'static str {
        match self {
            Self::ListDriveItems { .. } => "list_drive_items",
            Self::GetDriveItem { .. } => "get_drive_item",
            Self::ListDriveItemVersions { .. } => "list_drive_item_versions",
        }
    }

    /// File names and folder names identify entities and taxpayers. Callers
    /// must not persist or forward them outside this system without the same
    /// handling as the record they came from.
    pub fn pii_possible(&self) -> bool {
        true
    }

    fn named_request(&self) -> SharePointNamedOperationRequest {
        match self {
            Self::ListDriveItems {
                drive_id,
                folder_path,
            } => {
                let request = SharePointNamedOperationRequest::new(
                    SHAREPOINT_LIST_DRIVE_ITEMS_OPERATION,
                )
                .with_param(PARAM_DRIVE_ID, drive_id.as_str());
                match folder_path {
                    Some(folder) => request.with_param(PARAM_FOLDER_PATH, folder.as_str()),
                    None => request,
                }
            }
            Self::GetDriveItem { drive_id, item_id } => {
                SharePointNamedOperationRequest::new(SHAREPOINT_GET_DRIVE_ITEM_OPERATION)
                    .with_param(PARAM_DRIVE_ID, drive_id.as_str())
                    .with_param(PARAM_ITEM_ID, item_id.as_str())
            }
            Self::ListDriveItemVersions { drive_id, item_id } => {
                SharePointNamedOperationRequest::new(
                    SHAREPOINT_LIST_DRIVE_ITEM_VERSIONS_OPERATION,
                )
                .with_param(PARAM_DRIVE_ID, drive_id.as_str())
                .with_param(PARAM_ITEM_ID, item_id.as_str())
            }
        }
    }

    /// Validated request plan. An invalid id or path is an error here - it is
    /// never silently rewritten into a different route.
    pub fn plan(&self) -> Result<RequestPlan, SharePointProviderError> {
        let planned = SharePointOperationRegistry::default().build_plan(&self.named_request())?;
        Ok(to_request_plan(&planned))
    }
}

fn to_request_plan(plan: &SharePointRequestPlan) -> RequestPlan {
    let base = match plan.method {
        "GET" => RequestPlan::get(plan.path.clone()),
        "POST" => RequestPlan::post(plan.path.clone()),
        other => RequestPlan {
            method: reqwest::Method::from_bytes(other.as_bytes())
                .expect("framework crate only emits valid HTTP method names"),
            path: plan.path.clone(),
            query: Vec::new(),
            headers: Vec::new(),
            accept: vec!["application/json"],
        },
    };
    plan.query
        .iter()
        .fold(base, |acc, (key, value)| acc.query(key, *value))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn list_drive_items_targets_root_children_without_a_folder() {
        let op = ReadOperation::ListDriveItems {
            drive_id: "d1".into(),
            folder_path: None,
        };
        assert_eq!(op.plan().unwrap().path, "/drives/d1/root/children");
    }

    #[test]
    fn list_drive_items_targets_the_entity_folder_when_given() {
        let op = ReadOperation::ListDriveItems {
            drive_id: "d1".into(),
            folder_path: Some("ORA-100/2025".into()),
        };
        let plan = op.plan().unwrap();
        assert_eq!(plan.method, reqwest::Method::GET);
        assert_eq!(plan.path, "/drives/d1/root:/ORA-100/2025:/children");
    }

    #[test]
    fn get_drive_item_targets_the_item_by_id() {
        let op = ReadOperation::GetDriveItem {
            drive_id: "d1".into(),
            item_id: "item-1".into(),
        };
        assert_eq!(op.plan().unwrap().path, "/drives/d1/items/item-1");
    }

    #[test]
    fn list_versions_targets_the_versions_sub_resource() {
        let op = ReadOperation::ListDriveItemVersions {
            drive_id: "d1".into(),
            item_id: "item-1".into(),
        };
        assert_eq!(op.plan().unwrap().path, "/drives/d1/items/item-1/versions");
    }

    #[test]
    fn path_traversal_in_a_folder_is_an_error_not_a_rewrite() {
        let op = ReadOperation::ListDriveItems {
            drive_id: "d1".into(),
            folder_path: Some("../other-site".into()),
        };
        assert!(op.plan().is_err());
        let op = ReadOperation::GetDriveItem {
            drive_id: "d1".into(),
            item_id: "a/b".into(),
        };
        assert!(op.plan().is_err());
    }

    #[test]
    fn every_read_is_marked_pii_possible() {
        let op = ReadOperation::GetDriveItem {
            drive_id: "d".into(),
            item_id: "i".into(),
        };
        assert!(op.pii_possible());
    }
}
