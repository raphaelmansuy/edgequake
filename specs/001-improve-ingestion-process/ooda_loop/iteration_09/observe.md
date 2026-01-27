# Observe - Iteration 09: Retry Count Tracking and Error Categorization

## Current State Analysis

### What We Have

From previous iterations:

- ✅ Status sub-states (chunking, extracting, embedding, indexing)
- ✅ Error display with popover (copy + retry)
- ✅ ETA calculation in pipeline dialog
- ✅ Impact preview in rebuild dialogs
- ✅ Database migration for new statuses
- ✅ E2E test files

### What's Missing for Reliability (Objective 7)

1. **Retry Count Tracking**: Users can't see how many times a document failed
2. **Error Categorization**: All errors shown the same, no categorization
3. **Processing History**: No timeline of status changes
4. **Rate Limiting Indicators**: No visibility into API rate limits

### Data Sources to Examine

1. Document API response structure
2. Backend document model
3. Status badge current implementation
4. Document table current implementation

## Observations

### 1. Error Types in RAG Systems

Common error categories:

- **LLM Errors**: Rate limit, API unavailable, context too long
- **Embedding Errors**: Rate limit, dimension mismatch, API error
- **Storage Errors**: Database connection, constraint violation
- **Pipeline Errors**: Parse failure, chunk too large, invalid content

### 2. User Pain Points

- Can't tell if document will succeed on retry
- No indication of transient vs permanent failures
- No visibility into retry history
- Hard to debug systematic issues

### 3. Backend Capabilities

Need to check:

- Does document model have retry_count field?
- Does document model track error timestamps?
- What error information is stored?

## Next Step

Examine backend document model to understand current capabilities.
