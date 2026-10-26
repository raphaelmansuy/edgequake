//! SPEC-173 signed upload e2e. Database mode uses the relay. MinIO presign is separate.

mod common;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use common::spec028_mcp::{build_mcp_app, mcp_tools_call, tool_structured};
use edgequake_api::AppState;
use serde_json::json;
use sha2::{Digest, Sha256};
use tower::ServiceExt;

fn sha256_hex(bytes: &[u8]) -> String {
    hex::encode(Sha256::digest(bytes))
}

async fn relay_app() -> axum::Router {
    let mut state = AppState::test_state();
    state.auth.config.jwt_secret = "spec173-test-hmac-key-not-the-default-value".into();
    state.workspace_service.seed_default_workspace().await;
    build_mcp_app(state)
}

fn api_path(url: &str) -> String {
    let (_, rest) = url
        .split_once("/api/v1/")
        .expect("relay url contains /api/v1/");
    format!("/api/v1/{rest}")
}

async fn put_bytes(
    app: &axum::Router,
    url: &str,
    content_type: &str,
    body: &[u8],
    auth: Option<&str>,
) -> StatusCode {
    let mut builder = Request::builder()
        .method("PUT")
        .uri(api_path(url))
        .header("content-type", content_type);
    if let Some(auth) = auth {
        builder = builder.header("authorization", auth);
    }
    let response = app
        .clone()
        .oneshot(builder.body(Body::from(body.to_vec())).unwrap())
        .await
        .unwrap();
    response.status()
}

#[tokio::test]
async fn relay_put_commits_text() {
    let app = relay_app().await;
    let payload = b"hello from spec 173";
    let sha = sha256_hex(payload);
    let (_, begin) = mcp_tools_call(
        &app,
        "/mcp",
        "eq_upload_begin",
        json!({
            "filename": "note.txt",
            "media_type": "text/plain",
            "byte_length": payload.len(),
            "sha256": sha,
        }),
    )
    .await;
    let sc = tool_structured(&begin);
    assert_eq!(sc["ok"], true, "{sc}");
    assert_eq!(sc["transfer"], "relay");
    assert_eq!(sc["method"], "PUT");
    assert!(sc.get("url").is_some());
    let url = sc["url"].as_str().unwrap().to_string();
    let upload_id = sc["upload_id"].as_str().unwrap().to_string();
    assert_eq!(
        put_bytes(&app, &url, "text/plain", payload, None).await,
        StatusCode::NO_CONTENT
    );
    let (_, commit) = mcp_tools_call(
        &app,
        "/mcp",
        "eq_upload_commit",
        json!({"upload_id": upload_id}),
    )
    .await;
    let committed = tool_structured(&commit);
    assert_eq!(committed["status"], "pending", "{committed}");
    assert!(committed["document_id"].is_string());
    let doc = committed["document_id"].clone();
    let (_, again) = mcp_tools_call(
        &app,
        "/mcp",
        "eq_upload_commit",
        json!({"upload_id": upload_id}),
    )
    .await;
    assert_eq!(tool_structured(&again)["document_id"], doc);
}

#[tokio::test]
async fn begin_without_sha256_has_no_url() {
    let app = relay_app().await;
    let (_, begin) = mcp_tools_call(
        &app,
        "/mcp",
        "eq_upload_begin",
        json!({"filename": "a.pdf", "byte_length": 10}),
    )
    .await;
    let sc = tool_structured(&begin);
    assert_eq!(sc["ok"], false, "{sc}");
    assert_eq!(sc["error"]["code"], "eq/invalid_id");
    assert!(sc.get("url").is_none());
}

#[tokio::test]
async fn oversize_begin_has_no_url() {
    let mut state = AppState::test_state();
    state.auth.config.jwt_secret = "spec173-test-hmac-key-not-the-default-value".into();
    state.config.max_document_size = 4;
    state.workspace_service.seed_default_workspace().await;
    let app = build_mcp_app(state);
    let sha = "ab".repeat(32);
    let (_, begin) = mcp_tools_call(
        &app,
        "/mcp",
        "eq_upload_begin",
        json!({
            "filename": "a.txt",
            "byte_length": 8,
            "sha256": sha,
        }),
    )
    .await;
    let sc = tool_structured(&begin);
    assert_eq!(sc["error"]["code"], "eq/budget_exceeded");
    assert!(sc.get("url").is_none());
}

