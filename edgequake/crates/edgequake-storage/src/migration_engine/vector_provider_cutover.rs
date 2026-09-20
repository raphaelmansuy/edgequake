//! SPEC-149 J18 vector binding backfill, switch, and guarded rollback.

use async_trait::async_trait;
use edgequake_storage_contracts::{AccessError, AccessResult};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BindingCompleteness {
    pub visible_revisions: u64,
    pub incomplete_deliveries: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CutoverState {
    Backfilling,
    Switched,
    RolledBack,
}

impl CutoverState {
    #[allow(dead_code)]
    fn as_str(self) -> &'static str {
        match self {
            Self::Backfilling => "backfilling",
            Self::Switched => "switched",
            Self::RolledBack => "rolled_back",
        }
    }
}

#[async_trait]
pub trait VectorCutoverStore: Send + Sync {
    async fn record_cutover(
        &self,
        cutover_id: Uuid,
        old_binding_id: Uuid,
        new_binding_id: Uuid,
    ) -> AccessResult<()>;

    async fn save_cursor(&self, cutover_id: Uuid, cursor: &Value) -> AccessResult<()>;

    async fn load_cursor(&self, cutover_id: Uuid) -> AccessResult<Value>;

    async fn completeness(&self, binding_id: Uuid) -> AccessResult<BindingCompleteness>;

    /// Implementations must re-check completeness atomically with this transition.
    async fn activate(
        &self,
        cutover_id: Uuid,
        source_binding_id: Uuid,
        target_binding_id: Uuid,
        state: CutoverState,
    ) -> AccessResult<()>;
}

pub struct VectorProviderCutover<S> {
    store: S,
}

impl<S: VectorCutoverStore> VectorProviderCutover<S> {
    pub const fn new(store: S) -> Self {
        Self { store }
    }

    pub async fn begin(
        &self,
        cutover_id: Uuid,
        old_binding_id: Uuid,
        new_binding_id: Uuid,
    ) -> AccessResult<()> {
        if old_binding_id == new_binding_id {
            return Err(AccessError::InvalidInput(
                "vector cutover requires distinct old and new bindings".into(),
            ));
        }
        self.store
            .record_cutover(cutover_id, old_binding_id, new_binding_id)
            .await
    }

    pub async fn checkpoint(&self, cutover_id: Uuid, cursor: &Value) -> AccessResult<()> {
        self.store.save_cursor(cutover_id, cursor).await
    }

    pub async fn resume_cursor(&self, cutover_id: Uuid) -> AccessResult<Value> {
        self.store.load_cursor(cutover_id).await
    }

    pub async fn switch(
        &self,
        cutover_id: Uuid,
        old_binding_id: Uuid,
        new_binding_id: Uuid,
    ) -> AccessResult<()> {
        self.ensure_target_caught_up(old_binding_id, new_binding_id)
            .await?;
        self.store
            .activate(
                cutover_id,
                old_binding_id,
                new_binding_id,
                CutoverState::Switched,
            )
            .await
    }

    pub async fn rollback(
        &self,
        cutover_id: Uuid,
        current_binding_id: Uuid,
        rollback_binding_id: Uuid,
    ) -> AccessResult<()> {
        self.ensure_target_caught_up(current_binding_id, rollback_binding_id)
            .await?;
        self.store
            .activate(
                cutover_id,
                current_binding_id,
                rollback_binding_id,
                CutoverState::RolledBack,
            )
            .await
    }

    async fn ensure_target_caught_up(
        &self,
        source_binding_id: Uuid,
        target_binding_id: Uuid,
    ) -> AccessResult<()> {
        let source = self.store.completeness(source_binding_id).await?;
        let target = self.store.completeness(target_binding_id).await?;
        ensure_caught_up(source, target)
    }
}

pub fn ensure_caught_up(
    source: BindingCompleteness,
    target: BindingCompleteness,
) -> AccessResult<()> {
    if target.incomplete_deliveries != 0 || target.visible_revisions < source.visible_revisions {
        return Err(AccessError::Conflict(format!(
            "target binding is lagging: source visible={}, target visible={}, target incomplete={}",
            source.visible_revisions, target.visible_revisions, target.incomplete_deliveries
        )));
    }
    Ok(())
}

