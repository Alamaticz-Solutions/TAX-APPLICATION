use anyhow::Result;
use async_trait::async_trait;
use futures::StreamExt;
use inflector::cases::tablecase::to_table_case;
use mongodb::bson;
use mongodb::options::ReturnDocument;
use mongodb::{bson::Document, Client, Collection};
use serde_json::{json, Map, Value};
use std::sync::Arc;
use std::time::Instant;
use time::OffsetDateTime;
use tracing::{debug, error, info};

use appfw_provider_mongo::{
    aggregate_group_and_project as provider_aggregate_group_and_project,
    aggregate_having as provider_aggregate_having,
    aggregate_pipeline as provider_aggregate_pipeline, aggregate_sort as provider_aggregate_sort,
    combine_filter_documents, create_primary_key_value as provider_create_primary_key_value,
    get_item_pipeline as provider_get_item_pipeline,
    mongo_client_options as provider_mongo_client_options,
    mutation_id_filter as provider_mutation_id_filter,
    mutation_set_update_document as provider_mutation_set_update_document,
    mutation_set_update_pipeline as provider_mutation_set_update_pipeline,
    query_pipeline as provider_query_pipeline,
    relationship_lookup_stages as provider_relationship_lookup_stages,
    selection_projection as provider_selection_projection, MongoAggregateGroup,
    MongoAggregateHavingPredicate, MongoAggregateMetric, MongoAggregateSortSpec,
    MongoConnectionConfig, MongoExecutionClient, MongoRelationshipLookup,
    MongoSelectionProjectionNode,
};
use appfw_runtime::{
    connection_security::{self, Provider},
    extension::UserAuth,
    json::JsonObj,
    provider_keys::FrameworkProvider,
    record_locator::RECORD_LOCATOR_FIELD,
    PolicyAccess, RuntimeAuditEvent, RuntimeAuditQuery, RuntimeProviderIdentity,
    RuntimeProviderPlanInput,
};

use crate::config::app_config::AppConfig;
use crate::data::clients::database_client::{
    DatabaseClient, ProviderAggregatePlan, ProviderPoolStats, ProviderQueryPlan,
    ProviderRoutineArgument, ProviderRoutineCall,
};
use crate::data::clients::mongo::filter::{self};
use crate::data::clients::mongo::format::{bson_doc_to_json_obj, bson_vec_to_json_vec};
use crate::data::clients::mongo::projection::create_projection_for_entity;
use crate::data::clients::mongo::record::TYPE_PK_NAME;
use crate::data::clients::mongo::{record, sort};
use crate::data::query_ir::{FilterAst, QueryPlan, SelectionNode, SelectionTree};
use crate::product_api::runtime_data_type;
use crate::routes::app_error::AppError;
use crate::schemas::common::{JsonAggregateResult, JsonQueryResult};
use crate::schemas::system::{DataSourceEnvironment, DataType, EntityType, PropertyType};

fn validate_connection_security(
    env: &DataSourceEnvironment,
) -> Result<connection_security::ConnectionSecurity, AppError> {
    connection_security::validate(
        Provider::MongoDb,
        &env.name,
        &env.security_profile,
        &env.tls_mode,
        &env.db_host,
    )
    .map_err(|e| AppError::DataAccess(format!("invalid MongoDB connection security: {e}")))
}

pub struct MongoClient {
    execution: MongoExecutionClient,
    app_config: Arc<AppConfig>,
    data_source_name: String,
}

impl MongoClient {
    #[tracing::instrument(skip(app_config), fields(data_source = %data_source_name))]
    pub async fn init(
        app_config: Arc<AppConfig>,
        data_source_name: String,
    ) -> Result<MongoClient, AppError> {
        let data_source_env = app_config.get_data_source_env(&data_source_name)?;
        let execution = Self::get_client(data_source_env).await?;
        Ok(MongoClient {
            execution,
            app_config: app_config.clone(),
            data_source_name,
        })
    }

    // Connection parameters are loaded from DataSourceEnvironment; credentials
    // are resolved by the config loader through the secret provider.
    #[tracing::instrument(skip(data_source_env), fields(host = %data_source_env.db_host, database = %data_source_env.db_name))]
    async fn get_client(
        data_source_env: Arc<DataSourceEnvironment>,
    ) -> Result<MongoExecutionClient, AppError> {
        // Client is a pooled connection (managed internally by mongodb driver).
        let security = validate_connection_security(&data_source_env)?;
        let connection_config = MongoConnectionConfig::new(
            data_source_env.db_host.clone(),
            data_source_env.db_port.clone(),
            data_source_env.service_account_name.clone(),
            data_source_env.service_account_password.clone(),
        );
        let client_options = provider_mongo_client_options(&connection_config, &security)
            .await
            .map_err(AppError::from)?;

        let client = Client::with_options(client_options)
            .map_err(|e| AppError::DataAccess(e.to_string()))?;

        let execution = MongoExecutionClient::new(client, data_source_env.db_name.clone());

        match execution.health_check().await {
            Ok(_) => {
                info!("MongoDB client connected");
            }
            Err(e) => {
                error!(error = %e, "MongoDB ping failed");
                return Err(AppError::from(e));
            }
        }

        Ok(execution)
    }

