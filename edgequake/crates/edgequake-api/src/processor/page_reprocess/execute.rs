//! SPEC-151 — Execute page-scoped PdfProcessing.

use std::collections::BTreeMap;
use std::sync::Arc;

use edgequake_pdf::{
    build_page_asset_bundle, create_pdf_converter, replace_sections, PageAssetBuildConfig,
    PdfConversionConfig, PdfParserBackend, VisionConversionConfig,
};
use edgequake_pdf2md::PageSelection;
#[cfg(feature = "postgres")]
use edgequake_storage::{UpsertPageParse, PAGE_STAGE_FAILED, PAGE_STAGE_OK, PAGE_STAGE_RUNNING};
use edgequake_tasks::{Task, TaskError, TaskResult};
use sha2::{Digest, Sha256};
use tracing::{info, warn};
use uuid::Uuid;

use crate::processor::page_reprocess::plan::page_reprocess_actions;
use crate::processor::page_reprocess::record::record_figures_from_markdown_scoped;
use crate::processor::DocumentTaskProcessor;

impl DocumentTaskProcessor {
    /// SPEC-151: reprocess selected pages; splice markdown; enqueue Insert with reuse.
    #[cfg(feature = "postgres")]
    #[allow(clippy::too_many_arguments)]
    pub(crate) async fn process_page_scope_reprocess(
        &self,
        task: &mut Task,
        data: &edgequake_tasks::PdfProcessingData,
        pdf_bytes: &[u8],
        existing_markdown: &str,
        document_id: &str,
        filename: &str,
        page_count: Option<u32>,
        file_size_bytes: i64,
        sha256_checksum: &str,
    ) -> TaskResult<serde_json::Value> {
        let scope = data
            .page_scope
            .as_ref()
            .ok_or_else(|| TaskError::Processing("page_scope missing".into()))?;
        let stages = scope.effective_stages();
        let actions = page_reprocess_actions(&stages);
        let pages_usize: Vec<usize> = scope.pages.iter().map(|p| *p as usize).collect();
        info!(
            document_id = %document_id,
            pages = ?scope.pages,
            stages = ?stages.iter().map(|s| s.as_str()).collect::<Vec<_>>(),
            run_parse_ocr = actions.run_parse_ocr,
            run_figures_assets_only = actions.run_figures_assets_only,
            "SPEC-151: starting page-scope reprocess"
        );

        // Progress phase the UI already understands (EC-151-16).
        self.bump_task_progress(task, "page_reprocess".to_string(), 3, 15)
            .await;
        let _ =
            crate::services::patch_document_metadata(&self.kv_storage, document_id, |updated| {
                updated.insert(
                    "current_stage".to_string(),
                    serde_json::json!("page_reprocess"),
                );
                updated.insert(
                    "stage_message".to_string(),
                    serde_json::json!("Reprocessing pages"),
                );
                updated.insert("stage_progress".to_string(), serde_json::json!(0.15));
                updated.insert(
                    "updated_at".to_string(),
                    serde_json::json!(chrono::Utc::now().to_rfc3339()),
                );
            })
            .await;

        let doc_uuid = Uuid::parse_str(document_id).ok();
        let ws = data.workspace_id;
        let track_id = Some(task.track_id.clone());
        // Defense in depth: enqueue should resolve model, but never pass None into Vision.
        let resolved_vision_model = data.vision_model.clone().or_else(|| {
            Some(crate::vision_env::default_vision_model_for_provider(
                &data.vision_provider,
            ))
        });
        let page_state = self.page_state_storage.as_ref().or_else(|| {
            self.app_state
                .as_ref()
                .and_then(|s| s.storage.page_state_storage.as_ref())
        });

        // Parse RUNNING only when OCR will run (LAW-151-6 honesty).
        if actions.run_parse_ocr {
            if let (Some(doc), Some(store)) = (doc_uuid, page_state) {
                let rows: Vec<_> = scope
                    .pages
                    .iter()
                    .map(|p| UpsertPageParse {
                        page_number: *p as i32,
                        status: PAGE_STAGE_RUNNING.to_string(),
                        error: None,
                        method: Some("vision".into()),
                        model: resolved_vision_model.clone(),
                        raw_markdown: None,
                        raw_sha256: None,
                        increment_attempts: true,
                        track_id: track_id.clone(),
                    })
                    .collect();
                let _ = store.upsert_parse_batch(doc, ws, &rows).await;
            }
        }

        let mut markdown = existing_markdown.to_string();
        let mut replacements: BTreeMap<usize, String> = BTreeMap::new();

        if actions.run_parse_ocr {
            self.bump_task_progress(task, "page_reprocess".to_string(), 3, 35)
                .await;
            let converter = create_pdf_converter(PdfParserBackend::Vision);
            let sink_replacements =
                Arc::new(std::sync::Mutex::new(BTreeMap::<usize, String>::new()));
            let sink_clone = Arc::clone(&sink_replacements);
            let mut vision = VisionConversionConfig {
                provider_name: Some(data.vision_provider.clone()),
                model: resolved_vision_model.clone(),
                pages: Some(PageSelection::Set(pages_usize.clone())),
                no_resume: true,
                page_result_sink: Some(Arc::new(move |page, md| {
                    if let Ok(mut g) = sink_clone.lock() {
                        g.insert(page, format!("<!-- edgequake-page:{page} -->\n\n{md}"));
                    }
                })),
                ..Default::default()
            };
            if let Some(model) = resolved_vision_model.as_deref() {
                vision.llm =
                    crate::safety_limits::injected_vision_llm(&data.vision_provider, model)
                        .map_err(|error| {
                            edgequake_tasks::TaskError::Processing(format!(
                                "PDF vision provider {} unavailable: {error}",
                                data.vision_provider
                            ))
                        })?;
            }
            let cfg = PdfConversionConfig {
                vision: Some(vision),
                pages: Some(PageSelection::Set(pages_usize.clone())),
                ..Default::default()
            };

            match converter.convert(pdf_bytes, &cfg).await {
                Ok(part_md) => {
                    let (secs, _) = edgequake_pdf::split_sections(&part_md);
                    for s in secs {
                        replacements.insert(s.page, s.section);
                    }
                    if let Ok(g) = sink_replacements.lock() {
                        for (p, sec) in g.iter() {
                            replacements.entry(*p).or_insert_with(|| sec.clone());
                        }
                    }
                }
                Err(e) => {
                    warn!(error = %e, "SPEC-151: page-scope convert failed");
                    if let (Some(doc), Some(store)) = (doc_uuid, page_state) {
                        let rows: Vec<_> = scope
                            .pages
                            .iter()
                            .map(|p| UpsertPageParse {
                                page_number: *p as i32,
                                status: PAGE_STAGE_FAILED.to_string(),
                                error: Some(e.to_string()),
                                method: Some("vision".into()),
                                model: resolved_vision_model.clone(),
                                raw_markdown: None,
                                raw_sha256: None,
                                increment_attempts: false,
                                track_id: track_id.clone(),
                            })
                            .collect();
                        let _ = store.upsert_parse_batch(doc, ws, &rows).await;
                    }
                    return Err(TaskError::Processing(format!(
                        "page-scope convert failed: {e}"
                    )));
                }
            }

            markdown = replace_sections(&markdown, &replacements, true);

            if let (Some(doc), Some(store)) = (doc_uuid, page_state) {
                let rows: Vec<_> = replacements
                    .iter()
                    .map(|(page, sec)| {
                        let mut hasher = Sha256::new();
                        hasher.update(sec.as_bytes());
                        UpsertPageParse {
                            page_number: *page as i32,
                            status: PAGE_STAGE_OK.to_string(),
                            error: None,
                            method: Some("vision".into()),
                            model: resolved_vision_model.clone(),
                            raw_markdown: Some(sec.clone()),
                            raw_sha256: Some(format!("{:x}", hasher.finalize())),
                            increment_attempts: false,
                            track_id: track_id.clone(),
                        }
                    })
                    .collect();
                let _ = store.upsert_parse_batch(doc, ws, &rows).await;
            }
        }

        // Figures without Parse: asset writers only (REQ-151-04 / LAW-151-3).
        if actions.run_figures_assets_only {
            self.bump_task_progress(task, "page_reprocess".to_string(), 3, 50)
                .await;
            let assets_root = crate::services::document_mm_assets_root(document_id);
            if let Err(e) = tokio::fs::create_dir_all(&assets_root).await {
                warn!(
                    document_id = %document_id,
                    error = %e,
                    "SPEC-151: failed to create mm-assets root for figures-only"
                );
            }
            let bundle = build_page_asset_bundle(
                pdf_bytes,
                PageAssetBuildConfig {
                    assets_root: &assets_root,
                    pages: &pages_usize,
                    dpi: 150,
                    max_rendered_pixels: 3600,
                    write_figures: true,
                    write_page_pngs: true,
                    write_charts: true,
                    promote_fig_as_chart: true,
                    page_modality: None,
                    figure_filter_provider: None,
                },
            )
            .await;
            info!(
                document_id = %document_id,
                figure_pages = bundle.figure_map.len(),
                table_pages = bundle.table_map.len(),
                chart_crops = bundle.chart_crop_paths.len(),
                "SPEC-151: figures-only asset bundle built"
            );
            #[cfg(feature = "postgres")]
            {
                match crate::services::persist_mm_assets_with_storage(
                    self.mm_asset_storage.as_ref(),
                    self.kv_storage.as_ref(),
                    document_id,
                    data.workspace_id,
                    &assets_root,
                )
                .await
                {
                    Ok(n) => {
                        if n > 0 {
                            info!(
                                document_id = %document_id,
                                count = n,
                                "SPEC-151: persisted figures-only mm-assets"
                            );
                        }
                    }
                    Err(e) => warn!(
                        document_id = %document_id,
                        error = %e,
                        "SPEC-151: figures-only mm-asset persist failed (non-fatal)"
                    ),
                }
            }
        }

        if actions.record_figures_health {
            if let (Some(doc), Some(store)) = (doc_uuid, page_state) {
                record_figures_from_markdown_scoped(
                    store.as_ref(),
                    doc,
                    ws,
                    &markdown,
                    track_id.clone(),
                    Some(&scope.pages),
                )
                .await;
            }
        }

        if let Some(pdf_storage) = self.pdf_storage.as_ref() {
            use edgequake_storage::{PdfProcessingStatus, UpdatePdfProcessingRequest};
            let _ = pdf_storage
                .update_pdf_processing(UpdatePdfProcessingRequest {
                    pdf_id: data.pdf_id,
                    processing_status: PdfProcessingStatus::Completed,
                    markdown_content: Some(markdown.clone()),
                    extraction_method: None,
                    vision_model: resolved_vision_model.clone(),
                    extraction_errors: None,
                    document_id: Uuid::parse_str(document_id).ok(),
                })
                .await;
        }
        let content_key = format!("{document_id}-content");
        let _ = self
            .kv_storage
            .upsert(&[(content_key, serde_json::json!({ "content": markdown }))])
            .await;

        if actions.enqueue_entities {
            self.bump_task_progress(task, "page_reprocess".to_string(), 3, 70)
                .await;
            let text_data = edgequake_tasks::TextInsertData {
                text: markdown.clone(),
                file_source: filename.to_string(),
                workspace_id: data.workspace_id.to_string(),
                metadata: Some(serde_json::json!({
                    "document_id": document_id,
                    "source": "pdf_page_reprocess",
                    "source_type": "pdf",
                    "document_type": "pdf",
                    "pdf_id": data.pdf_id.to_string(),
                    "filename": filename,
                    "page_count": page_count,
                    "file_size_bytes": file_size_bytes,
                    "sha256_checksum": sha256_checksum,
                    "tenant_id": data.tenant_id.to_string(),
                    "workspace_id": data.workspace_id.to_string(),
                    "force_fresh_extraction": false,
                    "page_reprocess": true,
                })),
                reuse_excluded_pages: Some(scope.pages.clone()),
            };
            let ingest_timeout = 3600u64;
            let _ = self
                .enqueue_pdf_ingest_insert(task, data, text_data, ingest_timeout)
                .await?;
        }

        self.bump_task_progress(task, "page_reprocess".to_string(), 3, 100)
            .await;

        Ok(serde_json::json!({
            "status": "completed",
            "document_id": document_id,
            "pages": scope.pages,
            "stages": stages.iter().map(|s| s.as_str()).collect::<Vec<_>>(),
            "markdown_len": markdown.len(),
            "actions": {
                "parse_ocr": actions.run_parse_ocr,
                "figures_assets_only": actions.run_figures_assets_only,
                "entities": actions.enqueue_entities,
            },
        }))
    }
}
