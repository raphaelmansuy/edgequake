//! LLM temperature gating for extraction providers.

/// Returns the effective temperature override to send for a model.
///
/// WHY: Some OpenAI model families only accept their built-in default temperature
/// and reject any explicit override with an API error. Omitting the field is the
/// most compatible behavior for those models while preserving overrides for models
/// that still support them.
pub fn effective_temperature_for_model(model: &str, preferred_temperature: f32) -> Option<f32> {
    if model_requires_default_temperature(model) {
        None
    } else {
        Some(preferred_temperature)
    }
}

fn model_requires_default_temperature(model: &str) -> bool {
    let normalized = model
        .trim()
        .rsplit('/')
        .next()
        .unwrap_or(model)
        .to_ascii_lowercase();

    normalized.contains("gpt-5")
        || normalized.contains("gpt-4.1-nano")
        || normalized.contains("gpt-4.1-mini")
        || normalized.starts_with("o1")
        || normalized.starts_with("o3")
        || normalized.starts_with("o4")
}
