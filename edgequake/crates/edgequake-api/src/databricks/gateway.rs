//! Databricks AI Gateway (`/ai-gateway/mlflow/v1`) request and response mapping.
//!
//! Unity Gateway speaks OpenAI Chat Completions and Embeddings, with two
//! differences that break a stock OpenAI client:
//!
//! * The token budget field is `max_tokens`. `max_completion_tokens` is
//!   rejected with HTTP 400 and a Databricks error object
//!   (`{"error_code","message"}`), not an OpenAI `{"error":{...}}` envelope.
//!   That 400 is 101 bytes for this unknown field and serde then reports
//!   `missing field 'error'`.
//! * Model service names are Unity Catalog ids (`system.ai.*`). The legacy
//!   pay-per-token names (`databricks-*`) are refused on the gateway.

use edgequake_llm::{
    temperature_for_wire, ChatMessage, ChatRole, CompletionOptions, FunctionCall, ImageData,
    LlmError, Result as LlmResult, ToolCall,
};
use serde_json::{json, Value};

/// OpenAI-compatible root under the workspace host. No trailing slash.
pub fn ai_gateway_v1_base(host: &str) -> String {
    format!("{}/ai-gateway/mlflow/v1", host.trim_end_matches('/'))
}

/// Unity Catalog model service name for AI Gateway.
///
/// `system.ai.llama-4-maverick` stays as-is. Legacy `databricks-llama-4-maverick`
/// becomes `system.ai.llama-4-maverick`. Any other name is kept so a custom
/// endpoint is not rewritten.
pub fn model_service_name(model: &str) -> String {
    let model = model.trim();
    if let Some(rest) = model.strip_prefix("databricks-") {
        if !rest.is_empty() && !rest.contains('.') {
            return format!("system.ai.{rest}");
        }
    }
    model.to_string()
}

/// Chat Completions body. Never emits `max_completion_tokens`.
pub fn chat_request(
    model: &str,
    messages: &[ChatMessage],
    options: &CompletionOptions,
) -> LlmResult<Value> {
    let mut wire_messages = messages
        .iter()
        .map(message_json)
        .collect::<LlmResult<Vec<_>>>()?;
    if wants_json_object(options.response_format.as_deref()) {
        ensure_json_keyword(&mut wire_messages);
    }
    let mut body = json!({
        "model": model_service_name(model),
        "messages": wire_messages,
    });
    if let Some(max_tokens) = options.max_tokens {
        body["max_tokens"] = json!(max_tokens);
    }
    if let Some(temperature) = temperature_for_wire(options.temperature) {
        body["temperature"] = json!(temperature);
    }
    if let Some(top_p) = options.top_p {
        body["top_p"] = json!(top_p);
    }
    if let Some(stop) = &options.stop {
        body["stop"] = json!(stop);
    }
    if let Some(penalty) = options.frequency_penalty {
        body["frequency_penalty"] = json!(penalty);
    }
    if let Some(penalty) = options.presence_penalty {
        body["presence_penalty"] = json!(penalty);
    }
    if let Some(effort) = edgequake_llm::reasoning_effort_for_wire(options.reasoning_effort.clone())
    {
        body["reasoning_effort"] = json!(effort);
    }
    if wants_json_object(options.response_format.as_deref()) {
        body["response_format"] = json!({"type": "json_object"});
    }
    Ok(body)
}

/// Turn a non-success body into an [`LlmError`] the query UI can show.
///
/// Databricks errors are `{error_code, message}`. OpenAI errors stay
/// `{error: {message}}`. Neither path asks serde for a required `error` field.
pub fn error_from_body(status: u16, body: &str) -> LlmError {
    let trimmed = body.trim();
    if let Ok(value) = serde_json::from_str::<Value>(trimmed) {
        if let Some(message) = value.get("message").and_then(Value::as_str) {
            let code = value
                .get("error_code")
                .and_then(Value::as_str)
                .unwrap_or("error");
            return classify(status, format!("{code}: {message}"));
        }
        if let Some(message) = value.pointer("/error/message").and_then(Value::as_str) {
            let code = value
                .pointer("/error/code")
                .and_then(Value::as_str)
                .unwrap_or("error");
            return classify(status, format!("{code}: {message}"));
        }
    }
    let preview: String = trimmed.chars().take(400).collect();
    classify(status, format!("HTTP {status}: {preview}"))
}

