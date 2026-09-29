#![allow(dead_code)]

pub use crate::provider_contract_types::{
    CapabilityStatus, CertificationEvidence, ExecutableContract, ExecutableContractKind,
    GraphReadArea, GraphReadCapability, GraphReadSemanticProfile, ProviderCapability,
    ProviderContractArea, ProviderSemanticProfile, SaasReadArea, SaasReadCapability,
    SaasReadSemanticProfile,
};

use crate::provider_keys::FrameworkProvider;

pub const SUPPORTED_PROVIDERS: [FrameworkProvider; 5] = FrameworkProvider::DATABASE;

const FILTER_DSL_COMPILER: &str = "data::clients::contract_tests::filter_dsl_conformance_contract";
const SORT_COMPILER: &str = "data::clients::contract_tests::sort_conformance_contract";
const SELECTION_COMPILER: &str = "data::clients::contract_tests::selection_conformance_contract";
const RELATIONSHIP_IR_COMPILER: &str =
    "data::clients::contract_tests::generated_relationship_config_projects_into_query_ir_relationships";
const ACCESS_FILTER_COMPILER: &str =
    "data::clients::contract_tests::access_filter_contract_for_plan_based_clients";
const AGGREGATE_ACCESS_FILTER_COMPILER: &str =
    "data::clients::contract_tests::aggregate_access_filter_contract_for_plan_based_clients";
const CONCURRENCY_COMPILER: &str =
    "data::clients::contract_tests::optimistic_concurrency_contract_for_plan_based_clients";
const AUDIT_REDACTION_COMPILER: &str =
    "record_audit::tests::redaction_replaces_sensitive_properties_before_diffing";
const AUDIT_CHAIN_COMPILER: &str =
    "record_audit::tests::runtime_audit_event_can_continue_existing_record_chain";

const SCALAR_QUERY_LIVE: &str =
    "provider_semantic_contracts::provider_scalar_query_pagination_projection_and_error_contract";
const RELATIONSHIP_QUERY_LIVE: &str =
    "provider_semantic_contracts::provider_relationship_projection_contract";
const AGGREGATE_LIVE: &str = "provider_contracts::aggregate_accounts_contract";
const CONCURRENCY_LIVE: &str =
    "provider_semantic_contracts::provider_stale_delete_concurrency_contract";
const ACCESS_FILTER_LIVE: &str =
    "provider_semantic_contracts::provider_access_filter_policy_cannot_widen_user_filter_contract";
const TENANT_ISOLATION_LIVE: &str =
    "provider_semantic_contracts::provider_tenant_isolation_contract";
const ERROR_NORMALIZATION_LIVE: &str =
    "provider_semantic_contracts::provider_error_normalization_contract";
const AUDIT_LIVE: &str =
    "provider_semantic_contracts::provider_audit_append_redaction_chain_contract";
const STORED_ROUTINE_LIVE: &str =
    "provider_semantic_contracts::provider_stored_routine_return_payload_contract";
const POSTGRES_STORED_ROUTINE_COMPILER: &str =
    "appfw_provider_postgres::routine::tests::stored_procedure_call_quotes_identifiers_and_placeholders";
const MSSQL_STORED_ROUTINE_COMPILER: &str =
    "appfw_provider_mssql::routine::tests::stored_procedure_call_uses_provider_placeholders";
const SNOWFLAKE_STORED_ROUTINE_COMPILER: &str =
    "appfw_provider_snowflake::routine::tests::stored_procedure_call_uses_statement_bindings";

pub const EXECUTABLE_CONTRACTS: &[ExecutableContract] = &[
    executable_compiler(ProviderContractArea::ScalarFilters, FILTER_DSL_COMPILER),
    executable_compiler(ProviderContractArea::Sorting, SORT_COMPILER),
    executable_compiler(ProviderContractArea::NativeProjection, SELECTION_COMPILER),
    executable_compiler(
        ProviderContractArea::RelationshipFiltering,
        RELATIONSHIP_IR_COMPILER,
    ),
    executable_compiler(
        ProviderContractArea::RelationshipProjection,
        RELATIONSHIP_IR_COMPILER,
    ),
    executable_compiler(
        ProviderContractArea::ManyToManyProjection,
        RELATIONSHIP_IR_COMPILER,
    ),
    executable_compiler(
        ProviderContractArea::ManyToManyFiltering,
        RELATIONSHIP_IR_COMPILER,
    ),
    executable_compiler(ProviderContractArea::AccessFilters, ACCESS_FILTER_COMPILER),
    executable_compiler(
        ProviderContractArea::AggregateFilters,
        AGGREGATE_ACCESS_FILTER_COMPILER,
    ),
    executable_compiler(ProviderContractArea::Concurrency, CONCURRENCY_COMPILER),
    executable_compiler(ProviderContractArea::Audit, AUDIT_REDACTION_COMPILER),
    executable_compiler(ProviderContractArea::Audit, AUDIT_CHAIN_COMPILER),
    executable_compiler(
        ProviderContractArea::StoredRoutineInvocation,
        POSTGRES_STORED_ROUTINE_COMPILER,
    ),
    executable_compiler(
        ProviderContractArea::StoredRoutineInvocation,
        MSSQL_STORED_ROUTINE_COMPILER,
    ),
    executable_compiler(
        ProviderContractArea::StoredRoutineInvocation,
        SNOWFLAKE_STORED_ROUTINE_COMPILER,
    ),
    executable_compiler(
        ProviderContractArea::Aggregation,
        "data::query_ir::tests::aggregate_plan_parses_groups_metrics_having_sort_and_access_filter",
    ),
    executable_live(ProviderContractArea::ScalarFilters, SCALAR_QUERY_LIVE),
    executable_live(ProviderContractArea::Sorting, SCALAR_QUERY_LIVE),
    executable_live(ProviderContractArea::Pagination, SCALAR_QUERY_LIVE),
    executable_live(ProviderContractArea::NativeProjection, SCALAR_QUERY_LIVE),
    executable_live(
        ProviderContractArea::RelationshipFiltering,
        RELATIONSHIP_QUERY_LIVE,
    ),
    executable_live(
        ProviderContractArea::RelationshipProjection,
        RELATIONSHIP_QUERY_LIVE,
    ),
    executable_live(
        ProviderContractArea::ManyToManyProjection,
        RELATIONSHIP_QUERY_LIVE,
    ),
    executable_live(
        ProviderContractArea::ManyToManyFiltering,
        RELATIONSHIP_QUERY_LIVE,
    ),
    executable_live(ProviderContractArea::Aggregation, AGGREGATE_LIVE),
    executable_live(ProviderContractArea::AggregateFilters, AGGREGATE_LIVE),
    executable_live(ProviderContractArea::AccessFilters, ACCESS_FILTER_LIVE),
    executable_live(ProviderContractArea::TenantIsolation, TENANT_ISOLATION_LIVE),
    executable_live(
        ProviderContractArea::ErrorNormalization,
        ERROR_NORMALIZATION_LIVE,
    ),
    executable_live(ProviderContractArea::Concurrency, CONCURRENCY_LIVE),
    executable_live(ProviderContractArea::Audit, AUDIT_LIVE),
    executable_live(
        ProviderContractArea::StoredRoutineInvocation,
        STORED_ROUTINE_LIVE,
    ),
];

const SCALAR_FILTER_EVIDENCE: &[CertificationEvidence] = &[
    CertificationEvidence::CompilerContract(FILTER_DSL_COMPILER),
    CertificationEvidence::LiveContract(SCALAR_QUERY_LIVE),
];
const SORT_EVIDENCE: &[CertificationEvidence] = &[
    CertificationEvidence::CompilerContract(SORT_COMPILER),
    CertificationEvidence::LiveContract(SCALAR_QUERY_LIVE),
];
const PAGINATION_EVIDENCE: &[CertificationEvidence] =
    &[CertificationEvidence::LiveContract(SCALAR_QUERY_LIVE)];
const NATIVE_PROJECTION_EVIDENCE: &[CertificationEvidence] = &[
    CertificationEvidence::CompilerContract(SELECTION_COMPILER),
    CertificationEvidence::LiveContract(SCALAR_QUERY_LIVE),
];
const RELATIONSHIP_FILTER_EVIDENCE: &[CertificationEvidence] = &[
    CertificationEvidence::CompilerContract(RELATIONSHIP_IR_COMPILER),
    CertificationEvidence::LiveContract(RELATIONSHIP_QUERY_LIVE),
];
const RELATIONSHIP_PROJECTION_EVIDENCE: &[CertificationEvidence] = &[
    CertificationEvidence::CompilerContract(RELATIONSHIP_IR_COMPILER),
    CertificationEvidence::LiveContract(RELATIONSHIP_QUERY_LIVE),
];
const MANY_TO_MANY_PROJECTION_EVIDENCE: &[CertificationEvidence] = &[
    CertificationEvidence::CompilerContract(RELATIONSHIP_IR_COMPILER),
    CertificationEvidence::LiveContract(RELATIONSHIP_QUERY_LIVE),
];
const MANY_TO_MANY_FILTER_EVIDENCE: &[CertificationEvidence] = &[
    CertificationEvidence::CompilerContract(RELATIONSHIP_IR_COMPILER),
    CertificationEvidence::LiveContract(RELATIONSHIP_QUERY_LIVE),
];
const AGGREGATION_EVIDENCE: &[CertificationEvidence] = &[
    CertificationEvidence::CompilerContract(
        "data::query_ir::tests::aggregate_plan_parses_groups_metrics_having_sort_and_access_filter",
    ),
    CertificationEvidence::LiveContract(AGGREGATE_LIVE),
];
const AGGREGATE_FILTER_EVIDENCE: &[CertificationEvidence] = &[
    CertificationEvidence::CompilerContract(AGGREGATE_ACCESS_FILTER_COMPILER),
    CertificationEvidence::LiveContract(AGGREGATE_LIVE),
];
const ACCESS_FILTER_EVIDENCE: &[CertificationEvidence] = &[
    CertificationEvidence::CompilerContract(ACCESS_FILTER_COMPILER),
    CertificationEvidence::LiveContract(ACCESS_FILTER_LIVE),
];
const TENANT_ISOLATION_EVIDENCE: &[CertificationEvidence] =
    &[CertificationEvidence::LiveContract(TENANT_ISOLATION_LIVE)];
const ERROR_NORMALIZATION_EVIDENCE: &[CertificationEvidence] =
    &[CertificationEvidence::LiveContract(
        ERROR_NORMALIZATION_LIVE,
    )];
const CONCURRENCY_EVIDENCE: &[CertificationEvidence] = &[
    CertificationEvidence::CompilerContract(CONCURRENCY_COMPILER),
    CertificationEvidence::LiveContract(CONCURRENCY_LIVE),
];
const AUDIT_EVIDENCE: &[CertificationEvidence] = &[
    CertificationEvidence::CompilerContract(AUDIT_REDACTION_COMPILER),
    CertificationEvidence::CompilerContract(AUDIT_CHAIN_COMPILER),
    CertificationEvidence::LiveContract(AUDIT_LIVE),
];
const POSTGRES_STORED_ROUTINE_EVIDENCE: &[CertificationEvidence] = &[
    CertificationEvidence::CompilerContract(POSTGRES_STORED_ROUTINE_COMPILER),
    CertificationEvidence::LiveContract(STORED_ROUTINE_LIVE),
];
const MSSQL_STORED_ROUTINE_EVIDENCE: &[CertificationEvidence] =
    &[CertificationEvidence::CompilerContract(
        MSSQL_STORED_ROUTINE_COMPILER,
    )];
const SNOWFLAKE_STORED_ROUTINE_EVIDENCE: &[CertificationEvidence] =
    &[CertificationEvidence::CompilerContract(
        SNOWFLAKE_STORED_ROUTINE_COMPILER,
    )];