#[tokio::test]
async fn wrong_type_and_bad_sig_and_oversize_body() {
    let app = relay_app().await;
    let payload = b"abcd";
    let sha = sha256_hex(payload);
    let (_, begin) = mcp_tools_call(
        &app,
        "/mcp",
        "eq_upload_begin",
        json!({
            "filename": "a.txt",
            "media_type": "text/plain",
            "byte_length": payload.len(),
            "sha256": sha,
        }),
    )
    .await;
    let sc = tool_structured(&begin);
    let url = sc["url"].as_str().unwrap().to_string();
    assert_eq!(
        put_bytes(&app, &url, "application/pdf", payload, None).await,
        StatusCode::FORBIDDEN
    );
    let tampered = url.replace("sig=", "sig=00");
    assert_eq!(
        put_bytes(&app, &tampered, "text/plain", payload, None).await,
        StatusCode::FORBIDDEN
    );
    let big = b"abcdef";
    assert_eq!(
        put_bytes(&app, &url, "text/plain", big, None).await,
        StatusCode::PAYLOAD_TOO_LARGE
    );
}

#[tokio::test]
async fn default_jwt_secret_does_not_mint_a_relay_url() {
    let state = AppState::test_state();
    let app = build_mcp_app(state);
    let (_, begin) = mcp_tools_call(
        &app,
        "/mcp",
        "eq_upload_begin",
        json!({
            "filename": "a.txt",
            "byte_length": 1,
            "sha256": "aa".repeat(32),
        }),
    )
    .await;
    let sc = tool_structured(&begin);
    assert_eq!(sc["error"]["code"], "eq/not_implemented");
    assert!(sc.get("url").is_none());
}

#[tokio::test]
async fn zero_length_commit_before_put_and_abort() {
    let app = relay_app().await;
    let (_, zero) = mcp_tools_call(
        &app,
        "/mcp",
        "eq_upload_begin",
        json!({
            "filename": "a.txt",
            "byte_length": 0,
            "sha256": "aa".repeat(32),
        }),
    )
    .await;
    let zero = tool_structured(&zero);
    assert_eq!(zero["error"]["code"], "eq/invalid_id");
    assert!(zero.get("url").is_none());

    let payload = b"wait";
    let sha = sha256_hex(payload);
    let (_, begin) = mcp_tools_call(
        &app,
        "/mcp",
        "eq_upload_begin",
        json!({
            "filename": "../cafe.pdf",
            "media_type": "text/plain",
            "byte_length": payload.len(),
            "sha256": sha,
            "url": "https://evil.example/steal",
            "file": "file:///etc/passwd",
        }),
    )
    .await;
    let sc = tool_structured(&begin);
    let url = sc["url"].as_str().unwrap();
    assert!(url.contains("/api/v1/mcp/uploads/"), "{url}");
    assert!(!url.contains("evil.example"));
    assert!(!url.contains(".."));
    let upload_id = sc["upload_id"].as_str().unwrap();
    let (_, early) = mcp_tools_call(
        &app,
        "/mcp",
        "eq_upload_commit",
        json!({"upload_id": upload_id}),
    )
    .await;
    assert_eq!(tool_structured(&early)["error"]["code"], "eq/not_found");

    let (_, abort_begin) = mcp_tools_call(
        &app,
        "/mcp",
        "eq_upload_begin",
        json!({
            "filename": "gone.txt",
            "media_type": "text/plain",
            "byte_length": 1,
            "sha256": "bb".repeat(32),
        }),
    )
    .await;
    let abort_id = tool_structured(&abort_begin)["upload_id"]
        .as_str()
        .unwrap()
        .to_string();
    mcp_tools_call(
        &app,
        "/mcp",
        "eq_upload_abort",
        json!({"upload_id": abort_id}),
    )
    .await;
    let (_, after) = mcp_tools_call(
        &app,
        "/mcp",
        "eq_upload_commit",
        json!({"upload_id": abort_id}),
    )
    .await;
    assert_eq!(tool_structured(&after)["error"]["code"], "eq/not_found");
}

