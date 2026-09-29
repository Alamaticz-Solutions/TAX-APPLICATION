#![allow(dead_code, unreachable_patterns, unused_imports)]

#[cfg(feature = "http")]
mod admin_ui;
mod config;

pub mod data {
    pub(crate) mod data_access;
    pub(crate) mod query_ir;
    pub(crate) mod rules;

    pub mod clients {
        pub(crate) mod database_client;
        #[cfg(feature = "provider-mongo")]
        pub(crate) mod mongo;
        #[cfg(feature = "provider-mssql")]
        pub(crate) mod mssql;
        #[cfg(feature = "provider-neo4j")]
        pub(crate) mod neo4j;
        #[cfg(feature = "provider-postgres")]
        pub(crate) mod postgres;
        #[cfg(feature = "provider-snowflake")]
        pub(crate) mod snowflake;
    }
}

mod handlers;
#[cfg(feature = "kafka")]
mod kafka;
#[cfg(all(feature = "http", feature = "mcp"))]
mod mcp;
#[cfg(any(feature = "mcp", feature = "kafka"))]
mod operations;
mod product_api;
mod routes;
mod services;
#[cfg(feature = "sync")]
mod sync_workers;

pub mod schemas {
    pub(crate) mod common;
    pub(crate) mod crm;
    pub mod system;
    pub(crate) mod test;
}