    fn get_collection(&self, collection_name: &str) -> Collection<Document> {
        debug!(database = %self.execution.db_name(), collection = %collection_name, "resolving MongoDB collection");
        self.execution.collection(collection_name)
    }

    fn get_entity_collection(&self, entity_type: &EntityType) -> Collection<Document> {
        self.get_collection(&entity_collection_name(entity_type))
    }

    fn from_mongo_error(e: mongodb::error::Error) -> AppError {
        AppError::from(appfw_provider_mongo::mongo_runtime_error(e))
    }
    // fn from_bson_error(e: mongodb::bson::ser::Error) -> AppError {
    //   eprintln!("\n > MongoClient bson ERROR: {:?}", e.to_string());
    //   AppError::DataAccess(e.to_string())
    // }
    fn create_primary_key_value(
        &self,
        entity_type: Arc<EntityType>,
        input: &Map<String, Value>,
    ) -> Result<Value, AppError> {
        let pk_name = self.app_config.get_primary_key_name(entity_type.clone())?;
        let pk_prop = AppConfig::get_prop(entity_type, &pk_name)?;
        provider_create_primary_key_value(
            input.get(&pk_name),
            runtime_data_type(pk_prop.data_type),
            &pk_prop.name,
        )
        .map_err(AppError::from)
    }

    fn compile_filter_with_access(
        &self,
        entity_type: Arc<EntityType>,
        filter_value: Option<Value>,
        access: &PolicyAccess,
    ) -> Result<Document, AppError> {
        let filter_doc =
            filter::try_create_filter(self.app_config.clone(), entity_type.clone(), filter_value)?;
        let access_doc =
            filter::try_create_filter(self.app_config.clone(), entity_type, access.filter.clone())?;

        Ok(combine_filter_documents(filter_doc, access_doc))
    }

    fn compile_filter_ast_with_access(
        &self,
        entity_type: Arc<EntityType>,
        filter_ast: Option<&FilterAst>,
        access_filter_ast: Option<&FilterAst>,
    ) -> Result<Document, AppError> {
        let filter_doc = filter::try_create_filter_ast(
            self.app_config.clone(),
            entity_type.clone(),
            filter_ast,
        )?;
        let access_doc =
            filter::try_create_filter_ast(self.app_config.clone(), entity_type, access_filter_ast)?;

        Ok(combine_filter_documents(filter_doc, access_doc))
    }

    // fn get_pk_filter_from_id(id: String) -> Result<Document, AppError> {
    //   // Get the primary key from the input record, converted to ObjectId
    //   let id = match ObjectId::parse_str(id.to_string()) {
    //     Ok(id) => id,
    //     Err(e) => return Err(AppError::DataAccess(e.to_string()))
    //   };
    //   Ok( doc!{ MONGO_PK_NAME: id } )
    // }

    #[tracing::instrument(
    skip(self, entity_type, selections, input, filter),
    fields(entity = %entity_type.pascal_1)
  )]
    async fn upsert_item_json(
        &self,
        entity_type: Arc<EntityType>,
        selections: serde_json::Value,
        input: &Map<String, serde_json::Value>,
        filter: &Map<String, serde_json::Value>,
    ) -> Result<Map<String, serde_json::Value>, AppError> {
        debug!("upserting MongoDB item");

        let record = record::to_bson(self.app_config.clone(), entity_type.clone(), input.clone())?;

        let json_filter = Some(filter.to_owned().into());
        let filter_doc =
            filter::try_create_filter(self.app_config.clone(), entity_type.clone(), json_filter)?;

        let projection = create_projection_for_entity(entity_type.clone(), &selections)?;

        let update = provider_mutation_set_update_pipeline(record);

        let col = self.get_entity_collection(&entity_type);

        let result_doc = col
            .find_one_and_update(filter_doc, update)
            .upsert(true)
            .return_document(ReturnDocument::After)
            .projection(projection)
            .await
            .map_err(Self::from_mongo_error)?;

        debug!(
            has_document = result_doc.is_some(),
            "MongoDB upsert completed"
        );

        let result_doc = result_doc.ok_or_else(|| {
            AppError::DataAccess("MongoDB upsert did not return a document".to_string())
        })?;
        let res = bson_doc_to_json_obj(result_doc);

        Ok(res)
    }

    async fn refresh_account_health_provider_routine(
        &self,
        arguments: &[ProviderRoutineArgument],
    ) -> Result<Value, AppError> {
        let account_id = routine_string_argument(arguments, "account_id")?;
        let health_score = routine_f64_argument(arguments, "health_score")?;
        let refreshed_at = OffsetDateTime::now_utc().to_string();

        let collection = self.get_collection(&mongo_collection_name("crm", "accounts"));
        let result = collection
            .find_one_and_update(
                bson::doc! { "id": &account_id },
                bson::doc! {
                    "$set": {
                        "health_score": health_score,
                        "health_last_refreshed_at": refreshed_at.clone(),
                    },
                    "$inc": { "version": 1_i64 },
                },
            )
            .return_document(ReturnDocument::After)
            .await
            .map_err(Self::from_mongo_error)?;

        if result.is_none() {
            return Err(AppError::DataAccess(format!(
                "account `{account_id}` was not found"
            )));
        }

        Ok(json!({
            "account_id": account_id,
            "health_score": health_score,
            "refreshed_at": refreshed_at,
            "source": "mongo-stored-procedure",
        }))
    }
}

