//! SPEC-180 W2 — the BM25 statistics triggers, executed rather than string-matched.
//!
//! ```bash
//! DATABASE_URL=postgres://edgequake:edgequake@localhost:5432/edgequake \
//!   cargo test -p edgequake-api --test e2e_spec180_bm25_stats -- --nocapture
//! ```
//!
//! WHY this file exists: migration 225 shipped `eq_bm25_after_chunk_insert`,
//! `eq_bm25_after_chunk_delete` and `rebuild_bm25`, and the only test of them compared
//! `BM25_SQL` against literal strings. The triggers themselves had never been executed, which is
//! how they kept a defect that is invisible from the application side: they counted an *empty*
//! chunk as a document. An image chunk in a normal workspace is exactly that shape
//! (`source_modality = 'image'`, `content = ''`), so four photographs and no text produced
//! `bm25_doc_count = 4, bm25_total_len = 0`, and therefore `avgdl = 0.25` — a divisor that
//! collapses every length-normalised BM25 score in that workspace. Migration 227 fixes the three
//! functions and repairs the counters; these tests are what keeps them fixed.
//!
//! Every test runs inside a transaction that is rolled back, and every row it makes is owned by a
//! fresh workspace, so the developer's database is unchanged.

#![cfg(feature = "postgres")]

mod common;

use sqlx::{PgPool, Row};
use uuid::Uuid;

use common::spec180::block_on_big_stack;

/// A pool, or `None` when Postgres is unreachable (the suite soft-skips, like its siblings).
async fn pool() -> Option<PgPool> {
    let url = common::spec013_postgres::try_database_url()?;
    PgPool::connect(&url).await.ok()
}

/// Open a transaction that always rolls back. `None` when Postgres is unreachable.
async fn tx(pool: &PgPool) -> Option<sqlx::Transaction<'static, sqlx::Postgres>> {
    pool.begin().await.ok()
}

/// A tenant, a normal workspace and one document inside the given transaction.
#[allow(clippy::too_many_arguments)]
async fn seed(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    profile: &str,
    regconfig: &str,
) -> (Uuid, Uuid) {
    let tenant = Uuid::new_v4();
    let workspace = Uuid::new_v4();
    let document = Uuid::new_v4();
    let suffix = Uuid::new_v4().simple().to_string();

    sqlx::query("INSERT INTO public.tenants (tenant_id, name, slug, created_at, updated_at) VALUES ($1,$2,$3,now(),now())")
        .bind(tenant)
        .bind(format!("bm25 {suffix}"))
        .bind(format!("bm25-{suffix}"))
        .execute(&mut **tx)
        .await
        .expect("tenant");
    sqlx::query(
        "INSERT INTO public.workspaces (workspace_id, tenant_id, name, slug, is_active, metadata, settings,
             created_at, updated_at, rag_profile, embedding_modalities, fts_regconfig)
         VALUES ($1,$2,$3,$4,true,'{}','{}',now(),now(),$5,'text',$6)",
    )
    .bind(workspace)
    .bind(tenant)
    .bind(format!("ws {suffix}"))
    .bind(format!("bm25-ws-{suffix}"))
    .bind(profile)
    .bind(regconfig)
    .execute(&mut **tx)
    .await
    .expect("workspace");
    sqlx::query(
        "INSERT INTO public.documents (id, tenant_id, workspace_id, title, content, content_type, created_at, updated_at)
         VALUES ($1,$2,$3,$4,'','application/pdf',now(),now())",
    )
    .bind(document)
    .bind(tenant)
    .bind(workspace)
    .bind(format!("d-{suffix}.pdf"))
    .execute(&mut **tx)
    .await
    .expect("document");

    (workspace, document)
}

/// Insert one chunk. An image chunk is `content = ''` with `source_modality = 'image'`.
async fn chunk(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    workspace: Uuid,
    document: Uuid,
    index: i32,
    content: &str,
    modality: &str,
) -> Uuid {
    let id = Uuid::new_v4();
    let tenant: Uuid =
        sqlx::query_scalar("SELECT tenant_id FROM public.workspaces WHERE workspace_id=$1")
            .bind(workspace)
            .fetch_one(&mut **tx)
            .await
            .expect("tenant id");
    sqlx::query(
        "INSERT INTO public.chunks (id, document_id, tenant_id, workspace_id, content, chunk_index, source_modality)
         VALUES ($1,$2,$3,$4,$5,$6,$7)",
    )
    .bind(id)
    .bind(document)
    .bind(tenant)
    .bind(workspace)
    .bind(content)
    .bind(index)
    .bind(modality)
    .execute(&mut **tx)
    .await
    .expect("chunk");
    id
}

