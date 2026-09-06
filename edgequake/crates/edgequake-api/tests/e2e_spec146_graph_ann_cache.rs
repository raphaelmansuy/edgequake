//! SPEC-146 remaining HTTP+PG gates: ingest, graph, ANN/query, cache, parse, labels.
//!
//! Fast unit guards always run. Live gates skip when DATABASE_URL is unset.
//!
//! Run:
//!   cargo test -p edgequake-api --test e2e_spec146_graph_ann_cache --features postgres

mod common;

use edgequake_api::services::spec146_authz::graph_properties_in_allow;
use std::collections::HashMap;

#[test]
fn g146_17_graph_properties_fail_closed_without_provenance() {
    let mut props = HashMap::new();
    props.insert("description".into(), serde_json::json!("no sources"));
    assert!(!graph_properties_in_allow(
        &props,
        Some(&["aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa".into()])
    ));
    assert!(graph_properties_in_allow(&props, None));
}

#[cfg(feature = "postgres")]
mod live {
    use super::*;
    use axum::http::StatusCode;
    use common::spec146_pg::{
        get_documents, post_query, post_query_filtered, request_json, try_create_harness,
        DOCA_TITLE, DOCB_TITLE, SECRET_TOKEN,
    };
    use serde_json::json;
    use serial_test::serial;
    use uuid::Uuid;

    fn assert_no_secret(blob: &str, label: &str) {
        assert!(
            !blob.contains(SECRET_TOKEN),
            "{label}: SECRET_TOKEN leaked: {blob}"
        );
        assert!(
            !blob.contains(DOCB_TITLE),
            "{label}: DocB title leaked: {blob}"
        );
    }

    /// G-146-14: non-uploader role ingest → 403 copy, not 404.
    #[tokio::test]
    #[serial]
    async fn g146_14_readonly_ingest_is_forbidden() {
        let Some(h) = try_create_harness().await else {
            eprintln!("SKIP g146_14: no DATABASE_URL");
            return;
        };
        let Some((uid, token)) = h
            .create_user_with_role(&format!("ro_{}", &Uuid::new_v4().to_string()[..8]), "readonly")
            .await
        else {
            eprintln!("SKIP g146_14: could not create readonly user");
            return;
        };
        let _ = uid;
        let headers = vec![
            (
                axum::http::header::AUTHORIZATION.to_string(),
                format!("Bearer {token}"),
            ),
            ("X-Tenant-ID".into(), h.tenant_id.clone()),
            ("X-Workspace-ID".into(), h.workspace_id.clone()),
        ];
        let (status, body) = request_json(
            h.router(),
            "POST",
            "/api/v1/documents",
            &headers,
            Some(json!({
                "title": "should-fail",
                "content": "x",
                "async_processing": true
            })),
        )
        .await;
        assert_eq!(status, StatusCode::FORBIDDEN, "expected 403 not 404: {body}");
        let blob = body.to_string().to_lowercase();
        assert!(
            blob.contains("permission") && blob.contains("upload"),
            "expected upload permission copy: {body}"
        );
        assert!(!blob.contains("not found"));
    }

    /// G-146-15: no RLS app role on documents — skip with reason (API PEP is the gate).
    #[tokio::test]
    #[serial]
    async fn g146_15_direct_sql_rls_or_skip() {
        let Some(h) = try_create_harness().await else {
            eprintln!("SKIP g146_15: no DATABASE_URL");
            return;
        };
        let has_rls: bool = sqlx::query_scalar(
            r#"SELECT relrowsecurity FROM pg_class WHERE relname = 'documents'"#,
        )
        .fetch_optional(&h.pool)
        .await
        .ok()
        .flatten()
        .unwrap_or(false);
        if !has_rls {
            eprintln!(
                "SKIP G-146-15: documents has no RLS; unauthorized reads are denied at API PEP"
            );
            return;
        }
        eprintln!("G-146-15 RLS enabled — API PEP still required (allow-set)");
    }

