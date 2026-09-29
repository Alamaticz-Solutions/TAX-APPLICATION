# Runtime Ownership Inventory

This inventory is the packaging checkpoint for the remaining backend runtime
extraction after identity, auth, metadata, query cost, filter capability/data
type contracts, provider capabilities, and provider certification moved into
`appfw-runtime`. It names the framework-owned backend internals that still
appear inside product backend crates, where they should move, and what has to
be true before each move is safe.

Use this page before slimming `examples/products/crm/backend` or
`appfw new --profile crm-sample`. The former repository-root `backend`
validation fixture has been retired; product-relative paths such as
`backend/src/...` now refer to the CRM sample app root or another explicit
downstream app root. Template slimming should happen only after the runtime
contracts below exist and CRM proves them end-to-end.

## Classification

| State | Meaning |
| --- | --- |
| Extracted | Reusable behavior already lives in an upstream framework crate. |
| Facade | Product backend keeps a thin compatibility module over framework code. |
| Generated adapter | Code is generated from product config and should remain reviewable in product repos. |
| Product extension | Human-owned product handlers/services that survive regeneration. |
| Duplicated runtime | Framework behavior still copied into product backend crates. |

## Already Extracted Or Facaded

| Surface | Current Product Path | State | Target |
| --- | --- | --- | --- |
| Product identity contracts | `crate::product_api` plus direct runtime imports in framework-owned backend modules | Extracted | `appfw-runtime::extension`; no product `handlers/auth` source is required |
| Product extension aliases and guard boundary | `backend/src/product_api.rs` | Facade / Extracted | Thin product API facade over `appfw-runtime` and generated schema types; generic `RuntimeHandlerContext` and GraphQL user/data context extraction are runtime-owned, while product facades bind local `DataAccess`/`EntityType` and generated metadata lookup |
| CORS, GraphiQL shell, request limits, security, telemetry helpers, metrics, provider pool stats | Product-local CORS, GraphiQL, and observability facades are retired from the CRM sample; remaining generated route adapters are transitional product shape | Facade / Extracted | `appfw-runtime` |
| Runtime route shell and admin/MCP mount gating | `backend/src/routes/mod.rs` route assembly call | Partially extracted | `appfw-runtime::routing`; generated/product schema route adapters still local |
| MCP protocol constants, JSON-RPC envelopes, operation metadata | `backend/src/mcp/{protocol,operation}.rs` | Facade / Extracted | `appfw-runtime::mcp` |
| MCP operation helper adapters | `backend/src/mcp/generated_operations.rs` | Generated adapter | Argument decoding, result serialization, handler error mapping, operation resolution mapping, and recursive redaction are runtime-owned. Products keep only generated schema-specific operation dispatch and entity metadata redaction adapters. The previous product-local `backend/src/mcp/operation.rs` facade is retired. |
| Runtime error categories | `backend/src/routes/app_error.rs` compatibility facade | Facade / Extracted | `appfw-runtime::RuntimeError` and `appfw-runtime::RuntimeAppError`; backend `AppError` is a generated compatibility alias |
| App/auth state contract | generated routes, admin, MCP, and `main` | Extracted | `appfw-runtime::RuntimeAuthState`; product-local `app_state.rs` compatibility adapters are removed |
| Auth extraction and token verification | generated routes, admin, MCP, and product API context lookup | Extracted | `appfw-runtime::auth`; no product `jwt_extractor` or `okta_verifier` source is required |
| Policy access decision contract | Product-local `config/access.rs` facades are retired from the CRM sample; remaining generated config adapters are transitional product shape | Facade / Extracted | `appfw-runtime::access`; product config still evaluates policies, but allow/deny constructors, filter conjunction, and policy-result shape validation are runtime-owned |
| Pure record validation engine and provider validation hooks | `backend/src/data/validation.rs` and `backend/src/data/data_access.rs` adapter calls | Facade / Extracted | `appfw-runtime::record_validation` and `appfw-runtime::data_access`; product-shaped validation modules adapt generated schema structs while runtime owns pure validation plus primary-key, foreign-key, and uniqueness provider-hook result/error choreography |
| DateTime timezone record rule | Product-local `data/rules/timezone.rs` facades are retired from the CRM sample; remaining rule adapters are transitional product shape | Facade / Extracted | `appfw-runtime::record_timezone`; product-shaped rule modules adapt generated `PropertyType` and `UserAuth` values |
| Concurrency version record rule | Product-local `data/rules/version.rs` facades are retired from the CRM sample; remaining rule adapters are transitional product shape | Facade / Extracted | `appfw-runtime::record_version`; product-shaped rule modules adapt generated entity/property structs |
| Computed record rules | Product-local `data/rules/computed.rs` facades are retired from the CRM sample; remaining rule adapters are transitional product shape | Facade / Extracted | `appfw-runtime::record_computed`; product-shaped rule modules adapt generated `Computed` and `PropertyType` values |
| Metadata-driven audit helpers and event/query/chain contracts | Product-local `data/audit.rs` facades are retired from product templates | Extracted | `appfw-runtime::record_audit`; product call sites pass generated `EntityType` through runtime metadata adapters while runtime owns `RuntimeAuditEvent`, `RuntimeAuditQuery`, event construction, redaction, chain scope, evidence JSON, query table/limit derivation, record-chain continuation, and hash finalization |
| Provider capability profiles and executable evidence registry | Product-local provider-capability facades are retired from product templates | Extracted | `appfw-runtime::provider_capabilities`; products map generated data-source enums to `FrameworkProvider` through their product API boundary |
| Provider certification report semantics and SDK rule export | `appfw_runtime/src/provider_certification.rs`, `appfw_cli/src/bin/provider_certification_export.rs` | Extracted | `appfw-runtime::provider_certification` plus framework CLI exporter |
| Provider error classification and redaction | Product-local PostgreSQL provider-error facades are retired from the CRM sample; remaining provider adapters are transitional product shape | Facade / Extracted | `appfw-runtime::provider_error` plus provider packages; product-shaped provider modules keep only thin `AppError` adapters while runtime owns stable datastore classes, safe message normalization, provider labels, and redacted provider error logging, `appfw-provider-mongo` owns Mongo native driver error-code extraction, and `appfw-provider-postgres` owns PostgreSQL SQLSTATE/column extraction and native error conversion |
| Schema-neutral runtime metadata descriptors and lookup index | `backend/src/product_api.rs` adapter functions | Facade / Extracted | `appfw-runtime::model_metadata`; generated schema types stay product-side |
| Query cost budget and scoring | Product-local `data/query_cost.rs` facades are retired from the CRM sample; remaining query adapters are transitional product shape | Facade / Extracted | `appfw-runtime::query_cost`; product backend converts current generated `QueryPlan` shape into runtime cost inputs |
| Read-query DataAccess helper contracts | `backend/src/data/{data_access.rs,query_ir.rs}` adapter calls | Facade / Extracted | `appfw-runtime::data_access`; product-shaped DataAccess keeps auth, product rule closures, concrete providers, and plan construction while delegating query diagnostics, batch id filter planning, batch sizing, optional/list read provider execution wrappers, read-plan provider dispatch helpers, aggregate read provider execution wrappers, read result evaluation loops, provider count tracing callbacks, and keyset cursor/page finalization |
| Mutation DataAccess helper contracts | `backend/src/data/data_access.rs` adapter calls | Facade / Extracted | `appfw-runtime::data_access`; product-shaped DataAccess keeps auth, policy, validation, concrete providers, and mutation payload decisions while delegating mutation provider execution wrappers, mutation-plan provider dispatch helpers, mutation kind/counts, filtered-update denial checks, mutation audit record-id selection, delete audit outcome selection, and audit mutation/attempt append orchestration |
| Provider bridge vocabulary | `backend/src/data/{clients/database_client.rs,data_access.rs}` adapter calls | Facade / Extracted | `appfw-runtime::provider_bridge`; product-shaped provider clients keep concrete implementations while runtime owns provider descriptors, canonical operation tokens, overridable operation contracts, operation contract classification, unsupported explain diagnostics, pool-stat labels, and operation count normalization |
| Provider identity contract | `backend/src/data/clients/database_client.rs` supertrait plus provider impls | Facade / Extracted | `appfw-runtime::RuntimeProviderIdentity`; product-shaped providers expose canonical `FrameworkProvider` identity directly while generated `DataSourceType` stays in config/client factory adapters |
| Provider client base contract | `backend/src/data/clients/database_client.rs` supertrait | Facade / Extracted | `appfw-runtime::RuntimeProviderClient`; product `DatabaseClient` traits now hang from the runtime provider-client base and inherit canonical operation contract metadata |
| Provider result contracts and JSON conversion helpers | Product-local `data/json_utils.rs` facades are retired from product templates | Extracted | `appfw-runtime::provider_result` and `appfw-runtime::json`; runtime owns `RuntimeJsonObj`, `RuntimeJsonQueryResult`, `RuntimeJsonAggregateResult`, and generic GraphQL JSON object conversion while product-shaped call sites consume runtime helpers directly |
| Provider method input envelope | `backend/src/data/{data_access.rs,clients/*.rs}` primary plan method calls | Facade / Extracted | `appfw-runtime::RuntimeProviderPlanInput`; runtime owns the provider method request envelope over plan, user, and policy access |
| Provider mutation plan DTO | `backend/src/data/query_ir.rs` builder plus `DatabaseClient` mutation methods | Facade / Extracted | `appfw-runtime::RuntimeProviderMutationPlan`; product-shaped `MutationPlan` still parses generated metadata, then converts to the runtime provider mutation DTO before provider dispatch |
| Provider query plan DTO | `backend/src/data/query_ir.rs` builder plus `DatabaseClient` read methods | Facade / Extracted | `appfw-runtime::RuntimeProviderQueryPlan`; product-shaped `QueryPlan` still parses generated metadata, selection/filter/sort ASTs, and access filters, then converts to the runtime provider query DTO before provider dispatch |
| Provider aggregate plan DTO | `backend/src/data/query_ir.rs` builder plus `DatabaseClient` aggregate method | Facade / Extracted | `appfw-runtime::RuntimeProviderAggregatePlan`; product-shaped `AggregatePlan` still parses generated metadata, filters, groupings, metrics, having, sort ASTs, and access filters, then converts to the runtime provider aggregate DTO before provider dispatch |
| DataAccess provider dispatch boundary | `backend/src/data/data_access.rs` provider call sites | Facade / Extracted | `appfw-runtime::data_access` provider dispatch helpers over `RuntimeProviderDataClient`; product DataAccess now obtains a `DatabaseClientRuntimeAdapter` and dispatches health, explain, read, mutation, aggregate, audit append/query, uniqueness, and FK/PK validation calls through the runtime trait path; runtime helpers reject undeclared provider operations before invoking provider methods |
| Provider factory registry contract | `backend/src/routes/mod.rs` adapter calls | Facade / Extracted | `appfw-runtime::RuntimeProviderRegistry`; product-shaped route modules register local provider factories by canonical `FrameworkProvider` while runtime owns missing-provider handling and active data-source factory lookup |
| Provider time-period support utility | `backend/src/data/clients/time_period.rs` | Facade / Extracted | `appfw-runtime::provider_time_period`; products keep only generated `DataType` to `RuntimeDataType` adapters while runtime owns period token bounds, Sunday week-start behavior, and Date versus DateTime output shape |
| MongoDB connection option support | `backend/src/data/clients/mongo/mongo_client.rs` connection helpers | Provider package facade | `appfw-provider-mongo`; products keep generated data-source environment lookup, runtime connection-security validation, concrete `mongodb::Client` construction, and `AppError` adapters while the provider package owns Mongo driver options, credential/auth source shaping, pool/timeouts, app name, TLS option mapping, and port validation |
| MongoDB result formatting support | `backend/src/data/clients/mongo/format.rs` | Provider package facade | `appfw-provider-mongo`; products keep only compatibility re-exports while the provider package owns BSON document/vector conversion, extended JSON `$oid`/`$date` normalization, and Mongo `_id` to product `id` mapping |
| MongoDB BSON scalar conversion support | Product-local `backend/src/data/clients/mongo/bson_utils.rs` facades removed | Extracted | `appfw-provider-mongo`; the provider package owns ObjectId parsing, extended JSON `$oid` parsing, JSON-number to BSON-number conversion, filter value-vector conversion, DateTime parsing, and runtime error shaping |
| MongoDB projection and sort support | `backend/src/data/clients/mongo/{projection.rs,sort.rs}` | Provider package facade | `appfw-provider-mongo`; products keep generated `EntityType`/`PropertyType` lookup adapters while the provider package owns selection-shape parsing, raw `id` to `_id` projection mapping, typed projection document assembly from resolved fields, sort JSON normalization, and BSON sort direction document assembly |
| MongoDB filter criterion support | `backend/src/data/clients/mongo/filter.rs` | Provider package facade | `appfw-provider-mongo`; products keep generated `EntityType`/`PropertyType` lookup, JSON filter normalization, conjunction recursion, and relation-path qualification adapters while the provider package owns field-name mapping, scalar/null/array/date-period criterion compilation, BSON conversion, and invalid filter value errors over runtime data types |
| MongoDB relationship lookup, projection, query, and aggregate pipeline support | `backend/src/data/clients/mongo/mongo_client.rs` | Provider package facade | `appfw-provider-mongo`; products keep generated relationship metadata decisions, generated selection DTO adapters, and plan-to-provider DTO adapters while the provider package owns `$lookup`/`$set`/`$unset` stage shapes, nested projection document assembly, filter/access document composition, get-item/query facet pipeline assembly, aggregate group/project/having/sort document compilation, and aggregate facet pipeline assembly |
| MongoDB record value support | `backend/src/data/clients/mongo/record.rs` | Provider package facade | `appfw-provider-mongo`; products keep generated `EntityType`/`PropertyType` lookup and nested-object recursion adapters while the provider package owns Mongo record field-name mapping, nullability handling, scalar/array/JSON value conversion, and navigation-property persistence rejection over runtime data types |
| MongoDB audit document shaping and persistence support | `backend/src/data/clients/mongo/mongo_client.rs` audit helpers | Provider package facade | `appfw-provider-mongo`; products keep DataAccess audit orchestration and `AppError` adapters while the provider package owns audit collection naming, previous-hash filter, query filter, sort document, limit extraction, event BSON document conversion, previous-hash lookup, append/query execution, cursor handling, and BSON-to-JSON audit event conversion over runtime audit DTOs |
| MongoDB mutation support | `backend/src/data/clients/mongo/mongo_client.rs` mutation helpers | Provider package facade | `appfw-provider-mongo`; products keep generated primary-key metadata lookup, generated record conversion, native-property filtering, access-filter compilation, projection, concrete execution, and result decoding while the provider package owns generated key values, empty client key detection, id/version mutation filter payloads, and `$set` update document/pipeline shaping over runtime data types |
| MongoDB execution core support | `backend/src/data/clients/mongo/mongo_client.rs` driver execution helpers | Provider package facade | `appfw-provider-mongo`; products keep generated data-source environment lookup, runtime connection-security validation, concrete client construction, generated metadata planning, filter/projection/sort/pipeline compilation, legacy mutation execution, and `AppError` adapters while the provider package owns the Mongo driver handle, database/collection resolution, health ping, audit persistence, find execution, query/aggregate facet execution, facet result decoding, and native error normalization |
| PostgreSQL connection config support | `backend/src/data/clients/postgres/postgres_client.rs` connection adapter | Provider package facade | `appfw-provider-postgres`; products keep generated data-source environment lookup and `AppError` adapters while the provider package owns runtime connection-security validation, concrete pool construction, `NoTls`/Tokio runtime selection, deadpool-postgres config shaping, credential/database/host/port mapping, and port validation |
| PostgreSQL parameter binding support | Product-local `backend/src/data/clients/postgres/param_field.rs` facades are retired from the CRM sample | Extracted | `appfw-provider-postgres`; products keep only generated `DataType`/`PropertyType` adapters while the provider package owns placeholder casts, JSON/UUID/chrono binding support, runtime validation errors, and the shared SQL parameter type |
| PostgreSQL audit statement rendering and persistence support | `backend/src/data/clients/postgres/postgres_client.rs` audit helpers | Provider package facade | `appfw-provider-postgres`; products keep DataAccess audit orchestration and `AppError` adapters while the provider package owns previous-hash lookup SQL, audit append SQL, audit query SQL, placeholder ordering, JSON binding, parameter payload shape, previous-hash lookup execution, audit append/query execution, and audit JSON response decoding over runtime audit DTOs |
| PostgreSQL filter criterion support | `backend/src/data/clients/postgres/filter.rs` | Provider package facade | `appfw-provider-postgres`; products keep generated entity/property lookup, JSON filter normalization, conjunction recursion, navigation traversal, recursive inner-filter compilation, and `AppError` adapters while the provider package owns scalar/list/regex/LIKE, array, null, date-period criterion rendering, and relation `EXISTS` SQL rendering over runtime data types |
| PostgreSQL CTE/query rendering support | `backend/src/data/clients/postgres/cte.rs` CTE helpers plus query envelope assembly | Provider package facade | `appfw-provider-postgres`; products keep generated selection traversal, generated metadata lookup, access/filter composition, recursion/performance metadata, CTE definition input assembly, selection decoding, and execution while the provider package owns physical table-name normalization, JSON aggregation expression rendering, navigation join SQL, many-to-many junction join SQL, junction index-hint shapes, final CTE definition rendering, and paged CTE count/rows query envelope rendering |
| PostgreSQL mutation statement rendering support | `backend/src/data/clients/postgres/postgres_client.rs` build helpers | Provider package facade | `appfw-provider-postgres`; products keep generated property lookup, native-property filtering, primary-key discovery, relationship/junction orchestration, update access-filter composition, and execution while the provider package owns insert/delete statement rendering, junction insert/delete SQL rendering, update set/key/version rendering, final update statement assembly, placeholders, and mutation parameter binding over runtime data types |
| PostgreSQL execution core support | `backend/src/data/clients/postgres/postgres_client.rs` execution adapter | Provider package facade | `appfw-provider-postgres`; products keep generated data-source environment lookup, generated metadata planning, filter/access composition, selection decoding, mutation/junction execution, aggregate DTO adapters, and `AppError` adapters while the provider package owns connection-security validation, concrete pool construction, `NoTls`/Tokio runtime selection, the pool handle, client checkout, health check, audit persistence, generic JSON-row query execution, aggregate row execution, row count/array decoding, query envelope rendering, and native error normalization |
| MS SQL Server connection config support | `backend/src/data/clients/mssql/mssql_client.rs` connection helpers | Provider package facade | `appfw-provider-mssql`; products keep generated data-source environment lookup, runtime connection-security validation, and `AppError` adapters while the provider package owns tiberius config shaping, SQL auth mapping, encryption/trust-cert mapping, bb8 pool construction, connection manager creation, pool max-size constant, and port validation |
| MS SQL Server parameter binding support | `backend/src/data/clients/mssql/param.rs` | Provider package facade | `appfw-provider-mssql`; products keep only generated `DataType`/`PropertyType` adapters while the provider package owns T-SQL placeholders, `tiberius::ColumnData` mappings, JSON/array string binding, UUID/chrono support, DateTime normalization, and runtime validation errors |
| MS SQL Server audit statement rendering and persistence support | `backend/src/data/clients/mssql/mssql_client.rs` audit helpers | Provider package facade | `appfw-provider-mssql`; products keep DataAccess audit orchestration and `AppError` adapters while the provider package owns previous-hash lookup SQL, audit append SQL, audit query SQL, placeholder ordering, JSON string binding, parameter payload shape, previous-hash lookup execution, audit append/query execution, and audit JSON response decoding over runtime audit DTOs |
| MS SQL Server filter criterion support | `backend/src/data/clients/mssql/filter.rs` | Provider package facade | `appfw-provider-mssql`; products keep generated entity/property lookup, JSON filter normalization, conjunction recursion, navigation traversal, and `AppError` adapters while the provider package owns equality/operator, list, LIKE, array, and date-period criterion rendering over runtime data types |
| MS SQL Server mutation statement rendering support | `backend/src/data/clients/mssql/mssql_client.rs` build helpers | Provider package facade | `appfw-provider-mssql`; products keep generated property lookup, native-property filtering, primary-key discovery, relationship/junction orchestration, and execution while the provider package owns insert/update/delete SQL rendering, key/version clauses, output-id projection, placeholders, and mutation parameter binding over runtime data types |
| MS SQL Server execution core support | `backend/src/data/clients/mssql/mssql_client.rs` execution helpers | Provider package facade | `appfw-provider-mssql`; products keep generated data-source environment lookup, runtime connection-security validation, generated metadata planning, CTE/aggregate SQL assembly, mutation/junction transaction choreography, output-id interpretation, and `AppError` adapters while the provider package owns pool construction/handle/client checkout, query/execute/batch helpers, health check, audit persistence, JSON-row query execution, aggregate row execution, row count/JSON decoding, parameter binding onto `tiberius::Query`, and native error normalization |
| Snowflake connection/session/execution support | `backend/src/data/clients/snowflake/snowflake_client.rs` connection and execution helpers | Provider package facade | `appfw-provider-snowflake`; products keep generated data-source environment lookup, environment-variable reads, runtime connection-security validation, generated metadata planning, statement construction, row coercion/projection, relationship orchestration, and `AppError` adapters while the provider package owns SQL API endpoint shaping, token requirement/defaults, database/warehouse/role/session options, statement timeout defaults, HTTP client construction, statement execution/polling, response row decoding, health check, audit persistence/query execution, and native error normalization |
| Snowflake statement/binding support | `backend/src/data/clients/snowflake/statement.rs` | Provider package facade | `appfw-provider-snowflake`; products keep only generated `DataType`/`PropertyType` adapters while the provider package owns REST statement bodies, placeholder expressions, binding type/value conversion, JSON serialization, DateTime normalization, and runtime validation errors |
| Snowflake audit statement and execution support | `backend/src/data/clients/snowflake/snowflake_client.rs` audit helpers | Provider package facade | `appfw-provider-snowflake`; products keep DataAccess audit orchestration and `AppError` adapters while the provider package owns previous-hash lookup SQL, audit append SQL, audit query SQL, optional NULL handling, JSON binding, REST binding payload shape, previous-hash row interpretation, audit event finalization, append/query execution, and response decoding over runtime audit DTOs |
| Snowflake filter criterion support | `backend/src/data/clients/snowflake/filter.rs` | Provider package facade | `appfw-provider-snowflake`; products keep generated entity/property lookup, JSON filter normalization, conjunction recursion, navigation traversal, and `AppError` adapters while the provider package owns equality/operator, list, LIKE, array, and date-period criterion rendering over runtime data types |
| SQL sort rendering support | Product-local PostgreSQL sort facades are retired from the CRM sample; remaining SQL product-shaped modules keep transitional sort adapters | Provider package facade | `appfw-provider-postgres`, `appfw-provider-mssql`, and `appfw-provider-snowflake`; products keep generated property lookup, primary-key discovery, and aggregate-plan alias adapters while provider packages own provider-specific query/aggregate `ORDER BY` rendering and identifier quoting |
| SQL aggregate rendering support | `backend/src/data/clients/{postgres,mssql,snowflake}/*_client.rs` aggregate helpers | Provider package facade | `appfw-provider-postgres`, `appfw-provider-mssql`, and `appfw-provider-snowflake`; products keep generated aggregate-plan adapters, filter/access-filter assembly, table selection, and execution while provider packages own aggregate select-list, group-by, expression, HAVING, HAVING parameter binding over runtime data types, and PostgreSQL aggregate page/count query envelope rendering |
| Snowflake mutation statement rendering support | `backend/src/data/clients/snowflake/snowflake_client.rs` build helpers | Provider package facade | `appfw-provider-snowflake`; products keep generated property lookup, native-property filtering, update/delete access-filter composition, relationship/junction orchestration, refetch handling, and `AppError` adapters while the provider package owns insert/update/delete statement rendering, key/version clauses, count-statement rendering, placeholders, mutation binding over runtime data types, and raw statement execution |
| Database runner and migration workflow | `database/_pkg` plus framework `database` crate | Extracted runner | Framework `database` crate runs against product `database/_pkg` |