const POSTGRES_CAPABILITIES: [ProviderCapability; 18] = [
    live_certified(ProviderContractArea::ScalarFilters, SCALAR_FILTER_EVIDENCE),
    live_certified(
        ProviderContractArea::RelationshipFiltering,
        RELATIONSHIP_FILTER_EVIDENCE,
    ),
    live_certified(ProviderContractArea::Sorting, SORT_EVIDENCE),
    live_certified(ProviderContractArea::Pagination, PAGINATION_EVIDENCE),
    live_certified(
        ProviderContractArea::NativeProjection,
        NATIVE_PROJECTION_EVIDENCE,
    ),
    live_certified(
        ProviderContractArea::RelationshipProjection,
        RELATIONSHIP_PROJECTION_EVIDENCE,
    ),
    live_certified(
        ProviderContractArea::ManyToManyProjection,
        MANY_TO_MANY_PROJECTION_EVIDENCE,
    ),
    partial(
        ProviderContractArea::ManyToManyMutation,
        "provider mutation helpers exist; generated GraphQL many-to-many input still needs typed target-key parity",
        &[],
    ),
    unsupported(
        ProviderContractArea::ManyToManyFiltering,
        "relationship filtering through junction tables is not implemented",
        &[],
    ),
    live_certified(ProviderContractArea::Aggregation, AGGREGATION_EVIDENCE),
    live_certified(
        ProviderContractArea::AggregateFilters,
        AGGREGATE_FILTER_EVIDENCE,
    ),
    implemented(
        ProviderContractArea::PreparedStatementExecution,
        "appfw-provider-postgres exposes cached prepared query/execute helpers; stored routine live certification is tracked separately",
        &[],
    ),
    live_certified(
        ProviderContractArea::StoredRoutineInvocation,
        POSTGRES_STORED_ROUTINE_EVIDENCE,
    ),
    live_certified(ProviderContractArea::AccessFilters, ACCESS_FILTER_EVIDENCE),
    live_certified(
        ProviderContractArea::TenantIsolation,
        TENANT_ISOLATION_EVIDENCE,
    ),
    live_certified(
        ProviderContractArea::ErrorNormalization,
        ERROR_NORMALIZATION_EVIDENCE,
    ),
    live_certified(ProviderContractArea::Concurrency, CONCURRENCY_EVIDENCE),
    live_certified(ProviderContractArea::Audit, AUDIT_EVIDENCE),
];

const MONGO_CAPABILITIES: [ProviderCapability; 18] = [
    live_certified(ProviderContractArea::ScalarFilters, SCALAR_FILTER_EVIDENCE),
    live_certified(
        ProviderContractArea::RelationshipFiltering,
        RELATIONSHIP_FILTER_EVIDENCE,
    ),
    live_certified(ProviderContractArea::Sorting, SORT_EVIDENCE),
    live_certified(ProviderContractArea::Pagination, PAGINATION_EVIDENCE),
    live_certified(
        ProviderContractArea::NativeProjection,
        NATIVE_PROJECTION_EVIDENCE,
    ),
    live_certified(
        ProviderContractArea::RelationshipProjection,
        RELATIONSHIP_PROJECTION_EVIDENCE,
    ),
    live_certified(
        ProviderContractArea::ManyToManyProjection,
        MANY_TO_MANY_PROJECTION_EVIDENCE,
    ),
    unsupported(
        ProviderContractArea::ManyToManyMutation,
        "many-to-many mutation requires provider-owned junction handling",
        &[],
    ),
    live_certified(
        ProviderContractArea::ManyToManyFiltering,
        MANY_TO_MANY_FILTER_EVIDENCE,
    ),
    live_certified(ProviderContractArea::Aggregation, AGGREGATION_EVIDENCE),
    live_certified(
        ProviderContractArea::AggregateFilters,
        AGGREGATE_FILTER_EVIDENCE,
    ),
    unsupported(
        ProviderContractArea::PreparedStatementExecution,
        "MongoDB uses provider-native BSON commands and aggregation pipelines rather than SQL prepared statements",
        &[],
    ),
    unsupported(
        ProviderContractArea::StoredRoutineInvocation,
        "MongoDB provider execution is document/pipeline based; SQL stored procedures and functions are not applicable",
        &[],
    ),
    live_certified(ProviderContractArea::AccessFilters, ACCESS_FILTER_EVIDENCE),
    live_certified(
        ProviderContractArea::TenantIsolation,
        TENANT_ISOLATION_EVIDENCE,
    ),
    live_certified(
        ProviderContractArea::ErrorNormalization,
        ERROR_NORMALIZATION_EVIDENCE,
    ),
    live_certified(ProviderContractArea::Concurrency, CONCURRENCY_EVIDENCE),
    live_certified(ProviderContractArea::Audit, AUDIT_EVIDENCE),
];

const MSSQL_CAPABILITIES: [ProviderCapability; 18] = [
    live_certified(ProviderContractArea::ScalarFilters, SCALAR_FILTER_EVIDENCE),
    live_certified(
        ProviderContractArea::RelationshipFiltering,
        RELATIONSHIP_FILTER_EVIDENCE,
    ),
    live_certified(ProviderContractArea::Sorting, SORT_EVIDENCE),
    live_certified(ProviderContractArea::Pagination, PAGINATION_EVIDENCE),
    live_certified(
        ProviderContractArea::NativeProjection,
        NATIVE_PROJECTION_EVIDENCE,
    ),
    live_certified(
        ProviderContractArea::RelationshipProjection,
        RELATIONSHIP_PROJECTION_EVIDENCE,
    ),
    live_certified(
        ProviderContractArea::ManyToManyProjection,
        MANY_TO_MANY_PROJECTION_EVIDENCE,
    ),
    partial(
        ProviderContractArea::ManyToManyMutation,
        "provider mutation helpers exist; generated GraphQL many-to-many input still needs typed target-key parity",
        &[],
    ),
    unsupported(
        ProviderContractArea::ManyToManyFiltering,
        "relationship filtering through junction tables is not implemented",
        &[],
    ),
    live_certified(ProviderContractArea::Aggregation, AGGREGATION_EVIDENCE),
    live_certified(
        ProviderContractArea::AggregateFilters,
        AGGREGATE_FILTER_EVIDENCE,
    ),
    partial(
        ProviderContractArea::PreparedStatementExecution,
        "MS SQL Server provider exposes typed parameterized execution; reusable prepared-handle lifecycle is not implemented in the current ODBC path",
        &[],
    ),
    compiler_contracted(
        ProviderContractArea::StoredRoutineInvocation,
        MSSQL_STORED_ROUTINE_EVIDENCE,
    ),
    live_certified(ProviderContractArea::AccessFilters, ACCESS_FILTER_EVIDENCE),
    live_certified(
        ProviderContractArea::TenantIsolation,
        TENANT_ISOLATION_EVIDENCE,
    ),
    live_certified(
        ProviderContractArea::ErrorNormalization,
        ERROR_NORMALIZATION_EVIDENCE,
    ),
    live_certified(ProviderContractArea::Concurrency, CONCURRENCY_EVIDENCE),
    live_certified(ProviderContractArea::Audit, AUDIT_EVIDENCE),
];

const FABRIC_SQL_ANALYTICS_READ_ONLY_REASON: &str =
    "Microsoft Fabric SQL analytics endpoints are governed read-only TDS surfaces; generated mutations, migrations, and audit writes are intentionally blocked";

const FABRIC_SQL_ANALYTICS_CAPABILITIES: [ProviderCapability; 18] = [
    compiler_contracted(ProviderContractArea::ScalarFilters, SCALAR_FILTER_EVIDENCE),
    compiler_contracted(
        ProviderContractArea::RelationshipFiltering,
        RELATIONSHIP_FILTER_EVIDENCE,
    ),
    compiler_contracted(ProviderContractArea::Sorting, SORT_EVIDENCE),
    implemented(
        ProviderContractArea::Pagination,
        "FabricSqlAnalytics uses the shared MS SQL SELECT/OFFSET-FETCH query path; live Fabric certification is not yet retained",
        PAGINATION_EVIDENCE,
    ),
    compiler_contracted(
        ProviderContractArea::NativeProjection,
        NATIVE_PROJECTION_EVIDENCE,
    ),
    compiler_contracted(
        ProviderContractArea::RelationshipProjection,
        RELATIONSHIP_PROJECTION_EVIDENCE,
    ),
    compiler_contracted(
        ProviderContractArea::ManyToManyProjection,
        MANY_TO_MANY_PROJECTION_EVIDENCE,
    ),
    unsupported(
        ProviderContractArea::ManyToManyMutation,
        FABRIC_SQL_ANALYTICS_READ_ONLY_REASON,
        &[],
    ),
    unsupported(
        ProviderContractArea::ManyToManyFiltering,
        "relationship filtering through junction tables is not implemented for the shared MS SQL query path",
        &[],
    ),
    compiler_contracted(ProviderContractArea::Aggregation, AGGREGATION_EVIDENCE),
    compiler_contracted(
        ProviderContractArea::AggregateFilters,
        AGGREGATE_FILTER_EVIDENCE,
    ),
    partial(
        ProviderContractArea::PreparedStatementExecution,
        "FabricSqlAnalytics uses typed ODBC parameter binding; reusable prepared-handle lifecycle is not implemented",
        &[],
    ),
    partial(
        ProviderContractArea::StoredRoutineInvocation,
        "read-returning MS SQL compatible routines can use the shared routine helper; mutating/no-return procedures are blocked in FabricSqlAnalytics mode",
        MSSQL_STORED_ROUTINE_EVIDENCE,
    ),
    compiler_contracted(ProviderContractArea::AccessFilters, ACCESS_FILTER_EVIDENCE),
    implemented(
        ProviderContractArea::TenantIsolation,
        "FabricSqlAnalytics composes generated access filters through the shared MS SQL query path; live Fabric tenant-isolation evidence is not yet retained",
        TENANT_ISOLATION_EVIDENCE,
    ),
    implemented(
        ProviderContractArea::ErrorNormalization,
        "FabricSqlAnalytics uses shared ODBC/MS SQL error normalization; live Fabric error evidence is not yet retained",
        ERROR_NORMALIZATION_EVIDENCE,
    ),
    unsupported(
        ProviderContractArea::Concurrency,
        FABRIC_SQL_ANALYTICS_READ_ONLY_REASON,
        &[],
    ),
    unsupported(
        ProviderContractArea::Audit,
        FABRIC_SQL_ANALYTICS_READ_ONLY_REASON,
        &[],
    ),
];

const SNOWFLAKE_CAPABILITIES: [ProviderCapability; 18] = [
    live_certified(ProviderContractArea::ScalarFilters, SCALAR_FILTER_EVIDENCE),
    live_certified(
        ProviderContractArea::RelationshipFiltering,
        RELATIONSHIP_FILTER_EVIDENCE,
    ),
    live_certified(ProviderContractArea::Sorting, SORT_EVIDENCE),
    live_certified(ProviderContractArea::Pagination, PAGINATION_EVIDENCE),
    live_certified(
        ProviderContractArea::NativeProjection,
        NATIVE_PROJECTION_EVIDENCE,
    ),
    live_certified(
        ProviderContractArea::RelationshipProjection,
        RELATIONSHIP_PROJECTION_EVIDENCE,
    ),
    live_certified(
        ProviderContractArea::ManyToManyProjection,
        MANY_TO_MANY_PROJECTION_EVIDENCE,
    ),
    partial(
        ProviderContractArea::ManyToManyMutation,
        "provider mutation helpers exist; generated GraphQL many-to-many input still needs typed target-key parity",
        &[],
    ),
    unsupported(
        ProviderContractArea::ManyToManyFiltering,
        "relationship filtering through junction tables is not implemented",
        &[],
    ),
    emulator_limited(
        ProviderContractArea::Aggregation,
        "supported against hosted Snowflake; LocalStack Snowflake is public-preview parity coverage",
        AGGREGATION_EVIDENCE,
    ),
    emulator_limited(
        ProviderContractArea::AggregateFilters,
        "supported against hosted Snowflake; LocalStack Snowflake is public-preview parity coverage",
        AGGREGATE_FILTER_EVIDENCE,
    ),
    partial(
        ProviderContractArea::PreparedStatementExecution,
        "Snowflake SQL API statement bindings are supported; server-side prepared statement lifecycle is not native to this provider path",
        &[],
    ),
    compiler_contracted(
        ProviderContractArea::StoredRoutineInvocation,
        SNOWFLAKE_STORED_ROUTINE_EVIDENCE,
    ),
    live_certified(ProviderContractArea::AccessFilters, ACCESS_FILTER_EVIDENCE),
    live_certified(
        ProviderContractArea::TenantIsolation,
        TENANT_ISOLATION_EVIDENCE,
    ),
    live_certified(
        ProviderContractArea::ErrorNormalization,
        ERROR_NORMALIZATION_EVIDENCE,
    ),
    live_certified(ProviderContractArea::Concurrency, CONCURRENCY_EVIDENCE),
    live_certified(ProviderContractArea::Audit, AUDIT_EVIDENCE),
];