/// Parse a non-streaming chat completion.
pub fn parse_chat_response(body: &str) -> LlmResult<edgequake_llm::LLMResponse> {
    let value: Value = serde_json::from_str(body).map_err(|error| {
        LlmError::InvalidRequest(format!(
            "Failed to parse AI Gateway chat response: {error}; body: {}",
            preview(body)
        ))
    })?;
    if value.get("error_code").is_some() || value.get("error").is_some() {
        return Err(error_from_body(200, body));
    }
    let choice = value
        .pointer("/choices/0")
        .ok_or_else(|| LlmError::ApiError("No choices in response".into()))?;
    let finish_reason = choice
        .get("finish_reason")
        .and_then(Value::as_str)
        .map(str::to_string);
    if finish_reason.as_deref() == Some("content_filter") {
        return Err(LlmError::ApiError(
            "Response blocked by AI Gateway content filter (finish_reason=content_filter)".into(),
        ));
    }
    let message = choice.get("message").cloned().unwrap_or(Value::Null);
    let content = message
        .get("content")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();
    let refusal = message
        .get("refusal")
        .and_then(Value::as_str)
        .map(str::to_string);
    let tool_calls = parse_tool_calls(message.get("tool_calls"));
    let usage = value.get("usage");
    let prompt_tokens = usage_usize(usage, "prompt_tokens");
    let completion_tokens = usage_usize(usage, "completion_tokens");
    let total_tokens = usage
        .and_then(|usage| usage.get("total_tokens"))
        .and_then(Value::as_u64)
        .map(|n| n as usize)
        .unwrap_or(prompt_tokens + completion_tokens);
    let cache_hit_tokens = usage
        .and_then(|usage| usage.get("cache_read_input_tokens"))
        .and_then(Value::as_u64)
        .map(|n| n as usize)
        .or_else(|| {
            usage
                .and_then(|usage| usage.pointer("/prompt_tokens_details/cached_tokens"))
                .and_then(Value::as_u64)
                .map(|n| n as usize)
        });
    Ok(edgequake_llm::LLMResponse {
        content,
        refusal,
        prompt_tokens,
        completion_tokens,
        total_tokens,
        model: value
            .get("model")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string(),
        finish_reason,
        tool_calls,
        metadata: std::collections::HashMap::new(),
        cache_hit_tokens,
        cache_write_tokens: None,
        thinking_tokens: None,
        thinking_content: None,
    })
}

/// Content deltas from one AI Gateway SSE payload (already split into lines).
pub fn content_deltas_from_sse(chunk: &str) -> LlmResult<Vec<String>> {
    let mut deltas = Vec::new();
    for line in chunk.split('\n') {
        let line = line.trim().trim_end_matches('\r');
        if line.is_empty() || line.starts_with(':') {
            continue;
        }
        let Some(data) = line.strip_prefix("data:") else {
            continue;
        };
        let data = data.trim();
        if data == "[DONE]" {
            break;
        }
        let value: Value = serde_json::from_str(data).map_err(|error| {
            LlmError::InvalidRequest(format!(
                "Failed to parse AI Gateway stream event: {error}; event: {}",
                preview(data)
            ))
        })?;
        if value.get("error_code").is_some() || value.get("error").is_some() {
            return Err(error_from_body(400, data));
        }
        let finish = value
            .pointer("/choices/0/finish_reason")
            .and_then(Value::as_str);
        if finish == Some("content_filter") {
            return Err(LlmError::ApiError(
                "Response blocked by AI Gateway content filter (finish_reason=content_filter)"
                    .into(),
            ));
        }
        if let Some(text) = value
            .pointer("/choices/0/delta/content")
            .and_then(Value::as_str)
        {
            if !text.is_empty() {
                deltas.push(text.to_string());
            }
        }
    }
    Ok(deltas)
}

/// Output-token ceiling from a Databricks rejection.
///
/// `max_tokens (16384) cannot exceed 8192` → `Some(8192)`.
pub fn output_token_cap(body: &str) -> Option<usize> {
    let start = body.find("max_tokens")?;
    let rest = body[start..].find("cannot exceed ")?;
    let digits: String = body[start + rest + "cannot exceed ".len()..]
        .chars()
        .take_while(|c| c.is_ascii_digit())
        .collect();
    let cap = digits.parse::<usize>().ok()?;
    (cap > 0).then_some(cap)
}

fn wants_json_object(format: Option<&str>) -> bool {
    matches!(format, Some("json_object" | "json" | "json_schema"))
}