## Remaining Backend Runtime Inventory

| Surface | Current Product Path | Target Destination | Why Framework-Owned | Prerequisites Before Moving |
| --- | --- | --- | --- | --- |
| Backend process bootstrap | `backend/src/main.rs` | `appfw-runtime` host runner plus generated product runtime spec | Startup, tracing, security validation, app config init, listener binding, and shutdown are repeatable runtime behavior. | Runtime-owned `AppConfig`, auth state, route set factory, and provider registry contracts. |
| Security config facade | Product-local `config/security.rs` facades are retired from the CRM sample; remaining generated config adapters are transitional product shape | `appfw-runtime::security` direct runtime import | Security defaults and runtime safety validation are framework policy. | Generated/product code imports stable runtime config instead of local config module. |
| Secrets and connection-security helpers | Product-local `config/secrets.rs` facades are retired from the CRM sample; remaining generated config adapters are transitional product shape | `appfw-runtime` config/security helpers | Secret loading and TLS/local-development rules are cross-product runtime behavior. | Product data-source environment metadata is exposed through a schema-neutral adapter. |
| Product config model and policy evaluation | `backend/src/config/app_config.rs` plus generated metadata adapters | `appfw-runtime` config/policy engine with generated metadata adapter | Tenant-isolation detection/filter conjoining is runtime-owned; product config still loads policy engines and adapts generated entity metadata. Policy evaluation, metadata lookup, and deny-by-default behavior must remain consistent across products. Policy access decision shape and filter composition are already runtime-owned. | Define fuller runtime config/policy traits for schemas, entities, properties, relationships, data sources, and policy modules; generated code supplies the product metadata implementation. |
| Config file loader | `backend/src/config/loader.rs` | Split: generated product loader adapter plus runtime validator/evaluator | File locations are product-owned, but environment validation and parsed semantics are framework-owned. | Runtime metadata types and generated config publication contract are explicit. |
| Runtime error surface | `backend/src/routes/app_error.rs` | `appfw-runtime` error module plus GraphQL/HTTP conversion adapters | Error classes, redaction, and provider normalization are framework contracts. Provider error classification is now runtime-owned. | Replace generated-schema dependencies with runtime error categories and thin generated conversion where needed. |
| Data access orchestration | `backend/src/data/data_access.rs` | `appfw-runtime` data service | Authorization, validation, rules, audit, QueryIR construction, cost budgets, and provider dispatch are core framework behavior. Read-query diagnostics, batch id filter planning, batch sizing, optional/list read provider execution wrappers, read-plan provider dispatch helpers, aggregate read provider execution wrapper, mutation provider execution wrappers, mutation-plan provider dispatch helpers, read result evaluation loops, provider count tracing callbacks, query page finalization, keyset cursor finalization, mutation kind/counts, filtered-update denial checks, mutation audit record-id selection, delete audit outcome selection, audit mutation append, audit attempt append, audit failure/denial/not-applied evidence construction, record-chain audit attempt lookup/continuation, provider-backed PK/FK/uniqueness validation hook choreography, provider dispatch through `RuntimeProviderDataClient`, and provider operation-contract enforcement are runtime-owned. | Runtime metadata traits, runtime error type, generated record conversion adapters, and GraphQL type boundary are stable. Move the remaining auth/policy/validation/plan-construction service choreography only after the read/mutation/audit-attempt/validation-hook helper boundary stays green. |
| Query IR and query helpers | Product-local `filtering.rs`, `filter_capabilities.rs`, `json_utils.rs`, `snake.rs`, and `query_cost.rs` facades are retired from the CRM sample; remaining adapters around `backend/src/data/query_ir.rs` are transitional product shape | `appfw-runtime` query/model modules | Query semantics must be provider-neutral and certified once. Query cost scoring, filter operator semantics, filter capability DTOs, provider support matrix, canonical filterable data-type set, canonical operator specs, lenient identifier casing, basic runtime data-type classification, relationship target lookup, runtime descriptors on parsed filter/selection/sort nodes, pagination policy/default normalization, pagination strategy, sort direction normalization, sort input parsing, keyset cursor helpers, aggregate function semantics, aggregate field descriptors, aggregate alias rules, metric/groupability validation, default metric aliases, aggregate having operator policy, generic parser input normalization, filter conjunction shape checks, relationship filter object checks, scalar filter predicate parsing, and pure record validation are runtime-owned; generated `EntityType`/`PropertyType` fields remain for provider compatibility. | Move the rest of QueryIR parsing and validation behind runtime metadata descriptors, then bridge to generated/provider adapters only at the provider boundary. |
| Rule evaluation and audit helpers | Product-local audit facades are retired; remaining rule-module adapters are transitional product shape | `appfw-runtime` policy/rules/audit modules | Remaining rule orchestration and audit chains are cross-product contracts. Computed values, DateTime timezone conversion, concurrency version generation/lookup, audit selection, audit event shape, audit query shape, audit redaction, evidence JSON, diffing, chain scope, record-chain continuation, canonical event hashing, audit query table/limit derivation, and audit append/attempt orchestration are runtime-owned. | Runtime metadata traits and provider audit append/query contracts. |
| Provider trait | `backend/src/data/clients/database_client.rs` | `appfw-runtime` provider contract or provider SDK crate | Provider implementations should implement one stable framework trait. Runtime provider descriptors, identity, operation/count vocabulary, operation contract classification, provider-client base, provider result DTOs, provider method input envelope, provider mutation/query/aggregate plan DTOs, provider factory registry, and the async `RuntimeProviderDataClient` surface are now extracted. Root and CRM expose `DatabaseClientRuntimeAdapter`, and DataAccess dispatch now crosses that runtime trait path while existing concrete provider implementations stay local. | Package concrete provider implementations behind framework-owned crates or feature gates after provider certification stays green on the runtime dispatch path. |
| Provider implementations | `backend/src/data/clients/{postgres,mssql,mongo,snowflake}` | `appfw-provider-*` crates or feature-gated runtime modules | Provider behavior is framework-certified, not product customization. Concrete package boundaries now exist for MongoDB connection option/result formatting/BSON scalar conversion/projection/sort/filter/lookup/pipeline/record-value/audit-document/audit-persistence/mutation support and driver execution core, PostgreSQL connection config/parameter binding/audit/query and aggregate sort/filter/CTE-rendering/aggregate/mutation rendering, many-to-many CTE tuning config, plus execution core, MS SQL Server connection config/parameter binding/audit/query and aggregate sort/filter/aggregate/mutation rendering plus execution core, and Snowflake connection/session config/statement/binding/audit/query and aggregate sort/filter/aggregate/mutation rendering plus HTTP SQL API execution core. | Move provider-owned code in vertical slices that compile against runtime contracts; compile-time features select providers; provider certification remains green. |
| Provider support utilities | `backend/src/data/clients/*` shared helpers plus provider-specific helpers | `appfw-runtime` provider support or provider crates | SQL helpers, BSON/SQL conversion rules, and remaining provider utilities are provider framework behavior. Time-period semantics, provider error normalization, MongoDB connection option/result formatting/BSON scalar conversion/projection/sort/filter/lookup/pipeline/record-value/audit-document/audit-persistence/mutation support and driver execution core, PostgreSQL connection config/parameter binding/audit/query and aggregate sort/filter/CTE-rendering/aggregate/mutation rendering support plus execution core, MS SQL Server connection config/parameter binding/audit/query and aggregate sort/filter/aggregate/mutation rendering support plus execution core, and Snowflake connection/session config/statement/binding/audit/query and aggregate sort/filter/aggregate/mutation rendering support plus HTTP SQL API execution core are already runtime/provider-package-owned. | Provider package split design identifies shared support versus provider-specific support. |
| Provider contract tests | Root validation fixture `backend/src/data/clients/contract_tests.rs`; removed from `examples/products/crm/backend` | `appfw-test` or provider package tests | Provider certification belongs upstream, not in downstream product backends. | Provider trait/packages compile independently from product backend; CRM keeps product scenarios and product-local unit tests only. |
| Readiness/info routes | `backend/src/routes/info.rs` | Facade / Extracted | Health/readiness DTOs, liveness/readiness status mapping, version loading, metrics route exposure, and the route shell are runtime behavior. Products keep only provider readiness adapters that call local `DataAccess`. | Runtime provider registry can enumerate product-active data sources. |
| GraphQL schema routes | `backend/src/routes/{crm.rs,system.rs}` | Generated adapter over runtime GraphQL host | Schema object construction remains generated, but GraphiQL routing, JWT extraction, introspection routing, request-context capture, GraphQL error mapping, execution, and response annotation now delegate to `appfw-runtime::routing::runtime_graphql_schema_routes`. Runtime handler context packaging and schema-neutral GraphQL user/data extraction are framework-owned and bound through the product API facade. | Runtime `HandlerContext`, auth extractor, app config, data access, and GraphQL error mapping are stable. |
| Generated schemas and defaults | `backend/src/schemas`, `backend/src/handlers/<schema>/generated.rs`, route modules | Generated adapter | Product teams should review generated API surface and regenerate rather than hand-edit. | Keep generated ownership manifest accurate; do not move product schema types into runtime. |
| Product handlers | `backend/src/handlers/<schema>/<entity>.rs` | Product extension | Human-owned app behavior should survive regeneration. | Continue boundary-check enforcement against framework-internal imports. |
| Product services | `backend/src/services` | Product extension | Durable domain workflows belong to products. | Continue boundary-check enforcement and keep services on stable `product_api`/`DataAccess`. |
| Admin API shell | `backend/src/admin_ui.rs` | `appfw-runtime` admin service with generated metadata/data adapters | Admin diagnostics are model-driven framework UI/API behavior. Admin UI route path and asset mount shell, runtime state contract, standard handler shell, bundle path resolution, index response shaping, diagnostic errors, HTTP error response shaping, auth header extraction, migration manifest DTO/path/loading, admin model/schema response DTOs, admin model provider/service orchestration, admin endpoint HTTP auth/error/JSON response shell, schema summary input and assembly helpers, schema-health helper contracts and summary construction, migration dialect helper, provider capability response helpers, troubleshooting status, role/action helpers, policy-decision DTOs, policy-explain result helper and request/response orchestration, audit-timeline request/response orchestration, audit-timeline subject/result shell, and query-diagnose request/response orchestration are runtime-owned. | Runtime metadata, provider registry, data access diagnostics, auth/RBAC check, and safe diagnostic contracts. |
| Admin frontend source | `admin_ui` | Framework package/app asset | Downstream products should not fork admin UI implementation. Runtime owns the backend bundle path/index response shell; products mount assets and provide branding/topology through explicit extension points. | Product branding/topology extension points are explicit; build artifacts are packaged. |
| MCP catalog and server execution | `backend/src/mcp/mod.rs` plus generated operation adapter metadata conversion; product-local `mcp/catalog.rs` removed | `appfw-runtime` MCP service with generated operation registry | MCP is a runtime transport over the same handler/DataAccess/policy path as GraphQL. Schema-neutral access decisions, origin checks, HTTP route/auth/origin/response shell, result/resource bounds, tool result/error shaping, MCP error DTOs, payload/batch/JSON-RPC loop/envelope handling, catalog/resource/prompt/tool schema rendering, generated-operation catalog/dispatcher traits, initialize/ping/resource/prompt built-ins, support-tool execution, method dispatch/tool-list assembly, reflected generated-operation `tools/call` shell orchestration, `appfw_explain_access` request/response orchestration, generated-tool audit payload shaping/event construction, and generated-tool audit sink/store adapter contracts are runtime-owned. | Runtime metadata, auth/RBAC, generated dispatch adapter, access-explain policy adapter, redaction contract, and product DataAccess persistence adapter. |
| MCP generated dispatcher | `backend/src/mcp/generated_operations.rs` | Generated adapter | Operation list and argument decoding are product-schema-specific generated output. | Runtime MCP service accepts a generated registry/dispatcher trait. |
| Local binary exporters | `appfw_cli/src/bin/provider_certification_export.rs` | Framework command | Provider certification export mechanics are framework tooling and no longer require a product backend binary or generated `DataSourceType` bridge. | Provider keys and certification reports stay on runtime-owned `FrameworkProvider` keys. |

