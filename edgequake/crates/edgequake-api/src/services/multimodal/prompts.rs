//! Multimodal VLM / Extract prompt builders (LightRAG `prompt_multimodal.py` parity).

use base64::{engine::general_purpose::STANDARD as B64, Engine as _};
use edgequake_llm::traits::{ChatMessage, ImageData};

use super::prompt_context::{table_content_format_label, PromptContext};

const IMAGE_ANALYSIS_SYSTEM_PROMPT: &str = "\
You are an expert image analyzer. Analyze the provided image and return a single JSON object.

Use Additional Context (Captions, Footnotes, Leading/Trailing Text) only to disambiguate — the image itself takes priority.
Return ONLY valid JSON with keys: \"name\" (snake_case), \"type\" (Photo|Illustration|Screenshot|Icon|Chart|Table|Infographic|Flowchart|Chat Log|Wireframe|Texture|Other), \"description\" (markdown, ≤500 words).
Output values for name and description must be in the requested language.";

const TABLE_ANALYSIS_SYSTEM_PROMPT: &str = "\
You are an expert table analyzer. Analyze the table content and return a single JSON object.

Use Additional Context only for disambiguation — table content takes priority. Never invent rows or values.
Return ONLY valid JSON with keys: \"name\" (snake_case), \"type\" (always \"Table\"), \"description\" (markdown, ≤500 words).
Output values for name and description must be in the requested language.";

const EQUATION_ANALYSIS_SYSTEM_PROMPT: &str = "\
You are an expert equation analyzer. Analyze the equation and return a single JSON object.

Use Additional Context only for disambiguation — equation body takes priority.
Return ONLY valid JSON with keys: \"name\" (snake_case), \"equation\" (LaTeX math-mode body, no $ delimiters), \"description\" (≤300 words).
Output values for name and description must be in the requested language.";

/// Build initial VLM messages for image analysis with LightRAG context block.
pub fn image_analysis_messages(
    image_bytes: &[u8],
    mime_type: &str,
    ctx: &PromptContext,
) -> Vec<ChatMessage> {
    let base64_data = B64.encode(image_bytes);
    let image_data = ImageData::new(&base64_data, mime_type);

    let user_text = format!(
        "Analyze this image and return the JSON object.\n\
         Language: {}\n\n{}\n\nOutput:",
        ctx.language,
        ctx.additional_context_block()
    );

    vec![
        ChatMessage::system(IMAGE_ANALYSIS_SYSTEM_PROMPT),
        ChatMessage::user_with_images(user_text, vec![image_data]),
    ]
}

/// Extract-role messages for HTML/JSON table analysis.
pub fn table_analysis_messages(
    table_body: &str,
    format: &str,
    ctx: &PromptContext,
) -> Result<Vec<ChatMessage>, String> {
    let format_label = table_content_format_label(format)?;
    let user_text = format!(
        "Analyze this table and return the JSON object.\n\
         Language: {}\n\n\
         ================ TABLE CONTENT ================\n\
         The TABLE CONTENT below is in {format_label}.\n\
         ```\n{table_body}\n```\n\n\
         {}\n\nOutput:",
        ctx.language,
        ctx.additional_context_block()
    );
    Ok(vec![
        ChatMessage::system(TABLE_ANALYSIS_SYSTEM_PROMPT),
        ChatMessage::user(user_text),
    ])
}

/// Extract-role messages for equation analysis.
pub fn equation_analysis_messages(equation_body: &str, ctx: &PromptContext) -> Vec<ChatMessage> {
    let user_text = format!(
        "Analyze this equation and return the JSON object.\n\
         Language: {}\n\n\
         ================ EQUATION BODY ================\n\
         ```\n{equation_body}\n```\n\n\
         {}\n\nOutput:",
        ctx.language,
        ctx.additional_context_block()
    );
    vec![
        ChatMessage::system(EQUATION_ANALYSIS_SYSTEM_PROMPT),
        ChatMessage::user(user_text),
    ]
}

/// Fingerprint text for analysis cache hashing (LightRAG args_hash inputs).
pub fn prompt_cache_fingerprint(messages: &[ChatMessage]) -> String {
    use edgequake_llm::traits::ChatRole;
    messages
        .iter()
        .map(|m| {
            let role = match m.role {
                ChatRole::System => "system",
                ChatRole::User => "user",
                ChatRole::Assistant => "assistant",
                ChatRole::Tool => "tool",
                ChatRole::Function => "function",
            };
            format!("{role}:{}", m.content)
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// Repair-turn user message after invalid JSON.
pub fn json_repair_user_message(invalid_response: &str) -> String {
    format!("Previous invalid response:\n{invalid_response}\n\nReturn corrected JSON only.")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn image_prompt_includes_caption_from_context() {
        let ctx = PromptContext {
            language: "English".into(),
            captions: "Quarterly revenue chart".into(),
            footnotes: "n/a".into(),
            leading: "See figure below".into(),
            trailing: "n/a".into(),
        };
        let msgs = image_analysis_messages(&[0u8; 8], "image/png", &ctx);
        let user = msgs[1].content.as_str();
        assert!(user.contains("Quarterly revenue chart"));
        assert!(user.contains("See figure below"));
    }

    #[test]
    fn table_prompt_includes_format_label() {
        let ctx = PromptContext {
            language: "English".into(),
            captions: "n/a".into(),
            footnotes: "n/a".into(),
            leading: "n/a".into(),
            trailing: "n/a".into(),
        };
        let msgs = table_analysis_messages("<tr><td>A</td></tr>", "html", &ctx).unwrap();
        assert!(msgs[1].content.contains("HTML format"));
    }
}
