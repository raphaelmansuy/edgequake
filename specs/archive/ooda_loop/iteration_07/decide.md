# Iteration 07 - DECIDE Phase

## Decision: Create Migration for Processing Sub-states

### Migration Plan

Create `017_add_processing_substates.sql` to:

1. Drop existing constraint
2. Add new constraint with all valid status values
3. Update existing "indexed" status to "completed" for consistency

### Valid Status Values

| Status     | Description                           |
| ---------- | ------------------------------------- |
| pending    | Uploaded, waiting for processing      |
| processing | Generic processing state              |
| chunking   | Text being split into chunks          |
| extracting | LLM extracting entities/relationships |
| embedding  | Generating vector embeddings          |
| indexing   | Storing in graph/vector databases     |
| completed  | Successfully processed                |
| failed     | Processing failed with error          |
| cancelled  | User cancelled processing             |

### SQL Migration

```sql
-- Drop and recreate constraint with new status values
ALTER TABLE documents DROP CONSTRAINT IF EXISTS valid_document_status;
ALTER TABLE documents ADD CONSTRAINT valid_document_status CHECK (
    status IN (
        'pending',
        'processing',
        'chunking',
        'extracting',
        'embedding',
        'indexing',
        'completed',
        'indexed',  -- Legacy support
        'failed',
        'cancelled'
    )
);

-- Optionally migrate 'indexed' to 'completed'
UPDATE documents SET status = 'completed' WHERE status = 'indexed';
```

### Implementation Steps

1. Create migration file
2. Verify SQL syntax
3. Document migration in ACT phase
