//! SPEC-091 — prove serving-fence SQL against a live Postgres.
//!
//! Soft-skips without DATABASE_URL unless EDGEQUAKE_REQUIRE_POSTGRES_TESTS=1.

#![cfg(feature = "postgres")]

#[path = "support/postgres_test_config.rs"]
mod postgres_test_config;
#[path = "support/projection_fixture.rs"]
mod projection_fixture;

use std::time::Duration;

use edgequake_storage::projection::ProjectionWorkItem;
use edgequake_storage::serving_fence::{SERVING_STATE_EMBEDDED, SERVING_STATE_READY};
use edgequake_storage::{
    document_batch_deliveries_settled, open_serving_fence_when_deliveries_settled,
    open_settled_serving_fences_bounded, serving_fence_opened_total,
    traits::domain::ChunkRepository, PgProjectionLedger, PostgresChunkRepository,
    ProjectionWorkLedger,
};
use edgequake_storage_contracts::{AckDelivery, ClaimDeliveries, RenewDelivery};
use projection_fixture::{
    commit_single_chunk_batch, drain_document, gap_replay_worker, replay_worker, setup_scope,
};
use uuid::Uuid;

/// Must match `SERVING_FENCE_LOCK_NAMESPACE` in serving_fence_writer.rs.
const FENCE_LOCK_NAMESPACE: i32 = 149_091;

async fn chunk_is_ready(pool: &sqlx::PgPool, chunk_id: Uuid) -> bool {
    sqlx::query_scalar(
        "SELECT EXISTS (SELECT 1 FROM public.chunk_serving_state \
         WHERE chunk_id = $1 AND state = 'ready')",
    )
    .bind(chunk_id)
    .fetch_one(pool)
    .await
    .expect("ready probe")
}

/// Lease this document's deliveries; hand every other claimed delivery back.
async fn claim_document_deliveries(
    ledger: &PgProjectionLedger,
    owner: Uuid,
    document_id: Uuid,
) -> Vec<ProjectionWorkItem> {
    let claimed = ledger
        .claim_work(&ClaimDeliveries {
            owner_token: owner,
            limit: 1_000,
            lease_duration_ms: 60_000,
        })
        .await
        .expect("claim");
    let mut mine = Vec::new();
    for item in claimed {
        if item.event.object_id == document_id {
            mine.push(item);
            continue;
        }
        let renew = RenewDelivery {
            event_id: item.event.event_id,
            binding_id: item.binding_id(),
            owner_token: owner,
            epoch: item.delivery.epoch,
            lease_duration_ms: 60_000,
        };
        ledger
            .release_for_retry(&renew, 0)
            .await
            .expect("release foreign delivery");
    }
    mine
}

fn ack_for(item: &ProjectionWorkItem, owner: Uuid) -> AckDelivery {
    AckDelivery {
        event_id: item.event.event_id,
        binding_id: item.binding_id(),
        owner_token: owner,
        epoch: item.delivery.epoch,
        provider_receipt: "spec091-concurrent-ack".into(),
        completion_proof: item.expected_completion_proof.to_vec(),
    }
}

#[tokio::test]
async fn ack_opens_fence_in_the_same_transaction() {
    let Some((config, pool, tenant_id, workspace_id)) = setup_scope("spec091_fence_worker").await
    else {
        return;
    };
    let (document_id, chunk_id) = commit_single_chunk_batch(&pool, tenant_id, workspace_id).await;

    assert!(
        !document_batch_deliveries_settled(&pool, document_id)
            .await
            .expect("settled check"),
        "fresh commit must leave deliveries open"
    );
    let opened_early = open_serving_fence_when_deliveries_settled(&pool, document_id)
        .await
        .expect("early open");
    assert!(
        !opened_early,
        "opener must refuse while deliveries are open"
    );
    let ready_before: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM public.chunk_serving_state \
         WHERE chunk_id = $1 AND state = 'ready'",
    )
    .bind(chunk_id)
    .fetch_one(&pool)
    .await
    .expect("count ready before");
    assert_eq!(ready_before, 0);

    let opened_before = serving_fence_opened_total();
    // No post-ack opener: the ledger ack alone must open the fence.
    let worker = replay_worker(pool.clone(), config, None).await;
    drain_document(&worker, &pool, document_id).await;

    assert!(
        document_batch_deliveries_settled(&pool, document_id)
            .await
            .expect("settled after drain"),
        "drain must settle document_batch deliveries"
    );
    let ready_after: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM public.chunk_serving_state \
         WHERE chunk_id = $1 AND state = 'ready'",
    )
    .bind(chunk_id)
    .fetch_one(&pool)
    .await
    .expect("count ready after");
    assert_eq!(
        ready_after, 1,
        "the settling ack must open the serving fence"
    );
    assert!(
        serving_fence_opened_total() > opened_before,
        "opened counter must advance"
    );

    let second = open_serving_fence_when_deliveries_settled(&pool, document_id)
        .await
        .expect("second open");
    assert!(
        !second,
        "IS DISTINCT FROM guard: already-ready rows must not count as a change"
    );
}

