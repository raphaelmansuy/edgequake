//! Entity and relationship extraction via LLM.
//!
//! @implements FEAT0003
//! @implements FEAT0004
//! @implements FEAT0304
//!
//! # Implements
//!
//! - **FEAT0003**: Entity Extraction
//! - **FEAT0004**: Relationship Extraction
//! - **FEAT0304**: Gleaning (iterative re-extraction for completeness)
//!
//! # Enforces
//!
//! - **BR0003**: Entity types from configurable list
//! - **BR0004**: Relationship keywords max 5 per edge
//! - **BR0005**: Entity description max 512 tokens
//! - **BR0006**: Same-entity relationships forbidden
//! - **BR0008**: Entity names normalized (UPPERCASE_UNDERSCORE)
//!
//! # WHY: LLM-Based Extraction
//!
//! Using LLMs for extraction provides:
//! 1. Domain-agnostic entity recognition (no training required)
//! 2. Rich semantic descriptions (not just labels)
//! 3. Relationship inference beyond co-occurrence
//!
//! # Extraction Strategies
//!
//! | Strategy | Description | Use Case |
//! |----------|-------------|----------|
//! | [`SOTAExtractor`] | Tuple-based parsing | Production (robust) |
//! | [`SimpleExtractor`] | JSON-based parsing | Development/testing |
//! | [`GleaningExtractor`] | Iterative re-extraction | High-stakes domains |

use async_trait::async_trait;
use edgequake_llm::traits::{ChatMessage, CompletionOptions};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::chunker::TextChunk;
use crate::error::{PipelineError, Result};

/// Result of entity and relationship extraction.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ExtractionResult {
    /// Extracted entities.
    pub entities: Vec<ExtractedEntity>,

    /// Extracted relationships.
    pub relationships: Vec<ExtractedRelationship>,

    /// Source chunk ID.
    pub source_chunk_id: String,

    /// Processing metadata.
    pub metadata: HashMap<String, serde_json::Value>,

    /// Input tokens used for this extraction.
    pub input_tokens: usize,

    /// Output tokens generated for this extraction.
    pub output_tokens: usize,

    /// Extraction time in milliseconds.
    pub extraction_time_ms: u64,
}

impl ExtractionResult {
    /// Create a new empty extraction result.
    pub fn new(source_chunk_id: impl Into<String>) -> Self {
        Self {
            entities: Vec::new(),
            relationships: Vec::new(),
            source_chunk_id: source_chunk_id.into(),
            metadata: HashMap::new(),
            input_tokens: 0,
            output_tokens: 0,
            extraction_time_ms: 0,
        }
    }

    /// Add an entity.
    pub fn add_entity(&mut self, entity: ExtractedEntity) {
        self.entities.push(entity);
    }

    /// Add a relationship.
    pub fn add_relationship(&mut self, rel: ExtractedRelationship) {
        self.relationships.push(rel);
    }

    /// Set token usage information.
    pub fn with_token_usage(mut self, input_tokens: usize, output_tokens: usize) -> Self {
        self.input_tokens = input_tokens;
        self.output_tokens = output_tokens;
        self
    }

    /// Set extraction timing.
    pub fn with_timing(mut self, extraction_time_ms: u64) -> Self {
        self.extraction_time_ms = extraction_time_ms;
        self
    }
}

/// An extracted entity.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractedEntity {
    /// Entity name (normalized).
    pub name: String,

    /// Entity type (e.g., "PERSON", "ORGANIZATION", "CONCEPT").
    pub entity_type: String,

    /// Description of the entity.
    pub description: String,

    /// Importance score (0.0 to 1.0).
    pub importance: f32,

    /// Source text spans.
    pub source_spans: Vec<String>,

    /// Entity embedding.
    pub embedding: Option<Vec<f32>>,

    /// Source chunk IDs where this entity was mentioned.
    /// Used for citation tracking back to original document chunks.
    #[serde(default)]
    pub source_chunk_ids: Vec<String>,

    /// Source document ID (the document this entity was extracted from).
    #[serde(default)]
    pub source_document_id: Option<String>,

    /// Original file path of the source document.
    #[serde(default)]
    pub source_file_path: Option<String>,
}

impl ExtractedEntity {
    /// Create a new extracted entity.
    pub fn new(
        name: impl Into<String>,
        entity_type: impl Into<String>,
        description: impl Into<String>,
    ) -> Self {
        Self {
            name: name.into(),
            entity_type: entity_type.into(),
            description: description.into(),
            importance: 0.5,
            source_spans: Vec::new(),
            embedding: None,
            source_chunk_ids: Vec::new(),
            source_document_id: None,
            source_file_path: None,
        }
    }

    /// Set the importance score.
    pub fn with_importance(mut self, importance: f32) -> Self {
        self.importance = importance.clamp(0.0, 1.0);
        self
    }

    /// Add a source span.
    pub fn with_source_span(mut self, span: impl Into<String>) -> Self {
        self.source_spans.push(span.into());
        self
    }

    /// Add a source chunk ID.
    pub fn with_source_chunk_id(mut self, chunk_id: impl Into<String>) -> Self {
        let id = chunk_id.into();
        if !self.source_chunk_ids.contains(&id) {
            self.source_chunk_ids.push(id);
        }
        self
    }

    /// Set the source document ID.
    pub fn with_source_document_id(mut self, document_id: impl Into<String>) -> Self {
        self.source_document_id = Some(document_id.into());
        self
    }

    /// Set the source file path.
    pub fn with_source_file_path(mut self, file_path: impl Into<String>) -> Self {
        self.source_file_path = Some(file_path.into());
        self
    }

    /// Add source chunk ID (mutable reference version).
    pub fn add_source_chunk_id(&mut self, chunk_id: impl Into<String>) {
        let id = chunk_id.into();
        if !self.source_chunk_ids.contains(&id) {
            self.source_chunk_ids.push(id);
        }
    }
}