#[tokio::test]
async fn same_length_wrong_hash_is_forbidden() {
    let app = relay_app().await;
    let sha = sha256_hex(b"abcd");
    let (_, begin) = mcp_tools_call(
        &app,
        "/mcp",
        "eq_upload_begin",
        json!({
            "filename": "a.txt",
            "media_type": "text/plain",
            "byte_length": 4,
            "sha256": sha,
        }),
    )
    .await;
    let url = tool_structured(&begin)["url"].as_str().unwrap().to_string();
    assert_eq!(
        put_bytes(&app, &url, "text/plain", b"abce", None).await,
        StatusCode::FORBIDDEN
    );
}

#[tokio::test]
async fn query_profile_omits_upload_and_schema_names_chunk_base64() {
    use edgequake_api::mcp::gateway::tools::tools_list_for;
    use edgequake_api::mcp::project::profile::McpProfile;
    let control = tools_list_for(McpProfile::Control);
    let write = control["tools"]
        .as_array()
        .unwrap()
        .iter()
        .find(|tool| tool["name"] == "eq_upload_write")
        .unwrap();
    assert!(write["inputSchema"]["properties"]
        .get("chunk_base64")
        .is_some());
    let query = tools_list_for(McpProfile::Query);
    assert!(query["tools"]
        .as_array()
        .unwrap()
        .iter()
        .all(|tool| tool["name"] != "eq_upload_begin"));
}

#[tokio::test(flavor = "current_thread")]
async fn relay_log_does_not_contain_the_signature() {
    let app = relay_app().await;
    let payload = b"logged";
    let sha = sha256_hex(payload);
    let (_, begin) = mcp_tools_call(
        &app,
        "/mcp",
        "eq_upload_begin",
        json!({
            "filename": "a.txt",
            "media_type": "text/plain",
            "byte_length": payload.len(),
            "sha256": sha,
        }),
    )
    .await;
    let sc = tool_structured(&begin);
    let url = sc["url"].as_str().unwrap().to_string();
    assert!(url.contains("sig="));
    let buf = LogBuf(std::sync::Arc::new(std::sync::Mutex::new(Vec::new())));
    let subscriber = tracing_subscriber::fmt()
        .with_ansi(false)
        .with_max_level(tracing::Level::INFO)
        .with_writer(buf.clone())
        .finish();
    let _guard = tracing::subscriber::set_default(subscriber);
    assert_eq!(
        put_bytes(&app, &url, "text/plain", payload, None).await,
        StatusCode::NO_CONTENT
    );
    drop(_guard);
    let text = String::from_utf8(buf.0.lock().unwrap().clone()).unwrap();
    assert!(
        text.contains("mcp upload relay stored"),
        "expected the relay log, got {text}"
    );
    assert!(!text.contains("sig="), "{text}");
    assert!(!text.contains(&sha), "{text}");
}

#[derive(Clone)]
struct LogBuf(std::sync::Arc<std::sync::Mutex<Vec<u8>>>);

