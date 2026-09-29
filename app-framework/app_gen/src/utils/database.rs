use std::path::PathBuf;

use anyhow::Result;

use crate::app_workspace::AppWorkspace;
use crate::bootstrap_types::data_source::{DataSourceType, Schema};
use crate::utils::constants::ctx_name;
use crate::utils::{
    artifacts::{self, Artifact, OverwriteMode},
    console, context,
    ddl_plan::{self, DdlProvider},
    engine, files,
};

pub struct Postgres {}

impl Postgres {
    pub fn generate(
        workspace: &AppWorkspace,
        schema_dir_name: &str,
        schema: &Schema,
        target_schema_dir: &PathBuf,
    ) -> Result<()> {
        SqlDialect::postgres().generate(workspace, schema_dir_name, schema, target_schema_dir)
    }
}

pub struct MsSql {}

impl MsSql {
    pub fn generate(
        workspace: &AppWorkspace,
        schema_dir_name: &str,
        schema: &Schema,
        target_schema_dir: &PathBuf,
    ) -> Result<()> {
        SqlDialect::mssql().generate(workspace, schema_dir_name, schema, target_schema_dir)
    }
}

pub struct Snowflake {}

impl Snowflake {
    pub fn generate(
        workspace: &AppWorkspace,
        schema_dir_name: &str,
        schema: &Schema,
        target_schema_dir: &PathBuf,
    ) -> Result<()> {
        SqlDialect::snowflake().generate(workspace, schema_dir_name, schema, target_schema_dir)
    }
}

pub struct Mongo {}

impl Mongo {
    pub fn generate(
        workspace: &AppWorkspace,
        schema_dir_name: &str,
        schema: &Schema,
        target_schema_dir: &PathBuf,
    ) -> Result<()> {
        let seeds_dir = files::get_seeds_dir(workspace, schema_dir_name);
        if !seeds_dir.exists() {
            console::skip(&seeds_dir, "no seeds dir");
            return Ok(());
        }

        console::step(format!("Mongo seed JSON: {schema_dir_name}"));

        let templates_dir =
            files::get_database_templates_dir(workspace, DataSourceType::MongoDB).join("seed_json");
        let target_file = target_schema_dir.join("seed.mongo.json");

        let tera = engine::create(workspace, &templates_dir)?;
        let context = &mut context::create_for_schema(schema)?;
        context::load_dir(context, &seeds_dir, ctx_name::SEEDS)?;

        let output = tera.render("seed_json", context)?;
        emit_text(target_file, output)
    }
}

#[derive(Clone, Copy)]
struct SqlDialect {
    label: &'static str,
    data_source_type: DataSourceType,
    tables_file_name: &'static str,
    seed_file_name: &'static str,
}

impl SqlDialect {
    fn postgres() -> Self {
        Self {
            label: "Postgres",
            data_source_type: DataSourceType::PostgreSQL,
            tables_file_name: "tables.pg.sql",
            seed_file_name: "seed.pg.sql",
        }
    }

    fn mssql() -> Self {
        Self {
            label: "MsSql",
            data_source_type: DataSourceType::MsSqlServer,
            tables_file_name: "tables.mssql.sql",
            seed_file_name: "seed.mssql.sql",
        }
    }

    fn snowflake() -> Self {
        Self {
            label: "Snowflake",
            data_source_type: DataSourceType::Snowflake,
            tables_file_name: "tables.snowflake.sql",
            seed_file_name: "seed.snowflake.sql",
        }
    }

    fn generate(
        self,
        workspace: &AppWorkspace,
        schema_dir_name: &str,
        schema: &Schema,
        target_schema_dir: &PathBuf,
    ) -> Result<()> {
        self.gen_tables_sql(workspace, schema_dir_name, schema, target_schema_dir)?;
        self.gen_seed_sql(workspace, schema_dir_name, schema, target_schema_dir)
    }

    fn gen_tables_sql(
        self,
        workspace: &AppWorkspace,
        schema_dir_name: &str,
        schema: &Schema,
        target_schema_dir: &PathBuf,
    ) -> Result<()> {
        console::step(format!("{} tables: {schema_dir_name}", self.label));

        let target_file = target_schema_dir.join(self.tables_file_name);
        if schema_is_external_read_only(schema) {
            return emit_text(
                target_file,
                format!(
                    "-- DDL skipped: schema `{}` is configured as external read-only storage.\n",
                    schema.name
                ),
            );
        }

        let templates_dir = files::get_database_templates_dir(workspace, self.data_source_type)
            .join("create_tables_sql");
        let entity_types_file = files::get_entity_types_file(workspace, schema_dir_name);

        let tera = engine::create(workspace, &templates_dir)?;
        let context = &mut context::create_for_schema(schema)?;
        let entity_types = context::read_types(&entity_types_file)?;
        if let Some(provider) = DdlProvider::from_data_source_type(self.data_source_type) {
            let plan = ddl_plan::build(provider, &schema.name, &entity_types);
            context.insert("ddl", &plan);
        }
        context.insert(ctx_name::ENTITY_TYPES, &entity_types);

        let output = tera.render("create_tables_sql", context)?;
        emit_text(target_file, output)
    }

    fn gen_seed_sql(
        self,
        workspace: &AppWorkspace,
        schema_dir_name: &str,
        schema: &Schema,
        target_schema_dir: &PathBuf,
    ) -> Result<()> {
        let seeds_dir = files::get_seeds_dir(workspace, schema_dir_name);
        if !seeds_dir.exists() {
            console::skip(&seeds_dir, "no seeds dir");
            return Ok(());
        }

        console::step(format!("{} seed SQL: {schema_dir_name}", self.label));

        let templates_dir =
            files::get_database_templates_dir(workspace, self.data_source_type).join("seed_sql");
        let target_file = target_schema_dir.join(self.seed_file_name);

        let tera = engine::create(workspace, &templates_dir)?;
        let context = &mut context::create_for_schema(schema)?;
        context::load_dir(context, &seeds_dir, ctx_name::SEEDS)?;

        let output = tera.render("seed_sql", context)?;
        emit_text(target_file, output)
    }
}

fn schema_is_external_read_only(schema: &Schema) -> bool {
    let Some(meta) = schema.meta.as_ref() else {
        return false;
    };
    if meta
        .get("storage")
        .and_then(|storage| storage.get("external_read_only"))
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(false)
    {
        return true;
    }
    meta.get("storage")
        .and_then(|storage| storage.get("mode"))
        .and_then(serde_json::Value::as_str)
        .is_some_and(|mode| mode == "external_read_only")
}

fn emit_text(target_file: PathBuf, output: String) -> Result<()> {
    artifacts::emit(&Artifact::generated_text(
        target_file,
        output,
        OverwriteMode::Always,
    ))
}
