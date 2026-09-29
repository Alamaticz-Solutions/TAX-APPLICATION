use std::sync::atomic::{AtomicU8, Ordering};
use std::time::Instant;

use appfw_runtime::{
    provider_error as runtime_provider_error, provider_keys::FrameworkProvider, RuntimeAuditEvent,
    RuntimeAuditQuery, RuntimeError, RuntimeJsonAggregateResult, RuntimeJsonObj,
    RuntimeJsonQueryResult,
};
use bson::{doc, Bson, Document};
use futures::StreamExt;
use mongodb::options::{Acknowledgment, ReadConcern, TransactionOptions, WriteConcern};
use mongodb::{error::Error as MongoError, ClientSession};
use mongodb::{Client, Collection};
use serde_json::Value;

use crate::{
    audit_collection_name, audit_document_to_json, audit_event_document,
    audit_previous_hash_filter, audit_query_filter, audit_query_limit, audit_sort,
    bson_doc_to_json_obj, bson_vec_to_json_vec,
};

/// Cached result of probing the deployment for multi-document transaction
/// support. Transactions require a replica set or sharded (mongos) cluster;
/// standalone `mongod` rejects them. We probe once and remember the answer.
const TXN_SUPPORT_UNKNOWN: u8 = 0;
const TXN_SUPPORT_YES: u8 = 1;
const TXN_SUPPORT_NO: u8 = 2;

#[derive(Clone)]
pub struct MongoExecutionClient {
    client: Client,
    db_name: String,
    txn_support: std::sync::Arc<AtomicU8>,
}

impl MongoExecutionClient {
    pub fn new(client: Client, db_name: impl Into<String>) -> Self {
        Self {
            client,
            db_name: db_name.into(),
            txn_support: std::sync::Arc::new(AtomicU8::new(TXN_SUPPORT_UNKNOWN)),
        }
    }

    /// Probe (once, then cache) whether the connected deployment supports
    /// multi-document transactions. Transactions are only valid against a
    /// replica set member or a `mongos` router; a standalone `mongod` reports
    /// neither `setName` nor the `isdbgrid` marker in its `hello` response.
    pub async fn supports_transactions(&self) -> bool {
        match self.txn_support.load(Ordering::Relaxed) {
            TXN_SUPPORT_YES => return true,
            TXN_SUPPORT_NO => return false,
            _ => {}
        }
        let supported = match self
            .client
            .database("admin")
            .run_command(doc! { "hello": 1 })
            .await
        {
            Ok(reply) => {
                let is_replica_set = reply.get_str("setName").is_ok();
                let is_mongos = reply
                    .get_str("msg")
                    .map(|m| m == "isdbgrid")
                    .unwrap_or(false);
                is_replica_set || is_mongos
            }
            Err(_) => {
                // Be conservative: if we cannot determine topology, assume no
                // transaction support so we never hard-break standalone local.
                false
            }
        };
        self.txn_support.store(
            if supported {
                TXN_SUPPORT_YES
            } else {
                TXN_SUPPORT_NO
            },
            Ordering::Relaxed,
        );
        supported
    }

