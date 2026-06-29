//! v2 async job API (REST resource wrapper over task queue).

mod handlers;
mod submission;
mod types;

pub use crate::services::job_registry::{
    JobCatalogEntry, JobCatalogLinks, JobCatalogResponse, V2MigrationHint,
};
pub use handlers::*;
pub use types::*;
