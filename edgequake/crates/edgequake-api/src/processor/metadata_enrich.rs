use async_trait::async_trait;
use edgequake_pdf::{backend::EdgeParsePdfConverter, PdfConversionConfig, PdfConverter};
use edgequake_storage::{traits::KVStorage, PdfDocumentStorage};
use edgequake_tasks::{MetadataEnrichData, Task, TaskError, TaskProcessor, TaskResult};
use reqwest::Client;
use serde::Deserialize;
use std::sync::Arc;
use tokio_util::sync::CancellationToken;
use tracing::{info, warn};

use super::enrichment_config::EnrichmentConfig;

const MAX_TEXT_CHARS: usize = 8_000;

const ENRICHMENT_PROMPT: &str = "You are a document analyst. Given the text from the first pages \
of a document, extract structured metadata. Respond ONLY with valid JSON, no markdown, \
no explanation:\n\
{\n  \"summary\": \"2-3 paragraph summary written in the document's own language\",\n  \
\"topic\": \"single short topic phrase\",\n  \"language\": \"ISO 639-1 code (e.g. en, id, fr)\",\n  \
\"keywords\": [\"up to 10 keywords\"]\n}\n\nDocument text:";

#[derive(Debug, Deserialize)]
struct EnrichmentResponse {
    summary: String,
    topic: String,
    language: String,
    keywords: Vec<String>,
}

pub struct MetadataEnrichProcessor {
    kv_storage: Arc<dyn KVStorage>,
    pdf_storage: Option<Arc<dyn PdfDocumentStorage>>,
    config: EnrichmentConfig,
    http_client: Client,
}

impl MetadataEnrichProcessor {
    pub fn new(
        kv_storage: Arc<dyn KVStorage>,
        pdf_storage: Option<Arc<dyn PdfDocumentStorage>>,
        config: EnrichmentConfig,
    ) -> Self {
        Self {
            kv_storage,
            pdf_storage,
            config,
            http_client: Client::new(),
        }
    }

    async fn load_pdf_bytes(&self, pdf_id: &uuid::Uuid) -> TaskResult<Vec<u8>> {
        let storage = self.pdf_storage.as_ref().ok_or_else(|| {
            TaskError::Processing(
                "pdf_storage not available (postgres feature required)".to_string(),
            )
        })?;

        let doc = storage
            .get_pdf(pdf_id)
            .await
            .map_err(|e| TaskError::Processing(format!("Failed to load PDF: {}", e)))?
            .ok_or_else(|| TaskError::NotFound(format!("PDF not found: {}", pdf_id)))?;

        Ok(doc.pdf_data)
    }

    async fn write_metadata(
        &self,
        document_id: &str,
        updates: serde_json::Value,
    ) -> TaskResult<()> {
        let key = format!("{}-metadata", document_id);

        let existing = self
            .kv_storage
            .get_by_id(&key)
            .await
            .ok()
            .flatten()
            .unwrap_or_else(|| serde_json::json!({}));

        let mut obj = existing.as_object().cloned().unwrap_or_default();

        if let Some(map) = updates.as_object() {
            for (k, v) in map {
                obj.insert(k.clone(), v.clone());
            }
        }

        self.kv_storage
            .upsert(&[(key, serde_json::Value::Object(obj))])
            .await
            .map_err(|e| TaskError::Processing(format!("KV write failed: {}", e)))
    }

    async fn call_vlm(&self, text: &str) -> Result<EnrichmentResponse, String> {
        let prompt = format!("{}\n\n{}", ENRICHMENT_PROMPT, text);

        let body = serde_json::json!({
            "model": self.config.vlm_model,
            "messages": [{"role": "user", "content": prompt}],
            "temperature": 0.1,
        });

        let url = format!("{}/chat/completions", self.config.vlm_base_url);

        let response = self
            .http_client
            .post(&url)
            .json(&body)
            .timeout(std::time::Duration::from_secs(120))
            .send()
            .await
            .map_err(|e| format!("VLM request failed: {}", e))?;

        let resp_json: serde_json::Value = response
            .json()
            .await
            .map_err(|e| format!("Failed to read VLM response: {}", e))?;

        let content = resp_json["choices"][0]["message"]["content"]
            .as_str()
            .ok_or_else(|| format!("Unexpected VLM response shape: {}", resp_json))?;

        // Strip optional ```json fences some models add
        let json_str = content
            .trim()
            .trim_start_matches("```json")
            .trim_start_matches("```")
            .trim_end_matches("```")
            .trim();

        serde_json::from_str::<EnrichmentResponse>(json_str)
            .map_err(|e| format!("VLM returned invalid JSON ({e}): {json_str}"))
    }
}

