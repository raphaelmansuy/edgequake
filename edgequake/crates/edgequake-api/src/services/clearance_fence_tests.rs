//! Live contract: run with the isolated SPEC180_DATABASE_URL, never the default database.
use super::*;
use futures::StreamExt;

#[tokio::test]
#[ignore = "requires the disposable SPEC-180 database and seed.sql"]
async fn response_fence_aborts_after_policy_epoch_changes() {
    let url = std::env::var("SPEC180_DATABASE_URL").expect("SPEC180_DATABASE_URL is required");
    assert_eq!(url.rsplit('/').next(), Some("edgequake_spec180"));
    let pool = std::sync::Arc::new(sqlx::PgPool::connect(&url).await.unwrap());
    let workspace = Uuid::parse_str("00000000-0000-0000-0000-000000000003").unwrap();
    let user = Uuid::parse_str("18000000-0000-4000-8000-000000000011").unwrap();
    let (_, session) = load_scope(&pool, workspace, Some(user)).await.unwrap();
    assert!(session.enforced);
    let (sender, receiver) =
        tokio::sync::mpsc::channel::<Result<axum::body::Bytes, std::io::Error>>(2);
    let response = axum::response::Response::new(axum::body::Body::from_stream(
        tokio_stream::wrappers::ReceiverStream::new(receiver),
    ));
    let fenced = fence_clearance_response(response, pool.clone(), workspace, session);
    let mut body = fenced.into_body().into_data_stream();
    sender
        .send(Ok(axum::body::Bytes::from_static(
            b"authorized-first-chunk",
        )))
        .await
        .unwrap();
    assert_eq!(
        body.next().await.unwrap().unwrap(),
        "authorized-first-chunk"
    );
    sqlx::query("UPDATE public.workspace_clearance_settings SET policy_epoch=policy_epoch+1 WHERE workspace_id=$1").bind(workspace).execute(pool.as_ref()).await.unwrap();
    sender
        .send(Ok(axum::body::Bytes::from_static(
            b"forbidden-second-chunk",
        )))
        .await
        .unwrap();
    assert!(body.next().await.unwrap().is_err());
    assert!(body.next().await.is_none());
    assert!(body.next().await.is_none());
    pool.close().await;
}

#[tokio::test]
#[ignore = "requires the disposable SPEC-180 database and seed.sql"]
async fn http_stream_aborts_after_membership_revocation() {
    let url = std::env::var("SPEC180_DATABASE_URL").expect("SPEC180_DATABASE_URL is required");
    assert_eq!(url.rsplit('/').next(), Some("edgequake_spec180"));
    let pool = std::sync::Arc::new(sqlx::PgPool::connect(&url).await.unwrap());
    let workspace = Uuid::parse_str("00000000-0000-0000-0000-000000000003").unwrap();
    let user = Uuid::parse_str("18000000-0000-4000-8000-000000000012").unwrap();
    let (_, session) = load_scope(&pool, workspace, Some(user)).await.unwrap();
    let (sender, receiver) =
        tokio::sync::mpsc::channel::<Result<axum::body::Bytes, std::io::Error>>(2);
    let receiver = std::sync::Arc::new(tokio::sync::Mutex::new(Some(receiver)));
    let endpoint_pool = pool.clone();
    let app = axum::Router::new().route(
        "/stream",
        axum::routing::get(move || {
            let receiver = receiver.clone();
            let pool = endpoint_pool.clone();
            let session = session.clone();
            async move {
                let body =
                    axum::body::Body::from_stream(tokio_stream::wrappers::ReceiverStream::new(
                        receiver.lock().await.take().unwrap(),
                    ));
                fence_clearance_response(
                    axum::response::Response::new(body),
                    pool,
                    workspace,
                    session,
                )
            }
        }),
    );
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let server = tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    sender
        .send(Ok(axum::body::Bytes::from_static(b"authorized")))
        .await
        .unwrap();
    let response = reqwest::get(format!("http://{address}/stream"))
        .await
        .unwrap();
    assert_eq!(response.headers()["cache-control"], "private, no-store");
    let mut stream = response.bytes_stream();
    assert_eq!(stream.next().await.unwrap().unwrap(), "authorized");
    sqlx::query(
        "UPDATE public.memberships SET is_active=false WHERE workspace_id=$1 AND user_id=$2",
    )
    .bind(workspace)
    .bind(user)
    .execute(pool.as_ref())
    .await
    .unwrap();
    sender
        .send(Ok(axum::body::Bytes::from_static(b"forbidden")))
        .await
        .unwrap();
    let next = tokio::time::timeout(std::time::Duration::from_secs(5), stream.next()).await;
    sqlx::query(
        "UPDATE public.memberships SET is_active=true WHERE workspace_id=$1 AND user_id=$2",
    )
    .bind(workspace)
    .bind(user)
    .execute(pool.as_ref())
    .await
    .unwrap();
    server.abort();
    assert!(
        matches!(next, Ok(None) | Ok(Some(Err(_)))),
        "revoked HTTP response must terminate without another content chunk"
    );
    pool.close().await;
}

#[test]
fn pdf_identifiers_are_guarded_before_and_after_router_nesting() {
    let id = Uuid::parse_str("18000000-0000-4000-8000-000000000041").unwrap();
    for prefix in ["/api/v1", ""] {
        for suffix in ["", "/content", "/download", "/retry", "/cancel"] {
            assert_eq!(
                pdf_id_in_path(&format!("{prefix}/documents/pdf/{id}{suffix}")),
                Some(id)
            );
        }
    }
    for path in [
        "/api/v1/documents/pdf/batch",
        "/documents/pdf/progress/track",
        "/documents/labels/batch",
    ] {
        assert_eq!(pdf_id_in_path(path), None);
    }
}
