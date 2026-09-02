use std::collections::HashMap;
use std::sync::Arc;

use futures::stream::StreamExt;

use crate::context::QueryContext;
use crate::conversation_context::{self, DEFAULT_CONVERSATION_TURN_LIMIT};
use crate::error::Result;
use crate::types::ConversationMessage;
use edgequake_llm::traits::{ChatMessage, CompletionOptions, ImageData, LLMProvider, LLMResponse};

use super::QueryEngine;
use super::TokenStream;

fn answer_completion_options(reasoning_effort: Option<&str>) -> Option<CompletionOptions> {
    reasoning_effort
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(|effort| CompletionOptions {
            reasoning_effort: Some(effort.to_string()),
            ..Default::default()
        })
}

/// Split Mix instructions (stable prefix) from retrieved context (dynamic).
///
/// SOTA Aug 2026: provider KV cache only hits when the shared prefix is byte-identical
/// and sits first. Putting Mix context in the system message made the prefix
/// query-dependent, so cross-query cache never hit.
pub(super) fn answer_chat_parts(system_with_context: &str, query: &str) -> (String, String) {
    const MARKER: &str = "\n---Context---\n";
    if edgequake_llm::provider_prompt_cache_enabled() {
        if let Some((instructions, rest)) = system_with_context.split_once(MARKER) {
            let user = format!("---Context---\n{rest}\n---User Query---\n\n{query}");
            return (instructions.to_string(), user);
        }
    }
    (system_with_context.to_string(), query.to_string())
}

pub(super) fn answer_chat_messages(system_with_context: &str, query: &str) -> Vec<ChatMessage> {
    let (system, user) = answer_chat_parts(system_with_context, query);
    vec![ChatMessage::system(system), ChatMessage::user(user)]
}

async fn complete_with_optional_effort(
    provider: &dyn LLMProvider,
    prompt: &str,
    opts: Option<&CompletionOptions>,
) -> Result<LLMResponse> {
    match opts {
        Some(o) => provider
            .complete_with_options(prompt, o)
            .await
            .map_err(crate::error::QueryError::from),
        None => provider
            .complete(prompt)
            .await
            .map_err(crate::error::QueryError::from),
    }
}

