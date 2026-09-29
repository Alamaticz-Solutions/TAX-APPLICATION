pub mod audit;
pub mod bson_utils;
pub mod connection;
pub mod error;
pub mod execution;
pub mod filter;
pub mod format;
pub mod mutation;
pub mod naming;
pub mod pipeline;
pub mod projection;
pub mod record;
pub mod sort;

pub const TYPE_PK_NAME: &str = "id";
pub const MONGO_PK_NAME: &str = "_id";

pub use audit::{
    audit_sort, collection_name as audit_collection_name,
    document_to_json as audit_document_to_json, event_document as audit_event_document,
    previous_hash_filter as audit_previous_hash_filter, query_filter as audit_query_filter,
    query_limit as audit_query_limit,
};
pub use bson_utils::{
    json_number_to_bson, object_to_objectid, string_to_date_time, string_to_objectid,
    value_vec_to_num_vec, value_vec_to_objectid_vec, value_vec_to_str_vec,
};
pub use connection::{
    mongo_client_options, mongo_connection_summary, mongo_tls_options, MongoConnectionConfig,
    MongoConnectionMode, MongoConnectionSummary, DEFAULT_APP_NAME, DEFAULT_AUTH_SOURCE,
    DEFAULT_CONNECT_TIMEOUT, DEFAULT_MAX_IDLE_TIME, DEFAULT_MAX_POOL_SIZE, DEFAULT_MIN_POOL_SIZE,
    DEFAULT_SERVER_SELECTION_TIMEOUT,
};
pub use error::{classify_mongo_error, classify_mongo_error_code, mongo_runtime_error};
pub use execution::{aggregate_facet_result, query_facet_result, MongoExecutionClient};
pub use filter::{
    create_criterion, filter_field_name, qualify_criterion_fields, value_kind, MongoFilterProperty,
};
pub use format::{
    bson_doc_to_json_obj, bson_to_json_obj, bson_vec_to_json_vec, format_result, format_result_obj,
};
pub use mutation::{
    create_primary_key_value, id_filter as mutation_id_filter, is_empty_client_primary_key,
    new_primary_key_value, set_update_document as mutation_set_update_document,
    set_update_pipeline as mutation_set_update_pipeline,
};
pub use naming::{
    collection_name as mongo_collection_name,
    entity_collection_name as mongo_entity_collection_name,
    junction_collection_name as mongo_junction_collection_name,
    physical_collection_name as mongo_physical_collection_name,
};
pub use pipeline::{
    aggregate_group_and_project, aggregate_having, aggregate_pipeline, aggregate_sort,
    combine_filter_documents, get_item_pipeline, query_pipeline, relationship_lookup_stages,
    selection_projection, MongoAggregateGroup, MongoAggregateHavingPredicate, MongoAggregateMetric,
    MongoAggregateSortSpec, MongoRelationshipLookup, MongoSelectionProjectionNode,
};
pub use projection::{
    create_projection, create_projection_for_fields, mongo_field_name, raw_selection_field_name,
    selection_names, MongoProjectionField,
};
pub use record::{record_field_name, value_to_bson, MongoRecordProperty};
pub use sort::{create_sort_document, normalize_sort, sort_direction};
