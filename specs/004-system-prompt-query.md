# SPEC-004: System Prompt Extension Point for Queries

> **Issue**: [#70 — Add a system prompt extension point to query](https://github.com/raphaelmansuy/edgequake/issues/70)
> **Status**: Draft
> **Priority**: High
> **Complexity**: Medium

## Summary

Add an optional `system_prompt` field to query and chat completion requests. This field **extends** (does not replace) the existing RAG system prompt, allowing users to customize LLM behavior per-query (e.g., persona, tone, language constraints, domain-specific rules).

### Constraint (from issue)

> "It doesn't replace or supersede the system prompt — it will add an extension point to the system prompt."

---

## Context

### Why This Matters

The current system prompt is hardcoded in `build_prompt()` and optimized for general-purpose RAG responses. Users need the ability to:

1. **Set a persona**: "You are a legal consultant specializing in EU GDPR compliance."
2. **Constrain output format**: "Always respond with numbered bullet points."
3. **Add domain rules**: "When mentioning pharmaceutical compounds, include the CAS number."
4. **Restrict scope**: "Only answer questions about the 2024 financial reports."
5. **Per-workspace defaults**: Store a persistent system prompt at workspace level.

### Current Architecture

#### Prompt Construction Flow

```
API Handler (chat/streaming.rs)
  → Build EngineQueryRequest::new(&query)
  → Dispatch to SOTAQueryEngine method
    → Keyword extraction → Mode selection → Vector retrieval → Context building → Truncation
    → build_prompt(query, context)  ← HARDCODED system prompt here
    → llm_provider.complete(&prompt) OR llm_provider.stream(&prompt)
```

#### Current `build_prompt()` Template Structure

```
---Role---
You are an expert AI assistant specializing in synthesizing information...

---Goal---
Generate a comprehensive, well-structured answer...

---Instructions---
1. Step-by-Step Reasoning: ...
2. Content & Grounding: ...
3. Formatting & Language: ...

---Context---
{context_text}

---User Query---
{query}
```

#### Key Technical Facts

| Fact                     | Detail                                                                                                                                                 |
| ------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------ |
| LLM call pattern         | `llm_provider.complete(&prompt)` — single string, not chat messages                                                                                    |
| Streaming call           | `llm_provider.stream(&prompt)` — same single string pattern                                                                                            |
| `chat()` API exists      | Used in `title_generator.rs` with `ChatMessage::system()` + `ChatMessage::user()`                                                                      |
| `edgequake-llm`          | External crate v0.3.0 from crates.io (not a local path dependency)                                                                                     |
| No `system_prompt` field | Absent from all DTOs: engine `QueryRequest`, API `QueryRequest`, `ChatCompletionRequest`, `StreamQueryRequest`, and TypeScript `ChatCompletionRequest` |
| Prompt is pure string    | The system prompt, context, and user query are concatenated into one string                                                                            |

---

## Implementation Options Evaluated

### Option A: Prompt Template Injection (Recommended)

Add a `---Additional Instructions---` section to the existing prompt template string, placed between `---Instructions---` and `---Context---`.

```
---Instructions---
1. Step-by-Step Reasoning: ...
2. Content & Grounding: ...
3. Formatting & Language: ...

---Additional Instructions---          ← NEW SECTION (only if system_prompt is provided)
{user_system_prompt}

---Context---
{context_text}

---User Query---
{query}
```

| Criterion        | Assessment                                                                  |
| ---------------- | --------------------------------------------------------------------------- |
| Change surface   | **Small** — Only `build_prompt()` + DTO fields + frontend                   |
| Regression risk  | **Low** — When `system_prompt` is `None`, output is byte-identical to today |
| Streaming impact | **None** — `stream(&prompt)` still receives a single string                 |
| Backward compat  | **Full** — New field is optional, defaults to `None`                        |
| LLM behavior     | Good — Instructions section is well-positioned for following                |

### Option B: Switch to Chat API with Separate System Messages

Refactor the entire query pipeline from `complete(&prompt)` to `chat(&[ChatMessage::system(base), ChatMessage::system(extension), ChatMessage::user(query)])`.

| Criterion        | Assessment                                                                  |
| ---------------- | --------------------------------------------------------------------------- |
| Change surface   | **Large** — All query entry points (8+ methods), streaming, generation      |
| Regression risk  | **High** — Fundamentally changes how LLM receives instructions              |
| Streaming impact | **Requires rewrite** — Streaming uses `stream(&prompt)` not `chat_stream()` |
| Backward compat  | **Behavioral change** — LLM may respond differently to chat vs. complete    |
| LLM behavior     | Better — Separate system role has higher instruction-following priority     |

### Option C: Workspace-Level Only (No Per-Query Override)

Store `system_prompt` as a workspace setting in PostgreSQL, apply automatically to all queries for that workspace.

| Criterion    | Assessment                                                          |
| ------------ | ------------------------------------------------------------------- |
| Flexibility  | **Low** — Cannot vary prompt per query                              |
| Use case fit | **Partial** — Covers "workspace persona" but not "per-query format" |

### Decision: Option A (Prompt Template Injection)

**Rationale:**

1. The issue says "extend" — a new template section is the literal implementation of that.
2. Zero impact on streaming paths (`stream(&prompt)` unchanged).
3. When `system_prompt` is `None`, prompt output is identical to today (byte-for-byte).
4. Migration to Option B (chat API) can be spec'd independently later.
5. Minimal change surface = minimal regression risk.

> **Future consideration**: Option B (chat API migration) is a worthwhile separate spec. It would unlock proper system message role semantics. This spec intentionally keeps the scope narrow.

---

## Detailed Design

### 1. Engine-Level QueryRequest

**File**: `edgequake/crates/edgequake-query/src/engine.rs`

Add `system_prompt` field to `QueryRequest`:

```rust
/// A query request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryRequest {
    // ... existing fields ...

    /// Optional system prompt extension. Appended to the base RAG instructions
    /// as an "Additional Instructions" section. Does NOT replace the built-in
    /// system prompt.
    ///
    /// Use cases: persona, tone, output format constraints, domain rules.
    ///
    /// Max length: 4000 tokens (~16000 characters). Truncated with warning if exceeded.
    ///
    /// @implements SPEC-004: System prompt extension point
    #[serde(default)]
    pub system_prompt: Option<String>,
}
```

Add builder method:

```rust
impl QueryRequest {
    /// Set a system prompt extension for this query.
    /// This is appended to the base RAG instructions, not a replacement.
    /// @implements SPEC-004
    pub fn with_system_prompt(mut self, prompt: impl Into<String>) -> Self {
        self.system_prompt = Some(prompt.into());
        self
    }
}
```

Update `new()` to initialize:

```rust
pub fn new(query: impl Into<String>) -> Self {
    Self {
        // ... existing fields ...
        system_prompt: None,
    }
}
```

### 2. Prompt Construction

**File**: `edgequake/crates/edgequake-query/src/sota_engine/prompt.rs`

Modify `build_prompt()` signature and template:

```rust
/// Build prompt for LLM.
///
/// If `system_prompt_extension` is provided, it is injected as an
/// "Additional Instructions" section between the base instructions
/// and the context. This extends (does not replace) the base prompt.
///
/// @implements SPEC-004: System prompt extension point
pub(super) fn build_prompt(
    &self,
    query: &str,
    context: &QueryContext,
    system_prompt_extension: Option<&str>,
) -> String {
    if context.is_empty() {
        return "I'm sorry, but I couldn't find any relevant information in my knowledge base to answer your question.".to_string();
    }

    let context_text = context.to_context_string();

    // Build the additional instructions section only if provided
    let additional_instructions = match system_prompt_extension {
        Some(ext) if !ext.trim().is_empty() => {
            format!(
                "\n\n---Additional Instructions---\n\n{}\n",
                ext.trim()
            )
        }
        _ => String::new(),
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
```

### 3. All Callers of `build_prompt()`

Every method that calls `self.build_prompt(query, context)` must be updated to pass `system_prompt_extension`. The `QueryRequest.system_prompt` is threaded through.

**Files affected** (all in `edgequake/crates/edgequake-query/src/sota_engine/`):

| File                             | Methods                                                                                                                   |
| -------------------------------- | ------------------------------------------------------------------------------------------------------------------------- |
| `prompt.rs`                      | `generate_answer_with_provider()`, `generate_answer()`                                                                    |
| `query_entry/query_basic.rs`     | `query()`                                                                                                                 |
| `query_entry/query_workspace.rs` | `query_with_workspace_config()`, `query_with_full_config()`                                                               |
| `query_entry/query_stream.rs`    | `query_stream()`, `query_stream_with_context()`, `query_stream_with_context_and_llm()`, `query_stream_with_full_config()` |

**Pattern for each caller:**

```rust
// Before:
let prompt = self.build_prompt(&request.query, &final_context);

// After:
let prompt = self.build_prompt(
    &request.query,
    &final_context,
    request.system_prompt.as_deref(),
);
```

For `generate_answer_with_provider()`:

```rust
// Before:
pub(super) async fn generate_answer_with_provider(
    &self,
    query: &str,
    context: &QueryContext,
    llm_override: Option<&Arc<dyn crate::LLMProvider>>,
) -> Result<(String, usize)> {

// After:
pub(super) async fn generate_answer_with_provider(
    &self,
    query: &str,
    context: &QueryContext,
    llm_override: Option<&Arc<dyn crate::LLMProvider>>,
    system_prompt_extension: Option<&str>,
) -> Result<(String, usize)> {
    // ...
    let prompt = self.build_prompt(query, context, system_prompt_extension);
    // ...
}
```

### 4. API DTOs

#### 4.1 Query Request DTO

**File**: `edgequake/crates/edgequake-api/src/handlers/query_types.rs`

```rust
/// Query request.
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct QueryRequest {
    // ... existing fields ...

    /// Optional system prompt extension appended to the base RAG instructions.
    /// Does NOT replace the built-in system prompt — it extends it.
    ///
    /// Use cases: persona ("You are a legal advisor"), output format constraints
    /// ("Always respond with numbered lists"), domain rules, scope restrictions.
    ///
    /// Max length: 16000 characters (~4000 tokens). Silently truncated if exceeded.
    ///
    /// @implements SPEC-004: System prompt extension point
    #[serde(default)]
    pub system_prompt: Option<String>,
}
```

#### 4.2 Streaming Query Request DTO

**File**: `edgequake/crates/edgequake-api/src/handlers/query_types.rs`

```rust
/// Streaming query request.
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct StreamQueryRequest {
    pub query: String,
    #[serde(default)]
    pub mode: Option<String>,

    /// @implements SPEC-004
    #[serde(default)]
    pub system_prompt: Option<String>,
}
```

#### 4.3 Chat Completion Request DTO

**File**: `edgequake/crates/edgequake-api/src/handlers/chat_types.rs`

```rust
/// Unified chat completion request.
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct ChatCompletionRequest {
    // ... existing fields ...

    /// Optional system prompt extension appended to the base RAG instructions.
    /// Does NOT replace the built-in system prompt — it extends it.
    ///
    /// @implements SPEC-004: System prompt extension point
    #[serde(default)]
    pub system_prompt: Option<String>,
}
```

### 5. API Handlers — Threading system_prompt to Engine Request

All 4 handlers must pass `system_prompt` from the API DTO to the engine `QueryRequest`.

#### 5.1 Streaming Chat Handler

**File**: `edgequake/crates/edgequake-api/src/handlers/chat/streaming.rs`

```rust
// In the handler, after building engine_request:
let mut engine_request = EngineQueryRequest::new(&enriched_query).with_mode(query_mode);

// ADD: Thread system_prompt through
if let Some(ref sp) = request.system_prompt {
    engine_request = engine_request.with_system_prompt(sp.clone());
}
```

#### 5.2 Non-Streaming Chat Handler

**File**: `edgequake/crates/edgequake-api/src/handlers/chat/completion.rs`

Same pattern as 5.1.

#### 5.3 Query Execute Handler

**File**: `edgequake/crates/edgequake-api/src/handlers/query/query_execute.rs`

```rust
// When converting API QueryRequest to Engine QueryRequest:
if let Some(ref sp) = request.system_prompt {
    engine_request = engine_request.with_system_prompt(sp.clone());
}
```

#### 5.4 Query Stream Handler

**File**: `edgequake/crates/edgequake-api/src/handlers/query/query_stream.rs`

Same pattern.

#### 5.5 Ollama-Compatible Handlers

The Ollama-compatible API already accepts a `system` field in both `OllamaChatRequest` and `OllamaGenerateRequest` (defined in `ollama_types.rs`). These handlers build `EngineQueryRequest` internally but currently ignore the `system` field.

**Files**:

- `edgequake/crates/edgequake-api/src/handlers/ollama/chat.rs`
- `edgequake/crates/edgequake-api/src/handlers/ollama/generate.rs`

```rust
// Map existing Ollama `system` field to the engine's system_prompt
if let Some(ref system) = request.system {
    engine_request = engine_request.with_system_prompt(system.clone());
}
```

> **Note**: `ollama_types.rs` already has `system: Option<String>` on both request types — no DTO change needed. Only the handlers need the threading line above.

### 6. Validation & Safety

**File**: `edgequake/crates/edgequake-api/src/handlers/chat/streaming.rs` (and all other handlers)

Add validation before processing:

```rust
/// Maximum system prompt length in characters (~4000 tokens).
const MAX_SYSTEM_PROMPT_CHARS: usize = 16_000;

// In handler, after extracting request:
let system_prompt = request.system_prompt.as_ref().map(|sp| {
    let trimmed = sp.trim();
    if trimmed.len() > MAX_SYSTEM_PROMPT_CHARS {
        warn!(
            original_len = trimmed.len(),
            max_len = MAX_SYSTEM_PROMPT_CHARS,
            "System prompt truncated to max length"
        );
        &trimmed[..MAX_SYSTEM_PROMPT_CHARS]
    } else {
        trimmed
    }
});
```

**Considerations:**

- Max 16,000 characters (~4,000 tokens) to prevent context window exhaustion
- Trimmed of leading/trailing whitespace
- Empty strings treated as `None`
- No special character filtering needed — the prompt is injected into a clearly delineated template section, not into SQL or shell commands

### 6.1 Interaction with the `language` Field

The existing `language` field (e.g. `"fr"`, `"zh"`) is currently injected as a directive **appended to the query text**:

```
{user_query}

[IMPORTANT: You MUST respond in French]
```

Meanwhile, the base system prompt says:

> "The response MUST be in the same language as the user query."

A user's `system_prompt` can now also contain language instructions (e.g. `"Always respond in Spanish"`). This creates a potential three-way conflict:

| Source                     | Location in prompt                      | Typical strength                       |
| -------------------------- | --------------------------------------- | -------------------------------------- |
| Base system prompt         | `---Instructions---` section, rule 3    | Weakest (generic, "same as query")     |
| `system_prompt` extension  | `---Additional Instructions---` section | Medium (explicit user instruction)     |
| `language` field directive | Appended to `---User Query---` text     | Strongest (closest to query, ALL-CAPS) |

**Design decision**: The `language` field directive wins by position (it's the last instruction the LLM sees, right after the query). This is the correct default because:

1. The `language` field is set automatically from the UI locale — users expect the response in their UI language.
2. If a user explicitly sets both `system_prompt: "respond in Spanish"` **and** `language: "en"`, the `language` field should win because it's a deliberate UI-level choice.
3. If the user wants the `system_prompt` to control language, they simply **omit** the `language` field (set it to `null`/`undefined`).

**No code change needed** — the position-based precedence is already correct. The spec documents this so users understand the interaction:

> **Tip**: To let your system prompt control the response language, set `language` to `null` in the request. Otherwise the `language` field takes precedence.

### 7. Frontend Changes

#### 7.1 TypeScript API Types

**File**: `edgequake_webui/src/lib/api/chat.ts`

```typescript
export interface ChatCompletionRequest {
  // ... existing fields ...

  /**
   * Optional system prompt extension. Appended to the base RAG instructions.
   * Does NOT replace the built-in system prompt.
   *
   * Examples:
   * - "You are a legal advisor specializing in EU GDPR."
   * - "Always respond with numbered bullet points."
   * - "Only discuss topics from the 2024 annual report."
   *
   * Max length: 16000 characters.
   * @implements SPEC-004: System prompt extension point
   */
  system_prompt?: string;
}
```

#### 7.2 Settings Store

**File**: `edgequake_webui/src/stores/use-settings-store.ts`

Add `systemPrompt` to `QuerySettings`:

```typescript
// In types/index.ts or wherever QuerySettings is defined:
export interface QuerySettings {
  // ... existing fields ...
  /** Persistent system prompt for all queries (workspace-level default). */
  systemPrompt?: string;
}

const defaultQuerySettings: QuerySettings = {
  // ... existing defaults ...
  systemPrompt: undefined,
};
```

#### 7.3 Query Interface — Passing system_prompt

**File**: `edgequake_webui/src/components/query/query-interface.tsx`

In the `handleSend` function (or equivalent), include `system_prompt` from settings store:

```typescript
const { querySettings } = useSettingsStore();

// When building the request:
const request: ChatCompletionRequest = {
  message: input,
  conversation_id: activeConversationId,
  mode: querySettings.mode,
  stream: querySettings.stream,
  // ... other fields ...
  system_prompt: querySettings.systemPrompt || undefined,
};
```

#### 7.4 Settings Sheet — System Prompt Input

**File**: `edgequake_webui/src/components/query/query-settings-sheet.tsx`

Add a textarea in the "Generation" section of the settings sheet:

```tsx
{
  /* System Prompt Extension */
}
<div className="space-y-2">
  <div className="flex items-center gap-1.5">
    <Label className="text-sm font-medium">
      {t("query.settings.systemPrompt", "System Prompt")}
    </Label>
    <TooltipProvider>
      <Tooltip>
        <TooltipTrigger aria-label="System prompt help">
          <Info className="h-3 w-3 text-muted-foreground" />
        </TooltipTrigger>
        <TooltipContent side="top" className="max-w-[250px]">
          <p className="text-xs">
            {t(
              "query.settings.systemPromptHint",
              "Custom instructions added to the AI. Does not replace the base RAG prompt.",
            )}
          </p>
        </TooltipContent>
      </Tooltip>
    </TooltipProvider>
  </div>
  <Textarea
    placeholder={t(
      "query.settings.systemPromptPlaceholder",
      'e.g., "You are a legal advisor. Always cite specific regulations."',
    )}
    value={settings.systemPrompt ?? ""}
    onChange={(e) =>
      onSettingsChange({ systemPrompt: e.target.value || undefined })
    }
    rows={3}
    maxLength={16000}
    className="resize-y text-sm"
  />
  <p className="text-[10px] text-muted-foreground">
    {t(
      "query.settings.systemPromptNote",
      "Extends (does not replace) the built-in RAG instructions.",
    )}
  </p>
</div>;
```

---

## File Change Inventory

### Rust Backend (edgequake-query)

| File                                                             | Change                                                                                                          | Risk   |
| ---------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------- | ------ |
| `edgequake-query/src/engine.rs`                                  | Add `system_prompt: Option<String>` field + builder                                                             | Low    |
| `edgequake-query/src/sota_engine/prompt.rs`                      | Add `system_prompt_extension` param to `build_prompt()`, `generate_answer_with_provider()`, `generate_answer()` | Medium |
| `edgequake-query/src/sota_engine/query_entry/query_basic.rs`     | Pass `request.system_prompt.as_deref()` to `build_prompt()`                                                     | Low    |
| `edgequake-query/src/sota_engine/query_entry/query_workspace.rs` | Same — 2 methods                                                                                                | Low    |
| `edgequake-query/src/sota_engine/query_entry/query_stream.rs`    | Same — 4 methods                                                                                                | Low    |

### Rust Backend (edgequake-api)

| File                                                | Change                                                                | Risk |
| --------------------------------------------------- | --------------------------------------------------------------------- | ---- |
| `edgequake-api/src/handlers/query_types.rs`         | Add `system_prompt` to `QueryRequest` and `StreamQueryRequest`        | Low  |
| `edgequake-api/src/handlers/chat_types.rs`          | Add `system_prompt` to `ChatCompletionRequest`                        | Low  |
| `edgequake-api/src/handlers/chat/streaming.rs`      | Thread `system_prompt` to engine request                              | Low  |
| `edgequake-api/src/handlers/chat/completion.rs`     | Thread `system_prompt` to engine request                              | Low  |
| `edgequake-api/src/handlers/query/query_execute.rs` | Thread `system_prompt` to engine request                              | Low  |
| `edgequake-api/src/handlers/query/query_stream.rs`  | Thread `system_prompt` to engine request                              | Low  |
| `edgequake-api/src/handlers/ollama/chat.rs`         | Map existing `request.system` → `engine_request.with_system_prompt()` | Low  |
| `edgequake-api/src/handlers/ollama/generate.rs`     | Map existing `request.system` → `engine_request.with_system_prompt()` | Low  |

### Frontend (edgequake_webui)

| File                                            | Change                                                   | Risk |
| ----------------------------------------------- | -------------------------------------------------------- | ---- |
| `src/lib/api/chat.ts`                           | Add `system_prompt` to `ChatCompletionRequest` interface | Low  |
| `src/types/index.ts`                            | Add `systemPrompt` to `QuerySettings` type               | Low  |
| `src/stores/use-settings-store.ts`              | Add `systemPrompt` to default settings                   | Low  |
| `src/components/query/query-interface.tsx`      | Pass `system_prompt` in request                          | Low  |
| `src/components/query/query-settings-sheet.tsx` | Add textarea UI for system prompt                        | Low  |

### Total: ~18 files, ~120 lines of changes

---

## Backward Compatibility

| Aspect         | Impact                                                                               |
| -------------- | ------------------------------------------------------------------------------------ |
| API contract   | **Fully backward compatible** — `system_prompt` is optional with `#[serde(default)]` |
| Prompt output  | **Identical when `None`** — no `---Additional Instructions---` section emitted       |
| Streaming      | **Unchanged** — same `stream(&prompt)` pattern                                       |
| Existing tests | **No changes required** — all pass as-is (system_prompt defaults to `None`)          |
| Frontend       | **Non-breaking** — new field in settings sheet, existing UI unchanged                |
| OpenAPI schema | **Additive** — new optional field in request schemas                                 |
| SDKs           | **Non-breaking** — optional field, SDKs can adopt at their own pace                  |

---

## Token Budget Impact

| Component                                       | Tokens (typical)                   |
| ----------------------------------------------- | ---------------------------------- |
| Base system prompt (Role + Goal + Instructions) | ~350 tokens                        |
| Context section                                 | Up to 30,000 tokens (configurable) |
| User query                                      | ~50-200 tokens                     |
| **User system_prompt extension**                | **0-4,000 tokens (max)**           |
| **Total worst case**                            | **~34,550 tokens**                 |

The max 4,000 token budget for the extension is conservative. Most practical system prompts are 50-200 tokens. The validation cap at 16,000 characters (~4,000 tokens) prevents abuse without limiting reasonable use.

---

## Testing Strategy

### Unit Tests (edgequake-query)

```rust
#[test]
fn test_build_prompt_without_system_prompt() {
    // Verify prompt is identical to current behavior when system_prompt is None
    let prompt_without = engine.build_prompt("query", &context, None);
    assert!(!prompt_without.contains("Additional Instructions"));
    assert!(prompt_without.contains("---Context---"));
}

#[test]
fn test_build_prompt_with_system_prompt() {
    let prompt_with = engine.build_prompt(
        "query",
        &context,
        Some("You are a legal advisor."),
    );
    assert!(prompt_with.contains("---Additional Instructions---"));
    assert!(prompt_with.contains("You are a legal advisor."));
    // Verify ordering: Instructions → Additional Instructions → Context → Query
    let instructions_pos = prompt_with.find("---Instructions---").unwrap();
    let additional_pos = prompt_with.find("---Additional Instructions---").unwrap();
    let context_pos = prompt_with.find("---Context---").unwrap();
    let query_pos = prompt_with.find("---User Query---").unwrap();
    assert!(instructions_pos < additional_pos);
    assert!(additional_pos < context_pos);
    assert!(context_pos < query_pos);
}

#[test]
fn test_build_prompt_with_empty_system_prompt() {
    // Empty/whitespace-only should be treated as None
    let prompt = engine.build_prompt("query", &context, Some("   "));
    assert!(!prompt.contains("Additional Instructions"));
}
```

### Integration Tests (edgequake-api)

```rust
#[tokio::test]
async fn test_chat_completion_with_system_prompt() {
    let response = client
        .post("/api/v1/chat/completions")
        .json(&json!({
            "message": "What is GDPR?",
            "system_prompt": "You are a legal advisor. Be concise.",
        }))
        .send()
        .await;
    assert_eq!(response.status(), 200);
}

#[tokio::test]
async fn test_chat_completion_without_system_prompt() {
    // Backward compatibility: no system_prompt field at all
    let response = client
        .post("/api/v1/chat/completions")
        .json(&json!({
            "message": "Hello",
        }))
        .send()
        .await;
    assert_eq!(response.status(), 200);
}
```

### Frontend Tests

```typescript
it("should send system_prompt when configured", async () => {
  const { result } = renderHook(() => useSettingsStore());
  act(() => {
    result.current.setQuerySettings({ systemPrompt: "Be concise." });
  });
  // Verify request body includes system_prompt
});

it("should omit system_prompt when not configured", async () => {
  // Verify request body does NOT include system_prompt key
});
```

---

## OpenAPI Schema Update

The `system_prompt` field will automatically appear in the OpenAPI spec via `utoipa::ToSchema` derive. Sample schema fragment:

```yaml
ChatCompletionRequest:
  type: object
  required:
    - message
  properties:
    message:
      type: string
    system_prompt:
      type: string
      nullable: true
      maxLength: 16000
      description: >
        Optional system prompt extension. Appended to the base RAG instructions
        (does not replace them). Use for persona, output format, domain rules.
    # ... other fields
```

---

## Future Extensions (Out of Scope)

These are explicitly NOT part of this spec but are natural follow-ups:

1. **Workspace-level system prompt** (SPEC-004b): Store a default `system_prompt` in the workspace settings table. Applied to all queries unless overridden per-request. Requires DB migration.

2. **Chat API migration** (SPEC-004c): Migrate from `complete(&prompt)` to `chat(&[ChatMessage])` with proper system/user message roles. Would give stronger instruction-following semantics.

3. **System prompt templating**: Allow `{workspace_name}`, `{date}`, `{user_name}` variable interpolation in system prompts.

4. **System prompt library**: Pre-built system prompt templates selectable from UI (e.g., "Legal Advisor", "Technical Writer", "Data Analyst").

---

## Implementation Order

```
1. [ ] Add system_prompt field to engine QueryRequest (engine.rs)
2. [ ] Modify build_prompt() signature and template (prompt.rs)
3. [ ] Update generate_answer_with_provider() and generate_answer() (prompt.rs)
4. [ ] Update all query entry points to pass system_prompt (query_basic.rs, query_workspace.rs, query_stream.rs)
5. [ ] Add system_prompt to API DTOs (query_types.rs, chat_types.rs)
6. [ ] Thread system_prompt in all 4 API handlers (streaming.rs, completion.rs, query_execute.rs, query_stream.rs)
7. [ ] Map existing `system` field in Ollama handlers (ollama/chat.rs, ollama/generate.rs)
8. [ ] Add validation constant and truncation logic
8. [ ] Write unit tests for build_prompt() with/without system_prompt
9. [ ] Write integration tests for API endpoints
10. [ ] Add system_prompt to frontend TypeScript types (chat.ts, types/index.ts)
11. [ ] Add systemPrompt to settings store (use-settings-store.ts)
12. [ ] Add textarea UI in settings sheet (query-settings-sheet.tsx)
13. [ ] Pass system_prompt in query-interface.tsx request construction
14. [ ] Run cargo test, cargo clippy, bun test
15. [ ] Update OpenAPI docs and SDK examples
```

---

## References

- **Issue**: https://github.com/raphaelmansuy/edgequake/issues/70
- **SPEC-032**: Provider selection at query time (prior art for per-request overrides)
- **`build_prompt()`**: `edgequake/crates/edgequake-query/src/sota_engine/prompt.rs`
- **Engine `QueryRequest`**: `edgequake/crates/edgequake-query/src/engine.rs`
- **API DTOs**: `edgequake/crates/edgequake-api/src/handlers/query_types.rs`, `chat_types.rs`
- **Streaming handler**: `edgequake/crates/edgequake-api/src/handlers/chat/streaming.rs`
- **Frontend types**: `edgequake_webui/src/lib/api/chat.ts`