#[tokio::test]
async fn set_serving_state_counts_only_changed_rows() {
    let Some((_config, pool, _tenant_id, _workspace_id)) =
        setup_scope("spec091_fence_set_state").await
    else {
        return;
    };
    let document_id = Uuid::new_v4();
    let chunk_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO public.documents (id, title, content, status) \
         VALUES ($1, 'set-state', 'c', 'completed') ON CONFLICT (id) DO NOTHING",
    )
    .bind(document_id)
    .execute(&pool)
    .await
    .expect("seed document");
    sqlx::query(
        "INSERT INTO public.chunks (id, document_id, chunk_index, content) \
         VALUES ($1, $2, 0, 'one chunk')",
    )
    .bind(chunk_id)
    .bind(document_id)
    .execute(&pool)
    .await
    .expect("seed chunk");

    let repo = PostgresChunkRepository::new(pool);
    let doc = edgequake_storage::traits::domain::DocumentId::new(document_id);
    let first = repo
        .set_serving_state(doc, SERVING_STATE_READY)
        .await
        .expect("first ready");
    assert_eq!(first, 1, "first ready must touch one row");
    let second = repo
        .set_serving_state(doc, SERVING_STATE_READY)
        .await
        .expect("second ready");
    assert_eq!(second, 0, "noop ready must return 0");
    let third = repo
        .set_serving_state(doc, SERVING_STATE_EMBEDDED)
        .await
        .expect("embedded");
    assert_eq!(third, 1, "state change must touch one row");
}

#[tokio::test]
async fn bounded_reconcile_respects_limit() {
    let Some((config, pool, tenant_id, workspace_id)) = setup_scope("spec091_fence_bounded").await
    else {
        return;
    };
    let (doc_a, _chunk_a) = commit_single_chunk_batch(&pool, tenant_id, workspace_id).await;
    let (doc_b, _chunk_b) = commit_single_chunk_batch(&pool, tenant_id, workspace_id).await;

    // Settle deliveries without opening the fence (the pre-atomic crash gap).
    let worker = gap_replay_worker(pool.clone(), config).await;
    drain_document(&worker, &pool, doc_a).await;
    drain_document(&worker, &pool, doc_b).await;
    assert_eq!(ready_docs(&pool, &[doc_a, doc_b]).await, 0);

    assert_eq!(
        open_settled_serving_fences_bounded(&pool, 0)
            .await
            .expect("limit 0"),
        0
    );

    let mut ready = 0usize;
    for _ in 0..64 {
        let _touched = open_settled_serving_fences_bounded(&pool, 1)
            .await
            .expect("limit 1");
        let now = ready_docs(&pool, &[doc_a, doc_b]).await;
        assert!(
            now.saturating_sub(ready) <= 1,
            "limit=1 must open at most one of our docs per call (was {ready}, now {now})"
        );
        ready = now;
        if ready == 2 {
            break;
        }
    }
    assert_eq!(
        ready, 2,
        "repeated limit=1 opens must eventually ready both docs"
    );
}

/// Forces the interleaving that loses the fence without the per-document lock:
/// both final acks have run their UPDATE before either checks "settled".
#[tokio::test]
async fn concurrent_final_acks_still_open_the_fence() {
    let Some((_config, pool, tenant_id, workspace_id)) =
        setup_scope("spec091_fence_concurrent").await
    else {
        return;
    };
    let (document_id, chunk_id) = commit_single_chunk_batch(&pool, tenant_id, workspace_id).await;

    let ledger = PgProjectionLedger::new(pool.clone());
    let owner = Uuid::new_v4();
    let mine = claim_document_deliveries(&ledger, owner, document_id).await;
    assert_eq!(mine.len(), 2, "one graph and one vector delivery");

    let mut holder = pool.begin().await.expect("begin lock holder");
    sqlx::query("SELECT pg_advisory_xact_lock($1, hashtext($2::text))")
        .bind(FENCE_LOCK_NAMESPACE)
        .bind(document_id)
        .execute(&mut *holder)
        .await
        .expect("hold fence lock");

    let ack_a = ack_for(&mine[0], owner);
    let ack_b = ack_for(&mine[1], owner);
    let (ledger_a, ledger_b) = (ledger.clone(), ledger.clone());
    let task_a = tokio::spawn(async move { ledger_a.acknowledge(&ack_a).await });
    let task_b = tokio::spawn(async move { ledger_b.acknowledge(&ack_b).await });

    let mut waiting = 0i64;
    for _ in 0..200 {
        waiting = sqlx::query_scalar(
            "SELECT count(*) FROM pg_locks \
             WHERE locktype = 'advisory' AND NOT granted AND classid = $1::oid",
        )
        .bind(FENCE_LOCK_NAMESPACE)
        .fetch_one(&pool)
        .await
        .expect("count lock waiters");
        if waiting >= 2 {
            break;
        }
        tokio::time::sleep(Duration::from_millis(25)).await;
    }
    assert_eq!(
        waiting, 2,
        "both acks must block on the fence lock after their UPDATE"
    );
    assert!(!chunk_is_ready(&pool, chunk_id).await);

    holder.commit().await.expect("release fence lock");
    task_a.await.expect("join a").expect("ack a");
    task_b.await.expect("join b").expect("ack b");

    assert!(
        document_batch_deliveries_settled(&pool, document_id)
            .await
            .expect("settled"),
        "both deliveries applied"
    );
    assert!(
        chunk_is_ready(&pool, chunk_id).await,
        "the second ack to take the lock must see the first commit and open the fence"
    );
}

async fn ready_docs(pool: &sqlx::PgPool, docs: &[Uuid]) -> usize {
    let mut n = 0usize;
    for doc in docs {
        let ready: bool = sqlx::query_scalar(
            "SELECT EXISTS (
                SELECT 1 FROM public.chunks c
                JOIN public.chunk_serving_state s ON s.chunk_id = c.id AND s.state = 'ready'
                WHERE c.document_id = $1
            )",
        )
        .bind(doc)
        .fetch_one(pool)
        .await
        .expect("ready probe");
        if ready {
            n += 1;
        }
    }
    n
}
