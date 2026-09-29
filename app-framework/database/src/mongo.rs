use std::{fs, io::Result, time::Duration};

use bson::{Bson, Document};
use mongodb::{
    options::{ClientOptions, Credential, ServerAddress, Tls, TlsOptions, WriteModel},
    Client,
};
use serde_json::Value;

use appfw_runtime::connection_security::{self, Provider};

use crate::{
    bulk::BackfillCheckpoint,
    console,
    data_source::{DataSource, DataSourceEnvironment, Schema},
    error,
    loader::{get_data_source_schemas, get_schema_path},
    migration_options,
};

pub struct Mongo {
    client: Client,
    db_name: String,
    data_source: DataSource,
}

impl Mongo {
    pub async fn new(data_source: &DataSource) -> Result<Self> {
        let data_source_env = data_source
            .environments
            .first()
            .ok_or_else(|| {
                error::config(format!(
                    "data source `{}` has no selected environment",
                    data_source.name
                ))
            })?
            .to_owned();
        let db_name = data_source_env.db_name.clone();
        let client = Self::get_client(&data_source_env).await?;
        Ok(Self {
            client,
            db_name,
            data_source: data_source.clone(),
        })
    }

    async fn get_client(data_source_env: &DataSourceEnvironment) -> Result<Client> {
        let security = validate_connection_security(data_source_env)?;

        let addresses = vec![ServerAddress::Tcp {
            host: data_source_env.db_host.clone(),
            port: Some(parse_port(&data_source_env.db_port, "MongoDB")?),
        }];

        let credential = Credential::builder()
            .username(data_source_env.service_account_name.clone())
            .password(data_source_env.service_account_password.clone())
            .source("admin".to_string())
            .build();

        let options = ClientOptions::builder()
            .hosts(addresses)
            .credential(credential)
            .connect_timeout(Some(Duration::from_secs(30)))
            .server_selection_timeout(Some(Duration::from_secs(30)))
            .tls(Some(mongo_tls_options(&security)))
            .app_name("app_framework_migrator".to_string())
            .build();

        let client = Client::with_options(options)
            .map_err(|e| error::external(format!("could not create MongoDB client: {e}")))?;

        // Ping admin db to surface connection errors early.
        client
            .database("admin")
            .run_command(bson::doc! { "ping": 1 })
            .await
            .map_err(|e| error::external(format!("MongoDB ping failed: {e}")))?;

        Ok(client)
    }

    pub async fn migrate(&self) -> Result<()> {
        console::step("Mongo migration");

        let schemas = get_data_source_schemas(&self.data_source.name)?;

        for schema in schemas {
            console::item("schema", &schema.name);
            self.migrate_schema(&schema).await?;
        }

        Ok(())
    }

    pub async fn database_exists(&self) -> Result<bool> {
        let names = self
            .client
            .list_database_names()
            .await
            .map_err(|e| error::external(format!("MongoDB database listing failed: {e}")))?;
        Ok(names.iter().any(|name| name == &self.db_name))
    }

    async fn migrate_schema(&self, schema: &Schema) -> Result<()> {
        let schema_path = get_schema_path(schema)?;
        let seed_file = schema_path.join("seed.mongo.json");
        if migration_options::skip_seed() {
            console::skip(
                &seed_file,
                "seed execution skipped by APPFW_MIGRATE_SKIP_SEED",
            );
            return Ok(());
        }
        if !seed_file.exists() {
            console::skip(&seed_file, "file not found");
            return Ok(());
        }

        console::step_path("read seed JSON", &seed_file);
        let json = fs::read_to_string(&seed_file)?;
        let seeds: Value = serde_json::from_str(&json)
            .map_err(|e| error::data(format!("invalid Mongo seed JSON: {e}")))?;

        let seed_array = seeds
            .as_array()
            .ok_or_else(|| error::data("seed.mongo.json root must be an array"))?;

        // Mongo collection names are namespaced by schema (e.g. "crm.accounts") so
        // collections from different schemas do not collide in a single database.
        for entry in seed_array {
            let collection_name = entry["collection"]
                .as_str()
                .ok_or_else(|| error::data("seed entry missing collection"))?;
            let documents = entry["documents"]
                .as_array()
                .ok_or_else(|| error::data("seed entry missing documents"))?;

            let ns_collection = format!("{}.{}", schema.name, collection_name);
            self.upsert_documents(&ns_collection, documents).await?;
        }

        Ok(())
    }

    async fn upsert_documents(&self, collection_name: &str, documents: &[Value]) -> Result<()> {
        if documents.is_empty() {
            return Ok(());
        }

        let db = self.client.database(&self.db_name);
        let coll = db.collection::<Document>(collection_name);

        // Upsert by `id` so re-running the migrator is idempotent.
        let mut inserted = 0usize;
        for doc_json in documents {
            let doc = Self::json_to_bson(doc_json)?;
            let id = doc
                .get_str("id")
                .map_err(|_| error::data("document missing 'id' field"))?
                .to_string();

            let filter = bson::doc! { "id": &id };
            let update = bson::doc! { "$setOnInsert": &doc };
            let opts = mongodb::options::UpdateOptions::builder()
                .upsert(true)
                .build();

            let res = coll
                .update_one(filter, update)
                .with_options(opts)
                .await
                .map_err(|e| error::external(format!("MongoDB upsert failed: {e}")))?;
            if res.upserted_id.is_some() {
                inserted += 1;
            }
        }

        console::step(format!(
            "{}: {} inserted, {} total",
            collection_name,
            inserted,
            documents.len()
        ));
        Ok(())
    }

