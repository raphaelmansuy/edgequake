# Prompt Comparison: LightRAG (Python) vs EdgeQuake (Rust)

> Document ID: PROMPTS-001
> Version: 1.0
> Created: 2024-12-28

## Table of Contents

1. [Executive Summary](#1-executive-summary)
2. [Entity Extraction Prompts](#2-entity-extraction-prompts)
3. [Gleaning/Continue Extraction Prompts](#3-gleaningcontinue-extraction-prompts)
4. [Description Summarization Prompts](#4-description-summarization-prompts)
5. [Keyword Extraction Prompts](#5-keyword-extraction-prompts)
6. [RAG Response Generation Prompts](#6-rag-response-generation-prompts)
7. [Gap Analysis](#7-gap-analysis)
8. [Recommendations](#8-recommendations)

---

## 1. Executive Summary

### Prompt Categories Comparison

| Category | LightRAG (Python) | EdgeQuake (Rust) | Status |
|----------|-------------------|------------------|--------|
| Entity Extraction | ✅ Comprehensive (100+ lines) | ⚠️ Basic (30 lines) | **Gap** |
| Gleaning/Continue | ✅ Detailed | ⚠️ Basic | **Gap** |
| Description Summary | ✅ With constraints | ✅ Similar | OK |
| Keyword Extraction | ✅ With examples | ✅ Similar | OK |
| RAG Response | ✅ Full with references | ⚠️ Basic | **Gap** |
| Tuple Delimiter | ✅ `<\|#\|>` | ❌ Uses JSON | **Gap** |

### Key Findings

1. **LightRAG uses tuple-based extraction** with `<|#|>` delimiter - more robust for parsing
2. **LightRAG has detailed system prompts** with comprehensive instructions and examples
3. **EdgeQuake uses simpler JSON-based extraction** - easier but less reliable
4. **LightRAG includes multi-language support** in prompts
5. **LightRAG has reference citation system** in RAG responses

---

## 2. Entity Extraction Prompts

### 2.1 LightRAG Entity Extraction System Prompt

**File:** `lightrag/prompt.py` - `PROMPTS["entity_extraction_system_prompt"]`

```python
---Role---
You are a Knowledge Graph Specialist responsible for extracting entities and relationships from the input text.

---Instructions---
1.  **Entity Extraction & Output:**
    *   **Identification:** Identify clearly defined and meaningful entities in the input text.
    *   **Entity Details:** For each identified entity, extract the following information:
        *   `entity_name`: The name of the entity. If the entity name is case-insensitive, capitalize the first letter of each significant word (title case). Ensure **consistent naming** across the entire extraction process.
        *   `entity_type`: Categorize the entity using one of the following types: `{entity_types}`. If none of the provided entity types apply, do not add new entity type and classify it as `Other`.
        *   `entity_description`: Provide a concise yet comprehensive description of the entity's attributes and activities, based *solely* on the information present in the input text.
    *   **Output Format - Entities:** Output a total of 4 fields for each entity, delimited by `{tuple_delimiter}`, on a single line. The first field *must* be the literal string `entity`.
        *   Format: `entity{tuple_delimiter}entity_name{tuple_delimiter}entity_type{tuple_delimiter}entity_description`

2.  **Relationship Extraction & Output:**
    *   **Identification:** Identify direct, clearly stated, and meaningful relationships between previously extracted entities.
    *   **N-ary Relationship Decomposition:** If a single statement describes a relationship involving more than two entities (an N-ary relationship), decompose it into multiple binary (two-entity) relationship pairs for separate description.
        *   **Example:** For "Alice, Bob, and Carol collaborated on Project X," extract binary relationships such as "Alice collaborated with Project X," "Bob collaborated with Project X," and "Carol collaborated with Project X," or "Alice collaborated with Bob," based on the most reasonable binary interpretations.
    *   **Relationship Details:** For each binary relationship, extract the following fields:
        *   `source_entity`: The name of the source entity. Ensure **consistent naming** with entity extraction. Capitalize the first letter of each significant word (title case) if the name is case-insensitive.
        *   `target_entity`: The name of the target entity. Ensure **consistent naming** with entity extraction. Capitalize the first letter of each significant word (title case) if the name is case-insensitive.
        *   `relationship_keywords`: One or more high-level keywords summarizing the overarching nature, concepts, or themes of the relationship. Multiple keywords within this field must be separated by a comma `,`. **DO NOT use `{tuple_delimiter}` for separating multiple keywords within this field.**
        *   `relationship_description`: A concise explanation of the nature of the relationship between the source and target entities, providing a clear rationale for their connection.
    *   **Output Format - Relationships:** Output a total of 5 fields for each relationship, delimited by `{tuple_delimiter}`, on a single line. The first field *must* be the literal string `relation`.
        *   Format: `relation{tuple_delimiter}source_entity{tuple_delimiter}target_entity{tuple_delimiter}relationship_keywords{tuple_delimiter}relationship_description`

3.  **Delimiter Usage Protocol:**
    *   The `{tuple_delimiter}` is a complete, atomic marker and **must not be filled with content**. It serves strictly as a field separator.
    *   **Incorrect Example:** `entity{tuple_delimiter}Tokyo<|location|>Tokyo is the capital of Japan.`
    *   **Correct Example:** `entity{tuple_delimiter}Tokyo{tuple_delimiter}location{tuple_delimiter}Tokyo is the capital of Japan.`

4.  **Relationship Direction & Duplication:**
    *   Treat all relationships as **undirected** unless explicitly stated otherwise. Swapping the source and target entities for an undirected relationship does not constitute a new relationship.
    *   Avoid outputting duplicate relationships.

5.  **Output Order & Prioritization:**
    *   Output all extracted entities first, followed by all extracted relationships.
    *   Within the list of relationships, prioritize and output those relationships that are **most significant** to the core meaning of the input text first.

6.  **Context & Objectivity:**
    *   Ensure all entity names and descriptions are written in the **third person**.
    *   Explicitly name the subject or object; **avoid using pronouns** such as `this article`, `this paper`, `our company`, `I`, `you`, and `he/she`.

7.  **Language & Proper Nouns:**
    *   The entire output (entity names, keywords, and descriptions) must be written in `{language}`.
    *   Proper nouns (e.g., personal names, place names, organization names) should be retained in their original language if a proper, widely accepted translation is not available or would cause ambiguity.

8.  **Completion Signal:** Output the literal string `{completion_delimiter}` only after all entities and relationships, following all criteria, have been completely extracted and outputted.

---Examples---
{examples}

---Real Data to be Processed---
<Input>
Entity_types: [{entity_types}]
Text:
```
{input_text}
```
```

**Delimiter Constants:**
```python
PROMPTS["DEFAULT_TUPLE_DELIMITER"] = "<|#|>"
PROMPTS["DEFAULT_COMPLETION_DELIMITER"] = "<|COMPLETE|>"
```

**Example Output Format:**
```
entity<|#|>Alex<|#|>person<|#|>Alex is a character who experiences frustration and is observant.
entity<|#|>Taylor<|#|>person<|#|>Taylor is portrayed with authoritarian certainty.
relation<|#|>Alex<|#|>Taylor<|#|>power dynamics, observation<|#|>Alex observes Taylor's authoritarian behavior.
<|COMPLETE|>
```

---

### 2.2 EdgeQuake Entity Extraction Prompt

**File:** `edgequake/crates/edgequake-pipeline/src/extractor.rs` - `build_prompt()`

```rust
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
```

---

### 2.3 Comparison Table: Entity Extraction

| Feature | LightRAG | EdgeQuake | Impact |
|---------|----------|-----------|--------|
| Output Format | Tuple with `<\|#\|>` delimiter | JSON | LightRAG more robust for partial outputs |
| N-ary Decomposition | ✅ Explicitly instructed | ❌ Not mentioned | May miss complex relationships |
| Entity Naming | ✅ Title case, consistent naming | ❌ No guidance | Inconsistent entity names |
| Relationship Direction | ✅ Undirected by default | ❌ Not specified | May have duplicate edges |
| Priority Ordering | ✅ Most significant first | ❌ Not specified | Less optimal retrieval |
| Third Person | ✅ Required | ❌ Not specified | May have pronouns |
| Language Support | ✅ Multi-language with `{language}` | ❌ Not supported | English only |
| Completion Signal | ✅ `<\|COMPLETE\|>` | ❌ Not supported | Harder to detect incomplete |
| Examples | ✅ 3 detailed examples | ❌ None | Less accurate extraction |
| Lines of Code | ~100 lines | ~20 lines | - |

---

## 3. Gleaning/Continue Extraction Prompts

### 3.1 LightRAG Continue Extraction Prompt

**File:** `lightrag/prompt.py` - `PROMPTS["entity_continue_extraction_user_prompt"]`

```python
---Task---
Based on the last extraction task, identify and extract any **missed or incorrectly formatted** entities and relationships from the input text.

---Instructions---
1.  **Strict Adherence to System Format:** Strictly adhere to all format requirements for entity and relationship lists, including output order, field delimiters, and proper noun handling, as specified in the system instructions.
2.  **Focus on Corrections/Additions:**
    *   **Do NOT** re-output entities and relationships that were **correctly and fully** extracted in the last task.
    *   If an entity or relationship was **missed** in the last task, extract and output it now according to the system format.
    *   If an entity or relationship was **truncated, had missing fields, or was otherwise incorrectly formatted** in the last task, re-output the *corrected and complete* version in the specified format.
3.  **Output Format - Entities:** Output a total of 4 fields for each entity, delimited by `{tuple_delimiter}`, on a single line. The first field *must* be the literal string `entity`.
4.  **Output Format - Relationships:** Output a total of 5 fields for each relationship, delimited by `{tuple_delimiter}`, on a single line. The first field *must* be the literal string `relation`.
5.  **Output Content Only:** Output *only* the extracted list of entities and relationships. Do not include any introductory or concluding remarks, explanations, or additional text before or after the list.
6.  **Completion Signal:** Output `{completion_delimiter}` as the final line after all relevant missing or corrected entities and relationships have been extracted and presented.
7.  **Output Language:** Ensure the output language is {language}. Proper nouns (e.g., personal names, place names, organization names) must be kept in their original language and not translated.
```

---

### 3.2 EdgeQuake Gleaning Prompt

**File:** `edgequake/crates/edgequake-pipeline/src/extractor.rs` - `build_gleaning_prompt()`

```rust
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
```

---

### 3.3 Comparison Table: Gleaning

| Feature | LightRAG | EdgeQuake | Impact |
|---------|----------|-----------|--------|
| Deduplication | ✅ Explicit "don't re-output" | ⚠️ "Already identified" list | EdgeQuake still lists entities |
| Error Correction | ✅ Fix truncated/malformed | ❌ Not mentioned | May propagate errors |
| Format Adherence | ✅ Strict adherence | ⚠️ Same format request | - |
| Implicit Entities | ❌ Not specifically mentioned | ✅ Explicitly requested | EdgeQuake better here |
| Contextual Entities | ❌ Not mentioned | ✅ Dates, locations, concepts | EdgeQuake better here |

---

## 4. Description Summarization Prompts

### 4.1 LightRAG Summary Prompt

**File:** `lightrag/prompt.py` - `PROMPTS["summarize_entity_descriptions"]`

```python
---Role---
You are a Knowledge Graph Specialist, proficient in data curation and synthesis.

---Task---
Your task is to synthesize a list of descriptions of a given entity or relation into a single, comprehensive, and cohesive summary.

---Instructions---
1. Input Format: The description list is provided in JSON format. Each JSON object (representing a single description) appears on a new line within the `Description List` section.
2. Output Format: The merged description will be returned as plain text, presented in multiple paragraphs, without any additional formatting or extraneous comments before or after the summary.
3. Comprehensiveness: The summary must integrate all key information from *every* provided description. Do not omit any important facts or details.
4. Context: Ensure the summary is written from an objective, third-person perspective; explicitly mention the name of the entity or relation for full clarity and context.
5. Context & Objectivity:
  - Write the summary from an objective, third-person perspective.
  - Explicitly mention the full name of the entity or relation at the beginning of the summary to ensure immediate clarity and context.
6. Conflict Handling:
  - In cases of conflicting or inconsistent descriptions, first determine if these conflicts arise from multiple, distinct entities or relationships that share the same name.
  - If distinct entities/relations are identified, summarize each one *separately* within the overall output.
  - If conflicts within a single entity/relation (e.g., historical discrepancies) exist, attempt to reconcile them or present both viewpoints with noted uncertainty.
7. Length Constraint: The summary's total length must not exceed {summary_length} tokens, while still maintaining depth and completeness.
8. Language: The entire output must be written in {language}. Proper nouns (e.g., personal names, place names, organization names) may in their original language if proper translation is not available.

---Input---
{description_type} Name: {description_name}

Description List:
```
{description_list}
```

---Output---
```

---

### 4.2 EdgeQuake Summary Prompts

**File:** `edgequake/crates/edgequake-pipeline/src/summarizer.rs`

**Entity Summary:**
```rust
let prompt = format!(
    r#"You are a helpful assistant responsible for generating a comprehensive summary of the data provided below.

Given one or more descriptions of an entity, generate a single comprehensive description that:
1. Captures all unique information from the input descriptions
2. Resolves any contradictions by preferring more specific information
3. Is written in a clear, coherent style
4. Does not exceed 500 words

# Entity: {entity_name}

# Descriptions:
{descriptions_text}

# Comprehensive Summary:
"#
);
```

**Relationship Summary:**
```rust
let prompt = format!(
    r#"You are a helpful assistant responsible for generating a comprehensive summary of the relationship between two entities.

Given one or more descriptions of a relationship between "{source}" and "{target}", generate a single comprehensive description that:
1. Captures all unique information about their relationship
2. Resolves any contradictions by preferring more specific information
3. Is written in a clear, coherent style
4. Does not exceed 200 words

# Relationship: {source} → {target}

# Descriptions:
{descriptions_text}

# Comprehensive Summary:
"#
);
```

---

### 4.3 Comparison Table: Summarization

| Feature | LightRAG | EdgeQuake | Impact |
|---------|----------|-----------|--------|
| Conflict Handling | ✅ Detect same-name entities | ⚠️ "Prefer more specific" | May merge different entities |
| Third Person | ✅ Required | ❌ Not specified | Style inconsistency |
| Length Constraint | ✅ Token-based `{summary_length}` | ✅ Word-based (500/200) | Both acceptable |
| Language Support | ✅ Multi-language | ❌ English only | Limitation |
| JSON Input | ✅ Structured input | ✅ Numbered list | Both acceptable |
| Entity Name at Start | ✅ Required | ❌ Not specified | Context clarity |

---

## 5. Keyword Extraction Prompts

### 5.1 LightRAG Keyword Extraction

**File:** `lightrag/prompt.py` - `PROMPTS["keywords_extraction"]`

```python
---Role---
You are an expert keyword extractor, specializing in analyzing user queries for a Retrieval-Augmented Generation (RAG) system. Your purpose is to identify both high-level and low-level keywords in the user's query that will be used for effective document retrieval.

---Goal---
Given a user query, your task is to extract two distinct types of keywords:
1. **high_level_keywords**: for overarching concepts or themes, capturing user's core intent, the subject area, or the type of question being asked.
2. **low_level_keywords**: for specific entities or details, identifying the specific entities, proper nouns, technical jargon, product names, or concrete items.

---Instructions & Constraints---
1. **Output Format**: Your output MUST be a valid JSON object and nothing else. Do not include any explanatory text, markdown code fences (like ```json), or any other text before or after the JSON. It will be parsed directly by a JSON parser.
2. **Source of Truth**: All keywords must be explicitly derived from the user query, with both high-level and low-level keyword categories are required to contain content.
3. **Concise & Meaningful**: Keywords should be concise words or meaningful phrases. Prioritize multi-word phrases when they represent a single concept. For example, from "latest financial report of Apple Inc.", you should extract "latest financial report" and "Apple Inc." rather than "latest", "financial", "report", and "Apple".
4. **Handle Edge Cases**: For queries that are too simple, vague, or nonsensical (e.g., "hello", "ok", "asdfghjkl"), you must return a JSON object with empty lists for both keyword types.

---Examples---
{examples}

---Real Data---
User Query: {query}

---Output---
Output:
```

---

### 5.2 EdgeQuake Keyword Extraction

**File:** `edgequake/crates/edgequake-query/src/keywords.rs` - `build_prompt()`

```rust
fn build_prompt(&self, query: &str) -> String {
    format!(
        r#"Extract high-level and low-level keywords from the following query.

High-level keywords are abstract concepts, themes, or topics (e.g., "artificial intelligence", "climate change", "software architecture").
Low-level keywords are specific entities, technical terms, or proper nouns (e.g., "GPT-4", "neural network", "PostgreSQL").

Query: "{query}"

Respond ONLY with valid JSON in this exact format:
{{
  "high_level_keywords": ["concept1", "concept2"],
  "low_level_keywords": ["entity1", "entity2", "term1"]
}}

Examples:

Query: "How does machine learning improve healthcare outcomes?"
{{
  "high_level_keywords": ["machine learning", "healthcare", "outcomes", "improvement"],
  "low_level_keywords": ["ML algorithms", "medical diagnosis", "patient data"]
}}

Query: "What is the relationship between OpenAI and Microsoft?"
{{
  "high_level_keywords": ["business relationship", "partnership", "collaboration"],
  "low_level_keywords": ["OpenAI", "Microsoft", "GPT", "Azure"]
}}

Query: "Explain quantum computing applications in cryptography"
{{
  "high_level_keywords": ["quantum computing", "applications", "cryptography", "security"],
  "low_level_keywords": ["qubits", "Shor's algorithm", "quantum key distribution", "encryption"]
}}

Now extract keywords from the query above. Respond with JSON only:"#
    )
}
```

---

### 5.3 Comparison Table: Keywords

| Feature | LightRAG | EdgeQuake | Impact |
|---------|----------|-----------|--------|
| Role Definition | ✅ "Expert keyword extractor" | ❌ None | Less context |
| Multi-word Phrases | ✅ Explicitly instructed | ❌ Not mentioned | May split phrases |
| Edge Cases | ✅ "hello", "ok" = empty | ❌ Not handled | May return garbage |
| Examples | ✅ 3 examples | ✅ 3 examples | Both good |
| JSON Strictness | ✅ "No code fences" | ✅ "JSON only" | Both good |

---

## 6. RAG Response Generation Prompts

### 6.1 LightRAG RAG Response Prompt

**File:** `lightrag/prompt.py` - `PROMPTS["rag_response"]`

```python
---Role---
You are an expert AI assistant specializing in synthesizing information from a provided knowledge base. Your primary function is to answer user queries accurately by ONLY using the information within the provided **Context**.

---Goal---
Generate a comprehensive, well-structured answer to the user query.
The answer must integrate relevant facts from the Knowledge Graph and Document Chunks found in the **Context**.
Consider the conversation history if provided to maintain conversational flow and avoid repeating information.

---Instructions---
1. Step-by-Step Instruction:
  - Carefully determine the user's query intent in the context of the conversation history to fully understand the user's information need.
  - Scrutinize both `Knowledge Graph Data` and `Document Chunks` in the **Context**. Identify and extract all pieces of information that are directly relevant to answering the user query.
  - Weave the extracted facts into a coherent and logical response. Your own knowledge must ONLY be used to formulate fluent sentences and connect ideas, NOT to introduce any external information.
  - Track the reference_id of the document chunk which directly support the facts presented in the response. Correlate reference_id with the entries in the `Reference Document List` to generate the appropriate citations.
  - Generate a references section at the end of the response. Each reference document must directly support the facts presented in the response.
  - Do not generate anything after the reference section.

2. Content & Grounding:
  - Strictly adhere to the provided context from the **Context**; DO NOT invent, assume, or infer any information not explicitly stated.
  - If the answer cannot be found in the **Context**, state that you do not have enough information to answer. Do not attempt to guess.

3. Formatting & Language:
  - The response MUST be in the same language as the user query.
  - The response MUST utilize Markdown formatting for enhanced clarity and structure (e.g., headings, bold text, bullet points).
  - The response should be presented in {response_type}.

4. References Section Format:
  - The References section should be under heading: `### References`
  - Reference list entries should adhere to the format: `* [n] Document Title`. Do not include a caret (`^`) after opening square bracket (`[`).
  - The Document Title in the citation must retain its original language.
  - Output each citation on an individual line
  - Provide maximum of 5 most relevant citations.
  - Do not generate footnotes section or any comment, summary, or explanation after the references.

5. Reference Section Example:
```
### References

- [1] Document Title One
- [2] Document Title Two
- [3] Document Title Three
```

6. Additional Instructions: {user_prompt}

---Context---
{context_data}
```

---

### 6.2 EdgeQuake RAG Response Prompt

**File:** `edgequake/crates/edgequake-query/src/engine.rs` - `build_prompt()` and `query_stream()`

```rust
fn build_prompt(&self, query: &str, context: &QueryContext) -> String {
    let context_text = context.to_context_string();

    format!(
        r#"You are a helpful assistant. Answer the user's question based on the following context.

## Context
{context_text}

## Question
{query}

## Answer
Provide a clear, accurate answer based on the context above. If the context doesn't contain enough information to answer the question, say so."#
    )
}
```

---

### 6.3 Comparison Table: RAG Response

| Feature | LightRAG | EdgeQuake | Impact |
|---------|----------|-----------|--------|
| Role Definition | ✅ "Expert AI assistant" | ⚠️ "Helpful assistant" | Less authoritative |
| Grounding Instructions | ✅ "ONLY context, no external" | ⚠️ Implicit | May hallucinate |
| References/Citations | ✅ Full citation system | ❌ None | No traceability |
| Conversation History | ✅ Supported | ❌ Not mentioned | Stateless |
| Markdown Formatting | ✅ Required | ❌ Not specified | Inconsistent output |
| Language Match | ✅ "Same language as query" | ❌ Not specified | English only |
| Knowledge Graph Context | ✅ Entities + Relationships | ✅ Context string | Both included |
| Max Citations | ✅ 5 max | ❌ N/A | - |
| Response Type | ✅ Configurable `{response_type}` | ❌ Not configurable | - |
| Lines of Code | ~60 lines | ~15 lines | - |

---

## 7. Gap Analysis

### 7.1 Critical Gaps

| Gap ID | Description | Severity | Recommendation |
|--------|-------------|----------|----------------|
| GAP-P01 | No tuple delimiter extraction format | 🔴 High | Implement tuple parser |
| GAP-P02 | No completion signal detection | 🔴 High | Add `<\|COMPLETE\|>` support |
| GAP-P03 | No reference/citation system | 🔴 High | Add reference tracking |
| GAP-P04 | No multi-language support | 🟡 Medium | Add `{language}` parameter |
| GAP-P05 | No entity naming conventions | 🟡 Medium | Add title case instructions |
| GAP-P06 | No N-ary relationship decomposition | 🟡 Medium | Add to extraction prompt |
| GAP-P07 | No conversation history | 🟡 Medium | Add history parameter |
| GAP-P08 | No edge case handling for keywords | 🟢 Low | Add empty list fallback |

### 7.2 Strengths of EdgeQuake

| Feature | Description |
|---------|-------------|
| JSON Parsing | More structured, easier to validate |
| Implicit Entity Focus | Gleaning prompt explicitly targets implicit entities |
| Contextual Entities | Gleaning requests dates, locations, concepts |
| Simpler Prompts | Easier to modify and maintain |

---

## 8. Recommendations

### 8.1 High Priority (Implement in Phase 1)

1. **Add Tuple-Based Extraction Format**
   
   Create a new extraction format using `<|#|>` delimiter for more robust parsing:
   
   ```rust
   // New prompt format
   entity<|#|>ALICE_CHEN<|#|>person<|#|>A software engineer at TechCorp
   relation<|#|>ALICE_CHEN<|#|>TECHCORP<|#|>employment<|#|>Works at the company
   <|COMPLETE|>
   ```

2. **Add Reference/Citation System**
   
   Track source chunks and generate citations in RAG responses.

3. **Add Completion Signal Detection**
   
   Check for `<|COMPLETE|>` to detect incomplete extractions.

### 8.2 Medium Priority (Implement in Phase 2)

1. **Add Entity Naming Conventions**
   - Title case for names
   - Consistent naming across extraction
   - Third person perspective

2. **Add Multi-Language Support**
   - `language` parameter in prompts
   - Proper noun preservation

3. **Add N-ary Relationship Decomposition**
   - Instructions for breaking complex relationships into binary pairs

### 8.3 Low Priority (Implement in Phase 3)

1. **Add Conversation History**
   - Support for multi-turn conversations
   - Context carry-over

2. **Add Edge Case Handling**
   - Empty/nonsense queries return empty keywords

---

## Appendix A: Full LightRAG Examples

### Entity Extraction Example 1 (Narrative Text)

```
<Input Text>
while Alex clenched his jaw, the buzz of frustration dull against the backdrop of Taylor's authoritarian certainty. It was this competitive undercurrent that kept him alert...

<Output>
entity<|#|>Alex<|#|>person<|#|>Alex is a character who experiences frustration and is observant of the dynamics among other characters.
entity<|#|>Taylor<|#|>person<|#|>Taylor is portrayed with authoritarian certainty and shows a moment of reverence towards a device.
relation<|#|>Alex<|#|>Taylor<|#|>power dynamics, observation<|#|>Alex observes Taylor's authoritarian behavior.
<|COMPLETE|>
```

### Entity Extraction Example 2 (Financial Text)

```
<Input Text>
Stock markets faced a sharp downturn today as tech giants saw significant declines, with the global tech index dropping by 3.4%...

<Output>
entity<|#|>Global Tech Index<|#|>category<|#|>The Global Tech Index tracks the performance of major technology stocks.
entity<|#|>Nexon Technologies<|#|>organization<|#|>Nexon Technologies is a tech company that saw its stock decline by 7.8%.
relation<|#|>Global Tech Index<|#|>Market Selloff<|#|>market performance<|#|>The decline is part of the broader market selloff.
<|COMPLETE|>
```

---

## Appendix B: Prompt Template Variables

### LightRAG Variables

| Variable | Description | Example |
|----------|-------------|---------|
| `{tuple_delimiter}` | Field separator | `<\|#\|>` |
| `{completion_delimiter}` | End signal | `<\|COMPLETE\|>` |
| `{entity_types}` | Allowed types | `person, organization, location` |
| `{language}` | Output language | `English` |
| `{input_text}` | Text to process | Document content |
| `{examples}` | Few-shot examples | 3 detailed examples |
| `{summary_length}` | Max tokens | `500` |
| `{response_type}` | Answer format | `detailed paragraphs` |
| `{context_data}` | RAG context | Entities + chunks |

### EdgeQuake Variables

| Variable | Description | Example |
|----------|-------------|---------|
| `{entity_types_str}` | Allowed types | `PERSON, ORGANIZATION` |
| `{text}` | Text to process | Document content |
| `{query}` | User question | Query string |
| `{context_text}` | RAG context | Formatted context |
| `{entity_name}` | Entity being summarized | `ALICE_CHEN` |
| `{prev_entities_str}` | Already extracted | `ALICE, BOB` |

---
