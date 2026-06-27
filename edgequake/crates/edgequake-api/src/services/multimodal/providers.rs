//! Role-scoped LLM providers for multimodal analyze (VLM vs Extract).

use edgequake_llm::traits::LLMProvider;

/// VLM + Extract providers for the analyze stage (LightRAG role split).
#[derive(Clone, Copy)]
pub struct MultimodalProviders<'a> {
    pub vlm: &'a dyn LLMProvider,
    pub extract: &'a dyn LLMProvider,
}

impl<'a> MultimodalProviders<'a> {
    /// Use one provider for both roles (tests and degraded setups).
    pub fn single(llm: &'a dyn LLMProvider) -> Self {
        Self {
            vlm: llm,
            extract: llm,
        }
    }

    pub fn split(vlm: &'a dyn LLMProvider, extract: &'a dyn LLMProvider) -> Self {
        Self { vlm, extract }
    }
}
