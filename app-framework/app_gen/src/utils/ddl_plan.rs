use std::collections::BTreeMap;

use inflector::cases::tablecase::{is_table_case, to_table_case};
use serde::Serialize;

use crate::bootstrap_types::{
    data_source::DataSourceType,
    entity_type::{DataType, EntityType, PropertyType},
};

const RECORD_LOCATOR_FIELD: &str = "record_locator";

#[derive(Debug, Serialize)]
pub struct DdlPlan {
    pub provider: &'static str,
    pub schema_name: String,
    pub sequences: Vec<SequencePlan>,
    pub tables: Vec<TablePlan>,
    pub audit_tables: Vec<AuditTablePlan>,
    pub indexes: Vec<IndexPlan>,
    pub foreign_keys: Vec<ForeignKeyPlan>,
    pub constraints: Vec<ConstraintPlan>,
    pub junction_tables: Vec<JunctionTablePlan>,
}

#[derive(Debug, Serialize)]
pub struct SequencePlan {
    pub schema_name: String,
    pub name: String,
}

#[derive(Debug, Serialize)]
pub struct TablePlan {
    pub schema_name: String,
    pub name: String,
    pub columns: Vec<ColumnPlan>,
    pub alter_columns: Vec<ColumnPlan>,
}

#[derive(Debug, Serialize, Clone)]
pub struct ColumnPlan {
    pub name: String,
    pub sql_type: &'static str,
    pub create_definition: String,
    pub alter_definition: String,
    pub backfill_expression: Option<String>,
    pub set_not_null_after_backfill: bool,
}

#[derive(Debug, Serialize)]
pub struct AuditTablePlan {
    pub schema_name: String,
    pub source_entity_name: String,
    pub source_table_name: String,
    pub name: String,
    pub columns: Vec<RawColumnPlan>,
    pub migration_columns: Vec<DefaultColumnPlan>,
    pub indexes: Vec<IndexPlan>,
}

#[derive(Debug, Serialize)]
pub struct RawColumnPlan {
    pub name: &'static str,
    pub create_definition: String,
}

#[derive(Debug, Serialize)]
pub struct DefaultColumnPlan {
    pub name: &'static str,
    pub sql_type: &'static str,
    pub default_literal: String,
}

#[derive(Debug, Serialize, Clone)]
pub struct IndexPlan {
    pub name: String,
    pub schema_name: String,
    pub table_name: String,
    pub columns: Vec<String>,
    pub column_list: String,
    pub unique: bool,
}

#[derive(Debug, Serialize)]
pub struct ForeignKeyPlan {
    pub name: String,
    pub source_schema: String,
    pub source_table: String,
    pub source_column: String,
    pub target_schema: String,
    pub target_table: String,
    pub target_column: String,
    pub on_delete: &'static str,
    pub on_update: &'static str,
}

#[derive(Debug, Serialize)]
pub struct ConstraintPlan {
    pub name: String,
    pub schema_name: String,
    pub table_name: String,
    pub definition: String,
}

#[derive(Debug, Serialize)]
pub struct JunctionTablePlan {
    pub schema_name: String,
    pub table_name: String,
    pub raw_table_name: String,
    pub source_schema: String,
    pub source_table: String,
    pub source_entity_name: String,
    pub target_schema: String,
    pub target_table: String,
    pub target_entity_name: String,
    pub local_key: String,
    pub foreign_key: String,
}

#[derive(Debug, Clone, Copy)]
pub enum DdlProvider {
    Postgres,
    MsSql,
    Snowflake,
}

impl DdlProvider {
    pub fn from_data_source_type(data_source_type: DataSourceType) -> Option<Self> {
        match data_source_type {
            DataSourceType::PostgreSQL => Some(Self::Postgres),
            DataSourceType::MsSqlServer => Some(Self::MsSql),
            DataSourceType::Snowflake => Some(Self::Snowflake),
            DataSourceType::FabricSqlAnalytics
            | DataSourceType::MongoDB
            | DataSourceType::Neo4j
            | DataSourceType::ServiceNow
            | DataSourceType::Workday
            | DataSourceType::Icims
            | DataSourceType::Salesforce
            | DataSourceType::Anaplan
            | DataSourceType::OracleFinancials => None,
        }
    }

    fn label(self) -> &'static str {
        match self {
            Self::Postgres => "postgresql",
            Self::MsSql => "mssql",
            Self::Snowflake => "snowflake",
        }
    }
}

