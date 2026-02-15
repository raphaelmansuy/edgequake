//! E2E tests against a live EdgeQuake backend.
//!
//! Run with: `cargo test --test e2e_tests --features e2e`
//! Requires a running backend at EDGEQUAKE_BASE_URL (default: http://localhost:8080).
//!
//! These tests validate that SDK types and paths match the real API.

#![cfg(feature = "e2e")]

use edgequake_sdk::*;

fn e2e_client() -> EdgeQuakeClient {
    let base_url =
        std::env::var("EDGEQUAKE_BASE_URL").unwrap_or_else(|_| "http://localhost:8080".into());
    let mut builder = EdgeQuakeClient::builder().base_url(&base_url);

    if let Ok(key) = std::env::var("EDGEQUAKE_API_KEY") {
        builder = builder.api_key(&key);
    }
    // WHY: Default migration-created tenant/user always available for E2E
    let tid = std::env::var("EDGEQUAKE_TENANT_ID")
        .unwrap_or_else(|_| "00000000-0000-0000-0000-000000000002".into());
    builder = builder.tenant_id(&tid);

    let uid = std::env::var("EDGEQUAKE_USER_ID")
        .unwrap_or_else(|_| "00000000-0000-0000-0000-000000000001".into());
    builder = builder.user_id(&uid);

    if let Ok(wid) = std::env::var("EDGEQUAKE_WORKSPACE_ID") {
        builder = builder.workspace_id(&wid);
    }

    builder.max_retries(0).build().expect("failed to build client")
}

// ── Health ───────────────────────────────────────────────────────

#[tokio::test]
async fn e2e_health() {
    let c = e2e_client();
    let h = c.health().check().await.unwrap();
    assert_eq!(h.status, "healthy");
    println!("Health: status={} version={:?} storage={:?}", h.status, h.version, h.storage_mode);
}

// ── Documents ────────────────────────────────────────────────────

#[tokio::test]
async fn e2e_documents_list() {
    let c = e2e_client();
    let docs = c.documents().list().await.unwrap();
    println!("Documents: {} items", docs.documents.len());
}

// ── Graph ────────────────────────────────────────────────────────

#[tokio::test]
async fn e2e_graph_get() {
    let c = e2e_client();
    let g = c.graph().get().await.unwrap();
    println!("Graph: {} nodes, {} edges", g.nodes.len(), g.edges.len());
}

#[tokio::test]
async fn e2e_graph_search() {
    let c = e2e_client();
    let r = c.graph().search("test").await.unwrap();
    println!("Search: {} nodes found", r.nodes.len());
}

// ── Entities ─────────────────────────────────────────────────────

#[tokio::test]
async fn e2e_entities_list_and_create() {
    let c = e2e_client();

    // Cleanup any leftover entity from previous runs.
    let _ = c.entities().delete("E2E_RUST_TEST").await;

    let list = c.entities().list().await.unwrap();
    println!("Entities: {} total (page {})", list.total, list.page);

    // Create
    let req = types::graph::CreateEntityRequest {
        entity_name: "E2E_RUST_TEST".into(),
        entity_type: "TEST".into(),
        description: "Created by Rust SDK E2E test".into(),
        source_id: "manual_entry".into(),
        metadata: None,
    };
    let created = c.entities().create(&req).await.unwrap();
    assert_eq!(created.status, "success");
    println!("Created: {:?} status={}", created.entity.as_ref().map(|e| &e.entity_name), created.status);

    // Exists
    let exists = c.entities().exists("E2E_RUST_TEST").await.unwrap();
    assert!(exists.exists, "entity should exist after creation");

    // Get (detail)
    let detail = c.entities().get("E2E_RUST_TEST").await.unwrap();
    assert_eq!(detail.entity.entity_name, "E2E_RUST_TEST");
    println!("Detail: entity_name={} degree={:?}", detail.entity.entity_name, detail.entity.degree);

    // Delete
    let del_result = c.entities().delete("E2E_RUST_TEST").await;
    if let Err(e) = &del_result {
        println!("Warning: delete failed: {e}");
    }
}

// ── Relationships ────────────────────────────────────────────────

#[tokio::test]
async fn e2e_relationships_list() {
    let c = e2e_client();
    let r = c.relationships().list().await.unwrap();
    println!("Relationships: {} total (page {})", r.total, r.page);
}

// ── Query ────────────────────────────────────────────────────────

#[tokio::test]
async fn e2e_query_execute() {
    let c = e2e_client();
    let req = types::query::QueryRequest {
        query: "What is EdgeQuake?".into(),
        mode: None,
        top_k: Some(3),
        stream: None,
        only_need_context: None,
    };
    let r = c.query().execute(&req).await.unwrap();
    println!("Query: answer_len={} sources={}", r.answer.as_deref().unwrap_or("").len(), r.sources.len());
}

// ── Chat ─────────────────────────────────────────────────────────

#[tokio::test]
async fn e2e_chat_completions() {
    let c = e2e_client();
    // WHY: EdgeQuake chat API uses `message` (singular string), not messages array
    let req = types::chat::ChatCompletionRequest {
        message: "Say hello".into(),
        stream: Some(false),
        mode: None,
        conversation_id: None,
        max_tokens: None,
        temperature: None,
        top_k: None,
        parent_id: None,
        provider: None,
        model: None,
    };
    match c.chat().completions(&req).await {
        Ok(r) => println!("Chat: content_len={} conversation_id={:?}",
            r.content.as_deref().unwrap_or("").len(),
            r.conversation_id),
        Err(e) => println!("Chat: error (may need LLM configured): {e}"),
    }
}

