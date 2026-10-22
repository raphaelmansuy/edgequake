//! Databricks workspace helpers used when the server runs as an App.
//!
//! The Lakebase path is gated on `EDGEQUAKE_DB_AUTH=lakebase_oauth`. A normal
//! `DATABASE_URL` boot does not call Databricks.

mod gateway;
mod lakebase;
mod provider;

pub use lakebase::{resolve_database_url, spawn_credential_refresh, DbUrlError};
pub use provider::{
    embedding_for_model, embedding_provider_from_env, llm_for_model, llm_provider_selected,
    providers_from_env,
};