const POSTGRES_PROFILE: ProviderSemanticProfile<FrameworkProvider> = ProviderSemanticProfile {
    provider: FrameworkProvider::Postgres,
    capabilities: &POSTGRES_CAPABILITIES,
};

const MONGO_PROFILE: ProviderSemanticProfile<FrameworkProvider> = ProviderSemanticProfile {
    provider: FrameworkProvider::Mongo,
    capabilities: &MONGO_CAPABILITIES,
};

const MSSQL_PROFILE: ProviderSemanticProfile<FrameworkProvider> = ProviderSemanticProfile {
    provider: FrameworkProvider::Mssql,
    capabilities: &MSSQL_CAPABILITIES,
};

const FABRIC_SQL_ANALYTICS_PROFILE: ProviderSemanticProfile<FrameworkProvider> =
    ProviderSemanticProfile {
        provider: FrameworkProvider::FabricSqlAnalytics,
        capabilities: &FABRIC_SQL_ANALYTICS_CAPABILITIES,
    };

const SNOWFLAKE_PROFILE: ProviderSemanticProfile<FrameworkProvider> = ProviderSemanticProfile {
    provider: FrameworkProvider::Snowflake,
    capabilities: &SNOWFLAKE_CAPABILITIES,
};

pub fn semantic_profile(
    provider: FrameworkProvider,
) -> &'static ProviderSemanticProfile<FrameworkProvider> {
    match provider {
        FrameworkProvider::Postgres => &POSTGRES_PROFILE,
        FrameworkProvider::Mongo => &MONGO_PROFILE,
        FrameworkProvider::Mssql => &MSSQL_PROFILE,
        FrameworkProvider::FabricSqlAnalytics => &FABRIC_SQL_ANALYTICS_PROFILE,
        FrameworkProvider::Snowflake => &SNOWFLAKE_PROFILE,
        // Graph-read providers are not part of CRUD semantic parity; their
        // certification is data-driven through `graph_read_profile`.
        FrameworkProvider::Neo4j => {
            panic!("Neo4j is a graph read provider; use graph_read_profile() instead")
        }
        FrameworkProvider::ServiceNow
        | FrameworkProvider::Workday
        | FrameworkProvider::Icims
        | FrameworkProvider::Salesforce
        | FrameworkProvider::Anaplan
        | FrameworkProvider::OracleFinancials => {
            panic!("{provider:?} is an external API provider; use saas_read_profile() instead")
        }
        FrameworkProvider::AiSearch => {
            panic!("{provider:?} is an AI search provider; use provider-graduation AI search contract evidence instead")
        }
    }
}

// ---- Graph-read (and governed-write) certification profile ----

const NEO4J_CONNECTION_AUTH_COMPILER: &str =
    "appfw_provider_neo4j::connection::tests::managed_environment_requires_tls";
const NEO4J_READ_ONLY_COMPILER: &str =
    "appfw_provider_neo4j::query::tests::write_clauses_are_rejected";
const NEO4J_NAMED_QUERY_COMPILER: &str =
    "appfw_provider_neo4j::query::tests::read_only_named_query_maps_to_runtime_query";
const NEO4J_UNSAFE_CYPHER_COMPILER: &str =
    "appfw_provider_neo4j::query::tests::unbounded_traversal_is_rejected_at_construction";
const NEO4J_TENANT_ISOLATION_COMPILER: &str =
    "appfw_provider_neo4j::graph_read_provider::tenant_is_server_bound_and_isolated_per_caller";
const NEO4J_TRAVERSAL_COMPILER: &str =
    "appfw_provider_neo4j::query::tests::bounded_traversal_depth_is_resolved";
const NEO4J_LIMITS_COMPILER: &str =
    "appfw_provider_neo4j::limits::tests::requests_above_ceilings_are_clamped_down";
const NEO4J_AUDIT_COMPILER: &str =
    "appfw_provider_neo4j::audit::tests::audit_record_serializes_without_raw_parameter_values";
const NEO4J_FRESHNESS_COMPILER: &str =
    "appfw_provider_neo4j::graph_read_provider::named_query_executes_and_emits_audit_and_metadata";
const NEO4J_GOVERNED_WRITE_COMPILER: &str =
    "appfw_provider_neo4j::query::tests::governed_write_rejects_node_only_mutations";
const NEO4J_WRITE_AUDIT_COMPILER: &str =
    "appfw_provider_neo4j::graph_write_provider::governed_mutation_executes_and_audits";

const NEO4J_CONNECTION_AUTH_EVIDENCE: &[CertificationEvidence] =
    &[CertificationEvidence::CompilerContract(
        NEO4J_CONNECTION_AUTH_COMPILER,
    )];
const NEO4J_READ_ONLY_EVIDENCE: &[CertificationEvidence] =
    &[CertificationEvidence::CompilerContract(
        NEO4J_READ_ONLY_COMPILER,
    )];
const NEO4J_NAMED_QUERY_EVIDENCE: &[CertificationEvidence] =
    &[CertificationEvidence::CompilerContract(
        NEO4J_NAMED_QUERY_COMPILER,
    )];
const NEO4J_UNSAFE_CYPHER_EVIDENCE: &[CertificationEvidence] =
    &[CertificationEvidence::CompilerContract(
        NEO4J_UNSAFE_CYPHER_COMPILER,
    )];
const NEO4J_TENANT_ISOLATION_EVIDENCE: &[CertificationEvidence] =
    &[CertificationEvidence::CompilerContract(
        NEO4J_TENANT_ISOLATION_COMPILER,
    )];
const NEO4J_TRAVERSAL_EVIDENCE: &[CertificationEvidence] =
    &[CertificationEvidence::CompilerContract(
        NEO4J_TRAVERSAL_COMPILER,
    )];
const NEO4J_LIMITS_EVIDENCE: &[CertificationEvidence] = &[CertificationEvidence::CompilerContract(
    NEO4J_LIMITS_COMPILER,
)];
const NEO4J_AUDIT_EVIDENCE: &[CertificationEvidence] = &[CertificationEvidence::CompilerContract(
    NEO4J_AUDIT_COMPILER,
)];
const NEO4J_FRESHNESS_EVIDENCE: &[CertificationEvidence] =
    &[CertificationEvidence::CompilerContract(
        NEO4J_FRESHNESS_COMPILER,
    )];
const NEO4J_GOVERNED_WRITE_EVIDENCE: &[CertificationEvidence] =
    &[CertificationEvidence::CompilerContract(
        NEO4J_GOVERNED_WRITE_COMPILER,
    )];
const NEO4J_WRITE_AUDIT_EVIDENCE: &[CertificationEvidence] =
    &[CertificationEvidence::CompilerContract(
        NEO4J_WRITE_AUDIT_COMPILER,
    )];

const NEO4J_GRAPH_READ_CAPABILITIES: [GraphReadCapability; 11] = [
    graph_compiler_contracted(
        GraphReadArea::ConnectionAuth,
        NEO4J_CONNECTION_AUTH_EVIDENCE,
    ),
    graph_compiler_contracted(GraphReadArea::ReadOnlyEnforcement, NEO4J_READ_ONLY_EVIDENCE),
    graph_compiler_contracted(
        GraphReadArea::NamedQueryCompilation,
        NEO4J_NAMED_QUERY_EVIDENCE,
    ),
    graph_compiler_contracted(
        GraphReadArea::UnsafeCypherRejection,
        NEO4J_UNSAFE_CYPHER_EVIDENCE,
    ),
    graph_compiler_contracted(
        GraphReadArea::TenantIsolation,
        NEO4J_TENANT_ISOLATION_EVIDENCE,
    ),
    graph_compiler_contracted(GraphReadArea::TraversalLimits, NEO4J_TRAVERSAL_EVIDENCE),
    graph_compiler_contracted(GraphReadArea::ResultAndTimeoutCaps, NEO4J_LIMITS_EVIDENCE),
    graph_compiler_contracted(GraphReadArea::QueryMetricsAndAudit, NEO4J_AUDIT_EVIDENCE),
    graph_compiler_contracted(GraphReadArea::ProjectionFreshness, NEO4J_FRESHNESS_EVIDENCE),
    graph_compiler_contracted(
        GraphReadArea::GovernedWriteEnforcement,
        NEO4J_GOVERNED_WRITE_EVIDENCE,
    ),
    graph_compiler_contracted(
        GraphReadArea::WriteMetricsAndAudit,
        NEO4J_WRITE_AUDIT_EVIDENCE,
    ),
];

const NEO4J_GRAPH_READ_PROFILE: GraphReadSemanticProfile<FrameworkProvider> =
    GraphReadSemanticProfile {
        provider: FrameworkProvider::Neo4j,
        capabilities: &NEO4J_GRAPH_READ_CAPABILITIES,
    };

/// Graph-read certification profile for a graph provider. Panics for non-graph
/// providers (CRUD providers use [`semantic_profile`]).
pub fn graph_read_profile(
    provider: FrameworkProvider,
) -> &'static GraphReadSemanticProfile<FrameworkProvider> {
    match provider {
        FrameworkProvider::Neo4j => &NEO4J_GRAPH_READ_PROFILE,
        other => panic!("{other:?} is not a graph-read provider; use semantic_profile() instead"),
    }
}

// ---- SaaS/external-API certification profile ----

const SERVICE_NOW_AUTHENTICATED_EXPORT_REQUIRED_REASON: &str =
    "requires authenticated ServiceNow instance OpenAPI or dictionary export before named-operation contracts can be implemented or certified";
const ICIMS_AUTHENTICATED_DOCS_REQUIRED_REASON: &str =
    "requires authenticated iCIMS developer documentation before named-operation contracts can be implemented or certified";
const ANAPLAN_TENANT_OPERATION_SCOPE_REQUIRED_REASON: &str =
    "requires tenant-approved Anaplan auth, workspace/model binding, and selected metadata/export operations before named-operation contracts can be implemented or certified";
const ORACLE_FINANCIALS_OPERATION_SCOPE_REQUIRED_REASON: &str =
    "requires Oracle Financials tenant auth, business-unit or ledger scoping, and selected OpenAPI operation allow-lists before named-operation contracts can be implemented or certified";
const SAAS_LIVE_EVIDENCE_REQUIRED_REASON: &str =
    "requires live tenant evidence before this SaaS contract area can be certified";
const SALESFORCE_RATE_LIMIT_BACKOFF_PENDING_REASON: &str =
    "Salesforce rate-limit signals and retry decisions are compiler-contracted; circuit-breaker behavior still requires live tenant evidence";
const SAAS_GOVERNED_WRITE_UNSUPPORTED_REASON: &str =
    "SaaS provider writes remain unsupported until named mutation safety, idempotency, audit, and live write evidence exist";
const SAAS_DELEGATED_ACTOR_UNSUPPORTED_REASON: &str =
    "SaaS delegated/on-behalf-of actor context remains unsupported until per-user token storage, tenant binding, and impersonation audit evidence exist";
const SAAS_TOKEN_STORE_UNSUPPORTED_REASON: &str =
    "SaaS per-user token-store isolation remains unsupported until encrypted storage, rotation, tenant partitioning, and revocation evidence exist";