#[cfg(feature = "postgres")]
mod postgres {
    use sqlx::{FromRow, PgPool, Postgres, Transaction};

    use super::*;
    use crate::error::StorageError;

    #[derive(Clone)]
    pub struct PgVectorCutoverStore {
        pool: PgPool,
    }

    impl PgVectorCutoverStore {
        pub fn new(pool: PgPool) -> Self {
            Self { pool }
        }
    }

    #[derive(FromRow)]
    struct BindingRow {
        tenant_id: Uuid,
        workspace_id: Uuid,
        role: String,
        generation: i64,
        state: String,
    }

    #[async_trait]
    impl VectorCutoverStore for PgVectorCutoverStore {
        async fn record_cutover(
            &self,
            cutover_id: Uuid,
            old_binding_id: Uuid,
            new_binding_id: Uuid,
        ) -> AccessResult<()> {
            let old = binding(&self.pool, old_binding_id).await?;
            let new = binding(&self.pool, new_binding_id).await?;
            validate_binding_pair(&old, &new)?;
            sqlx::query(
                r#"
                INSERT INTO public.vector_provider_cutovers (
                    cutover_id, tenant_id, workspace_id,
                    old_binding_id, new_binding_id, state
                ) VALUES ($1, $2, $3, $4, $5, 'backfilling')
                ON CONFLICT (old_binding_id, new_binding_id) DO NOTHING
                "#,
            )
            .bind(cutover_id)
            .bind(old.tenant_id)
            .bind(old.workspace_id)
            .bind(old_binding_id)
            .bind(new_binding_id)
            .execute(&self.pool)
            .await
            .map_err(database_error)?;
            Ok(())
        }

        async fn save_cursor(&self, cutover_id: Uuid, cursor: &Value) -> AccessResult<()> {
            let changed = sqlx::query(
                r#"
                UPDATE public.vector_provider_cutovers
                SET backfill_cursor = $2, updated_at = now()
                WHERE cutover_id = $1
                  AND state IN ('backfilling', 'switched')
                "#,
            )
            .bind(cutover_id)
            .bind(cursor)
            .execute(&self.pool)
            .await
            .map_err(database_error)?
            .rows_affected();
            if changed == 0 {
                return Err(AccessError::Conflict(
                    "vector cutover is absent or no longer resumable".into(),
                ));
            }
            Ok(())
        }

        async fn load_cursor(&self, cutover_id: Uuid) -> AccessResult<Value> {
            sqlx::query_scalar(
                r#"
                SELECT backfill_cursor
                FROM public.vector_provider_cutovers
                WHERE cutover_id = $1
                "#,
            )
            .bind(cutover_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(database_error)?
            .ok_or_else(|| AccessError::NotFound(format!("vector cutover {cutover_id}")))
        }

        async fn completeness(&self, binding_id: Uuid) -> AccessResult<BindingCompleteness> {
            completeness(&self.pool, binding_id).await
        }

        async fn activate(
            &self,
            cutover_id: Uuid,
            source_binding_id: Uuid,
            target_binding_id: Uuid,
            state: CutoverState,
        ) -> AccessResult<()> {
            let mut tx = self.pool.begin().await.map_err(database_error)?;
            sqlx::query("SET TRANSACTION ISOLATION LEVEL SERIALIZABLE")
                .execute(&mut *tx)
                .await
                .map_err(database_error)?;
            lock_and_validate_cutover(
                &mut tx,
                cutover_id,
                source_binding_id,
                target_binding_id,
                state,
            )
            .await?;
            let source = completeness_tx(&mut tx, source_binding_id).await?;
            let target = completeness_tx(&mut tx, target_binding_id).await?;
            ensure_caught_up(source, target)?;

            sqlx::query(
                "UPDATE public.data_bindings SET state = 'draining' WHERE binding_id = $1 AND state = 'active'",
            )
            .bind(source_binding_id)
            .execute(&mut *tx)
            .await
            .map_err(database_error)?;
            let activated = sqlx::query(
                "UPDATE public.data_bindings SET state = 'active' WHERE binding_id = $1 AND state = 'draining'",
            )
            .bind(target_binding_id)
            .execute(&mut *tx)
            .await
            .map_err(database_error)?
            .rows_affected();
            if activated != 1 {
                return Err(AccessError::Conflict(
                    "target binding is not in draining state".into(),
                ));
            }
            sqlx::query(
                r#"
                UPDATE public.vector_provider_cutovers
                SET state = $2, updated_at = now()
                WHERE cutover_id = $1
                "#,
            )
            .bind(cutover_id)
            .bind(state.as_str())
            .execute(&mut *tx)
            .await
            .map_err(database_error)?;
            tx.commit().await.map_err(database_error)
        }
    }

