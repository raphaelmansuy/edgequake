//! SPEC-160 W3: one contract, two stores (memory and PostgreSQL).
//!
//! The Postgres run needs `DATABASE_URL`. It applies migration 166 (idempotent,
//! additive) and uses random workspace ids, then removes them. Without a
//! database it skips, unless `EDGEQUAKE_REQUIRE_POSTGRES_TESTS=1`.

#[cfg(feature = "postgres")]
use edgequake_storage::adapters::postgres::decision_store::PostgresDecisionStore;
use edgequake_storage::decision::{
    DecisionScope, DecisionStore, MemoryDecisionStore, ReviewKind, ReviewRow,
};
use serde_json::json;
use uuid::Uuid;

#[cfg(feature = "postgres")]
const MIGRATION: &str = include_str!("../../../migrations/166_spec160_decision.sql");

fn scope() -> DecisionScope {
    DecisionScope::new(
        Some(&Uuid::new_v4().to_string()),
        &Uuid::new_v4().to_string(),
    )
}

fn row(chunk: &str, subject: &str, score: f32) -> ReviewRow {
    ReviewRow {
        chunk_id: chunk.into(),
        kind: ReviewKind::Entity,
        subject: subject.into(),
        label: "PERSON".into(),
        object: None,
        score,
        reason: None,
        sentence: format!("{subject} is here."),
        model: "tev1:0.8b".into(),
        contract: "edgextract.decision.2026-10-06".into(),
    }
}

fn relation_row(chunk: &str) -> ReviewRow {
    ReviewRow {
        kind: ReviewKind::Relation,
        object: Some("ACME".into()),
        reason: Some("endpoint_not_accepted".into()),
        label: "WORKS_AT".into(),
        ..row(chunk, "ADA", 0.55)
    }
}

/// T-160-I17..I21 — the behavior every store must have.
async fn run_contract(store: &dyn DecisionStore) {
    let (a, b) = (scope(), scope());
    let doc = Uuid::new_v4().to_string();

    // Answers: miss, put, hit, replace; isolation between workspaces (EC-160-38).
    assert_eq!(store.get_answer(&a, "k1").await.unwrap(), None);
    store
        .put_answer(&a, "k1", "m", "c", &json!({"v": 1}))
        .await
        .unwrap();
    assert_eq!(
        store.get_answer(&a, "k1").await.unwrap(),
        Some(json!({"v": 1}))
    );
    store
        .put_answer(&a, "k1", "m", "c", &json!({"v": 2}))
        .await
        .unwrap();
    assert_eq!(
        store.get_answer(&a, "k1").await.unwrap(),
        Some(json!({"v": 2}))
    );
    assert_eq!(
        store.get_answer(&b, "k1").await.unwrap(),
        None,
        "other workspace"
    );
    assert_eq!(store.cache_len(&a).await.unwrap(), 1);

    // Review: replace is idempotent, order is best score first (EC-160-43).
    let rows = vec![
        row("c1", "LOW", 0.51),
        row("c1", "HIGH", 0.79),
        relation_row("c2"),
    ];
    store.replace_review(&a, &doc, &rows).await.unwrap();
    store.replace_review(&a, &doc, &rows).await.unwrap();
    let listed = store.list_review(&a, &doc).await.unwrap();
    assert_eq!(listed.len(), 3, "no stacking after a second replace");
    assert_eq!(listed[0].subject, "HIGH");
    let rel = listed
        .iter()
        .find(|r| r.kind == ReviewKind::Relation)
        .unwrap();
    assert_eq!(rel.object.as_deref(), Some("ACME"));
    assert_eq!(rel.reason.as_deref(), Some("endpoint_not_accepted"));
    assert!(
        store.list_review(&b, &doc).await.unwrap().is_empty(),
        "other workspace"
    );

    // Replace with fewer rows shrinks; empty replace clears.
    store.replace_review(&a, &doc, &rows[..1]).await.unwrap();
    assert_eq!(store.list_review(&a, &doc).await.unwrap().len(), 1);
    store.replace_review(&a, &doc, &[]).await.unwrap();
    assert!(store.list_review(&a, &doc).await.unwrap().is_empty());

    // Document delete touches review only.
    store.replace_review(&a, &doc, &rows).await.unwrap();
    assert_eq!(store.delete_document(&a, &doc).await.unwrap(), 3);
    assert_eq!(
        store.cache_len(&a).await.unwrap(),
        1,
        "cache survives a document delete"
    );
    assert_eq!(store.delete_document(&a, &doc).await.unwrap(), 0);

    // Cap sweep keeps the newest rows (EC-160-40).
    for i in 0..5 {
        store
            .put_answer(&a, &format!("s{i}"), "m", "c", &json!(i))
            .await
            .unwrap();
        tokio::time::sleep(std::time::Duration::from_millis(5)).await;
    }
    assert_eq!(store.cache_len(&a).await.unwrap(), 6);
    assert_eq!(store.sweep_cache(&a, 30, 3).await.unwrap(), 3);
    assert_eq!(store.cache_len(&a).await.unwrap(), 3);
    assert!(
        store.get_answer(&a, "s4").await.unwrap().is_some(),
        "newest kept"
    );
    assert!(
        store.get_answer(&a, "s0").await.unwrap().is_none(),
        "oldest gone"
    );

    // Workspace delete removes cache and review together.
    store.replace_review(&a, &doc, &rows).await.unwrap();
    assert!(store.delete_workspace(&a).await.unwrap() >= 4);
    assert_eq!(store.cache_len(&a).await.unwrap(), 0);
    assert!(store.list_review(&a, &doc).await.unwrap().is_empty());
}

