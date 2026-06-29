//! v2 job type catalog — Level 4 workspace-scoped REST SSOT (SPEC-027 IMP-025).

use serde::Serialize;
use utoipa::ToSchema;

/// One entry in the v2 job catalog.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct JobCatalogEntry {
    pub job_type: String,
    pub description: String,
    pub creatable_via_v2: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub v1_equivalent: Option<String>,
    pub endpoints: Vec<String>,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct JobCatalogLinks {
    pub create: String,
    pub list: String,
    pub catalog: String,
}

/// Full catalog for `GET /api/v2/workspaces/{workspace_id}/jobs/catalog`.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct JobCatalogResponse {
    pub workspace_id: String,
    pub entries: Vec<JobCatalogEntry>,
    pub links: JobCatalogLinks,
}

/// Additive hint on v1 RPC responses pointing integrators to Level 4 v2 jobs (ascending-compat).
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct V2MigrationHint {
    pub job_type: String,
    pub catalog: String,
    pub create: String,
    pub note: String,
}

/// All job types creatable via `POST .../jobs` (SSOT — must match `job_catalog` entries).
pub const CREATABLE_V2_JOB_TYPES: &[&str] = &[
    "upload",
    "insert",
    "scan",
    "reindex",
    "pdf_processing",
    "knowledge_injection",
    "rebuild_embeddings",
    "rebuild_knowledge_graph",
    "reprocess_all",
    "reprocess_failed",
    "recover_stuck",
    "reanalyze_multimodal",
];

/// Returns true when `job_type` is a known creatable v2 job (case-insensitive).
pub fn is_creatable_v2_job_type(job_type: &str) -> bool {
    CREATABLE_V2_JOB_TYPES.contains(&job_type.to_ascii_lowercase().as_str())
}

/// Build v2 migration hint for a workspace-scoped job type.
pub fn v2_migration_hint(job_type: &str, workspace_id: &str) -> V2MigrationHint {
    let base = format!("/api/v2/workspaces/{workspace_id}/jobs");
    V2MigrationHint {
        job_type: job_type.to_string(),
        catalog: format!("{base}/catalog"),
        create: base,
        note: format!(
            "Prefer POST {{create}} with {{\"job_type\":\"{job_type}\"}} (Level 4 v2 API)"
        ),
    }
}

/// v1 OpenAPI path → v2 job_type (for OAS enrichment only).
pub const V1_RPC_V2_JOB_TYPES: &[(&str, &str)] = &[
    (
        "/api/v1/workspaces/{workspace_id}/rebuild-embeddings",
        "rebuild_embeddings",
    ),
    (
        "/api/v1/workspaces/{workspace_id}/rebuild-knowledge-graph",
        "rebuild_knowledge_graph",
    ),
    (
        "/api/v1/workspaces/{workspace_id}/reprocess-documents",
        "reprocess_all",
    ),
    ("/api/v1/documents/reprocess", "reprocess_failed"),
    ("/api/v1/documents/recover-stuck", "recover_stuck"),
    (
        "/api/v1/documents/{document_id}/reanalyze",
        "reanalyze_multimodal",
    ),
];

fn ws_base(workspace_id: &str) -> String {
    format!("/api/v2/workspaces/{workspace_id}/jobs")
}

/// Static catalog — SSOT for handlers, OpenAPI examples, and tests.
pub fn job_catalog(workspace_id: &str) -> JobCatalogResponse {
    let base = ws_base(workspace_id);
    JobCatalogResponse {
        workspace_id: workspace_id.to_string(),
        links: JobCatalogLinks {
            create: base.clone(),
            list: base.clone(),
            catalog: format!("{base}/catalog"),
        },
        entries: vec![
            job_entry(
                workspace_id,
                "upload",
                "Upload and ingest a document from inline content or URL.",
            ),
            job_entry(
                workspace_id,
                "insert",
                "Insert pre-chunked text content into the knowledge graph.",
            ),
            job_entry(workspace_id, "scan", "Scan a document source for new content."),
            job_entry(workspace_id, "reindex", "Reindex existing vectors for a document."),
            job_entry(
                workspace_id,
                "pdf_processing",
                "Extract markdown from PDF bytes and enqueue pipeline processing.",
            ),
            job_entry(
                workspace_id,
                "knowledge_injection",
                "Process a knowledge-injection payload into the workspace graph.",
            ),
            job_entry(
                workspace_id,
                "rebuild_embeddings",
                "Rebuild all workspace vector embeddings with the current model.",
            ),
            job_entry(
                workspace_id,
                "rebuild_knowledge_graph",
                "Clear and rebuild the workspace knowledge graph from documents.",
            ),
            job_entry(
                workspace_id,
                "reprocess_all",
                "Reprocess all (or failed) documents in a workspace.",
            ),
            job_entry(
                workspace_id,
                "reprocess_failed",
                "Requeue failed documents for pipeline processing.",
            ),
            job_entry(
                workspace_id,
                "recover_stuck",
                "Recover documents stuck in processing beyond a threshold.",
            ),
            job_entry(
                workspace_id,
                "reanalyze_multimodal",
                "Re-run multimodal (VLM) enrichment on a single document (requires payload.document_id).",
            ),
        ],
    }
}

fn job_entry(workspace_id: &str, job_type: &str, description: &str) -> JobCatalogEntry {
    let base = ws_base(workspace_id);
    JobCatalogEntry {
        job_type: job_type.to_string(),
        description: description.to_string(),
        creatable_via_v2: true,
        v1_equivalent: v1_hint(job_type),
        endpoints: vec![
            format!("POST {base} {{ \"job_type\": \"{job_type}\" }}"),
            format!("GET {base}/{{job_id}}"),
            format!("DELETE {base}/{{job_id}}"),
        ],
    }
}

fn v1_hint(job_type: &str) -> Option<String> {
    match job_type {
        "rebuild_embeddings" => {
            Some("POST /api/v1/workspaces/{workspace_id}/rebuild-embeddings".into())
        }
        "rebuild_knowledge_graph" => {
            Some("POST /api/v1/workspaces/{workspace_id}/rebuild-knowledge-graph".into())
        }
        "reprocess_all" => {
            Some("POST /api/v1/workspaces/{workspace_id}/reprocess-documents".into())
        }
        "reprocess_failed" => Some("POST /api/v1/documents/reprocess".into()),
        "recover_stuck" => Some("POST /api/v1/documents/recover-stuck".into()),
        "reanalyze_multimodal" => Some("POST /api/v1/documents/{document_id}/reanalyze".into()),
        _ => Some(format!("POST /api/v1/tasks (type={job_type})")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn creatable_job_types_match_catalog() {
        let ws = "cccccccc-0027-0027-0027-cccccccccccc";
        let catalog = job_catalog(ws);
        assert_eq!(catalog.entries.len(), CREATABLE_V2_JOB_TYPES.len());
        for entry in &catalog.entries {
            assert!(is_creatable_v2_job_type(&entry.job_type));
        }
    }

    #[test]
    fn catalog_is_workspace_scoped_and_all_creatable() {
        let ws = "cccccccc-0027-0027-0027-cccccccccccc";
        let catalog = job_catalog(ws);
        assert_eq!(catalog.workspace_id, ws);
        assert!(catalog.links.create.contains(ws));
        assert_eq!(catalog.entries.len(), 12);
        assert!(catalog.entries.iter().all(|e| e.creatable_via_v2));
        let rebuild = catalog
            .entries
            .iter()
            .find(|e| e.job_type == "rebuild_embeddings")
            .expect("rebuild_embeddings");
        assert!(rebuild.endpoints[0].contains(ws));
    }
}
