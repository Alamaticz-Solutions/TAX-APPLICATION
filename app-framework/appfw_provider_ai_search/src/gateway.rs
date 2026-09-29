use crate::{AiSearchAuthFlow, AI_SEARCH_IMPLEMENTATION_METADATA_PATH, AI_SEARCH_PROVIDER_KEY};

pub const AI_SEARCH_GATEWAY_CONTRACT_DOC: &str = "docs/runtime/ai-chat-search.md#auth-and-security";
pub const AI_SEARCH_GATEWAY_VENDOR: &str = "litellm";
pub const AI_SEARCH_GATEWAY_DEPLOYMENT_POSTURE: &str = "in_perimeter";
pub const AI_SEARCH_GATEWAY_POLICY_AUTHORITY: &str =
    "runtime_policy_and_generated_operation_dispatcher";
pub const AI_SEARCH_GATEWAY_CREDENTIAL_CUSTODY: &str = "server_secret_ref_only";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AiSearchGatewayKind {
    LiteLlm,
    OpenAiCompatible,
    InternalPdsSearch,
}

impl AiSearchGatewayKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::LiteLlm => "litellm",
            Self::OpenAiCompatible => "openai_compatible",
            Self::InternalPdsSearch => "internal_pds_search",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AiSearchGatewayEvidence {
    GatewaySelectionDecision,
    ServerSecretRefCustody,
    ClientCredentialsTokenCache,
    PromptAuditSiemRetention,
    EgressAllowlist,
    PolicyReResolution,
}

impl AiSearchGatewayEvidence {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::GatewaySelectionDecision => "gateway-selection-decision",
            Self::ServerSecretRefCustody => "server-secret-ref-custody",
            Self::ClientCredentialsTokenCache => "client-credentials-token-cache",
            Self::PromptAuditSiemRetention => "prompt-audit-siem-retention",
            Self::EgressAllowlist => "egress-allowlist",
            Self::PolicyReResolution => "policy-re-resolution",
        }
    }

    pub fn description(self) -> &'static str {
        match self {
            Self::GatewaySelectionDecision => {
                "signed decision selecting the gateway and approved model backend class"
            }
            Self::ServerSecretRefCustody => {
                "credential material remains server-side behind secret references"
            }
            Self::ClientCredentialsTokenCache => {
                "client-credentials flow uses shared M2M token-cache semantics"
            }
            Self::PromptAuditSiemRetention => {
                "prompt/answer audit retention and SIEM routing are configured before traffic"
            }
            Self::EgressAllowlist => "egress origin is pinned to an approved gateway origin",
            Self::PolicyReResolution => {
                "model-returned references are re-resolved through runtime policy"
            }
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AiSearchGatewayContract {
    pub preferred_gateway: AiSearchGatewayKind,
    pub allowed_gateways: &'static [AiSearchGatewayKind],
    pub preferred_auth_flow: AiSearchAuthFlow,
    pub credential_custody: &'static str,
    pub deployment_posture: &'static str,
    pub policy_authority: &'static str,
    pub required_evidence: &'static [AiSearchGatewayEvidence],
    pub source_refs: &'static [&'static str],
}

impl AiSearchGatewayContract {
    pub fn default_ch7() -> Self {
        Self {
            preferred_gateway: AiSearchGatewayKind::LiteLlm,
            allowed_gateways: &AI_SEARCH_GATEWAY_KINDS,
            preferred_auth_flow: AiSearchAuthFlow::ClientCredentials,
            credential_custody: AI_SEARCH_GATEWAY_CREDENTIAL_CUSTODY,
            deployment_posture: AI_SEARCH_GATEWAY_DEPLOYMENT_POSTURE,
            policy_authority: AI_SEARCH_GATEWAY_POLICY_AUTHORITY,
            required_evidence: &AI_SEARCH_GATEWAY_EVIDENCE,
            source_refs: &AI_SEARCH_GATEWAY_SOURCE_REFS,
        }
    }

    pub fn token_cache_key(
        self,
        data_source_name: &str,
        tenant_key: &str,
    ) -> AiSearchM2mTokenCacheKey {
        AiSearchM2mTokenCacheKey {
            provider_key: AI_SEARCH_PROVIDER_KEY,
            data_source_name: data_source_name.to_string(),
            tenant_key: tenant_key.to_string(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AiSearchM2mTokenCacheKey {
    pub provider_key: &'static str,
    pub data_source_name: String,
    pub tenant_key: String,
}

pub fn ai_search_gateway_contract() -> AiSearchGatewayContract {
    AiSearchGatewayContract::default_ch7()
}

pub const AI_SEARCH_GATEWAY_KINDS: [AiSearchGatewayKind; 3] = [
    AiSearchGatewayKind::LiteLlm,
    AiSearchGatewayKind::OpenAiCompatible,
    AiSearchGatewayKind::InternalPdsSearch,
];

pub const AI_SEARCH_GATEWAY_EVIDENCE: [AiSearchGatewayEvidence; 6] = [
    AiSearchGatewayEvidence::GatewaySelectionDecision,
    AiSearchGatewayEvidence::ServerSecretRefCustody,
    AiSearchGatewayEvidence::ClientCredentialsTokenCache,
    AiSearchGatewayEvidence::PromptAuditSiemRetention,
    AiSearchGatewayEvidence::EgressAllowlist,
    AiSearchGatewayEvidence::PolicyReResolution,
];

const AI_SEARCH_GATEWAY_SOURCE_REFS: [&str; 2] = [
    AI_SEARCH_GATEWAY_CONTRACT_DOC,
    AI_SEARCH_IMPLEMENTATION_METADATA_PATH,
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gateway_contract_prefers_litellm_with_server_secret_custody() {
        let contract = ai_search_gateway_contract();

        assert_eq!(contract.preferred_gateway, AiSearchGatewayKind::LiteLlm);
        assert_eq!(
            contract.preferred_gateway.as_str(),
            AI_SEARCH_GATEWAY_VENDOR
        );
        assert_eq!(contract.deployment_posture, "in_perimeter");
        assert_eq!(contract.credential_custody, "server_secret_ref_only");
        assert_eq!(
            contract.policy_authority,
            "runtime_policy_and_generated_operation_dispatcher"
        );
    }

    #[test]
    fn gateway_contract_requires_prompt_audit_and_policy_re_resolution() {
        let evidence = ai_search_gateway_contract()
            .required_evidence
            .iter()
            .map(|evidence| evidence.as_str())
            .collect::<Vec<_>>();

        assert!(evidence.contains(&"prompt-audit-siem-retention"));
        assert!(evidence.contains(&"policy-re-resolution"));
        assert!(evidence.contains(&"server-secret-ref-custody"));
    }

    #[test]
    fn token_cache_key_is_provider_and_tenant_scoped() {
        let key = ai_search_gateway_contract().token_cache_key("ai_search_primary", "tenant-alpha");

        assert_eq!(key.provider_key, "ai_search");
        assert_eq!(key.data_source_name, "ai_search_primary");
        assert_eq!(key.tenant_key, "tenant-alpha");
    }
}