/// `(bm25_doc_count, bm25_total_len)` for one workspace.
async fn stats(tx: &mut sqlx::Transaction<'_, sqlx::Postgres>, workspace: Uuid) -> (i32, i64) {
    let row = sqlx::query(
        "SELECT bm25_doc_count, bm25_total_len FROM public.workspaces WHERE workspace_id=$1",
    )
    .bind(workspace)
    .fetch_one(&mut **tx)
    .await
    .expect("stats");
    (
        row.get::<i32, _>("bm25_doc_count"),
        row.get::<i64, _>("bm25_total_len"),
    )
}

async fn df(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    workspace: Uuid,
    term: &str,
) -> Option<i32> {
    sqlx::query_scalar(
        "SELECT df FROM public.workspace_term_stats WHERE workspace_id=$1 AND term=$2",
    )
    .bind(workspace)
    .bind(term)
    .fetch_optional(&mut **tx)
    .await
    .expect("df")
}

fn run<F>(name: &str, body: impl FnOnce() -> F + Send + 'static)
where
    F: std::future::Future<Output = ()> + Send + 'static,
{
    let _ = name;
    block_on_big_stack(body());
}

#[test]
fn t_180_bm25_empty_chunks_are_not_documents() {
    run("empty chunks", || async {
        let Some(pool) = pool().await else {
            eprintln!("SKIP spec180 bm25 stats: postgres not reachable");
            return;
        };
        let Some(mut tx) = tx(&pool).await else {
            return;
        };
        let (ws, doc) = seed(&mut tx, "normal", "english").await;

        // Four pictures: each is a chunk with no text. None is in the lexicon.
        for i in 0..4 {
            chunk(&mut tx, ws, doc, i, "", "image").await;
        }
        let (count, len) = stats(&mut tx, ws).await;
        assert_eq!(
            (count, len),
            (0, 0),
            "an empty image chunk carries no term, so it must not enter N or the length sum"
        );

        // The caption: the first chunk that has text.
        chunk(&mut tx, ws, doc, 9, "a dog runs on the beach", "text").await;
        let (count, len) = stats(&mut tx, ws).await;
        assert_eq!(count, 1, "one chunk with terms is one document");
        assert!(len > 0, "the caption's terms contribute to the length sum");
        assert_eq!(df(&mut tx, ws, "dog").await, Some(1));

        // Deleting it returns the workspace to exactly the state it started in.
        sqlx::query("DELETE FROM public.chunks WHERE workspace_id=$1 AND content<>''")
            .bind(ws)
            .execute(&mut *tx)
            .await
            .expect("delete caption");
        assert_eq!(stats(&mut tx, ws).await, (0, 0));
        assert_eq!(
            df(&mut tx, ws, "dog").await,
            None,
            "no zero-df row is left behind"
        );
    });
}

#[test]
fn t_180_bm25_delete_subtracts_exactly_what_insert_added() {
    run("insert delete symmetry", || async {
        let Some(pool) = pool().await else { return };
        let Some(mut tx) = tx(&pool).await else {
            return;
        };
        let (ws, doc) = seed(&mut tx, "normal", "english").await;

        chunk(
            &mut tx,
            ws,
            doc,
            0,
            "refunds are accepted within thirty days",
            "text",
        )
        .await;
        chunk(&mut tx, ws, doc, 1, "refunds are accepted", "text").await;
        chunk(&mut tx, ws, doc, 2, "", "image").await;
        let with_text = stats(&mut tx, ws).await;
        assert_eq!(with_text.0, 2, "two text chunks, one image chunk");
        assert_eq!(df(&mut tx, ws, "refund").await, Some(2));

        // Removing the image chunk must not move the counters: it never contributed.
        sqlx::query("DELETE FROM public.chunks WHERE workspace_id=$1 AND content=''")
            .bind(ws)
            .execute(&mut *tx)
            .await
            .expect("delete image chunk");
        assert_eq!(stats(&mut tx, ws).await, with_text);

        // Removing one text chunk must move them by exactly one.
        sqlx::query(
            "DELETE FROM public.chunks WHERE workspace_id=$1 AND content='refunds are accepted'",
        )
        .bind(ws)
        .execute(&mut *tx)
        .await
        .expect("delete text chunk");
        assert_eq!(stats(&mut tx, ws).await.0, 1);
        assert_eq!(df(&mut tx, ws, "refund").await, Some(1));
    });
}

#[test]
fn t_180_bm25_graph_workspaces_keep_no_postings() {
    run("graph profile", || async {
        let Some(pool) = pool().await else { return };
        let Some(mut tx) = tx(&pool).await else {
            return;
        };
        let (ws, doc) = seed(&mut tx, "graph", "english").await;

        chunk(
            &mut tx,
            ws,
            doc,
            0,
            "quarterly revenue grew twelve percent",
            "text",
        )
        .await;
        assert_eq!(
            stats(&mut tx, ws).await,
            (0, 0),
            "a graph workspace keeps no BM25 index"
        );
        let postings: i64 =
            sqlx::query_scalar("SELECT count(*) FROM public.chunk_terms WHERE workspace_id=$1")
                .bind(ws)
                .fetch_one(&mut *tx)
                .await
                .expect("postings");
        assert_eq!(postings, 0);
    });
}

#[test]
fn t_180_bm25_regconfig_is_the_workspace_language() {
    run("regconfig", || async {
        let Some(pool) = pool().await else { return };
        let Some(mut tx) = tx(&pool).await else {
            return;
        };

        // The same French sentence under two configurations must stem differently, which is the
        // whole reason `fts_regconfig` is a create-time column (EC-180-35).
        let (fr_ws, fr_doc) = seed(&mut tx, "normal", "french").await;
        chunk(
            &mut tx,
            fr_ws,
            fr_doc,
            0,
            "les chevaux galopent rapidement",
            "text",
        )
        .await;
        let (en_ws, en_doc) = seed(&mut tx, "normal", "english").await;
        chunk(
            &mut tx,
            en_ws,
            en_doc,
            0,
            "les chevaux galopent rapidement",
            "text",
        )
        .await;

        let fr: Vec<String> = sqlx::query_scalar(
            "SELECT term FROM public.chunk_terms WHERE workspace_id=$1 ORDER BY term",
        )
        .bind(fr_ws)
        .fetch_all(&mut *tx)
        .await
        .expect("fr terms");
        let en: Vec<String> = sqlx::query_scalar(
            "SELECT term FROM public.chunk_terms WHERE workspace_id=$1 ORDER BY term",
        )
        .bind(en_ws)
        .fetch_all(&mut *tx)
        .await
        .expect("en terms");

        assert!(!fr.is_empty() && !en.is_empty());
        assert_ne!(
            fr, en,
            "the French configuration must stem differently from English"
        );
        assert!(
            fr.iter().any(|t| t.starts_with("cheval")),
            "French stemming should reduce 'chevaux' to its stem, got {fr:?}"
        );
    });
}

#[test]
fn t_180_bm25_rebuild_reproduces_the_trigger_state() {
    run("rebuild", || async {
        let Some(pool) = pool().await else { return };
        let Some(mut tx) = tx(&pool).await else {
            return;
        };
        let (ws, doc) = seed(&mut tx, "normal", "english").await;

        chunk(
            &mut tx,
            ws,
            doc,
            0,
            "refunds are accepted within thirty days",
            "text",
        )
        .await;
        chunk(&mut tx, ws, doc, 1, "", "image").await;
        let from_trigger = stats(&mut tx, ws).await;

        // `rebuild_bm25` is the repair entry point; it must land on the same numbers the
        // triggers produced, or a repaired workspace ranks differently from an intact one.
        sqlx::query("SELECT public.rebuild_bm25($1)")
            .bind(ws)
            .execute(&mut *tx)
            .await
            .expect("rebuild");
        assert_eq!(stats(&mut tx, ws).await, from_trigger);
        assert_eq!(df(&mut tx, ws, "refund").await, Some(1));
    });
}

#[test]
fn t_180_trigger_refuses_a_profile_change_below_the_api() {
    // EC-180-58: "Trigger bypass via SQL from a bug → profile flips". The API returns 409 before
    // writing, so the API test cannot prove the second lock exists. This one goes under it.
    //
    // A failed statement aborts its transaction, so each column needs its own: the transaction is
    // also the isolation that keeps the attempt out of the developer's database.
    run("trigger", || async {
        for (column, value, code) in [
            ("rag_profile", "graph", "rag_profile_immutable"),
            (
                "embedding_modalities",
                "text,image",
                "embedding_modalities_immutable",
            ),
            ("fts_regconfig", "french", "fts_regconfig_immutable"),
        ] {
            let Some(pool) = pool().await else {
                eprintln!("SKIP spec180 bm25 stats: postgres not reachable");
                return;
            };
            let Some(mut tx) = tx(&pool).await else {
                return;
            };
            let (ws, _doc) = seed(&mut tx, "normal", "english").await;

            let sql = format!("UPDATE public.workspaces SET {column} = $1 WHERE workspace_id = $2");
            let result = sqlx::query(&sql)
                .bind(value)
                .bind(ws)
                .execute(&mut *tx)
                .await;

            match result {
                Ok(_) => panic!("a raw UPDATE of {column} was accepted; the trigger is missing"),
                Err(error) => {
                    let message = error.to_string();
                    assert!(
                        message.contains(code),
                        "{column} must fail with {code}, got: {message}"
                    );
                }
            }
            // The transaction is aborted; rolling it back discards the attempt and the seed.
            tx.rollback().await.ok();
        }
    });
}