/// An extracted relationship between entities.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractedRelationship {
    /// Source entity name.
    pub source: String,

    /// Target entity name.
    pub target: String,

    /// Relationship type/description.
    pub relation_type: String,

    /// Relationship description.
    pub description: String,

    /// Weight/strength (0.0 to 1.0).
    pub weight: f32,

    /// Keywords associated with this relationship.
    pub keywords: Vec<String>,

    /// Relationship embedding (for similarity search).
    /// Computed from: keywords + source + target + description
    pub embedding: Option<Vec<f32>>,

    /// Source chunk ID where this relationship was extracted.
    #[serde(default)]
    pub source_chunk_id: Option<String>,

    /// Source document ID.
    #[serde(default)]
    pub source_document_id: Option<String>,

    /// Original file path of the source document.
    #[serde(default)]
    pub source_file_path: Option<String>,
}

impl ExtractedRelationship {
    /// Create a new extracted relationship.
    pub fn new(
        source: impl Into<String>,
        target: impl Into<String>,
        relation_type: impl Into<String>,
    ) -> Self {
        Self {
            source: source.into(),
            target: target.into(),
            relation_type: relation_type.into(),
            description: String::new(),
            weight: 0.5,
            keywords: Vec::new(),
            embedding: None,
            source_chunk_id: None,
            source_document_id: None,
            source_file_path: None,
        }
    }

    /// Set the description.
    pub fn with_description(mut self, desc: impl Into<String>) -> Self {
        self.description = desc.into();
        self
    }

    /// Set the weight.
    pub fn with_weight(mut self, weight: f32) -> Self {
        self.weight = weight.clamp(0.0, 1.0);
        self
    }

    /// Add keywords.
    pub fn with_keywords(mut self, keywords: Vec<String>) -> Self {
        self.keywords = keywords;
        self
    }

    /// Set the source chunk ID.
    pub fn with_source_chunk_id(mut self, chunk_id: impl Into<String>) -> Self {
        self.source_chunk_id = Some(chunk_id.into());
        self
    }

    /// Set the source document ID.
    pub fn with_source_document_id(mut self, document_id: impl Into<String>) -> Self {
        self.source_document_id = Some(document_id.into());
        self
    }

    /// Set the source file path.
    pub fn with_source_file_path(mut self, file_path: impl Into<String>) -> Self {
        self.source_file_path = Some(file_path.into());
        self
    }
}

/// Trait for entity extraction implementations.
#[async_trait]
/// @implements FEAT0009
pub trait EntityExtractor: Send + Sync {
    /// Extract entities and relationships from a text chunk.
    async fn extract(&self, chunk: &TextChunk) -> Result<ExtractionResult>;

    /// Extract from multiple chunks in batch.
    async fn extract_batch(&self, chunks: &[TextChunk]) -> Result<Vec<ExtractionResult>> {
        let mut results = Vec::with_capacity(chunks.len());
        for chunk in chunks {
            results.push(self.extract(chunk).await?);
        }
        Ok(results)
    }

    /// Get the name of this extractor.
    fn name(&self) -> &str;

    /// Get the model name used by this extractor (if applicable).
    fn model_name(&self) -> &str {
        "unknown"
    }

    /// Get the provider name used by this extractor (if applicable).
    ///
    /// @implements SPEC-032/OODA-226: Provider tracking in ProcessingStats
    fn provider_name(&self) -> &str {
        "unknown"
    }
}

/// Simple regex-based entity extractor for testing.
pub struct SimpleExtractor {
    /// Entity type patterns.
    patterns: HashMap<String, regex::Regex>,
}

impl SimpleExtractor {
    /// Create a new simple extractor with default patterns.
    pub fn new() -> Result<Self> {
        let mut patterns = HashMap::new();

        // Simple patterns for common entity types
        patterns.insert(
            "PERSON".to_string(),
            regex::Regex::new(r"\b[A-Z][a-z]+ [A-Z][a-z]+\b")
                .map_err(|e| PipelineError::ConfigError(e.to_string()))?,
        );

        patterns.insert(
            "ORGANIZATION".to_string(),
            regex::Regex::new(r"\b[A-Z][A-Za-z]+ (?:Inc|Corp|LLC|Ltd|Company)\b")
                .map_err(|e| PipelineError::ConfigError(e.to_string()))?,
        );

        Ok(Self { patterns })
    }
}

impl Default for SimpleExtractor {
    fn default() -> Self {
        Self::new().expect("Failed to create SimpleExtractor")
    }
}

#[async_trait]
impl EntityExtractor for SimpleExtractor {
    async fn extract(&self, chunk: &TextChunk) -> Result<ExtractionResult> {
        let mut result = ExtractionResult::new(&chunk.id);

        for (entity_type, pattern) in &self.patterns {
            for cap in pattern.find_iter(&chunk.content) {
                let name = cap.as_str().to_string();
                let entity =
                    ExtractedEntity::new(&name, entity_type, &name).with_source_span(cap.as_str());
                result.add_entity(entity);
            }
        }

        Ok(result)
    }

    fn name(&self) -> &str {
        "simple"
    }
}

/// LLM-based entity extractor using structured prompts.
///
/// # WHY: LLM Extraction Strategy
///
/// LLM extraction is the core of knowledge graph construction:
///
/// 1. **Structured Prompt** - Uses a carefully designed prompt that:
///    - Lists valid entity types to constrain LLM output
///    - Requests JSON format for reliable parsing
///    - Asks for descriptions to enrich entity/relationship context
///    - WHY JSON: Tuples are faster but JSON is more reliable for complex relationships
///
/// 2. **Entity Type Constraints** - Pre-defined types (PERSON, ORG, LOCATION, etc.)
///    - WHY: Constraining types improves extraction consistency
///    - WHY custom types: Domain-specific extraction (e.g., PROTEIN for biomedical)
///
/// 3. **Relationship Extraction** - Source → Relationship → Target triples
///    - WHY tuples: Graph databases need explicit source/target
///    - WHY descriptions: Context for semantic search
///
/// 4. **Error-Tolerant Parsing** - Handles malformed LLM output
///    - WHY: LLMs occasionally produce invalid JSON; we extract what we can
pub struct LLMExtractor<L>
where
    L: edgequake_llm::LLMProvider + ?Sized,
{
    llm_provider: std::sync::Arc<L>,
    entity_types: Vec<String>,
}

