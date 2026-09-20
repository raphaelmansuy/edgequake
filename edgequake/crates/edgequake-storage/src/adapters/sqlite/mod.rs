//! SQLite relational-authority adapters for P3.

mod ingestion_committer;
mod pool;
mod projection_ledger;

pub use ingestion_committer::SqliteIngestionCommitter;
pub use pool::{connect_sqlite, validate_sqlite_deployment};
pub use projection_ledger::SqliteProjectionLedger;

#[cfg(test)]
mod tests {
    use edgequake_storage_contracts::{
        AccessError, AccessScope, AckDelivery, ClaimDeliveries, DeleteDocument, DocumentId,
        DocumentReader, IngestionCommitter, LifecycleCommitter, PreparedIngestionBatch,
        PreparedRecord, ProjectionLedger, TenantId, WorkspaceId,
    };
    use uuid::Uuid;

    use super::*;

    fn command(tenant: Uuid, workspace: Uuid, document: Uuid) -> PreparedIngestionBatch {
        PreparedIngestionBatch {
            scope: AccessScope::new(TenantId::new(tenant), WorkspaceId::new(workspace)),
            document_id: DocumentId::new(document),
            ingest_generation: 1,
            batch_ordinal: 0,
            expected_revision: Some(0),
            idempotency_key: "sqlite-commit-1".into(),
            schema_version: 1,
            canonical_digest: [7; 32],
            chunks: vec![PreparedRecord {
                id: Uuid::new_v4(),
                revision: 1,
                digest: [8; 32],
                payload: b"chunk".to_vec(),
            }],
            facts: Vec::new(),
            contributions: Vec::new(),
            embeddings: Vec::new(),
        }
    }

    #[tokio::test]
    async fn commit_is_atomic_and_idempotent() {
        let temp = tempfile::tempdir().unwrap();
        let pool = connect_sqlite(temp.path().join("authority.db").to_string_lossy())
            .await
            .unwrap();
        let tenant = Uuid::new_v4();
        let workspace = Uuid::new_v4();
        let document = Uuid::new_v4();
        let committer = SqliteIngestionCommitter::new(pool.clone());
        let command = command(tenant, workspace, document);

        let first = committer.commit_batch(&command).await.unwrap();
        let replay = committer.commit_batch(&command).await.unwrap();

        assert_eq!(first, replay);
        // chunk revision + document generation row
        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM object_revisions")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(count, 2);
        let views = committer
            .get_many(
                &command.scope,
                &[command.document_id],
            )
            .await
            .unwrap();
        assert_eq!(views[0].as_ref().unwrap().revision, 1);
        assert!(!views[0].as_ref().unwrap().deleted);
    }

    #[tokio::test]
    async fn duplicate_logical_revision_is_rejected_before_mutation() {
        let temp = tempfile::tempdir().unwrap();
        let pool = connect_sqlite(temp.path().join("dup.db").to_string_lossy())
            .await
            .unwrap();
        let tenant = Uuid::new_v4();
        let workspace = Uuid::new_v4();
        let document = Uuid::new_v4();
        let fact_id = Uuid::new_v4();
        let mut command = command(tenant, workspace, document);
        command.chunks.clear();
        command.facts = vec![
            PreparedRecord {
                id: fact_id,
                revision: 1,
                digest: [1; 32],
                payload: b"a".to_vec(),
            },
            PreparedRecord {
                id: fact_id,
                revision: 1,
                digest: [1; 32],
                payload: b"a".to_vec(),
            },
        ];
        let err = SqliteIngestionCommitter::new(pool.clone())
            .commit_batch(&command)
            .await
            .unwrap_err();
        assert!(matches!(err, AccessError::InvalidInput(_)));
        assert!(err.to_string().contains("byte-identical duplicate"));
        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM object_revisions")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(count, 0);
    }