fn entity_collection_name(entity_type: &EntityType) -> String {
    mongo_collection_name(&entity_type.schema_name, &entity_type.snake_n)
}

fn mongo_collection_name(schema_name: &str, collection_name: &str) -> String {
    format!("{schema_name}.{collection_name}")
}

fn new_record_locator() -> String {
    format!("rl_{}", uuid::Uuid::new_v4().simple())
}

fn routine_argument<'a>(
    arguments: &'a [ProviderRoutineArgument],
    name: &str,
) -> Result<&'a Value, AppError> {
    arguments
        .iter()
        .find(|argument| argument.name == name)
        .map(|argument| &argument.value)
        .ok_or_else(|| AppError::Validation(format!("missing routine argument `{name}`")))
}

fn routine_string_argument(
    arguments: &[ProviderRoutineArgument],
    name: &str,
) -> Result<String, AppError> {
    routine_argument(arguments, name)?
        .as_str()
        .map(ToString::to_string)
        .ok_or_else(|| AppError::Validation(format!("routine argument `{name}` must be a string")))
}

fn routine_f64_argument(
    arguments: &[ProviderRoutineArgument],
    name: &str,
) -> Result<f64, AppError> {
    routine_argument(arguments, name)?
        .as_f64()
        .ok_or_else(|| AppError::Validation(format!("routine argument `{name}` must be a number")))
}

fn junction_collection_name(schema_name: &str, junction_table: &str) -> String {
    mongo_collection_name(schema_name, &to_table_case(junction_table))
}

fn storage_field_name(prop: &PropertyType) -> String {
    if prop.is_key && prop.name == TYPE_PK_NAME && prop.data_type == DataType::ObjectId {
        "_id".to_string()
    } else {
        prop.name.clone()
    }
}

fn primary_key_storage_field(entity_type: Arc<EntityType>) -> Result<String, AppError> {
    let prop = entity_type
        .props
        .iter()
        .find(|prop| prop.is_key)
        .ok_or_else(
            || crate::routes::app_error::MetadataError::MissingPrimaryKey {
                entity_type: entity_type.pascal_1.clone(),
            },
        )?;
    Ok(storage_field_name(prop))
}

fn current_entity_has_storage_field(entity_type: &EntityType, field_name: &str) -> bool {
    entity_type
        .props
        .iter()
        .any(|prop| prop.name == field_name && AppConfig::is_native_prop(Arc::new(prop.clone())))
}