pub fn build(provider: DdlProvider, schema_name: &str, entity_types: &[EntityType]) -> DdlPlan {
    let table_entities = entity_types
        .iter()
        .filter(|entity_type| entity_type.is_table)
        .filter(|entity_type| !is_generated_audit_entity(entity_type))
        .collect::<Vec<_>>();

    DdlPlan {
        provider: provider.label(),
        schema_name: schema_name.to_string(),
        sequences: sequences(provider, &table_entities),
        tables: table_entities
            .iter()
            .map(|entity_type| table_plan(provider, entity_type))
            .collect(),
        audit_tables: table_entities
            .iter()
            .filter(|entity_type| has_facet(entity_type, "audited"))
            .map(|entity_type| audit_table_plan(provider, entity_type))
            .collect(),
        indexes: table_entities
            .iter()
            .flat_map(|entity_type| table_indexes(provider, entity_type))
            .collect(),
        foreign_keys: table_entities
            .iter()
            .flat_map(|entity_type| foreign_keys(provider, entity_type))
            .collect(),
        constraints: table_entities
            .iter()
            .flat_map(|entity_type| constraints(entity_type))
            .collect(),
        junction_tables: dedupe_junction_tables(
            table_entities
                .iter()
                .flat_map(|entity_type| junction_tables(entity_type))
                .collect(),
        ),
    }
}

fn is_generated_audit_entity(entity_type: &EntityType) -> bool {
    entity_type
        .meta
        .as_ref()
        .and_then(|meta| meta.get("generatedAuditEntity"))
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(false)
}

fn sequences(provider: DdlProvider, entity_types: &[&EntityType]) -> Vec<SequencePlan> {
    if !matches!(provider, DdlProvider::Postgres) {
        return vec![];
    }

    entity_types
        .iter()
        .filter(|entity_type| {
            entity_type
                .props
                .iter()
                .any(|prop| prop.is_key && is_integer_type(prop.data_type))
        })
        .map(|entity_type| SequencePlan {
            schema_name: entity_type.schema_name.clone(),
            name: format!("{}_seq", entity_type.snake_n),
        })
        .collect()
}

fn table_plan(provider: DdlProvider, entity_type: &EntityType) -> TablePlan {
    let mut native_columns = entity_type
        .props
        .iter()
        .filter(|prop| is_native_data_type(prop.data_type))
        .map(|prop| column_plan(provider, entity_type, prop))
        .collect::<Vec<_>>();
    native_columns.push(record_locator_column_plan(provider, entity_type));
    let alter_columns = native_columns
        .iter()
        .filter(|column| {
            if column.name == RECORD_LOCATOR_FIELD {
                return true;
            }
            entity_type
                .props
                .iter()
                .find(|prop| prop.name == column.name)
                .map(|prop| !prop.is_key)
                .unwrap_or(false)
        })
        .cloned()
        .collect();

    TablePlan {
        schema_name: entity_type.schema_name.clone(),
        name: entity_type.snake_n.clone(),
        columns: native_columns,
        alter_columns,
    }
}

fn column_plan(provider: DdlProvider, entity_type: &EntityType, prop: &PropertyType) -> ColumnPlan {
    let sql_type = data_type(provider, prop.data_type);
    let column_clause = if prop.is_key {
        primary_key_clause(provider, entity_type, prop)
    } else if prop.is_concurrency_control {
        concurrency_column_clause(provider, entity_type, prop)
    } else {
        String::new()
    };
    let column_name = quote_ident(provider, &prop.name);

    ColumnPlan {
        name: prop.name.clone(),
        sql_type,
        create_definition: format!("{column_name} {sql_type}{column_clause}"),
        alter_definition: format!("{column_name} {sql_type}{column_clause}"),
        backfill_expression: None,
        set_not_null_after_backfill: false,
    }
}