const SAAS_NAMED_MUTATION_UNSUPPORTED_REASON: &str =
    "SaaS named mutations remain unsupported until a provider-specific mutation registry and deny-by-default exposure gate exist";
const SAAS_MUTATION_BINDING_UNSUPPORTED_REASON: &str =
    "SaaS mutation request binding remains unsupported until provider-specific request templates, parameter binding, and redaction evidence exist";
const SAAS_IDEMPOTENCY_UNSUPPORTED_REASON: &str =
    "SaaS write idempotency and replay protection remain unsupported until idempotency keys, retry classification, and duplicate-suppression evidence exist";
const SAAS_WRITE_POLICY_UNSUPPORTED_REASON: &str =
    "SaaS write policy and scope enforcement remain unsupported until operation-level policy, MCP/Kafka/frontend exposure gates, and least-privilege scopes are proven";
const SAAS_WRITE_AUDIT_UNSUPPORTED_REASON: &str =
    "SaaS write audit remains unsupported until before/after redaction, request correlation, undo/compensation posture, and retained evidence exist";
const SAAS_AREA_COUNT: usize = SaasReadArea::ALL.len();

const SALESFORCE_REGISTRY_EVIDENCE: &[CertificationEvidence] = &[
    CertificationEvidence::CompilerContract(
        "appfw_provider_salesforce::registry::tests::registry_exposes_stable_operation_names",
    ),
    CertificationEvidence::CompilerContract(
        "appfw_provider_salesforce::metadata::tests::metadata_operation_names_match_registry_constants",
    ),
    CertificationEvidence::CompilerContract(
        "appfw_provider_salesforce::registry::tests::unknown_operation_is_rejected",
    ),
];
const SALESFORCE_REQUEST_BINDING_EVIDENCE: &[CertificationEvidence] = &[
    CertificationEvidence::CompilerContract(
        "appfw_provider_salesforce::registry::tests::account_by_id_operation_converts_to_saas_query_request_plan",
    ),
    CertificationEvidence::CompilerContract(
        "appfw_provider_salesforce::registry::tests::account_describe_rest_plan_converts_to_saas_request_plan",
    ),
    CertificationEvidence::CompilerContract(
        "appfw_provider_salesforce::registry::tests::account_get_updated_ids_converts_to_saas_rest_plan",
    ),
    CertificationEvidence::CompilerContract(
        "appfw_provider_salesforce::registry::tests::account_get_deleted_ids_converts_to_saas_rest_plan",
    ),
];
const SALESFORCE_PAGINATION_EVIDENCE: &[CertificationEvidence] =
    &[
        CertificationEvidence::CompilerContract(
        "appfw_provider_salesforce::registry::tests::continue_page_rest_plan_converts_to_saas_request_plan",
        ),
        CertificationEvidence::CompilerContract(
            "appfw_provider_salesforce::response::tests::query_response_page_exposes_valid_shared_url_continuation",
        ),
    ];
const SALESFORCE_INCREMENTAL_WATERMARK_EVIDENCE: &[CertificationEvidence] = &[
    CertificationEvidence::CompilerContract(
        "appfw_provider_salesforce::registry::tests::updated_accounts_operation_builds_watermark_query",
    ),
    CertificationEvidence::CompilerContract(
        "appfw_provider_salesforce::registry::tests::account_get_updated_ids_converts_to_saas_rest_plan",
    ),
    CertificationEvidence::CompilerContract(
        "appfw_provider_salesforce::registry::tests::account_get_deleted_ids_converts_to_saas_rest_plan",
    ),
    CertificationEvidence::CompilerContract(
        "appfw_provider_salesforce::response::tests::updated_ids_response_parses_ids_and_latest_date",
    ),
    CertificationEvidence::CompilerContract(
        "appfw_provider_salesforce::response::tests::deleted_ids_response_parses_deleted_records_and_coverage_dates",
    ),
];
const SALESFORCE_RATE_LIMIT_EVIDENCE: &[CertificationEvidence] = &[
    CertificationEvidence::CompilerContract(
        "appfw_provider_salesforce::response::tests::fetch_limits_summary_extracts_daily_api_remaining",
    ),
    CertificationEvidence::CompilerContract(
        "appfw_provider_salesforce::response::tests::rate_limit_signals_do_not_expose_header_values",
    ),
    CertificationEvidence::CompilerContract(
        "appfw_provider_salesforce::response::tests::rate_limit_summary_honors_retry_after_in_retry_decision",
    ),
];
const SALESFORCE_FIELD_REDACTION_EVIDENCE: &[CertificationEvidence] =
    &[CertificationEvidence::CompilerContract(
        "appfw_provider_salesforce::response::tests::query_response_json_redacts_records_and_returns_url_continuation",
    )];
const SALESFORCE_TENANT_SCOPING_EVIDENCE: &[CertificationEvidence] =
    &[CertificationEvidence::CompilerContract(
        "appfw_provider_salesforce::auth::tests::tenant_binding_rejects_paths_queries_and_userinfo",
    )];
const SALESFORCE_SCHEMA_VERSION_EVIDENCE: &[CertificationEvidence] = &[
    CertificationEvidence::CompilerContract(
        "appfw_provider_salesforce::metadata::tests::api_67_metadata_is_exposed_for_named_operations",
    ),
    CertificationEvidence::CompilerContract(
        "appfw_provider_salesforce::metadata::tests::health_cloud_projection_metadata_is_catalog_sourced_and_describe_gated",
    ),
    CertificationEvidence::CompilerContract(
        "appfw_provider_salesforce::registry::tests::account_describe_rest_plan_converts_to_saas_request_plan",
    ),
    CertificationEvidence::CompilerContract(
        "appfw_provider_salesforce::response::tests::describe_object_summary_fails_closed_when_account_field_missing",
    ),
    CertificationEvidence::CompilerContract(
        "appfw_provider_salesforce::registry::tests::updated_accounts_operation_converts_to_saas_query_request_plan",
    ),
    CertificationEvidence::CompilerContract(
        "appfw_provider_salesforce::registry::tests::account_get_updated_ids_converts_to_saas_rest_plan",
    ),
];
const SALESFORCE_RESULT_CAPS_EVIDENCE: &[CertificationEvidence] = &[
    CertificationEvidence::CompilerContract(
        "appfw_provider_salesforce::registry::tests::account_fetch_by_ids_converts_to_saas_query_request_plan",
    ),
    CertificationEvidence::CompilerContract(
        "appfw_provider_salesforce::registry::tests::account_get_deleted_ids_converts_to_saas_rest_plan",
    ),
    CertificationEvidence::CompilerContract(
        "appfw_provider_salesforce::registry::tests::updated_accounts_saas_plan_caps_max_results_at_salesforce_soql_limit",
    ),
    CertificationEvidence::CompilerContract(
        "appfw_provider_salesforce::registry::tests::zero_timeout_surfaces_as_invalid_saas_request_plan",
    ),
];
const SALESFORCE_GOVERNED_WRITE_EVIDENCE: &[CertificationEvidence] =
    &[CertificationEvidence::CompilerContract(
        "appfw_provider_salesforce::provider::tests::provider_rejects_mutation_planning_for_now",
    )];

const WORKDAY_REGISTRY_EVIDENCE: &[CertificationEvidence] = &[
    CertificationEvidence::CompilerContract(
        "appfw_provider_workday::registry::tests::registry_exposes_stable_operation_names",
    ),
    CertificationEvidence::CompilerContract(
        "appfw_provider_workday::registry::tests::unknown_operation_is_rejected",
    ),
];
const WORKDAY_REQUEST_BINDING_EVIDENCE: &[CertificationEvidence] = &[
    CertificationEvidence::CompilerContract(
        "appfw_provider_workday::registry::tests::get_workers_page_converts_to_valid_saas_soap_request_plan",
    ),
    CertificationEvidence::CompilerContract(
        "appfw_provider_workday::registry::tests::metadata_alias_converts_to_valid_saas_soap_request_plan",
    ),
];
const WORKDAY_PAGINATION_EVIDENCE: &[CertificationEvidence] =
    &[
        CertificationEvidence::CompilerContract(
            "appfw_provider_workday::registry::tests::get_workers_page_converts_to_valid_saas_soap_request_plan",
        ),
        CertificationEvidence::CompilerContract(
            "appfw_provider_workday::pagination::tests::next_continuation_from_page_one_preserves_count_and_stable_as_of",
        ),
    ];
const WORKDAY_INCREMENTAL_WATERMARK_EVIDENCE: &[CertificationEvidence] =
    &[CertificationEvidence::CompilerContract(
        "appfw_provider_workday::operation::tests::transaction_log_criteria_requires_paired_updated_range",
    )];
const WORKDAY_TENANT_SCOPING_EVIDENCE: &[CertificationEvidence] = &[
    CertificationEvidence::CompilerContract(
        "appfw_provider_workday::auth::tests::tenant_binding_rejects_url_and_path_shapes",
    ),
    CertificationEvidence::CompilerContract(
        "appfw_provider_workday::provider::tests::provider_plans_named_reads_as_shared_saas_request_from_server_bound_tenant",
    ),
    CertificationEvidence::CompilerContract(
        "appfw_provider_workday::registry::tests::non_human_resources_wws_path_shape_is_rejected",
    ),
];
const WORKDAY_SCHEMA_VERSION_EVIDENCE: &[CertificationEvidence] = &[
    CertificationEvidence::CompilerContract(
        "appfw_provider_workday::registry::tests::get_workers_page_converts_to_valid_saas_soap_request_plan",
    ),
    CertificationEvidence::CompilerContract(
        "appfw_provider_workday::registry::tests::metadata_alias_converts_to_valid_saas_soap_request_plan",
    ),
];
const WORKDAY_RESULT_CAPS_EVIDENCE: &[CertificationEvidence] = &[
    CertificationEvidence::CompilerContract(
        "appfw_provider_workday::registry::tests::get_workers_page_converts_to_valid_saas_soap_request_plan",
    ),
    CertificationEvidence::CompilerContract(
        "appfw_provider_workday::registry::tests::metadata_alias_converts_to_valid_saas_soap_request_plan",
    ),
];
const WORKDAY_FRESHNESS_EVIDENCE: &[CertificationEvidence] = &[
    CertificationEvidence::CompilerContract(
        "appfw_provider_workday::response::tests::parses_workday_server_timestamp",
    ),
    CertificationEvidence::CompilerContract(
        "appfw_provider_workday::registry::tests::get_server_timestamp_builds_single_row_metadata_request",
    ),
];
const WORKDAY_GOVERNED_WRITE_EVIDENCE: &[CertificationEvidence] =
    &[CertificationEvidence::CompilerContract(
        "appfw_provider_workday::provider::tests::provider_rejects_mutation_planning_for_now",
    )];
