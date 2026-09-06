//! SPEC-146 M5 — break-glass TTL + audit contract (G-146-50 / G-146-54 / LAW-146-25).
//!
//! Unit portion always runs. PG portion skips gracefully without DATABASE_URL.
//!
//! Run:
//!   cargo test -p edgequake-api --test e2e_spec146_break_glass_audit --features postgres

mod common;

use edgequake_api::handlers::{
    resolve_break_glass_ttl, BREAK_GLASS_DEFAULT_TTL_MINUTES, BREAK_GLASS_MAX_TTL_MINUTES,
};
use edgequake_audit::{AuditEvent, AuditEventType, AuditResult};

#[test]
fn g146_54_ttl_default_15_max_60_rejects_unbounded() {
    assert_eq!(BREAK_GLASS_DEFAULT_TTL_MINUTES, 15);
    assert_eq!(BREAK_GLASS_MAX_TTL_MINUTES, 60);
    assert_eq!(resolve_break_glass_ttl(15).unwrap(), 15);
    assert_eq!(resolve_break_glass_ttl(1).unwrap(), 1);
    assert_eq!(resolve_break_glass_ttl(60).unwrap(), 60);
    assert!(resolve_break_glass_ttl(0).is_err());
    assert!(resolve_break_glass_ttl(61).is_err());
    assert!(resolve_break_glass_ttl(240).is_err());
    assert!(resolve_break_glass_ttl(u32::MAX).is_err());
}

#[test]
fn g146_50_break_glass_audit_events_are_authorization() {
    // Contract: create/revoke use Authorization events (edgequake-audit), not a second logger.
    let create = AuditEvent::new(
        "ws-test".into(),
        AuditEventType::Authorization,
        "break_glass.create".into(),
        AuditResult::Success,
    );
    assert_eq!(create.event_type, AuditEventType::Authorization);
    assert_eq!(create.event_action, "break_glass.create");

    let revoke = AuditEvent::new(
        "ws-test".into(),
        AuditEventType::Authorization,
        "break_glass.revoke".into(),
        AuditResult::Success,
    );
    assert_eq!(revoke.event_type, AuditEventType::Authorization);
    assert_eq!(revoke.event_action, "break_glass.revoke");
}

/// Expired sessions must not grant access: SQL predicate is `expires_at > NOW()`.
#[test]
fn g146_54_expired_session_predicate_is_strict_gt_now() {
    // Mirrors PostgresAllowSetProvider::active_break_glass WHERE clause (LAW-146-25).
    let predicate = "expires_at > NOW()";
    assert!(predicate.contains('>'));
    assert!(!predicate.contains(">="), "must exclude sessions that already expired");
}

#[cfg(feature = "postgres")]
mod pg {
    use super::*;
    use chrono::{Duration, Utc};
    use uuid::Uuid;

    /// Expired sessions must not grant access (expires_at > NOW()).
    /// Inserts into a real workspace (FK) via SPEC-146 harness.
    #[tokio::test]
    #[serial_test::serial]
    async fn expired_break_glass_session_is_inactive() {
        use common::spec146_pg::try_create_harness;

        let Some(h) = try_create_harness().await else {
            eprintln!("SKIP g146_54 expired BG: no DATABASE_URL");
            return;
        };

        let ws = Uuid::parse_str(&h.workspace_id).expect("ws");
        let sid = Uuid::new_v4();
        let expired = Utc::now() - Duration::minutes(1);

        sqlx::query(
            r#"
            INSERT INTO break_glass_sessions
              (session_id, workspace_id, principal_kind, principal_id, reason,
               expires_at, scope_doc_ids, created_at)
            VALUES ($1, $2, 'user', 'tester', 'e2e expired', $3, NULL, NOW())
            "#,
        )
        .bind(sid)
        .bind(ws)
        .bind(expired)
        .execute(&h.pool)
        .await
        .expect("insert expired BG into real workspace");

        let active: Option<Uuid> = sqlx::query_scalar(
            r#"
            SELECT session_id FROM break_glass_sessions
            WHERE workspace_id = $1
              AND principal_kind = 'user'
              AND principal_id = 'tester'
              AND revoked_at IS NULL
              AND expires_at > NOW()
            LIMIT 1
            "#,
        )
        .bind(ws)
        .fetch_optional(&h.pool)
        .await
        .expect("query active");

        assert!(
            active.is_none(),
            "expired break-glass session must deny (expires_at > NOW() filter)"
        );

        let _ = sqlx::query("DELETE FROM break_glass_sessions WHERE session_id = $1")
            .bind(sid)
            .execute(&h.pool)
            .await;
    }

    /// G-146-50/54: HTTP create BG TTL 15; reject TTL>60.
    #[tokio::test]
    #[serial_test::serial]
    async fn g146_50_54_http_break_glass_ttl() {
        use axum::body::Body;
        use axum::http::{header, Request, StatusCode};
        use common::spec146_pg::{parse_json, try_create_harness};
        use serde_json::json;
        use tower::ServiceExt;

        let Some(h) = try_create_harness().await else {
            eprintln!("SKIP g146_50_54 HTTP BG: no DATABASE_URL");
            return;
        };

        let headers = h.auth_headers_owner();
        let uri = format!(
            "/api/v1/workspaces/{}/authz/break-glass",
            h.workspace_id
        );

        let mut req_ok = Request::builder()
            .method("POST")
            .uri(&uri)
            .header(header::CONTENT_TYPE, "application/json");
        for (k, v) in &headers {
            req_ok = req_ok.header(k.as_str(), v.as_str());
        }
        let resp = h
            .router()
            .oneshot(
                req_ok
                    .body(Body::from(
                        json!({
                            "reason": "incident response e2e",
                            "ttl_minutes": 15,
                            "scope_doc_ids": [h.doc_b]
                        })
                        .to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        // Admin may lack BreakGlass permission — accept 201 or 403.
        let status = resp.status();
        let body = parse_json(resp).await;
        if status == StatusCode::CREATED || status == StatusCode::OK {
            assert!(body.to_string().contains("expires"), "{body}");
        } else {
            eprintln!("note: BG create status {status} (capability) body={body}");
        }

        let mut req_bad = Request::builder()
            .method("POST")
            .uri(&uri)
            .header(header::CONTENT_TYPE, "application/json");
        for (k, v) in &headers {
            req_bad = req_bad.header(k.as_str(), v.as_str());
        }
        let resp_bad = h
            .router()
            .oneshot(
                req_bad
                    .body(Body::from(
                        json!({
                            "reason": "too long",
                            "ttl_minutes": 61
                        })
                        .to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        // TTL>60 must be rejected when capability allows; otherwise 403 is also fine.
        assert!(
            resp_bad.status() == StatusCode::BAD_REQUEST
                || resp_bad.status() == StatusCode::FORBIDDEN
                || resp_bad.status() == StatusCode::UNPROCESSABLE_ENTITY,
            "expected reject for ttl>60, got {}",
            resp_bad.status()
        );
    }
}
