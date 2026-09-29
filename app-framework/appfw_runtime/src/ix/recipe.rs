//! PDS-owned Intelligent Experience recipe registration at the runtime seam.
//!
//! The embedded registry is a byte-identical packaged projection of the PDS
//! authority. Product code supplies only its artifact type; the selected
//! recipe fixes intent, presentation schema, renderer, and capabilities.

use std::{
    collections::{BTreeMap, BTreeSet},
    sync::OnceLock,
};

use serde::{Deserialize, Serialize};

use super::contract::{
    validate_key, validate_text, IxArtifactBinding, IxContractError, IxRunRequest,
};

pub const PDS_IX_RECIPE_REGISTRY_SCHEMA_VERSION: &str = "pds.ix.recipe_registry@1";
pub const PDS_IX_RECIPE_REGISTRATION_SCHEMA_VERSION: &str = "pds.ix.recipe_registration@1";

const PDS_IX_RECIPE_REGISTRY_BYTES: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/contracts/pds_health/pds.ix.recipe-registry.v1.json"
));

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct IxRecipeRegistration {
    pub schema_version: String,
    pub recipe_id: String,
    pub intent_key: String,
    pub artifact_type: String,
    pub content_schema_version: String,
    pub renderer_key: String,
    pub required_capabilities: Vec<String>,
}

impl IxRecipeRegistration {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        schema_version: impl Into<String>,
        recipe_id: impl Into<String>,
        intent_key: impl Into<String>,
        artifact_type: impl Into<String>,
        content_schema_version: impl Into<String>,
        renderer_key: impl Into<String>,
        required_capabilities: Vec<String>,
    ) -> Result<Self, IxContractError> {
        let registration = Self {
            schema_version: schema_version.into(),
            recipe_id: recipe_id.into(),
            intent_key: intent_key.into(),
            artifact_type: artifact_type.into(),
            content_schema_version: content_schema_version.into(),
            renderer_key: renderer_key.into(),
            required_capabilities,
        };
        registration.validate()?;
        Ok(registration)
    }

    pub fn validate(&self) -> Result<(), IxContractError> {
        if self.schema_version != PDS_IX_RECIPE_REGISTRATION_SCHEMA_VERSION {
            return Err(IxContractError::InvalidRecipeRegistration);
        }
        for (field, value) in [
            ("recipeRegistration.recipeId", self.recipe_id.as_str()),
            ("recipeRegistration.intentKey", self.intent_key.as_str()),
            (
                "recipeRegistration.artifactType",
                self.artifact_type.as_str(),
            ),
            (
                "recipeRegistration.contentSchemaVersion",
                self.content_schema_version.as_str(),
            ),
            ("recipeRegistration.rendererKey", self.renderer_key.as_str()),
        ] {
            validate_key(field, value).map_err(|_| IxContractError::InvalidRecipeRegistration)?;
        }
        let registry = registry()?;
        let recipe = registry
            .recipes
            .iter()
            .find(|recipe| recipe.id == self.recipe_id)
            .ok_or(IxContractError::InvalidRecipeRegistration)?;
        if self.intent_key != recipe.intent_key
            || self.content_schema_version != recipe.presentation_schema_version
            || self.renderer_key != recipe.renderer_key
            || self.required_capabilities != recipe.required_capabilities
        {
            return Err(IxContractError::InvalidRecipeRegistration);
        }
        Ok(())
    }

    pub fn recipe_id(&self) -> &str {
        &self.recipe_id
    }

    pub fn intent_key(&self) -> &str {
        &self.intent_key
    }

    pub fn artifact_type(&self) -> &str {
        &self.artifact_type
    }

    pub fn content_schema_version(&self) -> &str {
        &self.content_schema_version
    }

    pub fn renderer_key(&self) -> &str {
        &self.renderer_key
    }

    pub fn required_capabilities(&self) -> &[String] {
        &self.required_capabilities
    }

    pub(crate) fn artifact_binding(&self) -> Result<IxArtifactBinding, IxContractError> {
        self.validate()?;
        IxArtifactBinding::new(
            self.artifact_type.clone(),
            self.content_schema_version.clone(),
            self.renderer_key.clone(),
        )
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct PdsIxRecipeRegistry {
    schema_version: String,
    presentation_schema_version: String,
    projections: Vec<String>,
    capabilities: Vec<String>,
    recipes: Vec<PdsIxRecipeDescriptor>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct PdsIxRecipeDescriptor {
    id: String,
    ordinal: u64,
    name: String,
    intent_key: String,
    renderer_key: String,
    presentation_schema_version: String,
    required_capabilities: Vec<String>,
    projections: BTreeMap<String, PdsIxProjectionState>,
}

#[derive(Debug, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct PdsIxProjectionState {
    applicability: String,
    readiness: String,
}

fn registry() -> Result<&'static PdsIxRecipeRegistry, IxContractError> {
    static REGISTRY: OnceLock<Result<PdsIxRecipeRegistry, ()>> = OnceLock::new();
    REGISTRY
        .get_or_init(|| {
            let registry: PdsIxRecipeRegistry =
                serde_json::from_slice(PDS_IX_RECIPE_REGISTRY_BYTES).map_err(|_| ())?;
            validate_registry(&registry).map_err(|_| ())?;
            Ok(registry)
        })
        .as_ref()
        .map_err(|_| IxContractError::InvalidRecipeRegistration)
}