const ANAPLAN_REGISTRY_EVIDENCE: &[CertificationEvidence] = &[
    CertificationEvidence::CompilerContract(
        "appfw_provider_anaplan::registry::tests::registry_exposes_stable_operation_names",
    ),
    CertificationEvidence::CompilerContract(
        "appfw_provider_anaplan::registry::tests::unknown_operation_is_rejected",
    ),
];
const ANAPLAN_REQUEST_BINDING_EVIDENCE: &[CertificationEvidence] = &[
    CertificationEvidence::CompilerContract(
        "appfw_provider_anaplan::registry::tests::model_status_converts_to_server_bound_saas_request_plan",
    ),
    CertificationEvidence::CompilerContract(
        "appfw_provider_anaplan::registry::tests::files_list_uses_fixed_sort_offset_limit_and_caps",
    ),
    CertificationEvidence::CompilerContract(
        "appfw_provider_anaplan::registry::tests::registry_bound_view_read_request_converts_to_saas_post_plan",
    ),
    CertificationEvidence::CompilerContract(
        "appfw_provider_anaplan::registry::tests::registry_bound_view_read_page_uses_typed_continuation",
    ),
    CertificationEvidence::CompilerContract(
        "appfw_provider_anaplan::registry::tests::registry_bound_file_chunk_uses_typed_chunk_continuation",
    ),
];
const ANAPLAN_PAGINATION_EVIDENCE: &[CertificationEvidence] = &[
    CertificationEvidence::CompilerContract(
        "appfw_provider_anaplan::registry::tests::files_list_uses_fixed_sort_offset_limit_and_caps",
    ),
    CertificationEvidence::CompilerContract(
        "appfw_provider_anaplan::registry::tests::registry_bound_view_read_page_uses_typed_continuation",
    ),
    CertificationEvidence::CompilerContract(
        "appfw_provider_anaplan::response::tests::paging_summary_builds_offset_continuation",
    ),
];
const ANAPLAN_TENANT_SCOPING_EVIDENCE: &[CertificationEvidence] = &[
    CertificationEvidence::CompilerContract(
        "appfw_provider_anaplan::auth::tests::tenant_binding_rejects_paths_queries_and_untrusted_ids",
    ),
    CertificationEvidence::CompilerContract(
        "appfw_provider_anaplan::provider::tests::provider_plans_named_reads_as_shared_saas_request_without_network",
    ),
];
const ANAPLAN_SCHEMA_VERSION_EVIDENCE: &[CertificationEvidence] = &[
    CertificationEvidence::CompilerContract(
        "appfw_provider_anaplan::identity::tests::descriptor_uses_anaplan_provider_key_and_api_pin",
    ),
    CertificationEvidence::CompilerContract(
        "appfw_provider_anaplan::operation::tests::api_base_path_uses_anaplan_integration_api_v2",
    ),
];
const ANAPLAN_RESULT_CAPS_EVIDENCE: &[CertificationEvidence] = &[
    CertificationEvidence::CompilerContract(
        "appfw_provider_anaplan::registry::tests::files_list_uses_fixed_sort_offset_limit_and_caps",
    ),
    CertificationEvidence::CompilerContract(
        "appfw_provider_anaplan::registry::tests::files_list_caps_max_results_at_anaplan_page_limit",
    ),
    CertificationEvidence::CompilerContract(
        "appfw_provider_anaplan::registry::tests::registry_bound_file_chunk_uses_typed_chunk_continuation",
    ),
];
const ANAPLAN_GOVERNED_WRITE_EVIDENCE: &[CertificationEvidence] =
    &[CertificationEvidence::CompilerContract(
        "appfw_provider_anaplan::provider::tests::provider_rejects_mutation_planning_for_now",
    )];
const ORACLE_FINANCIALS_REGISTRY_EVIDENCE: &[CertificationEvidence] = &[
    CertificationEvidence::CompilerContract(
        "appfw_provider_oracle_financials::registry::tests::registry_exposes_stable_operation_names",
    ),
    CertificationEvidence::CompilerContract(
        "appfw_provider_oracle_financials::metadata::tests::metadata_operation_names_match_registry_constants",
    ),
    CertificationEvidence::CompilerContract(
        "appfw_provider_oracle_financials::registry::tests::unknown_operation_is_rejected",
    ),
];
const ORACLE_FINANCIALS_REQUEST_BINDING_EVIDENCE: &[CertificationEvidence] = &[
    CertificationEvidence::CompilerContract(
        "appfw_provider_oracle_financials::registry::tests::accounting_period_status_lov_list_converts_to_fixed_saas_request_plan",
    ),
    CertificationEvidence::CompilerContract(
        "appfw_provider_oracle_financials::registry::tests::accounting_period_status_lov_get_requires_single_id_path_segment",
    ),
    CertificationEvidence::CompilerContract(
        "appfw_provider_oracle_financials::registry::tests::broad_openapi_operations_are_planned_until_allowlisted",
    ),
    CertificationEvidence::CompilerContract(
        "appfw_provider_oracle_financials::registry::tests::submit_erp_integration_remains_unsupported_write_planning",
    ),
];
const ORACLE_FINANCIALS_PAGINATION_EVIDENCE: &[CertificationEvidence] = &[
    CertificationEvidence::CompilerContract(
        "appfw_provider_oracle_financials::registry::tests::accounting_period_status_lov_list_accepts_typed_offset_continuation",
    ),
    CertificationEvidence::CompilerContract(
        "appfw_provider_oracle_financials::response::tests::collection_summary_builds_next_offset_continuation",
    ),
];
const ORACLE_FINANCIALS_TENANT_SCOPING_EVIDENCE: &[CertificationEvidence] = &[
    CertificationEvidence::CompilerContract(
        "appfw_provider_oracle_financials::auth::tests::tenant_binding_rejects_paths_ports_and_empty_scope",
    ),
    CertificationEvidence::CompilerContract(
        "appfw_provider_oracle_financials::provider::tests::provider_plans_named_reads_as_shared_saas_request_without_network",
    ),
];
const ORACLE_FINANCIALS_SCHEMA_VERSION_EVIDENCE: &[CertificationEvidence] = &[
    CertificationEvidence::CompilerContract(
        "appfw_provider_oracle_financials::identity::tests::descriptor_uses_oracle_financials_provider_key_and_api_pin",
    ),
    CertificationEvidence::CompilerContract(
        "appfw_provider_oracle_financials::metadata::tests::api_snapshot_metadata_pins_oracle_financials_openapi_release",
    ),
    CertificationEvidence::CompilerContract(
        "appfw_provider_oracle_financials::operation::tests::resources_base_path_uses_oracle_fusion_rest_resource_root",
    ),
];
const ORACLE_FINANCIALS_RESULT_CAPS_EVIDENCE: &[CertificationEvidence] = &[
    CertificationEvidence::CompilerContract(
        "appfw_provider_oracle_financials::registry::tests::accounting_period_status_lov_list_converts_to_fixed_saas_request_plan",
    ),
    CertificationEvidence::CompilerContract(
        "appfw_provider_oracle_financials::registry::tests::accounting_period_status_lov_list_caps_page_limit",
    ),
];
const ORACLE_FINANCIALS_GOVERNED_WRITE_EVIDENCE: &[CertificationEvidence] =
    &[CertificationEvidence::CompilerContract(
        "appfw_provider_oracle_financials::provider::tests::provider_rejects_mutation_planning_for_now",
    )];

const SERVICE_NOW_SAAS_CAPABILITIES: [SaasReadCapability; SAAS_AREA_COUNT] =
    planned_saas_capabilities(SERVICE_NOW_AUTHENTICATED_EXPORT_REQUIRED_REASON);
const WORKDAY_SAAS_CAPABILITIES: [SaasReadCapability; SAAS_AREA_COUNT] = [
    saas_unsupported(
        SaasReadArea::ConnectionAuth,
        SAAS_LIVE_EVIDENCE_REQUIRED_REASON,
    ),
    saas_compiler_contracted(
        SaasReadArea::NamedOperationRegistry,
        WORKDAY_REGISTRY_EVIDENCE,
    ),
    saas_compiler_contracted(
        SaasReadArea::RequestBinding,
        WORKDAY_REQUEST_BINDING_EVIDENCE,
    ),
    saas_compiler_contracted(
        SaasReadArea::PaginationCursoring,
        WORKDAY_PAGINATION_EVIDENCE,
    ),
    saas_unsupported(
        SaasReadArea::RateLimitBackoff,
        SAAS_LIVE_EVIDENCE_REQUIRED_REASON,
    ),
    saas_compiler_contracted(
        SaasReadArea::IncrementalWatermark,
        WORKDAY_INCREMENTAL_WATERMARK_EVIDENCE,
    ),
    saas_unsupported(
        SaasReadArea::FieldRedaction,
        SAAS_LIVE_EVIDENCE_REQUIRED_REASON,
    ),
    saas_compiler_contracted(SaasReadArea::TenantScoping, WORKDAY_TENANT_SCOPING_EVIDENCE),
    saas_compiler_contracted(
        SaasReadArea::SchemaVersionPinning,
        WORKDAY_SCHEMA_VERSION_EVIDENCE,
    ),
    saas_compiler_contracted(
        SaasReadArea::ResultAndTimeoutCaps,
        WORKDAY_RESULT_CAPS_EVIDENCE,
    ),
    saas_unsupported(
        SaasReadArea::QueryMetricsAndAudit,
        SAAS_LIVE_EVIDENCE_REQUIRED_REASON,
    ),
    saas_unsupported_with_evidence(
        SaasReadArea::FreshnessReporting,
        SAAS_LIVE_EVIDENCE_REQUIRED_REASON,
        WORKDAY_FRESHNESS_EVIDENCE,
    ),
    saas_unsupported_with_evidence(
        SaasReadArea::GovernedWriteEnforcement,
        SAAS_GOVERNED_WRITE_UNSUPPORTED_REASON,
        WORKDAY_GOVERNED_WRITE_EVIDENCE,
    ),
    saas_unsupported(
        SaasReadArea::DelegatedActorContext,
        SAAS_DELEGATED_ACTOR_UNSUPPORTED_REASON,
    ),
    saas_unsupported(
        SaasReadArea::TokenStoreIsolation,
        SAAS_TOKEN_STORE_UNSUPPORTED_REASON,
    ),
    saas_unsupported(
        SaasReadArea::NamedMutationRegistry,
        SAAS_NAMED_MUTATION_UNSUPPORTED_REASON,
    ),
    saas_unsupported(
        SaasReadArea::MutationRequestBinding,
        SAAS_MUTATION_BINDING_UNSUPPORTED_REASON,
    ),
    saas_unsupported(
        SaasReadArea::IdempotencyAndReplayProtection,
        SAAS_IDEMPOTENCY_UNSUPPORTED_REASON,
    ),
    saas_unsupported(
        SaasReadArea::WritePolicyAndScopeEnforcement,
        SAAS_WRITE_POLICY_UNSUPPORTED_REASON,
    ),
    saas_unsupported(
        SaasReadArea::WriteAuditAndEvidence,
        SAAS_WRITE_AUDIT_UNSUPPORTED_REASON,
    ),
];
const ICIMS_SAAS_CAPABILITIES: [SaasReadCapability; SAAS_AREA_COUNT] =
    planned_saas_capabilities(ICIMS_AUTHENTICATED_DOCS_REQUIRED_REASON);
