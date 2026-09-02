//! Analysis cache (LightRAG `handle_cache` + `save_to_cache` parity, Phase 4i).

use std::sync::Arc;

use edgequake_llm::traits::{ChatMessage, LLMProvider};
use edgequake_storage::kv_keys;
use edgequake_storage::traits::KVStorage;
use md5;
use serde_json::json;

use super::item_record::MultimodalItemRecord;
use super::prompts::prompt_cache_fingerprint;

/// When true, multimodal analyze reads/writes KV LLM cache entries.
pub fn analysis_cache_enabled() -> bool {
    matches!(
        std::env::var("EDGEQUAKE_MM_ANALYSIS_CACHE")
            .ok()
            .as_deref()
            .map(str::trim),
        Some("1") | Some("true") | Some("yes")
    )
}

/// LightRAG `compute_args_hash` (MD5 of concatenated args).
pub fn compute_args_hash(parts: &[&str]) -> String {
    let joined = parts.concat();
    format!("{:x}", md5::compute(joined.as_bytes()))
}

/// LightRAG flattened cache id: `{mode}:{cache_type}:{hash}`.
pub fn generate_cache_key(mode: &str, cache_type: &str, hash_value: &str) -> String {
    format!("{mode}:{cache_type}:{hash_value}")
}

/// Build cache key for an analyzed item.
pub fn analysis_cache_key(item_id: &str, modality: &str, model: &str, prompt_hash: &str) -> String {
    generate_cache_key(
        "default",
        "analysis",
        &compute_args_hash(&[item_id, modality, model, prompt_hash]),
    )
}

fn kv_storage_key(flattened: &str) -> String {
    kv_keys::llm_cache(flattened)
}

/// Try read cached LLM response text.
pub async fn get_analysis_cache(kv: &dyn KVStorage, flattened_key: &str) -> Option<String> {
    let stored = kv.get_by_id(&kv_storage_key(flattened_key)).await.ok()??;
    stored
        .get("return")
        .and_then(|v| v.as_str())
        .map(str::to_string)
}

/// Persist LLM response to analysis cache.
pub async fn save_analysis_cache(
    kv: &dyn KVStorage,
    flattened_key: &str,
    prompt: &str,
    content: &str,
) -> Result<(), String> {
    let entry = json!({
        "return": content,
        "cache_type": "analysis",
        "original_prompt": prompt,
    });
    kv.upsert(&[(kv_storage_key(flattened_key), entry)])
        .await
        .map_err(|e| format!("cache write failed: {e}"))
}

/// Append cache key to record when caching is enabled.
pub fn attach_cache_key(record: &mut MultimodalItemRecord, cache_key: &str) {
    if !analysis_cache_enabled() {
        return;
    }
    if !record.llm_cache_list.iter().any(|k| k == cache_key) {
        record.llm_cache_list.push(cache_key.to_string());
    }
}

/// Append cache key to record when caching is enabled (alias for analyzer call sites).
pub fn maybe_attach_cache_key(record: &mut MultimodalItemRecord, cache_id: Option<&str>) {
    if let Some(id) = cache_id {
        attach_cache_key(record, id);
    }
}

/// Call LLM with optional KV analysis cache + JSON retry (LightRAG analyze loop).
pub async fn chat_json_with_analysis_cache<T, F, G>(
    llm: &dyn LLMProvider,
    kv: Option<Arc<dyn KVStorage>>,
    item_id: &str,
    modality: &str,
    initial_messages: Vec<ChatMessage>,
    parse: F,
    build_repair_user: G,
) -> Result<(T, Option<String>), String>
where
    F: Fn(&str) -> Result<T, String>,
    G: Fn(&str) -> String,
{
    let fingerprint = prompt_cache_fingerprint(&initial_messages);
    let cache_id = analysis_cache_key(item_id, modality, llm.model(), &fingerprint);

    if analysis_cache_enabled() {
        if let Some(ref storage) = kv {
            if let Some(cached) = get_analysis_cache(storage.as_ref(), &cache_id).await {
                if let Ok(parsed) = parse(&cached) {
                    return Ok((parsed, Some(cache_id)));
                }
            }
        }
    }

    let (parsed, response_text) =
        chat_json_with_retry_collect(llm, initial_messages.clone(), &parse, &build_repair_user)
            .await?;

    if analysis_cache_enabled() {
        if let Some(ref storage) = kv {
            let _ = save_analysis_cache(storage.as_ref(), &cache_id, &fingerprint, &response_text)
                .await;
        }
    }

    Ok((parsed, Some(cache_id)))
}