// ── Tasks ────────────────────────────────────────────────────────

#[tokio::test]
async fn e2e_tasks_list() {
    let c = e2e_client();
    let r = c.tasks().list().await.unwrap();
    println!("Tasks: {} total", r.total);
}

// ── Pipeline ─────────────────────────────────────────────────────

#[tokio::test]
async fn e2e_pipeline_status() {
    let c = e2e_client();
    let r = c.pipeline().status().await.unwrap();
    println!("Pipeline: is_busy={} pending={} processing={}", r.is_busy, r.pending_tasks, r.processing_tasks);
}

#[tokio::test]
async fn e2e_pipeline_metrics() {
    let c = e2e_client();
    let r = c.pipeline().metrics().await.unwrap();
    println!("Queue metrics: depth={} processing={}", r.queue_depth, r.processing);
}

// ── Costs ────────────────────────────────────────────────────────

#[tokio::test]
async fn e2e_costs_summary() {
    let c = e2e_client();
    let r = c.costs().summary().await.unwrap();
    println!("Costs: total=${:.4} tokens={}", r.total_cost_usd, r.total_tokens);
}

#[tokio::test]
async fn e2e_costs_budget() {
    let c = e2e_client();
    let r = c.costs().budget().await.unwrap();
    println!("Budget: current_spend=${:.4}", r.current_spend_usd);
}

// ── Models ───────────────────────────────────────────────────────

#[tokio::test]
async fn e2e_models_list() {
    let c = e2e_client();
    let catalog = c.models().list().await.unwrap();
    println!("Models: {} providers", catalog.providers.len());
    for p in &catalog.providers {
        println!("  - {} ({:?}): {} models", p.name, p.display_name, p.models.len());
    }
}

#[tokio::test]
async fn e2e_models_providers_health() {
    let c = e2e_client();
    let health = c.models().providers_health().await.unwrap();
    println!("Provider health: {} providers", health.len());
}

// ── Tenants ──────────────────────────────────────────────────────

#[tokio::test]
async fn e2e_tenants_list() {
    let c = e2e_client();
    let resp = c.tenants().list().await.unwrap();
    println!("Tenants: {} items", resp.items.len());
    for t in &resp.items {
        println!("  - {} (slug={:?} plan={:?})", t.name, t.slug, t.plan);
    }
}

// ── Lineage ──────────────────────────────────────────────────────

#[tokio::test]
async fn e2e_lineage_for_entity() {
    let c = e2e_client();
    // Expected to 404 for non-existent entity
    match c.provenance().for_entity("NONEXISTENT_ENTITY").await {
        Ok(r) => println!("Provenance: {} records", r.len()),
        Err(e) => println!("Lineage: expected error for nonexistent entity: {e}"),
    }
}

// ── Document Lineage (OODA-21) ───────────────────────────────────

#[tokio::test]
async fn e2e_document_lineage() {
    let c = e2e_client();
    let docs = c.documents().list().await.unwrap();
    if let Some(doc) = docs.documents.first() {
        match c.documents().get_lineage(&doc.id).await {
            Ok(lineage) => {
                println!(
                    "Document lineage: doc={} metadata={:?} lineage={:?}",
                    lineage.document_id,
                    lineage.metadata.is_some(),
                    lineage.lineage.is_some()
                );
                assert_eq!(lineage.document_id, doc.id);
            }
            Err(e) => println!("Document lineage: error (may not have lineage data): {e}"),
        }
    } else {
        println!("Document lineage: skipped (no documents)");
    }
}

#[tokio::test]
async fn e2e_document_metadata() {
    let c = e2e_client();
    let docs = c.documents().list().await.unwrap();
    if let Some(doc) = docs.documents.first() {
        match c.documents().get_metadata(&doc.id).await {
            Ok(metadata) => {
                println!("Document metadata: {} keys", metadata.as_object().map(|o| o.len()).unwrap_or(0));
                // Metadata should be an object with at least an id
                assert!(metadata.is_object(), "metadata should be a JSON object");
            }
            Err(e) => println!("Document metadata: error: {e}"),
        }
    } else {
        println!("Document metadata: skipped (no documents)");
    }
}

#[tokio::test]
async fn e2e_chunk_lineage() {
    let c = e2e_client();
    let docs = c.documents().list().await.unwrap();
    if let Some(doc) = docs.documents.first() {
        // Try chunk-0 for this document
        let chunk_id = format!("{}-chunk-0", doc.id);
        match c.chunks().get_lineage(&chunk_id).await {
            Ok(lineage) => {
                println!(
                    "Chunk lineage: chunk={} doc={:?} entities={}",
                    lineage.chunk_id, lineage.document_id,
                    lineage.entity_names.len()
                );
                assert_eq!(lineage.document_id, Some(doc.id.clone()));
            }
            Err(e) => println!("Chunk lineage: error (chunk may not exist): {e}"),
        }
    } else {
        println!("Chunk lineage: skipped (no documents)");
    }
}
