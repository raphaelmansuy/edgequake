use std::collections::HashMap;
use std::sync::Arc;

use crate::context::QueryContext;
use crate::error::Result;
use edgequake_llm::traits::{ChatMessage, ImageData};

use super::SOTAQueryEngine;

impl SOTAQueryEngine {
    /// Check if metadata matches tenant/workspace filter.
    ///
    /// DEPRECATED (SPEC-007): Prefer `query_filtered()` which pushes filtering to SQL.
    /// Retained for backward-compat with custom VectorStorage impls that don't override
    /// `query_filtered()`.
    #[allow(dead_code)]
    pub(super) fn matches_tenant_filter(
        &self,
        metadata: &serde_json::Value,
        tenant_id: &Option<String>,
        workspace_id: &Option<String>,
    ) -> bool {
        edgequake_storage::MetadataFilter::matches_tenant_workspace_value(
            metadata,
            tenant_id,
            workspace_id,
        )
    }

    /// Check if properties match tenant filter.
    ///
    /// DEPRECATED (SPEC-007): Prefer `query_filtered()` which pushes filtering to SQL.
    #[allow(dead_code)]
    pub(super) fn matches_tenant_filter_props(
        &self,
        properties: &HashMap<String, serde_json::Value>,
        tenant_id: &Option<String>,
        workspace_id: &Option<String>,
    ) -> bool {
        edgequake_storage::MetadataFilter::matches_tenant_workspace_properties(
            properties,
            tenant_id,
            workspace_id,
        )
    }

    // ── Private helpers ──────────────────────────────────────────────────────

    /// Build the shared context section (context text + optional extra instructions).
    ///
    /// WHY (DRY): Both `build_prompt` (text-only path) and
    /// `build_vision_system_message` (chat/vision path) need the same context
    /// block.  Centralising it here avoids duplication and ensures a single
    /// point of change.
    fn format_context_section(
        context: &QueryContext,
        system_prompt_extension: Option<&str>,
    ) -> (String, String) {
        let context_text = context.to_context_string();
        // SPEC-004: optional additional instructions injected by callers
        let additional_instructions = match system_prompt_extension {
            Some(ext) if !ext.trim().is_empty() => {
                format!("\n\n---Additional Instructions---\n\n{}\n", ext.trim())
            }
            _ => String::new(),
        };
        (context_text, additional_instructions)
    }

    // ── Public(super) prompt builders ────────────────────────────────────────

    /// Build an all-in-one text prompt for `provider.complete()` (text-only path).
    ///
    /// WHY: The prompt is designed to maximise information extraction from available
    /// context.  When comparing products where one term doesn't exist in the knowledge
    /// base, we still want to provide useful information about what IS available,
    /// rather than just saying "no information found."
    ///
    /// `system_prompt_extension`: Optional additional instructions injected between
    /// the base instructions and the context section (SPEC-004).
    pub(super) fn build_prompt(
        &self,
        query: &str,
        context: &QueryContext,
        system_prompt_extension: Option<&str>,
    ) -> String {
        if context.is_empty() {
            return "I'm sorry, but I couldn't find any relevant information in my knowledge base to answer your question.".to_string();
        }

        let (context_text, additional_instructions) =
            Self::format_context_section(context, system_prompt_extension);

        format!(
            r#"---Role---

You are an expert AI assistant specializing in synthesizing information from a provided knowledge base. Your primary function is to answer user queries accurately by ONLY using the information within the provided **Context**.

---Goal---

Generate a comprehensive, well-structured answer to the user query.
The answer must integrate relevant facts from the Knowledge Graph and Document Chunks found in the **Context**.

---Instructions---

1. Step-by-Step Reasoning:
  - Carefully determine the user's query intent to fully understand the information need.
  - Scrutinize both Knowledge Graph Data (Entities and Relationships) and Document Chunks in the **Context**. Identify and extract all pieces of information that are directly relevant to answering the user query.
  - Weave the extracted facts into a coherent and logical response. Your own knowledge must ONLY be used to formulate fluent sentences and connect ideas, NOT to introduce any external information.

2. Content & Grounding:
  - Strictly adhere to the provided context; DO NOT invent, assume, or infer any information not explicitly stated.
  - If the answer cannot be fully determined from the **Context**, state what information IS available and note what is missing. A partial answer with specific data is better than a generic "insufficient information" response.

3. Formatting & Language:
  - The response MUST be in the same language as the user query.
  - Use Markdown formatting for clarity (headings, bold text, bullet points).
{additional_instructions}
---Context---

{context_text}

---User Query---

{query}"#
        )
    }

