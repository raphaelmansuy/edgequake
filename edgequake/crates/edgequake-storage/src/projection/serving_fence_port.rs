//! Port: open SPEC-091 serving fence after projection deliveries settle.
//!
//! ProjectionWorker depends on this trait — never on `sqlx::PgPool` — so fence
//! behaviour is unit-testable with a recording fake (DIP).

use async_trait::async_trait;
use edgequake_storage_contracts::AccessResult;
use uuid::Uuid;

/// Open chunk serving state to `ready` once `document_batch` deliveries settle.
#[async_trait]
pub trait ServingFenceOpener: Send + Sync {
    /// `Ok(true)` only when at least one chunk became servable.
    async fn open_when_settled(&self, document_id: Uuid) -> AccessResult<bool>;
}