impl<L> LLMExtractor<L>
where
    L: edgequake_llm::LLMProvider + ?Sized,
{
    /// Create a new LLM extractor.
    pub fn new(llm_provider: std::sync::Arc<L>) -> Self {
        Self {
            llm_provider,
            entity_types: vec![
                "PERSON".to_string(),
                "ORGANIZATION".to_string(),
                "LOCATION".to_string(),
                "EVENT".to_string(),
                "CONCEPT".to_string(),
                "TECHNOLOGY".to_string(),
                "PRODUCT".to_string(),
            ],
        }
    }

    /// Create with custom entity types.
    pub fn with_entity_types(mut self, types: Vec<String>) -> Self {
        self.entity_types = types;
        self
    }

    /// Build the extraction prompt.
    fn build_prompt(&self, text: &str) -> String {
        let entity_types_str = self.entity_types.join(", ");

        format!(
            r#"Extract entities and relationships from the following text.

## Entity Types
{entity_types_str}

## Output Format
Respond with valid JSON in this exact format:
{{
  "entities": [
    {{"name": "Entity Name", "type": "ENTITY_TYPE", "description": "Brief description"}}
  ],
  "relationships": [
    {{"source": "Source Entity", "target": "Target Entity", "type": "RELATIONSHIP_TYPE", "description": "Brief description"}}
  ]
}}

## Text to Analyze
{text}

## JSON Response"#
        )
    }

    /// Parse the LLM response into extraction result.
    fn parse_response(&self, response: &str, chunk_id: &str) -> Result<ExtractionResult> {
        let mut result = ExtractionResult::new(chunk_id);

        // Try to extract JSON from the response
        let json_str = extract_json_from_response(response);

        let parsed: serde_json::Value = serde_json::from_str(&json_str)
            .map_err(|e| PipelineError::ExtractionError(format!("Invalid JSON: {}", e)))?;

        // Extract entities
        if let Some(entities) = parsed.get("entities").and_then(|v| v.as_array()) {
            for entity_val in entities {
                if let (Some(name), Some(entity_type), Some(description)) = (
                    entity_val.get("name").and_then(|v| v.as_str()),
                    entity_val.get("type").and_then(|v| v.as_str()),
                    entity_val.get("description").and_then(|v| v.as_str()),
                ) {
                    result.add_entity(ExtractedEntity::new(name, entity_type, description));
                }
            }
        }

        // Extract relationships
        if let Some(relationships) = parsed.get("relationships").and_then(|v| v.as_array()) {
            for rel_val in relationships {
                if let (Some(source), Some(target), Some(rel_type)) = (
                    rel_val.get("source").and_then(|v| v.as_str()),
                    rel_val.get("target").and_then(|v| v.as_str()),
                    rel_val.get("type").and_then(|v| v.as_str()),
                ) {
                    let description = rel_val
                        .get("description")
                        .and_then(|v| v.as_str())
                        .unwrap_or("");

                    result.add_relationship(
                        ExtractedRelationship::new(source, target, rel_type)
                            .with_description(description),
                    );
                }
            }
        }

        Ok(result)
    }
}

#[async_trait]
impl<L> EntityExtractor for LLMExtractor<L>
where
    L: edgequake_llm::LLMProvider + Send + Sync + ?Sized,
{
    async fn extract(&self, chunk: &TextChunk) -> Result<ExtractionResult> {
        let prompt = self.build_prompt(&chunk.content);

        let response = self
            .llm_provider
            .complete(&prompt)
            .await
            .map_err(|e| PipelineError::ExtractionError(format!("LLM error: {}", e)))?;

        let mut result = self.parse_response(&response.content, &chunk.id)?;

        // Set token usage from the LLM response
        result.input_tokens = response.prompt_tokens;
        result.output_tokens = response.completion_tokens;

        Ok(result)
    }

    fn name(&self) -> &str {
        "llm"
    }

    fn model_name(&self) -> &str {
        self.llm_provider.model()
    }

    /// @implements SPEC-032/OODA-226: Provider tracking in ProcessingStats
    fn provider_name(&self) -> &str {
        self.llm_provider.name()
    }
}

/// Extract JSON from a potentially wrapped LLM response.
fn extract_json_from_response(response: &str) -> String {
    let response = response.trim();

    // Try to find JSON block markers
    if let Some(start) = response.find("```json") {
        if let Some(end) = response[start + 7..].find("```") {
            return response[start + 7..start + 7 + end].trim().to_string();
        }
    }

    // Try to find JSON starting with {
    if let Some(start) = response.find('{') {
        if let Some(end) = response.rfind('}') {
            if end > start {
                return response[start..=end].to_string();
            }
        }
    }

    response.to_string()
}

/// SOTA LLM-based entity extractor using tuple-format prompts.
///
/// @implements FEAT0303
///
/// This extractor uses the SOTA prompt system ported from LightRAG,
/// featuring tuple-based output format for more robust parsing.
pub struct SOTAExtractor<L>
where
    L: edgequake_llm::LLMProvider + ?Sized,
{
    llm_provider: std::sync::Arc<L>,
    entity_types: Vec<String>,
    prompts: crate::prompts::EntityExtractionPrompts,
    parser: crate::prompts::HybridExtractionParser,
    language: String,
}

impl<L> SOTAExtractor<L>
where
    L: edgequake_llm::LLMProvider + ?Sized,
{
    /// Create a new SOTA extractor with default settings.
    pub fn new(llm_provider: std::sync::Arc<L>) -> Self {
        Self {
            llm_provider,
            entity_types: crate::prompts::default_entity_types(),
            prompts: crate::prompts::EntityExtractionPrompts::default(),
            parser: crate::prompts::HybridExtractionParser::new(true),
            language: "English".to_string(),
        }
    }

    /// Set custom entity types.
    pub fn with_entity_types(mut self, types: Vec<String>) -> Self {
        self.entity_types = types;
        self
    }

    /// Set output language.
    pub fn with_language(mut self, language: impl Into<String>) -> Self {
        self.language = language.into();
        self
    }

    /// Set custom prompts.
    pub fn with_prompts(mut self, prompts: crate::prompts::EntityExtractionPrompts) -> Self {
        self.prompts = prompts;
        self
    }
}

