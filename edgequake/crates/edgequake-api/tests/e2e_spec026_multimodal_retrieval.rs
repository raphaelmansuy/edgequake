//! SPEC-026 Phase 4g — retrieval E2E: VLM content indexed and queryable.

mod common;

use common::spec026_multimodal::{
    allow_tiny_images_in_tests, parse_accepted_upload, png_upload_request, restore_vlm_image_limits,
};
use common::{count_doc_chunks, wait_for_document_processed};
use edgequake_storage::kv_keys;
use serde_json::json;
use serial_test::serial;
use std::time::Duration;
use tower::ServiceExt;

const VLM_RETRIEVAL_JSON: &str = r#"{"name":"zurich_lab_photo","type":"Photo","description":"EdgeQuake multimodal retrieval fixture: research laboratory in Zurich with distinctive phrase MM-RETRIEVAL-026."}"#;

async fn doc_chunks_contain(
    kv: &std::sync::Arc<dyn edgequake_storage::traits::KVStorage>,
    doc_id: &str,
    needle: &str,
) -> bool {
    let prefix = kv_keys::doc_chunk_prefix(doc_id);
    let Ok(keys) = kv.keys_with_prefix(&prefix).await else {
        return false;
    };
    for key in keys {
        if let Ok(Some(val)) = kv.get_by_id(&key).await {
            let content = val.get("content").and_then(|v| v.as_str()).unwrap_or("");
            if content.contains(needle) {
                return true;
            }
        }
    }
    false
}

async fn chunk_key_contains(
    kv: &std::sync::Arc<dyn edgequake_storage::traits::KVStorage>,
    chunk_key: &str,
    needle: &str,
) -> bool {
    kv.get_by_id(chunk_key)
        .await
        .ok()
        .flatten()
        .and_then(|v| {
            v.get("content")
                .and_then(|c| c.as_str())
                .map(|c| c.contains(needle))
        })
        .unwrap_or(false)
}

async fn sources_reference_vlm_content(
    parsed: &serde_json::Value,
    kv: &std::sync::Arc<dyn edgequake_storage::traits::KVStorage>,
) -> bool {
    let Some(sources) = parsed["sources"].as_array() else {
        return false;
    };
    for s in sources {
        let snippet = s["snippet"].as_str().unwrap_or("");
        let content = s["content"].as_str().unwrap_or("");
        if snippet.contains("MM-RETRIEVAL-026")
            || content.contains("MM-RETRIEVAL-026")
            || snippet.contains("zurich_lab")
        {
            return true;
        }
        if let Some(ids) = s["source_chunk_ids"].as_array() {
            for id in ids {
                if let Some(key) = id.as_str() {
                    if chunk_key_contains(kv, key, "MM-RETRIEVAL-026").await
                        || chunk_key_contains(kv, key, "[Image Name]zurich_lab_photo").await
                    {
                        return true;
                    }
                }
            }
        }
    }
    false
}

#[tokio::test]
#[serial]
async fn vlm_image_mm_chunks_indexed_and_local_query_hits_content() {
    allow_tiny_images_in_tests();
    std::env::set_var("EDGEQUAKE_MM_CHUNKS", "1");

    let workers = common::create_test_app_with_llm_responses(&[VLM_RETRIEVAL_JSON]).await;
    let app = workers.app();

    let (doc_id, track_id) = parse_accepted_upload(
        app.clone()
            .oneshot(png_upload_request("----spec026retrieve", "retrieve.png"))
            .await
            .unwrap(),
    )
    .await;

    assert_eq!(
        wait_for_document_processed(app, &track_id, Duration::from_secs(120)).await,
        "completed"
    );

    assert!(
        count_doc_chunks(&workers.kv_storage, &doc_id).await >= 1,
        "document should have at least one indexed chunk"
    );

    assert!(
        doc_chunks_contain(&workers.kv_storage, &doc_id, "MM-RETRIEVAL-026").await
            || doc_chunks_contain(&workers.kv_storage, &doc_id, "[Image Name]zurich_lab_photo")
                .await,
        "chunk KV should contain VLM body or LightRAG mm-chunk label"
    );

    let query_body = json!({
        "query": "MM-RETRIEVAL-026 Zurich laboratory",
        "mode": "local",
        "context_only": true,
        "enable_rerank": false,
    });

    let response = app
        .clone()
        .oneshot(
            axum::http::Request::builder()
                .method("POST")
                .uri("/api/v1/query")
                .header("Content-Type", "application/json")
                .body(axum::body::Body::from(query_body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), axum::http::StatusCode::OK);
    let bytes = axum::body::to_bytes(response.into_body(), 1024 * 1024)
        .await
        .unwrap();
    let parsed: serde_json::Value = serde_json::from_slice(&bytes).unwrap();

    assert!(
        sources_reference_vlm_content(&parsed, &workers.kv_storage).await,
        "local query sources should link to VLM/mm-chunk content; sources={}",
        parsed["sources"]
    );

    std::env::remove_var("EDGEQUAKE_MM_CHUNKS");
    restore_vlm_image_limits();
}