fn mongo_relationship_lookup_stages(
    entity_type: Arc<EntityType>,
    prop: Arc<PropertyType>,
    target_entity_type: Arc<EntityType>,
) -> Result<Vec<Document>, AppError> {
    match prop.data_type {
        DataType::NavToOne | DataType::NavToMany => {
            let nav = prop.nav_by_fk_property.clone().ok_or_else(|| {
                crate::routes::app_error::MetadataError::MissingNavigation {
                    entity_type: entity_type.pascal_1.clone(),
                    property_name: prop.name.clone(),
                }
            })?;
            let target_pk = primary_key_storage_field(target_entity_type.clone())?;
            let source_pk = primary_key_storage_field(entity_type.clone())?;
            let (local_field, foreign_field) =
                if current_entity_has_storage_field(&entity_type, &nav.prop_name) {
                    (nav.prop_name.clone(), target_pk)
                } else {
                    (source_pk, nav.prop_name.clone())
                };
            Ok(provider_relationship_lookup_stages(
                MongoRelationshipLookup::Direct {
                    relationship_name: prop.name.clone(),
                    target_collection: entity_collection_name(&target_entity_type),
                    local_field,
                    foreign_field,
                    flatten_to_one: prop.data_type == DataType::NavToOne,
                },
            ))
        }
        DataType::ManyToMany => {
            let many_to_many = prop.many_to_many_property.clone().ok_or_else(|| {
                crate::routes::app_error::MetadataError::MissingManyToMany {
                    entity_type: entity_type.pascal_1.clone(),
                    property_name: prop.name.clone(),
                }
            })?;
            let junction_schema = many_to_many
                .junction_schema
                .clone()
                .unwrap_or_else(|| entity_type.schema_name.clone());
            let junction_alias = format!("__{}_junction", prop.name);
            let source_pk = primary_key_storage_field(entity_type)?;
            let target_pk = primary_key_storage_field(target_entity_type.clone())?;
            Ok(provider_relationship_lookup_stages(
                MongoRelationshipLookup::ManyToMany {
                    relationship_name: prop.name.clone(),
                    junction_collection: junction_collection_name(
                        &junction_schema,
                        &many_to_many.junction_table,
                    ),
                    junction_alias,
                    source_key_field: source_pk,
                    junction_local_key: many_to_many.local_key.clone(),
                    junction_foreign_key: many_to_many.foreign_key.clone(),
                    target_collection: entity_collection_name(&target_entity_type),
                    target_key_field: target_pk,
                },
            ))
        }
        _ => Err(AppError::Validation(format!(
            "property '{}' is not a relationship",
            prop.name
        ))),
    }
}

fn push_unique_relationship_lookup(
    stages: &mut Vec<Document>,
    seen: &mut std::collections::HashSet<String>,
    entity_type: Arc<EntityType>,
    prop: Arc<PropertyType>,
    target_entity_type: Arc<EntityType>,
) -> Result<(), AppError> {
    let key = format!("{}.{}", entity_type.pascal_1, prop.name);
    if seen.insert(key) {
        stages.extend(mongo_relationship_lookup_stages(
            entity_type,
            prop,
            target_entity_type,
        )?);
    }
    Ok(())
}

fn collect_selection_relationship_lookups(
    stages: &mut Vec<Document>,
    seen: &mut std::collections::HashSet<String>,
    entity_type: Arc<EntityType>,
    nodes: &[SelectionNode],
) -> Result<(), AppError> {
    for node in nodes {
        if let Some(target) = &node.target_entity_type {
            push_unique_relationship_lookup(
                stages,
                seen,
                entity_type.clone(),
                node.prop.clone(),
                target.clone(),
            )?;
        }
    }
    Ok(())
}

fn collect_filter_relationship_lookups(
    stages: &mut Vec<Document>,
    seen: &mut std::collections::HashSet<String>,
    entity_type: Arc<EntityType>,
    filter: Option<&FilterAst>,
) -> Result<(), AppError> {
    let Some(filter) = filter else {
        return Ok(());
    };
    match filter {
        FilterAst::All | FilterAst::Field(_) => Ok(()),
        FilterAst::And(items) | FilterAst::Or(items) => {
            for item in items {
                collect_filter_relationship_lookups(stages, seen, entity_type.clone(), Some(item))?;
            }
            Ok(())
        }
        FilterAst::Relation(relation) => push_unique_relationship_lookup(
            stages,
            seen,
            entity_type,
            relation.prop.clone(),
            relation.target_entity_type.clone(),
        ),
    }
}

fn relationship_lookup_stages_for_plan(
    plan: &ProviderQueryPlan,
) -> Result<Vec<Document>, AppError> {
    let mut stages = Vec::new();
    let mut seen = std::collections::HashSet::new();
    collect_selection_relationship_lookups(
        &mut stages,
        &mut seen,
        plan.entity_type.clone(),
        &plan.selection.fields,
    )?;
    collect_filter_relationship_lookups(
        &mut stages,
        &mut seen,
        plan.entity_type.clone(),
        plan.filter.as_ref(),
    )?;
    collect_filter_relationship_lookups(
        &mut stages,
        &mut seen,
        plan.entity_type.clone(),
        plan.access_filter.as_ref(),
    )?;
    Ok(stages)
}

fn mongo_projection_from_selection(selection: &SelectionTree) -> Document {
    let nodes: Vec<_> = selection.fields.iter().map(mongo_projection_node).collect();
    provider_selection_projection(&nodes)
}