    /// Run a multi-document write body inside a MongoDB transaction when the
    /// deployment supports one, falling back to a plain session-less execution
    /// otherwise.
    ///
    /// Gating:
    /// - Replica set / `mongos`: a real transaction is used (`start_session` +
    ///   `start_transaction` + commit/abort).
    /// - Standalone with `require_transactions = false` (local dev): logs a
    ///   warning and runs the body without a session so local MongoDB keeps
    ///   working. Mutations are still single-document atomic at the server.
    /// - Standalone with `require_transactions = true` (managed env): fails
    ///   closed rather than silently writing without transactional guarantees.
    ///
    /// The body receives an `Option<&mut ClientSession>` — `Some` when running
    /// transactionally, `None` on the standalone fall-back path — so callers
    /// can thread the session into their collection operations.
    ///
    /// When the deployment does not support transactions and they are not
    /// required, `fell_back` in the returned tuple is `true` so the caller can
    /// emit a warning at its own logging layer (this crate is logging-free).
    pub async fn with_transaction<R, F>(
        &self,
        require_transactions: bool,
        body: F,
    ) -> Result<(R, bool), RuntimeError>
    where
        F: for<'s> FnOnce(
            Option<&'s mut ClientSession>,
        ) -> futures::future::BoxFuture<'s, Result<R, RuntimeError>>,
    {
        if !self.supports_transactions().await {
            if require_transactions {
                return Err(RuntimeError::DataAccess(
                    "MongoDB transactions are required in this environment but the deployment is \
                     not a replica set or sharded cluster"
                        .to_string(),
                ));
            }
            // Standalone local: run without a session. Mutations remain
            // single-document atomic at the server.
            return body(None).await.map(|value| (value, true));
        }

        let mut session = self.client.start_session().await.map_err(mongo_error)?;
        let txn_options = TransactionOptions::builder()
            .read_concern(ReadConcern::majority())
            .write_concern(WriteConcern::builder().w(Acknowledgment::Majority).build())
            .build();
        session
            .start_transaction()
            .with_options(txn_options)
            .await
            .map_err(mongo_error)?;

        match body(Some(&mut session)).await {
            Ok(value) => {
                session.commit_transaction().await.map_err(mongo_error)?;
                Ok((value, false))
            }
            Err(err) => {
                // Best-effort abort; surface the original error regardless.
                let _ = session.abort_transaction().await;
                Err(err)
            }
        }
    }

    pub fn collection(&self, collection_name: &str) -> Collection<Document> {
        self.client
            .database(&self.db_name)
            .collection::<Document>(collection_name)
    }

    pub fn db_name(&self) -> &str {
        &self.db_name
    }

    pub async fn health_check(&self) -> Result<(), RuntimeError> {
        self.client
            .database("admin")
            .run_command(doc! {"ping": 1})
            .await
            .map_err(mongo_error)?;
        Ok(())
    }

    pub async fn append_audit_event(&self, event: RuntimeAuditEvent) -> Result<(), RuntimeError> {
        let collection = self.collection(&audit_collection_name(
            &event.schema_name,
            &event.audit_table_name,
        ));
        let prev_hash = collection
            .find_one(audit_previous_hash_filter(&event))
            .sort(audit_sort())
            .await
            .map_err(mongo_error)?
            .and_then(|doc| doc.get_str("event_hash").ok().map(ToString::to_string));
        let event = event.finalize(prev_hash)?;
        let doc = audit_event_document(&event)?;
        collection.insert_one(doc).await.map_err(mongo_error)?;
        Ok(())
    }

    pub async fn query_audit_events(
        &self,
        query: RuntimeAuditQuery,
    ) -> Result<Vec<Value>, RuntimeError> {
        let collection = self.collection(&audit_collection_name(
            &query.schema_name,
            &query.audit_table_name,
        ));
        let mut cursor = collection
            .find(audit_query_filter(&query))
            .sort(audit_sort())
            .limit(audit_query_limit(&query))
            .await
            .map_err(mongo_error)?;
        let mut events = Vec::new();
        while let Some(doc) = cursor.next().await {
            let doc = doc.map_err(mongo_error)?;
            events.push(audit_document_to_json(doc)?);
        }
        Ok(events)
    }

    pub async fn find_documents(
        &self,
        collection_name: &str,
        filter: Document,
    ) -> Result<Vec<RuntimeJsonObj>, RuntimeError> {
        let mut cursor = self
            .collection(collection_name)
            .find(filter)
            .await
            .map_err(mongo_error)?;
        let mut results = Vec::new();
        while let Some(doc) = cursor.next().await {
            let doc = doc.map_err(mongo_error)?;
            results.push(bson_doc_to_json_obj(doc));
        }
        Ok(results)
    }

    pub async fn query_facet(
        &self,
        collection_name: &str,
        pipeline: Vec<Document>,
        skip: i32,
        limit: i32,
        request_datetime: String,
        request_start: Instant,
    ) -> Result<RuntimeJsonQueryResult, RuntimeError> {
        let doc = self
            .first_aggregate_document(collection_name, pipeline)
            .await?;
        query_facet_result(doc.as_ref(), skip, limit, request_datetime, request_start)
    }

    pub async fn aggregate_facet(
        &self,
        collection_name: &str,
        pipeline: Vec<Document>,
        skip: i32,
        limit: i32,
        request_datetime: String,
        request_start: Instant,
    ) -> Result<RuntimeJsonAggregateResult, RuntimeError> {
        let doc = self
            .first_aggregate_document(collection_name, pipeline)
            .await?;
        aggregate_facet_result(doc.as_ref(), skip, limit, request_datetime, request_start)
    }

    async fn first_aggregate_document(
        &self,
        collection_name: &str,
        pipeline: Vec<Document>,
    ) -> Result<Option<Document>, RuntimeError> {
        let mut cursor = self
            .collection(collection_name)
            .aggregate(pipeline)
            .await
            .map_err(mongo_error)?;
        match cursor.next().await {
            Some(result) => result.map(Some).map_err(mongo_error),
            None => Ok(None),
        }
    }
}