    #[tokio::test]
    async fn generation_two_reprocess_and_contributions() {
        let temp = tempfile::tempdir().unwrap();
        let pool = connect_sqlite(temp.path().join("gen2.db").to_string_lossy())
            .await
            .unwrap();
        let tenant = Uuid::new_v4();
        let workspace = Uuid::new_v4();
        let document = Uuid::new_v4();
        let committer = SqliteIngestionCommitter::new(pool.clone());
        let mut first = command(tenant, workspace, document);
        let contrib_id = Uuid::new_v4();
        let fact_id = Uuid::new_v4();
        first.chunks.clear();
        first.facts = vec![PreparedRecord {
            id: fact_id,
            revision: 1,
            digest: [2; 32],
            payload: serde_json::to_vec(&serde_json::json!({"kind":"node"})).unwrap(),
        }];
        first.contributions = vec![PreparedRecord {
            id: contrib_id,
            revision: 1,
            digest: [3; 32],
            payload: serde_json::to_vec(&serde_json::json!({
                "fact_id": fact_id,
                "fact_revision": 1
            }))
            .unwrap(),
        }];
        committer.commit_batch(&first).await.unwrap();

        let contrib_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM graph_contributions")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(contrib_count, 1);

        let mut second = first.clone();
        second.ingest_generation = 2;
        second.expected_revision = Some(1);
        second.idempotency_key = "sqlite-commit-2".into();
        second.canonical_digest = [9; 32];
        second.facts[0].revision = 2;
        second.facts[0].digest = [4; 32];
        let contrib2 = Uuid::new_v4();
        second.contributions[0].id = contrib2;
        second.contributions[0].revision = 2;
        second.contributions[0].digest = [5; 32];
        second.contributions[0].payload = serde_json::to_vec(&serde_json::json!({
            "fact_id": fact_id,
            "fact_revision": 2
        }))
        .unwrap();
        committer.commit_batch(&second).await.unwrap();

        let views = committer
            .get_many(&second.scope, &[second.document_id])
            .await
            .unwrap();
        assert_eq!(views[0].as_ref().unwrap().revision, 2);
        let contrib_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM graph_contributions")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(contrib_count, 2);
    }

    #[tokio::test]
    async fn tombstone_advances_document_revision() {
        let temp = tempfile::tempdir().unwrap();
        let pool = connect_sqlite(temp.path().join("tomb.db").to_string_lossy())
            .await
            .unwrap();
        let tenant = Uuid::new_v4();
        let workspace = Uuid::new_v4();
        let document = Uuid::new_v4();
        let committer = SqliteIngestionCommitter::new(pool.clone());
        committer
            .commit_batch(&command(tenant, workspace, document))
            .await
            .unwrap();

        let scope = AccessScope::new(TenantId::new(tenant), WorkspaceId::new(workspace));
        let delete = DeleteDocument {
            scope,
            document_id: DocumentId::new(document),
            expected_revision: 1,
            idempotency_key: format!("delete:{document}:1"),
            command_digest: [11; 32],
        };
        let receipt = committer.tombstone_document(&delete).await.unwrap();
        assert_eq!(receipt.tombstone_revision, 2);

        let views = committer
            .get_many(&scope, &[DocumentId::new(document)])
            .await
            .unwrap();
        assert!(views[0].as_ref().unwrap().deleted);
        assert_eq!(views[0].as_ref().unwrap().revision, 2);
    }

    #[tokio::test]
    async fn claim_ack_uses_epoch_fencing() {
        let temp = tempfile::tempdir().unwrap();
        let pool = connect_sqlite(temp.path().join("ledger.db").to_string_lossy())
            .await
            .unwrap();
        let tenant = Uuid::new_v4();
        let workspace = Uuid::new_v4();
        let binding = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO data_bindings \
             (binding_id,tenant_id,workspace_id,role,generation,state) \
             VALUES (?,?,?,?,1,'active')",
        )
        .bind(binding.to_string())
        .bind(tenant.to_string())
        .bind(workspace.to_string())
        .bind("vector")
        .execute(&pool)
        .await
        .unwrap();
        let command = command(tenant, workspace, Uuid::new_v4());
        SqliteIngestionCommitter::new(pool.clone())
            .commit_batch(&command)
            .await
            .unwrap();

        let owner = Uuid::new_v4();
        let claimed = SqliteProjectionLedger::new(pool.clone())
            .claim(&ClaimDeliveries {
                owner_token: owner,
                limit: 1,
                lease_duration_ms: 60_000,
            })
            .await
            .unwrap();
        assert_eq!(claimed.len(), 1);
        let delivery = &claimed[0];
        let stale = AckDelivery {
            event_id: delivery.event_id,
            binding_id: delivery.binding_id,
            owner_token: owner,
            epoch: delivery.epoch.saturating_sub(1),
            provider_receipt: "stale".into(),
            completion_proof: vec![1],
        };
        assert!(SqliteProjectionLedger::new(pool.clone())
            .ack(&stale)
            .await
            .is_err());

        let current = AckDelivery {
            epoch: delivery.epoch,
            provider_receipt: "ok".into(),
            completion_proof: vec![2],
            ..stale
        };
        let acked = SqliteProjectionLedger::new(pool)
            .ack(&current)
            .await
            .unwrap();
        assert_eq!(
            acked.state,
            edgequake_storage_contracts::DeliveryState::Applied
        );
    }
}
