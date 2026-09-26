//! P0 delete authority for hand-built `AppState` harnesses.
//!
//! `perform_document_deletion` refuses physical cleanup without a lifecycle
//! committer (tombstone authority). Harnesses that keep the legacy inline
//! ingest path still need the same `PgIngestionCommitter` production wires
//! as `lifecycle_committer` + `document_reader`.

use std::sync::Arc;

use edgequake_storage::contracts::{DocumentReader, LifecycleCommitter};
use edgequake_storage::PgIngestionCommitter;
use sqlx::PgPool;

pub struct P0DeleteAuthority {
    pub lifecycle_committer: Option<Arc<dyn LifecycleCommitter>>,
    pub document_reader: Option<Arc<dyn DocumentReader>>,
}

pub fn p0_delete_authority(pool: &PgPool) -> P0DeleteAuthority {
    let committer = Arc::new(PgIngestionCommitter::new(pool.clone()));
    P0DeleteAuthority {
        lifecycle_committer: Some(Arc::clone(&committer) as _),
        document_reader: Some(committer as _),
    }
}