## Target Runtime Contract Set

The remaining extraction should converge on these stable contracts before the
CRM backend template is slimmed:

| Contract | Purpose | Product Contribution |
| --- | --- | --- |
| `ProductRuntimeSpec` | Runtime host input: app identity, schema routers, generated metadata, operation registry, active data sources. | Generated from topology and config. |
| `ModelMetadata` | Schema/entity/property/relationship lookup without importing generated `EntityType` internals directly. First descriptors live in `appfw-runtime::model_metadata`. | Generated metadata adapter over product schema types. |
| `RuntimeAppConfig` | Loaded product config plus policy engines and data-source environments. | Product-owned config files and generated loader adapter. |
| `RuntimeError` | Stable error classes for GraphQL, REST/admin, MCP, provider, config, and policy failures. First category contract lives in `appfw-runtime::RuntimeError`. | Generated adapters convert where product schema types require it. |
| `RuntimeAuth` | Auth state, token extraction, test auth constraints, and identity contracts. | Product supplies environment and optional branding/policy claims config. |
| `RuntimeHost` | Independent ingress module loading, runtime modes, feature-gated host plans, and HTTP/MCP/Kafka service selection. | Product deployment chooses `APPFW_RUNTIME_MODE` / `APPFW_MODULES`; generated backend templates wire product routes and worker entrypoints. |
| `ProviderRegistry` | Compile-time provider availability plus runtime active data-source construction. First factory registry contract lives in `appfw-runtime::RuntimeProviderRegistry`. | Product Cargo features select providers; topology selects active data sources; current product route adapters register local factories until provider packages exist. |
| `ProviderClient` | Provider implementation trait over runtime metadata, QueryIR, MutationPlan, AggregatePlan, audit, health, and diagnostics. | No direct product implementation except explicit custom-provider packages. |
| `OperationRegistry` | GraphQL/MCP/admin operation metadata and generated dispatch hooks. | Generated operation registry and product handler implementations. |
| `AdminSurface` | Model-driven admin routes, diagnostics, pool stats, readiness, and safe query explain. | Product branding/topology and generated metadata. |
| `McpSurface` | MCP transport, catalog, RBAC, resources, tools, and execution shell. | Generated operation dispatcher and redacted entity serialization. |
| `KafkaIngress` | Kafka worker ingress configuration, broker-auth config validation, service-principal actor mapping, tenant derivation, operation bindings, retry/DLQ/readiness validation, and message-to-operation dispatch shell. | Product topology enables Kafka entries; generated `backend/config/ingress/kafka.yaml` is human-owned; product/platform supplies concrete broker message source, secrets, and business event fixtures. |