#[async_trait]
impl TaskProcessor for MetadataEnrichProcessor {
    async fn process(
        &self,
        task: &mut Task,
        _cancel_token: CancellationToken,
    ) -> TaskResult<serde_json::Value> {
        let data: MetadataEnrichData =
            serde_json::from_value(task.task_data.clone()).map_err(|e| {
                TaskError::InvalidPayload(format!("Invalid MetadataEnrichData: {}", e))
            })?;

        self.write_metadata(
            &data.document_id,
            serde_json::json!({"enrichment_status": "processing"}),
        )
        .await?;

        let pdf_bytes = self.load_pdf_bytes(&data.pdf_id).await?;

        let converter = EdgeParsePdfConverter::default();
        let full_text = match converter
            .convert(&pdf_bytes, &PdfConversionConfig::default())
            .await
        {
            Ok(text) => text,
            Err(e) => {
                warn!(
                    document_id = %data.document_id,
                    "EdgeParser failed, skipping enrichment: {}", e
                );
                self.write_metadata(
                    &data.document_id,
                    serde_json::json!({
                        "enrichment_status": "skipped",
                        "enrichment_error": format!("EdgeParser error: {}", e),
                    }),
                )
                .await?;
                return Ok(serde_json::json!({"enrichment_status": "skipped"}));
            }
        };

        if full_text.trim().is_empty() {
            self.write_metadata(
                &data.document_id,
                serde_json::json!({"enrichment_status": "skipped"}),
            )
            .await?;
            return Ok(serde_json::json!({"enrichment_status": "skipped"}));
        }

        let text = if full_text.len() > MAX_TEXT_CHARS {
            &full_text[..MAX_TEXT_CHARS]
        } else {
            full_text.as_str()
        };

        // One internal retry for JSON parse failures; worker pool handles broader retries
        let result = match self.call_vlm(text).await {
            Ok(r) => r,
            Err(first_err) => {
                warn!(
                    document_id = %data.document_id,
                    "VLM call failed ({}), retrying once", first_err
                );
                self.call_vlm(text).await.map_err(|e| {
                    TaskError::Processing(format!("VLM enrichment failed after retry: {}", e))
                })?
            }
        };

        let keywords: Vec<String> = result.keywords.into_iter().take(10).collect();

        self.write_metadata(
            &data.document_id,
            serde_json::json!({
                "enrichment_status": "completed",
                "enrichment_summary": result.summary,
                "enrichment_topic": result.topic,
                "enrichment_language": result.language,
                "enrichment_keywords": keywords,
                "enrichment_completed_at": chrono::Utc::now().to_rfc3339(),
                "enrichment_error": null,
            }),
        )
        .await?;

        info!(
            document_id = %data.document_id,
            topic = %result.topic,
            language = %result.language,
            "Metadata enrichment completed"
        );

        Ok(serde_json::json!({
            "enrichment_status": "completed",
            "topic": result.topic,
            "language": result.language,
        }))
    }

