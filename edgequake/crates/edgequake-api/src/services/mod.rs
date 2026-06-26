//! Services module for shared business logic.
//!
//! WHY-OODA83: Extracted services follow SRP and DRY principles.
//! Consolidates repeated logic into single, testable modules.

pub mod audit;
pub mod content_hasher;
pub mod document_graph_cascade;
pub mod graph_community;
pub mod graph_materialization;
pub mod ingest_admission;
pub mod pdf_admission_registry;
pub mod pdf_workspace_dedup;
pub mod query_execution;

pub use audit::{record_audit, record_compliance_event, with_request_context};

pub use content_hasher::ContentHasher;
pub use document_graph_cascade::{
    analyze_deletion_impact_stats, cascade_remove_document_sources, find_document_edges,
    find_document_nodes, sources_for_document, CascadeStats, DocumentSourceScope,
};
pub use graph_community::detect_communities_guarded;
pub use graph_materialization::{
    admit_graph_materialization, graph_query_timeout, run_timed_graph_query,
    GraphMaterializationGuard,
};
pub use ingest_admission::{
    admit_pdf_processing_enqueue, persist_pdf_task_document_id,
    provision_queued_pdf_document_shell, resolve_pdf_ingest_document_id,
    resolve_worker_pdf_document_id, QueuedPdfDocumentShell,
};
pub use pdf_admission_registry::PdfAdmissionRegistry;
pub use pdf_workspace_dedup::{
    find_kv_document_id_for_pdf, recycle_orphan_workspace_pdf,
    workspace_has_visible_document_for_pdf,
};
pub use query_execution::{
    execute_sota_query, execute_sota_query_stream, execute_sota_query_stream_with_auth_fallback,
    execute_sota_query_with_auth_fallback, is_llm_auth_failure, llm_override_from_request,
    resolve_workspace_query_resources, validate_llm_override_pair, WorkspaceQueryResources,
};
