//! Databricks workspace helpers used when the server runs as an App.
//!
//! The Lakebase path is gated on `EDGEQUAKE_DB_AUTH=lakebase_oauth`. A normal
//! `DATABASE_URL` boot does not call Databricks.

mod lakebase;

pub use lakebase::{resolve_database_url, spawn_credential_refresh, DbUrlError};