impl std::io::Write for LogBuf {
    fn write(&mut self, data: &[u8]) -> std::io::Result<usize> {
        self.0.lock().unwrap().extend_from_slice(data);
        Ok(data.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

impl<'a> tracing_subscriber::fmt::MakeWriter<'a> for LogBuf {
    type Writer = LogBuf;

    fn make_writer(&'a self) -> Self::Writer {
        self.clone()
    }
}

#[tokio::test]
async fn second_copy_is_duplicate_and_pdf_admits() {
    let app = relay_app().await;
    let payload = b"duplicate-body-spec173";
    let first = put_and_commit(&app, "note.txt", "text/plain", payload).await;
    assert_eq!(first["status"], "pending", "{first}");
    let second = put_and_commit(&app, "note-again.txt", "text/plain", payload).await;
    assert_eq!(second["status"], "duplicate_processing", "{second}");

    let pdf = b"%PDF-1.4\n1 0 obj<<>>endobj\ntrailer<<>>\n%%EOF\n";
    let admitted = put_and_commit(&app, "paper.pdf", "application/pdf", pdf).await;
    assert_eq!(admitted["status"], "pending", "{admitted}");
    assert!(admitted["document_id"].is_string(), "{admitted}");
    assert!(admitted["task_id"].is_string(), "{admitted}");
}

async fn put_and_commit(
    app: &axum::Router,
    filename: &str,
    media_type: &str,
    payload: &[u8],
) -> serde_json::Value {
    let sha = sha256_hex(payload);
    let (_, begin) = mcp_tools_call(
        app,
        "/mcp",
        "eq_upload_begin",
        json!({
            "filename": filename,
            "media_type": media_type,
            "byte_length": payload.len(),
            "sha256": sha,
        }),
    )
    .await;
    let sc = tool_structured(&begin);
    assert_eq!(sc["ok"], true, "{sc}");
    let url = sc["url"].as_str().unwrap();
    assert_eq!(
        put_bytes(app, url, media_type, payload, None).await,
        StatusCode::NO_CONTENT
    );
    let upload_id = sc["upload_id"].as_str().unwrap();
    let (_, commit) = mcp_tools_call(
        app,
        "/mcp",
        "eq_upload_commit",
        json!({"upload_id": upload_id}),
    )
    .await;
    tool_structured(&commit).clone()
}

#[tokio::test]
async fn minio_presign_put_rejects_a_different_body() {
    if std::env::var("EDGEQUAKE_ORIGINAL_STORE_E2E")
        .ok()
        .as_deref()
        != Some("1")
    {
        eprintln!("SKIP: set EDGEQUAKE_ORIGINAL_STORE_E2E=1 for the MinIO presign gate");
        return;
    }
    let endpoint = std::env::var("EDGEQUAKE_ORIGINAL_STORE_ENDPOINT")
        .unwrap_or_else(|_| "http://127.0.0.1:19010".into());
    let settings = edgequake_storage::originals::Settings {
        mode: edgequake_storage::originals::StoreMode::Object,
        url: Some("s3://eq-originals/dev".into()),
        endpoint: Some(endpoint),
        region: "us-east-1".into(),
        access_key: Some(
            std::env::var("EDGEQUAKE_ORIGINAL_STORE_ACCESS_KEY_ID")
                .unwrap_or_else(|_| "eq-originals-app".into()),
        ),
        secret_key: Some(
            std::env::var("EDGEQUAKE_ORIGINAL_STORE_SECRET_ACCESS_KEY")
                .unwrap_or_else(|_| "eq-originals-app-dev".into()),
        ),
        path_style: Some(true),
        allow_http: true,
        ..edgequake_storage::originals::Settings::default()
    };
    let opened = edgequake_storage::originals::open_settings(&settings)
        .await
        .expect("minio client");
    let client = opened.client.expect("object client");
    let signer = client.blobs.signer().expect("s3 signer");
    let body = b"spec173 minio body";
    let sha = sha256_hex(body);
    let key = format!(
        "dev/t/00000000-0000-0000-0000-000000000002/w/00000000-0000-0000-0000-000000000003/upload/{}",
        uuid::Uuid::new_v4()
    );
    let signed = edgequake_storage::originals::presign_put(
        signer.as_ref(),
        &key,
        std::time::Duration::from_secs(120),
        "text/plain",
        &sha,
        client.blobs.checksum_header(),
    )
    .await
    .expect("presign");
    assert_eq!(signed.url.scheme(), "http");
    let http = reqwest::Client::new();
    let mut good = http.put(signed.url.as_str());
    for (name, value) in &signed.headers {
        good = good.header(name, value);
    }
    let ok = good.body(body.to_vec()).send().await.expect("put");
    assert!(ok.status().is_success(), "presigned put {}", ok.status());
    let stored = client.blobs.get_all(&key).await.expect("get");
    assert_eq!(&stored[..], body);

    let wrong = edgequake_storage::originals::presign_put(
        signer.as_ref(),
        &format!("{key}-bad"),
        std::time::Duration::from_secs(120),
        "text/plain",
        &sha,
        client.blobs.checksum_header(),
    )
    .await
    .expect("presign bad");
    let mut bad = http.put(wrong.url.as_str());
    for (name, value) in &wrong.headers {
        bad = bad.header(name, value);
    }
    let rejected = bad
        .body(b"not the signed body".to_vec())
        .send()
        .await
        .expect("bad put");
    assert!(
        !rejected.status().is_success(),
        "a different body must fail the signed checksum, got {}",
        rejected.status()
    );
    client.blobs.delete_key(&key).await.expect("delete staging");
}

#[cfg(feature = "postgres")]
#[tokio::test]
async fn postgres_session_is_tenant_scoped_and_claim_is_once() {
    let base = std::fs::read_to_string("/tmp/edgequake-db-url").unwrap_or_else(|_| {
        "postgresql://edgequake:edgequake_secret@localhost:5432/edgequake".into()
    });
    let url = common::test_db::isolated_test_url(base.trim());
    let pool = match sqlx::PgPool::connect(url.trim()).await {
        Ok(pool) => pool,
        Err(err) => {
            eprintln!("SKIP postgres session: {err}");
            return;
        }
    };
    let tenant_a = uuid::Uuid::new_v4();
    let tenant_b = uuid::Uuid::new_v4();
    let workspace_a = uuid::Uuid::new_v4();
    let workspace_b = uuid::Uuid::new_v4();
    sqlx::query("INSERT INTO tenants (tenant_id, name, slug) VALUES ($1, $2, $3), ($4, $5, $6)")
        .bind(tenant_a)
        .bind("spec173-a")
        .bind(format!(
            "s173a{}",
            &tenant_a.to_string().replace('-', "")[..8]
        ))
        .bind(tenant_b)
        .bind("spec173-b")
        .bind(format!(
            "s173b{}",
            &tenant_b.to_string().replace('-', "")[..8]
        ))
        .execute(&pool)
        .await
        .expect("tenants");
    sqlx::query(
        "INSERT INTO workspaces (workspace_id, tenant_id, name, slug) VALUES ($1, $2, 'a', 'a'), ($3, $4, 'b', 'b')",
    )
    .bind(workspace_a)
    .bind(tenant_a)
    .bind(workspace_b)
    .bind(tenant_b)
    .execute(&pool)
    .await
    .expect("workspaces");

    let sessions = edgequake_storage::mcp_upload::postgres(pool);
    let scope_a = edgequake_storage::mcp_upload::UploadScope {
        tenant_id: Some(tenant_a.to_string()),
        workspace_id: Some(workspace_a.to_string()),
        user_id: None,
    };
    let scope_b = edgequake_storage::mcp_upload::UploadScope {
        tenant_id: Some(tenant_b.to_string()),
        workspace_id: Some(workspace_b.to_string()),
        user_id: None,
    };
    let upload_id = format!("upl_{}", uuid::Uuid::new_v4());
    sessions
        .insert(edgequake_storage::mcp_upload::UploadMeta {
            access: None,
            upload_id: upload_id.clone(),
            scope: scope_a.clone(),
            filename: "a.txt".into(),
            media_type: "text/plain".into(),
            byte_length: Some(3),
            sha256: Some("ab".repeat(32)),
            staging_key: Some("dev/staging".into()),
            transfer: edgequake_storage::mcp_upload::UploadTransfer::Relay,
            bind_bearer: false,
            status: edgequake_storage::mcp_upload::UploadStatus::Open,
            received: 0,
            expires_at_unix: edgequake_storage::mcp_upload::now_unix() + 600,
            document_id: None,
            task_id: None,
            commit_status: None,
            admitting: false,
        })
        .await
        .expect("insert");
    assert!(sessions
        .get(&scope_b, &upload_id)
        .await
        .expect("get b")
        .is_none());
    assert!(sessions
        .get(&scope_a, &upload_id)
        .await
        .expect("get a")
        .is_some());

    let left = sessions.clone();
    let right = sessions.clone();
    let id_left = upload_id.clone();
    let id_right = upload_id.clone();
    let scope_left = scope_a.clone();
    let scope_right = scope_a.clone();
    let (one, two) = tokio::join!(
        left.claim(&scope_left, &id_left),
        right.claim(&scope_right, &id_right)
    );
    let ready = [one.expect("claim"), two.expect("claim")]
        .into_iter()
        .filter(|claim| matches!(claim, edgequake_storage::mcp_upload::UploadClaim::Ready(_)))
        .count();
    assert_eq!(ready, 1, "one claim wins");

    assert!(sessions.mark_expired(&upload_id).await.expect("expire"));
    let keys = sessions.reclaim_expired().await.expect("reclaim");
    assert!(keys.iter().any(|key| key == "dev/staging"), "{keys:?}");
    let after = sessions.get(&scope_a, &upload_id).await.expect("after");
    assert_eq!(
        after.expect("row").status,
        edgequake_storage::mcp_upload::UploadStatus::Expired
    );
}
