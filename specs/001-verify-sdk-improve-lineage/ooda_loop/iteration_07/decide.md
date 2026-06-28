# OODA-07: TypeScript SDK Audit - DECIDE

**Date**: 2026-02-13  
**Decision Focus**: Prioritized action plan for TypeScript SDK

---

## Decision: Add `exportLineage()` to DocumentsResource

**Priority**: High (API parity requirement)  
**Effort**: Low (30 minutes)  
**Risk**: Low (additive change)

---

## Implementation Plan

### Step 1: Add Type Definitions

**File**: `src/types/lineage.ts`

```typescript
// Add export-specific types
export interface LineageExportOptions {
  format?: "json" | "csv";
}
```

### Step 2: Add Method to DocumentsResource

**File**: `src/resources/documents.ts`

```typescript
/**
 * Export document lineage as JSON or CSV file.
 *
 * WHY: Compliance and data portability - users need lineage exports
 * for auditing, archival, and integration with external tools.
 *
 * @param documentId - The document to export lineage for
 * @param options - Export format (default: json)
 * @returns Raw response for file download (JSON string or CSV)
 */
async exportLineage(
  documentId: string,
  options?: LineageExportOptions
): Promise<string> {
  const params = new URLSearchParams();
  if (options?.format) {
    params.set('format', options.format);
  }
  const query = params.toString() ? `?${params.toString()}` : '';
  return this._getRaw(`/api/v1/documents/${documentId}/lineage/export${query}`);
}
```

### Step 3: Add Unit Tests

**File**: `tests/unit/documents.test.ts`

```typescript
describe("exportLineage", () => {
  it("exports lineage as JSON by default", async () => {
    // Test default format
  });

  it("exports lineage as CSV when specified", async () => {
    // Test CSV format
  });

  it("handles document not found error", async () => {
    // Test 404 handling
  });
});
```

### Step 4: Add E2E Test

**File**: `tests/e2e/lineage.test.ts`

```typescript
describe("lineage export", () => {
  it("exports document lineage as JSON", async () => {
    // E2E test against live backend
  });
});
```

### Step 5: Update Index Exports

**File**: `src/index.ts` - Verify `LineageExportOptions` is exported

---

## Acceptance Criteria

| Criterion                         | Status |
| --------------------------------- | ------ |
| Method added to DocumentsResource | ⬜     |
| Unit tests pass                   | ⬜     |
| E2E tests pass                    | ⬜     |
| TypeScript types exported         | ⬜     |
| All 353+ tests pass               | ⬜     |

---

## Rollback Plan

If issues arise:

1. Revert commit `OODA-07`
2. The method is additive - no breaking changes to existing code
3. Users can manually call `getLineage()` and serialize

---

## Commit Message

```
OODA-07: Add exportLineage() to TypeScript SDK

- Add exportLineage(documentId, {format?: 'json' | 'csv'}) to DocumentsResource
- Add unit tests for JSON/CSV export
- Add E2E test against live backend
- 100% lineage endpoint coverage (8/8)

Closes API gap: /api/v1/documents/{id}/lineage/export
```