#[tokio::test]
async fn memory_store_meets_contract() {
    run_contract(&MemoryDecisionStore::new()).await;
}

#[cfg(feature = "postgres")]
fn require_postgres() -> bool {
    std::env::var("EDGEQUAKE_REQUIRE_POSTGRES_TESTS")
        .ok()
        .is_some_and(|v| v == "1" || v.eq_ignore_ascii_case("true"))
}

#[cfg(feature = "postgres")]
static MIGRATED: tokio::sync::OnceCell<()> = tokio::sync::OnceCell::const_new();

/// Apply migration 166 once per process. Parallel `CREATE TABLE IF NOT EXISTS`
/// can race inside Postgres; the server avoids this with an advisory lock.
#[cfg(feature = "postgres")]
async fn migrate_once(pool: &sqlx::PgPool) {
    MIGRATED
        .get_or_init(|| async {
            sqlx::raw_sql(MIGRATION)
                .execute(pool)
                .await
                .expect("migration 166 applies");
        })
        .await;
}

#[cfg(feature = "postgres")]
async fn pg_pool() -> Option<sqlx::PgPool> {
    let url = std::env::var("DATABASE_URL").ok().filter(|u| !u.is_empty());
    let Some(url) = url else {
        assert!(
            !require_postgres(),
            "EDGEQUAKE_REQUIRE_POSTGRES_TESTS=1 but DATABASE_URL is unset"
        );
        eprintln!("SKIP: DATABASE_URL is unset");
        return None;
    };
    match sqlx::PgPool::connect(&url).await {
        Ok(pool) => Some(pool),
        Err(e) => {
            assert!(!require_postgres(), "cannot reach PostgreSQL: {e}");
            eprintln!("SKIP: PostgreSQL unreachable: {e}");
            None
        }
    }
}

#[tokio::test]
#[cfg(feature = "postgres")]
async fn postgres_store_meets_contract() {
    let Some(pool) = pg_pool().await else { return };
    migrate_once(&pool).await;
    run_contract(&PostgresDecisionStore::new(pool)).await;
}

// T-160-I21 — migration 166 can run twice (expand-only, idempotent).
#[tokio::test]
#[cfg(feature = "postgres")]
async fn migration_is_idempotent() {
    let Some(pool) = pg_pool().await else { return };
    migrate_once(&pool).await;
    sqlx::raw_sql(MIGRATION)
        .execute(&pool)
        .await
        .expect("a later run is a no-op");
}

// T-160-I20 — an id that is not a UUID cannot leak or crash: it reads empty, writes nothing.
#[tokio::test]
#[cfg(feature = "postgres")]
async fn postgres_non_uuid_ids_are_inert() {
    let Some(pool) = pg_pool().await else { return };
    migrate_once(&pool).await;
    let store = PostgresDecisionStore::new(pool);
    let bad = DecisionScope::new(None, "default");
    store
        .put_answer(&bad, "k", "m", "c", &json!(1))
        .await
        .unwrap();
    assert_eq!(store.get_answer(&bad, "k").await.unwrap(), None);
    assert_eq!(store.cache_len(&bad).await.unwrap(), 0);
    store
        .replace_review(&bad, "doc", &[row("c", "X", 0.5)])
        .await
        .unwrap();
    assert!(store.list_review(&bad, "doc").await.unwrap().is_empty());
}
