# OODA-32: Observe

## Focus: Rust SDK — Complete API Coverage Sweep

### Observations

Audited all 135+ backend routes against Rust SDK resource methods. Found ~30 missing endpoints across 12 resource modules:

- **health.rs**: Missing ready, live, metrics
- **auth.rs**: Missing logout
- **documents.rs**: Missing delete_all, reprocess, recover_stuck, retry_chunks, failed_chunks
- **graph.rs**: Missing get_node, search_labels, popular_labels
- **entities.rs**: Missing update
- **relationships.rs**: Missing get, update
- **tasks.rs**: Missing retry
- **pipeline.rs**: Missing cancel
- **costs.rs**: Missing pricing, estimate, update_budget
- **tenants.rs**: Missing update
- **folders.rs**: Missing update
- **conversations.rs**: Missing import, update, unshare, bulk_archive, bulk_move
- **models.rs**: Missing list_llm, list_embedding, get_provider, get_model
- **workspaces.rs**: Missing get, update, delete, metrics_history, rebuild_embeddings, rebuild_knowledge_graph, reprocess_documents