const ANAPLAN_SAAS_CAPABILITIES: [SaasReadCapability; SAAS_AREA_COUNT] = [
    saas_unsupported(
        SaasReadArea::ConnectionAuth,
        SAAS_LIVE_EVIDENCE_REQUIRED_REASON,
    ),
    saas_compiler_contracted(
        SaasReadArea::NamedOperationRegistry,
        ANAPLAN_REGISTRY_EVIDENCE,
    ),
    saas_compiler_contracted(
        SaasReadArea::RequestBinding,
        ANAPLAN_REQUEST_BINDING_EVIDENCE,
    ),
    saas_compiler_contracted(
        SaasReadArea::PaginationCursoring,
        ANAPLAN_PAGINATION_EVIDENCE,
    ),
    saas_unsupported(
        SaasReadArea::RateLimitBackoff,
        SAAS_LIVE_EVIDENCE_REQUIRED_REASON,
    ),
    saas_unsupported(
        SaasReadArea::IncrementalWatermark,
        ANAPLAN_TENANT_OPERATION_SCOPE_REQUIRED_REASON,
    ),
    saas_unsupported(
        SaasReadArea::FieldRedaction,
        SAAS_LIVE_EVIDENCE_REQUIRED_REASON,
    ),
    saas_compiler_contracted(SaasReadArea::TenantScoping, ANAPLAN_TENANT_SCOPING_EVIDENCE),
    saas_compiler_contracted(
        SaasReadArea::SchemaVersionPinning,
        ANAPLAN_SCHEMA_VERSION_EVIDENCE,
    ),
    saas_compiler_contracted(
        SaasReadArea::ResultAndTimeoutCaps,
        ANAPLAN_RESULT_CAPS_EVIDENCE,
    ),
    saas_unsupported(
        SaasReadArea::QueryMetricsAndAudit,
        SAAS_LIVE_EVIDENCE_REQUIRED_REASON,
    ),
    saas_unsupported(
        SaasReadArea::FreshnessReporting,
        SAAS_LIVE_EVIDENCE_REQUIRED_REASON,
    ),
    saas_unsupported_with_evidence(
        SaasReadArea::GovernedWriteEnforcement,
        SAAS_GOVERNED_WRITE_UNSUPPORTED_REASON,
        ANAPLAN_GOVERNED_WRITE_EVIDENCE,
    ),
    saas_unsupported(
        SaasReadArea::DelegatedActorContext,
        SAAS_DELEGATED_ACTOR_UNSUPPORTED_REASON,
    ),
    saas_unsupported(
        SaasReadArea::TokenStoreIsolation,
        SAAS_TOKEN_STORE_UNSUPPORTED_REASON,
    ),
    saas_unsupported(
        SaasReadArea::NamedMutationRegistry,
        SAAS_NAMED_MUTATION_UNSUPPORTED_REASON,
    ),
    saas_unsupported(
        SaasReadArea::MutationRequestBinding,
        SAAS_MUTATION_BINDING_UNSUPPORTED_REASON,
    ),
    saas_unsupported(
        SaasReadArea::IdempotencyAndReplayProtection,
        SAAS_IDEMPOTENCY_UNSUPPORTED_REASON,
    ),
    saas_unsupported(
        SaasReadArea::WritePolicyAndScopeEnforcement,
        SAAS_WRITE_POLICY_UNSUPPORTED_REASON,
    ),
    saas_unsupported(
        SaasReadArea::WriteAuditAndEvidence,
        SAAS_WRITE_AUDIT_UNSUPPORTED_REASON,
    ),
];
const ORACLE_FINANCIALS_SAAS_CAPABILITIES: [SaasReadCapability; SAAS_AREA_COUNT] = [
    saas_unsupported(
        SaasReadArea::ConnectionAuth,
        SAAS_LIVE_EVIDENCE_REQUIRED_REASON,
    ),
    saas_compiler_contracted(
        SaasReadArea::NamedOperationRegistry,
        ORACLE_FINANCIALS_REGISTRY_EVIDENCE,
    ),
    saas_compiler_contracted(
        SaasReadArea::RequestBinding,
        ORACLE_FINANCIALS_REQUEST_BINDING_EVIDENCE,
    ),
    saas_compiler_contracted(
        SaasReadArea::PaginationCursoring,
        ORACLE_FINANCIALS_PAGINATION_EVIDENCE,
    ),
    saas_unsupported(
        SaasReadArea::RateLimitBackoff,
        SAAS_LIVE_EVIDENCE_REQUIRED_REASON,
    ),
    saas_unsupported(
        SaasReadArea::IncrementalWatermark,
        ORACLE_FINANCIALS_OPERATION_SCOPE_REQUIRED_REASON,
    ),
    saas_unsupported(
        SaasReadArea::FieldRedaction,
        SAAS_LIVE_EVIDENCE_REQUIRED_REASON,
    ),
    saas_compiler_contracted(
        SaasReadArea::TenantScoping,
        ORACLE_FINANCIALS_TENANT_SCOPING_EVIDENCE,
    ),
    saas_compiler_contracted(
        SaasReadArea::SchemaVersionPinning,
        ORACLE_FINANCIALS_SCHEMA_VERSION_EVIDENCE,
    ),
    saas_compiler_contracted(
        SaasReadArea::ResultAndTimeoutCaps,
        ORACLE_FINANCIALS_RESULT_CAPS_EVIDENCE,
    ),
    saas_unsupported(
        SaasReadArea::QueryMetricsAndAudit,
        SAAS_LIVE_EVIDENCE_REQUIRED_REASON,
    ),
    saas_unsupported(
        SaasReadArea::FreshnessReporting,
        SAAS_LIVE_EVIDENCE_REQUIRED_REASON,
    ),
    saas_unsupported_with_evidence(
        SaasReadArea::GovernedWriteEnforcement,
        SAAS_GOVERNED_WRITE_UNSUPPORTED_REASON,
        ORACLE_FINANCIALS_GOVERNED_WRITE_EVIDENCE,
    ),
    saas_unsupported(
        SaasReadArea::DelegatedActorContext,
        SAAS_DELEGATED_ACTOR_UNSUPPORTED_REASON,
    ),
    saas_unsupported(
        SaasReadArea::TokenStoreIsolation,
        SAAS_TOKEN_STORE_UNSUPPORTED_REASON,
    ),
    saas_unsupported(
        SaasReadArea::NamedMutationRegistry,
        SAAS_NAMED_MUTATION_UNSUPPORTED_REASON,
    ),
    saas_unsupported(
        SaasReadArea::MutationRequestBinding,
        SAAS_MUTATION_BINDING_UNSUPPORTED_REASON,
    ),
    saas_unsupported(
        SaasReadArea::IdempotencyAndReplayProtection,
        SAAS_IDEMPOTENCY_UNSUPPORTED_REASON,
    ),
    saas_unsupported(
        SaasReadArea::WritePolicyAndScopeEnforcement,
        SAAS_WRITE_POLICY_UNSUPPORTED_REASON,
    ),
    saas_unsupported(
        SaasReadArea::WriteAuditAndEvidence,
        SAAS_WRITE_AUDIT_UNSUPPORTED_REASON,
    ),
];
const SALESFORCE_SAAS_CAPABILITIES: [SaasReadCapability; SAAS_AREA_COUNT] = [
    saas_unsupported(
        SaasReadArea::ConnectionAuth,
        SAAS_LIVE_EVIDENCE_REQUIRED_REASON,
    ),
    saas_compiler_contracted(
        SaasReadArea::NamedOperationRegistry,
        SALESFORCE_REGISTRY_EVIDENCE,
    ),
    saas_compiler_contracted(
        SaasReadArea::RequestBinding,
        SALESFORCE_REQUEST_BINDING_EVIDENCE,
    ),
    saas_compiler_contracted(
        SaasReadArea::PaginationCursoring,
        SALESFORCE_PAGINATION_EVIDENCE,
    ),
    saas_unsupported_with_evidence(
        SaasReadArea::RateLimitBackoff,
        SALESFORCE_RATE_LIMIT_BACKOFF_PENDING_REASON,
        SALESFORCE_RATE_LIMIT_EVIDENCE,
    ),
    saas_compiler_contracted(
        SaasReadArea::IncrementalWatermark,
        SALESFORCE_INCREMENTAL_WATERMARK_EVIDENCE,
    ),
    saas_compiler_contracted(
        SaasReadArea::FieldRedaction,
        SALESFORCE_FIELD_REDACTION_EVIDENCE,
    ),
    saas_compiler_contracted(
        SaasReadArea::TenantScoping,
        SALESFORCE_TENANT_SCOPING_EVIDENCE,
    ),
    saas_compiler_contracted(
        SaasReadArea::SchemaVersionPinning,
        SALESFORCE_SCHEMA_VERSION_EVIDENCE,
    ),
    saas_compiler_contracted(
        SaasReadArea::ResultAndTimeoutCaps,
        SALESFORCE_RESULT_CAPS_EVIDENCE,
    ),
    saas_unsupported(
        SaasReadArea::QueryMetricsAndAudit,
        SAAS_LIVE_EVIDENCE_REQUIRED_REASON,
    ),
    saas_unsupported(
        SaasReadArea::FreshnessReporting,
        SAAS_LIVE_EVIDENCE_REQUIRED_REASON,
    ),
    saas_unsupported_with_evidence(
        SaasReadArea::GovernedWriteEnforcement,
        SAAS_GOVERNED_WRITE_UNSUPPORTED_REASON,
        SALESFORCE_GOVERNED_WRITE_EVIDENCE,
    ),
    saas_unsupported(
        SaasReadArea::DelegatedActorContext,
        SAAS_DELEGATED_ACTOR_UNSUPPORTED_REASON,
    ),
    saas_unsupported(
        SaasReadArea::TokenStoreIsolation,
        SAAS_TOKEN_STORE_UNSUPPORTED_REASON,
    ),
    saas_unsupported(
        SaasReadArea::NamedMutationRegistry,
        SAAS_NAMED_MUTATION_UNSUPPORTED_REASON,
    ),
    saas_unsupported(
        SaasReadArea::MutationRequestBinding,
        SAAS_MUTATION_BINDING_UNSUPPORTED_REASON,
    ),
    saas_unsupported(
        SaasReadArea::IdempotencyAndReplayProtection,
        SAAS_IDEMPOTENCY_UNSUPPORTED_REASON,
    ),
    saas_unsupported(
        SaasReadArea::WritePolicyAndScopeEnforcement,
        SAAS_WRITE_POLICY_UNSUPPORTED_REASON,
    ),
    saas_unsupported(
        SaasReadArea::WriteAuditAndEvidence,
        SAAS_WRITE_AUDIT_UNSUPPORTED_REASON,
    ),
];

const SERVICE_NOW_SAAS_PROFILE: SaasReadSemanticProfile<FrameworkProvider> =
    SaasReadSemanticProfile {
        provider: FrameworkProvider::ServiceNow,
        capabilities: &SERVICE_NOW_SAAS_CAPABILITIES,
    };
const WORKDAY_SAAS_PROFILE: SaasReadSemanticProfile<FrameworkProvider> = SaasReadSemanticProfile {
    provider: FrameworkProvider::Workday,
    capabilities: &WORKDAY_SAAS_CAPABILITIES,
};
const ICIMS_SAAS_PROFILE: SaasReadSemanticProfile<FrameworkProvider> = SaasReadSemanticProfile {
    provider: FrameworkProvider::Icims,
    capabilities: &ICIMS_SAAS_CAPABILITIES,
};
const SALESFORCE_SAAS_PROFILE: SaasReadSemanticProfile<FrameworkProvider> =
    SaasReadSemanticProfile {
        provider: FrameworkProvider::Salesforce,
        capabilities: &SALESFORCE_SAAS_CAPABILITIES,
    };
const ANAPLAN_SAAS_PROFILE: SaasReadSemanticProfile<FrameworkProvider> = SaasReadSemanticProfile {
    provider: FrameworkProvider::Anaplan,
    capabilities: &ANAPLAN_SAAS_CAPABILITIES,
};
const ORACLE_FINANCIALS_SAAS_PROFILE: SaasReadSemanticProfile<FrameworkProvider> =
    SaasReadSemanticProfile {
        provider: FrameworkProvider::OracleFinancials,
        capabilities: &ORACLE_FINANCIALS_SAAS_CAPABILITIES,
    };

/// SaaS/external-API certification profile. Panics for non-external-API
/// providers (CRUD providers use [`semantic_profile`], graph providers use
/// [`graph_read_profile`]).
pub fn saas_read_profile(
    provider: FrameworkProvider,
) -> &'static SaasReadSemanticProfile<FrameworkProvider> {
    match provider {
        FrameworkProvider::ServiceNow => &SERVICE_NOW_SAAS_PROFILE,
        FrameworkProvider::Workday => &WORKDAY_SAAS_PROFILE,
        FrameworkProvider::Icims => &ICIMS_SAAS_PROFILE,
        FrameworkProvider::Salesforce => &SALESFORCE_SAAS_PROFILE,
        FrameworkProvider::Anaplan => &ANAPLAN_SAAS_PROFILE,
        FrameworkProvider::OracleFinancials => &ORACLE_FINANCIALS_SAAS_PROFILE,
        other => {
            panic!("{other:?} is not an external API provider; use the provider-specific profile")
        }
    }
}