fn record_locator_column_plan(provider: DdlProvider, entity_type: &EntityType) -> ColumnPlan {
    let column_name = quote_ident(provider, RECORD_LOCATOR_FIELD);
    let sql_type = data_type(provider, DataType::String);
    let default = match provider {
        DdlProvider::Postgres => {
            "('rl_' || replace(gen_random_uuid()::text, '-', ''))".to_string()
        }
        DdlProvider::MsSql => format!(
            "CONSTRAINT [df_{}_{}] DEFAULT CONCAT(N'rl_', REPLACE(CONVERT(nvarchar(36), NEWID()), N'-', N''))",
            entity_type.snake_n, RECORD_LOCATOR_FIELD
        ),
        DdlProvider::Snowflake => "('rl_' || REPLACE(UUID_STRING(), '-', ''))".to_string(),
    };
    let create_definition = match provider {
        DdlProvider::MsSql => format!("{column_name} {sql_type} NOT NULL {default}"),
        DdlProvider::Postgres | DdlProvider::Snowflake => {
            format!("{column_name} {sql_type} NOT NULL DEFAULT {default}")
        }
    };
    let (alter_definition, backfill_expression, set_not_null_after_backfill) = match provider {
        DdlProvider::Snowflake => (format!("{column_name} {sql_type}"), Some(default), true),
        DdlProvider::Postgres | DdlProvider::MsSql => (create_definition.clone(), None, false),
    };

    ColumnPlan {
        name: RECORD_LOCATOR_FIELD.to_string(),
        sql_type,
        create_definition,
        alter_definition,
        backfill_expression,
        set_not_null_after_backfill,
    }
}

fn concurrency_column_clause(
    provider: DdlProvider,
    entity_type: &EntityType,
    prop: &PropertyType,
) -> String {
    match provider {
        DdlProvider::Postgres | DdlProvider::Snowflake => " NOT NULL DEFAULT 0".to_string(),
        DdlProvider::MsSql => format!(
            " NOT NULL CONSTRAINT [df_{}_{}] DEFAULT 0",
            entity_type.snake_n, prop.name
        ),
    }
}

fn primary_key_clause(
    provider: DdlProvider,
    entity_type: &EntityType,
    prop: &PropertyType,
) -> String {
    match provider {
        DdlProvider::Postgres => {
            if is_integer_type(prop.data_type) {
                format!(
                    " NOT NULL DEFAULT nextval('{}.{}_seq') PRIMARY KEY",
                    entity_type.schema_name, entity_type.snake_n
                )
            } else {
                " NOT NULL DEFAULT gen_random_uuid() PRIMARY KEY".to_string()
            }
        }
        DdlProvider::MsSql => {
            if is_integer_type(prop.data_type) {
                format!(
                    " NOT NULL IDENTITY(1,1) CONSTRAINT [pk_{}] PRIMARY KEY",
                    entity_type.snake_n
                )
            } else {
                format!(
                    " NOT NULL CONSTRAINT [df_{}_{}] DEFAULT NEWID() CONSTRAINT [pk_{}] PRIMARY KEY",
                    entity_type.snake_n, prop.name, entity_type.snake_n
                )
            }
        }
        DdlProvider::Snowflake => {
            if is_integer_type(prop.data_type) {
                " NOT NULL IDENTITY START 1 INCREMENT 1 PRIMARY KEY".to_string()
            } else if matches!(
                prop.data_type,
                DataType::Uuid | DataType::String | DataType::ObjectId
            ) {
                " NOT NULL DEFAULT UUID_STRING() PRIMARY KEY".to_string()
            } else {
                " NOT NULL PRIMARY KEY".to_string()
            }
        }
    }
}

fn audit_table_plan(provider: DdlProvider, entity_type: &EntityType) -> AuditTablePlan {
    let audit_table_name = format!("{}_audit", entity_type.snake_n);
    let columns = audit_columns(provider, &audit_table_name);
    let migration_columns = vec![
        DefaultColumnPlan {
            name: "schema_name",
            sql_type: audit_text_type(provider),
            default_literal: string_literal(provider, &entity_type.schema_name),
        },
        DefaultColumnPlan {
            name: "entity_name",
            sql_type: audit_text_type(provider),
            default_literal: string_literal(provider, &entity_type.pascal_1),
        },
        DefaultColumnPlan {
            name: "table_name",
            sql_type: audit_text_type(provider),
            default_literal: string_literal(provider, &entity_type.snake_n),
        },
        DefaultColumnPlan {
            name: "audit_table_name",
            sql_type: audit_text_type(provider),
            default_literal: string_literal(provider, &audit_table_name),
        },
    ];
    let indexes = match provider {
        DdlProvider::Postgres | DdlProvider::MsSql => vec![
            index_plan(
                provider,
                &entity_type.schema_name,
                &audit_table_name,
                format!("idx_{}_audit_record", entity_type.snake_n),
                vec!["record_id".to_string(), "occurred_at".to_string()],
                false,
            ),
            index_plan(
                provider,
                &entity_type.schema_name,
                &audit_table_name,
                format!("idx_{}_audit_actor", entity_type.snake_n),
                vec!["actor_user_name".to_string(), "occurred_at".to_string()],
                false,
            ),
            index_plan(
                provider,
                &entity_type.schema_name,
                &audit_table_name,
                format!("idx_{}_audit_chain", entity_type.snake_n),
                vec!["chain_scope".to_string(), "occurred_at".to_string()],
                false,
            ),
            index_plan(
                provider,
                &entity_type.schema_name,
                &audit_table_name,
                format!("idx_{}_audit_event_hash", entity_type.snake_n),
                vec!["event_hash".to_string()],
                true,
            ),
        ],
        DdlProvider::Snowflake => vec![],
    };

    AuditTablePlan {
        schema_name: entity_type.schema_name.clone(),
        source_entity_name: entity_type.pascal_1.clone(),
        source_table_name: entity_type.snake_n.clone(),
        name: audit_table_name,
        columns,
        migration_columns,
        indexes,
    }
}