pub fn query_facet_result(
    result: Option<&Document>,
    skip: i32,
    limit: i32,
    request_datetime: String,
    request_start: Instant,
) -> Result<RuntimeJsonQueryResult, RuntimeError> {
    let (items, query_count) = match result {
        Some(doc) => (facet_items(doc)?, facet_query_count(doc)?),
        None => (Vec::new(), 0),
    };
    Ok(RuntimeJsonQueryResult::new(
        skip,
        limit,
        query_count,
        items,
        request_datetime,
        request_start,
    ))
}

pub fn aggregate_facet_result(
    result: Option<&Document>,
    skip: i32,
    limit: i32,
    request_datetime: String,
    request_start: Instant,
) -> Result<RuntimeJsonAggregateResult, RuntimeError> {
    let (items, query_count) = match result {
        Some(doc) => (
            facet_items(doc)?.into_iter().map(Value::Object).collect(),
            facet_query_count(doc)?,
        ),
        None => (Vec::new(), 0),
    };
    Ok(RuntimeJsonAggregateResult::new(
        skip,
        limit,
        query_count,
        items,
        request_datetime,
        request_start,
    ))
}

fn facet_items(doc: &Document) -> Result<Vec<RuntimeJsonObj>, RuntimeError> {
    match doc.get_array("items") {
        Ok(items) => Ok(bson_vec_to_json_vec(items.to_owned())),
        Err(_) => Ok(Vec::new()),
    }
}

fn facet_query_count(doc: &Document) -> Result<i64, RuntimeError> {
    let Ok(query_info) = doc.get_array("query_info") else {
        return Ok(0);
    };
    let Some(info) = query_info.first().and_then(Bson::as_document) else {
        return Ok(0);
    };
    info.get_i64("count")
        .or_else(|_| info.get_i32("count").map(i64::from))
        .map_err(|e| RuntimeError::DataAccess(e.to_string()))
}

fn mongo_error(error: MongoError) -> RuntimeError {
    let message = error.to_string();
    if let Some(kind) = classify_mongo_error(&error) {
        RuntimeError::DataStore(runtime_provider_error::stable_provider_error(
            FrameworkProvider::Mongo,
            message,
            kind,
        ))
    } else {
        RuntimeError::DataStore(runtime_provider_error::normalize_provider_error(
            FrameworkProvider::Mongo,
            message,
        ))
    }
}

fn classify_mongo_error(error: &MongoError) -> Option<appfw_runtime::DataStoreError> {
    use mongodb::error::{ErrorKind, WriteFailure};

    let code = match error.kind.as_ref() {
        ErrorKind::Command(command) => Some(command.code),
        ErrorKind::InsertMany(insert_many) => insert_many
            .write_errors
            .as_ref()
            .and_then(|errors| errors.first())
            .map(|error| error.code),
        ErrorKind::Write(WriteFailure::WriteError(write_error)) => Some(write_error.code),
        ErrorKind::Write(WriteFailure::WriteConcernError(write_error)) => Some(write_error.code),
        _ => None,
    };

    code.and_then(runtime_provider_error::classify_mongo_code)
}

#[cfg(test)]
mod tests {
    use bson::{doc, oid::ObjectId};
    use serde_json::json;

    use super::*;

    #[test]
    fn query_facet_result_decodes_items_and_count() {
        let id = ObjectId::parse_str("6568f0f00000000000000001").expect("object id");
        let result = doc! {
            "items": [
                { "_id": id, "name": "Acme" },
                { "id": "custom", "name": "Zenith" }
            ],
            "query_info": [{ "count": 2_i32 }]
        };

        let page = query_facet_result(Some(&result), 0, 10, "now".to_string(), Instant::now())
            .expect("query page");

        assert_eq!(page.query_count, 2);
        assert_eq!(page.items.len(), 2);
        assert_eq!(page.items[0]["id"], json!("6568f0f00000000000000001"));
        assert_eq!(page.items[1]["id"], json!("custom"));
    }

    #[test]
    fn aggregate_facet_result_decodes_values_and_i64_count() {
        let result = doc! {
            "items": [
                { "is_active": true, "count": 3_i32 }
            ],
            "query_info": [{ "count": 1_i64 }]
        };

        let page = aggregate_facet_result(Some(&result), 0, 10, "now".to_string(), Instant::now())
            .expect("aggregate page");

        assert_eq!(page.query_count, 1);
        assert_eq!(page.items, vec![json!({ "is_active": true, "count": 3 })]);
    }

    #[test]
    fn empty_facet_result_returns_empty_page() {
        let page =
            query_facet_result(None, 0, 10, "now".to_string(), Instant::now()).expect("empty page");

        assert_eq!(page.query_count, 0);
        assert!(page.items.is_empty());
    }
}