#[async_trait]
impl<L> EntityExtractor for SOTAExtractor<L>
where
    L: edgequake_llm::LLMProvider + Send + Sync + ?Sized,
{
    async fn extract(&self, chunk: &TextChunk) -> Result<ExtractionResult> {
        let start = std::time::Instant::now();

        // Pre-validate chunk size to fail fast on oversized chunks
        // WHY: Large chunks (>1500 tokens ~6KB) likely exceed LLM timeout (120s)
        // Based on LightRAG research: 1200-1500 tokens is optimal for reliability
        // This prevents wasting API calls and provides immediate, actionable feedback
        let chunk_size_bytes = chunk.content.len();
        let estimated_tokens = chunk_size_bytes / 4; // Rough estimate: 1 token ≈ 4 chars
        const MAX_CHUNK_TOKENS: usize = 1500; // Reduced from 4000 based on research

        if estimated_tokens > MAX_CHUNK_TOKENS {
            // Calculate adaptive chunk size based on estimated document size
            let recommended_chunk_size = if chunk_size_bytes > 100_000 {
                600 // >100KB: minimal chunks
            } else if chunk_size_bytes > 50_000 {
                800 // 50-100KB: reduced chunks
            } else {
                1200 // <50KB: standard chunks
            };

            let error_msg = format!(
                "Chunk too large for LLM processing. Chunk size: {}KB (~{} tokens, max: {}). \
                Suggestions:\n\
                1. Use adaptive chunking: Set chunk_size={} for this document size\n\
                2. Split document into smaller files (<50KB each)\n\
                3. Current config uses chunk_size=1200 which is too large for {}KB documents\n\
                4. Alternative: Use Ollama with 300s timeout instead of OpenAI (120s)\n\
                Chunk ID: {} | Document size: ~{}KB",
                chunk_size_bytes / 1024,
                estimated_tokens,
                MAX_CHUNK_TOKENS,
                recommended_chunk_size,
                chunk_size_bytes / 1024,
                chunk.id,
                chunk_size_bytes / 1024
            );
            tracing::error!(
                chunk_id = %chunk.id,
                chunk_size_bytes = chunk_size_bytes,
                estimated_tokens = estimated_tokens,
                max_tokens = MAX_CHUNK_TOKENS,
                "{}",
                error_msg
            );
            return Err(PipelineError::Validation(error_msg));
        }

        // Build system and user prompts
        let system_prompt = self
            .prompts
            .system_prompt(&self.entity_types, &self.language);
        let user_prompt =
            self.prompts
                .user_prompt(&chunk.content, &self.entity_types, &self.language);

        // Create chat messages for system + user prompt
        let messages = vec![
            ChatMessage::system(system_prompt),
            ChatMessage::user(user_prompt),
        ];

        // ═══════════════════════════════════════════════════════════════════════════════
        // WHY DO WE NEED ADAPTIVE MAX_TOKENS? (INPUT vs OUTPUT Token Management)
        // ═══════════════════════════════════════════════════════════════════════════════
        //
        // PROBLEM: Document chunking (INPUT) ≠ Response size management (OUTPUT)
        //
        // ┌─────────────────────────────────────────────────────────────────────────┐
        // │ INPUT SIDE (Document Chunking) - Already Working ✅                      │
        // └─────────────────────────────────────────────────────────────────────────┘
        //
        //   137KB Document (1234 lines)
        //         │
        //         ├─> Chunk 1: ~5KB (~1200 tokens) ──┐
        //         ├─> Chunk 2: ~5KB (~1200 tokens) ──┤
        //         ├─> Chunk 3: ~5KB (~1200 tokens) ──┤─> Process each chunk
        //         ├─> Chunk 4: ~5KB (~1200 tokens) ──┤   separately with LLM
        //         └─> Chunk 5: ~5KB (~1200 tokens) ──┘
        //
        //   INPUT chunking prevents LLM from being overwhelmed by large documents.
        //   Orchestrator uses adaptive INPUT chunk sizes: 600-1200 tokens based on doc size.
        //
        // ┌─────────────────────────────────────────────────────────────────────────┐
        // │ OUTPUT SIDE (LLM Response) - The ACTUAL Problem We Fix Here! ❌ → ✅     │
        // └─────────────────────────────────────────────────────────────────────────┘
        //
        //   Single 5KB Chunk (~1500 tokens INPUT)
        //         │
        //         ├─> LLM Entity Extraction
        //         │
        //         └─> JSON Response (OUTPUT): {
        //               "entities": [
        //                 {"name": "SARAH_CHEN", "type": "PERSON", ...},
        //                 {"name": "NEURAL_NETWORK", "type": "CONCEPT", ...},
        //                 {"name": "GRADIENT_DESCENT", "type": "METHOD", ...},
        //                 ... 50+ more entities ...
        //               ],
        //               "relationships": [
        //                 {"source": "SARAH_CHEN", "target": "NEURAL_NETWORK", ...},
        //                 ... 100+ more relationships ...
        //               ]
        //             }
        //
        //   OUTPUT Response Size: ~9,000 tokens (6x larger than INPUT!)
        //                         ▲
        //                         │
        //                 EXCEEDED 8192 TOKEN LIMIT!
        //                         │
        //   Result: JSON truncated mid-array → "EOF while parsing a list at line 984"
        //
        // ┌─────────────────────────────────────────────────────────────────────────┐
        // │ KEY INSIGHT: Small INPUT ≠ Small OUTPUT                                  │
        // └─────────────────────────────────────────────────────────────────────────┘
        //
        //   • Academic papers/technical docs have HIGH entity density
        //   • Single 1500-token chunk can generate 50+ entities + 100+ relationships
        //   • Complex JSON structure multiplies output size (IDs, types, descriptions)
        //   • SafetyLimitedProvider DEFAULT_MAX_TOKENS=8192 was TOO LOW for OUTPUT
        //
        // ┌─────────────────────────────────────────────────────────────────────────┐
        // │ SOLUTION: Adaptive max_tokens for OUTPUT Responses                      │
        // └─────────────────────────────────────────────────────────────────────────┘
        //
        //   We calculate base_max_tokens based on CHUNK COMPLEXITY (not just size):
        //
        //     <25KB chunks  → 4096 tokens  (simple content, few entities)
        //     25-75KB       → 8192 tokens  (medium complexity)
        //     75-125KB      → 12288 tokens (high entity density)
        //     >125KB        → 16384 tokens (very complex, many entities)
        //
        //   PLUS progressive retry logic:
        //   • Detect truncation via finish_reason="length" or JSON parse errors
        //   • Double max_tokens on retry (up to 32768 maximum)
        //   • Retry up to 3 times with exponential backoff
        //
        //   Result: Adaptive OUTPUT limits match ACTUAL response complexity!
        //
        // ═══════════════════════════════════════════════════════════════════════════

        let base_max_tokens = if chunk_size_bytes < 25_000 {
            4096 // <25KB: small documents, likely simple content
        } else if chunk_size_bytes < 75_000 {
            8192 // 25-75KB: medium documents, moderate entity density
        } else if chunk_size_bytes < 125_000 {
            12288 // 75-125KB: large documents, high entity density
        } else {
            16384 // >125KB: very large documents, very high entity density
        };

        // ═══════════════════════════════════════════════════════════════════════════════
        // RETRY STRATEGY: Progressive max_tokens Increase on Truncation Detection
        // ═══════════════════════════════════════════════════════════════════════════════
        //
        // ┌─────────────────────────────────────────────────────────────────────────┐
        // │ Adaptive Retry Flow (Exponential Backoff + Progressive Token Increase)   │
        // └─────────────────────────────────────────────────────────────────────────┘
        //
        //   Attempt 1: base_max_tokens (e.g., 8192)
        //      │
        //      ├─> LLM Call
        //      │
        //      ├─> Check finish_reason
        //      │   │
        //      │   ├─> "stop" → Success ✅ Parse JSON
        //      │   │
        //      │   └─> "length" → Truncated! ❌
        //      │       │
        //      │       └─> Sleep 100ms, Retry with 16384 tokens
        //      │
        //   Attempt 2: 2x tokens (16384)
        //      │
        //      ├─> LLM Call
        //      │
        //      ├─> Parse JSON
        //      │   │
        //      │   ├─> Success ✅ Return result
        //      │   │
        //      │   └─> "EOF while parsing" ❌
        //      │       │
        //      │       └─> Detected JSON truncation → Sleep 200ms, Retry with 32768
        //      │
        //   Attempt 3: 4x tokens (32768 MAX)
        //      │
        //      ├─> LLM Call
        //      │
        //      ├─> Parse JSON
        //      │   │
        //      │   ├─> Success ✅ Return result
        //      │   │
        //      │   └─> Still truncated ❌
        //      │       │
        //      │       └─> ERROR: Document too complex, suggest splitting
        //
        // WHY PROGRESSIVE INCREASE?
        // • Start conservative to save API costs (smaller tokens = cheaper)
        // • Only increase when PROVEN necessary (truncation detected)
        // • Cap at 32768 to prevent runaway costs
        // • Exponential backoff (100ms → 200ms → 400ms) prevents API rate limits
        //
        // TRUNCATION DETECTION:
        // 1. finish_reason="length" → LLM hit max_tokens limit (lines 807-830)
        // 2. JSON parse errors (EOF, unclosed) → Response truncated mid-JSON (lines 897-928)
        //
        // ═══════════════════════════════════════════════════════════════════════════

        const MAX_RETRIES: u32 = 3;
        let mut last_error = None;
        let mut current_max_tokens = base_max_tokens;

        for attempt in 1..=MAX_RETRIES {
            // Create completion options with adaptive max_tokens
            let options = CompletionOptions {
                max_tokens: Some(current_max_tokens),
                temperature: Some(0.0), // Deterministic for extraction
                ..Default::default()
            };

            tracing::debug!(
                attempt = attempt,
                chunk_id = %chunk.id,
                chunk_size_kb = chunk_size_bytes / 1024,
                max_tokens = current_max_tokens,
                "Making LLM call with adaptive max_tokens"
            );

            // Make LLM call using chat interface with options
            let response = match self.llm_provider.chat(&messages, Some(&options)).await {
                Ok(resp) => resp,
                Err(e) => {
                    let error_str = e.to_string().to_lowercase();
                    let is_timeout =
                        error_str.contains("timeout") || error_str.contains("timed out");

                    tracing::warn!(
                        attempt = attempt,
                        max_retries = MAX_RETRIES,
                        error = %e,
                        chunk_id = %chunk.id,
                        chunk_size_kb = chunk_size_bytes / 1024,
                        estimated_tokens = estimated_tokens,
                        is_timeout = is_timeout,
                        "LLM call failed, retrying..."
                    );

                    // Build enhanced error message with diagnostic info
                    let enhanced_error = if is_timeout {
                        // Calculate adaptive chunk size recommendation
                        let recommended_chunk_size = if chunk_size_bytes > 100_000 {
                            600 // >100KB: minimal chunks
                        } else if chunk_size_bytes > 50_000 {
                            800 // 50-100KB: reduced chunks
                        } else {
                            1200 // <50KB: standard chunks
                        };

                        format!(
                            "LLM timeout after 120s. Chunk: {}KB (~{} tokens, max: {}). \
                            Document appears too large for current settings. \
                            Suggestions:\n\
                            1. BEST: Use adaptive chunking with chunk_size={} (recommended for {}KB documents)\n\
                            2. Split document into smaller files (<50KB each)\n\
                            3. Switch to Ollama provider (300s timeout vs OpenAI 120s)\n\
                            4. Alternative: Increase LLM_TIMEOUT_SECS env variable (not recommended)\n\
                            Chunk ID: {} | Attempt: {}/{} | Document size: ~{}KB | Original error: {}",
                            chunk_size_bytes / 1024,
                            estimated_tokens,
                            MAX_CHUNK_TOKENS,
                            recommended_chunk_size,
                            chunk_size_bytes / 1024,
                            chunk.id,
                            attempt,
                            MAX_RETRIES,
                            chunk_size_bytes / 1024,
                            e
                        )
                    } else {
                        format!("LLM error: {}", e)
                    };

                    last_error = Some(PipelineError::ExtractionError(enhanced_error));
                    if attempt < MAX_RETRIES {
                        tokio::time::sleep(tokio::time::Duration::from_millis(
                            100 * 2_u64.pow(attempt - 1),
                        ))
                        .await;
                        continue;
                    } else {
                        return Err(last_error.unwrap());
                    }
                }
            };

            // DEBUG: Log raw LLM response for debugging JSON parsing errors
            tracing::debug!(
                chunk_id = %chunk.id,
                attempt = attempt,
                response_len = response.content.len(),
                response_preview = %&response.content[..response.content.len().min(500)],
                finish_reason = ?response.finish_reason,
                "Raw LLM response received"
            );

            // CRITICAL: Validate response is not empty
            // WHY: Empty LLM responses cause "Invalid JSON: expected value at line 1 column 1" errors
            // This provides actionable error message instead of cryptic JSON parse errors
            let trimmed_response = response.content.trim();
            if trimmed_response.is_empty() {
                let error_msg = format!(
                    "LLM returned EMPTY response. Chunk: {}KB (~{} tokens). \
                    This usually indicates:\n\
                    1. LLM timeout (check Ollama logs: journalctl -u ollama -f)\n\
                    2. Model crashed or OOM (check ollama ps)\n\
                    3. Context window exhausted (reduce chunk_size)\n\
                    4. Network issue with Ollama server\n\
                    Chunk ID: {} | Attempt: {}/{} | Prompt tokens: {}",
                    chunk_size_bytes / 1024,
                    estimated_tokens,
                    chunk.id,
                    attempt,
                    MAX_RETRIES,
                    response.prompt_tokens
                );
                tracing::error!(
                    chunk_id = %chunk.id,
                    attempt = attempt,
                    prompt_tokens = response.prompt_tokens,
                    completion_tokens = response.completion_tokens,
                    "{}",
                    error_msg
                );
                last_error = Some(PipelineError::ExtractionError(error_msg));
                if attempt < MAX_RETRIES {
                    tokio::time::sleep(tokio::time::Duration::from_millis(
                        100 * 2_u64.pow(attempt - 1),
                    ))
                    .await;
                    continue;
                } else {
                    return Err(last_error.unwrap());
                }
            }

            // Check if response was truncated (finish_reason = "length")
            let was_truncated = response
                .finish_reason
                .as_ref()
                .map(|r| r.contains("length"))
                .unwrap_or(false);

            if was_truncated {
                tracing::warn!(
                    attempt = attempt,
                    chunk_id = %chunk.id,
                    current_max_tokens = current_max_tokens,
                    completion_tokens = response.completion_tokens,
                    "LLM response truncated (finish_reason=length), will retry with higher limit"
                );

                // Double max_tokens for retry (up to 32768 max)
                if attempt < MAX_RETRIES && current_max_tokens < 32768 {
                    current_max_tokens = (current_max_tokens * 2).min(32768);
                    last_error = Some(PipelineError::ExtractionError(format!(
                        "JSON truncated at {} tokens. Retrying with {} tokens (attempt {}/{})",
                        response.completion_tokens,
                        current_max_tokens,
                        attempt + 1,
                        MAX_RETRIES
                    )));
                    tokio::time::sleep(tokio::time::Duration::from_millis(
                        100 * 2_u64.pow(attempt - 1),
                    ))
                    .await;
                    continue;
                } else {
                    return Err(PipelineError::ExtractionError(format!(
                        "JSON still truncated after {} retries. Max tokens reached: {}. \
                        Document may be too large for entity extraction. \
                        Suggestion: Split document into smaller chunks or use a model with larger context window.",
                        MAX_RETRIES,
                        current_max_tokens
                    )));
                }
            }

            // Parse response using hybrid parser (with built-in fallbacks)
            match self.parser.parse(&response.content, &chunk.id) {
                Ok(mut result) => {
                    // Add token usage from response
                    result.input_tokens = response.prompt_tokens;
                    result.output_tokens = response.completion_tokens;
                    result.extraction_time_ms = start.elapsed().as_millis() as u64;

                    // Add source chunk line info to metadata
                    result
                        .metadata
                        .insert("extractor".to_string(), serde_json::json!("sota"));
                    result
                        .metadata
                        .insert("language".to_string(), serde_json::json!(self.language));
                    result
                        .metadata
                        .insert("model".to_string(), serde_json::json!(response.model));
                    result
                        .metadata
                        .insert("parse_attempts".to_string(), serde_json::json!(attempt));
                    result.metadata.insert(
                        "max_tokens_used".to_string(),
                        serde_json::json!(current_max_tokens),
                    );

                    if attempt > 1 {
                        tracing::info!(
                            attempt = attempt,
                            chunk_id = %chunk.id,
                            entities = result.entities.len(),
                            "Extraction succeeded after retry"
                        );
                    }

                    return Ok(result);
                }
                Err(e) => {
                    let error_msg = e.to_string().to_lowercase();
                    let is_json_truncation = error_msg.contains("eof")
                        || error_msg.contains("unexpected end")
                        || error_msg.contains("unclosed");

                    tracing::warn!(
                        attempt = attempt,
                        max_retries = MAX_RETRIES,
                        error = %e,
                        chunk_id = %chunk.id,
                        is_json_truncation = is_json_truncation,
                        current_max_tokens = current_max_tokens,
                        response_preview = %&response.content[..response.content.len().min(200)],
                        "Parsing failed - malformed LLM response"
                    );

                    // If likely JSON truncation, increase max_tokens and retry
                    if is_json_truncation && attempt < MAX_RETRIES && current_max_tokens < 32768 {
                        current_max_tokens = (current_max_tokens * 2).min(32768);
                        tracing::info!(
                            chunk_id = %chunk.id,
                            attempt = attempt + 1,
                            max_retries = MAX_RETRIES,
                            new_max_tokens = current_max_tokens,
                            "Detected JSON truncation, increasing max_tokens and retrying"
                        );
                        last_error = Some(e);
                        tokio::time::sleep(tokio::time::Duration::from_millis(
                            100 * 2_u64.pow(attempt - 1),
                        ))
                        .await;
                        continue;
                    }

                    last_error = Some(e);

                    if attempt < MAX_RETRIES {
                        // Exponential backoff: 100ms, 200ms, 400ms
                        tokio::time::sleep(tokio::time::Duration::from_millis(
                            100 * 2_u64.pow(attempt - 1),
                        ))
                        .await;
                        continue;
                    }
                }
            }
        }

        // All retries exhausted
        Err(last_error.unwrap_or_else(|| {
            PipelineError::ExtractionError("Unknown extraction error after retries".to_string())
        }))
    }

    fn name(&self) -> &str {
        "sota"
    }

    fn model_name(&self) -> &str {
        self.llm_provider.model()
    }
}