    async fn binding(pool: &PgPool, binding_id: Uuid) -> AccessResult<BindingRow> {
        sqlx::query_as(
            r#"
            SELECT tenant_id, workspace_id, role, generation, state
            FROM public.data_bindings
            WHERE binding_id = $1
            "#,
        )
        .bind(binding_id)
        .fetch_optional(pool)
        .await
        .map_err(database_error)?
        .ok_or_else(|| AccessError::NotFound(format!("data binding {binding_id}")))
    }

    fn validate_binding_pair(old: &BindingRow, new: &BindingRow) -> AccessResult<()> {
        if old.tenant_id != new.tenant_id
            || old.workspace_id != new.workspace_id
            || old.role != new.role
            || !matches!(
                old.role.as_str(),
                "vector" | "vector_projection" | "embedding"
            )
        {
            return Err(AccessError::InvalidInput(
                "cutover bindings must share vector role and tenant/workspace scope".into(),
            ));
        }
        if old.state != "active" || new.state != "draining" {
            return Err(AccessError::Conflict(
                "cutover requires active old binding and draining new binding".into(),
            ));
        }
        if new.generation <= old.generation {
            return Err(AccessError::InvalidInput(
                "new vector binding generation must exceed old generation".into(),
            ));
        }
        Ok(())
    }

    async fn completeness(pool: &PgPool, binding_id: Uuid) -> AccessResult<BindingCompleteness> {
        let (visible, incomplete): (i64, i64) = sqlx::query_as(
            r#"
            SELECT
                (SELECT count(*) FROM public.projection_visibility WHERE binding_id = $1),
                (SELECT count(*) FROM public.projection_deliveries
                 WHERE binding_id = $1 AND state <> 'applied')
            "#,
        )
        .bind(binding_id)
        .fetch_one(pool)
        .await
        .map_err(database_error)?;
        checked_completeness(visible, incomplete)
    }

    async fn completeness_tx(
        tx: &mut Transaction<'_, Postgres>,
        binding_id: Uuid,
    ) -> AccessResult<BindingCompleteness> {
        let (visible, incomplete): (i64, i64) = sqlx::query_as(
            r#"
            SELECT
                (SELECT count(*) FROM public.projection_visibility WHERE binding_id = $1),
                (SELECT count(*) FROM public.projection_deliveries
                 WHERE binding_id = $1 AND state <> 'applied')
            "#,
        )
        .bind(binding_id)
        .fetch_one(&mut **tx)
        .await
        .map_err(database_error)?;
        checked_completeness(visible, incomplete)
    }

    fn checked_completeness(visible: i64, incomplete: i64) -> AccessResult<BindingCompleteness> {
        Ok(BindingCompleteness {
            visible_revisions: u64::try_from(visible)
                .map_err(|_| AccessError::CorruptData("negative visibility count".into()))?,
            incomplete_deliveries: u64::try_from(incomplete)
                .map_err(|_| AccessError::CorruptData("negative delivery count".into()))?,
        })
    }