    /// Build the **system message** for a vision-enabled `provider.chat()` call.
    ///
    /// WHY (First Principles): The chat API separates concerns cleanly —
    /// role/instructions/context belong in the *system* message; the user's
    /// actual query (+ images) belong in the *user* message.  Putting the role
    /// text ("ONLY use the knowledge graph") inside the *user* message (as the
    /// previous code did) caused the LLM to refuse image queries because the
    /// role text explicitly said to ignore non-textual input.
    ///
    /// This method returns only the system half.  The caller is responsible for
    /// constructing `ChatMessage::user_with_images(query, images)`.
    pub(super) fn build_vision_system_message(
        &self,
        context: &QueryContext,
        system_prompt_extension: Option<&str>,
    ) -> String {
        let (context_text, additional_instructions) =
            Self::format_context_section(context, system_prompt_extension);

        format!(
            r#"---Role---

You are an expert AI assistant that can analyse images and synthesise information from a provided knowledge base. Your primary function is to answer user queries by using:
1. The visual content of any attached images.
2. The information within the provided **Context** (knowledge graph entities, relationships, and document chunks).

---Goal---

Generate a comprehensive, well-structured answer that integrates observations from the attached images with relevant facts from the Knowledge Graph and Document Chunks.

---Instructions---

1. Visual Analysis:
  - Examine every attached image carefully before answering.
  - Describe, identify, or interpret visual content as requested by the user.
  - Cross-reference visual observations with knowledge graph entities when relevant.

2. Step-by-Step Reasoning:
  - Carefully determine the user's query intent.
  - Extract facts from both the images and the **Context** that are relevant to the query.
  - Weave observations and facts into a coherent, logical response.

3. Content & Grounding:
  - Prefer explicit visual evidence from images and stated facts from the context.
  - If the answer cannot be fully determined, state what IS available and note what is missing.

4. Formatting & Language:
  - The response MUST be in the same language as the user query.
  - Use Markdown formatting for clarity (headings, bold text, bullet points).
{additional_instructions}
---Context---

{context_text}"#
        )
    }

    /// Generate answer using LLM.
    ///
    /// If `llm_override` is provided, uses that provider instead of the default.
    /// This enables per-request provider selection (SPEC-032).
    ///
    /// If `images` is Some and non-empty, uses `provider.chat()` with image
    /// attachments instead of `provider.complete()` (FEAT0203: vision queries).
    pub(super) async fn generate_answer_with_provider(
        &self,
        query: &str,
        context: &QueryContext,
        llm_override: Option<&Arc<dyn crate::LLMProvider>>,
        system_prompt_extension: Option<&str>,
        images: Option<&[ImageData]>,
    ) -> Result<(String, usize)> {
        if context.is_empty() {
            return Ok((
                "I'm sorry, but I couldn't find any relevant information in my knowledge base to answer your question.".to_string(),
                0,
            ));
        }

        let provider = llm_override.unwrap_or(&self.llm_provider);

        // FEAT0203: Two distinct call paths based on whether images are attached.
        //
        // WHY (First Principles): chat() separates system instructions from the user
        // turn.  Putting role text ("ONLY use text context") into the *user* message
        // alongside images caused the LLM to refuse image queries.  The fix is:
        //   • system message  → role + instructions + RAG context (no images, no query)
        //   • user message    → raw query + images
        // This gives the LLM the full context AND the visual content in the correct
        // roles, so it can use both freely.
        //
        // Text-only path keeps using provider.complete() to avoid an unnecessary
        // chat-API round-trip for providers that support both.
        let response = if let Some(imgs) = images.filter(|i| !i.is_empty()) {
            let system_text = self.build_vision_system_message(context, system_prompt_extension);
            let messages = vec![
                ChatMessage::system(&system_text),
                ChatMessage::user_with_images(query, imgs.to_vec()),
            ];
            match provider.chat(&messages, None).await {
                Ok(r) => r,
                Err(e) => {
                    tracing::warn!(error = %e, "Vision chat failed; retrying as text-only query");
                    provider
                        .complete(&self.build_prompt(query, context, system_prompt_extension))
                        .await?
                }
            }
        } else {
            provider
                .complete(&self.build_prompt(query, context, system_prompt_extension))
                .await?
        };

        Ok((response.content, response.completion_tokens))
    }

    /// Generate answer using the default LLM.
    pub(super) async fn generate_answer(
        &self,
        query: &str,
        context: &QueryContext,
        system_prompt_extension: Option<&str>,
    ) -> Result<(String, usize)> {
        self.generate_answer_with_provider(query, context, None, system_prompt_extension, None)
            .await
    }
}
