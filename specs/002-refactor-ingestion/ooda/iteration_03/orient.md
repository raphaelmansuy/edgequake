# OODA Iteration 03 - ORIENT

## Analysis of Findings

### Problem Statement

Documents with partial chunk extraction failures are marked "completed" when they should show "partial_success" status. This hides data loss from users.

### Solution Approaches

#### Option A: Add `partial_success` Status (Backend + Frontend)

**Description**:

1. Backend: Set `status: "partial_success"` when `failed_chunks > 0 && successful_chunks > 0`
2. Backend: Store `failed_chunks` and `successful_chunks` in document metadata
3. Frontend: Add `partial_success` variant to EnhancedStatusBadge

**Pros**:

- Clear, explicit status
- Users can identify documents needing attention
- Maintains backward compatibility

**Cons**:

- Requires both backend and frontend changes

**Effort**: 4 hours

#### Option B: Use Sub-status with Warning

**Description**:

- Keep `status: "completed"` but add `warning: "partial_success"`
- Add `failed_chunks` count to metadata
- Frontend shows warning badge next to status

**Pros**:

- Backward compatible (status still "completed")
- Existing filters still work

**Cons**:

- More confusing for API users
- Two fields to check instead of one

**Effort**: 3 hours

#### Option C: Status + Details Object

**Description**:

- Keep `status: "completed"` but add `status_details: { partial: true, failed_chunks: 2 }`
- Frontend parses details for enhanced display

**Pros**:

- Backward compatible
- Rich detail without status proliferation

**Cons**:

- Complex to query/filter
- Frontend parsing overhead

**Effort**: 5 hours

### Decision Matrix

| Criteria              | Weight | Option A | Option B | Option C |
| --------------------- | ------ | -------- | -------- | -------- |
| Clarity               | 40%    | 10/10    | 6/10     | 7/10     |
| Ease of filtering     | 25%    | 10/10    | 5/10     | 4/10     |
| Implementation effort | 20%    | 7/10     | 8/10     | 6/10     |
| Backward compat       | 15%    | 8/10     | 10/10    | 10/10    |
| **Weighted Score**    | 100%   | **8.95** | **6.7**  | **6.65** |

### Recommendation

**Option A: Add `partial_success` Status**

**Rationale**:

1. Mission spec explicitly says "Add `partial_success` status"
2. Clear, queryable status for API users
3. Enables filtering documents by extraction quality
4. Already have infrastructure (stats tracked, events broadcast)

### Implementation Strategy

1. **Backend Change** (documents.rs:1196):
   - Check `result.stats.failed_chunks > 0`
   - If true AND `successful_chunks > 0`: set `status: "partial_success"`
   - If true AND `successful_chunks == 0`: set `status: "failed"` (already handled)
   - Add `failed_chunks` and `successful_chunks` to metadata

2. **Frontend Change** (EnhancedStatusBadge):
   - Add `partial_success` variant with amber/warning styling
   - Show "Partial (N/M)" label with chunk counts

3. **Translation Keys**:
   - Add i18n keys for "partial_success" status

### Files to Modify

| File                   | Change                          |
| ---------------------- | ------------------------------- |
| `documents.rs:1196`    | Conditional status assignment   |
| `EnhancedStatusBadge`  | Add `partial_success` variant   |
| `document-manager.tsx` | Display chunk failure count     |
| Translation files      | Add status.partial_success keys |