    #[allow(dead_code)]
    pub async fn bulk_replace_documents(
        &self,
        collection_name: &str,
        documents: &[Value],
    ) -> Result<u64> {
        if documents.is_empty() {
            return Ok(0);
        }

        let db = self.client.database(&self.db_name);
        let coll = db.collection::<Document>(collection_name);
        let mut models = Vec::with_capacity(documents.len());

        for doc_json in documents {
            let doc = Self::json_to_bson(doc_json)?;
            let id = doc
                .get_str("id")
                .map_err(|_| error::data("document missing 'id' field"))?
                .to_string();
            let filter = bson::doc! { "id": &id };
            let mut model = coll
                .replace_one_model(filter, &doc)
                .map_err(|e| error::data(format!("invalid MongoDB bulk model: {e}")))?;
            model.upsert = Some(true);
            models.push(WriteModel::from(model));
        }

        let mut affected = 0u64;
        for chunk in models.chunks(500) {
            let result = self
                .client
                .bulk_write(chunk.iter().cloned())
                .ordered(false)
                .await
                .map_err(|e| error::external(format!("MongoDB bulk_write failed: {e}")))?;
            affected += (result.inserted_count + result.modified_count + result.upserted_count)
                .max(0) as u64;
        }

        Ok(affected)
    }

    #[allow(dead_code)]
    pub async fn backfill_checkpoint(
        &self,
        job_name: &str,
        chunk_key: &str,
    ) -> Result<Option<BackfillCheckpoint>> {
        let db = self.client.database(&self.db_name);
        let coll = db.collection::<Document>("app_meta.backfill_checkpoints");
        let filter = bson::doc! { "job_name": job_name, "chunk_key": chunk_key };
        let Some(doc) = coll
            .find_one(filter)
            .await
            .map_err(|e| error::external(format!("MongoDB checkpoint lookup failed: {e}")))?
        else {
            return Ok(None);
        };

        Ok(Some(BackfillCheckpoint {
            job_name: doc.get_str("job_name").unwrap_or_default().to_string(),
            chunk_key: doc.get_str("chunk_key").unwrap_or_default().to_string(),
            last_value: doc.get_str("last_value").ok().map(str::to_string),
            rows_processed: bson_i64(doc.get("rows_processed")),
        }))
    }

    #[allow(dead_code)]
    pub async fn record_backfill_checkpoint(&self, checkpoint: &BackfillCheckpoint) -> Result<()> {
        let db = self.client.database(&self.db_name);
        let coll = db.collection::<Document>("app_meta.backfill_checkpoints");
        let filter =
            bson::doc! { "job_name": &checkpoint.job_name, "chunk_key": &checkpoint.chunk_key };
        let update = bson::doc! {
            "$set": {
                "last_value": checkpoint.last_value.clone(),
                "rows_processed": checkpoint.rows_processed,
                "updated_at": Bson::DateTime(bson::DateTime::now()),
            },
            "$setOnInsert": {
                "job_name": &checkpoint.job_name,
                "chunk_key": &checkpoint.chunk_key,
            }
        };
        let opts = mongodb::options::UpdateOptions::builder()
            .upsert(true)
            .build();

        coll.update_one(filter, update)
            .with_options(opts)
            .await
            .map_err(|e| error::external(format!("MongoDB checkpoint update failed: {e}")))?;
        Ok(())
    }

    fn json_to_bson(value: &Value) -> Result<Document> {
        // Round-trip through bson::to_bson to preserve types (numbers, bools, nulls).
        let bson_val =
            bson::to_bson(value).map_err(|e| error::data(format!("invalid BSON document: {e}")))?;
        match bson_val {
            bson::Bson::Document(d) => Ok(d),
            _ => Err(error::data("document must be a JSON object")),
        }
    }
}

#[allow(dead_code)]
fn bson_i64(value: Option<&Bson>) -> i64 {
    match value {
        Some(Bson::Int64(value)) => *value,
        Some(Bson::Int32(value)) => i64::from(*value),
        _ => 0,
    }
}

fn validate_connection_security(
    env: &DataSourceEnvironment,
) -> Result<connection_security::ConnectionSecurity> {
    connection_security::validate(
        Provider::MongoDb,
        &env.name,
        &env.security_profile,
        &env.tls_mode,
        &env.db_host,
    )
    .map_err(|e| error::external(format!("invalid MongoDB connection security: {e}")))
}

fn mongo_tls_options(security: &connection_security::ConnectionSecurity) -> Tls {
    if !security.requires_tls() {
        return Tls::Disabled;
    }

    let tls_options = if security.allows_unvalidated_certificate() {
        TlsOptions::builder()
            .allow_invalid_certificates(true)
            .build()
    } else {
        TlsOptions::default()
    };

    Tls::Enabled(tls_options)
}

fn parse_port(value: &str, provider: &str) -> Result<u16> {
    value.parse::<u16>().map_err(|e| {
        error::config(format!(
            "{provider} port `{value}` is not a valid TCP port: {e}"
        ))
    })
}