fn audit_columns(provider: DdlProvider, audit_table_name: &str) -> Vec<RawColumnPlan> {
    let text = audit_text_type(provider);
    let text_nullable = audit_nullable_text_type(provider);
    let long_text = audit_long_text_type(provider);
    let json = audit_json_type(provider);
    let timestamp = audit_timestamp_type(provider);
    let audit_id = audit_id_type(provider);
    let action = audit_action_type(provider);
    let chain_scope = audit_chain_scope_type(provider);
    let hash = audit_hash_type(provider);
    let nullable_hash = audit_nullable_hash_type(provider);

    let primary_key = match provider {
        DdlProvider::Postgres | DdlProvider::Snowflake => " PRIMARY KEY".to_string(),
        DdlProvider::MsSql => format!(" CONSTRAINT [pk_{audit_table_name}] PRIMARY KEY"),
    };

    vec![
        raw_col("audit_id", format!("{audit_id} NOT NULL{primary_key}")),
        raw_col("occurred_at", format!("{timestamp} NOT NULL")),
        raw_col("tenant_id", text_nullable.to_string()),
        raw_col("actor_user_name", format!("{text} NOT NULL")),
        raw_col("actor_roles", format!("{json} NOT NULL")),
        raw_col("action", format!("{action} NOT NULL")),
        raw_col("outcome", format!("{action} NOT NULL")),
        raw_col("schema_name", format!("{text} NOT NULL")),
        raw_col("entity_name", format!("{text} NOT NULL")),
        raw_col("table_name", format!("{text} NOT NULL")),
        raw_col("audit_table_name", format!("{text} NOT NULL")),
        raw_col("record_id", text_nullable.to_string()),
        raw_col("before_json", nullable(json)),
        raw_col("after_json", nullable(json)),
        raw_col("diff_json", format!("{json} NOT NULL")),
        raw_col("policy_json", nullable(json)),
        raw_col("redactions_json", format!("{json} NOT NULL")),
        raw_col("chain_scope", format!("{chain_scope} NOT NULL")),
        raw_col("prev_hash", nullable_hash.to_string()),
        raw_col("event_hash", format!("{hash} NOT NULL")),
        raw_col("signature", nullable(long_text)),
    ]
}

fn raw_col(name: &'static str, definition: String) -> RawColumnPlan {
    RawColumnPlan {
        name,
        create_definition: definition,
    }
}

fn table_indexes(provider: DdlProvider, entity_type: &EntityType) -> Vec<IndexPlan> {
    match provider {
        DdlProvider::Postgres | DdlProvider::MsSql => {
            let mut indexes = vec![index_plan(
                provider,
                &entity_type.schema_name,
                &entity_type.snake_n,
                format!("ux_{}_{}", entity_type.snake_n, RECORD_LOCATOR_FIELD),
                vec![RECORD_LOCATOR_FIELD.to_string()],
                true,
            )];
            if let Some(columns) = &entity_type.indexes {
                if !columns.is_empty() {
                    indexes.push(index_plan(
                        provider,
                        &entity_type.schema_name,
                        &entity_type.snake_n,
                        format!("idx_{}_{}", entity_type.snake_n, columns.join("_")),
                        columns.clone(),
                        false,
                    ));
                }
            }

            indexes.extend(
                entity_type
                    .props
                    .iter()
                    .filter(|prop| prop.foreign_key.is_some())
                    .map(|prop| {
                        index_plan(
                            provider,
                            &entity_type.schema_name,
                            &entity_type.snake_n,
                            format!("idx_{}_{}", entity_type.snake_n, prop.name),
                            vec![prop.name.clone()],
                            false,
                        )
                    }),
            );
            indexes
        }
        DdlProvider::Snowflake => vec![],
    }
}