## Recommended Extraction Order

1. Keep the schema-neutral metadata and runtime error contracts stable and
   extend them only where the next extraction slice needs a missing descriptor.
   These contracts now unblock config, DataAccess, provider, admin, and MCP
   moves without dragging generated `EntityType` through framework crates.
2. Extract QueryIR, rules, validation, audit helpers, and DataAccess behind the
   metadata/error/auth contracts. Query cost scoring, filter capability
   contracts, pagination/keyset helpers, aggregate validation primitives,
   generic parser shape checks, pure record validation, DateTime timezone
   record rules, concurrency version rules, computed record rules,
   metadata-driven audit helpers and event contracts, read-query DataAccess
   helper contracts, read/aggregate provider execution wrappers, mutation
   helper contracts, mutation provider execution wrappers, mutation-plan
   provider dispatch helpers, audit
   append/attempt orchestration, provider bridge vocabulary, provider
   identity, provider-client base contracts, provider result DTOs, provider
   method input envelope, provider mutation/query/aggregate plan DTOs,
   provider factory registry, provider error normalization, provider-backed
   validation hooks, the async provider trait surface, DataAccess provider
   dispatch helpers, and provider operation-contract enforcement are
   runtime-owned. Policy access composition is runtime-owned; next data-plane
   slices should move deeper DataAccess orchestration behind the stable
   provider dispatch boundary.
   Keep product handlers and services calling the
   stable `product_api` facade.
