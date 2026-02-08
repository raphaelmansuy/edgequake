# OODA-15: Decide - Model Configuration Decisions

**Date**: 2026-02-08
**Mission**: Reliable Document Ingestion Pipeline
**Focus**: Finalize model selection and test plan

---

## Decision Summary

| #   | Decision                                       | Priority | Effort | Impact |
| --- | ---------------------------------------------- | -------- | ------ | ------ |
| D1  | Test gpt-4.1-nano as alternative to gpt-5-nano | P0       | Low    | High   |
| D2  | Update workspace config for testing            | P0       | Low    | High   |
| D3  | E2E test upload via Playwright UI              | P0       | Medium | High   |
| D4  | Update pricing reference in mission doc        | P1       | Low    | Medium |

---

## D1: Test gpt-4.1-nano vs gpt-5-nano

**Rationale**:

- gpt-5-nano uses reasoning tokens that may truncate JSON
- gpt-4.1-nano has no reasoning tokens, more reliable JSON output
- Cost difference: $0.05/M input vs $0.10/M input (2x difference, ~$0.005/doc)

**Test Plan**:

1. Update workspace to use gpt-4.1-nano
2. Upload test PDF via API
3. Verify entity extraction JSON is complete
4. Compare entity count with gpt-5-nano results

**Success Criteria**:

- JSON response is valid and complete
- Entity extraction produces ≥15 entities for test document
- No truncation errors in logs

---

## D2: Update Workspace Configuration

**Current**:

```json
{
  "llm_provider": "openai",
  "llm_model": "gpt-5-nano",
  "embedding_provider": "openai",
  "embedding_model": "text-embedding-3-small",
  "embedding_dimension": 1536
}
```

**Target**:

```json
{
  "llm_provider": "openai",
  "llm_model": "gpt-4.1-nano",
  "embedding_provider": "openai",
  "embedding_model": "text-embedding-3-small",
  "embedding_dimension": 1536
}
```

**API Call**:

```bash
curl -X PUT "http://localhost:8080/api/v1/workspaces/00000000-0000-0000-0000-000000000003" \
  -H "Content-Type: application/json" \
  -H "X-Tenant-ID: 00000000-0000-0000-0000-000000000002" \
  -H "X-Workspace-ID: 00000000-0000-0000-0000-000000000003" \
  -d '{"llm_model": "gpt-4.1-nano"}'
```

---

## D3: E2E Test with Playwright

**Test Flow**:

```
┌─────────────────────────────────────────────────────────────────┐
│                     E2E TEST FLOW                                │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│  1. Navigate to http://localhost:3000/documents                 │
│                                                                  │
│  2. Click Upload button                                          │
│                                                                  │
│  3. Upload test PDF from zz-explore/EMILE_FREY/                 │
│                                                                  │
│  4. Wait for processing to complete (status: Completed)          │
│                                                                  │
│  5. Click on document to view details                            │
│                                                                  │
│  6. Verify entities are extracted                                │
│                                                                  │
│  7. Run query against uploaded document                          │
│                                                                  │
└─────────────────────────────────────────────────────────────────┘
```

**Test Document**:

- `zz-explore/EMILE_FREY/DO-DONT.pdf` (multi-page, good for testing)

---

## D4: Update Pricing Reference

Add to mission file `specs/001-reliable-ingestion-mission.md`:

```markdown
### OpenAI Model Pricing Reference (2026-02 Latest)

| Model                  | Input/1M | Output/1M | Use Case            | JSON Reliability          |
| ---------------------- | -------- | --------- | ------------------- | ------------------------- |
| gpt-5-nano             | $0.05    | $0.40     | Cheap extraction    | Medium (reasoning tokens) |
| gpt-4.1-nano           | $0.10    | $0.40     | Reliable extraction | High (no reasoning)       |
| gpt-5-mini             | $0.25    | $2.00     | Complex tasks       | High                      |
| text-embedding-3-small | $0.02    | -         | Embeddings (1536d)  | N/A                       |

**Recommendation**: Use `gpt-4.1-nano` for production (reliable JSON) or `gpt-5-nano` for cost savings (with validation).
```

---

## Execution Order

1. **D2** → Update workspace config to gpt-4.1-nano
2. **D3** → Run E2E test with Playwright
3. **D1** → Verify entity extraction results
4. **D4** → Update documentation if tests pass

---

## Risk Mitigation

| Risk                                 | Mitigation                             |
| ------------------------------------ | -------------------------------------- |
| gpt-4.1-nano produces fewer entities | Compare counts, fallback to gpt-5-nano |
| API rate limiting                    | Add delays between requests            |
| Frontend not responding              | Restart with `make dev`                |

---

## Commit Plan

```
OODA-15: Update model config for price/performance

- Test gpt-4.1-nano for reliable JSON extraction
- Update workspace configuration
- E2E test via Playwright
- Document pricing reference

Files changed:
- specs/001-reliable-ingestion-mission.md (pricing table)
- Workspace API config (runtime change)
```