fn validate_registry(registry: &PdsIxRecipeRegistry) -> Result<(), IxContractError> {
    let expected_projections = ["web-dom", "native-ios", "native-android"];
    if registry.schema_version != PDS_IX_RECIPE_REGISTRY_SCHEMA_VERSION
        || registry.presentation_schema_version != super::PDS_IX_PRESENTATION_SCHEMA_VERSION
        || registry
            .projections
            .iter()
            .map(String::as_str)
            .collect::<Vec<_>>()
            != expected_projections
        || registry.capabilities.len() != 12
        || registry.recipes.len() != 8
    {
        return Err(IxContractError::InvalidRecipeRegistration);
    }
    let capability_set = registry.capabilities.iter().collect::<BTreeSet<_>>();
    if capability_set.len() != registry.capabilities.len() {
        return Err(IxContractError::InvalidRecipeRegistration);
    }
    for capability in &registry.capabilities {
        validate_key("recipeRegistry.capability", capability)
            .map_err(|_| IxContractError::InvalidRecipeRegistration)?;
    }
    let mut ids = BTreeSet::new();
    let mut intents = BTreeSet::new();
    let mut renderers = BTreeSet::new();
    for (index, recipe) in registry.recipes.iter().enumerate() {
        if recipe.ordinal != (index + 1) as u64
            || recipe.presentation_schema_version != registry.presentation_schema_version
            || recipe.required_capabilities.len() != 5
            || recipe
                .required_capabilities
                .iter()
                .collect::<BTreeSet<_>>()
                .len()
                != recipe.required_capabilities.len()
            || recipe
                .required_capabilities
                .iter()
                .any(|capability| !capability_set.contains(capability))
            || !ids.insert(&recipe.id)
            || !intents.insert(&recipe.intent_key)
            || !renderers.insert(&recipe.renderer_key)
        {
            return Err(IxContractError::InvalidRecipeRegistration);
        }
        validate_key("recipeRegistry.recipeId", &recipe.id)
            .map_err(|_| IxContractError::InvalidRecipeRegistration)?;
        validate_text("recipeRegistry.name", &recipe.name, 512)
            .map_err(|_| IxContractError::InvalidRecipeRegistration)?;
        validate_key("recipeRegistry.intentKey", &recipe.intent_key)
            .map_err(|_| IxContractError::InvalidRecipeRegistration)?;
        validate_key("recipeRegistry.rendererKey", &recipe.renderer_key)
            .map_err(|_| IxContractError::InvalidRecipeRegistration)?;
        let projection_keys = recipe
            .projections
            .keys()
            .map(String::as_str)
            .collect::<BTreeSet<_>>();
        if projection_keys != expected_projections.into_iter().collect::<BTreeSet<_>>() {
            return Err(IxContractError::InvalidRecipeRegistration);
        }
        for (projection, state) in &recipe.projections {
            let expected_readiness = if projection == "web-dom" {
                "prototype"
            } else {
                "not-qualified"
            };
            if state.applicability != "supported" || state.readiness != expected_readiness {
                return Err(IxContractError::InvalidRecipeRegistration);
            }
        }
    }
    Ok(())
}