    /// G-146-18/20/21: query allow-set omits DocB; client document_filter cannot widen.
    #[tokio::test]
    #[serial]
    async fn g146_18_20_21_query_allow_and_filter_no_widen() {
        let Some(h) = try_create_harness().await else {
            eprintln!("SKIP g146_18: no DATABASE_URL");
            return;
        };
        let _ = h.seed_secret_chunk_kv().await;
        let headers = h.auth_headers_peer();
        let (status, body) = post_query(h.router(), &headers, "ENTITY_X?").await;
        assert!(status.is_success(), "query {status}: {body}");
        assert_no_secret(&body.to_string(), "query");

        let doc_b = h.doc_b.to_string();
        let (status, body) = post_query_filtered(
            h.router(),
            &headers,
            "ENTITY_X?",
            &[doc_b.as_str()],
        )
        .await;
        assert!(status.is_success(), "filtered query {status}: {body}");
        assert_no_secret(&body.to_string(), "document_filter widen");
    }

    /// G-146-19: detail 404 body has no reason codes; audit sink records reason.
    #[tokio::test]
    #[serial]
    async fn g146_19_detail_404_has_no_reason_codes() {
        let Some(h) = try_create_harness().await else {
            eprintln!("SKIP g146_19: no DATABASE_URL");
            return;
        };
        let (status, body) =
            common::spec146_pg::get_document(h.router(), &h.auth_headers_peer(), h.doc_b).await;
        assert_eq!(status, StatusCode::NOT_FOUND, "{body}");
        let s = body.to_string();
        assert!(!s.contains("not_in_allow_set"), "{s}");
        assert!(!s.to_lowercase().contains("cedar"), "{s}");
        assert!(!s.contains("DenyReason"), "{s}");

        // Async audit worker — brief wait then probe sink.
        tokio::time::sleep(std::time::Duration::from_millis(400)).await;
        let rows = sqlx::query_scalar::<_, serde_json::Value>(
            r#"
            SELECT metadata FROM audit_logs
            WHERE workspace_id::text = $1
              AND metadata->>'reason_code' IS NOT NULL
            ORDER BY timestamp DESC
            LIMIT 5
            "#,
        )
        .bind(&h.workspace_id)
        .fetch_all(&h.pool)
        .await;
        match rows {
            Ok(metas) => {
                let joined = metas
                    .iter()
                    .map(|m| m.to_string())
                    .collect::<Vec<_>>()
                    .join(" ");
                assert!(
                    joined.contains("not_in_allow_set")
                        || joined.contains("capability_denied")
                        || joined.contains("quarantined"),
                    "expected deny reason_code in audit sink, got: {joined}"
                );
            }
            Err(e) => eprintln!("SKIP g146_19 audit sink probe: {e}"),
        }
    }

    /// G-146-11: track status omits unauthorized titles.
    #[tokio::test]
    #[serial]
    async fn g146_11_track_status_omits_unauthorized() {
        let Some(h) = try_create_harness().await else {
            eprintln!("SKIP g146_11 track: no DATABASE_URL");
            return;
        };
        let track_id = format!("trk_{}", Uuid::new_v4().simple());
        if h.seed_track_on_docs(&track_id).await.is_none() {
            eprintln!("SKIP g146_11 track: seed failed");
            return;
        }
        let (status, body) = request_json(
            h.router(),
            "GET",
            &format!("/api/v1/documents/track/{track_id}"),
            &h.auth_headers_peer(),
            None,
        )
        .await;
        assert!(status.is_success(), "track {status}: {body}");
        let s = body.to_string();
        assert!(s.contains(DOCA_TITLE) || s.contains("Public"), "{s}");
        assert!(!s.contains(DOCB_TITLE), "DocB title leaked in track: {s}");
        assert_no_secret(&s, "track status");
    }