/// Configuration for gleaning (re-extraction).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GleaningConfig {
    /// Maximum number of gleaning iterations.
    pub max_gleaning: usize,
    /// Whether to continue extraction even if first pass finds entities.
    pub always_glean: bool,
}

impl Default for GleaningConfig {
    fn default() -> Self {
        Self {
            max_gleaning: 1, // LightRAG default
            always_glean: false,
        }
    }
}

/// A wrapper extractor that performs gleaning (re-extraction) to find missed entities.
///
/// # WHY: Multi-Pass Extraction (Gleaning)
///
/// LLMs often miss entities in a single pass due to:
/// - Attention limits on long texts
/// - Implicit entities (e.g., "the company" referring to earlier-mentioned "Apple")
/// - Context overload when many entities are present
///
/// **Gleaning Strategy:**
/// 1. First pass: Normal extraction with base extractor
/// 2. Subsequent passes: Re-prompt LLM with "What did you miss?"
///    - Include previously-found entities to avoid duplicates
///    - Focus on implicit/indirect entity mentions
///
/// **LightRAG Research Finding:**
/// - 1-2 gleaning iterations improve recall by 15-25%
/// - Diminishing returns after 2 iterations
/// - Cost: Each iteration = 1 additional LLM call
///
/// This implements GAP-018: Max Gleaning from LightRAG.
/// @implements FEAT0305 (GleaningExtractor)
pub struct GleaningExtractor {
    /// The underlying LLM provider.
    llm_provider: std::sync::Arc<dyn edgequake_llm::LLMProvider>,
    /// The base extractor to use.
    base_extractor: std::sync::Arc<dyn EntityExtractor>,
    /// Gleaning configuration.
    config: GleaningConfig,
}

