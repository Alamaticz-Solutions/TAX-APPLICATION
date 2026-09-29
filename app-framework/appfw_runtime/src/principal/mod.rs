use std::collections::HashSet;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::extension::RuntimePrincipalType;

pub const PRINCIPAL_CONTRACT_VERSION: &str = "principal@1";

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct AuthoritativeSubject {
    pub issuer: String,
    pub subject_id: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct BoundedServiceIdentity {
    pub service: AuthoritativeSubject,
    pub purpose: String,
    pub allowed_operations: Vec<String>,
    pub allowed_scopes: Vec<String>,
    pub valid_until: DateTime<Utc>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct PermissionedPrincipal {
    pub contract_version: String,
    pub principal_type: RuntimePrincipalType,
    pub subject: AuthoritativeSubject,
    pub tenant_id: String,
    #[serde(default)]
    pub roles: Vec<String>,
    #[serde(default)]
    pub scopes: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub service_identity: Option<BoundedServiceIdentity>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub on_behalf_of: Option<AuthoritativeSubject>,
}

#[derive(Clone, Debug, Error, Eq, PartialEq)]
pub enum PrincipalValidationError {
    #[error("unsupported principal contract version `{0}`")]
    UnsupportedVersion(String),
    #[error("principal field `{0}` must be non-empty and contain no control characters")]
    InvalidField(&'static str),
    #[error("principal list `{field}` contains an invalid or duplicate value `{value}`")]
    InvalidListValue { field: &'static str, value: String },
    #[error("human principals cannot carry a bounded service identity")]
    HumanServiceIdentity,
    #[error("human principals cannot carry on-behalf-of identity")]
    HumanOnBehalfOf,
    #[error("service and agent principals require a bounded service identity")]
    MissingServiceIdentity,
    #[error("bounded service identity must allow at least one operation")]
    MissingAllowedOperation,
    #[error("principal scope `{0}` exceeds the bounded service scope")]
    ScopeExceedsServiceBoundary(String),
    #[error("direct service principal subject must match its bounded service identity")]
    ServiceSubjectMismatch,
    #[error("agent subject must be distinct from its owning bounded service identity")]
    AgentServiceIdentityCollision,
    #[error("bounded service identity expired at {0}")]
    ServiceIdentityExpired(DateTime<Utc>),
    #[error("on-behalf-of identity must differ from the calling subject")]
    SelfDelegation,
}

impl PermissionedPrincipal {
    pub fn validate_at(&self, now: DateTime<Utc>) -> Result<(), PrincipalValidationError> {
        if self.contract_version != PRINCIPAL_CONTRACT_VERSION {
            return Err(PrincipalValidationError::UnsupportedVersion(
                self.contract_version.clone(),
            ));
        }

        validate_subject("subject.issuer", "subject.subject_id", &self.subject)?;
        validate_token("tenant_id", &self.tenant_id)?;
        validate_unique_list("roles", &self.roles)?;
        validate_unique_list("scopes", &self.scopes)?;

        match self.principal_type {
            RuntimePrincipalType::User => {
                if self.service_identity.is_some() {
                    return Err(PrincipalValidationError::HumanServiceIdentity);
                }
                if self.on_behalf_of.is_some() {
                    return Err(PrincipalValidationError::HumanOnBehalfOf);
                }
            }
            RuntimePrincipalType::Service => {
                let service = self
                    .service_identity
                    .as_ref()
                    .ok_or(PrincipalValidationError::MissingServiceIdentity)?;
                service.validate_at(now)?;
                if self.subject != service.service {
                    return Err(PrincipalValidationError::ServiceSubjectMismatch);
                }
                validate_service_scopes(&self.scopes, service)?;
            }
            RuntimePrincipalType::Agent => {
                let service = self
                    .service_identity
                    .as_ref()
                    .ok_or(PrincipalValidationError::MissingServiceIdentity)?;
                service.validate_at(now)?;
                if self.subject == service.service {
                    return Err(PrincipalValidationError::AgentServiceIdentityCollision);
                }
                validate_service_scopes(&self.scopes, service)?;
            }
        }

        if let Some(subject) = &self.on_behalf_of {
            validate_subject("on_behalf_of.issuer", "on_behalf_of.subject_id", subject)?;
            if subject == &self.subject {
                return Err(PrincipalValidationError::SelfDelegation);
            }
        }

        Ok(())
    }

    pub fn effective_subject(&self) -> &AuthoritativeSubject {
        self.on_behalf_of.as_ref().unwrap_or(&self.subject)
    }

    pub fn allows_operation(&self, operation_id: &str) -> bool {
        match self.principal_type {
            RuntimePrincipalType::User => true,
            RuntimePrincipalType::Service | RuntimePrincipalType::Agent => {
                self.service_identity.as_ref().is_some_and(|service| {
                    service
                        .allowed_operations
                        .iter()
                        .any(|allowed| allowed == operation_id)
                })
            }
        }
    }

    pub fn owning_service(&self) -> Option<&AuthoritativeSubject> {
        self.service_identity
            .as_ref()
            .map(|identity| &identity.service)
    }
}

impl BoundedServiceIdentity {
    fn validate_at(&self, now: DateTime<Utc>) -> Result<(), PrincipalValidationError> {
        validate_subject("service.issuer", "service.subject_id", &self.service)?;
        validate_token("service_identity.purpose", &self.purpose)?;
        validate_unique_list(
            "service_identity.allowed_operations",
            &self.allowed_operations,
        )?;
        validate_unique_list("service_identity.allowed_scopes", &self.allowed_scopes)?;
        if self.allowed_operations.is_empty() {
            return Err(PrincipalValidationError::MissingAllowedOperation);
        }
        if self.valid_until <= now {
            return Err(PrincipalValidationError::ServiceIdentityExpired(
                self.valid_until,
            ));
        }
        Ok(())
    }
}

fn validate_subject(
    issuer_field: &'static str,
    subject_field: &'static str,
    subject: &AuthoritativeSubject,
) -> Result<(), PrincipalValidationError> {
    validate_token(issuer_field, &subject.issuer)?;
    validate_token(subject_field, &subject.subject_id)
}

fn validate_service_scopes(
    scopes: &[String],
    service: &BoundedServiceIdentity,
) -> Result<(), PrincipalValidationError> {
    for scope in scopes {
        if !service
            .allowed_scopes
            .iter()
            .any(|allowed| allowed == scope)
        {
            return Err(PrincipalValidationError::ScopeExceedsServiceBoundary(
                scope.clone(),
            ));
        }
    }
    Ok(())
}

fn validate_unique_list(
    field: &'static str,
    values: &[String],
) -> Result<(), PrincipalValidationError> {
    let mut seen = HashSet::with_capacity(values.len());
    for value in values {
        if !valid_token(value) || !seen.insert(value.as_str()) {
            return Err(PrincipalValidationError::InvalidListValue {
                field,
                value: value.clone(),
            });
        }
    }
    Ok(())
}

fn validate_token(field: &'static str, value: &str) -> Result<(), PrincipalValidationError> {
    if valid_token(value) {
        Ok(())
    } else {
        Err(PrincipalValidationError::InvalidField(field))
    }
}

fn valid_token(value: &str) -> bool {
    !value.trim().is_empty() && !value.chars().any(char::is_control)
}

#[cfg(test)]
mod tests {
    use chrono::{Duration, TimeZone};

    use super::*;

    fn now() -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 7, 10, 12, 0, 0).unwrap()
    }

    fn subject(id: &str) -> AuthoritativeSubject {
        AuthoritativeSubject {
            issuer: "https://identity.example.test".to_string(),
            subject_id: id.to_string(),
        }
    }

    fn service_boundary() -> BoundedServiceIdentity {
        BoundedServiceIdentity {
            service: subject("svc-projection"),
            purpose: "refresh one permissioned projection".to_string(),
            allowed_operations: vec!["projection.refresh".to_string()],
            allowed_scopes: vec!["records.read".to_string()],
            valid_until: now() + Duration::minutes(15),
        }
    }

    fn principal(principal_type: RuntimePrincipalType) -> PermissionedPrincipal {
        let non_human = principal_type != RuntimePrincipalType::User;
        let subject_id = match principal_type {
            RuntimePrincipalType::User => "casey",
            RuntimePrincipalType::Service => "svc-projection",
            RuntimePrincipalType::Agent => "agent-projection",
        };
        PermissionedPrincipal {
            contract_version: PRINCIPAL_CONTRACT_VERSION.to_string(),
            principal_type,
            subject: subject(subject_id),
            tenant_id: "tenant-a".to_string(),
            roles: vec![],
            scopes: vec!["records.read".to_string()],
            service_identity: non_human.then(service_boundary),
            on_behalf_of: None,
        }
    }

    #[test]
    fn accepts_direct_human_identity_without_service_authority() {
        let principal = principal(RuntimePrincipalType::User);

        assert_eq!(principal.validate_at(now()), Ok(()));
        assert_eq!(principal.effective_subject(), &subject("casey"));
    }

    #[test]
    fn accepts_bounded_agent_with_explicit_on_behalf_of_identity() {
        let mut principal = principal(RuntimePrincipalType::Agent);
        principal.on_behalf_of = Some(subject("casey"));

        assert_eq!(principal.validate_at(now()), Ok(()));
        assert_eq!(principal.effective_subject(), &subject("casey"));
        assert_eq!(principal.owning_service(), Some(&subject("svc-projection")));
        assert!(principal.allows_operation("projection.refresh"));
        assert!(!principal.allows_operation("projection.write"));
    }

    #[test]
    fn rejects_unbounded_or_over_scoped_non_human_identity() {
        let mut principal = principal(RuntimePrincipalType::Service);
        principal.scopes = vec!["records.write".to_string()];
        principal.service_identity = None;

        assert_eq!(
            principal.validate_at(now()),
            Err(PrincipalValidationError::MissingServiceIdentity)
        );

        principal.service_identity = Some(service_boundary());
        assert_eq!(
            principal.validate_at(now()),
            Err(PrincipalValidationError::ScopeExceedsServiceBoundary(
                "records.write".to_string()
            ))
        );
    }

    #[test]
    fn rejects_expired_service_identity_and_human_delegation() {
        let mut service = principal(RuntimePrincipalType::Service);
        service.service_identity.as_mut().unwrap().valid_until = now();
        assert!(matches!(
            service.validate_at(now()),
            Err(PrincipalValidationError::ServiceIdentityExpired(_))
        ));

        let mut human = principal(RuntimePrincipalType::User);
        human.on_behalf_of = Some(subject("jordan"));
        assert_eq!(
            human.validate_at(now()),
            Err(PrincipalValidationError::HumanOnBehalfOf)
        );
    }

    #[test]
    fn rejects_duplicate_authority_values_and_self_delegation() {
        let mut principal = principal(RuntimePrincipalType::Agent);
        principal.roles = vec!["reader".to_string(), "reader".to_string()];
        assert!(matches!(
            principal.validate_at(now()),
            Err(PrincipalValidationError::InvalidListValue { field: "roles", .. })
        ));

        principal.roles.clear();
        principal.on_behalf_of = Some(principal.subject.clone());
        assert_eq!(
            principal.validate_at(now()),
            Err(PrincipalValidationError::SelfDelegation)
        );
    }

    #[test]
    fn binds_service_subject_and_keeps_agent_owner_distinct() {
        let mut service = principal(RuntimePrincipalType::Service);
        service.subject = subject("different-service");
        assert_eq!(
            service.validate_at(now()),
            Err(PrincipalValidationError::ServiceSubjectMismatch)
        );

        let mut agent = principal(RuntimePrincipalType::Agent);
        agent.subject = subject("svc-projection");
        assert_eq!(
            agent.validate_at(now()),
            Err(PrincipalValidationError::AgentServiceIdentityCollision)
        );
    }
}