    async fn on_permanent_failure(&self, task: &Task, error_msg: &str) {
        if let Ok(data) = serde_json::from_value::<MetadataEnrichData>(task.task_data.clone()) {
            let _ = self
                .write_metadata(
                    &data.document_id,
                    serde_json::json!({
                        "enrichment_status": "failed",
                        "enrichment_error": error_msg,
                    }),
                )
                .await;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use edgequake_storage::MemoryKVStorage;
    use edgequake_tasks::TaskStatus;

    fn make_processor() -> MetadataEnrichProcessor {
        let kv = Arc::new(MemoryKVStorage::new("test"));
        let config = EnrichmentConfig {
            vlm_base_url: "http://localhost:1".to_string(), // unreachable — not called in these tests
            vlm_model: "test-model".to_string(),
            max_pages: 5,
            concurrent: 1,
        };
        MetadataEnrichProcessor::new(kv, None, config)
    }

    fn make_task(document_id: &str, pdf_id: uuid::Uuid) -> Task {
        let tenant_id = uuid::Uuid::new_v4();
        let workspace_id = uuid::Uuid::new_v4();
        let data = MetadataEnrichData {
            document_id: document_id.to_string(),
            pdf_id,
            tenant_id,
            workspace_id,
            max_pages: 5,
        };
        Task {
            track_id: format!("metadata_enrich-{}", uuid::Uuid::new_v4()),
            tenant_id,
            workspace_id,
            task_type: TaskType::MetadataEnrich,
            status: TaskStatus::Failed,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            started_at: None,
            completed_at: None,
            error_message: None,
            error: None,
            retry_count: 3,
            max_retries: 3,
            consecutive_timeout_failures: 0,
            circuit_breaker_tripped: false,
            task_data: serde_json::to_value(&data).unwrap(),
            metadata: None,
            progress: None,
            result: None,
        }
    }

    #[tokio::test]
    async fn test_on_permanent_failure_writes_failed_status() {
        let processor = make_processor();
        let doc_id = uuid::Uuid::new_v4().to_string();
        let task = make_task(&doc_id, uuid::Uuid::new_v4());

        processor.on_permanent_failure(&task, "retries exhausted").await;

        let kv = Arc::new(MemoryKVStorage::new("test"));
        // Verify by re-constructing the processor with a shared kv
        let kv_shared = Arc::new(MemoryKVStorage::new("verify"));
        let config = EnrichmentConfig {
            vlm_base_url: "http://localhost:1".to_string(),
            vlm_model: "test".to_string(),
            max_pages: 5,
            concurrent: 1,
        };
        let p2 = MetadataEnrichProcessor::new(Arc::clone(&kv_shared) as Arc<dyn KVStorage>, None, config);
        let doc_id2 = uuid::Uuid::new_v4().to_string();
        let task2 = make_task(&doc_id2, uuid::Uuid::new_v4());

        p2.on_permanent_failure(&task2, "test error msg").await;

        let stored = kv_shared
            .get_by_id(&format!("{}-metadata", doc_id2))
            .await
            .unwrap()
            .unwrap();
        assert_eq!(stored["enrichment_status"], "failed");
        assert!(stored["enrichment_error"]
            .as_str()
            .unwrap()
            .contains("test error msg"));
        drop(kv);
    }

    #[test]
    fn test_metadata_enrich_data_serde_default() {
        // max_pages should default to 5 when absent from JSON
        let json = serde_json::json!({
            "document_id": "doc-abc",
            "pdf_id": "00000000-0000-0000-0000-000000000001",
            "tenant_id": "00000000-0000-0000-0000-000000000002",
            "workspace_id": "00000000-0000-0000-0000-000000000003"
        });
        let data: MetadataEnrichData = serde_json::from_value(json).unwrap();
        assert_eq!(data.max_pages, 5);
        assert_eq!(data.document_id, "doc-abc");
    }

    #[test]
    fn test_strip_json_fences() {
        // Verify the fence-stripping logic works for models that wrap output in ```json
        let raw = "```json\n{\"summary\":\"s\",\"topic\":\"t\",\"language\":\"en\",\"keywords\":[]}\n```";
        let stripped = raw
            .trim()
            .trim_start_matches("```json")
            .trim_start_matches("```")
            .trim_end_matches("```")
            .trim();
        let parsed: serde_json::Value = serde_json::from_str(stripped).unwrap();
        assert_eq!(parsed["topic"], "t");
    }
}