const fn planned_saas_capabilities(reason: &'static str) -> [SaasReadCapability; SAAS_AREA_COUNT] {
    [
        saas_unsupported(SaasReadArea::ConnectionAuth, reason),
        saas_unsupported(SaasReadArea::NamedOperationRegistry, reason),
        saas_unsupported(SaasReadArea::RequestBinding, reason),
        saas_unsupported(SaasReadArea::PaginationCursoring, reason),
        saas_unsupported(SaasReadArea::RateLimitBackoff, reason),
        saas_unsupported(SaasReadArea::IncrementalWatermark, reason),
        saas_unsupported(SaasReadArea::FieldRedaction, reason),
        saas_unsupported(SaasReadArea::TenantScoping, reason),
        saas_unsupported(SaasReadArea::SchemaVersionPinning, reason),
        saas_unsupported(SaasReadArea::ResultAndTimeoutCaps, reason),
        saas_unsupported(SaasReadArea::QueryMetricsAndAudit, reason),
        saas_unsupported(SaasReadArea::FreshnessReporting, reason),
        saas_unsupported(SaasReadArea::GovernedWriteEnforcement, reason),
        saas_unsupported(SaasReadArea::DelegatedActorContext, reason),
        saas_unsupported(SaasReadArea::TokenStoreIsolation, reason),
        saas_unsupported(SaasReadArea::NamedMutationRegistry, reason),
        saas_unsupported(SaasReadArea::MutationRequestBinding, reason),
        saas_unsupported(SaasReadArea::IdempotencyAndReplayProtection, reason),
        saas_unsupported(SaasReadArea::WritePolicyAndScopeEnforcement, reason),
        saas_unsupported(SaasReadArea::WriteAuditAndEvidence, reason),
    ]
}

const fn graph_compiler_contracted(
    area: GraphReadArea,
    evidence: &'static [CertificationEvidence],
) -> GraphReadCapability {
    GraphReadCapability {
        area,
        status: CapabilityStatus::CompilerContracted,
        evidence,
    }
}

const fn saas_unsupported(area: SaasReadArea, reason: &'static str) -> SaasReadCapability {
    SaasReadCapability {
        area,
        status: CapabilityStatus::Unsupported(reason),
        evidence: &[],
    }
}

const fn saas_unsupported_with_evidence(
    area: SaasReadArea,
    reason: &'static str,
    evidence: &'static [CertificationEvidence],
) -> SaasReadCapability {
    SaasReadCapability {
        area,
        status: CapabilityStatus::Unsupported(reason),
        evidence,
    }
}

const fn saas_compiler_contracted(
    area: SaasReadArea,
    evidence: &'static [CertificationEvidence],
) -> SaasReadCapability {
    SaasReadCapability {
        area,
        status: CapabilityStatus::CompilerContracted,
        evidence,
    }
}

pub fn capability(provider: FrameworkProvider, area: ProviderContractArea) -> CapabilityStatus {
    provider_capability(provider, area).status
}

pub fn provider_capability(
    provider: FrameworkProvider,
    area: ProviderContractArea,
) -> &'static ProviderCapability {
    semantic_profile(provider)
        .capabilities
        .iter()
        .find(|capability| capability.area == area)
        .expect("semantic profiles must declare every contract area")
}

const fn implemented(
    area: ProviderContractArea,
    reason: &'static str,
    evidence: &'static [CertificationEvidence],
) -> ProviderCapability {
    ProviderCapability {
        area,
        status: CapabilityStatus::Implemented(reason),
        evidence,
    }
}

const fn compiler_contracted(
    area: ProviderContractArea,
    evidence: &'static [CertificationEvidence],
) -> ProviderCapability {
    ProviderCapability {
        area,
        status: CapabilityStatus::CompilerContracted,
        evidence,
    }
}

const fn live_certified(
    area: ProviderContractArea,
    evidence: &'static [CertificationEvidence],
) -> ProviderCapability {
    ProviderCapability {
        area,
        status: CapabilityStatus::LiveCertified,
        evidence,
    }
}

const fn partial(
    area: ProviderContractArea,
    reason: &'static str,
    evidence: &'static [CertificationEvidence],
) -> ProviderCapability {
    ProviderCapability {
        area,
        status: CapabilityStatus::Partial(reason),
        evidence,
    }
}

const fn unsupported(
    area: ProviderContractArea,
    reason: &'static str,
    evidence: &'static [CertificationEvidence],
) -> ProviderCapability {
    ProviderCapability {
        area,
        status: CapabilityStatus::Unsupported(reason),
        evidence,
    }
}

const fn emulator_limited(
    area: ProviderContractArea,
    reason: &'static str,
    evidence: &'static [CertificationEvidence],
) -> ProviderCapability {
    ProviderCapability {
        area,
        status: CapabilityStatus::EmulatorLimited(reason),
        evidence,
    }
}

const fn executable_compiler(area: ProviderContractArea, name: &'static str) -> ExecutableContract {
    ExecutableContract {
        area,
        kind: ExecutableContractKind::Compiler,
        name,
    }
}