fn index_plan(
    provider: DdlProvider,
    schema_name: &str,
    table_name: &str,
    name: String,
    columns: Vec<String>,
    unique: bool,
) -> IndexPlan {
    let column_list = columns
        .iter()
        .map(|column| quote_ident(provider, column))
        .collect::<Vec<_>>()
        .join(", ");

    IndexPlan {
        name,
        schema_name: schema_name.to_string(),
        table_name: table_name.to_string(),
        columns,
        column_list,
        unique,
    }
}

fn foreign_keys(provider: DdlProvider, entity_type: &EntityType) -> Vec<ForeignKeyPlan> {
    if matches!(provider, DdlProvider::Snowflake) {
        return vec![];
    }

    entity_type
        .props
        .iter()
        .filter_map(|prop| {
            let fk = prop.foreign_key.as_ref()?;
            let target_schema = if fk.schema_name.trim().is_empty() {
                entity_type.schema_name.clone()
            } else {
                fk.schema_name.clone()
            };
            Some(ForeignKeyPlan {
                name: format!("fk_{}_{}", entity_type.snake_n, prop.name),
                source_schema: entity_type.schema_name.clone(),
                source_table: entity_type.snake_n.clone(),
                source_column: prop.name.clone(),
                target_schema,
                target_table: snake_n(&fk.type_name),
                target_column: "id".to_string(),
                on_delete: match provider {
                    DdlProvider::Postgres => "RESTRICT",
                    DdlProvider::MsSql => "NO ACTION",
                    DdlProvider::Snowflake => unreachable!(),
                },
                on_update: match provider {
                    DdlProvider::Postgres => "CASCADE",
                    DdlProvider::MsSql => "NO ACTION",
                    DdlProvider::Snowflake => unreachable!(),
                },
            })
        })
        .collect()
}

fn constraints(entity_type: &EntityType) -> Vec<ConstraintPlan> {
    entity_type
        .constraints
        .clone()
        .unwrap_or_default()
        .into_iter()
        .map(|definition| ConstraintPlan {
            name: definition
                .split_whitespace()
                .next()
                .unwrap_or_default()
                .to_string(),
            schema_name: entity_type.schema_name.clone(),
            table_name: entity_type.snake_n.clone(),
            definition,
        })
        .collect()
}

fn junction_tables(entity_type: &EntityType) -> Vec<JunctionTablePlan> {
    entity_type
        .props
        .iter()
        .filter_map(|prop| {
            let many_to_many = prop.many_to_many_property.as_ref()?;
            let junction_schema = many_to_many
                .junction_schema
                .clone()
                .unwrap_or_else(|| entity_type.schema_name.clone());
            Some(JunctionTablePlan {
                schema_name: junction_schema,
                table_name: snake_n(&many_to_many.junction_table),
                raw_table_name: many_to_many.junction_table.clone(),
                source_schema: entity_type.schema_name.clone(),
                source_table: entity_type.snake_n.clone(),
                source_entity_name: entity_type.pascal_1.clone(),
                target_schema: many_to_many.target_schema.clone(),
                target_table: snake_n(&many_to_many.target_type),
                target_entity_name: many_to_many.target_type.clone(),
                local_key: many_to_many.local_key.clone(),
                foreign_key: many_to_many.foreign_key.clone(),
            })
        })
        .collect()
}

fn dedupe_junction_tables(junction_tables: Vec<JunctionTablePlan>) -> Vec<JunctionTablePlan> {
    let mut tables = BTreeMap::new();
    for junction in junction_tables {
        let key = format!("{}.{}", junction.schema_name, junction.table_name);
        tables.entry(key).or_insert(junction);
    }
    tables.into_values().collect()
}

