# OODA Iteration 221 - DECIDE

## Focus: Action Plan for Provider Verification

**Date**: 2025-01-15

---

## Decision Summary

Based on OBSERVE and ORIENT phases, the following actions are prioritized:

## Priority 1: Provider Lineage Verification

### Objective

Verify that document processing uses workspace-configured provider

### Actions

1. Check backend logs for provider name during document processing
2. Verify pipeline handler reads workspace configuration
3. Confirm LLM provider name is logged/stored with document

### Files to Examine

- `edgequake/crates/edgequake-api/src/handlers/documents.rs`
- `edgequake/crates/edgequake-pipeline/src/lib.rs`
- `edgequake/crates/edgequake-core/src/pipeline.rs`

---

## Priority 2: Interactive Provider Switching Test

### Test Scenario

1. Start with workspace: ollama/gemma3:12b
2. Change to: openai/gpt-4o-mini
3. Upload new document
4. Verify extraction used openai (not ollama)

### Verification Methods

- Backend log inspection
- API response metadata
- Cost field (OpenAI has different pricing)

---

## Priority 3: Rebuild Knowledge Graph Test

### Test Scenario

1. With existing documents in workspace
2. Click "Rebuild Knowledge Graph"
3. Observe progress/status
4. Verify documents are reprocessed with current provider

### Expected Behavior

- Dialog shows progress
- Documents show "Processing" status
- Entity count may change (different LLM = different extraction)
- Provider name visible in logs

---

## Priority 4: Code Verification

### Key Code Paths to Verify

#### Document Upload Handler

```
POST /documents
→ DocumentHandler::upload()
→ Pipeline::process_document()
→ LLMProvider::extract_entities()
```

Must confirm: Pipeline reads workspace LLM config

#### Query Handler

```
POST /query
→ ChatHandler::query()
→ EmbeddingProvider::embed()
→ VectorSearch::search()
→ LLMProvider::generate()
```

Must confirm: Both embedding and LLM use workspace config

---

## Decision: ACT Phase Actions

1. **Read Document Handler Code** - Verify workspace provider is used
2. **Read Pipeline Code** - Verify LLM provider creation
3. **Interactive Test** - Change provider and upload document
4. **Log Analysis** - Check for provider name in logs

---

## Continue to ACT Phase

See [act.md](./act.md) for implementation.