const fn executable_live(area: ProviderContractArea, name: &'static str) -> ExecutableContract {
    ExecutableContract {
        area,
        kind: ExecutableContractKind::Live,
        name,
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::*;

    #[test]
    fn provider_certification_profiles_cover_every_contract_area() {
        for provider in SUPPORTED_PROVIDERS {
            let profile = semantic_profile(provider);
            assert_eq!(profile.provider, provider);
            assert_eq!(profile.capabilities.len(), ProviderContractArea::ALL.len());

            let declared: HashSet<_> = profile
                .capabilities
                .iter()
                .map(|capability| capability.area)
                .collect();
            let keys: HashSet<_> = profile
                .capabilities
                .iter()
                .map(|capability| capability.area.key())
                .collect();
            assert_eq!(
                declared.len(),
                ProviderContractArea::ALL.len(),
                "{provider:?} must not declare duplicate certification areas"
            );
            assert_eq!(
                keys.len(),
                ProviderContractArea::ALL.len(),
                "{provider:?} must not declare duplicate certification area keys"
            );

            for area in ProviderContractArea::ALL {
                assert!(
                    declared.contains(&area),
                    "{provider:?} missing {}",
                    area.label()
                );
            }
        }
    }

    #[test]
    fn neo4j_graph_read_profile_covers_every_graph_area() {
        let profile = graph_read_profile(FrameworkProvider::Neo4j);
        assert_eq!(profile.provider, FrameworkProvider::Neo4j);
        assert_eq!(profile.capabilities.len(), GraphReadArea::ALL.len());

        let declared: HashSet<_> = profile
            .capabilities
            .iter()
            .map(|capability| capability.area)
            .collect();
        let keys: HashSet<_> = profile
            .capabilities
            .iter()
            .map(|capability| capability.area.key())
            .collect();
        assert_eq!(declared.len(), GraphReadArea::ALL.len());
        assert_eq!(keys.len(), GraphReadArea::ALL.len());

        for area in GraphReadArea::ALL {
            assert!(
                declared.contains(&area),
                "missing graph area {}",
                area.label()
            );
        }

        for capability in profile.capabilities {
            assert_eq!(capability.status, CapabilityStatus::CompilerContracted);
            assert!(
                capability
                    .evidence
                    .iter()
                    .any(|evidence| evidence.is_compiler()),
                "graph area {} is compiler-contracted without a compiler contract",
                capability.area.label()
            );
            for evidence in capability.evidence {
                assert!(
                    !evidence.contract().trim().is_empty(),
                    "graph area {} has empty evidence",
                    capability.area.label()
                );
            }
        }
    }

    #[test]
    fn external_api_profiles_cover_every_saas_area_without_crud_parity() {
        for provider in FrameworkProvider::EXTERNAL_API {
            assert!(!SUPPORTED_PROVIDERS.contains(&provider));

            let profile = saas_read_profile(provider);
            assert_eq!(profile.provider, provider);
            assert_eq!(profile.capabilities.len(), SaasReadArea::ALL.len());

            let declared: HashSet<_> = profile
                .capabilities
                .iter()
                .map(|capability| capability.area)
                .collect();
            let keys: HashSet<_> = profile
                .capabilities
                .iter()
                .map(|capability| capability.area.key())
                .collect();
            assert_eq!(declared.len(), SaasReadArea::ALL.len());
            assert_eq!(keys.len(), SaasReadArea::ALL.len());

            for area in SaasReadArea::ALL {
                assert!(
                    declared.contains(&area),
                    "missing SaaS area {}",
                    area.label()
                );
            }

            for capability in profile.capabilities {
                for evidence in capability.evidence {
                    assert_eq!(
                        evidence.contract(),
                        evidence.contract().trim(),
                        "{provider:?} {} {} `{}` must not carry padding",
                        capability.area.label(),
                        evidence.label(),
                        evidence.contract()
                    );
                    assert!(
                        evidence.is_compiler(),
                        "{provider:?} {} must not claim live evidence before tenant certification",
                        capability.area.label()
                    );
                }
            }

            match provider {
                FrameworkProvider::Salesforce
                | FrameworkProvider::Workday
                | FrameworkProvider::Anaplan
                | FrameworkProvider::OracleFinancials => {
                    let promoted_areas = match provider {
                        FrameworkProvider::Anaplan | FrameworkProvider::OracleFinancials => vec![
                            SaasReadArea::NamedOperationRegistry,
                            SaasReadArea::RequestBinding,
                            SaasReadArea::PaginationCursoring,
                            SaasReadArea::TenantScoping,
                            SaasReadArea::SchemaVersionPinning,
                            SaasReadArea::ResultAndTimeoutCaps,
                        ],
                        FrameworkProvider::Salesforce | FrameworkProvider::Workday => vec![
                            SaasReadArea::NamedOperationRegistry,
                            SaasReadArea::RequestBinding,
                            SaasReadArea::PaginationCursoring,
                            SaasReadArea::IncrementalWatermark,
                            SaasReadArea::SchemaVersionPinning,
                            SaasReadArea::ResultAndTimeoutCaps,
                        ],
                        _ => unreachable!("outer match limits provider"),
                    };
                    for area in promoted_areas {
                        let capability = profile
                            .capabilities
                            .iter()
                            .find(|capability| capability.area == area)
                            .expect("promoted SaaS area");
                        assert_eq!(capability.status, CapabilityStatus::CompilerContracted);
                        assert!(
                            !capability.evidence.is_empty(),
                            "{provider:?} {} must cite compiler evidence",
                            capability.area.label()
                        );
                    }

                    let governed_write = profile
                        .capabilities
                        .iter()
                        .find(|capability| {
                            capability.area == SaasReadArea::GovernedWriteEnforcement
                        })
                        .expect("governed write area");
                    assert!(matches!(
                        governed_write.status,
                        CapabilityStatus::Unsupported(reason) if reason.contains("writes remain unsupported")
                    ));
                    assert!(
                        !governed_write.evidence.is_empty(),
                        "{provider:?} governed write unsupported posture should cite mutation rejection evidence"
                    );

                    let field_redaction = profile
                        .capabilities
                        .iter()
                        .find(|capability| capability.area == SaasReadArea::FieldRedaction)
                        .expect("field redaction area");
                    match provider {
                        FrameworkProvider::Salesforce => {
                            assert_eq!(
                                field_redaction.status,
                                CapabilityStatus::CompilerContracted
                            );
                            assert!(field_redaction.evidence.iter().any(|evidence| {
                                evidence
                                    .contract()
                                    .contains("query_response_json_redacts_records")
                            }));
                        }
                        FrameworkProvider::Workday => {
                            assert!(matches!(
                                field_redaction.status,
                                CapabilityStatus::Unsupported(reason)
                                    if reason.contains("requires live tenant evidence")
                            ));
                        }
                        FrameworkProvider::Anaplan => {
                            assert!(matches!(
                                field_redaction.status,
                                CapabilityStatus::Unsupported(reason)
                                    if reason.contains("requires live tenant evidence")
                            ));
                        }
                        FrameworkProvider::OracleFinancials => {
                            assert!(matches!(
                                field_redaction.status,
                                CapabilityStatus::Unsupported(reason)
                                    if reason.contains("requires live tenant evidence")
                            ));
                        }
                        _ => unreachable!("outer match limits provider"),
                    }

                    let rate_limit = profile
                        .capabilities
                        .iter()
                        .find(|capability| capability.area == SaasReadArea::RateLimitBackoff)
                        .expect("rate limit area");
                    match provider {
                        FrameworkProvider::Salesforce => {
                            assert!(matches!(
                                rate_limit.status,
                                CapabilityStatus::Unsupported(reason)
                                    if reason.contains("rate-limit signals and retry decisions are compiler-contracted")
                            ));
                            assert!(rate_limit.evidence.iter().any(|evidence| {
                                evidence
                                    .contract()
                                    .contains("rate_limit_signals_do_not_expose_header_values")
                            }));
                        }
                        FrameworkProvider::Workday => {
                            assert!(matches!(
                                rate_limit.status,
                                CapabilityStatus::Unsupported(reason)
                                    if reason.contains("requires live tenant evidence")
                            ));
                            assert!(rate_limit.evidence.is_empty());
                        }
                        FrameworkProvider::Anaplan => {
                            assert!(matches!(
                                rate_limit.status,
                                CapabilityStatus::Unsupported(reason)
                                    if reason.contains("requires live tenant evidence")
                            ));
                            assert!(rate_limit.evidence.is_empty());
                        }
                        FrameworkProvider::OracleFinancials => {
                            assert!(matches!(
                                rate_limit.status,
                                CapabilityStatus::Unsupported(reason)
                                    if reason.contains("requires live tenant evidence")
                            ));
                            assert!(rate_limit.evidence.is_empty());
                        }
                        _ => unreachable!("outer match limits provider"),
                    }

                    let incremental_watermark = profile
                        .capabilities
                        .iter()
                        .find(|capability| capability.area == SaasReadArea::IncrementalWatermark)
                        .expect("incremental watermark area");
                    match provider {
                        FrameworkProvider::Anaplan => {
                            assert!(matches!(
                                incremental_watermark.status,
                                CapabilityStatus::Unsupported(reason)
                                    if reason.contains("tenant-approved Anaplan auth")
                            ));
                            assert!(incremental_watermark.evidence.is_empty());
                        }
                        FrameworkProvider::OracleFinancials => {
                            assert!(matches!(
                                incremental_watermark.status,
                                CapabilityStatus::Unsupported(reason)
                                    if reason.contains("Oracle Financials tenant auth")
                            ));
                            assert!(incremental_watermark.evidence.is_empty());
                        }
                        FrameworkProvider::Salesforce | FrameworkProvider::Workday => {}
                        _ => unreachable!("outer match limits provider"),
                    }

                    let tenant_scoping = profile
                        .capabilities
                        .iter()
                        .find(|capability| capability.area == SaasReadArea::TenantScoping)
                        .expect("tenant scoping area");
                    match provider {
                        FrameworkProvider::Salesforce => {
                            assert_eq!(tenant_scoping.status, CapabilityStatus::CompilerContracted);
                            assert!(tenant_scoping.evidence.iter().any(|evidence| {
                                evidence
                                    .contract()
                                    .contains("tenant_binding_rejects_paths_queries")
                            }));
                        }
                        FrameworkProvider::Workday => {
                            assert_eq!(tenant_scoping.status, CapabilityStatus::CompilerContracted);
                            assert!(tenant_scoping.evidence.iter().any(|evidence| {
                                evidence
                                    .contract()
                                    .contains("provider_plans_named_reads_as_shared_saas_request")
                            }));
                        }
                        FrameworkProvider::Anaplan => {
                            assert_eq!(tenant_scoping.status, CapabilityStatus::CompilerContracted);
                            assert!(tenant_scoping.evidence.iter().any(|evidence| {
                                evidence
                                    .contract()
                                    .contains("tenant_binding_rejects_paths_queries")
                            }));
                        }
                        FrameworkProvider::OracleFinancials => {
                            assert_eq!(tenant_scoping.status, CapabilityStatus::CompilerContracted);
                            assert!(tenant_scoping.evidence.iter().any(|evidence| {
                                evidence
                                    .contract()
                                    .contains("tenant_binding_rejects_paths_ports")
                            }));
                        }
                        _ => unreachable!("outer match limits provider"),
                    }

                    let freshness = profile
                        .capabilities
                        .iter()
                        .find(|capability| capability.area == SaasReadArea::FreshnessReporting)
                        .expect("freshness area");
                    match provider {
                        FrameworkProvider::Salesforce => {
                            assert!(matches!(
                                freshness.status,
                                CapabilityStatus::Unsupported(reason)
                                    if reason.contains("requires live tenant evidence")
                            ));
                            assert!(freshness.evidence.is_empty());
                        }
                        FrameworkProvider::Workday => {
                            assert!(matches!(
                                freshness.status,
                                CapabilityStatus::Unsupported(reason)
                                    if reason.contains("requires live tenant evidence")
                            ));
                            assert!(freshness.evidence.iter().any(|evidence| {
                                evidence
                                    .contract()
                                    .contains("parses_workday_server_timestamp")
                            }));
                        }
                        FrameworkProvider::Anaplan => {
                            assert!(matches!(
                                freshness.status,
                                CapabilityStatus::Unsupported(reason)
                                    if reason.contains("requires live tenant evidence")
                            ));
                            assert!(freshness.evidence.is_empty());
                        }
                        FrameworkProvider::OracleFinancials => {
                            assert!(matches!(
                                freshness.status,
                                CapabilityStatus::Unsupported(reason)
                                    if reason.contains("requires live tenant evidence")
                            ));
                            assert!(freshness.evidence.is_empty());
                        }
                        _ => unreachable!("outer match limits provider"),
                    }
                }
                FrameworkProvider::ServiceNow | FrameworkProvider::Icims => {
                    for capability in profile.capabilities {
                        match provider {
                            FrameworkProvider::ServiceNow => assert!(matches!(
                                capability.status,
                                CapabilityStatus::Unsupported(reason)
                                    if reason.contains("authenticated ServiceNow instance OpenAPI")
                            )),
                            FrameworkProvider::Icims => assert!(matches!(
                                capability.status,
                                CapabilityStatus::Unsupported(reason)
                                    if reason.contains("authenticated iCIMS developer documentation")
                            )),
                            _ => unreachable!("outer match limits provider"),
                        }
                        assert!(capability.evidence.is_empty());
                    }
                }
                other => panic!("unexpected external API provider {other:?}"),
            }
        }
    }

    #[test]
    fn certification_statuses_have_required_evidence_or_reasons() {
        for provider in SUPPORTED_PROVIDERS {
            for capability in semantic_profile(provider).capabilities {
                if capability.status.requires_reason() {
                    assert!(
                        capability
                            .status
                            .reason()
                            .map(|reason| !reason.trim().is_empty())
                            .unwrap_or(false),
                        "{provider:?} {} must explain its certification gap",
                        capability.area.label()
                    );
                }

                match capability.status {
                    CapabilityStatus::LiveCertified => assert!(
                        capability
                            .evidence
                            .iter()
                            .any(|evidence| evidence.is_live()),
                        "{provider:?} {} is live-certified without a live contract",
                        capability.area.label()
                    ),
                    CapabilityStatus::CompilerContracted => assert!(
                        capability
                            .evidence
                            .iter()
                            .any(|evidence| evidence.is_compiler()),
                        "{provider:?} {} is compiler-contracted without a compiler contract",
                        capability.area.label()
                    ),
                    _ => {}
                }

                for evidence in capability.evidence {
                    assert!(
                        !evidence.contract().trim().is_empty(),
                        "{provider:?} {} has empty {} evidence",
                        capability.area.label(),
                        evidence.label()
                    );
                }
            }
        }

        assert!(capability(
            FrameworkProvider::Mongo,
            ProviderContractArea::RelationshipProjection
        )
        .is_live_certified());
        assert!(matches!(
            capability(
                FrameworkProvider::Snowflake,
                ProviderContractArea::Aggregation
            ),
            CapabilityStatus::EmulatorLimited(_)
        ));
        assert!(matches!(
            capability(
                FrameworkProvider::Postgres,
                ProviderContractArea::AccessFilters
            ),
            CapabilityStatus::LiveCertified
        ));
        assert!(matches!(
            capability(
                FrameworkProvider::Postgres,
                ProviderContractArea::TenantIsolation
            ),
            CapabilityStatus::LiveCertified
        ));
        assert!(matches!(
            capability(FrameworkProvider::Postgres, ProviderContractArea::Audit),
            CapabilityStatus::LiveCertified
        ));
        assert!(matches!(
            capability(
                FrameworkProvider::FabricSqlAnalytics,
                ProviderContractArea::ScalarFilters
            ),
            CapabilityStatus::CompilerContracted
        ));
        assert!(matches!(
            capability(
                FrameworkProvider::FabricSqlAnalytics,
                ProviderContractArea::ManyToManyMutation
            ),
            CapabilityStatus::Unsupported(reason) if reason.contains("read-only")
        ));
        assert!(matches!(
            capability(
                FrameworkProvider::FabricSqlAnalytics,
                ProviderContractArea::Audit
            ),
            CapabilityStatus::Unsupported(reason) if reason.contains("read-only")
        ));
    }

    #[test]
    fn certification_evidence_is_backed_by_executable_contract_registry() {
        let registry: HashSet<_> = EXECUTABLE_CONTRACTS
            .iter()
            .map(|contract| (contract.area, contract.kind, contract.name))
            .collect();

        for provider in SUPPORTED_PROVIDERS {
            for capability in semantic_profile(provider).capabilities {
                for evidence in capability.evidence {
                    let kind = match evidence {
                        CertificationEvidence::CompilerContract(_) => {
                            ExecutableContractKind::Compiler
                        }
                        CertificationEvidence::LiveContract(_) => ExecutableContractKind::Live,
                    };
                    assert!(
                        registry.contains(&(capability.area, kind, evidence.contract())),
                        "{provider:?} {} references evidence `{}` that is not in EXECUTABLE_CONTRACTS",
                        capability.area.label(),
                        evidence.contract()
                    );
                }

                if capability.status == CapabilityStatus::LiveCertified {
                    assert!(
                        EXECUTABLE_CONTRACTS.iter().any(|contract| {
                            contract.area == capability.area
                                && contract.kind == ExecutableContractKind::Live
                                && capability.evidence.iter().any(|evidence| {
                                    matches!(evidence, CertificationEvidence::LiveContract(name) if *name == contract.name)
                                })
                        }),
                        "{provider:?} {} is live-certified without registered live evidence",
                        capability.area.label()
                    );
                }
            }
        }
    }

    #[test]
    fn certification_contract_evidence_names_are_stable_and_unique() {
        let mut registry_entries = HashSet::new();
        for contract in EXECUTABLE_CONTRACTS {
            assert_eq!(
                contract.name,
                contract.name.trim(),
                "{} registry evidence `{}` must not carry padding",
                contract.area.label(),
                contract.name
            );
            assert!(
                !contract.name.is_empty(),
                "{} registry evidence names must not be empty",
                contract.area.label()
            );
            assert!(
                registry_entries.insert((contract.area, contract.kind, contract.name)),
                "{} registry evidence `{}` is duplicated",
                contract.area.label(),
                contract.name
            );
        }

        for provider in SUPPORTED_PROVIDERS {
            for capability in semantic_profile(provider).capabilities {
                let mut evidence_entries = HashSet::new();
                let mut live_contract_count = 0;
                for evidence in capability.evidence {
                    assert_eq!(
                        evidence.contract(),
                        evidence.contract().trim(),
                        "{provider:?} {} {} `{}` must not carry padding",
                        capability.area.label(),
                        evidence.label(),
                        evidence.contract()
                    );
                    assert!(
                        evidence_entries.insert((evidence.label(), evidence.contract())),
                        "{provider:?} {} {} `{}` is duplicated",
                        capability.area.label(),
                        evidence.label(),
                        evidence.contract()
                    );
                    if evidence.is_live() {
                        live_contract_count += 1;
                    }
                }
                assert!(
                    live_contract_count <= 1,
                    "{provider:?} {} declares multiple live contracts, but provider certification artifacts report a single live_contract per area",
                    capability.area.label()
                );
            }
        }
    }
}