impl GleaningExtractor {
    /// Create a new gleaning extractor.
    pub fn new(
        llm_provider: std::sync::Arc<dyn edgequake_llm::LLMProvider>,
        base_extractor: std::sync::Arc<dyn EntityExtractor>,
    ) -> Self {
        Self {
            llm_provider,
            base_extractor,
            config: GleaningConfig::default(),
        }
    }

    /// Set the gleaning configuration.
    pub fn with_config(mut self, config: GleaningConfig) -> Self {
        self.config = config;
        self
    }

    /// Set the maximum gleaning iterations.
    pub fn with_max_gleaning(mut self, max: usize) -> Self {
        self.config.max_gleaning = max;
        self
    }

    /// Build the gleaning prompt.
    fn build_gleaning_prompt(&self, text: &str, previous_entities: &[String]) -> String {
        let prev_entities_str = previous_entities.join(", ");

        format!(
            r#"MANY entities and relationships were missed in the last extraction. 
Please identify any ADDITIONAL entities and relationships that were not already captured.

## Already Identified Entities
{prev_entities_str}

## Instructions
Look for entities and relationships that were missed in the previous extraction.
Focus on:
- Implicit entities (mentioned indirectly)
- Additional relationships between known entities
- Contextual entities (dates, locations, concepts)

## Output Format
Respond with valid JSON in this exact format:
{{
  "entities": [
    {{"name": "Entity Name", "type": "ENTITY_TYPE", "description": "Brief description"}}
  ],
  "relationships": [
    {{"source": "Source Entity", "target": "Target Entity", "type": "RELATIONSHIP_TYPE", "description": "Brief description"}}
  ]
}}

## Text to Re-Analyze
{text}

## JSON Response"#
        )
    }

