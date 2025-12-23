# Gap Analysis

## Overview

This document provides a detailed comparison between EdgeQuake's current implementation and the ideal state inspired by LightRAG's mature features.

## Feature Comparison Matrix

| Feature | EdgeQuake | LightRAG | Gap Level |
|---------|-----------|----------|-----------|
| **Document Upload** ||||
| Async processing | ✅ | ✅ | None |
| Progress tracking | ✅ Basic | ✅ Rich | Medium |
| Duplicate detection | ❌ | ✅ | High |
| Batch track_id | ❌ | ✅ | High |
| File path storage | ❌ | ✅ | Medium |
| **Pipeline Status** ||||
| Busy indicator | ✅ | ✅ | None |
| Task counts | ✅ | ✅ | None |
| Batch progress | ❌ | ✅ | Critical |
| History messages | ❌ | ✅ | Critical |
| Latest message | ❌ | ✅ | Critical |
| Cancellation | ✅ | ✅ | None |
| Cancel confirmation | ❌ | ✅ | Low |
| **Document List** ||||
| Pagination | ✅ | ✅ | None |
| Status filtering | ✅ | ✅ | None |
| Status counts (API) | ❌ | ✅ | High |
| Content summary | ❌ | ✅ | Medium |
| Error message | ❌ | ✅ | Medium |
| Track ID grouping | ❌ | ✅ | Medium |
| **Task System** ||||
| Task status | ✅ | ✅ | None |
| Task progress | ✅ | ✅ | Low |
| Task retry | ✅ | ✅ | None |
| Task metadata | ✅ | ✅ | None |

## Critical Gaps

### 1. No Pipeline History Messages

**Current State:**
```typescript
// EdgeQuake - No message history
interface PipelineStatus {
  is_busy: boolean;
  running_tasks: number;
  queued_tasks: number;
  completed_tasks: number;
  failed_tasks: number;
  tasks: TaskResponse[];
}
```

**Desired State:**
```typescript
interface PipelineStatus {
  // ... existing fields
  job_name: string;                    // "Indexing 5 documents"
  job_start: string;                   // ISO timestamp
  docs: number;                        // Total documents
  batchs: number;                      // Total batches
  cur_batch: number;                   // Current batch
  latest_message: string;              // Most recent log
  history_messages: string[];          // Full log history
}
```

**Impact:** Users have no visibility into what's happening during processing. They only see numbers.

### 2. No Batch Progress Tracking

**Current State:**
```
Processing: 3
Queued: 7
Completed: 50
Failed: 2
```

**Desired State:**
```
Job: Indexing 10 documents
Started: 10:30:45
Progress: Batch 2/4 (5 of 10 documents)

Latest: Extracting entities from document_003...
```

**Impact:** For large uploads (e.g., 100 documents), users have no idea how long it will take.

### 3. No Track ID for Upload Batches

**Current State:**
```javascript
// Upload returns individual task_id per document
{ document_id: "doc_001", task_id: "task_abc" }
{ document_id: "doc_002", task_id: "task_def" }
{ document_id: "doc_003", task_id: "task_ghi" }
// No correlation between them
```

**Desired State:**
```javascript
// All documents from same upload share track_id
{ document_id: "doc_001", task_id: "task_abc", track_id: "upload_20240115_103045" }
{ document_id: "doc_002", task_id: "task_def", track_id: "upload_20240115_103045" }
{ document_id: "doc_003", task_id: "task_ghi", track_id: "upload_20240115_103045" }

// Can query by track_id to see batch status
GET /documents/track/upload_20240115_103045
{
  track_id: "upload_20240115_103045",
  documents: [...],
  status_summary: { processed: 2, processing: 1 }
}
```

**Impact:** Cannot show "Uploaded 5 documents - 3 complete, 2 processing"

### 4. No Status Counts in List API

**Current State:**
```typescript
// API returns only current page
GET /documents?page=1&page_size=20
{
  documents: [...],
  total: 150,
  page: 1,
  page_size: 20
}

// Client must load ALL documents to count statuses
const statusCounts = {
  pending: docs.filter(d => d.status === 'pending').length,  // Requires all docs!
  processing: docs.filter(d => d.status === 'processing').length,
  // ...
};
```

