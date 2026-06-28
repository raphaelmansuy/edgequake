//! Single entry point for the multimodal analyze stage (DRY SSOT for PDF paths).

use std::path::Path;
use std::sync::Arc;
use uuid::Uuid;

use edgequake_llm::traits::LLMProvider;
use edgequake_storage::traits::KVStorage;
use tracing::warn;

use super::super::vlm_provider_resolver::{
    resolve_extract_provider_for_workspace, resolve_vlm_provider_for_workspace,
};
use super::analyzer::analyze_multimodal_images;
use super::manifest_store::{metadata_multimodal_patch, persist_manifest};
use super::providers::MultimodalProviders;
use crate::state::SharedWorkspaceService;

/// Run inline-image VLM enrichment on markdown (convert path, resume path, reprocess).
#[allow(clippy::too_many_arguments)]
pub async fn run_multimodal_analyze_stage(
    markdown: String,
    process_options: Option<&str>,
    filename: &str,
    workspace_service: Option<&SharedWorkspaceService>,
    workspace_id: Uuid,
    fallback_llm: Arc<dyn LLMProvider>,
    asset_base_dir: Option<&Path>,
    document_id: Option<&str>,
    kv_storage: Option<Arc<dyn KVStorage>>,
) -> String {
    run_multimodal_analyze_stage_outcome(
        markdown,
        process_options,
        filename,
        workspace_service,
        workspace_id,
        fallback_llm,
        asset_base_dir,
        document_id,
        kv_storage,
    )
    .await
    .markdown
}

/// Same as [`run_multimodal_analyze_stage`] but returns full [`AnalyzeOutcome`].
#[allow(clippy::too_many_arguments)]
pub async fn run_multimodal_analyze_stage_outcome(
    markdown: String,
    process_options: Option<&str>,
    filename: &str,
    workspace_service: Option<&SharedWorkspaceService>,
    workspace_id: Uuid,
    fallback_llm: Arc<dyn LLMProvider>,
    asset_base_dir: Option<&Path>,
    document_id: Option<&str>,
    kv_storage: Option<Arc<dyn KVStorage>>,
) -> super::analyzer::AnalyzeOutcome {
    let vlm = resolve_vlm_provider_for_workspace(
        workspace_service,
        workspace_id,
        None,
        Arc::clone(&fallback_llm),
    )
    .await;
    let extract =
        resolve_extract_provider_for_workspace(workspace_service, workspace_id, fallback_llm).await;

    let outcome = analyze_multimodal_images(
        &markdown,
        process_options,
        filename,
        MultimodalProviders::split(vlm.as_ref(), extract.as_ref()),
        asset_base_dir,
        kv_storage.clone(),
    )
    .await;

    if let Some(err) = &outcome.hard_error {
        warn!(error = %err, "multimodal analyze stage hard error (strict mode)");
    }

    if let (Some(doc_id), Some(kv)) = (document_id, kv_storage.as_ref()) {
        if let Err(e) = persist_manifest(kv.as_ref(), doc_id, &outcome.manifest).await {
            warn!(document_id = %doc_id, error = %e, "failed to persist multimodal manifest");
        } else {
            let total = outcome.summary.success + outcome.summary.skipped + outcome.summary.failed;
            if total > 0 {
                if let Ok(Some(existing)) = kv
                    .get_by_id(&edgequake_storage::kv_keys::doc_metadata(doc_id))
                    .await
                {
                    if let Some(obj) = existing.as_object().cloned() {
                        let mut merged = obj;
                        if let Some(patch) =
                            metadata_multimodal_patch(&outcome.summary, process_options).as_object()
                        {
                            for (k, v) in patch {
                                merged.insert(k.clone(), v.clone());
                            }
                        }
                        let meta_key = edgequake_storage::kv_keys::doc_metadata(doc_id);
                        let payload = serde_json::Value::Object(merged);
                        let _ = crate::services::upsert_metadata_kv_with_index(
                            kv.as_ref(),
                            &meta_key,
                            payload,
                        )
                        .await;
                    }
                }
            }
        }
    }

    outcome
}