    /// Parse gleaning response.
    fn parse_gleaning_response(
        &self,
        response: &str,
    ) -> Result<(Vec<ExtractedEntity>, Vec<ExtractedRelationship>)> {
        let json_str = extract_json_from_response(response);

        let parsed: serde_json::Value = serde_json::from_str(&json_str).map_err(|e| {
            PipelineError::ExtractionError(format!("Invalid JSON in gleaning: {}", e))
        })?;

        let mut entities = Vec::new();
        let mut relationships = Vec::new();

        // Extract entities
        if let Some(entity_arr) = parsed.get("entities").and_then(|v| v.as_array()) {
            for entity_val in entity_arr {
                if let (Some(name), Some(entity_type), Some(description)) = (
                    entity_val.get("name").and_then(|v| v.as_str()),
                    entity_val.get("type").and_then(|v| v.as_str()),
                    entity_val.get("description").and_then(|v| v.as_str()),
                ) {
                    entities.push(ExtractedEntity::new(name, entity_type, description));
                }
            }
        }

        // Extract relationships
        if let Some(rel_arr) = parsed.get("relationships").and_then(|v| v.as_array()) {
            for rel_val in rel_arr {
                if let (Some(source), Some(target), Some(rel_type)) = (
                    rel_val.get("source").and_then(|v| v.as_str()),
                    rel_val.get("target").and_then(|v| v.as_str()),
                    rel_val.get("type").and_then(|v| v.as_str()),
                ) {
                    let description = rel_val
                        .get("description")
                        .and_then(|v| v.as_str())
                        .unwrap_or("");

                    relationships.push(
                        ExtractedRelationship::new(source, target, rel_type)
                            .with_description(description),
                    );
                }
            }
        }

        Ok((entities, relationships))
    }

    /// Merge gleaning results with original results.
    fn merge_results(
        &self,
        original: &mut ExtractionResult,
        glean_entities: Vec<ExtractedEntity>,
        glean_relationships: Vec<ExtractedRelationship>,
    ) {
        // For entities: compare descriptions and keep the better (longer) one
        for glean_entity in glean_entities {
            let existing = original
                .entities
                .iter_mut()
                .find(|e| e.name.to_uppercase() == glean_entity.name.to_uppercase());

            if let Some(existing) = existing {
                // Keep the entity with the longer description
                if glean_entity.description.len() > existing.description.len() {
                    existing.description = glean_entity.description;
                    existing.entity_type = glean_entity.entity_type;
                }
            } else {
                // New entity from gleaning
                original.entities.push(glean_entity);
            }
        }

        // For relationships: compare and keep better descriptions
        for glean_rel in glean_relationships {
            let existing = original.relationships.iter_mut().find(|r| {
                r.source.to_uppercase() == glean_rel.source.to_uppercase()
                    && r.target.to_uppercase() == glean_rel.target.to_uppercase()
            });

            if let Some(existing) = existing {
                if glean_rel.description.len() > existing.description.len() {
                    existing.description = glean_rel.description;
                    existing.relation_type = glean_rel.relation_type;
                }
            } else {
                original.relationships.push(glean_rel);
            }
        }
    }
}