**Desired State:**
```typescript
// API returns counts for ALL documents
GET /documents?page=1&page_size=20
{
  documents: [...],
  pagination: { total: 150, page: 1, page_size: 20 },
  status_counts: {
    pending: 10,
    processing: 5,
    completed: 130,
    failed: 5
  }
}
```

**Impact:** Status filter badges show wrong counts until all pages are loaded.

## High-Priority Gaps

### 5. No Content Summary

**Current State:**
```typescript
interface DocumentSummary {
  id: string;
  title: string;
  chunk_count: number;
  // No preview of content
}
```

**Desired State:**
```typescript
interface DocumentSummary {
  id: string;
  title: string;
  chunk_count: number;
  content_summary: string;  // First 200 chars
  content_length: number;   // Total characters
}
```

**Impact:** Users cannot preview document content without clicking into details.

### 6. No Error Details

**Current State:**
```typescript
interface TaskResponse {
  status: "failed";
  error_message: string;  // Generic "Processing failed"
}
```

**Desired State:**
```typescript
interface TaskResponse {
  status: "failed";
  error_message: string;           // "Entity extraction failed"
  error_details: {
    step: string;                  // "entity_extraction"
    reason: string;                // "Rate limit exceeded"
    suggestion: string;            // "Retry in 30 seconds"
    stack_trace?: string;          // For debugging
  };
}
```

**Impact:** Users don't know why processing failed or how to fix it.

### 7. No Duplicate Detection

**Current State:**
- Same document can be uploaded multiple times
- No warning or deduplication
- Wastes processing resources

**Desired State:**
```json
{
  "status": "duplicated",
  "message": "Document 'report.txt' already exists (doc_abc)",
  "existing_id": "doc_abc"
}
```

**Impact:** Users may accidentally upload duplicates, wasting time and resources.

## Medium-Priority Gaps

### 8. No File Path Storage

**Current State:**
- Only stores content, not original path
- Cannot show "Uploaded from: ~/Documents/report.txt"

**Desired State:**
```typescript
interface Document {
  file_path: string;  // Original file location
  file_name: string;  // Original filename
  // ...
}
```

### 9. No Preprocessed State

**Current State:**
```
pending → processing → completed
```

**Desired State:**
```
pending → processing → preprocessed → completed
                 ↓           ↓
               failed      failed
```

**Impact:** Cannot distinguish "chunked but not indexed" from "indexing".

### 10. No Cancellation Confirmation

**Current State:**
- Single click cancels immediately
- No undo or confirmation

**Desired State:**
- Two-step confirmation
- Shows what will be lost
- Option to keep completed work

## Gap Priority Matrix

```
                     Business Impact
                     Low    Medium    High
                    ┌──────┬──────┬──────┐
            Low     │  10  │   8  │      │
 Implementation     ├──────┼──────┼──────┤
   Effort           │   9  │ 5, 6 │ 4, 7 │
           Medium   ├──────┼──────┼──────┤
                    │      │      │ 1,2,3│
            High    └──────┴──────┴──────┘

Priority Order:
1. Pipeline History Messages (Critical)
2. Batch Progress Tracking (Critical)
3. Track ID for Batches (Critical → High)
4. Status Counts in API (High)
5. Content Summary (High → Medium)
6. Error Details (High → Medium)
7. Duplicate Detection (High → Medium)
8. File Path Storage (Medium)
9. Preprocessed State (Medium → Low)
10. Cancel Confirmation (Low)
```

## Implementation Complexity

### Quick Wins (1-2 hours each)
- Cancel confirmation dialog
- Status counts in list API response

### Medium Effort (4-8 hours each)
- Track ID generation and storage
- Content summary extraction
- Error details structure

### Significant Effort (1-2 days each)
- Pipeline history messages
- Batch progress tracking
- Message streaming (WebSocket/SSE)

## Summary

### Must Have (P0)
1. **Pipeline history messages** - Users need to see what's happening
2. **Batch progress** - "3/10 documents processed"
3. **Track ID** - Group related documents

### Should Have (P1)
4. **Status counts in API** - Accurate filter badges
5. **Content summary** - Document preview
6. **Error details** - Actionable error messages

### Nice to Have (P2)
7. **Duplicate detection** - Prevent wasted processing
8. **File path storage** - Know where documents came from
9. **Preprocessed state** - More granular status
10. **Cancel confirmation** - Prevent accidents

---

**Next:** [Proposed Improvements](./05-proposed-improvements.md)