pub(crate) fn validate_recipe_binding(
    registration: Option<&IxRecipeRegistration>,
    request: &IxRunRequest,
    registered_artifacts: &BTreeSet<IxArtifactBinding>,
) -> Result<(), IxContractError> {
    let Some(registration) = registration else {
        return Ok(());
    };
    registration.validate()?;
    let expected_binding = registration.artifact_binding()?;
    if request.intent_key != registration.intent_key
        || registered_artifacts.len() != 1
        || !registered_artifacts.contains(&expected_binding)
    {
        return Err(IxContractError::InvalidRecipeRegistration);
    }
    if let Some(related) = &request.related_artifact {
        if related.artifact_type != registration.artifact_type
            || related.content_schema_version != registration.content_schema_version
            || related.renderer_key != registration.renderer_key
        {
            return Err(IxContractError::InvalidRecipeRegistration);
        }
    }
    Ok(())
}

#[cfg(test)]
pub(crate) fn test_registrations() -> Vec<IxRecipeRegistration> {
    registry()
        .expect("packaged registry must validate")
        .recipes
        .iter()
        .map(|recipe| IxRecipeRegistration {
            schema_version: PDS_IX_RECIPE_REGISTRATION_SCHEMA_VERSION.to_string(),
            recipe_id: recipe.id.clone(),
            intent_key: recipe.intent_key.clone(),
            artifact_type: format!("product.ix.{}@1", recipe.id),
            content_schema_version: recipe.presentation_schema_version.clone(),
            renderer_key: recipe.renderer_key.clone(),
            required_capabilities: recipe.required_capabilities.clone(),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn packaged_registry_drives_exact_eight_recipe_registration() {
        let registrations = test_registrations();
        assert_eq!(registrations.len(), 8);
        for registration in registrations {
            registration.validate().expect("canonical registration");
            let json = serde_json::to_value(&registration).expect("registration serializes");
            assert_eq!(json.as_object().expect("object").len(), 7);
            assert_eq!(
                json["schemaVersion"],
                PDS_IX_RECIPE_REGISTRATION_SCHEMA_VERSION
            );
        }
    }

    #[test]
    fn registration_deserialization_is_closed_and_validation_fails_substitution() {
        let canonical = &test_registrations()[0];
        let mut unknown = serde_json::to_value(canonical).expect("registration serializes");
        unknown["providerRoute"] = json!("not-allowed");
        assert!(serde_json::from_value::<IxRecipeRegistration>(unknown).is_err());

        let mut wrong_intent = canonical.clone();
        wrong_intent.intent_key = "pds.ix.intent.contextual-conversation@1".to_string();
        let mut wrong_renderer = canonical.clone();
        wrong_renderer.renderer_key = "pds.ix.recipe.contextual-conversation@1".to_string();
        let mut missing_capability = canonical.clone();
        missing_capability.required_capabilities.pop();
        let mut extra_capability = canonical.clone();
        extra_capability
            .required_capabilities
            .push("pds.ix.capability.contextual-follow-up@1".to_string());
        let mut reordered_capabilities = canonical.clone();
        reordered_capabilities.required_capabilities.swap(0, 1);
        let mut duplicate_capability = canonical.clone();
        let last = duplicate_capability.required_capabilities.len() - 1;
        duplicate_capability.required_capabilities[last] =
            duplicate_capability.required_capabilities[0].clone();
        let mut wrong_schema = canonical.clone();
        wrong_schema.content_schema_version = "product.presentation@1".to_string();
        let mut unknown_recipe = canonical.clone();
        unknown_recipe.recipe_id = "unknown-recipe".to_string();
        let mut invalid_artifact = canonical.clone();
        invalid_artifact.artifact_type = "contains whitespace".to_string();

        for candidate in [
            wrong_intent,
            wrong_renderer,
            missing_capability,
            extra_capability,
            reordered_capabilities,
            duplicate_capability,
            wrong_schema,
            unknown_recipe,
            invalid_artifact,
        ] {
            assert_eq!(
                candidate.validate(),
                Err(IxContractError::InvalidRecipeRegistration)
            );
        }
    }
}
