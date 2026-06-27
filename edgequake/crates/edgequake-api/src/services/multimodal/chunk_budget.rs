//! MM chunk token budgets (LightRAG `pipeline.py` `_build_mm_chunks_from_sidecars` L4430+).

use super::chunks::render_mm_chunk_with_description;
use super::item_record::MultimodalItemRecord;
use super::surrounding::SurroundingTokenCounter;

/// LightRAG `DEFAULT_MAX_EXTRACT_INPUT_TOKENS`.
const DEFAULT_MAX_EXTRACT_INPUT_TOKENS: usize = 20_480;

/// LightRAG `DEFAULT_MM_CHUNK_DESCRIPTION_MIN_TOKENS`.
const DEFAULT_MM_CHUNK_DESCRIPTION_MIN_TOKENS: usize = 100;

fn env_usize(name: &str) -> Option<usize> {
    std::env::var(name)
        .ok()
        .and_then(|v| v.parse().ok())
        .filter(|&n| n > 0)
}

/// Max tokens for a rendered multimodal chunk (env: `MAX_EXTRACT_INPUT_TOKENS`).
pub fn max_mm_chunk_tokens() -> usize {
    env_usize("MAX_EXTRACT_INPUT_TOKENS").unwrap_or(DEFAULT_MAX_EXTRACT_INPUT_TOKENS)
}

/// Minimum description tokens preserved when truncating (env: `MM_CHUNK_DESCRIPTION_MIN_TOKENS`).
pub fn min_mm_chunk_description_tokens() -> usize {
    env_usize("MM_CHUNK_DESCRIPTION_MIN_TOKENS").unwrap_or(DEFAULT_MM_CHUNK_DESCRIPTION_MIN_TOKENS)
}

fn truncate_text_to_tokens(
    text: &str,
    max_tokens: usize,
    counter: SurroundingTokenCounter,
) -> String {
    if counter.count(text) <= max_tokens {
        return text.to_string();
    }
    let mut acc = String::new();
    for ch in text.chars() {
        let candidate = format!("{acc}{ch}");
        if counter.count(&candidate) > max_tokens && !acc.is_empty() {
            break;
        }
        acc = candidate;
    }
    acc
}

/// Render mm chunk text with description-only truncation (LightRAG `_compose` loop).
pub fn render_mm_chunk_with_budget(
    record: &MultimodalItemRecord,
    modality: &str,
    footnotes: &[String],
) -> Result<String, String> {
    record
        .name
        .as_deref()
        .filter(|s| !s.is_empty())
        .ok_or_else(|| {
            format!(
                "{modality}/{}: success result missing 'name'",
                record.item_id
            )
        })?;
    let description = record
        .description
        .as_deref()
        .filter(|s| !s.is_empty())
        .ok_or_else(|| {
            format!(
                "{modality}/{}: success result missing 'description'",
                record.item_id
            )
        })?;
    if modality == "drawing"
        && record
            .item_type
            .as_deref()
            .filter(|s| !s.is_empty())
            .is_none()
    {
        return Err(format!(
            "drawings/{}: success result missing 'type'",
            record.item_id
        ));
    }
    if modality == "equation"
        && record
            .equation
            .as_deref()
            .filter(|s| !s.is_empty())
            .is_none()
    {
        return Err(format!(
            "equations/{}: success result missing 'equation'",
            record.item_id
        ));
    }

    let counter = SurroundingTokenCounter::from_env();
    let max_tokens = max_mm_chunk_tokens();
    let min_desc_tokens = min_mm_chunk_description_tokens();

    let compose =
        |desc: &str| render_mm_chunk_with_description(record, modality, footnotes, Some(desc));

    let mut chunk_content = compose(description);
    let mut tokens = counter.count(&chunk_content);
    if tokens <= max_tokens {
        return Ok(chunk_content);
    }

    let desc_tokens = counter.count(description);
    let overflow = tokens.saturating_sub(max_tokens);
    let mut keep = desc_tokens.saturating_sub(overflow).max(min_desc_tokens);

    loop {
        let truncated_desc = truncate_text_to_tokens(description, keep, counter);
        chunk_content = compose(&truncated_desc);
        tokens = counter.count(&chunk_content);
        if tokens <= max_tokens || keep <= min_desc_tokens {
            break;
        }
        keep = keep
            .saturating_sub(tokens.saturating_sub(max_tokens))
            .max(min_desc_tokens);
    }

    if tokens > max_tokens {
        return Err(format!(
            "{modality}/{}: multimodal chunk exceeds {max_tokens} tokens even after truncating description to {min_desc_tokens} tokens",
            record.item_id
        ));
    }
    Ok(chunk_content)
}

#[cfg(test)]
mod tests {
    use super::super::item_record::MultimodalItemRecord;
    use super::*;

    #[test]
    fn truncates_oversized_description() {
        std::env::set_var("EDGEQUAKE_MM_SURROUNDING_TOKENS", "char");
        std::env::set_var("MAX_EXTRACT_INPUT_TOKENS", "80");
        std::env::set_var("MM_CHUNK_DESCRIPTION_MIN_TOKENS", "10");
        let record = MultimodalItemRecord::success_image(
            "d1",
            "chart".to_string(),
            "Chart".to_string(),
            "x".repeat(200),
        );
        let text = render_mm_chunk_with_budget(&record, "drawing", &[]).unwrap();
        assert!(SurroundingTokenCounter::Char.count(&text) <= 80);
        assert!(text.contains("[Image Name]chart"));
        std::env::remove_var("EDGEQUAKE_MM_SURROUNDING_TOKENS");
        std::env::remove_var("MAX_EXTRACT_INPUT_TOKENS");
        std::env::remove_var("MM_CHUNK_DESCRIPTION_MIN_TOKENS");
    }
}
