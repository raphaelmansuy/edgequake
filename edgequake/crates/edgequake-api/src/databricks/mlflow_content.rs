//! MLflow / AI Gateway message content.
//!
//! Chat Completions on `/ai-gateway/mlflow/v1` returns `content` as a string
//! or as a list of parts. Reasoning parts are not the answer.

use serde_json::Value;

/// Answer text and optional reasoning text from a `content` field.
pub fn answer_and_thinking(content: &Value) -> (String, Option<String>) {
    match content {
        Value::String(text) => (text.clone(), None),
        Value::Array(parts) => {
            let mut answer = String::new();
            let mut thinking = String::new();
            for part in parts {
                match part {
                    Value::String(text) => answer.push_str(text),
                    Value::Object(map) => {
                        let kind = map.get("type").and_then(Value::as_str).unwrap_or("");
                        if kind == "reasoning"
                            || map.get("thought").and_then(Value::as_bool) == Some(true)
                        {
                            push_part_text(&mut thinking, map.get("text"));
                            if let Some(summary) = map.get("summary").and_then(Value::as_array) {
                                for item in summary {
                                    if let Some(text) = item.get("text").and_then(Value::as_str) {
                                        thinking.push_str(text);
                                    }
                                }
                            }
                        } else {
                            push_part_text(&mut answer, map.get("text"));
                        }
                    }
                    _ => {}
                }
            }
            let thinking = if thinking.is_empty() {
                None
            } else {
                Some(thinking)
            };
            (answer, thinking)
        }
        Value::Null => (String::new(), None),
        other => (other.to_string(), None),
    }
}

/// Answer text only. Reasoning parts are dropped.
pub fn answer_text(content: &Value) -> String {
    answer_and_thinking(content).0
}

fn push_part_text(out: &mut String, text: Option<&Value>) {
    if let Some(text) = text.and_then(Value::as_str) {
        out.push_str(text);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn string_content_is_the_answer() {
        let (answer, thinking) = answer_and_thinking(&json!("pong"));
        assert_eq!(answer, "pong");
        assert!(thinking.is_none());
    }

    #[test]
    fn text_parts_join_and_reasoning_is_separate() {
        let content = json!([
            {"type": "reasoning", "summary": [{"type": "summary_text", "text": "think"}]},
            {"type": "text", "text": "pon"},
            {"type": "text", "text": "g"}
        ]);
        let (answer, thinking) = answer_and_thinking(&content);
        assert_eq!(answer, "pong");
        assert_eq!(thinking.as_deref(), Some("think"));
    }

    #[test]
    fn reasoning_only_does_not_become_the_answer() {
        let content = json!([{"type": "reasoning", "summary": [{"text": "hidden"}]}]);
        let (answer, thinking) = answer_and_thinking(&content);
        assert!(answer.is_empty());
        assert_eq!(thinking.as_deref(), Some("hidden"));
    }
}