async fn chat_json_with_retry_collect<T, F, G>(
    llm: &dyn LLMProvider,
    initial_messages: Vec<ChatMessage>,
    parse: &F,
    build_repair_user: &G,
) -> Result<(T, String), String>
where
    F: Fn(&str) -> Result<T, String>,
    G: Fn(&str) -> String,
{
    let response = edgequake_observability::with_llm_generation(
        "pdf-pass-b-figure",
        llm.model(),
        llm.name(),
        async {
            let resp = llm
                .chat(
                    &initial_messages,
                    Some(
                        &edgequake_llm::CompletionOptions::default()
                            .with_role_cache("multimodal", llm),
                    ),
                )
                .await
                .map_err(|e| format!("LLM call failed: {e}"))?;
            let llm_input = edgequake_observability::format_llm_chat_turns_for_observation(
                initial_messages.iter().map(|m| {
                    let role = match m.role {
                        edgequake_llm::traits::ChatRole::System => "System",
                        edgequake_llm::traits::ChatRole::Assistant => "Assistant",
                        edgequake_llm::traits::ChatRole::User => "User",
                        edgequake_llm::traits::ChatRole::Tool => "Tool",
                        edgequake_llm::traits::ChatRole::Function => "Function",
                    };
                    (
                        role,
                        m.content.as_str(),
                        m.images.as_ref().map(|i| i.len()).unwrap_or(0),
                    )
                }),
            );
            let rec = edgequake_observability::LlmGenerationRecord::from_response(
                Some(&llm_input),
                &resp.content,
                resp.prompt_tokens as u64,
                resp.completion_tokens as u64,
            );
            Ok::<_, String>((resp, rec))
        },
    )
    .await?;
    let text = response.content.trim().to_string();
    if text.is_empty() {
        return Err("LLM returned empty content".into());
    }

    match parse(&text) {
        Ok(value) => Ok((value, text)),
        Err(first_err) => {
            let repair_messages = vec![
                ChatMessage::system(
                    "Return ONLY a single JSON object. No markdown fences or commentary.",
                ),
                ChatMessage::user(build_repair_user(&text)),
            ];
            let repair = edgequake_observability::with_llm_generation(
                "pdf-pass-b-figure",
                llm.model(),
                llm.name(),
                async {
                    let resp = llm
                        .chat(
                            &repair_messages,
                            Some(
                                &edgequake_llm::CompletionOptions::default()
                                    .with_role_cache("multimodal", llm),
                            ),
                        )
                        .await
                        .map_err(|e| format!("LLM repair call failed: {e}"))?;
                    let llm_input = edgequake_observability::format_llm_chat_turns_for_observation(
                        repair_messages.iter().map(|m| {
                            let role = match m.role {
                                edgequake_llm::traits::ChatRole::System => "System",
                                edgequake_llm::traits::ChatRole::Assistant => "Assistant",
                                edgequake_llm::traits::ChatRole::User => "User",
                                edgequake_llm::traits::ChatRole::Tool => "Tool",
                                edgequake_llm::traits::ChatRole::Function => "Function",
                            };
                            (
                                role,
                                m.content.as_str(),
                                m.images.as_ref().map(|i| i.len()).unwrap_or(0),
                            )
                        }),
                    );
                    let rec = edgequake_observability::LlmGenerationRecord::from_response(
                        Some(&llm_input),
                        &resp.content,
                        resp.prompt_tokens as u64,
                        resp.completion_tokens as u64,
                    );
                    Ok::<_, String>((resp, rec))
                },
            )
            .await?;
            let repair_text = repair.content.trim().to_string();
            parse(&repair_text)
                .map_err(|second_err| {
                    format!("JSON parse failed after retry: {first_err}; repair: {second_err}")
                })
                .map(|v| (v, repair_text))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::item_record::{MultimodalItemRecord, MultimodalItemStatus};
    use super::*;
    use edgequake_storage::adapters::memory::MemoryKVStorage;

    #[test]
    #[serial_test::serial]
    fn cache_disabled_by_default() {
        std::env::remove_var("EDGEQUAKE_MM_ANALYSIS_CACHE");
        assert!(!analysis_cache_enabled());
    }

    #[test]
    fn cache_key_matches_lightrag_format() {
        let key = generate_cache_key("default", "analysis", "abc123");
        assert_eq!(key, "default:analysis:abc123");
    }

    #[tokio::test]
    #[serial_test::serial]
    async fn roundtrip_analysis_cache_in_kv() {
        std::env::set_var("EDGEQUAKE_MM_ANALYSIS_CACHE", "1");
        let kv: Arc<dyn KVStorage> = Arc::new(MemoryKVStorage::new("test"));
        let id = generate_cache_key("default", "analysis", "deadbeef");
        save_analysis_cache(kv.as_ref(), &id, "prompt", r#"{"name":"x"}"#)
            .await
            .unwrap();
        let hit = get_analysis_cache(kv.as_ref(), &id).await.unwrap();
        assert!(hit.contains("name"));
        std::env::remove_var("EDGEQUAKE_MM_ANALYSIS_CACHE");
    }

    #[test]
    #[serial_test::serial]
    fn attaches_key_when_enabled() {
        std::env::set_var("EDGEQUAKE_MM_ANALYSIS_CACHE", "1");
        let mut record = MultimodalItemRecord {
            item_id: "im-1".into(),
            modality: "drawing".into(),
            status: MultimodalItemStatus::Success,
            analyze_time: chrono::Utc::now(),
            name: Some("x".into()),
            item_type: Some("Photo".into()),
            description: Some("d".into()),
            equation: None,
            message: None,
            llm_cache_list: Vec::new(),
        };
        attach_cache_key(&mut record, "default:analysis:abc");
        assert_eq!(record.llm_cache_list, vec!["default:analysis:abc"]);
        std::env::remove_var("EDGEQUAKE_MM_ANALYSIS_CACHE");
    }
}