    async fn lock_and_validate_cutover(
        tx: &mut Transaction<'_, Postgres>,
        cutover_id: Uuid,
        source_binding_id: Uuid,
        target_binding_id: Uuid,
        target_state: CutoverState,
    ) -> AccessResult<()> {
        let row: Option<(Uuid, Uuid, String)> = sqlx::query_as(
            r#"
            SELECT old_binding_id, new_binding_id, state
            FROM public.vector_provider_cutovers
            WHERE cutover_id = $1
            FOR UPDATE
            "#,
        )
        .bind(cutover_id)
        .fetch_optional(&mut **tx)
        .await
        .map_err(database_error)?;
        let (old, new, current_state) =
            row.ok_or_else(|| AccessError::NotFound(format!("vector cutover {cutover_id}")))?;
        let valid = match target_state {
            CutoverState::Switched => {
                current_state == "backfilling"
                    && source_binding_id == old
                    && target_binding_id == new
            }
            CutoverState::RolledBack => {
                current_state == "switched" && source_binding_id == new && target_binding_id == old
            }
            CutoverState::Backfilling => false,
        };
        if !valid {
            return Err(AccessError::Conflict(
                "vector cutover state or binding direction does not permit transition".into(),
            ));
        }
        sqlx::query(
            "SELECT binding_id FROM public.data_bindings WHERE binding_id = ANY($1) ORDER BY binding_id FOR UPDATE",
        )
        .bind(vec![source_binding_id, target_binding_id])
        .fetch_all(&mut **tx)
        .await
        .map_err(database_error)?;
        Ok(())
    }

    fn database_error(error: sqlx::Error) -> AccessError {
        AccessError::from(StorageError::from(error))
    }
}

#[cfg(feature = "postgres")]
pub use postgres::PgVectorCutoverStore;

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::sync::Mutex;

    use super::*;

    struct MemoryStore {
        completeness: HashMap<Uuid, BindingCompleteness>,
        transitions: Mutex<Vec<CutoverState>>,
    }

    #[async_trait]
    impl VectorCutoverStore for MemoryStore {
        async fn record_cutover(
            &self,
            _cutover_id: Uuid,
            _old_binding_id: Uuid,
            _new_binding_id: Uuid,
        ) -> AccessResult<()> {
            Ok(())
        }

        async fn save_cursor(&self, _cutover_id: Uuid, _cursor: &Value) -> AccessResult<()> {
            Ok(())
        }

        async fn load_cursor(&self, _cutover_id: Uuid) -> AccessResult<Value> {
            Ok(serde_json::json!({}))
        }

        async fn completeness(&self, binding_id: Uuid) -> AccessResult<BindingCompleteness> {
            self.completeness
                .get(&binding_id)
                .copied()
                .ok_or_else(|| AccessError::NotFound(binding_id.to_string()))
        }

        async fn activate(
            &self,
            _cutover_id: Uuid,
            _source_binding_id: Uuid,
            _target_binding_id: Uuid,
            state: CutoverState,
        ) -> AccessResult<()> {
            self.transitions.lock().unwrap().push(state);
            Ok(())
        }
    }

    #[tokio::test]
    async fn rollback_refuses_when_old_binding_is_not_caught_up() {
        let old = Uuid::from_u128(1);
        let current = Uuid::from_u128(2);
        let store = MemoryStore {
            completeness: HashMap::from([
                (
                    old,
                    BindingCompleteness {
                        visible_revisions: 9,
                        incomplete_deliveries: 1,
                    },
                ),
                (
                    current,
                    BindingCompleteness {
                        visible_revisions: 10,
                        incomplete_deliveries: 0,
                    },
                ),
            ]),
            transitions: Mutex::new(Vec::new()),
        };
        let cutover = VectorProviderCutover::new(store);

        let error = cutover
            .rollback(Uuid::from_u128(3), current, old)
            .await
            .unwrap_err();

        assert!(matches!(error, AccessError::Conflict(_)));
        assert!(cutover.store.transitions.lock().unwrap().is_empty());
    }
}
