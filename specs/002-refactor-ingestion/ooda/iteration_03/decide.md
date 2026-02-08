# OODA Iteration 03 - DECIDE

## Planned Changes

### Change 1: Conditional Status Assignment in Backend

**File**: `edgequake/crates/edgequake-api/src/handlers/documents.rs`
**Lines**: 1187-1210

**Before**:
```rust
let doc_metadata = serde_json::json!({
    "id": document_id,
    "status": "completed",
    "chunk_count": result.stats.chunk_count,
    // ... other fields
});
```

**After**:
```rust
// OODA-03: Determine final status based on chunk extraction results
let final_status = if result.stats.failed_chunks > 0 {
    if result.stats.successful_chunks > 0 {
        "partial_success"
    } else {
        "failed"
    }
} else {
    "completed"
};

let doc_metadata = serde_json::json!({
    "id": document_id,
    "status": final_status,
    "chunk_count": result.stats.chunk_count,
    "successful_chunks": result.stats.successful_chunks,
    "failed_chunks": result.stats.failed_chunks,
    // ... other fields
});
```

### Change 2: Add `partial_success` to EnhancedStatusBadge

**File**: `edgequake_webui/src/components/documents/enhanced-status-badge.tsx`

**Add variant**:
```typescript
partial_success: {
  variant: 'warning',
  label: t('status.partial_success', 'Partial'),
  icon: AlertTriangle,
  description: t('status.partial_desc', 'Some chunks failed'),
},
```

### Change 3: Display Chunk Counts in Badge

**File**: `edgequake_webui/src/components/documents/enhanced-status-badge.tsx`

**Add props**:
```typescript
interface EnhancedStatusBadgeProps {
  status: DocumentStatus;
  failedChunks?: number;
  successfulChunks?: number;
  totalChunks?: number;
}
```

**Display**:
```typescript
{status === 'partial_success' && failedChunks && totalChunks && (
  <span className="text-xs text-muted-foreground ml-1">
    ({totalChunks - failedChunks}/{totalChunks})
  </span>
)}
```

### Change 4: Pass Chunk Counts from DocumentManager

**File**: `edgequake_webui/src/components/documents/document-manager.tsx`

**Update StatusBadge usage** (where document list is rendered):
```tsx
<EnhancedStatusBadge
  status={doc.status}
  failedChunks={doc.failed_chunks}
  successfulChunks={doc.successful_chunks}
  totalChunks={doc.chunk_count}
/>
```

### Change 5: Add Translation Keys

**File**: `edgequake_webui/public/locales/en/common.json`

```json
{
  "status": {
    "partial_success": "Partial Success",
    "partial_desc": "{{successful}}/{{total}} chunks extracted successfully"
  }
}
```

### Verification Plan

1. **Backend Test**: Upload document, mock one chunk to fail, verify status is "partial_success"
2. **Frontend Test**: 
   - Verify EnhancedStatusBadge renders correctly for partial_success
   - Verify chunk counts display
3. **E2E Test**: Full flow with simulated partial failure

### Rollback Plan

If issues found:
1. Backend can return to always using "completed" status
2. Frontend can hide partial_success badge (treat as completed)
3. No breaking changes to API contract
