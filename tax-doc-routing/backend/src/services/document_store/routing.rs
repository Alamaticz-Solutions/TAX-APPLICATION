//! Folder rules from the Pega process (`CreateTargetFolders` + `CreateFolderReleasedToClient`),
//! as a pure function. The decision tables `GetInternalFolderName` / `GetExternalFolderName`
//! are the `document_type_routes` rows, passed in as `DocumentRoute`.

/// One `document_type_routes` row, reduced to what folder building needs.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DocumentRoute {
    pub document_type: String,
    /// `GetInternalFolderName` (e.g. `4. Tax Documents`). `None` for external-only types.
    pub internal_folder: Option<String>,
    /// `GetExternalFolderName` (e.g. `K-1s`).
    pub external_folder: String,
    /// S Corp Election, Form 8308, Extensions: no internal side is built
    /// (Pega's `ExternalDocumentType` when rule).
    pub is_external_only: bool,
}

/// Folder paths for one document. Each path is a chain of folder names below a root.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FilingPlan {
    /// Under the client's **Internal** root: `[Year, <internal type folder>]`.
    pub internal: Option<Vec<String>>,
    /// Under the client's **Internal** root: `[Year, "6. Released to Client", <client type folder>]`.
    pub released: Option<Vec<String>>,
    /// Under the client's **Client** root: `[Year, <client type folder>]`.
    pub external: Vec<String>,
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum RoutingError {
    /// Pega silently builds nothing for an unmapped type; we refuse instead.
    #[error("document type '{0}' has no folder mapping")]
    UnknownDocumentType(String),
    #[error("document type '{0}' is missing a folder name")]
    MissingFolderName(String),
    #[error("invalid document year {0}")]
    InvalidYear(i32),
}

pub fn plan_for(year: i32, route: &DocumentRoute, released_folder: &str) -> Result<FilingPlan, RoutingError> {
    if !(1900..=2200).contains(&year) {
        return Err(RoutingError::InvalidYear(year));
    }
    let year_folder = year.to_string();
    let external_name = route.external_folder.trim();
    if external_name.is_empty() || released_folder.trim().is_empty() {
        return Err(RoutingError::MissingFolderName(route.document_type.clone()));
    }
    let external = vec![year_folder.clone(), external_name.to_string()];
    if route.is_external_only {
        return Ok(FilingPlan { internal: None, released: None, external });
    }
    let internal_name = route
        .internal_folder
        .as_deref()
        .map(str::trim)
        .filter(|n| !n.is_empty())
        .ok_or_else(|| RoutingError::MissingFolderName(route.document_type.clone()))?;
    Ok(FilingPlan {
        internal: Some(vec![year_folder.clone(), internal_name.to_string()]),
        released: Some(vec![year_folder, released_folder.trim().to_string(), external_name.to_string()]),
        external,
    })
}

/// Finds the route for a document type: exact, then letters/digits-only and case-insensitive,
/// like Pega's `toLowerCase(toJavaIdentifierNoReplacing(...))` key, so `K 1`, `k-1` and `K-1`
/// all match.
pub fn find_route<'a>(routes: &'a [DocumentRoute], document_type: &str) -> Result<&'a DocumentRoute, RoutingError> {
    let key = |s: &str| s.chars().filter(|c| c.is_alphanumeric()).collect::<String>().to_lowercase();
    routes
        .iter()
        .find(|r| r.document_type == document_type)
        .or_else(|| routes.iter().find(|r| key(&r.document_type) == key(document_type)))
        .ok_or_else(|| RoutingError::UnknownDocumentType(document_type.to_string()))
}
