//! Canonical JSON extraction from LLM responses (SPEC-017 P1-06).
//!
//! Handles markdown fences, truncated objects/arrays, and preamble text.

/// Extract JSON from a potentially wrapped or truncated LLM response.
pub fn extract_json_from_response(response: &str) -> String {
    let response = response.trim();

    if let Some(start) = response.find("```json") {
        if let Some(end) = response[start + 7..].find("```") {
            return response[start + 7..start + 7 + end].trim().to_string();
        }
    }

    if let Some(start) = response.find("```") {
        if let Some(end) = response[start + 3..].find("```") {
            let content = response[start + 3..start + 3 + end].trim();
            if content.starts_with('{') || content.starts_with('[') {
                return content.to_string();
            }
        }
    }

    let start_obj = response.find('{');
    let start_arr = response.find('[');

    let (start_idx, start_ch) = match (start_obj, start_arr) {
        (Some(o), Some(a)) => {
            if o <= a {
                (o, '{')
            } else {
                (a, '[')
            }
        }
        (Some(o), None) => (o, '{'),
        (None, Some(a)) => (a, '['),
        (None, None) => (usize::MAX, '\0'),
    };

    if start_idx == usize::MAX {
        return String::new();
    }

    match start_ch {
        '{' => extract_balanced_json_slice(response, start_idx, '{', '}'),
        '[' => extract_balanced_json_slice(response, start_idx, '[', ']'),
        _ => String::new(),
    }
}

fn extract_balanced_json_slice(
    response: &str,
    start_idx: usize,
    open: char,
    close: char,
) -> String {
    let mut balance: i32 = 0;
    for c in response[start_idx..].chars() {
        if c == open {
            balance += 1;
        } else if c == close {
            balance -= 1;
        }
    }

    if balance == 0 {
        if let Some(end) = response.rfind(close) {
            if end > start_idx {
                return response[start_idx..=end].to_string();
            }
        }
    }

    response[start_idx..].to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn truncated_object_returns_suffix_for_recovery() {
        let response = "Here is JSON: {\"entities\": [{\"name\": \"A\"";
        let extracted = extract_json_from_response(response);
        assert!(extracted.starts_with('{'));
        assert!(!extracted.ends_with('}'));
    }

    #[test]
    fn truncated_array_returns_suffix_for_recovery() {
        let response = "prefix [{\"name\": \"A\"";
        let extracted = extract_json_from_response(response);
        assert!(extracted.starts_with('['));
    }

    #[test]
    fn fenced_json_block() {
        let response = "```json\n{\"key\": \"value\"}\n```";
        assert_eq!(extract_json_from_response(response), "{\"key\": \"value\"}");
    }

    #[test]
    fn no_json_returns_empty() {
        assert_eq!(extract_json_from_response("no json here"), "");
    }
}