/// AI Gateway rejects `response_format=json_object` unless some message
/// contains the word `json`.
fn ensure_json_keyword(messages: &mut Vec<Value>) {
    if messages_contain_json(messages) {
        return;
    }
    messages.push(json!({
        "role": "system",
        "content": "Respond with json."
    }));
}

fn messages_contain_json(messages: &[Value]) -> bool {
    messages.iter().any(value_contains_json)
}

fn value_contains_json(value: &Value) -> bool {
    match value {
        Value::String(text) => text.to_ascii_lowercase().contains("json"),
        Value::Array(items) => items.iter().any(value_contains_json),
        Value::Object(map) => map.values().any(value_contains_json),
        _ => false,
    }
}

fn message_json(message: &ChatMessage) -> LlmResult<Value> {
    match message.role {
        ChatRole::System => Ok(json!({"role": "system", "content": message.content})),
        ChatRole::User => Ok(user_message(message)),
        ChatRole::Assistant => Ok(assistant_message(message)),
        ChatRole::Tool => {
            let tool_call_id = message.tool_call_id.clone().ok_or_else(|| {
                LlmError::InvalidRequest("Tool message missing required tool_call_id".into())
            })?;
            Ok(json!({
                "role": "tool",
                "content": message.content,
                "tool_call_id": tool_call_id,
            }))
        }
        ChatRole::Function => Ok(json!({"role": "user", "content": message.content})),
    }
}

fn user_message(message: &ChatMessage) -> Value {
    if !message.has_images() {
        return json!({"role": "user", "content": message.content});
    }
    let mut parts = Vec::new();
    if !message.content.is_empty() {
        parts.push(json!({"type": "text", "text": message.content}));
    }
    if let Some(images) = &message.images {
        for image in images {
            parts.push(image_part(image));
        }
    }
    json!({"role": "user", "content": parts})
}

fn image_part(image: &ImageData) -> Value {
    let mut image_url = json!({"url": image.to_api_url()});
    if let Some(detail) = &image.detail {
        image_url["detail"] = json!(detail);
    }
    json!({"type": "image_url", "image_url": image_url})
}

fn assistant_message(message: &ChatMessage) -> Value {
    let mut value = json!({"role": "assistant"});
    if !message.content.is_empty() || message.tool_calls.is_none() {
        value["content"] = json!(message.content);
    }
    if let Some(calls) = &message.tool_calls {
        value["tool_calls"] = json!(calls
            .iter()
            .map(|call| {
                json!({
                    "id": call.id,
                    "type": "function",
                    "function": {
                        "name": call.function.name,
                        "arguments": call.function.arguments,
                    }
                })
            })
            .collect::<Vec<_>>());
    }
    value
}

fn parse_tool_calls(value: Option<&Value>) -> Vec<ToolCall> {
    let Some(Value::Array(calls)) = value else {
        return Vec::new();
    };
    calls
        .iter()
        .filter_map(|call| {
            let id = call.get("id")?.as_str()?.to_string();
            let name = call.pointer("/function/name")?.as_str()?.to_string();
            let arguments = call
                .pointer("/function/arguments")
                .and_then(Value::as_str)
                .unwrap_or("{}")
                .to_string();
            Some(ToolCall {
                id,
                call_type: "function".into(),
                function: FunctionCall { name, arguments },
                thought_signature: None,
            })
        })
        .collect()
}

fn usage_usize(usage: Option<&Value>, field: &str) -> usize {
    usage
        .and_then(|usage| usage.get(field))
        .and_then(Value::as_u64)
        .unwrap_or(0) as usize
}

fn classify(status: u16, message: String) -> LlmError {
    match status {
        401 => LlmError::AuthError(message),
        429 => LlmError::RateLimited(message),
        _ => LlmError::ApiError(message),
    }
}