fn data_type(provider: DdlProvider, data_type: DataType) -> &'static str {
    match provider {
        DdlProvider::Postgres => match data_type {
            DataType::Uuid => "uuid",
            DataType::UuidArray => "uuid[]",
            DataType::ObjectId => "varchar",
            DataType::ObjectIdArray => "varchar[]",
            DataType::Boolean => "boolean",
            DataType::String => "varchar",
            DataType::StringArray => "varchar[]",
            DataType::Date => "date",
            DataType::DateTime => "timestamptz",
            DataType::Time => "time",
            DataType::Int8 | DataType::Int16 => "smallint",
            DataType::Int8Array | DataType::Int16Array => "smallint[]",
            DataType::Int32 => "integer",
            DataType::Int32Array => "integer[]",
            DataType::Int64 => "bigint",
            DataType::Int64Array => "bigint[]",
            DataType::Float32 => "real",
            DataType::Float64 => "double precision",
            DataType::Enum => "varchar",
            DataType::EnumArray => "varchar[]",
            DataType::Object | DataType::Json => "jsonb",
            DataType::ObjectArray | DataType::JsonArray => "jsonb[]",
            DataType::NavToOne | DataType::NavToMany | DataType::ManyToMany => "",
        },
        DdlProvider::MsSql => match data_type {
            DataType::Uuid => "uniqueidentifier",
            DataType::UuidArray => "nvarchar(max)",
            DataType::ObjectId => "nvarchar(64)",
            DataType::ObjectIdArray => "nvarchar(max)",
            DataType::Boolean => "bit",
            DataType::String => "nvarchar(450)",
            DataType::StringArray => "nvarchar(max)",
            DataType::Date => "date",
            DataType::DateTime => "datetimeoffset(7)",
            DataType::Time => "time",
            DataType::Int8 | DataType::Int16 => "smallint",
            DataType::Int8Array | DataType::Int16Array => "nvarchar(max)",
            DataType::Int32 => "int",
            DataType::Int32Array => "nvarchar(max)",
            DataType::Int64 => "bigint",
            DataType::Int64Array => "nvarchar(max)",
            DataType::Float32 => "real",
            DataType::Float64 => "float",
            DataType::Enum => "nvarchar(255)",
            DataType::EnumArray => "nvarchar(max)",
            DataType::Object | DataType::ObjectArray | DataType::Json | DataType::JsonArray => {
                "nvarchar(max)"
            }
            DataType::NavToOne | DataType::NavToMany | DataType::ManyToMany => "",
        },
        DdlProvider::Snowflake => match data_type {
            DataType::Uuid | DataType::ObjectId | DataType::String | DataType::Enum => "VARCHAR",
            DataType::UuidArray
            | DataType::ObjectIdArray
            | DataType::StringArray
            | DataType::EnumArray
            | DataType::Object
            | DataType::ObjectArray
            | DataType::Json
            | DataType::JsonArray => "VARIANT",
            DataType::Boolean => "BOOLEAN",
            DataType::Date => "DATE",
            DataType::DateTime => "TIMESTAMP_TZ",
            DataType::Time => "TIME",
            DataType::Int8 => "NUMBER(3,0)",
            DataType::Int8Array => "VARIANT",
            DataType::Int16 => "NUMBER(5,0)",
            DataType::Int16Array => "VARIANT",
            DataType::Int32 => "NUMBER(10,0)",
            DataType::Int32Array => "VARIANT",
            DataType::Int64 => "NUMBER(19,0)",
            DataType::Int64Array => "VARIANT",
            DataType::Float32 | DataType::Float64 => "FLOAT",
            DataType::NavToOne | DataType::NavToMany | DataType::ManyToMany => "",
        },
    }
}

fn quote_ident(provider: DdlProvider, ident: &str) -> String {
    match provider {
        DdlProvider::Postgres | DdlProvider::Snowflake => format!("\"{ident}\""),
        DdlProvider::MsSql => format!("[{ident}]"),
    }
}

fn string_literal(provider: DdlProvider, value: &str) -> String {
    let escaped = value.replace('\'', "''");
    match provider {
        DdlProvider::MsSql => format!("N'{escaped}'"),
        DdlProvider::Postgres | DdlProvider::Snowflake => format!("'{escaped}'"),
    }
}

fn audit_text_type(provider: DdlProvider) -> &'static str {
    match provider {
        DdlProvider::Postgres => "varchar",
        DdlProvider::MsSql => "nvarchar(255)",
        DdlProvider::Snowflake => "VARCHAR",
    }
}

