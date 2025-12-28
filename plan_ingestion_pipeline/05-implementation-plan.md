# Implementation Plan: SOTA Ingestion Pipeline

> Document ID: IMPL-001
> Version: 2.0
> Created: 2024-12-28
> Updated: 2024-12-28 (Added SOTA Prompt System Integration)

## Table of Contents

1. [Executive Summary](#1-executive-summary)
2. [SOTA Prompt System Integration](#2-sota-prompt-system-integration)
3. [Implementation Phases](#3-implementation-phases)
4. [Phase 1: Core Enhancements + Prompt Upgrade](#4-phase-1-core-enhancements--prompt-upgrade)
5. [Phase 2: MapReduce & Caching](#5-phase-2-mapreduce--caching)
6. [Phase 3: Progress & Cost Tracking](#6-phase-3-progress--cost-tracking)
7. [Phase 4: Lineage & Document Management](#7-phase-4-lineage--document-management)
8. [Phase 5: API & Integration](#8-phase-5-api--integration)
9. [Code Changes Reference](#9-code-changes-reference)
10. [Migration Strategy](#10-migration-strategy)
11. [Risk Assessment & Roadblock Analysis](#11-risk-assessment--roadblock-analysis)

---

## 1. Executive Summary

This implementation plan outlines the step-by-step approach to enhance EdgeQuake's ingestion pipeline to SOTA (State-of-the-Art) standards. The implementation is divided into 5 phases spanning approximately 4-6 weeks.

### 1.1 Goals

| Goal                                | Priority | Phase   |
| ----------------------------------- | -------- | ------- |
| Line number tracking in chunks      | P0       | Phase 1 |
| Parallel chunk processing           | P0       | Phase 1 |
| MapReduce description summarization | P0       | Phase 2 |
| Comprehensive LLM caching           | P0       | Phase 2 |
| Real-time progress tracking         | P0       | Phase 3 |
| Cost tracking per operation         | P0       | Phase 3 |
| Full lineage tracking               | P1       | Phase 4 |
| Document suppression                | P1       | Phase 4 |
| Enhanced API endpoints              | P1       | Phase 5 |
| WebSocket progress events           | P2       | Phase 5 |

### 1.2 Timeline

```
Week 1-2: Phase 1 - Core Enhancements + Prompt Upgrade
Week 2-3: Phase 2 - MapReduce & Caching
Week 3-4: Phase 3 - Progress & Cost Tracking
Week 4-5: Phase 4 - Lineage & Document Management
Week 5-6: Phase 5 - API & Integration
```

---

## 2. SOTA Prompt System Integration

### 2.1 Critical Gap Analysis: Prompts

The current EdgeQuake prompt system has significant gaps compared to LightRAG's SOTA implementation:

| Feature                      | LightRAG (SOTA)                  | EdgeQuake (Current) | Gap Severity |
| ---------------------------- | -------------------------------- | ------------------- | ------------ |
| **Extraction Format**        | Tuple with `<\|#\|>` delimiter   | JSON                | 🔴 Critical  |
| **Completion Signal**        | `<\|COMPLETE\|>` detection       | None                | 🔴 Critical  |
| **N-ary Decomposition**      | ✅ Explicit instructions         | ❌ Not mentioned    | 🔴 Critical  |
| **Entity Naming**            | ✅ Title case, consistent naming | ❌ No guidance      | 🟡 High      |
| **Multi-Language Support**   | ✅ `{language}` parameter        | ❌ English only     | 🟡 High      |
| **Third Person Perspective** | ✅ Required                      | ❌ Not specified    | 🟡 Medium    |
| **Citation System**          | ✅ Full reference tracking       | ❌ None             | 🔴 Critical  |
| **Detailed Examples**        | ✅ 3 comprehensive examples      | ❌ None             | 🟡 High      |
| **Relationship Direction**   | ✅ Undirected by default         | ❌ Not specified    | 🟡 Medium    |
| **Gleaning Instructions**    | ✅ Focus on missed/malformed     | ⚠️ Basic            | 🟡 Medium    |

### 2.2 SOTA Prompt Templates

#### 2.2.1 Entity Extraction System Prompt (New)

**File:** `edgequake/crates/edgequake-pipeline/src/prompts/entity_extraction.rs` (NEW)

```rust
/// SOTA Entity Extraction Prompts ported from LightRAG
pub struct EntityExtractionPrompts {
    /// Default tuple delimiter for parsing
    pub tuple_delimiter: &'static str,
    /// Completion signal to detect complete extractions
    pub completion_delimiter: &'static str,
}

impl Default for EntityExtractionPrompts {
    fn default() -> Self {
        Self {
            tuple_delimiter: "<|#|>",
            completion_delimiter: "<|COMPLETE|>",
        }
    }
}

impl EntityExtractionPrompts {
    /// Build the system prompt for entity extraction
    pub fn system_prompt(&self, entity_types: &[String], language: &str) -> String {
        let entity_types_str = entity_types.join(", ");

        format!(
            r#"---Role---
You are a Knowledge Graph Specialist responsible for extracting entities and relationships from the input text.

---Instructions---
1.  **Entity Extraction & Output:**
    *   **Identification:** Identify clearly defined and meaningful entities in the input text.
    *   **Entity Details:** For each identified entity, extract the following information:
        *   `entity_name`: The name of the entity. If the entity name is case-insensitive, capitalize the first letter of each significant word (title case). Ensure **consistent naming** across the entire extraction process.
        *   `entity_type`: Categorize the entity using one of the following types: `{entity_types}`. If none of the provided entity types apply, classify it as `Other`.
        *   `entity_description`: Provide a concise yet comprehensive description of the entity's attributes and activities, based *solely* on the information present in the input text.
    *   **Output Format - Entities:** Output a total of 4 fields for each entity, delimited by `{tuple_delimiter}`, on a single line. The first field *must* be the literal string `entity`.
        *   Format: `entity{tuple_delimiter}entity_name{tuple_delimiter}entity_type{tuple_delimiter}entity_description`

2.  **Relationship Extraction & Output:**
    *   **Identification:** Identify direct, clearly stated, and meaningful relationships between previously extracted entities.
    *   **N-ary Relationship Decomposition:** If a single statement describes a relationship involving more than two entities (an N-ary relationship), decompose it into multiple binary (two-entity) relationship pairs for separate description.
        *   **Example:** For "Alice, Bob, and Carol collaborated on Project X," extract binary relationships such as "Alice collaborated with Project X," "Bob collaborated with Project X," and "Carol collaborated with Project X."
    *   **Relationship Details:** For each binary relationship, extract the following fields:
        *   `source_entity`: The name of the source entity. Ensure **consistent naming** with entity extraction.
        *   `target_entity`: The name of the target entity. Ensure **consistent naming** with entity extraction.
        *   `relationship_keywords`: One or more high-level keywords summarizing the overarching nature of the relationship. Multiple keywords separated by comma.
        *   `relationship_description`: A concise explanation of the nature of the relationship between the source and target entities.
    *   **Output Format - Relationships:** Output a total of 5 fields for each relationship, delimited by `{tuple_delimiter}`, on a single line. The first field *must* be the literal string `relation`.
        *   Format: `relation{tuple_delimiter}source_entity{tuple_delimiter}target_entity{tuple_delimiter}relationship_keywords{tuple_delimiter}relationship_description`

3.  **Delimiter Usage Protocol:**
    *   The `{tuple_delimiter}` is a complete, atomic marker and **must not be filled with content**. It serves strictly as a field separator.
    *   **Correct Example:** `entity{tuple_delimiter}Tokyo{tuple_delimiter}location{tuple_delimiter}Tokyo is the capital of Japan.`

4.  **Relationship Direction & Duplication:**
    *   Treat all relationships as **undirected** unless explicitly stated otherwise.
    *   Avoid outputting duplicate relationships.

5.  **Output Order & Prioritization:**
    *   Output all extracted entities first, followed by all extracted relationships.
    *   Within the list of relationships, prioritize those that are **most significant** to the core meaning of the input text.

6.  **Context & Objectivity:**
    *   Ensure all entity names and descriptions are written in the **third person**.
    *   Explicitly name the subject or object; **avoid using pronouns** such as `this article`, `our company`, `I`, `you`.

7.  **Language & Proper Nouns:**
    *   The entire output (entity names, keywords, and descriptions) must be written in `{language}`.
    *   Proper nouns should be retained in their original language if translation would cause ambiguity.

8.  **Completion Signal:** Output the literal string `{completion_delimiter}` only after all entities and relationships have been completely extracted.

---Examples---
{examples}"#,
            entity_types = entity_types_str,
            tuple_delimiter = self.tuple_delimiter,
            language = language,
            completion_delimiter = self.completion_delimiter,
            examples = self.get_examples()
        )
    }

    /// Build the user prompt for extraction
    pub fn user_prompt(&self, input_text: &str, entity_types: &[String], language: &str) -> String {
        let entity_types_str = entity_types.join(", ");

        format!(
            r#"---Task---
Extract entities and relationships from the input text below.

---Instructions---
1. Strictly adhere to all format requirements for entity and relationship lists.
2. Output *only* the extracted list of entities and relationships. No introductory or concluding remarks.
3. Output `{completion_delimiter}` as the final line after all extractions.
4. Ensure the output language is {language}.

---Data to be Processed---
<Entity_types>
[{entity_types}]

<Input Text>
```

{input_text}

```

<Output>"#,
            completion_delimiter = self.completion_delimiter,
            language = language,
            entity_types = entity_types_str,
            input_text = input_text
        )
    }

    fn get_examples(&self) -> &'static str {
        r#"
Example 1:
<Input Text>
while Alex clenched his jaw, the buzz of frustration dull against the backdrop of Taylor's authoritarian certainty. It was this competitive undercurrent that kept him alert, the sense that his and Jordan's shared commitment to discovery was an unspoken rebellion against Cruz's narrowing vision of control and order.

<Output>
entity<|#|>Alex<|#|>person<|#|>Alex is a character who experiences frustration and is observant of the dynamics among other characters.
entity<|#|>Taylor<|#|>person<|#|>Taylor is portrayed with authoritarian certainty and shows a moment of reverence towards a device.
entity<|#|>Jordan<|#|>person<|#|>Jordan shares a commitment to discovery with Alex.
entity<|#|>Cruz<|#|>person<|#|>Cruz is associated with a vision of control and order.
relation<|#|>Alex<|#|>Taylor<|#|>power dynamics, observation<|#|>Alex observes Taylor's authoritarian behavior.
relation<|#|>Alex<|#|>Jordan<|#|>shared goals, rebellion<|#|>Alex and Jordan share a commitment to discovery.
relation<|#|>Jordan<|#|>Cruz<|#|>ideological conflict<|#|>Jordan's discovery commitment rebels against Cruz's control vision.
<|COMPLETE|>

Example 2:
<Input Text>
Stock markets faced a sharp downturn today as tech giants saw significant declines, with the global tech index dropping by 3.4%.

<Output>
entity<|#|>Global Tech Index<|#|>category<|#|>The Global Tech Index tracks major technology stocks and dropped 3.4%.
entity<|#|>Market Selloff<|#|>event<|#|>Market selloff refers to the significant decline in stock values.
relation<|#|>Global Tech Index<|#|>Market Selloff<|#|>market performance<|#|>The tech index decline is part of the broader selloff.
<|COMPLETE|>
"#
    }
}
```

#### 2.2.2 Continue Extraction (Gleaning) Prompt

```rust
impl EntityExtractionPrompts {
    /// Build the gleaning/continue extraction prompt
    pub fn continue_extraction_prompt(&self, language: &str) -> String {
        format!(
            r#"---Task---
Based on the last extraction task, identify and extract any **missed or incorrectly formatted** entities and relationships from the input text.

---Instructions---
1.  **Strict Adherence to System Format:** Follow all format requirements from the system instructions.
2.  **Focus on Corrections/Additions:**
    *   **Do NOT** re-output entities and relationships that were **correctly and fully** extracted.
    *   If an entity or relationship was **missed**, extract and output it now.
    *   If an entity or relationship was **truncated or malformed**, re-output the *corrected and complete* version.
3.  **Output Format - Entities:** 4 fields per entity, delimited by `{tuple_delimiter}`.
4.  **Output Format - Relationships:** 5 fields per relationship, delimited by `{tuple_delimiter}`.
5.  **Output Content Only:** No introductory or concluding remarks.
6.  **Completion Signal:** Output `{completion_delimiter}` as the final line.
7.  **Output Language:** Ensure the output language is {language}.

<Output>"#,
            tuple_delimiter = self.tuple_delimiter,
            completion_delimiter = self.completion_delimiter,
            language = language
        )
    }
}
```

#### 2.2.3 Tuple Parser Implementation

```rust
/// Parser for tuple-delimited extraction results
pub struct TupleParser {
    tuple_delimiter: String,
    completion_delimiter: String,
}

impl TupleParser {
    pub fn new() -> Self {
        Self {
            tuple_delimiter: "<|#|>".to_string(),
            completion_delimiter: "<|COMPLETE|>".to_string(),
        }
    }

    /// Parse extraction results from tuple format
    pub fn parse(&self, response: &str) -> Result<ExtractionResult> {
        let mut entities = Vec::new();
        let mut relationships = Vec::new();
        let mut is_complete = false;

        for line in response.lines() {
            let line = line.trim();

            // Check for completion signal
            if line.contains(&self.completion_delimiter) {
                is_complete = true;
                continue;
            }

            let parts: Vec<&str> = line.split(&self.tuple_delimiter).collect();

            match parts.first().map(|s| s.trim()) {
                Some("entity") if parts.len() >= 4 => {
                    let entity = ExtractedEntity::new(
                        normalize_entity_name(parts[1].trim()),
                        parts[2].trim().to_uppercase(),
                        parts[3].trim(),
                    );
                    entities.push(entity);
                }
                Some("relation") if parts.len() >= 5 => {
                    let keywords: Vec<String> = parts.get(3)
                        .map(|s| s.split(',').map(|k| k.trim().to_string()).collect())
                        .unwrap_or_default();

                    let relationship = ExtractedRelationship::new(
                        normalize_entity_name(parts[1].trim()),
                        normalize_entity_name(parts[2].trim()),
                        parts.get(3).unwrap_or(&"RELATED_TO").trim(),
                    )
                    .with_description(parts.get(4).unwrap_or(&"").trim())
                    .with_keywords(keywords);

                    relationships.push(relationship);
                }
                _ => {
                    // Skip malformed lines, log for debugging
                    tracing::debug!(line = %line, "Skipping unrecognized line in extraction");
                }
            }
        }

        let mut result = ExtractionResult::new("parsed");
        result.entities = entities;
        result.relationships = relationships;
        result.metadata.insert(
            "is_complete".to_string(),
            serde_json::json!(is_complete),
        );

        Ok(result)
    }
}

/// Normalize entity name to consistent format (title case, trimmed)
fn normalize_entity_name(name: &str) -> String {
    name.split_whitespace()
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                None => String::new(),
                Some(first) => first.to_uppercase().chain(chars.flat_map(|c| c.to_lowercase())).collect(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
        .replace(' ', "_")
        .to_uppercase()
}
```

### 2.3 Prompt System Architecture

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                        SOTA PROMPT SYSTEM                                   │
└─────────────────────────────────────────────────────────────────────────────┘

                            ┌────────────────────────┐
                            │    PromptRegistry      │
                            │  (Configuration Hub)   │
                            └───────────┬────────────┘
                                        │
              ┌─────────────────────────┼─────────────────────────┐
              │                         │                         │
              ▼                         ▼                         ▼
   ┌──────────────────┐     ┌──────────────────┐     ┌──────────────────┐
   │ EntityExtraction │     │  Summarization   │     │    Keywords      │
   │     Prompts      │     │    Prompts       │     │    Prompts       │
   └────────┬─────────┘     └────────┬─────────┘     └────────┬─────────┘
            │                        │                        │
            ▼                        ▼                        ▼
   ┌──────────────────┐     ┌──────────────────┐     ┌──────────────────┐
   │   TupleParser    │     │   JSONParser     │     │   JSONParser     │
   │  (Robust Parse)  │     │  (Structured)    │     │  (Structured)    │
   └──────────────────┘     └──────────────────┘     └──────────────────┘

   ┌─────────────────────────────────────────────────────────────────────────┐
   │                         OUTPUT FORMATS                                   │
   │                                                                          │
   │  Entity Extraction:  Tuple Format (<|#|> delimiter)                     │
   │  Summarization:      Plain Text                                          │
   │  Keywords:           JSON { high_level: [], low_level: [] }             │
   │  RAG Response:       Markdown with References                            │
   └─────────────────────────────────────────────────────────────────────────┘
```

### 2.4 Migration Path: JSON → Tuple Format

The migration from JSON to tuple-based extraction is designed for zero-disruption:

```rust
/// Hybrid parser supporting both JSON and Tuple formats
pub struct HybridExtractionParser {
    json_parser: JsonExtractionParser,
    tuple_parser: TupleParser,
}

impl HybridExtractionParser {
    /// Parse extraction result, auto-detecting format
    pub fn parse(&self, response: &str) -> Result<ExtractionResult> {
        // Try tuple format first (preferred for robustness)
        if response.contains("<|#|>") {
            return self.tuple_parser.parse(response);
        }

        // Fall back to JSON parsing
        self.json_parser.parse(response)
    }
}
```

### 2.5 RAG Response with Citations

```rust
/// RAG Response prompt with citation support
pub fn rag_response_prompt(
    query: &str,
    context: &QueryContext,
    language: &str,
    response_type: &str,
    user_prompt: Option<&str>,
) -> String {
    format!(
        r#"---Role---
You are an expert AI assistant synthesizing information from a provided knowledge base.
Answer queries accurately using ONLY the information in the provided **Context**.

---Goal---
Generate a comprehensive, well-structured answer integrating facts from the Knowledge Graph and Document Chunks.

---Instructions---
1. **Step-by-Step:**
   - Analyze the query intent
   - Extract relevant facts from Context
   - Weave facts into coherent response
   - Track reference_ids for citations
   - Generate References section at the end

2. **Content & Grounding:**
   - ONLY use information from Context
   - If answer not in Context, state so
   - DO NOT invent or infer information

3. **Formatting & Language:**
   - Response MUST be in {language}
   - Use Markdown formatting
   - Present in {response_type}

4. **References Format:**
   - Under heading: `### References`
   - Format: `* [n] Document Title`
   - Maximum 5 most relevant citations
   - No content after references

{user_instructions}

---Context---
{context_data}

---Query---
{query}"#,
        language = language,
        response_type = response_type,
        user_instructions = user_prompt.map(|p| format!("5. Additional Instructions: {}", p)).unwrap_or_default(),
        context_data = context.to_string(),
        query = query
    )
}
```

---

## 3. Implementation Phases

```
┌─────────────────────────────────────────────────────────────────────────┐
│                      IMPLEMENTATION PHASES                              │
└─────────────────────────────────────────────────────────────────────────┘

Phase 1: Core Enhancements
══════════════════════════
  ┌─────────────────┐
  │ Line Number     │
  │ Tracking        │────┐
  └─────────────────┘    │
  ┌─────────────────┐    │
  │ Parallel        │────┼───▶ Phase 1 Complete
  │ Processing      │    │
  └─────────────────┘    │
  ┌─────────────────┐    │
  │ Token Usage     │────┘
  │ Enhancement     │
  └─────────────────┘

Phase 2: MapReduce & Caching
════════════════════════════
  ┌─────────────────┐
  │ MapReduce       │
  │ Summarization   │────┐
  └─────────────────┘    │
  ┌─────────────────┐    │
  │ LLM Response    │────┼───▶ Phase 2 Complete
  │ Caching         │    │
  └─────────────────┘    │
  ┌─────────────────┐    │
  │ Rebuild from    │────┘
  │ Cache           │
  └─────────────────┘

Phase 3: Progress & Cost
════════════════════════
  ┌─────────────────┐
  │ Progress        │
  │ Tracking        │────┐
  └─────────────────┘    │
  ┌─────────────────┐    │
  │ Cost Tracking   │────┼───▶ Phase 3 Complete
  │ Per Operation   │    │
  └─────────────────┘    │
  ┌─────────────────┐    │
  │ Event           │────┘
  │ Streaming       │
  └─────────────────┘

Phase 4: Lineage & Docs
═══════════════════════
  ┌─────────────────┐
  │ Full Lineage    │
  │ Storage         │────┐
  └─────────────────┘    │
  ┌─────────────────┐    │
  │ Document        │────┼───▶ Phase 4 Complete
  │ Suppression     │    │
  └─────────────────┘    │
  ┌─────────────────┐    │
  │ Entity CRUD     │────┘
  │ Cascade         │
  └─────────────────┘

Phase 5: API & Integration
══════════════════════════
  ┌─────────────────┐
  │ Enhanced API    │
  │ Endpoints       │────┐
  └─────────────────┘    │
  ┌─────────────────┐    │
  │ WebSocket       │────┼───▶ Phase 5 Complete
  │ Events          │    │
  └─────────────────┘    │
  ┌─────────────────┐    │
  │ Integration     │────┘
  │ Tests           │
  └─────────────────┘
```

---

## 4. Phase 1: Core Enhancements + Prompt Upgrade

### 4.1 Task List

| Task ID | Task                                  | File(s)                      | Effort | Dependencies |
| ------- | ------------------------------------- | ---------------------------- | ------ | ------------ |
| P1-01   | Add line number tracking to TextChunk | chunker.rs                   | 2h     | None         |
| P1-02   | Implement line number calculation     | chunker.rs                   | 3h     | P1-01        |
| P1-03   | Add parallel chunk processing         | pipeline.rs                  | 4h     | None         |
| P1-04   | Enhance token usage tracking          | extractor.rs                 | 2h     | None         |
| P1-05   | Add processing metadata to extraction | extractor.rs                 | 2h     | P1-04        |
| P1-06   | **Create prompts module**             | prompts/mod.rs (NEW)         | 2h     | None         |
| P1-07   | **Implement SOTA entity prompts**     | prompts/entity_extraction.rs | 4h     | P1-06        |
| P1-08   | **Implement tuple parser**            | prompts/parser.rs            | 3h     | P1-07        |
| P1-09   | **Add hybrid parser for migration**   | prompts/parser.rs            | 2h     | P1-08        |
| P1-10   | **Integrate prompts into extractor**  | extractor.rs                 | 3h     | P1-07, P1-08 |
| P1-11   | Update tests for new fields + prompts | tests/\*.rs                  | 4h     | P1-01..10    |

### 4.2 Prompt System Implementation

#### P1-06: Create Prompts Module

**File:** `edgequake/crates/edgequake-pipeline/src/prompts/mod.rs` (NEW)

```rust
//! SOTA Prompt Templates for Entity Extraction
//!
//! This module contains production-quality prompts ported from LightRAG,
//! implementing tuple-based extraction format for robustness.

mod entity_extraction;
mod parser;
mod summarization;

pub use entity_extraction::EntityExtractionPrompts;
pub use parser::{TupleParser, HybridExtractionParser};
pub use summarization::SummarizationPrompts;

/// Default delimiters for tuple-based extraction
pub const DEFAULT_TUPLE_DELIMITER: &str = "<|#|>";
pub const DEFAULT_COMPLETION_DELIMITER: &str = "<|COMPLETE|>";

/// Supported output languages
pub const SUPPORTED_LANGUAGES: &[&str] = &["English", "Chinese", "Japanese", "Korean", "Spanish", "French", "German"];
```

#### P1-07: SOTA Entity Extraction Prompts

See [Section 2.2.1](#221-entity-extraction-system-prompt-new) for full implementation.

Key features:

- System prompt with comprehensive instructions
- User prompt with input text formatting
- Continue extraction (gleaning) prompt
- Multi-language support via `{language}` parameter
- N-ary relationship decomposition instructions
- Third-person perspective requirement
- Completion signal detection

#### P1-08: Tuple Parser Implementation

See [Section 2.2.3](#223-tuple-parser-implementation) for full implementation.

Key features:

- Robust line-by-line parsing
- Entity/relationship detection by prefix
- Completion signal detection for retry logic
- Entity name normalization (UPPERCASE_WITH_UNDERSCORES)
- Graceful handling of malformed lines

#### P1-09: Hybrid Parser for Migration

````rust
/// Hybrid parser supporting gradual migration from JSON to Tuple format
pub struct HybridExtractionParser {
    json_parser: JsonExtractionParser,
    tuple_parser: TupleParser,
    prefer_tuple: bool,
}

impl HybridExtractionParser {
    pub fn new(prefer_tuple: bool) -> Self {
        Self {
            json_parser: JsonExtractionParser::new(),
            tuple_parser: TupleParser::new(),
            prefer_tuple,
        }
    }

    /// Parse extraction result, auto-detecting format
    pub fn parse(&self, response: &str, chunk_id: &str) -> Result<ExtractionResult> {
        // Detect format by content
        let has_tuple_markers = response.contains("<|#|>") || response.contains("entity<|");
        let has_json_markers = response.trim_start().starts_with('{')
            || response.contains("```json");

        if has_tuple_markers && (!has_json_markers || self.prefer_tuple) {
            self.tuple_parser.parse(response, chunk_id)
        } else {
            self.json_parser.parse(response, chunk_id)
        }
    }
}
````

#### P1-10: Integrate Prompts into Extractor

**File:** `edgequake/crates/edgequake-pipeline/src/extractor.rs`

```rust
use crate::prompts::{EntityExtractionPrompts, HybridExtractionParser};

/// Enhanced LLM-based entity extractor using SOTA prompts
pub struct LLMExtractor<L>
where
    L: edgequake_llm::LLMProvider + ?Sized,
{
    llm_provider: std::sync::Arc<L>,
    entity_types: Vec<String>,
    prompts: EntityExtractionPrompts,
    parser: HybridExtractionParser,
    language: String,
    use_sota_prompts: bool,
}

impl<L> LLMExtractor<L>
where
    L: edgequake_llm::LLMProvider + ?Sized,
{
    /// Create with SOTA prompt system (recommended)
    pub fn new_sota(llm_provider: std::sync::Arc<L>) -> Self {
        Self {
            llm_provider,
            entity_types: default_entity_types(),
            prompts: EntityExtractionPrompts::default(),
            parser: HybridExtractionParser::new(true), // Prefer tuple format
            language: "English".to_string(),
            use_sota_prompts: true,
        }
    }

    /// Create with legacy JSON prompts (for migration)
    pub fn new_legacy(llm_provider: std::sync::Arc<L>) -> Self {
        Self {
            llm_provider,
            entity_types: default_entity_types(),
            prompts: EntityExtractionPrompts::default(),
            parser: HybridExtractionParser::new(false), // Prefer JSON
            language: "English".to_string(),
            use_sota_prompts: false,
        }
    }

    /// Set output language
    pub fn with_language(mut self, language: impl Into<String>) -> Self {
        self.language = language.into();
        self
    }
}

#[async_trait]
impl<L> EntityExtractor for LLMExtractor<L>
where
    L: edgequake_llm::LLMProvider + Send + Sync + ?Sized,
{
    async fn extract(&self, chunk: &TextChunk) -> Result<ExtractionResult> {
        let (system_prompt, user_prompt) = if self.use_sota_prompts {
            (
                Some(self.prompts.system_prompt(&self.entity_types, &self.language)),
                self.prompts.user_prompt(&chunk.content, &self.entity_types, &self.language),
            )
        } else {
            (None, self.build_legacy_prompt(&chunk.content))
        };

        let response = if let Some(system) = system_prompt {
            self.llm_provider
                .complete_with_system(&system, &user_prompt)
                .await
                .map_err(|e| PipelineError::ExtractionError(format!("LLM error: {}", e)))?
        } else {
            self.llm_provider
                .complete(&user_prompt)
                .await
                .map_err(|e| PipelineError::ExtractionError(format!("LLM error: {}", e)))?
        };

        self.parser.parse(&response.content, &chunk.id)
    }

    fn name(&self) -> &str {
        if self.use_sota_prompts { "llm-sota" } else { "llm-legacy" }
    }
}
```

### 4.3 Original Core Enhancement Tasks

#### P1-01: Add Line Number Tracking to TextChunk

**File:** `edgequake/crates/edgequake-pipeline/src/chunker.rs`

```rust
// ADD to TextChunk struct
pub struct TextChunk {
    pub id: String,
    pub content: String,
    pub index: usize,
    // Existing
    pub start_offset: usize,
    pub end_offset: usize,
    // NEW: Line number tracking
    pub start_line: usize,      // 1-based line number
    pub end_line: usize,        // 1-based, inclusive
    pub token_count: usize,
    pub embedding: Option<Vec<f32>>,
}
```

#### P1-02: Implement Line Number Calculation

**File:** `edgequake/crates/edgequake-pipeline/src/chunker.rs`

```rust
// ADD helper function
fn calculate_line_numbers(full_text: &str, start_offset: usize, end_offset: usize) -> (usize, usize) {
    let before_chunk = &full_text[..start_offset];
    let chunk_text = &full_text[start_offset..end_offset];

    // Count newlines before start
    let start_line = before_chunk.chars().filter(|&c| c == '\n').count() + 1;

    // Count newlines in chunk
    let lines_in_chunk = chunk_text.chars().filter(|&c| c == '\n').count();
    let end_line = start_line + lines_in_chunk;

    (start_line, end_line)
}

// MODIFY chunk_sync to calculate line numbers
fn chunk_sync(&self, text: &str, doc_id: &str) -> Result<Vec<TextChunk>> {
    // ... existing chunking logic ...

    Ok(chunks
        .into_iter()
        .enumerate()
        .map(|(index, (content, start, end))| {
            let (start_line, end_line) = calculate_line_numbers(text, start, end);
            let id = format!("{}-chunk-{}", doc_id, index);
            TextChunk {
                id,
                content,
                index,
                start_offset: start,
                end_offset: end,
                start_line,
                end_line,
                token_count: estimate_tokens(&content),
                embedding: None,
            }
        })
        .collect())
}
```

#### P1-03: Implement Parallel Chunk Processing

**File:** `edgequake/crates/edgequake-pipeline/src/pipeline.rs`

```rust
use futures::stream::{self, StreamExt};

impl Pipeline {
    /// Process chunks in parallel with semaphore control
    async fn extract_parallel(
        &self,
        chunks: &[TextChunk],
        extractor: &Arc<dyn EntityExtractor>,
    ) -> Result<Vec<ExtractionResult>> {
        let semaphore = Arc::new(tokio::sync::Semaphore::new(
            self.config.max_concurrent_extractions
        ));

        let futures: Vec<_> = chunks.iter().map(|chunk| {
            let semaphore = semaphore.clone();
            let extractor = extractor.clone();
            let chunk = chunk.clone();

            async move {
                let _permit = semaphore.acquire().await
                    .map_err(|e| PipelineError::ExtractionError(e.to_string()))?;
                extractor.extract(&chunk).await
            }
        }).collect();

        let results: Vec<Result<ExtractionResult>> = stream::iter(futures)
            .buffer_unordered(self.config.max_concurrent_extractions)
            .collect()
            .await;

        results.into_iter().collect()
    }

    /// Updated process method using parallel extraction
    pub async fn process(&self, document_id: &str, content: &str) -> Result<ProcessingResult> {
        let start = std::time::Instant::now();
        let mut stats = ProcessingStats::default();

        // Step 1: Chunk the document
        let mut chunks = self.chunker.chunk(content, document_id)?;
        stats.chunk_count = chunks.len();

        // Step 2: Extract in parallel
        let mut extractions = Vec::new();
        if self.config.enable_entity_extraction || self.config.enable_relationship_extraction {
            if let Some(extractor) = &self.extractor {
                extractions = self.extract_parallel(&chunks, extractor).await?;

                // Aggregate stats
                for extraction in &extractions {
                    stats.entity_count += extraction.entities.len();
                    stats.relationship_count += extraction.relationships.len();
                    stats.llm_calls += 1;
                }
            }
        }

        // ... rest of processing ...
    }
}
```

#### P1-04: Enhance Token Usage Tracking

**File:** `edgequake/crates/edgequake-pipeline/src/extractor.rs`

```rust
// ADD to ExtractionResult
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ExtractionResult {
    pub entities: Vec<ExtractedEntity>,
    pub relationships: Vec<ExtractedRelationship>,
    pub source_chunk_id: String,
    pub metadata: HashMap<String, serde_json::Value>,
    // NEW: Token usage tracking
    pub input_tokens: usize,
    pub output_tokens: usize,
    pub extraction_time_ms: u64,
}

// MODIFY LLMExtractor.extract to track tokens
#[async_trait]
impl<L> EntityExtractor for LLMExtractor<L>
where
    L: edgequake_llm::LLMProvider + Send + Sync + ?Sized,
{
    async fn extract(&self, chunk: &TextChunk) -> Result<ExtractionResult> {
        let start = std::time::Instant::now();
        let prompt = self.build_prompt(&chunk.content);

        let response = self
            .llm_provider
            .complete(&prompt)
            .await
            .map_err(|e| PipelineError::ExtractionError(format!("LLM error: {}", e)))?;

        let mut result = self.parse_response(&response.content, &chunk.id)?;

        // NEW: Track token usage
        result.input_tokens = response.input_tokens.unwrap_or(0);
        result.output_tokens = response.output_tokens.unwrap_or(0);
        result.extraction_time_ms = start.elapsed().as_millis() as u64;

        Ok(result)
    }
}
```

### 3.3 Acceptance Criteria

- [ ] TextChunk includes start_line and end_line fields
- [ ] Line numbers are correctly calculated for all chunks
- [ ] Parallel processing works with configurable concurrency
- [ ] Token usage is tracked per extraction
- [ ] All existing tests pass
- [ ] New tests for line number tracking pass

---

## 5. Phase 2: MapReduce & Caching

### 5.1 Task List

| Task ID | Task                             | File(s)        | Effort | Dependencies |
| ------- | -------------------------------- | -------------- | ------ | ------------ |
| P2-01   | Create MapReduce summarizer      | summarizer.rs  | 6h     | None         |
| P2-02   | Add LLM response caching trait   | cache.rs (new) | 4h     | None         |
| P2-03   | Implement in-memory cache        | cache.rs       | 3h     | P2-02        |
| P2-04   | Implement PostgreSQL cache       | cache.rs       | 4h     | P2-02        |
| P2-05   | Integrate caching into extractor | extractor.rs   | 3h     | P2-03        |
| P2-06   | Implement rebuild from cache     | pipeline.rs    | 4h     | P2-05        |
| P2-07   | Integrate MapReduce into merger  | merger.rs      | 3h     | P2-01        |
| P2-08   | Add tests for caching            | tests/\*.rs    | 4h     | P2-01..07    |

### 5.2 Detailed Implementation

#### P2-01: Create MapReduce Summarizer

**File:** `edgequake/crates/edgequake-pipeline/src/summarizer.rs`

```rust
use async_trait::async_trait;
use edgequake_llm::LLMProvider;
use std::sync::Arc;

/// Configuration for MapReduce summarization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SummarizerConfig {
    /// Maximum context size in tokens
    pub context_size: usize,
    /// Target summary length in tokens
    pub summary_length: usize,
    /// Minimum descriptions before forcing LLM summary
    pub force_llm_summary_on_merge: usize,
    /// Separator between descriptions
    pub separator: String,
}

impl Default for SummarizerConfig {
    fn default() -> Self {
        Self {
            context_size: 4000,
            summary_length: 500,
            force_llm_summary_on_merge: 6,
            separator: "\n\n".to_string(),
        }
    }
}

/// MapReduce description summarizer
pub struct MapReduceSummarizer<L: LLMProvider> {
    llm_provider: Arc<L>,
    config: SummarizerConfig,
}

impl<L: LLMProvider + Send + Sync> MapReduceSummarizer<L> {
    pub fn new(llm_provider: Arc<L>, config: SummarizerConfig) -> Self {
        Self { llm_provider, config }
    }

    /// Summarize descriptions using map-reduce approach
    pub async fn summarize(&self, descriptions: Vec<String>) -> Result<(String, bool)> {
        // Base case: single description
        if descriptions.len() == 1 {
            return Ok((descriptions[0].clone(), false));
        }

        let total_tokens: usize = descriptions.iter()
            .map(|d| estimate_tokens(d))
            .sum();

        // If within limits, just join
        if total_tokens <= self.config.context_size
            && descriptions.len() < self.config.force_llm_summary_on_merge
        {
            return Ok((descriptions.join(&self.config.separator), false));
        }

        // MAP phase: split into chunks and summarize each
        let chunks = self.split_into_chunks(&descriptions);
        let mut summaries = Vec::new();

        for chunk in chunks {
            if chunk.len() == 1 {
                summaries.push(chunk[0].clone());
            } else {
                let summary = self.llm_summarize(&chunk).await?;
                summaries.push(summary);
            }
        }

        // REDUCE phase: recursively summarize summaries
        if summaries.len() > 1 {
            Box::pin(self.summarize(summaries)).await
        } else {
            Ok((summaries.into_iter().next().unwrap_or_default(), true))
        }
    }

    fn split_into_chunks(&self, descriptions: &[String]) -> Vec<Vec<String>> {
        let mut chunks = Vec::new();
        let mut current_chunk = Vec::new();
        let mut current_tokens = 0;

        for desc in descriptions {
            let desc_tokens = estimate_tokens(desc);

            if current_tokens + desc_tokens > self.config.context_size && !current_chunk.is_empty() {
                chunks.push(current_chunk);
                current_chunk = Vec::new();
                current_tokens = 0;
            }

            current_chunk.push(desc.clone());
            current_tokens += desc_tokens;
        }

        if !current_chunk.is_empty() {
            chunks.push(current_chunk);
        }

        chunks
    }

    async fn llm_summarize(&self, descriptions: &[String]) -> Result<String> {
        let prompt = format!(
            r#"Summarize the following descriptions into a single, comprehensive description.
Keep all important facts and details. Maximum length: {} tokens.

Descriptions:
{}

Summary:"#,
            self.config.summary_length,
            descriptions.join("\n---\n")
        );

        let response = self.llm_provider.complete(&prompt).await
            .map_err(|e| PipelineError::SummarizationError(e.to_string()))?;

        Ok(response.content.trim().to_string())
    }
}
```

#### P2-02: Add LLM Response Caching

**File:** `edgequake/crates/edgequake-pipeline/src/cache.rs` (NEW)

```rust
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Cache entry for LLM responses
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheEntry {
    pub id: String,
    pub cache_type: CacheType,
    pub chunk_id: Option<String>,
    pub prompt_hash: String,
    pub response: String,
    pub input_tokens: usize,
    pub output_tokens: usize,
    pub model: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum CacheType {
    Extract,
    Glean,
    Summary,
}

/// Trait for LLM response caching
#[async_trait]
pub trait LLMCache: Send + Sync {
    /// Get cached response by prompt hash
    async fn get(&self, prompt_hash: &str) -> Result<Option<CacheEntry>>;

    /// Store response in cache
    async fn set(&self, entry: CacheEntry) -> Result<()>;

    /// Get all cache entries for a chunk
    async fn get_by_chunk(&self, chunk_id: &str) -> Result<Vec<CacheEntry>>;

    /// Delete cache entries by chunk ID
    async fn delete_by_chunk(&self, chunk_id: &str) -> Result<usize>;

    /// Clear all cache entries
    async fn clear(&self) -> Result<()>;
}

/// In-memory cache implementation
pub struct MemoryLLMCache {
    entries: tokio::sync::RwLock<HashMap<String, CacheEntry>>,
    chunk_index: tokio::sync::RwLock<HashMap<String, Vec<String>>>,
}

impl MemoryLLMCache {
    pub fn new() -> Self {
        Self {
            entries: tokio::sync::RwLock::new(HashMap::new()),
            chunk_index: tokio::sync::RwLock::new(HashMap::new()),
        }
    }
}

#[async_trait]
impl LLMCache for MemoryLLMCache {
    async fn get(&self, prompt_hash: &str) -> Result<Option<CacheEntry>> {
        let entries = self.entries.read().await;
        Ok(entries.get(prompt_hash).cloned())
    }

    async fn set(&self, entry: CacheEntry) -> Result<()> {
        let mut entries = self.entries.write().await;
        let mut chunk_index = self.chunk_index.write().await;

        if let Some(chunk_id) = &entry.chunk_id {
            chunk_index
                .entry(chunk_id.clone())
                .or_default()
                .push(entry.prompt_hash.clone());
        }

        entries.insert(entry.prompt_hash.clone(), entry);
        Ok(())
    }

    async fn get_by_chunk(&self, chunk_id: &str) -> Result<Vec<CacheEntry>> {
        let entries = self.entries.read().await;
        let chunk_index = self.chunk_index.read().await;

        let hashes = chunk_index.get(chunk_id);
        let mut results = Vec::new();

        if let Some(hashes) = hashes {
            for hash in hashes {
                if let Some(entry) = entries.get(hash) {
                    results.push(entry.clone());
                }
            }
        }

        Ok(results)
    }

    async fn delete_by_chunk(&self, chunk_id: &str) -> Result<usize> {
        let mut entries = self.entries.write().await;
        let mut chunk_index = self.chunk_index.write().await;

        let hashes = chunk_index.remove(chunk_id).unwrap_or_default();
        let count = hashes.len();

        for hash in hashes {
            entries.remove(&hash);
        }

        Ok(count)
    }

    async fn clear(&self) -> Result<()> {
        let mut entries = self.entries.write().await;
        let mut chunk_index = self.chunk_index.write().await;

        entries.clear();
        chunk_index.clear();

        Ok(())
    }
}
```

### 5.3 Acceptance Criteria

- [ ] MapReduce summarizer handles large description sets
- [ ] LLM caching reduces redundant API calls
- [ ] Cache hit rate is tracked in stats
- [ ] Rebuild from cache works correctly
- [ ] Integration tests pass for caching scenarios

---

## 6. Phase 3: Progress & Cost Tracking

### 6.1 Task List

| Task ID | Task                           | File(s)           | Effort | Dependencies |
| ------- | ------------------------------ | ----------------- | ------ | ------------ |
| P3-01   | Create progress tracking types | types/progress.rs | 3h     | None         |
| P3-02   | Create cost tracking types     | types/cost.rs     | 3h     | None         |
| P3-03   | Implement progress reporter    | progress.rs (new) | 4h     | P3-01        |
| P3-04   | Implement cost calculator      | cost.rs (new)     | 3h     | P3-02        |
| P3-05   | Integrate into pipeline        | pipeline.rs       | 4h     | P3-03, P3-04 |
| P3-06   | Add progress storage           | storage/\*.rs     | 3h     | P3-03        |
| P3-07   | Add event streaming            | events.rs (new)   | 4h     | P3-03        |
| P3-08   | Add tests                      | tests/\*.rs       | 3h     | P3-01..07    |

### 6.2 Detailed Implementation

#### P3-03: Implement Progress Reporter

**File:** `edgequake/crates/edgequake-core/src/progress.rs` (NEW)

```rust
use chrono::{DateTime, Utc};
use std::sync::Arc;
use tokio::sync::RwLock;

/// Progress tracking for ingestion jobs
pub struct ProgressTracker {
    job_id: String,
    document_id: String,
    state: Arc<RwLock<ProgressState>>,
    event_sender: Option<tokio::sync::broadcast::Sender<ProgressEvent>>,
}

#[derive(Debug, Clone)]
struct ProgressState {
    status: IngestionStatus,
    current_stage: PipelineStage,
    stages: Vec<StageProgress>,
    messages: Vec<ProgressMessage>,
    errors: Vec<IngestionError>,
    started_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl ProgressTracker {
    pub fn new(job_id: String, document_id: String) -> Self {
        let stages = vec![
            StageProgress::new(PipelineStage::Preprocessing),
            StageProgress::new(PipelineStage::Chunking),
            StageProgress::new(PipelineStage::Extracting),
            StageProgress::new(PipelineStage::Merging),
            StageProgress::new(PipelineStage::Embedding),
            StageProgress::new(PipelineStage::Storing),
        ];

        Self {
            job_id,
            document_id,
            state: Arc::new(RwLock::new(ProgressState {
                status: IngestionStatus::Pending,
                current_stage: PipelineStage::Preprocessing,
                stages,
                messages: Vec::new(),
                errors: Vec::new(),
                started_at: Utc::now(),
                updated_at: Utc::now(),
            })),
            event_sender: None,
        }
    }

    pub fn with_event_channel(mut self, sender: tokio::sync::broadcast::Sender<ProgressEvent>) -> Self {
        self.event_sender = Some(sender);
        self
    }

    /// Start processing
    pub async fn start(&self) {
        let mut state = self.state.write().await;
        state.status = IngestionStatus::Running;
        state.updated_at = Utc::now();

        self.emit_event(ProgressEvent::Started {
            job_id: self.job_id.clone(),
            document_id: self.document_id.clone(),
        }).await;
    }

    /// Begin a stage
    pub async fn begin_stage(&self, stage: PipelineStage, total_items: usize) {
        let mut state = self.state.write().await;
        state.current_stage = stage;

        if let Some(s) = state.stages.iter_mut().find(|s| s.stage == stage) {
            s.status = StageStatus::Running;
            s.total_items = total_items;
            s.started_at = Some(Utc::now());
        }

        state.updated_at = Utc::now();
        self.add_message(&mut state, format!("Starting {}", stage.as_str()));

        self.emit_event(ProgressEvent::StageStarted {
            job_id: self.job_id.clone(),
            stage,
            total_items,
        }).await;
    }

    /// Update stage progress
    pub async fn update_progress(&self, completed_items: usize, message: Option<&str>) {
        let mut state = self.state.write().await;

        if let Some(s) = state.stages.iter_mut().find(|s| s.stage == state.current_stage) {
            s.completed_items = completed_items;
        }

        if let Some(msg) = message {
            self.add_message(&mut state, msg.to_string());
        }

        state.updated_at = Utc::now();

        let progress = self.calculate_percentage(&state);
        self.emit_event(ProgressEvent::Progress {
            job_id: self.job_id.clone(),
            stage: state.current_stage,
            completed: completed_items,
            percentage: progress,
        }).await;
    }

    /// Complete a stage
    pub async fn complete_stage(&self, stage: PipelineStage) {
        let mut state = self.state.write().await;

        if let Some(s) = state.stages.iter_mut().find(|s| s.stage == stage) {
            s.status = StageStatus::Completed;
            s.completed_items = s.total_items;
            s.completed_at = Some(Utc::now());
        }

        state.updated_at = Utc::now();
        self.add_message(&mut state, format!("Completed {}", stage.as_str()));

        self.emit_event(ProgressEvent::StageCompleted {
            job_id: self.job_id.clone(),
            stage,
        }).await;
    }

    /// Record an error
    pub async fn record_error(&self, error: IngestionError) {
        let mut state = self.state.write().await;
        state.errors.push(error.clone());
        state.updated_at = Utc::now();

        if !error.recoverable {
            state.status = IngestionStatus::Failed;
        }

        self.emit_event(ProgressEvent::Error {
            job_id: self.job_id.clone(),
            error,
        }).await;
    }

    /// Complete the job
    pub async fn complete(&self, result: IngestionResult) {
        let mut state = self.state.write().await;
        state.status = IngestionStatus::Completed;
        state.updated_at = Utc::now();

        self.emit_event(ProgressEvent::Completed {
            job_id: self.job_id.clone(),
            result,
        }).await;
    }

    fn calculate_percentage(&self, state: &ProgressState) -> f32 {
        let total_weight: f32 = state.stages.len() as f32;
        let completed: f32 = state.stages.iter()
            .map(|s| match s.status {
                StageStatus::Completed => 1.0,
                StageStatus::Running if s.total_items > 0 => {
                    s.completed_items as f32 / s.total_items as f32
                }
                _ => 0.0,
            })
            .sum();

        (completed / total_weight) * 100.0
    }

    fn add_message(&self, state: &mut ProgressState, message: String) {
        state.messages.push(ProgressMessage {
            message,
            level: MessageLevel::Info,
            timestamp: Utc::now(),
        });
    }

    async fn emit_event(&self, event: ProgressEvent) {
        if let Some(sender) = &self.event_sender {
            let _ = sender.send(event);
        }
    }

    /// Get current progress snapshot
    pub async fn snapshot(&self) -> IngestionProgress {
        let state = self.state.read().await;
        IngestionProgress {
            job_id: self.job_id.clone(),
            document_id: self.document_id.clone(),
            status: state.status,
            current_stage: state.current_stage,
            completion_percentage: self.calculate_percentage(&state),
            stages: state.stages.clone(),
            latest_message: state.messages.last()
                .map(|m| m.message.clone())
                .unwrap_or_default(),
            history_messages: state.messages.clone(),
            errors: state.errors.clone(),
            started_at: state.started_at,
            updated_at: state.updated_at,
            completed_at: None,
            eta_seconds: None,
        }
    }
}
```

#### P3-04: Implement Cost Calculator

**File:** `edgequake/crates/edgequake-core/src/cost.rs` (NEW)

```rust
use std::collections::HashMap;

/// Cost configuration for different models
#[derive(Debug, Clone)]
pub struct ModelCost {
    /// Cost per 1000 input tokens
    pub input_per_1k: f64,
    /// Cost per 1000 output tokens
    pub output_per_1k: f64,
}

/// Known model costs (as of Dec 2024)
lazy_static::lazy_static! {
    static ref MODEL_COSTS: HashMap<&'static str, ModelCost> = {
        let mut m = HashMap::new();
        m.insert("gpt-4o-mini", ModelCost { input_per_1k: 0.00015, output_per_1k: 0.0006 });
        m.insert("gpt-4o", ModelCost { input_per_1k: 0.005, output_per_1k: 0.015 });
        m.insert("gpt-4", ModelCost { input_per_1k: 0.03, output_per_1k: 0.06 });
        m.insert("text-embedding-3-small", ModelCost { input_per_1k: 0.00002, output_per_1k: 0.0 });
        m.insert("text-embedding-3-large", ModelCost { input_per_1k: 0.00013, output_per_1k: 0.0 });
        m
    };
}

/// Cost calculator for ingestion operations
pub struct CostCalculator {
    custom_costs: HashMap<String, ModelCost>,
}

impl CostCalculator {
    pub fn new() -> Self {
        Self {
            custom_costs: HashMap::new(),
        }
    }

    pub fn with_custom_cost(mut self, model: &str, cost: ModelCost) -> Self {
        self.custom_costs.insert(model.to_string(), cost);
        self
    }

    /// Calculate cost for an operation
    pub fn calculate(
        &self,
        model: &str,
        input_tokens: usize,
        output_tokens: usize,
    ) -> f64 {
        let cost = self.custom_costs.get(model)
            .or_else(|| MODEL_COSTS.get(model))
            .cloned()
            .unwrap_or(ModelCost { input_per_1k: 0.0, output_per_1k: 0.0 });

        let input_cost = (input_tokens as f64 / 1000.0) * cost.input_per_1k;
        let output_cost = (output_tokens as f64 / 1000.0) * cost.output_per_1k;

        input_cost + output_cost
    }

    /// Create cost breakdown from processing stats
    pub fn create_breakdown(
        &self,
        extraction_model: &str,
        embedding_model: &str,
        stats: &ProcessingStats,
    ) -> CostBreakdown {
        let extraction_cost = self.calculate(
            extraction_model,
            stats.extraction_input_tokens,
            stats.extraction_output_tokens,
        );

        let gleaning_cost = self.calculate(
            extraction_model,
            stats.gleaning_input_tokens,
            stats.gleaning_output_tokens,
        );

        let summarization_cost = self.calculate(
            extraction_model,
            stats.summarization_input_tokens,
            stats.summarization_output_tokens,
        );

        let embedding_cost = self.calculate(
            embedding_model,
            stats.embedding_tokens,
            0,
        );

        CostBreakdown {
            extraction: OperationCost {
                api_calls: stats.extraction_calls,
                input_tokens: stats.extraction_input_tokens,
                output_tokens: stats.extraction_output_tokens,
                cost_usd: extraction_cost,
                model: extraction_model.to_string(),
            },
            gleaning: OperationCost {
                api_calls: stats.gleaning_calls,
                input_tokens: stats.gleaning_input_tokens,
                output_tokens: stats.gleaning_output_tokens,
                cost_usd: gleaning_cost,
                model: extraction_model.to_string(),
            },
            summarization: OperationCost {
                api_calls: stats.summarization_calls,
                input_tokens: stats.summarization_input_tokens,
                output_tokens: stats.summarization_output_tokens,
                cost_usd: summarization_cost,
                model: extraction_model.to_string(),
            },
            embedding: OperationCost {
                api_calls: stats.embedding_calls,
                input_tokens: stats.embedding_tokens,
                output_tokens: 0,
                cost_usd: embedding_cost,
                model: embedding_model.to_string(),
            },
            total_usd: extraction_cost + gleaning_cost + summarization_cost + embedding_cost,
        }
    }
}
```

### 6.3 Acceptance Criteria

- [ ] Progress is tracked at stage level
- [ ] Messages are recorded in history
- [ ] Errors are tracked with context
- [ ] Cost is calculated accurately per model
- [ ] Events are emitted in real-time
- [ ] Progress can be queried via API

---

## 7. Phase 4: Lineage & Document Management

### 7.1 Task List

| Task ID | Task                            | File(s)               | Effort | Dependencies |
| ------- | ------------------------------- | --------------------- | ------ | ------------ |
| P4-01   | Create lineage types            | types/lineage.rs      | 3h     | None         |
| P4-02   | Implement lineage storage       | storage/lineage.rs    | 4h     | P4-01        |
| P4-03   | Integrate lineage into pipeline | pipeline.rs           | 4h     | P4-02        |
| P4-04   | Implement document suppression  | documents.rs          | 4h     | P4-03        |
| P4-05   | Implement cascade delete        | graph.rs              | 4h     | P4-04        |
| P4-06   | Add impact analysis             | handlers/documents.rs | 3h     | P4-05        |
| P4-07   | Add tests                       | tests/\*.rs           | 4h     | P4-01..06    |

### 7.2 Acceptance Criteria

- [ ] Lineage tracks document → chunk → entity/relationship
- [ ] Line numbers are preserved in lineage
- [ ] Document suppression removes associated graph entries
- [ ] Orphaned entities are handled correctly
- [ ] Impact analysis shows deletion effects before execution

---

## 8. Phase 5: API & Integration

### 8.1 Task List

| Task ID | Task                        | File(s)               | Effort | Dependencies |
| ------- | --------------------------- | --------------------- | ------ | ------------ |
| P5-01   | Add progress endpoints      | handlers/pipeline.rs  | 3h     | Phase 3      |
| P5-02   | Add lineage endpoints       | handlers/documents.rs | 3h     | Phase 4      |
| P5-03   | Add cost endpoints          | handlers/costs.rs     | 2h     | Phase 3      |
| P5-04   | Implement WebSocket handler | ws.rs (new)           | 6h     | Phase 3      |
| P5-05   | Update OpenAPI spec         | openapi.rs            | 3h     | P5-01..04    |
| P5-06   | Create E2E tests            | e2e/\*.rs             | 6h     | P5-01..05    |
| P5-07   | Update documentation        | docs/\*.md            | 4h     | P5-01..06    |

### 8.2 Acceptance Criteria

- [ ] All API endpoints documented in OpenAPI
- [ ] WebSocket events work for progress tracking
- [ ] E2E tests cover full ingestion flow
- [ ] Documentation is updated with new features

---

## 9. Code Changes Reference

### 9.1 Files to Modify

| File                                 | Phase | Changes                      |
| ------------------------------------ | ----- | ---------------------------- |
| edgequake-pipeline/src/chunker.rs    | 1     | Line number tracking         |
| edgequake-pipeline/src/pipeline.rs   | 1, 2  | Parallel processing, caching |
| edgequake-pipeline/src/extractor.rs  | 1, 2  | Token tracking, caching      |
| edgequake-pipeline/src/merger.rs     | 2     | MapReduce integration        |
| edgequake-pipeline/src/summarizer.rs | 2     | MapReduce summarizer         |
| edgequake-core/src/orchestrator.rs   | 3, 4  | Progress, lineage            |
| edgequake-storage/src/traits/\*.rs   | 4     | Lineage storage              |
| edgequake-api/src/handlers/\*.rs     | 5     | New endpoints                |

### 9.2 New Files to Create

| File                                      | Phase | Purpose            |
| ----------------------------------------- | ----- | ------------------ |
| edgequake-pipeline/src/prompts/mod.rs     | 1     | SOTA prompt module |
| edgequake-pipeline/src/prompts/entity.rs  | 1     | Entity prompts     |
| edgequake-pipeline/src/prompts/parser.rs  | 1     | TupleParser        |
| edgequake-pipeline/src/prompts/hybrid.rs  | 1     | HybridParser       |
| edgequake-pipeline/src/cache.rs           | 2     | LLM caching        |
| edgequake-core/src/progress.rs            | 3     | Progress tracking  |
| edgequake-core/src/cost.rs                | 3     | Cost calculation   |
| edgequake-core/src/types/lineage.rs       | 4     | Lineage types      |
| edgequake-storage/src/adapters/lineage.rs | 4     | Lineage storage    |
| edgequake-api/src/ws.rs                   | 5     | WebSocket handler  |

---

## 10. Migration Strategy

### 10.1 Database Migrations

```sql
-- Migration 001: Add line numbers to chunks
ALTER TABLE chunks ADD COLUMN start_line INTEGER NOT NULL DEFAULT 1;
ALTER TABLE chunks ADD COLUMN end_line INTEGER NOT NULL DEFAULT 1;

-- Migration 002: Add lineage tables
CREATE TABLE document_lineage (...);
CREATE TABLE chunk_lineage (...);
CREATE TABLE entity_lineage (...);

-- Migration 003: Add cost tracking
CREATE TABLE ingestion_costs (...);

-- Migration 004: Add LLM cache
CREATE TABLE llm_cache (...);
```

### 10.2 Backward Compatibility

- All new fields have sensible defaults
- API changes are additive (new endpoints)
- Existing data remains valid
- Lineage can be backfilled from existing data
- **Prompt system uses hybrid parsing for gradual migration**

---

## 11. Risk Assessment & Roadblock Analysis

### 11.1 Risk Matrix

| Risk                                    | Impact | Probability | Mitigation                             |
| --------------------------------------- | ------ | ----------- | -------------------------------------- |
| Parallel processing increases LLM costs | Medium | Medium      | Rate limiting, cost alerts             |
| MapReduce adds latency                  | Low    | High        | Make optional, tune thresholds         |
| Cache invalidation complexity           | Medium | Medium      | Clear invalidation rules               |
| WebSocket scaling issues                | High   | Low         | Load testing, connection limits        |
| Migration data loss                     | High   | Low         | Backup before migration, rollback plan |
| **Prompt format change breaks parsing** | High   | Medium      | **Hybrid parser, gradual rollout**     |
| **LLM output non-compliance**           | Medium | High        | **Retry logic, fallback to JSON**      |
| **Multi-language prompt complexity**    | Medium | Low         | **Start English-only, add languages**  |

### 11.2 Potential Roadblocks & Mitigations

#### RB-001: LLM Non-Compliance with Tuple Format

**Risk Level:** 🟡 Medium-High

**Description:** Some LLM models may not consistently follow the tuple-based output format, especially smaller models or when context is limited.

**Indicators:**

- Missing `<|COMPLETE|>` signal
- Incorrect delimiter usage
- Mixed JSON and tuple output
- Truncated responses

**Mitigation Strategy:**

```rust
/// Robust extraction with retries and fallbacks
async fn extract_with_retry(&self, chunk: &TextChunk) -> Result<ExtractionResult> {
    const MAX_RETRIES: usize = 3;

    for attempt in 0..MAX_RETRIES {
        let response = self.llm_provider.complete(&self.prompt).await?;

        // Check for completion signal
        let is_complete = response.content.contains("<|COMPLETE|>");

        match self.parser.parse(&response.content, &chunk.id) {
            Ok(result) if !result.entities.is_empty() || is_complete => {
                return Ok(result);
            }
            Ok(result) if attempt < MAX_RETRIES - 1 => {
                // Retry with continue extraction prompt
                tracing::warn!("Extraction incomplete, retrying with continue prompt");
                continue;
            }
            Err(e) if attempt < MAX_RETRIES - 1 => {
                tracing::warn!(error = %e, "Parse error, falling back to JSON");
                // Try JSON fallback
                if let Ok(json_result) = self.json_parser.parse(&response.content, &chunk.id) {
                    return Ok(json_result);
                }
                continue;
            }
            result => return result,
        }
    }

    // Return empty result rather than failing completely
    Ok(ExtractionResult::new(&chunk.id))
}
```

---

#### RB-002: System Prompt Support Variability

**Risk Level:** 🟡 Medium

**Description:** Not all LLM providers/models support system prompts the same way. OpenAI has native support, but Ollama and some providers may not.

**Mitigation Strategy:**

```rust
/// LLMProvider trait extension for system prompt support
#[async_trait]
pub trait LLMProviderExt: LLMProvider {
    /// Check if provider supports system prompts
    fn supports_system_prompt(&self) -> bool {
        true // Default: assume support
    }

    /// Complete with optional system prompt
    async fn complete_with_system(
        &self,
        system: &str,
        user: &str,
    ) -> Result<LLMResponse> {
        if self.supports_system_prompt() {
            // Use native system prompt
            self.complete_with_system_native(system, user).await
        } else {
            // Concatenate system + user as single prompt
            let combined = format!("{}\n\n---\n\n{}", system, user);
            self.complete(&combined).await
        }
    }
}
```

---

#### RB-003: Token Limit for Large Entity Descriptions

**Risk Level:** 🟢 Low-Medium

**Description:** When merging descriptions from multiple extractions, the combined text may exceed LLM context limits.

**Mitigation Strategy:**

- Already addressed by MapReduce summarization in Phase 2
- Pre-flight token estimation before LLM calls
- Configurable `force_llm_summary_on_merge` threshold

```rust
/// Check if descriptions need MapReduce summarization
fn needs_mapreduce(&self, descriptions: &[String]) -> bool {
    let total_tokens: usize = descriptions.iter()
        .map(|d| estimate_tokens(d))
        .sum();

    total_tokens > self.config.context_size
        || descriptions.len() >= self.config.force_llm_summary_on_merge
}
```

---

#### RB-004: Entity Name Normalization Conflicts

**Risk Level:** 🟡 Medium

**Description:** Different extractions might produce slightly different entity names (e.g., "John Doe" vs "John D." vs "JOHN_DOE"), leading to fragmented graph nodes.

**Mitigation Strategy:**

```rust
/// Comprehensive entity name normalization
pub fn normalize_entity_name(raw_name: &str) -> String {
    raw_name
        .trim()
        // Remove common suffixes/prefixes that don't add identity
        .trim_start_matches("The ")
        .trim_start_matches("A ")
        .trim_end_matches("'s")
        // Normalize whitespace
        .split_whitespace()
        .filter(|w| !w.is_empty())
        // Convert to title case then uppercase with underscores
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                None => String::new(),
                Some(first) => first.to_uppercase()
                    .chain(chars.flat_map(|c| c.to_lowercase()))
                    .collect(),
            }
        })
        .collect::<Vec<_>>()
        .join("_")
        .to_uppercase()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalization_consistency() {
        assert_eq!(normalize_entity_name("John Doe"), "JOHN_DOE");
        assert_eq!(normalize_entity_name("john doe"), "JOHN_DOE");
        assert_eq!(normalize_entity_name("  John  Doe  "), "JOHN_DOE");
        assert_eq!(normalize_entity_name("The John Doe"), "JOHN_DOE");
    }
}
```

---

#### RB-005: Parallel Processing Race Conditions

**Risk Level:** 🟢 Low

**Description:** Concurrent chunk extraction could lead to race conditions when updating shared state.

**Mitigation Strategy:**

- Use `Arc<RwLock<>>` for shared state
- Semaphore-based concurrency limiting
- Stateless extraction (merge happens after all extractions complete)

```rust
/// Thread-safe parallel extraction
pub async fn extract_parallel(
    &self,
    chunks: &[TextChunk],
) -> Result<Vec<ExtractionResult>> {
    let semaphore = Arc::new(Semaphore::new(self.config.max_concurrent_extractions));

    let futures = chunks.iter().map(|chunk| {
        let sem = semaphore.clone();
        let extractor = self.extractor.clone();
        let chunk = chunk.clone();

        async move {
            // Acquire permit (automatically released on drop)
            let _permit = sem.acquire().await?;
            extractor.extract(&chunk).await
        }
    });

    // Results collected in order, no shared mutable state during extraction
    futures::future::try_join_all(futures).await
}
```

---

#### RB-006: WebSocket Connection Limits

**Risk Level:** 🟡 Medium

**Description:** Too many concurrent WebSocket connections for progress tracking could exhaust server resources.

**Mitigation Strategy:**

- Connection pooling per job ID
- Maximum connections per client IP
- Graceful degradation to polling

```rust
/// WebSocket connection manager with limits
pub struct WsConnectionManager {
    max_connections_per_job: usize,
    max_connections_per_ip: usize,
    connections: DashMap<String, Vec<WsConnection>>,
}

impl WsConnectionManager {
    pub fn can_accept(&self, job_id: &str, client_ip: &str) -> bool {
        let job_count = self.connections.get(job_id)
            .map(|v| v.len())
            .unwrap_or(0);

        let ip_count = self.connections.iter()
            .flat_map(|r| r.value().iter())
            .filter(|c| c.client_ip == client_ip)
            .count();

        job_count < self.max_connections_per_job
            && ip_count < self.max_connections_per_ip
    }
}
```

---

### 11.3 Execution Checklist (No-Roadblock Validation)

Before starting each phase, validate:

```
Phase 1 Pre-flight:
- [x] LLM provider supports required context length (32KB+)
- [x] LLM provider supports system prompts (or has fallback)
- [x] Tuple delimiter `<|#|>` doesn't appear in expected input texts
- [x] Test prompts against target LLM model for format compliance
- [x] Verify async runtime configured for parallel execution

Phase 2 Pre-flight:
- [ ] Estimate maximum description sizes for MapReduce thresholds
- [ ] Test caching backend connectivity (PostgreSQL/Memory)
- [ ] Verify cache key generation doesn't produce collisions

Phase 3 Pre-flight:
- [ ] WebSocket infrastructure ready (if enabled)
- [ ] Cost tracking decimal precision sufficient for micro-costs
- [ ] Event channel buffer sizes adequate for burst traffic

Phase 4 Pre-flight:
- [ ] Database schema supports cascade delete performance
- [ ] Lineage table indexes optimized for common queries
- [ ] Document suppression tested with large document sets

Phase 5 Pre-flight:
- [ ] OpenAPI spec validates against schema
- [ ] E2E test environment matches production configuration
- [ ] Rate limiting tested under load
```

### 11.4 Pragmatic SOTA Principles

This implementation follows pragmatic SOTA principles:

1. **Incremental Improvement**: Hybrid parser allows gradual migration from JSON to tuple format
2. **Graceful Degradation**: Fallbacks at every level (retry → JSON fallback → empty result)
3. **Configuration-Driven**: All thresholds and limits are configurable
4. **Observable**: Comprehensive logging and metrics at critical points
5. **Testable**: Each component independently testable with mocks
6. **Production-Ready**: Rate limiting, cost tracking, and resource management built-in

---

## Appendix: Quick Reference

### Commands

```bash
# Run tests
cargo test --package edgequake-pipeline

# Build with all features
cargo build --release --all-features

# Run specific migration
cargo run --bin migrate -- up 001

# Generate OpenAPI spec
cargo run --bin api -- --openapi-only

# Test SOTA prompts against real LLM
OPENAI_API_KEY="sk-..." cargo test --package edgequake-pipeline sota_prompt_tests -- --nocapture
```

### Feature Flags

```toml
[features]
default = ["parallel", "caching", "sota-prompts"]
parallel = []
caching = []
mapreduce = []
websocket = ["tokio-tungstenite"]
sota-prompts = []  # Enable SOTA tuple-based prompts
legacy-prompts = []  # Keep JSON-based prompts for migration
```

---
