#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ProviderContractArea {
    ScalarFilters,
    RelationshipFiltering,
    Sorting,
    Pagination,
    NativeProjection,
    RelationshipProjection,
    ManyToManyProjection,
    ManyToManyMutation,
    ManyToManyFiltering,
    Aggregation,
    AggregateFilters,
    PreparedStatementExecution,
    StoredRoutineInvocation,
    AccessFilters,
    TenantIsolation,
    ErrorNormalization,
    Concurrency,
    Audit,
}

impl ProviderContractArea {
    pub const ALL: [ProviderContractArea; 18] = [
        ProviderContractArea::ScalarFilters,
        ProviderContractArea::RelationshipFiltering,
        ProviderContractArea::Sorting,
        ProviderContractArea::Pagination,
        ProviderContractArea::NativeProjection,
        ProviderContractArea::RelationshipProjection,
        ProviderContractArea::ManyToManyProjection,
        ProviderContractArea::ManyToManyMutation,
        ProviderContractArea::ManyToManyFiltering,
        ProviderContractArea::Aggregation,
        ProviderContractArea::AggregateFilters,
        ProviderContractArea::PreparedStatementExecution,
        ProviderContractArea::StoredRoutineInvocation,
        ProviderContractArea::AccessFilters,
        ProviderContractArea::TenantIsolation,
        ProviderContractArea::ErrorNormalization,
        ProviderContractArea::Concurrency,
        ProviderContractArea::Audit,
    ];