fn audit_id_type(provider: DdlProvider) -> &'static str {
    match provider {
        DdlProvider::Postgres => "varchar",
        DdlProvider::MsSql => "nvarchar(64)",
        DdlProvider::Snowflake => "VARCHAR",
    }
}

fn audit_action_type(provider: DdlProvider) -> &'static str {
    match provider {
        DdlProvider::Postgres => "varchar",
        DdlProvider::MsSql => "nvarchar(64)",
        DdlProvider::Snowflake => "VARCHAR",
    }
}

fn audit_chain_scope_type(provider: DdlProvider) -> &'static str {
    match provider {
        DdlProvider::Postgres => "varchar",
        DdlProvider::MsSql => "nvarchar(512)",
        DdlProvider::Snowflake => "VARCHAR",
    }
}

fn audit_hash_type(provider: DdlProvider) -> &'static str {
    match provider {
        DdlProvider::Postgres => "varchar",
        DdlProvider::MsSql => "nvarchar(128)",
        DdlProvider::Snowflake => "VARCHAR",
    }
}

fn audit_nullable_hash_type(provider: DdlProvider) -> &'static str {
    match provider {
        DdlProvider::Postgres => "varchar",
        DdlProvider::MsSql => "nvarchar(128) NULL",
        DdlProvider::Snowflake => "VARCHAR",
    }
}

fn audit_nullable_text_type(provider: DdlProvider) -> &'static str {
    match provider {
        DdlProvider::Postgres => "varchar",
        DdlProvider::MsSql => "nvarchar(255) NULL",
        DdlProvider::Snowflake => "VARCHAR",
    }
}

fn audit_long_text_type(provider: DdlProvider) -> &'static str {
    match provider {
        DdlProvider::Postgres => "varchar",
        DdlProvider::MsSql => "nvarchar(max)",
        DdlProvider::Snowflake => "VARCHAR",
    }
}

fn audit_json_type(provider: DdlProvider) -> &'static str {
    match provider {
        DdlProvider::Postgres => "jsonb",
        DdlProvider::MsSql => "nvarchar(max)",
        DdlProvider::Snowflake => "VARIANT",
    }
}

fn audit_timestamp_type(provider: DdlProvider) -> &'static str {
    match provider {
        DdlProvider::Postgres => "timestamptz",
        DdlProvider::MsSql => "datetimeoffset(7)",
        DdlProvider::Snowflake => "TIMESTAMP_TZ",
    }
}

fn nullable(sql_type: &str) -> String {
    format!("{sql_type} NULL")
}

fn is_native_data_type(data_type: DataType) -> bool {
    !matches!(
        data_type,
        DataType::NavToOne | DataType::NavToMany | DataType::ManyToMany
    )
}

fn is_integer_type(data_type: DataType) -> bool {
    matches!(
        data_type,
        DataType::Int8 | DataType::Int16 | DataType::Int32 | DataType::Int64
    )
}

fn has_facet(entity_type: &EntityType, facet: &str) -> bool {
    entity_type
        .facets
        .as_ref()
        .map(|facets| facets.iter().any(|candidate| candidate == facet))
        .unwrap_or(false)
}