3. Move remaining DataAccess orchestration behind the stable runtime provider
   trait/service boundary, then move provider implementations into
   feature-gated runtime/provider packages. The runtime factory registry
   contract is already in place for product route adapters.
4. Extract admin diagnostics and remaining MCP audit/service adapters using the
   same metadata, auth, DataAccess, diagnostics, and operation registry
   contracts.
5. Move backend host bootstrap/readiness into a runtime host runner. Generated
   route modules should provide schema routers and product adapters only.
6. Slim CRM and `appfw new --profile crm-sample` after these contracts are
   real. The product backend should then contain generated adapters/defaults,
   product handlers, product services, and product config wiring only.

## Tactical Handoff For The Next Runtime Thread

Current active slice:
`codex/runtime-proof-and-admin-cleanup`.

The MCP HTTP shell and admin/MCP adapter cleanups from
`codex/runtime-mcp-http-shell` are merged. The active work is now proof and
handoff consolidation before template slimming:

1. Prove the merged runtime path on `main` with targeted runtime/root/CRM MCP
   and admin tests.
2. Run local validation, boundary checks, generated drift checks, full local
   tests, docs-check, diff-check, and handoff.
3. Record the live-provider evidence gap separately because
   `scripts/appfw release-check --json` expects running provider-backed
   backends for PostgreSQL, MongoDB, MS SQL Server, and Snowflake.