fn preview(body: &str) -> String {
    body.chars().take(180).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    const MAX_COMPLETION_REJECTION: &str = "{\"error_code\":\"BAD_REQUEST\",\"message\":\"Bad request: json: unknown field \\\"max_completion_tokens\\\"\\n\"}";

    #[test]
    fn unity_catalog_names_stay_and_legacy_paygo_names_move() {
        assert_eq!(
            model_service_name("system.ai.llama-4-maverick"),
            "system.ai.llama-4-maverick"
        );
        assert_eq!(
            model_service_name("  system.ai.gte-large-en  "),
            "system.ai.gte-large-en"
        );
        assert_eq!(
            model_service_name("databricks-llama-4-maverick"),
            "system.ai.llama-4-maverick"
        );
        assert_eq!(
            model_service_name("databricks-gte-large-en"),
            "system.ai.gte-large-en"
        );
        assert_eq!(model_service_name("my-endpoint"), "my-endpoint");
        assert_eq!(model_service_name("system.ai."), "system.ai.");
    }

    #[test]
    fn gateway_base_has_no_double_slash() {
        assert_eq!(
            ai_gateway_v1_base("https://example.cloud.databricks.com/"),
            "https://example.cloud.databricks.com/ai-gateway/mlflow/v1"
        );
    }

    #[test]
    fn chat_body_uses_max_tokens_not_max_completion_tokens() {
        let options = CompletionOptions {
            max_tokens: Some(16384),
            temperature: Some(0.0),
            ..CompletionOptions::default()
        };
        let body = chat_request(
            "system.ai.llama-4-maverick",
            &[ChatMessage::user("What are the main entities?")],
            &options,
        )
        .unwrap();
        assert_eq!(body["max_tokens"], 16384);
        assert!(body.get("max_completion_tokens").is_none());
        assert_eq!(body["model"], "system.ai.llama-4-maverick");
        assert_eq!(body["temperature"], 0.0);
        let encoded = body.to_string();
        assert!(!encoded.contains("max_completion_tokens"));
        assert!(!encoded.contains("prompt_cache_key"));
    }

    #[test]
    fn output_cap_is_the_limit_the_gateway_names() {
        let message = "BAD_REQUEST: max_tokens (16384) cannot exceed 8192. Please reduce the length of max output tokens generated.\n";
        assert_eq!(output_token_cap(message), Some(8192));
        assert_eq!(output_token_cap("unrelated"), None);
    }

    #[test]
    fn databricks_error_object_is_not_a_missing_error_field() {
        let error = error_from_body(400, MAX_COMPLETION_REJECTION);
        let text = error.to_string();
        assert!(text.contains("BAD_REQUEST"), "{text}");
        assert!(text.contains("max_completion_tokens"), "{text}");
        assert!(!text.contains("missing field"), "{text}");
        assert_eq!(MAX_COMPLETION_REJECTION.len(), 101);
    }

    #[test]
    fn json_object_format_mentions_json_when_the_prompt_does_not() {
        let options = CompletionOptions {
            response_format: Some("json_object".into()),
            ..CompletionOptions::default()
        };
        let body = chat_request(
            "system.ai.llama-4-maverick",
            &[ChatMessage::user("list entities")],
            &options,
        )
        .unwrap();
        assert_eq!(body["response_format"]["type"], "json_object");
        let rendered = body["messages"].to_string().to_ascii_lowercase();
        assert!(rendered.contains("json"), "{rendered}");
    }

    #[test]
    fn vision_user_message_is_an_image_url_part() {
        let message = ChatMessage::user_with_images(
            "describe",
            vec![ImageData::new("aaaa", "image/png").with_detail("low")],
        );
        let body = chat_request(
            "system.ai.llama-4-maverick",
            &[message],
            &CompletionOptions::default(),
        )
        .unwrap();
        let part = &body["messages"][0]["content"][1];
        assert_eq!(part["type"], "image_url");
        assert_eq!(part["image_url"]["url"], "data:image/png;base64,aaaa");
        assert_eq!(part["image_url"]["detail"], "low");
    }

    #[test]
    fn success_payload_and_sse_keep_the_answer_text() {
        let body = r#"{"id":"chatcmpl_x","object":"chat.completion","model":"meta-llama-4-maverick","choices":[{"index":0,"message":{"role":"assistant","content":"ok"},"finish_reason":"stop"}],"usage":{"prompt_tokens":16,"completion_tokens":2,"total_tokens":18,"cache_read_input_tokens":4}}"#;
        let parsed = parse_chat_response(body).unwrap();
        assert_eq!(parsed.content, "ok");
        assert_eq!(parsed.prompt_tokens, 16);
        assert_eq!(parsed.completion_tokens, 2);
        assert_eq!(parsed.cache_hit_tokens, Some(4));
        assert_eq!(parsed.finish_reason.as_deref(), Some("stop"));

        let sse = "data: {\"choices\":[{\"delta\":{\"content\":\"ok\"},\"finish_reason\":null}]}\n\ndata: [DONE]\n";
        assert_eq!(
            content_deltas_from_sse(sse).unwrap(),
            vec!["ok".to_string()]
        );
    }
}