fn snake_n(value: &str) -> String {
    if is_table_case(value) {
        value.to_string()
    } else {
        to_table_case(value)
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;
    use crate::bootstrap_types::entity_type::Computed;

    fn prop(name: &str, data_type: DataType, is_key: bool) -> PropertyType {
        PropertyType {
            id: format!("test.Account.{name}"),
            name: name.to_string(),
            caption: name.to_string(),
            is_key,
            is_caption: false,
            is_required: is_key,
            is_read_only: false,
            is_concurrency_control: false,
            data_type,
            computed: Computed::None,
            default_value: None,
            foreign_key: None,
            nav_by_fk_property: None,
            many_to_many_property: None,
            nested_entity_type: None,
            enum_type_name: None,
            meta: None,
        }
    }

    fn account_entity() -> EntityType {
        EntityType {
            id: "test.Account".to_string(),
            schema_name: "crm".to_string(),
            schema_id: None,
            pascal_1: "Account".to_string(),
            pascal_n: "Accounts".to_string(),
            snake_1: "account".to_string(),
            snake_n: "accounts".to_string(),
            caption_1: "Account".to_string(),
            caption_n: "Accounts".to_string(),
            is_union: false,
            base_type: None,
            is_table: true,
            facets: None,
            indexes: None,
            constraints: None,
            meta: None,
            execution: None,
            standard_methods: None,
            custom_methods: None,
            props: vec![
                prop("id", DataType::Uuid, true),
                prop("name", DataType::String, false),
            ],
        }
    }

    fn generated_audit_entity() -> EntityType {
        EntityType {
            id: "test.AccountAudit".to_string(),
            schema_name: "crm".to_string(),
            schema_id: None,
            pascal_1: "AccountAudit".to_string(),
            pascal_n: "AccountAudits".to_string(),
            snake_1: "account_audit".to_string(),
            snake_n: "accounts_audit".to_string(),
            caption_1: "Account audit".to_string(),
            caption_n: "Account audits".to_string(),
            is_union: false,
            base_type: None,
            is_table: true,
            facets: None,
            indexes: None,
            constraints: None,
            meta: Some(json!({ "generatedAuditEntity": true })),
            execution: None,
            standard_methods: None,
            custom_methods: None,
            props: vec![prop("audit_id", DataType::String, true)],
        }
    }

    fn record_locator_column(plan: &DdlPlan) -> &ColumnPlan {
        plan.tables[0]
            .columns
            .iter()
            .find(|column| column.name == RECORD_LOCATOR_FIELD)
            .expect("record locator column")
    }

    fn record_locator_index(plan: &DdlPlan) -> Option<&IndexPlan> {
        plan.indexes
            .iter()
            .find(|index| index.columns == vec![RECORD_LOCATOR_FIELD.to_string()])
    }

    #[test]
    fn postgres_record_locator_column_default_and_unique_index_are_planned() {
        let plan = build(DdlProvider::Postgres, "crm", &[account_entity()]);
        let column = record_locator_column(&plan);

        assert_eq!(column.sql_type, "varchar");
        assert_eq!(
            column.create_definition,
            "\"record_locator\" varchar NOT NULL DEFAULT ('rl_' || replace(gen_random_uuid()::text, '-', ''))"
        );
        assert_eq!(column.alter_definition, column.create_definition);
        assert!(column.backfill_expression.is_none());
        assert!(!column.set_not_null_after_backfill);

        let index = record_locator_index(&plan).expect("record locator index");
        assert_eq!(index.name, "ux_accounts_record_locator");
        assert_eq!(index.column_list, "\"record_locator\"");
        assert!(index.unique);
    }

    #[test]
    fn mssql_record_locator_column_default_and_unique_index_are_planned() {
        let plan = build(DdlProvider::MsSql, "crm", &[account_entity()]);
        let column = record_locator_column(&plan);

        assert_eq!(column.sql_type, "nvarchar(450)");
        assert_eq!(
            column.create_definition,
            "[record_locator] nvarchar(450) NOT NULL CONSTRAINT [df_accounts_record_locator] DEFAULT CONCAT(N'rl_', REPLACE(CONVERT(nvarchar(36), NEWID()), N'-', N''))"
        );
        assert_eq!(column.alter_definition, column.create_definition);
        assert!(column.backfill_expression.is_none());
        assert!(!column.set_not_null_after_backfill);

        let index = record_locator_index(&plan).expect("record locator index");
        assert_eq!(index.name, "ux_accounts_record_locator");
        assert_eq!(index.column_list, "[record_locator]");
        assert!(index.unique);
    }

    #[test]
    fn snowflake_record_locator_create_default_and_migration_backfill_are_planned() {
        let plan = build(DdlProvider::Snowflake, "crm", &[account_entity()]);
        let column = record_locator_column(&plan);

        assert_eq!(column.sql_type, "VARCHAR");
        assert_eq!(
            column.create_definition,
            "\"record_locator\" VARCHAR NOT NULL DEFAULT ('rl_' || REPLACE(UUID_STRING(), '-', ''))"
        );
        assert_eq!(column.alter_definition, "\"record_locator\" VARCHAR");
        assert_eq!(
            column.backfill_expression.as_deref(),
            Some("('rl_' || REPLACE(UUID_STRING(), '-', ''))")
        );
        assert!(column.set_not_null_after_backfill);
        assert!(record_locator_index(&plan).is_none());
    }

    #[test]
    fn generated_audit_entities_are_not_planned_as_locator_tables() {
        let plan = build(DdlProvider::Postgres, "crm", &[generated_audit_entity()]);

        assert!(plan.tables.is_empty());
        assert!(plan.indexes.is_empty());
    }
}