4. Start CRM/backend and `appfw new --profile crm-sample` slimming only after
   local proof is clean and live-provider proof needs are explicit.

Non-goals for the active slice:

- Do not move concrete provider behavior or provider certification tests.
- Do not change MCP/admin runtime behavior unless proof finds a regression.
- Do not slim `examples/products/crm/backend` or `appfw new` output in the
  same PR as proof/handoff cleanup.
- Do not move generated schema structs into `appfw-runtime`.

Recommended next PR after this proof lane:

1. Fix any local proof-gate gaps.
2. Run or schedule live provider certification/release-check with running
   provider-backed backend instances.
3. Slim product templates only after MCP/admin runtime services consume stable
   generated adapters and proof evidence is clean.

Likely files for the active proof PR:

```text
docs/architecture/runtime-ownership-inventory.md
docs/architecture/product-packaging-extraction-plan.md
docs/architecture/runtime-extraction-coordination.md
docs/release/roadmap.md
```

Acceptance checks:

```bash
scripts/appfw validate --json
examples/products/crm/scripts/appfw validate --json
scripts/appfw boundary-check --json
examples/products/crm/scripts/appfw boundary-check --json
cargo test --locked -p appfw-runtime mcp::
cargo test --locked -p appfw-runtime admin::
cargo test --locked --manifest-path examples/products/crm/backend/Cargo.toml mcp:: -- --test-threads=1
cargo test --locked --manifest-path examples/products/crm/backend/Cargo.toml admin_ui:: -- --test-threads=1
scripts/appfw generate --check --json
examples/products/crm/scripts/appfw generate --check --json
scripts/appfw test --json
scripts/appfw docs-check --json
git diff --check
scripts/appfw handoff --json
```

Provider certification evidence is not required for this docs/proof slice, but
it remains mandatory before release promotion or any provider-visible
connection, query/filter/mutation, audit, or persistence behavior change.

## Validation Spine

Every extraction slice should prove the CRM sample product:

```bash
scripts/appfw validate --json
examples/products/crm/scripts/appfw validate --json
scripts/appfw boundary-check --json
examples/products/crm/scripts/appfw boundary-check --json
scripts/appfw generate --check --json
examples/products/crm/scripts/appfw generate --check --json
scripts/appfw test --fast --json
examples/products/crm/scripts/appfw test --fast --json
scripts/appfw handoff --json
```

Run full `scripts/appfw test --json` and the CRM equivalent before PRs that
touch data access, provider behavior, GraphQL/MCP/admin execution, or auth.
Provider behavior changes also require provider certification evidence.
