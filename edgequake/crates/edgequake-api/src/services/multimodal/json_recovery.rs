//! VLM JSON extraction + one conformance retry (LightRAG `_json_extract` subset).

use edgequake_llm::traits::{ChatMessage, LLMProvider};

#[allow(dead_code)] // repair path wired in Phase 4n strict retry parity
const JSON_REPAIR_SYSTEM: &str = "\
The previous response was not valid JSON. Return ONLY a single JSON object with keys \
\"name\", \"type\", and \"description\". No markdown fences or commentary.";

/// Extract the outermost `{…}` object from model text (handles fenced blocks).
pub fn extract_json_object(text: &str) -> Option<&str> {
    let trimmed = text.trim();
    if trimmed.starts_with("```") {
        let without_fence = trimmed.trim_start_matches('`');
        let json_body = without_fence
            .strip_prefix("json")
            .or_else(|| without_fence.strip_prefix("JSON"))
            .unwrap_or(without_fence)
            .trim_start_matches('`')
            .trim();
        if let Some(end) = json_body.rfind("```") {
            return extract_json_object(json_body[..end].trim());
        }
        return extract_json_object(json_body);
    }
    let start = trimmed.find('{')?;
    let end = trimmed.rfind('}')?;
    if end >= start {
        Some(&trimmed[start..=end])
    } else {
        None
    }
}

/// Parse JSON string; returns human-readable error.
pub fn parse_json_object<T: serde::de::DeserializeOwned>(text: &str) -> Result<T, String> {
    let json_str =
        extract_json_object(text).ok_or_else(|| "no JSON object in response".to_string())?;
    serde_json::from_str(json_str).map_err(|e| format!("invalid JSON: {e}"))
}

/// Call VLM once; on parse failure retry once with repair prompt.
#[allow(dead_code)] // repair path wired in Phase 4n strict retry parity
pub async fn chat_json_with_retry<T, F, G>(
    llm: &dyn LLMProvider,
    initial_messages: Vec<ChatMessage>,
    parse: F,
    build_repair_user: G,
) -> Result<T, String>
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
                .map_err(|e| format!("VLM call failed: {e}"))?;
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
    let text = response.content.trim();
    if text.is_empty() {
        return Err("VLM returned empty content".into());
    }

    match parse(text) {
        Ok(value) => Ok(value),
        Err(first_err) => {
            let repair_messages = vec![
                ChatMessage::system(JSON_REPAIR_SYSTEM),
                ChatMessage::user(build_repair_user(text)),
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
                        .map_err(|e| format!("VLM repair call failed: {e}"))?;
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
            parse(repair.content.trim()).map_err(|second_err| {
                format!("JSON parse failed after retry: {first_err}; repair: {second_err}")
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_fenced_json() {
        let raw = r#"Here is the result:
```json
{"name":"x","type":"Chart","description":"y"}
```"#;
        let json = extract_json_object(raw).unwrap();
        assert!(json.contains("\"name\":\"x\""));
    }

    #[test]
    fn rejects_plain_text_without_object() {
        assert!(extract_json_object("no json here").is_none());
    }
}