fn mongo_projection_node(node: &SelectionNode) -> MongoSelectionProjectionNode {
    MongoSelectionProjectionNode {
        name: node.prop.name.clone(),
        is_object_id_key: node.prop.is_key
            && node.prop.name == TYPE_PK_NAME
            && node.prop.data_type == DataType::ObjectId,
        children: node.children.iter().map(mongo_projection_node).collect(),
    }
}

fn mongo_query_pipeline(
    plan: &ProviderQueryPlan,
    filter_doc: Document,
    sort_doc: Document,
) -> Result<Vec<Document>, AppError> {
    Ok(provider_query_pipeline(
        relationship_lookup_stages_for_plan(plan)?,
        filter_doc,
        sort_doc,
        plan.pagination.skip,
        plan.pagination.limit,
        mongo_projection_from_selection(&plan.selection),
    ))
}

fn mongo_aggregate_group_and_project(
    plan: &ProviderAggregatePlan,
) -> Result<(Document, Document), AppError> {
    let groups: Vec<_> = plan
        .group_by
        .iter()
        .map(|group| MongoAggregateGroup {
            alias: group.alias.clone(),
            field_name: group.prop.name.clone(),
        })
        .collect();
    let metrics: Vec<_> = plan
        .metrics
        .iter()
        .map(|metric| MongoAggregateMetric {
            function: metric.function,
            alias: metric.alias.clone(),
            field_name: metric.prop.as_ref().map(|prop| prop.name.clone()),
        })
        .collect();

    provider_aggregate_group_and_project(&groups, &metrics).map_err(AppError::from)
}

fn mongo_aggregate_having(plan: &ProviderAggregatePlan) -> Result<Document, AppError> {
    let Some(having) = &plan.having else {
        return Ok(Document::new());
    };
    let predicates: Vec<_> = having
        .predicates
        .iter()
        .map(|predicate| MongoAggregateHavingPredicate {
            metric_alias: predicate.metric_alias.clone(),
            op: predicate.op,
            value: predicate.value.clone(),
        })
        .collect();
    provider_aggregate_having(&predicates).map_err(AppError::from)
}

fn mongo_aggregate_sort(plan: &ProviderAggregatePlan) -> Document {
    let specs: Vec<_> = plan
        .sort
        .specs
        .iter()
        .map(|spec| MongoAggregateSortSpec {
            alias: spec.alias.clone(),
            direction: spec.direction,
        })
        .collect();
    provider_aggregate_sort(&specs)
}

impl RuntimeProviderIdentity for MongoClient {
    fn data_source_name(&self) -> &str {
        &self.data_source_name
    }

    fn framework_provider(&self) -> FrameworkProvider {
        FrameworkProvider::Mongo
    }

    fn pool_stats(&self) -> ProviderPoolStats {
        self.provider_descriptor().opaque_pool_stats()
    }
}

#[async_trait]
impl DatabaseClient for MongoClient {
    async fn health_check(&self) -> Result<(), AppError> {
        self.execution.health_check().await.map_err(AppError::from)
    }

    async fn append_audit_event(&self, event: RuntimeAuditEvent) -> Result<(), AppError> {
        self.execution
            .append_audit_event(event)
            .await
            .map_err(AppError::from)
    }

    async fn query_audit_events(&self, query: RuntimeAuditQuery) -> Result<Vec<Value>, AppError> {
        self.execution
            .query_audit_events(query)
            .await
            .map_err(AppError::from)
    }

    async fn call_provider_routine_json(
        &self,
        routine: &ProviderRoutineCall,
        arguments: &[ProviderRoutineArgument],
        _user: &UserAuth,
        access: &PolicyAccess,
    ) -> Result<Value, AppError> {
        if !access.allow {
            return Err(AppError::AccessDenied);
        }

        match routine.method_name {
            "refresh_account_health_stored_procedure" => {
                self.refresh_account_health_provider_routine(arguments)
                    .await
            }
            _ => Err(AppError::DataAccess(format!(
                "provider routine `{}` is not implemented for data source `{}`",
                routine.method_name,
                self.data_source_name()
            ))),
        }
    }

    async fn get_items_plan_json(
        &self,
        input: RuntimeProviderPlanInput<'_, ProviderQueryPlan>,
    ) -> Result<Vec<JsonObj>, AppError> {
        Ok(self.query_items_plan_json(input).await?.items)
    }

