# Iteration 07 - ORIENT Phase

## Gap Analysis

### Critical Issue Found
The database constraint `valid_document_status` only allows:
- `pending`
- `processing`
- `indexed`
- `failed`

But processor.rs now sets:
- `chunking`
- `extracting`
- `embedding`
- `indexing`
- `completed`

**This will cause database errors when status updates are attempted!**

### Resolution Options

1. **Update constraint** - Add new status values to constraint
2. **Use processing + sub_status** - Keep main status, add sub_status column
3. **Remove constraint** - Allow any string (not recommended)

### Recommended Approach

**Option 1: Update constraint** with all valid status values:
- `pending` - Document uploaded, waiting for processing
- `processing` - Generic processing (fallback)
- `chunking` - Text being split into chunks
- `extracting` - LLM extracting entities
- `embedding` - Generating vector embeddings
- `indexing` - Storing in graph/vector DB
- `completed` - Successfully processed (replaces "indexed")
- `failed` - Processing failed
- `cancelled` - User cancelled processing

### Migration Required

Create new migration: `017_add_processing_substates.sql`

## Dependencies

- Need to create migration file
- Need to run migration on dev database
- Frontend already supports all states (status-badge.tsx)
