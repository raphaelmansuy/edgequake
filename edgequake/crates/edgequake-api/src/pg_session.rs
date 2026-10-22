//! Keep the request tenant on tasks spawned from a request or worker.
//!
//! `tokio::spawn` does not inherit task-locals. Chat streaming saves the
//! assistant message on a spawned task, and PDF ingestion spawns checkpoint
//! writes. Without the scope, Lakebase rejects those inserts.

use std::future::Future;

pub fn spawn_with_rls<F>(future: F) -> tokio::task::JoinHandle<F::Output>
where
    F: Future + Send + 'static,
    F::Output: Send + 'static,
{
    #[cfg(feature = "postgres")]
    {
        edgequake_storage::adapters::postgres::rls::spawn_with_rls(future)
    }
    #[cfg(not(feature = "postgres"))]
    {
        tokio::spawn(future)
    }
}