    async fn query_items_plan_json(
        &self,
        input: RuntimeProviderPlanInput<'_, ProviderQueryPlan>,
    ) -> Result<JsonQueryResult, AppError> {
        let (plan, _user, access) = input.into_parts();
        debug!("querying MongoDB items from query plan");

        if !access.allow {
            debug!("MongoDB query denied by access policy");
            return Err(AppError::AccessDenied);
        }

        let use_skip = plan.pagination.skip;
        let use_limit = plan.pagination.limit;
        let request_datetime = OffsetDateTime::now_utc();
        let request_start = Instant::now();
        let sort_doc = sort::create_if_exists_ast(plan.entity_type.clone(), &plan.sort)?;
        let filter_doc = self.compile_filter_ast_with_access(
            plan.entity_type.clone(),
            plan.filter.as_ref(),
            plan.access_filter.as_ref(),
        )?;

        let pipeline = mongo_query_pipeline(&plan, filter_doc, sort_doc)?;

        self.execution
            .query_facet(
                &entity_collection_name(&plan.entity_type),
                pipeline,
                use_skip,
                use_limit,
                request_datetime.to_string(),
                request_start,
            )
            .await
            .map_err(AppError::from)
    }

    async fn aggregate_items_plan_json(
        &self,
        input: RuntimeProviderPlanInput<'_, ProviderAggregatePlan>,
    ) -> Result<JsonAggregateResult, AppError> {
        let (plan, _user, access) = input.into_parts();
        debug!("aggregating MongoDB items from aggregate plan");

        if !access.allow {
            debug!("MongoDB aggregate denied by access policy");
            return Err(AppError::AccessDenied);
        }

        let use_skip = plan.pagination.skip;
        let use_limit = plan.pagination.limit;
        let request_datetime = OffsetDateTime::now_utc();
        let request_start = Instant::now();
        let filter_doc = self.compile_filter_ast_with_access(
            plan.entity_type.clone(),
            plan.filter.as_ref(),
            plan.access_filter.as_ref(),
        )?;
        let (group_doc, project_doc) = mongo_aggregate_group_and_project(&plan)?;
        let having_doc = mongo_aggregate_having(&plan)?;
        let sort_doc = mongo_aggregate_sort(&plan);

        let pipeline = provider_aggregate_pipeline(
            filter_doc,
            group_doc,
            project_doc,
            having_doc,
            sort_doc,
            use_skip,
            use_limit,
        );

        self.execution
            .aggregate_facet(
                &entity_collection_name(&plan.entity_type),
                pipeline,
                use_skip,
                use_limit,
                request_datetime.to_string(),
                request_start,
            )
            .await
            .map_err(AppError::from)
    }