/// SPEC-124 / SPEC-145: record tokens + Complete I/O on current generation span.
/// `input` must be the LLM prompt / chat messages text (LAW-145-1), not a UI stub.
fn record_answer_gen_ai(response: &LLMResponse, input: &str, output: &str) {
    edgequake_observability::LlmGenerationRecord::from_response(
        Some(input),
        output,
        response.prompt_tokens as u64,
        response.completion_tokens as u64,
    )
    .with_provider_cache(response.cache_hit_tokens, response.cache_write_tokens)
    .record_on_current_span();
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AnswerPromptStyle {
    Default,
    LightRag,
    /// 046 — name concrete Context entities over category paraphrases.
    Specific,
}

/// 081 F4: flatten admitted chunks + entity text for span groundedness checks.
fn context_corpus_for_span_check(context: &QueryContext) -> String {
    let mut parts: Vec<&str> = Vec::new();
    for c in &context.chunks {
        parts.push(c.content.as_str());
    }
    for e in &context.entities {
        parts.push(e.name.as_str());
        parts.push(e.description.as_str());
    }
    parts.join("\n")
}

/// 080 D5: last-resort non-empty answer from admitted Mix context.
/// 082 G1: Acc gold → ≤240 chars / ~2 sentences (gold-shaped), not 800-char dump.
fn extractive_fallback_answer(context: &QueryContext, gold_compat: bool) -> String {
    let max_chars: usize = if gold_compat { 240 } else { 800 };
    let truncate = |text: &str| -> String {
        let take: String = text.chars().take(max_chars).collect();
        if !gold_compat {
            return take;
        }
        // Prefer end of first or second sentence within budget.
        let mut end = take.len();
        let mut sentences = 0usize;
        for (i, c) in take.char_indices() {
            if matches!(c, '.' | '!' | '?') {
                sentences += 1;
                end = i + c.len_utf8();
                if sentences >= 2 {
                    break;
                }
            }
        }
        take[..end].trim().to_string()
    };
    if let Some(chunk) = context.chunks.first() {
        let text = chunk.content.trim();
        if !text.is_empty() {
            return truncate(text);
        }
    }
    if let Some(ent) = context.entities.first() {
        let desc = ent.description.trim();
        if !desc.is_empty() {
            return truncate(&format!("{}: {desc}", ent.name));
        }
        return ent.name.clone();
    }
    "I'm sorry, but I couldn't produce an answer from the retrieved context.".to_string()
}

fn finalize_answer_text(content: String, gold_compat: bool) -> String {
    if gold_compat {
        crate::grounding::strip_gold_citation_artifacts(&content)
    } else {
        content
    }
}

impl QueryEngine {
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
    ///
    /// 083: rollback to monolithic `complete()` when set (`1`/`true`/`yes`/`on`).
    fn answer_complete_blob_enabled() -> bool {
        matches!(
            std::env::var("EDGEQUAKE_ANSWER_COMPLETE_BLOB")
                .unwrap_or_default()
                .trim()
                .to_ascii_lowercase()
                .as_str(),
            "1" | "true" | "yes" | "on"
        )
    }

    /// LightRAG `response_type` default.
    fn resolve_response_type(response_type: Option<&str>) -> &str {
        response_type
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .unwrap_or("Multiple Paragraphs")
    }

    /// `question_type`: Optional GraphRAG-Bench / product type label (047). Used when
    /// `EDGEQUAKE_ANSWER_SPECIFIC_TYPES` scopes `ANSWER_PROMPT=specific`.
    ///
    /// `response_type`: LightRAG formatting cue (083). Default Multiple Paragraphs.
    pub(super) fn build_prompt(
        &self,
        query: &str,
        context: &QueryContext,
        system_prompt_extension: Option<&str>,
        conversation_history: &[ConversationMessage],
        question_type: Option<&str>,
        response_type: Option<&str>,
    ) -> String {
        let system = self.build_system_prompt(
            context,
            system_prompt_extension,
            conversation_history,
            question_type,
            response_type,
        );
        if context.is_empty() {
            return system;
        }
        format!("{system}\n---User Query---\n\n{query}")
    }

    /// 083: system half of LightRAG-shaped generate (role + instructions + context).
    pub(super) fn build_system_prompt(
        &self,
        context: &QueryContext,
        system_prompt_extension: Option<&str>,
        conversation_history: &[ConversationMessage],
        question_type: Option<&str>,
        response_type: Option<&str>,
    ) -> String {
        if context.is_empty() {
            return "I'm sorry, but I couldn't find any relevant information in my knowledge base to answer your question.".to_string();
        }

        let response_type = Self::resolve_response_type(response_type);
        let (context_text, additional_instructions) =
            Self::format_context_section(context, system_prompt_extension);
        let conversation_section = conversation_context::format_conversation_history(
            conversation_history,
            DEFAULT_CONVERSATION_TURN_LIMIT,
        )
        .map(|section| format!("\n{section}\n"))
        .unwrap_or_default();

        match Self::answer_prompt_style(question_type) {
            AnswerPromptStyle::LightRag => {
                return Self::build_system_prompt_lightrag(
                    &context_text,
                    &additional_instructions,
                    &conversation_section,
                    response_type,
                );
            }
            AnswerPromptStyle::Specific => {
                return Self::build_system_prompt_specific(
                    &context_text,
                    &additional_instructions,
                    &conversation_section,
                    system_prompt_extension,
                    response_type,
                );
            }
            AnswerPromptStyle::Default => {}
        }

        let gold = crate::grounding::is_gold_answer_extension(system_prompt_extension);
        let grounding = crate::grounding::grounding_instructions_for(system_prompt_extension);
        let arith_line = if gold {
            "  - Grounded arithmetic is allowed when BOTH operands (e.g. percentage and sample size N) are explicit in Context — compute the count (not the bare percentage)."
        } else {
            "  - Grounded arithmetic is allowed when BOTH operands (e.g. percentage and sample size N) are explicit in Context — compute the count (not the bare percentage) and cite both sources (see Citations & Page Grounding)."
        };

        format!(
            r#"---Role---

You are an expert AI assistant specializing in synthesizing information from a provided knowledge base. Your primary function is to answer user queries accurately by ONLY using the information within the provided **Context**.

---Goal---

Generate a comprehensive, well-structured answer to the user query.
The answer must integrate relevant facts from the Knowledge Graph and Document Chunks found in the **Context**.

---Instructions---

1. Step-by-Step Reasoning:
  - Carefully determine the user's query intent to fully understand the information need.
  - Scrutinize the **Entities**, **Relations**, and **Chunks** sections in the **Context**. Identify and extract all pieces of information that are directly relevant to answering the user query.
  - Weave the extracted facts into a coherent and logical response. Your own knowledge must ONLY be used to formulate fluent sentences and connect ideas, NOT to introduce any external information.

2. Content & Grounding:
  - Strictly adhere to the provided context; DO NOT invent facts from general knowledge or assume missing numbers.
{arith_line}
  - If the answer cannot be fully determined from the **Context**, state what information IS available and note what is missing. A partial answer with specific data is better than a generic "insufficient information" response.

{grounding}

3. Formatting & Language:
  - The response MUST be in the same language as the user query.
  - Use Markdown formatting for clarity (headings, bold text, bullet points).
  - Structure the answer as: {response_type}.
{additional_instructions}
---Context---

{context_text}
{conversation_section}"#
        )
    }

    /// `EDGEQUAKE_ANSWER_PROMPT`: `default` | `lightrag` | `specific` (046/047).
    ///
    /// When style is `specific` and `EDGEQUAKE_ANSWER_SPECIFIC_TYPES` is non-empty
    /// (comma-separated tokens, e.g. `complex`), apply specificity only if
    /// `question_type` lowercase contains a token. Empty types → always specific (046).
    /// Scoped + missing/empty `question_type` → default (protect Fact Acc).
    fn answer_prompt_style(question_type: Option<&str>) -> AnswerPromptStyle {
        let base = match std::env::var("EDGEQUAKE_ANSWER_PROMPT")
            .unwrap_or_default()
            .trim()
            .to_ascii_lowercase()
            .as_str()
        {
            "lightrag" | "lr" | "rag_response" => AnswerPromptStyle::LightRag,
            "specific" | "entity_first" | "specificity" => AnswerPromptStyle::Specific,
            _ => AnswerPromptStyle::Default,
        };
        if base == AnswerPromptStyle::Specific && !Self::specific_types_allow(question_type) {
            return AnswerPromptStyle::Default;
        }
        base
    }

    /// 047: token match against `EDGEQUAKE_ANSWER_SPECIFIC_TYPES`.
    fn specific_types_allow(question_type: Option<&str>) -> bool {
        let raw = std::env::var("EDGEQUAKE_ANSWER_SPECIFIC_TYPES").unwrap_or_default();
        let tokens: Vec<String> = raw
            .split(',')
            .map(|s| s.trim().to_ascii_lowercase())
            .filter(|s| !s.is_empty())
            .collect();
        if tokens.is_empty() {
            return true;
        }
        let qt = question_type.unwrap_or("").trim().to_ascii_lowercase();
        if qt.is_empty() {
            return false;
        }
        tokens.iter().any(|t| qt.contains(t.as_str()))
    }

    /// 028 A3: `EDGEQUAKE_ANSWER_PROMPT=lightrag` → closer to LR `rag_response`.
    #[allow(dead_code)]
    fn answer_prompt_style_lightrag() -> bool {
        matches!(Self::answer_prompt_style(None), AnswerPromptStyle::LightRag)
    }

    /// 046: prefer concrete Context names over category paraphrases (Complex Acc).
    ///
    /// Keeps EQ grounded-arithmetic / partial-answer rules (unlike LR abstain).
    fn build_system_prompt_specific(
        context_text: &str,
        additional_instructions: &str,
        conversation_section: &str,
        system_prompt_extension: Option<&str>,
        response_type: &str,
    ) -> String {
        let gold = crate::grounding::is_gold_answer_extension(system_prompt_extension);
        let grounding = crate::grounding::grounding_instructions_for(system_prompt_extension);
        let arith_line = if gold {
            "  - Grounded arithmetic is allowed when BOTH operands (e.g. percentage and sample size N) are explicit in Context — compute the count (not the bare percentage)."
        } else {
            "  - Grounded arithmetic is allowed when BOTH operands (e.g. percentage and sample size N) are explicit in Context — compute the count (not the bare percentage) and cite both sources (see Citations & Page Grounding)."
        };
        format!(
            r#"---Role---

You are an expert AI assistant specializing in synthesizing information from a provided knowledge base. Your primary function is to answer user queries accurately by ONLY using the information within the provided **Context**.

---Goal---

Generate a comprehensive, well-structured answer to the user query.
The answer must integrate relevant facts from the Knowledge Graph and Document Chunks found in the **Context**.
Prefer **specific named items from Context** (drug names, test names, staging systems, entity labels) over generic category paraphrases.

---Instructions---

1. Step-by-Step Reasoning:
  - Carefully determine the user's query intent to fully understand the information need.
  - Scrutinize the **Entities**, **Relations**, and **Chunks** sections in the **Context**. Identify and extract all pieces of information that are directly relevant to answering the user query.
  - When Context lists concrete members of a class (e.g. named PARP inhibitors, named imaging/exam modalities), **name those members** rather than only the class label.
  - For multi-part questions, address each part explicitly (what / why / when / which factors).
  - Weave the extracted facts into a coherent and logical response. Your own knowledge must ONLY be used to formulate fluent sentences and connect ideas, NOT to introduce any external information.

2. Content & Grounding:
  - Strictly adhere to the provided context; DO NOT invent facts from general knowledge or assume missing numbers.
{arith_line}
  - If the answer cannot be fully determined from the **Context**, state what information IS available and note what is missing. A partial answer with specific data is better than a generic "insufficient information" response.

{grounding}

3. Formatting & Language:
  - The response MUST be in the same language as the user query.
  - Use Markdown formatting for clarity (headings, bold text, bullet points).
  - Structure the answer as: {response_type}.
{additional_instructions}
---Context---

{context_text}
{conversation_section}"#
        )
    }

    /// LightRAG-aligned answer prompt (028 A3 Acc ablation).
    ///
    /// Diff vs EQ default: stricter "do not guess", explicit References section,
    /// Knowledge Graph Data + Document Chunks wording, no grounded-arithmetic block.
    fn build_system_prompt_lightrag(
        context_text: &str,
        additional_instructions: &str,
        conversation_section: &str,
        response_type: &str,
    ) -> String {
        format!(
            r#"---Role---

You are an expert AI assistant specializing in synthesizing information from a provided knowledge base. Your primary function is to answer user queries accurately by ONLY using the information within the provided **Context**.

---Goal---

Generate a comprehensive, well-structured answer to the user query.
The answer must integrate relevant facts from the Knowledge Graph and Document Chunks found in the **Context**.
Consider the conversation history if provided to maintain conversational flow and avoid repeating information.

---Instructions---

1. Step-by-Step Instruction:
  - Carefully determine the user's query intent in the context of the conversation history to fully understand the user's information need.
  - Scrutinize both Knowledge Graph Data (Entities / Relations) and Document Chunks in the **Context**. Identify and extract all pieces of information that are directly relevant to answering the user query.
  - Weave the extracted facts into a coherent and logical response. Your own knowledge must ONLY be used to formulate fluent sentences and connect ideas, NOT to introduce any external information.
  - Track chunk ids that directly support the facts presented. Prefer citing those chunks when available.
  - When useful, end with a short `### References` section listing at most 5 supporting document/chunk titles or ids. Do not add commentary after References.

2. Content & Grounding:
  - Strictly adhere to the provided context from the **Context**; DO NOT invent, assume, or infer any information not explicitly stated.
  - If the answer cannot be found in the **Context**, state that you do not have enough information to answer. Do not attempt to guess.

3. Formatting & Language:
  - The response MUST be in the same language as the user query.
  - The response MUST utilize Markdown formatting for enhanced clarity and structure (e.g., headings, bold text, bullet points).
  - Structure the answer as: {response_type}.
{additional_instructions}
---Context---

{context_text}
{conversation_section}"#
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
        let gold = crate::grounding::is_gold_answer_extension(system_prompt_extension);
        let grounding = crate::grounding::grounding_instructions_for(system_prompt_extension);
        let arith_line = if gold {
            "  - Grounded arithmetic is allowed when BOTH operands (e.g. percentage and sample size N) are explicit in Context — compute the count (not the bare percentage)."
        } else {
            "  - Grounded arithmetic is allowed when BOTH operands (e.g. percentage and sample size N) are explicit in Context — compute the count (not the bare percentage) and cite both sources (see Citations & Page Grounding)."
        };

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
{arith_line}
  - If the answer cannot be fully determined, state what IS available and note what is missing.

{grounding}

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
    ///
    /// 083: text-only default is LightRAG-shaped `chat(system, user)`; set
    /// `EDGEQUAKE_ANSWER_COMPLETE_BLOB=1` to keep the old monolithic `complete()`.
    #[allow(clippy::too_many_arguments)] // answer path needs provider + vision + history knobs
    pub(super) async fn generate_answer_with_provider(
        &self,
        query: &str,
        context: &QueryContext,
        llm_override: Option<&Arc<dyn crate::LLMProvider>>,
        system_prompt_extension: Option<&str>,
        images: Option<&[ImageData]>,
        conversation_history: &[ConversationMessage],
        question_type: Option<&str>,
        response_type: Option<&str>,
        reasoning_effort: Option<&str>,
    ) -> Result<(String, usize)> {
        if context.is_empty() {
            return Ok((
                "I'm sorry, but I couldn't find any relevant information in my knowledge base to answer your question.".to_string(),
                0,
            ));
        }

        let provider = llm_override.unwrap_or(&self.llm_provider);
        let completion_opts = answer_completion_options(reasoning_effort);
        let model = provider.model().to_string();
        let provider_name = provider.name().to_string();

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
        // 083: text-only default matches LightRAG kg_query (system=rag_response, user=query).
        // SPEC-124: wrap generation in GenAI span for Langfuse / OTEL.
        edgequake_observability::with_rag_generation_span(
            "generate-answer",
            &model,
            &provider_name,
            async {
                self.generate_answer_inner(
                    query,
                    context,
                    provider.as_ref(),
                    system_prompt_extension,
                    images,
                    conversation_history,
                    question_type,
                    response_type,
                    completion_opts.as_ref(),
                )
                .await
            },
        )
        .await
    }

    /// Inner answer generation (called under `rag.generation` span).
    #[allow(clippy::too_many_arguments)]
    async fn generate_answer_inner(
        &self,
        query: &str,
        context: &QueryContext,
        provider: &dyn crate::LLMProvider,
        system_prompt_extension: Option<&str>,
        images: Option<&[ImageData]>,
        conversation_history: &[ConversationMessage],
        question_type: Option<&str>,
        response_type: Option<&str>,
        opts_ref: Option<&CompletionOptions>,
    ) -> Result<(String, usize)> {
        let prompt = self.build_prompt(
            query,
            context,
            system_prompt_extension,
            conversation_history,
            question_type,
            response_type,
        );
        let system_text = self.build_system_prompt(
            context,
            system_prompt_extension,
            conversation_history,
            question_type,
            response_type,
        );
        let use_complete_blob = Self::answer_complete_blob_enabled();
        let chat_opts = opts_ref
            .cloned()
            .unwrap_or_default()
            .with_provider_prompt_cache("query", provider.name(), provider.model());

        // LAW-145-1: observation input = actual LLM text (full prompt / chat turns).
        let (response, llm_input) = if let Some(imgs) = images.filter(|i| !i.is_empty()) {
            let system_text = self.build_vision_system_message(context, system_prompt_extension);
            let user_text = conversation_context::query_with_conversation_context(
                query,
                conversation_history,
                DEFAULT_CONVERSATION_TURN_LIMIT,
            );
            let (sys, user) = answer_chat_parts(&system_text, &user_text);
            let messages = vec![
                ChatMessage::system(&sys),
                ChatMessage::user_with_images(&user, imgs.to_vec()),
            ];
            let chat_input = conversation_context::format_chat_messages_for_observation(&messages);
            match provider.chat(&messages, Some(&chat_opts)).await {
                Ok(r) => (r, chat_input),
                Err(e) => {
                    tracing::warn!(error = %e, "Vision chat failed; retrying as text-only query");
                    (
                        complete_with_optional_effort(provider, &prompt, opts_ref).await?,
                        prompt.clone(),
                    )
                }
            }
        } else if use_complete_blob {
            (
                complete_with_optional_effort(provider, &prompt, opts_ref).await?,
                prompt.clone(),
            )
        } else {
            let messages = answer_chat_messages(&system_text, query);
            let chat_input = conversation_context::format_chat_messages_for_observation(&messages);
            match provider.chat(&messages, Some(&chat_opts)).await {
                Ok(r) => (r, chat_input),
                Err(e) => {
                    tracing::warn!(
                        error = %e,
                        "083 chat generate failed; falling back to complete blob"
                    );
                    (
                        complete_with_optional_effort(provider, &prompt, opts_ref).await?,
                        prompt.clone(),
                    )
                }
            }
        };

        let gold_compat = crate::grounding::is_gold_answer_extension(system_prompt_extension);

        // 080 D5 / R5: never emit empty when Mix admitted context (LightRAG fail_response only when empty KG).
        let content = response.content.trim().to_string();
        if content.is_empty() {
            tracing::warn!("080 empty LLM answer with non-empty context — retry once");
            let (retry, retry_input) = if use_complete_blob {
                (
                    complete_with_optional_effort(provider, &prompt, opts_ref).await?,
                    prompt.clone(),
                )
            } else {
                let messages = answer_chat_messages(&system_text, query);
                let chat_input =
                    conversation_context::format_chat_messages_for_observation(&messages);
                match provider.chat(&messages, Some(&chat_opts)).await {
                    Ok(r) => (r, chat_input),
                    Err(_) => (
                        complete_with_optional_effort(provider, &prompt, opts_ref).await?,
                        prompt.clone(),
                    ),
                }
            };
            let retry_content = retry.content.trim().to_string();
            if !retry_content.is_empty() {
                record_answer_gen_ai(&retry, &retry_input, &retry_content);
                return Ok((
                    finalize_answer_text(retry_content, gold_compat),
                    retry.completion_tokens,
                ));
            }
            let fallback = extractive_fallback_answer(context, gold_compat);
            tracing::warn!(
                fallback_chars = fallback.len(),
                gold_compat,
                "080 empty LLM after retry — extractive fallback from context"
            );
            edgequake_observability::record_observation_io(Some(&llm_input), Some(&fallback));
            return Ok((finalize_answer_text(fallback, gold_compat), 0));
        }

        // 081 F4: opt-in groundedness retry (EDGEQUAKE_ANSWER_GROUNDED_RETRY=1).
        // Mid gate T022412Z Acc-regressed vs E2-B5 → default OFF; Acc prompt unchanged.
        if !context.is_empty() && crate::grounding::grounded_retry_enabled() {
            let corpus = context_corpus_for_span_check(context);
            if crate::grounding::needs_groundedness_retry(&content, &corpus) {
                tracing::warn!(
                    coverage = crate::grounding::answer_context_token_coverage(&content, &corpus),
                    "081 F4 low answer↔context coverage — groundedness retry once"
                );
                let reinforced = format!(
                    "{prompt}\n\n---Additional Grounding---\n\
Rewrite so the answer includes at least one contiguous phrase (3+ words) copied \
exactly from a Document Chunk (or entity description) that supports the claim, \
then elaborate. Do not invent facts outside Context.\n"
                );
                if let Ok(retry) = provider.complete(&reinforced).await {
                    let retry_content = retry.content.trim().to_string();
                    if !retry_content.is_empty()
                        && (crate::grounding::answer_context_token_coverage(
                            &retry_content,
                            &corpus,
                        ) > crate::grounding::answer_context_token_coverage(&content, &corpus)
                            || crate::grounding::answer_has_context_span(&retry_content, &corpus))
                    {
                        record_answer_gen_ai(&retry, &reinforced, &retry_content);
                        return Ok((
                            finalize_answer_text(retry_content, gold_compat),
                            retry.completion_tokens,
                        ));
                    }
                }
            }
        }

        record_answer_gen_ai(&response, &llm_input, &content);
        Ok((
            finalize_answer_text(content, gold_compat),
            response.completion_tokens,
        ))
    }

    /// Generate a *direct* LLM chatbot answer with no retrieval context (P-G8 / RC-13).
    ///
    /// WHY (First Principles): Bypass / Chat mode means "skip retrieval, talk to
    /// the LLM like a chatbot" — the opposite of RAG. The RAG answer path guards
    /// on `context.is_empty()` and returns the *apology* string for a real
    /// retrieval miss, which is correct for Local/Global/Hybrid/Naive but wrong
    /// for Bypass, where an empty context is *intentional*.
    ///
    /// Message shape (DRY with `conversation_context::build_bypass_chat_messages`):
    /// `[system persona (+ optional extension)] + cut(history) + [current user]`.
    /// History uses the shared sliding-window cut (`DEFAULT_CONVERSATION_TURN_LIMIT`).
    ///
    /// E23: an empty/whitespace query still reaches the LLM; the provider is
    /// responsible for its own handling.
    pub(super) async fn generate_bypass_answer(
        &self,
        query: &str,
        llm_override: Option<&Arc<dyn crate::LLMProvider>>,
        system_prompt_extension: Option<&str>,
        images: Option<&[ImageData]>,
        conversation_history: &[ConversationMessage],
    ) -> Result<(String, usize)> {
        let provider = llm_override.unwrap_or(&self.llm_provider);
        let model = provider.model().to_string();
        let provider_name = provider.name().to_string();
        edgequake_observability::with_rag_generation_span(
            "generate-bypass-answer",
            &model,
            &provider_name,
            async {
                let messages = conversation_context::build_bypass_chat_messages(
                    query,
                    conversation_history,
                    system_prompt_extension,
                    DEFAULT_CONVERSATION_TURN_LIMIT,
                    images,
                );

                let bypass_opts =
                    CompletionOptions::default().with_role_cache("bypass", provider.as_ref());
                let response = match provider.chat(&messages, Some(&bypass_opts)).await {
                    Ok(r) => (r, conversation_context::format_chat_messages_for_observation(&messages)),
                    Err(e) if images.is_some_and(|i| !i.is_empty()) => {
                        tracing::warn!(error = %e, "Bypass vision chat failed; retrying as text-only");
                        let text_only = conversation_context::build_bypass_chat_messages(
                            query,
                            conversation_history,
                            system_prompt_extension,
                            DEFAULT_CONVERSATION_TURN_LIMIT,
                            None,
                        );
                        let llm_input =
                            conversation_context::format_chat_messages_for_observation(&text_only);
                        (provider.chat(&text_only, Some(&bypass_opts)).await?, llm_input)
                    }
                    Err(e) => return Err(e.into()),
                };
                let (response, llm_input) = response;

                record_answer_gen_ai(&response, &llm_input, &response.content);
                Ok((response.content, response.completion_tokens))
            },
        )
        .await
    }

    /// Stream Bypass / Chat as real tokens when the provider supports `stream()`.
    ///
    /// WHY (2026): Chat UX expects incremental tokens. Providers expose role-aware
    /// `chat()` and text `stream()`. We build the same chatbot messages as sync,
    /// flatten to a role-labeled prompt (DRY with `format_bypass_messages_as_prompt`),
    /// then stream — falling back to one-shot chat if streaming is unsupported.
    pub(super) async fn stream_bypass_answer(
        &self,
        query: &str,
        llm_override: Option<Arc<dyn crate::LLMProvider>>,
        system_prompt_extension: Option<&str>,
        images: Option<&[ImageData]>,
        conversation_history: &[ConversationMessage],
    ) -> Result<TokenStream> {
        let provider = llm_override.unwrap_or_else(|| self.llm_provider.clone());
        let messages = conversation_context::build_bypass_chat_messages(
            query,
            conversation_history,
            system_prompt_extension,
            DEFAULT_CONVERSATION_TURN_LIMIT,
            images.filter(|i| !i.is_empty()),
        );

        if images.is_some_and(|i| !i.is_empty()) || !provider.supports_streaming() {
            let (answer, _) = self
                .generate_bypass_answer(
                    query,
                    Some(&provider),
                    system_prompt_extension,
                    images,
                    conversation_history,
                )
                .await?;
            return Ok(futures::stream::once(async move { Ok(answer) }).boxed());
        }

        let prompt = conversation_context::format_bypass_messages_as_prompt(&messages);
        let model = provider.model().to_string();
        let provider_name = provider.name().to_string();
        let raw = provider
            .stream(&prompt)
            .await
            .map_err(crate::error::QueryError::from)?;
        Ok(futures::StreamExt::boxed(
            edgequake_observability::instrument_generation_token_stream(
                "generate-bypass-answer",
                &model,
                &provider_name,
                prompt,
                raw.map(|res| res.map_err(crate::error::QueryError::from)),
            ),
        ))
    }

    /// Stream a vision (image-attached) answer (P-G11 / RC-16).
    ///
    /// WHY (First Principles): the `LLMProvider::stream` trait method takes only
    /// a text prompt — it cannot carry images. The vision-capable path is
    /// `provider.chat()` with image attachments (FEAT0203). So streaming vision
    /// parity means: when images are attached, run the vision `chat` call and
    /// emit its result as a one-shot token stream. This keeps the streaming
    /// entry's contract (a `Stream<Item = Result<String>>`) while using the
    /// vision path — the same trade-off the sync path already makes.
    ///
    /// E30: if the vision chat fails (e.g., vision LLM unavailable), fall back
    /// to the text-only `stream`/`complete` path — identical to
    /// `generate_answer_with_provider`'s image fallback.
    pub(super) async fn stream_vision_answer(
        &self,
        query: &str,
        context: &QueryContext,
        llm_override: Option<Arc<dyn crate::LLMProvider>>,
        system_prompt_extension: Option<&str>,
        images: &[ImageData],
    ) -> Result<TokenStream> {
        let provider = llm_override.unwrap_or_else(|| self.llm_provider.clone());
        let model = provider.model().to_string();
        let provider_name = provider.name().to_string();
        edgequake_observability::with_rag_generation_span(
            "generate-answer",
            &model,
            &provider_name,
            async {
                let system_text =
                    self.build_vision_system_message(context, system_prompt_extension);
                let (sys, user) = answer_chat_parts(&system_text, query);
                let chat_opts = CompletionOptions::default().with_provider_prompt_cache(
                    "vision",
                    provider.name(),
                    provider.model(),
                );
                let messages = vec![
                    ChatMessage::system(&sys),
                    ChatMessage::user_with_images(&user, images.to_vec()),
                ];

                match provider.chat(&messages, Some(&chat_opts)).await {
                    Ok(r) => {
                        let llm_input =
                            conversation_context::format_chat_messages_for_observation(&messages);
                        record_answer_gen_ai(&r, &llm_input, &r.content);
                        Ok(futures::stream::once(async move { Ok(r.content) }).boxed())
                    }
                    Err(e) => {
                        tracing::warn!(
                            error = %e,
                            "Streaming vision chat failed; falling back to text-only stream"
                        );
                        let prompt = self.build_prompt(
                            query,
                            context,
                            system_prompt_extension,
                            &[],
                            None,
                            None,
                        );
                        if provider.supports_streaming() {
                            // Span stays open until tokens finish (LAW-145-9).
                            let raw = provider
                                .stream(&prompt)
                                .await
                                .map_err(crate::error::QueryError::from)?;
                            Ok(futures::StreamExt::boxed(
                                edgequake_observability::instrument_generation_token_stream(
                                    "generate-answer",
                                    provider.model(),
                                    provider.name(),
                                    prompt,
                                    raw.map(|res| res.map_err(crate::error::QueryError::from)),
                                ),
                            ))
                        } else {
                            let resp = provider
                                .complete(&prompt)
                                .await
                                .map_err(crate::error::QueryError::from)?;
                            record_answer_gen_ai(&resp, &prompt, &resp.content);
                            Ok(futures::stream::once(async move { Ok(resp.content) }).boxed())
                        }
                    }
                }
            },
        )
        .await
    }
}

#[cfg(test)]
mod prefix_cache_tests {
    use super::answer_chat_parts;

    #[test]
    fn splits_context_into_user_when_prompt_cache_on() {
        let system = "---Role---\nYou are helpful.\n---Context---\nENTITIES: X";
        let (sys, user) = answer_chat_parts(system, "What is X?");
        if edgequake_llm::provider_prompt_cache_enabled() {
            assert!(!sys.contains("ENTITIES: X"));
            assert!(user.contains("ENTITIES: X"));
            assert!(user.contains("What is X?"));
        } else {
            assert_eq!(sys, system);
            assert_eq!(user, "What is X?");
        }
    }
}