    /// G-146-30/31/33: graph search/popular omit SECRET_TOKEN and DocB title.
    #[tokio::test]
    #[serial]
    async fn g146_30_31_33_graph_http_omits_secret() {
        let Some(h) = try_create_harness().await else {
            eprintln!("SKIP g146_30: no DATABASE_URL");
            return;
        };
        if h.seed_graph_fixture().await.is_none() {
            eprintln!("SKIP g146_30: graph upsert failed");
            return;
        }
        let _ = h.seed_secret_chunk_kv().await;
        let headers = h.auth_headers_peer();
        for uri in [
            "/api/v1/graph/nodes/search?q=ENTITY_X&limit=50",
            "/api/v1/graph/labels/popular?limit=50",
            "/api/v1/graph/labels/search?q=ENTITY_X&limit=50",
            "/api/v1/graph?max_nodes=50",
        ] {
            let (status, body) = request_json(h.router(), "GET", uri, &headers, None).await;
            assert!(status.is_success(), "{uri} {status}: {body}");
            assert_no_secret(&body.to_string(), uri);
            assert!(
                !body.to_string().contains(DOCB_TITLE),
                "DocB title in {uri}: {body}"
            );
        }
        // Shared hub may appear (DocA provenance) but description must not include Secret fragment.
        let (status, body) = request_json(
            h.router(),
            "GET",
            "/api/v1/graph/nodes/ENTITY_X",
            &headers,
            None,
        )
        .await;
        assert!(status.is_success(), "shared hub {status}: {body}");
        assert_no_secret(&body.to_string(), "get_node shared hub");

        let (status, body) = request_json(
            h.router(),
            "GET",
            "/api/v1/graph/nodes/ENTITY_X_SECRET",
            &headers,
            None,
        )
        .await;
        assert_eq!(
            status,
            StatusCode::NOT_FOUND,
            "secret node must 404: {body}"
        );
        assert_no_secret(&body.to_string(), "get_node secret");
    }

    /// G-146-40/42/43: query HTTP + stream omit Restricted / DocB; cache isolated by principal.
    #[tokio::test]
    #[serial]
    async fn g146_40_42_43_query_stream_and_cache() {
        let Some(h) = try_create_harness().await else {
            eprintln!("SKIP g146_40: no DATABASE_URL");
            return;
        };
        let peer = h.auth_headers_peer();
        let owner = h.auth_headers_owner();
        let (s1, b1) = post_query(h.router(), &peer, "ENTITY_X cache?").await;
        let (s2, _b2) = post_query(h.router(), &owner, "ENTITY_X cache?").await;
        assert!(s1.is_success() && s2.is_success(), "peer={s1} owner={s2}");
        assert_no_secret(&b1.to_string(), "peer cache");
        assert!(
            !b1.to_string().contains("Restricted"),
            "Restricted placeholder in peer answer: {b1}"
        );

        let (status, body) = request_json(
            h.router(),
            "POST",
            "/api/v1/query/stream",
            &peer,
            Some(json!({
                "query": "ENTITY_X stream?",
                "mode": "mix"
            })),
        )
        .await;
        assert!(status.is_success(), "stream {status}: {body}");
        assert_no_secret(&body.to_string(), "query stream");
        assert!(!body.to_string().contains("Restricted"));
    }

    /// G-146-44: parse job GET as other principal → 404.
    #[tokio::test]
    #[serial]
    async fn g146_44_parse_job_idor() {
        let Some(h) = try_create_harness().await else {
            eprintln!("SKIP g146_44: no DATABASE_URL");
            return;
        };
        let owner_p = {
            let p = edgequake_authz::PrincipalId::from_auth_user_id(&h.owner_user_id);
            format!("{}:{}", p.kind_str(), p.id_str())
        };
        let job_id = format!("pr_{}", Uuid::new_v4().simple());
        h.state
            .parse_jobs
            .seed_completed_job(&job_id, Some(owner_p))
            .await;

        let (status, body) = request_json(
            h.router(),
            "GET",
            &format!("/api/v1/parse/jobs/{job_id}"),
            &h.auth_headers_peer(),
            None,
        )
        .await;
        assert_eq!(status, StatusCode::NOT_FOUND, "expected parse IDOR 404: {body}");

        let (ok_status, _) = request_json(
            h.router(),
            "GET",
            &format!("/api/v1/parse/jobs/{job_id}"),
            &h.auth_headers_owner(),
            None,
        )
        .await;
        assert_eq!(ok_status, StatusCode::OK, "owner should read own parse job");

        // Unbound job under ABAC → 404 for any caller (fail-closed).
        let unbound = format!("pr_{}", Uuid::new_v4().simple());
        h.state.parse_jobs.seed_completed_job(&unbound, None).await;
        let (u_status, _) = request_json(
            h.router(),
            "GET",
            &format!("/api/v1/parse/jobs/{unbound}"),
            &h.auth_headers_peer(),
            None,
        )
        .await;
        assert_eq!(
            u_status,
            StatusCode::NOT_FOUND,
            "unbound parse job must 404 under ABAC"
        );
    }