    #[allow(unused)]
    #[tracing::instrument(
    skip(self, entity_type, selections, input, user, access),
    fields(entity = %entity_type.pascal_1, access_allowed = access.allow)
  )]
    async fn create_item_json(
        &self,
        entity_type: Arc<EntityType>,
        selections: serde_json::Value,
        mut input: JsonObj,
        user: &UserAuth,
        access: &PolicyAccess,
    ) -> Result<JsonObj, AppError> {
        debug!("creating MongoDB item");

        if access.allow {
            let new_id = self.create_primary_key_value(entity_type.clone(), &input)?;

            input.insert(TYPE_PK_NAME.to_string(), new_id.clone());
            if entity_type.is_table && !input.contains_key(RECORD_LOCATOR_FIELD) {
                input.insert(
                    RECORD_LOCATOR_FIELD.to_string(),
                    Value::String(new_record_locator()),
                );
            }

            let filter = provider_mutation_id_filter(new_id.clone(), None);

            self.upsert_item_json(entity_type.clone(), selections, &input, &filter)
                .await
        } else {
            debug!("MongoDB create denied by access policy");
            Err(AppError::AccessDenied)
        }
    }

    // Future enhancement: implement optimistic concurrency control using read_version timestamp
    #[allow(unused)]
    #[tracing::instrument(
    skip(self, entity_type, selections, input, user, access, read_version),
    fields(entity = %entity_type.pascal_1, access_allowed = access.allow, has_read_version = read_version.is_some())
  )]
    async fn update_item_json(
        &self,
        entity_type: Arc<EntityType>,
        selections: serde_json::Value,
        input: JsonObj,
        user: &UserAuth,
        access: &PolicyAccess,
        read_version: Option<Value>,
    ) -> Result<JsonObj, AppError> {
        debug!("updating MongoDB item");

        if access.allow {
            let id = input.get(TYPE_PK_NAME).cloned().ok_or_else(|| {
                AppError::Validation("update requires the primary key".to_string())
            })?;
            let filter = provider_mutation_id_filter(id, read_version);

            let mut remove_attrs: Vec<String> = Vec::new();
            let mut input_clone = input.clone();
            for attr in input.into_iter() {
                let field_name = attr.0.as_str().to_string();
                let prop = AppConfig::get_prop(entity_type.clone(), &field_name)?;
                if !AppConfig::is_native_prop(prop.clone()) {
                    remove_attrs.push(prop.name.clone());
                }
            }
            for remove_attr in remove_attrs {
                input_clone.remove_entry(&remove_attr);
            }
            input_clone.remove_entry(TYPE_PK_NAME);

            let record =
                record::to_bson(self.app_config.clone(), entity_type.clone(), input_clone)?;
            let json_filter = Some(filter.to_owned().into());
            let filter_doc =
                self.compile_filter_with_access(entity_type.clone(), json_filter, access)?;
            let projection = create_projection_for_entity(entity_type.clone(), &selections)?;
            let collection = self.get_entity_collection(&entity_type);

            let result_doc = collection
                .find_one_and_update(filter_doc, provider_mutation_set_update_document(record))
                .return_document(ReturnDocument::After)
                .projection(projection)
                .await
                .map_err(Self::from_mongo_error)?;

            let result_doc = result_doc.ok_or(AppError::InvalidKeyOrVersion)?;
            Ok(bson_doc_to_json_obj(result_doc))
        } else {
            debug!("MongoDB update denied by access policy");
            Err(AppError::AccessDenied)
        }
    }

    #[allow(unused)]
    #[tracing::instrument(
    skip(self, entity_type, input, user, access, read_version),
    fields(entity = %entity_type.pascal_1, access_allowed = access.allow, has_read_version = read_version.is_some())
  )]
    async fn delete_item_json(
        &self,
        entity_type: Arc<EntityType>,
        input: JsonObj,
        user: &UserAuth,
        access: &PolicyAccess,
        read_version: Option<Value>,
    ) -> Result<i64, AppError> {
        debug!("deleting MongoDB item");

        if access.allow {
            let id = input.get(TYPE_PK_NAME).cloned().ok_or_else(|| {
                AppError::Validation("delete requires the primary key".to_string())
            })?;
            let filter = provider_mutation_id_filter(id, read_version);
            let json_filter = Some(filter.to_owned().into());
            let filter_doc =
                self.compile_filter_with_access(entity_type.clone(), json_filter, access)?;

            let collection = self.get_entity_collection(&entity_type);

            let result = collection
                .delete_one(filter_doc)
                .await
                .map_err(Self::from_mongo_error)?;

            if result.deleted_count == 0 {
                return Err(AppError::InvalidKeyOrVersion);
            }

            Ok(result.deleted_count as i64)
        } else {
            debug!("MongoDB delete denied by access policy");
            Err(AppError::AccessDenied)
        }
    }

    // Future enhancement: implement field selection/projection based on GraphQL selections
    #[allow(unused)]
    #[tracing::instrument(
    skip(self, entity_type, selections, id, user, access),
    fields(entity = %entity_type.pascal_1, access_allowed = access.allow, has_id = !id.is_empty())
  )]
    async fn find_item_json(
        &self,
        entity_type: Arc<EntityType>,
        selections: serde_json::Value,
        id: String,
        user: &UserAuth,
        access: &PolicyAccess,
    ) -> Result<Option<JsonObj>, AppError> {
        debug!("finding MongoDB item");

        if access.allow {
            let plan = QueryPlan::new(
                self.app_config.clone(),
                entity_type.clone(),
                selections,
                Some(json!({ TYPE_PK_NAME: id })),
                None,
                0,
                1,
                access,
            )?;
            let filter_doc = self.compile_filter_ast_with_access(
                plan.entity_type.clone(),
                plan.filter.as_ref(),
                plan.access_filter.as_ref(),
            )?;

            debug!(
                filter_field_count = filter_doc.len(),
                "compiled MongoDB find filter"
            );

            let provider_plan = plan.into_runtime_provider_plan();
            let pipeline = provider_get_item_pipeline(
                relationship_lookup_stages_for_plan(&provider_plan)?,
                filter_doc,
                mongo_projection_from_selection(&provider_plan.selection),
            );

            let collection = self.get_entity_collection(&entity_type);
            let mut cursor = collection
                .aggregate(pipeline)
                .await
                .map_err(Self::from_mongo_error)?;
            let document = match cursor.next().await {
                Some(result) => Some(result.map_err(Self::from_mongo_error)?),
                None => None,
            };

            debug!(has_document = document.is_some(), "MongoDB find completed");

            Ok(document.map(bson_doc_to_json_obj))
        } else {
            debug!("MongoDB find denied by access policy");
            Err(AppError::AccessDenied)
        }
    }

    // Future enhancement: implement field selection/projection based on GraphQL selections
    #[allow(unused)]
    #[tracing::instrument(
    skip(self, entity_type, selections, filter, user, access),
    fields(entity = %entity_type.pascal_1, access_allowed = access.allow)
  )]
    async fn get_items_json(
        &self,
        entity_type: Arc<EntityType>,
        selections: serde_json::Value,
        filter: Option<serde_json::Value>,
        user: &UserAuth,
        access: &PolicyAccess,
    ) -> Result<Vec<JsonObj>, AppError> {
        debug!("getting MongoDB items");

        if access.allow {
            let collection = self.get_entity_collection(&entity_type);

            let filter_doc =
                self.compile_filter_with_access(entity_type.clone(), filter, access)?;

            let mut cursor = collection
                .find(filter_doc)
                .await
                .map_err(Self::from_mongo_error)?;

            let mut results = Vec::new();
            while let Some(doc) = cursor.next().await {
                let doc = doc.map_err(Self::from_mongo_error)?;
                results.push(bson_doc_to_json_obj(doc));
            }

            debug!(item_count = results.len(), "got MongoDB items");
            Ok(results)
        } else {
            debug!("MongoDB get-items denied by access policy");
            Err(AppError::AccessDenied)
        }
    }

    #[allow(unused)]
    #[tracing::instrument(
    skip(self, entity_type, selections, filter, sort, user, access),
    fields(entity = %entity_type.pascal_1, access_allowed = access.allow, skip = skip, limit = limit)
  )]
    async fn query_items_json(
        &self,
        entity_type: Arc<EntityType>,
        selections: serde_json::Value,
        filter: Option<Value>,
        sort: Option<Value>,
        skip: i32,
        limit: i32,
        user: &UserAuth,
        access: &PolicyAccess,
    ) -> Result<JsonQueryResult, AppError> {
        debug!("querying MongoDB items");

        if access.allow {
            if skip < 0 {
                return Err(AppError::Validation(
                    "pagination skip must be greater than or equal to 0".to_string(),
                ));
            }
            if limit <= 0 {
                return Err(AppError::Validation(
                    "pagination limit must be greater than 0".to_string(),
                ));
            }
            let use_limit = limit;

            // Metrics
            let request_datetime = OffsetDateTime::now_utc();
            let request_start = Instant::now();

            let projection = create_projection_for_entity(entity_type.clone(), &selections)?;
            // let lookups = &mut self.create_lookups(&selections, entity_type.snake_1);

            let pk_name = self.app_config.get_primary_key_name(entity_type.clone())?;

            // Sort
            let sort_doc = sort::create_if_exists(entity_type.clone(), sort)?;

            // Filter
            let filter_doc =
                self.compile_filter_with_access(entity_type.clone(), filter, access)?;

            debug!(
                filter_field_count = filter_doc.len(),
                "compiled MongoDB query filter"
            );

            let pipeline = provider_query_pipeline(
                Vec::new(),
                filter_doc,
                sort_doc,
                skip,
                use_limit,
                projection,
            );
            // println!("\n > pipeline:: {:?}", pipeline);

            // Execute aggregation
            let mut json_vec: Vec<Map<String, serde_json::Value>> = Vec::new();
            let mut query_count_i32: i32 = 0;
            let col = self.get_entity_collection(&entity_type);
            let mut cursor = col
                .aggregate(pipeline)
                .await
                .map_err(Self::from_mongo_error)?;

            if let Some(result) = cursor.next().await {
                let res_doc = result.map_err(Self::from_mongo_error)?;
                if res_doc.contains_key("items") {
                    let bson_vec = res_doc
                        .get_array("items")
                        .map_err(|e| AppError::DataAccess(e.to_string()))?;
                    json_vec.append(&mut bson_vec_to_json_vec(bson_vec.to_owned()));
                }

                if res_doc.contains_key("query_info") {
                    let query_info = res_doc
                        .get_array("query_info")
                        .map_err(|e| AppError::DataAccess(e.to_string()))?;
                    for item in query_info {
                        let info: bson::Document = bson::from_bson(item.to_owned())
                            .map_err(|e| AppError::DataAccess(e.to_string()))?;
                        query_count_i32 = info
                            .get_i32("count")
                            .map_err(|e| AppError::DataAccess(e.to_string()))?;
                        break;
                    }
                }
            }

            // Prepare & return result
            let page_count = (query_count_i32 / limit) + 1;
            let page_index = skip / limit;
            let date_time = format!("{request_datetime}");
            let request_duration = request_start.elapsed().as_secs_f64();

            let query_count: i64 = query_count_i32 as i64;

            debug!(
                query_count,
                item_count = json_vec.len(),
                "MongoDB query completed"
            );

            Ok(JsonQueryResult {
                date_time,
                request_duration,
                skip,
                limit,
                page_count,
                page_index,
                query_count,
                next_cursor: None,
                previous_cursor: None,
                items: json_vec,
            })
        } else {
            debug!("MongoDB query denied by access policy");
            Err(AppError::AccessDenied)
        }
    }
}
