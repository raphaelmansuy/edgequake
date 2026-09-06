//! SPEC-146 — document PEP + existence-hiding + dual-write (HTTP+PG when available).
//!
//! Fast unit guards always run. Live gates skip when DATABASE_URL is unset.
//!
//! Run:
//!   cargo test -p edgequake-api --test e2e_spec146_document_pep --features postgres

mod common;

use edgequake_api::services::spec146_authz::{
    existence_hiding_not_found, filter_metadata_entries_by_allow_set, parse_security_labels,
    SecurityAdmissionLabels, SecurityFormOverrides,
};
use edgequake_authz::{AllowSet, PrincipalId};
use serde_json::json;
use uuid::Uuid;

#[test]
fn existence_hiding_helper_is_not_found() {
    let err = existence_hiding_not_found();
    let msg = format!("{err:?}");
    assert!(
        msg.contains("NotFound") || msg.to_lowercase().contains("not found"),
        "expected NotFound helper, got {msg}"
    );
    assert!(!msg.to_lowercase().contains("forbidden"));
    assert!(!msg.to_lowercase().contains("not_in_allow_set"));
}

#[test]
fn security_labels_default_do_not_affect_allow_set() {
    let labels = SecurityAdmissionLabels::default();
    assert!(!labels.affects_allow_set());
    let parsed = parse_security_labels(None, &SecurityFormOverrides::default());
    assert_eq!(parsed.share_mode, "workspace");
    assert_eq!(parsed.security_status, "ok");
}

#[test]
fn g146_53_worker_principal_maps_and_allow_set_empty() {
    let worker = PrincipalId::from_auth_user_id("worker-ingest-1");
    assert!(matches!(worker, PrincipalId::Worker));
    assert_eq!(worker.kind_str(), "worker");
    let allow = AllowSet::empty();
    assert!(allow.is_empty());
}

#[test]
fn list_pep_filters_unauthorized_titles_from_kv_entries() {
    let allowed = Uuid::new_v4();
    let secret = Uuid::new_v4();
    let allow = AllowSet::from_ids([allowed]);
    let entries = vec![
        (
            format!("{allowed}-metadata"),
            json!({"id": allowed.to_string(), "title": "Public Doc"}),
        ),
        (
            format!("{secret}-metadata"),
            json!({"id": secret.to_string(), "title": "Secret Doc"}),
        ),
    ];
    let filtered = filter_metadata_entries_by_allow_set(entries, &allow);
    assert_eq!(filtered.len(), 1);
    assert_eq!(
        filtered[0].1.get("title").and_then(|v| v.as_str()),
        Some("Public Doc")
    );
}

#[cfg(feature = "postgres")]
mod live {
    use super::*;
    use axum::http::StatusCode;
    use common::spec146_pg::{
        get_document, get_documents, try_create_harness, DOCA_TITLE, DOCB_TITLE, SECRET_TOKEN,
    };
    use serial_test::serial;

    /// G-146-10/11/16: peer sees DocA, not DocB; detail DocB → 404 existence-hiding.
    #[tokio::test]
    #[serial]
    async fn g146_10_11_16_list_and_detail_existence_hiding() {
        let Some(h) = try_create_harness().await else {
            eprintln!("SKIP g146_10_11_16: no DATABASE_URL / harness failed");
            return;
        };

        let headers = h.auth_headers_peer();
        let (status, body) = get_documents(h.router(), &headers).await;
        assert_eq!(status, StatusCode::OK, "list failed: {body}");

        let blob = body.to_string();
        assert!(
            blob.contains(DOCA_TITLE),
            "DocA title missing from list: {blob}"
        );
        assert!(
            !blob.contains(DOCB_TITLE),
            "DocB title leaked in list: {blob}"
        );
        assert!(
            !blob.contains("Restricted") && !blob.to_lowercase().contains("hidden"),
            "existence-hiding leak: {blob}"
        );

        let (detail_status, detail) = get_document(h.router(), &headers, h.doc_b).await;
        assert_eq!(
            detail_status,
            StatusCode::NOT_FOUND,
            "expected 404 for unauthorized DocB: {detail}"
        );
        let detail_s = detail.to_string();
        assert!(
            detail_s.to_lowercase().contains("not found")
                || detail_s.contains("Document not found"),
            "existence-hiding copy missing: {detail_s}"
        );
        assert!(
            !detail_s.contains("not_in_allow_set"),
            "deny reason leaked: {detail_s}"
        );
    }

    /// G-146-12: dual-write classification on SQL documents table after labels set.
    #[tokio::test]
    #[serial]
    async fn g146_12_sql_and_kv_labels_dual_write() {
        let Some(h) = try_create_harness().await else {
            eprintln!("SKIP g146_12: no DATABASE_URL");
            return;
        };

        let class: Option<String> = sqlx::query_scalar(
            "SELECT classification FROM documents WHERE id = $1",
        )
        .bind(h.doc_b)
        .fetch_one(&h.pool)
        .await
        .ok();
        assert_eq!(class.as_deref(), Some("secret"));

        let share: Option<String> =
            sqlx::query_scalar("SELECT share_mode FROM documents WHERE id = $1")
                .bind(h.doc_b)
                .fetch_one(&h.pool)
                .await
                .ok();
        assert_eq!(share.as_deref(), Some("owner_only"));

        // KV dual-write
        let key = format!("{}-metadata", h.doc_b);
        let kv = h
            .state
            .storage
            .kv_storage
            .get_by_id(&key)
            .await
            .expect("kv get");
        let meta = kv.expect("kv metadata present");
        assert_eq!(
            meta.get("classification").and_then(|v| v.as_str()),
            Some("secret")
        );
        assert_eq!(
            meta.get("share_mode").and_then(|v| v.as_str()),
            Some("owner_only")
        );
        let _ = SECRET_TOKEN; // keep fixture linked
    }

    /// G-146-90: ABAC off → both titles listed.
    #[tokio::test]
    #[serial]
    async fn g146_90_flag_off_lists_both() {
        let Some(h) = try_create_harness().await else {
            eprintln!("SKIP g146_90: no DATABASE_URL");
            return;
        };

        let headers = h.auth_headers_peer();
        let (status, body) = get_documents(h.router_abac_off(), &headers).await;
        assert_eq!(status, StatusCode::OK, "{body}");
        let blob = body.to_string();
        assert!(blob.contains(DOCA_TITLE), "missing DocA: {blob}");
        assert!(blob.contains(DOCB_TITLE), "missing DocB when ABAC off: {blob}");
    }
}