    pub fn label(self) -> &'static str {
        match self {
            ProviderContractArea::ScalarFilters => "scalar filters",
            ProviderContractArea::RelationshipFiltering => "relationship filtering",
            ProviderContractArea::Sorting => "sorting",
            ProviderContractArea::Pagination => "pagination",
            ProviderContractArea::NativeProjection => "native projection",
            ProviderContractArea::RelationshipProjection => "relationship projection",
            ProviderContractArea::ManyToManyProjection => "many-to-many projection",
            ProviderContractArea::ManyToManyMutation => "many-to-many mutation",
            ProviderContractArea::ManyToManyFiltering => "many-to-many filtering",
            ProviderContractArea::Aggregation => "aggregation",
            ProviderContractArea::AggregateFilters => "aggregate filters",
            ProviderContractArea::PreparedStatementExecution => "prepared statement execution",
            ProviderContractArea::StoredRoutineInvocation => "stored routine invocation",
            ProviderContractArea::AccessFilters => "access filters",
            ProviderContractArea::TenantIsolation => "tenant isolation",
            ProviderContractArea::ErrorNormalization => "error normalization",
            ProviderContractArea::Concurrency => "concurrency",
            ProviderContractArea::Audit => "audit",
        }
    }

    pub fn key(self) -> &'static str {
        match self {
            ProviderContractArea::ScalarFilters => "scalar_filters",
            ProviderContractArea::RelationshipFiltering => "relationship_filtering",
            ProviderContractArea::Sorting => "sorting",
            ProviderContractArea::Pagination => "pagination",
            ProviderContractArea::NativeProjection => "native_projection",
            ProviderContractArea::RelationshipProjection => "relationship_projection",
            ProviderContractArea::ManyToManyProjection => "many_to_many_projection",
            ProviderContractArea::ManyToManyMutation => "many_to_many_mutation",
            ProviderContractArea::ManyToManyFiltering => "many_to_many_filtering",
            ProviderContractArea::Aggregation => "aggregation",
            ProviderContractArea::AggregateFilters => "aggregate_filters",
            ProviderContractArea::PreparedStatementExecution => "prepared_statement_execution",
            ProviderContractArea::StoredRoutineInvocation => "stored_routine_invocation",
            ProviderContractArea::AccessFilters => "access_filters",
            ProviderContractArea::TenantIsolation => "tenant_isolation",
            ProviderContractArea::ErrorNormalization => "error_normalization",
            ProviderContractArea::Concurrency => "concurrency",
            ProviderContractArea::Audit => "audit",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CapabilityStatus {
    Implemented(&'static str),
    CompilerContracted,
    LiveCertified,
    Partial(&'static str),
    Unsupported(&'static str),
    EmulatorLimited(&'static str),
}

impl CapabilityStatus {
    pub fn is_live_certified(self) -> bool {
        matches!(self, CapabilityStatus::LiveCertified)
    }

    pub fn requires_reason(self) -> bool {
        matches!(
            self,
            CapabilityStatus::Implemented(_)
                | CapabilityStatus::Partial(_)
                | CapabilityStatus::Unsupported(_)
                | CapabilityStatus::EmulatorLimited(_)
        )
    }

    pub fn reason(self) -> Option<&'static str> {
        match self {
            CapabilityStatus::Implemented(reason)
            | CapabilityStatus::Partial(reason)
            | CapabilityStatus::Unsupported(reason)
            | CapabilityStatus::EmulatorLimited(reason) => Some(reason),
            CapabilityStatus::CompilerContracted | CapabilityStatus::LiveCertified => None,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            CapabilityStatus::Implemented(_) => "implemented",
            CapabilityStatus::CompilerContracted => "compiler-contracted",
            CapabilityStatus::LiveCertified => "live-certified",
            CapabilityStatus::Partial(_) => "partial",
            CapabilityStatus::Unsupported(_) => "unsupported",
            CapabilityStatus::EmulatorLimited(_) => "emulator-limited",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CertificationEvidence {
    CompilerContract(&'static str),
    LiveContract(&'static str),
}

impl CertificationEvidence {
    pub fn contract(self) -> &'static str {
        match self {
            CertificationEvidence::CompilerContract(contract)
            | CertificationEvidence::LiveContract(contract) => contract,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            CertificationEvidence::CompilerContract(_) => "compiler-contract",
            CertificationEvidence::LiveContract(_) => "live-contract",
        }
    }

    pub fn is_live(self) -> bool {
        matches!(self, CertificationEvidence::LiveContract(_))
    }

    pub fn is_compiler(self) -> bool {
        matches!(self, CertificationEvidence::CompilerContract(_))
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ExecutableContractKind {
    Compiler,
    Live,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ExecutableContract {
    pub area: ProviderContractArea,
    pub kind: ExecutableContractKind,
    pub name: &'static str,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ProviderCapability {
    pub area: ProviderContractArea,
    pub status: CapabilityStatus,
    pub evidence: &'static [CertificationEvidence],
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ProviderSemanticProfile<P> {
    pub provider: P,
    pub capabilities: &'static [ProviderCapability],
}

/// Certification areas for graph-read (and governed-write) providers.
///
/// Deliberately a separate enum from [`ProviderContractArea`]: graph providers
/// are not part of CRUD semantic parity. These are provider-specific guardrails
/// (read-only enforcement, traversal caps, governed writes), rendered through
/// the same data-driven matrix path as the CRUD areas.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum GraphReadArea {
    ConnectionAuth,
    ReadOnlyEnforcement,
    NamedQueryCompilation,
    UnsafeCypherRejection,
    TenantIsolation,
    TraversalLimits,
    ResultAndTimeoutCaps,
    QueryMetricsAndAudit,
    ProjectionFreshness,
    GovernedWriteEnforcement,
    WriteMetricsAndAudit,
}

impl GraphReadArea {
    pub const ALL: [GraphReadArea; 11] = [
        GraphReadArea::ConnectionAuth,
        GraphReadArea::ReadOnlyEnforcement,
        GraphReadArea::NamedQueryCompilation,
        GraphReadArea::UnsafeCypherRejection,
        GraphReadArea::TenantIsolation,
        GraphReadArea::TraversalLimits,
        GraphReadArea::ResultAndTimeoutCaps,
        GraphReadArea::QueryMetricsAndAudit,
        GraphReadArea::ProjectionFreshness,
        GraphReadArea::GovernedWriteEnforcement,
        GraphReadArea::WriteMetricsAndAudit,
    ];

    pub fn key(self) -> &'static str {
        match self {
            GraphReadArea::ConnectionAuth => "connection_auth",
            GraphReadArea::ReadOnlyEnforcement => "read_only_enforcement",
            GraphReadArea::NamedQueryCompilation => "named_query_compilation",
            GraphReadArea::UnsafeCypherRejection => "unsafe_cypher_rejection",
            GraphReadArea::TenantIsolation => "tenant_isolation",
            GraphReadArea::TraversalLimits => "traversal_limits",
            GraphReadArea::ResultAndTimeoutCaps => "result_and_timeout_caps",
            GraphReadArea::QueryMetricsAndAudit => "query_metrics_and_audit",
            GraphReadArea::ProjectionFreshness => "projection_freshness",
            GraphReadArea::GovernedWriteEnforcement => "governed_write_enforcement",
            GraphReadArea::WriteMetricsAndAudit => "write_metrics_and_audit",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            GraphReadArea::ConnectionAuth => "connection and auth",
            GraphReadArea::ReadOnlyEnforcement => "read-only enforcement",
            GraphReadArea::NamedQueryCompilation => "named query compilation",
            GraphReadArea::UnsafeCypherRejection => "unsafe cypher rejection",
            GraphReadArea::TenantIsolation => "tenant isolation",
            GraphReadArea::TraversalLimits => "traversal limits",
            GraphReadArea::ResultAndTimeoutCaps => "result and timeout caps",
            GraphReadArea::QueryMetricsAndAudit => "query metrics and audit",
            GraphReadArea::ProjectionFreshness => "projection freshness",
            GraphReadArea::GovernedWriteEnforcement => "governed write enforcement",
            GraphReadArea::WriteMetricsAndAudit => "write metrics and audit",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GraphReadCapability {
    pub area: GraphReadArea,
    pub status: CapabilityStatus,
    pub evidence: &'static [CertificationEvidence],
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GraphReadSemanticProfile<P> {
    pub provider: P,
    pub capabilities: &'static [GraphReadCapability],
}

/// Certification areas for SaaS/external-API providers.
///
/// Deliberately a separate enum from [`ProviderContractArea`]: SaaS providers
/// do not implement generated CRUD semantic parity. They expose named,
/// parameterized operations with provider-specific request binding,
/// pagination, rate limits, and freshness/governance evidence.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SaasReadArea {
    ConnectionAuth,
    NamedOperationRegistry,
    RequestBinding,
    PaginationCursoring,
    RateLimitBackoff,
    IncrementalWatermark,
    FieldRedaction,
    TenantScoping,
    SchemaVersionPinning,
    ResultAndTimeoutCaps,
    QueryMetricsAndAudit,
    FreshnessReporting,
    GovernedWriteEnforcement,
    DelegatedActorContext,
    TokenStoreIsolation,
    NamedMutationRegistry,
    MutationRequestBinding,
    IdempotencyAndReplayProtection,
    WritePolicyAndScopeEnforcement,
    WriteAuditAndEvidence,
}

impl SaasReadArea {
    pub const ALL: [SaasReadArea; 20] = [
        SaasReadArea::ConnectionAuth,
        SaasReadArea::NamedOperationRegistry,
        SaasReadArea::RequestBinding,
        SaasReadArea::PaginationCursoring,
        SaasReadArea::RateLimitBackoff,
        SaasReadArea::IncrementalWatermark,
        SaasReadArea::FieldRedaction,
        SaasReadArea::TenantScoping,
        SaasReadArea::SchemaVersionPinning,
        SaasReadArea::ResultAndTimeoutCaps,
        SaasReadArea::QueryMetricsAndAudit,
        SaasReadArea::FreshnessReporting,
        SaasReadArea::GovernedWriteEnforcement,
        SaasReadArea::DelegatedActorContext,
        SaasReadArea::TokenStoreIsolation,
        SaasReadArea::NamedMutationRegistry,
        SaasReadArea::MutationRequestBinding,
        SaasReadArea::IdempotencyAndReplayProtection,
        SaasReadArea::WritePolicyAndScopeEnforcement,
        SaasReadArea::WriteAuditAndEvidence,
    ];

    pub fn key(self) -> &'static str {
        match self {
            SaasReadArea::ConnectionAuth => "connection_auth",
            SaasReadArea::NamedOperationRegistry => "named_operation_registry",
            SaasReadArea::RequestBinding => "request_binding",
            SaasReadArea::PaginationCursoring => "pagination_cursoring",
            SaasReadArea::RateLimitBackoff => "rate_limit_backoff",
            SaasReadArea::IncrementalWatermark => "incremental_watermark",
            SaasReadArea::FieldRedaction => "field_redaction",
            SaasReadArea::TenantScoping => "tenant_scoping",
            SaasReadArea::SchemaVersionPinning => "schema_version_pinning",
            SaasReadArea::ResultAndTimeoutCaps => "result_and_timeout_caps",
            SaasReadArea::QueryMetricsAndAudit => "query_metrics_and_audit",
            SaasReadArea::FreshnessReporting => "freshness_reporting",
            SaasReadArea::GovernedWriteEnforcement => "governed_write_enforcement",
            SaasReadArea::DelegatedActorContext => "delegated_actor_context",
            SaasReadArea::TokenStoreIsolation => "token_store_isolation",
            SaasReadArea::NamedMutationRegistry => "named_mutation_registry",
            SaasReadArea::MutationRequestBinding => "mutation_request_binding",
            SaasReadArea::IdempotencyAndReplayProtection => "idempotency_and_replay_protection",
            SaasReadArea::WritePolicyAndScopeEnforcement => "write_policy_and_scope_enforcement",
            SaasReadArea::WriteAuditAndEvidence => "write_audit_and_evidence",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            SaasReadArea::ConnectionAuth => "connection and auth",
            SaasReadArea::NamedOperationRegistry => "named operation registry",
            SaasReadArea::RequestBinding => "request binding",
            SaasReadArea::PaginationCursoring => "pagination and cursoring",
            SaasReadArea::RateLimitBackoff => "rate limit and backoff",
            SaasReadArea::IncrementalWatermark => "incremental watermark",
            SaasReadArea::FieldRedaction => "field redaction",
            SaasReadArea::TenantScoping => "tenant scoping",
            SaasReadArea::SchemaVersionPinning => "schema version pinning",
            SaasReadArea::ResultAndTimeoutCaps => "result and timeout caps",
            SaasReadArea::QueryMetricsAndAudit => "query metrics and audit",
            SaasReadArea::FreshnessReporting => "freshness reporting",
            SaasReadArea::GovernedWriteEnforcement => "governed write enforcement",
            SaasReadArea::DelegatedActorContext => "delegated actor context",
            SaasReadArea::TokenStoreIsolation => "token-store isolation",
            SaasReadArea::NamedMutationRegistry => "named mutation registry",
            SaasReadArea::MutationRequestBinding => "mutation request binding",
            SaasReadArea::IdempotencyAndReplayProtection => "idempotency and replay protection",
            SaasReadArea::WritePolicyAndScopeEnforcement => "write policy and scope enforcement",
            SaasReadArea::WriteAuditAndEvidence => "write audit and evidence",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SaasReadCapability {
    pub area: SaasReadArea,
    pub status: CapabilityStatus,
    pub evidence: &'static [CertificationEvidence],
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SaasReadSemanticProfile<P> {
    pub provider: P,
    pub capabilities: &'static [SaasReadCapability],
}