#[async_trait]
impl EntityExtractor for GleaningExtractor {
    async fn extract(&self, chunk: &TextChunk) -> Result<ExtractionResult> {
        // First pass: use base extractor
        let mut result = self.base_extractor.extract(chunk).await?;

        // Skip gleaning if disabled
        if self.config.max_gleaning == 0 {
            return Ok(result);
        }

        // Perform gleaning iterations
        for iteration in 0..self.config.max_gleaning {
            tracing::debug!(
                chunk_id = %chunk.id,
                iteration = iteration,
                "Performing gleaning iteration"
            );

            // Collect entity names for the prompt
            let entity_names: Vec<String> =
                result.entities.iter().map(|e| e.name.clone()).collect();

            // Build and execute gleaning prompt
            let gleaning_prompt = self.build_gleaning_prompt(&chunk.content, &entity_names);

            let response = self
                .llm_provider
                .complete(&gleaning_prompt)
                .await
                .map_err(|e| {
                    PipelineError::ExtractionError(format!("Gleaning LLM error: {}", e))
                })?;

            // Accumulate token usage from gleaning iterations
            result.input_tokens += response.prompt_tokens;
            result.output_tokens += response.completion_tokens;

            // Parse gleaning results
            match self.parse_gleaning_response(&response.content) {
                Ok((glean_entities, glean_relationships)) => {
                    let new_entities = glean_entities.len();
                    let new_relationships = glean_relationships.len();

                    self.merge_results(&mut result, glean_entities, glean_relationships);

                    tracing::debug!(
                        chunk_id = %chunk.id,
                        iteration = iteration,
                        new_entities = new_entities,
                        new_relationships = new_relationships,
                        "Gleaning completed"
                    );
                }
                Err(e) => {
                    tracing::warn!(
                        chunk_id = %chunk.id,
                        iteration = iteration,
                        error = %e,
                        "Gleaning parse error, continuing"
                    );
                }
            }
        }

        // Record gleaning metadata
        result.metadata.insert(
            "gleaning_iterations".to_string(),
            serde_json::json!(self.config.max_gleaning),
        );

        Ok(result)
    }

    fn name(&self) -> &str {
        "gleaning"
    }

    fn model_name(&self) -> &str {
        self.llm_provider.model()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extracted_entity_builder() {
        let entity = ExtractedEntity::new("John Doe", "PERSON", "A person named John")
            .with_importance(0.8)
            .with_source_span("John Doe is a developer");

        assert_eq!(entity.name, "John Doe");
        assert_eq!(entity.entity_type, "PERSON");
        assert_eq!(entity.importance, 0.8);
        assert_eq!(entity.source_spans.len(), 1);
    }

    #[test]
    fn test_extracted_entity_source_tracking() {
        let entity = ExtractedEntity::new("Sarah Chen", "PERSON", "Lead researcher")
            .with_source_chunk_id("chunk-001")
            .with_source_document_id("doc-abc123")
            .with_source_file_path("/documents/research.pdf");

        assert_eq!(entity.source_chunk_ids.len(), 1);
        assert_eq!(entity.source_chunk_ids[0], "chunk-001");
        assert_eq!(entity.source_document_id, Some("doc-abc123".to_string()));
        assert_eq!(
            entity.source_file_path,
            Some("/documents/research.pdf".to_string())
        );
    }

    #[test]
    fn test_extracted_entity_multiple_source_chunks() {
        let mut entity = ExtractedEntity::new("ACME Corp", "ORGANIZATION", "A company")
            .with_source_chunk_id("chunk-001")
            .with_source_document_id("doc-abc123");

        entity.add_source_chunk_id("chunk-002");
        entity.add_source_chunk_id("chunk-003");

        assert_eq!(entity.source_chunk_ids.len(), 3);
        assert!(entity.source_chunk_ids.contains(&"chunk-001".to_string()));
        assert!(entity.source_chunk_ids.contains(&"chunk-002".to_string()));
        assert!(entity.source_chunk_ids.contains(&"chunk-003".to_string()));
    }

    #[test]
    fn test_extracted_relationship_source_tracking() {
        let rel = ExtractedRelationship::new("Alice", "Bob", "KNOWS")
            .with_description("Alice knows Bob from work")
            .with_source_chunk_id("chunk-005")
            .with_source_document_id("doc-xyz789")
            .with_source_file_path("/documents/team.md");

        // Relationship has source_chunk_id as Option<String> (singular)
        assert_eq!(rel.source_chunk_id, Some("chunk-005".to_string()));
        assert_eq!(rel.source_document_id, Some("doc-xyz789".to_string()));
        assert_eq!(rel.source_file_path, Some("/documents/team.md".to_string()));
    }

    #[test]
    fn test_extracted_relationship_builder() {
        let rel = ExtractedRelationship::new("Alice", "Bob", "KNOWS")
            .with_description("Alice knows Bob from work")
            .with_weight(0.9)
            .with_keywords(vec!["colleague".to_string(), "friend".to_string()]);

        assert_eq!(rel.source, "Alice");
        assert_eq!(rel.target, "Bob");
        assert_eq!(rel.weight, 0.9);
        assert_eq!(rel.keywords.len(), 2);
    }

    #[tokio::test]
    async fn test_simple_extractor() {
        let extractor = SimpleExtractor::new().unwrap();
        let chunk = TextChunk::new("chunk-1", "John Doe works at Acme Corp.", 0, 0, 30);

        let result = extractor.extract(&chunk).await.unwrap();

        // Should find "John Doe" as a person
        assert!(result.entities.iter().any(|e| e.name == "John Doe"));
    }

    #[test]
    fn test_extraction_result() {
        let mut result = ExtractionResult::new("chunk-1");

        result.add_entity(ExtractedEntity::new("Test", "CONCEPT", "A test"));
        result.add_relationship(ExtractedRelationship::new("A", "B", "RELATED_TO"));

        assert_eq!(result.entities.len(), 1);
        assert_eq!(result.relationships.len(), 1);
    }
}
