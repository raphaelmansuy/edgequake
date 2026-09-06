//! SPEC-146 G-146-52 — unauthorized context rate TARGET 0.
//!
//! Unit-style harness always runs. Live HTTP+PG asserts SECRET_TOKEN absent
//! from query response body when peer queries ENTITY_X.
//!
//! Run:
//!   cargo test -p edgequake-api --test e2e_spec146_unauthorized_context_rate --features postgres

mod common;

use edgequake_authz::AllowSet;
use edgequake_query::context::{QueryContext, RetrievedChunk};
use edgequake_query::context_filter::filter_context_by_document_ids;
use uuid::Uuid;

const SECRET_TOKEN: &str = "SECRET_TOKEN_SPEC146_UNAUTHORIZED";

#[test]
fn g146_52_allow_set_intersect_never_widens() {
    let allowed = Uuid::new_v4();
    let other = Uuid::new_v4();
    let allow = AllowSet::from_ids([allowed]);

    let intersected = allow.intersect_filter(Some(&[allowed, other]));
    assert!(intersected.contains(&allowed));
    assert!(!intersected.contains(&other));
    assert_eq!(intersected.len(), 1);

    let same = allow.intersect_filter(None);
    assert_eq!(same.len(), 1);
    assert!(same.contains(&allowed));
}

#[test]
fn g146_52_unauthorized_secret_token_absent_from_context() {
    let public_doc = Uuid::new_v4().to_string();
    let secret_doc = Uuid::new_v4().to_string();

    let mut ctx = QueryContext::default();
    ctx.chunks.push({
        let mut c = RetrievedChunk::new("chunk-public", "public content ok", 0.9);
        c.document_id = Some(public_doc.clone());
        c
    });
    ctx.chunks.push({
        let mut c = RetrievedChunk::new(
            "chunk-secret",
            format!("leaked payload {SECRET_TOKEN}"),
            0.95,
        );
        c.document_id = Some(secret_doc);
        c
    });

    let allow = AllowSet::from_ids([Uuid::parse_str(&public_doc).unwrap()]);
    let allowed_ids = allow.as_string_vec();

    filter_context_by_document_ids(&mut ctx, Some(&allowed_ids));

    let blob = serde_json::to_string(&ctx).unwrap_or_else(|_| {
        ctx.chunks
            .iter()
            .map(|c| c.content.clone())
            .collect::<Vec<_>>()
            .join("\n")
    });

    assert!(
        !blob.contains(SECRET_TOKEN),
        "TARGET 0 violated: SECRET_TOKEN present in filtered context: {blob}"
    );
    assert_eq!(ctx.chunks.len(), 1);
    assert_eq!(
        ctx.chunks[0].document_id.as_deref(),
        Some(public_doc.as_str())
    );
}

#[test]
fn g146_52_empty_allow_set_clears_all_chunks() {
    let mut ctx = QueryContext::default();
    ctx.chunks.push({
        let mut c = RetrievedChunk::new("c1", SECRET_TOKEN, 1.0);
        c.document_id = Some(Uuid::new_v4().to_string());
        c
    });
    filter_context_by_document_ids(&mut ctx, Some(&[]));
    assert!(ctx.chunks.is_empty());
}

#[cfg(feature = "postgres")]
mod live {
    use super::*;
    use axum::http::StatusCode;
    use common::spec146_pg::{
        post_query, request_json, try_create_harness, DOCB_TITLE, SECRET_TOKEN as HARNESS_SECRET,
    };
    use edgequake_authz::ZERO_AUTHZ_ANSWER;
    use serde_json::json;
    use serial_test::serial;

    /// HTTP query must not contain SECRET_TOKEN or DocB title (G-146-52).
    #[tokio::test]
    #[serial]
    async fn g146_52_http_query_omits_secret_token() {
        let Some(h) = try_create_harness().await else {
            eprintln!("SKIP g146_52 live: no DATABASE_URL");
            return;
        };

        let headers = h.auth_headers_peer();
        let (status, body) = post_query(h.router(), &headers, "What is ENTITY_X?").await;
        assert!(
            status.is_success() || status == StatusCode::OK,
            "query status {status}: {body}"
        );
        let blob = body.to_string();
        assert!(
            !blob.contains(HARNESS_SECRET) && !blob.contains(SECRET_TOKEN),
            "TARGET 0 violated — SECRET_TOKEN in HTTP body: {blob}"
        );
        assert!(
            !blob.contains(DOCB_TITLE),
            "DocB title leaked in query response: {blob}"
        );
    }

    /// Empty allow-set (owner_only docs only, peer has none) → ZERO_AUTHZ_ANSWER.
    #[tokio::test]
    #[serial]
    async fn g146_52_empty_allow_set_returns_ssot_answer() {
        let Some(h) = try_create_harness().await else {
            eprintln!("SKIP g146_52 empty: no DATABASE_URL");
            return;
        };

        // Quarantine DocA so peer allow-set is empty (only DocB owner_only remains, denied).
        sqlx::query("UPDATE documents SET security_status = 'quarantined' WHERE id = $1")
            .bind(h.doc_a)
            .execute(&h.pool)
            .await
            .expect("quarantine DocA");

        let headers = h.auth_headers_peer();
        let (status, body) = post_query(h.router(), &headers, "ENTITY_X?").await;
        assert_eq!(status, StatusCode::OK, "{body}");
        let answer = body.get("answer").and_then(|v| v.as_str()).unwrap_or("");
        assert_eq!(
            answer, ZERO_AUTHZ_ANSWER,
            "expected SSOT empty-authz answer, got {answer:?} body={body}"
        );
        assert!(!body.to_string().contains(HARNESS_SECRET));
    }

    /// G-146-7: chat empty allow-set → same SSOT copy, no Restricted / Secret.
    #[tokio::test]
    #[serial]
    async fn g146_7_chat_empty_allow_returns_ssot_answer() {
        let Some(h) = try_create_harness().await else {
            eprintln!("SKIP g146_7 chat: no DATABASE_URL");
            return;
        };

        sqlx::query("UPDATE documents SET security_status = 'quarantined' WHERE id = $1")
            .bind(h.doc_a)
            .execute(&h.pool)
            .await
            .expect("quarantine DocA");

        let (status, body) = request_json(
            h.router(),
            "POST",
            "/api/v1/chat/completions",
            &h.auth_headers_peer(),
            Some(json!({
                "message": "ENTITY_X?",
                "stream": false
            })),
        )
        .await;
        assert_eq!(status, StatusCode::OK, "{body}");
        let content = body
            .get("content")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        assert_eq!(
            content, ZERO_AUTHZ_ANSWER,
            "chat empty-allow must match query SSOT, got {content:?} body={body}"
        );
        let blob = body.to_string();
        assert!(!blob.contains(HARNESS_SECRET) && !blob.contains(SECRET_TOKEN));
        assert!(!blob.contains("Restricted"));
    }
}