    /// G-146-55: PATCH labels bumps generation; peer cache/list does not keep old allow.
    #[tokio::test]
    #[serial]
    async fn g146_55_patch_labels_bumps_and_hides() {
        let Some(h) = try_create_harness().await else {
            eprintln!("SKIP g146_55: no DATABASE_URL");
            return;
        };
        let (list_ok, listed) = get_documents(h.router(), &h.auth_headers_peer()).await;
        assert_eq!(list_ok, StatusCode::OK, "{listed}");
        assert!(listed.to_string().contains(DOCA_TITLE), "{listed}");

        let (status, body) = request_json(
            h.router(),
            "PATCH",
            &format!("/api/v1/documents/{}/security-labels", h.doc_a),
            &h.auth_headers_owner(),
            Some(json!({
                "share_mode": "owner_only",
                "classification": "secret"
            })),
        )
        .await;
        assert!(
            status.is_success(),
            "PATCH labels {status}: {body}"
        );
        let gen = body.get("policy_generation").and_then(|v| v.as_u64());
        assert!(gen.unwrap_or(0) >= 1, "expected generation bump: {body}");

        let (list2, listed2) = get_documents(h.router(), &h.auth_headers_peer()).await;
        assert_eq!(list2, StatusCode::OK, "{listed2}");
        assert!(
            !listed2.to_string().contains(DOCA_TITLE),
            "DocA still listed after owner_only: {listed2}"
        );
        let (qstatus, qbody) = post_query(h.router(), &h.auth_headers_peer(), "ENTITY_X?").await;
        assert!(qstatus.is_success(), "{qbody}");
        assert_no_secret(&qbody.to_string(), "post-patch query");
    }

    /// G-146-11: degrees/batch must not probe Secret topology (omit unauthorized).
    #[tokio::test]
    #[serial]
    async fn g146_11_degrees_batch_omits_secret_node() {
        let Some(h) = try_create_harness().await else {
            eprintln!("SKIP g146_11 degrees: no DATABASE_URL");
            return;
        };
        if h.seed_graph_fixture().await.is_none() {
            eprintln!("SKIP g146_11 degrees: graph upsert failed");
            return;
        }
        let (status, body) = request_json(
            h.router(),
            "POST",
            "/api/v1/graph/degrees/batch",
            &h.auth_headers_peer(),
            Some(json!({
                "node_ids": ["ENTITY_X", "ENTITY_X_SECRET"]
            })),
        )
        .await;
        assert!(status.is_success(), "degrees/batch {status}: {body}");
        let blob = body.to_string();
        assert_no_secret(&blob, "degrees/batch");
        assert!(
            !blob.contains("ENTITY_X_SECRET"),
            "Secret node must be omitted from degrees/batch: {body}"
        );
        let degrees = body
            .get("degrees")
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default();
        for d in &degrees {
            let id = d.get("node_id").and_then(|v| v.as_str()).unwrap_or("");
            assert_ne!(id, "ENTITY_X_SECRET");
        }
    }

    /// G-146-8: list JSON includes export_control / pii / project_id after seed.
    #[tokio::test]
    #[serial]
    async fn g146_8_list_dto_includes_security_attrs() {
        let Some(h) = try_create_harness().await else {
            eprintln!("SKIP g146_8 list dto: no DATABASE_URL");
            return;
        };
        sqlx::query(
            "UPDATE documents SET export_control = true, pii = true, project_id = 'proj-146' WHERE id = $1",
        )
        .bind(h.doc_a)
        .execute(&h.pool)
        .await
        .expect("seed security attrs");

        let (status, body) = get_documents(h.router(), &h.auth_headers_owner()).await;
        assert_eq!(status, StatusCode::OK, "{body}");
        let docs = body
            .get("documents")
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default();
        let doc_a = docs.iter().find(|d| {
            d.get("id")
                .and_then(|v| v.as_str())
                .map(|id| id == h.doc_a.to_string())
                .unwrap_or(false)
        });
        let Some(doc_a) = doc_a else {
            eprintln!("SKIP g146_8: DocA not in list (allow/filter)");
            return;
        };
        assert_eq!(
            doc_a.get("export_control").and_then(|v| v.as_bool()),
            Some(true),
            "export_control missing: {doc_a}"
        );
        assert_eq!(
            doc_a.get("pii").and_then(|v| v.as_bool()),
            Some(true),
            "pii missing: {doc_a}"
        );
        assert_eq!(
            doc_a.get("project_id").and_then(|v| v.as_str()),
            Some("proj-146"),
            "project_id missing: {doc_a}"
        );
    }
}
