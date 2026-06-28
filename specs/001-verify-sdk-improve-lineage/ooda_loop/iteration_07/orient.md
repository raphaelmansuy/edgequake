# OODA-07: TypeScript SDK Audit - ORIENT

**Date**: 2026-02-13  
**Analysis Focus**: Gap assessment and solution design

---

## First Principles Analysis

### Current State

The TypeScript SDK is **95%+ complete** with:

- 353 tests passing (100% pass rate)
- 7/8 lineage endpoints implemented
- Strict TypeScript types matching Rust backend
- Clean code quality (ESLint, Vitest)

### Gap: Missing `exportLineage()` Method

**Why does the backend have this endpoint?**

- Data portability: Users need to export lineage for compliance, auditing
- Integration: CSV format for Excel/Google Sheets, JSON for programmatic use
- Archival: Store lineage snapshots for versioning

**Why is it missing in TypeScript SDK?**

- Added to backend after SDK was generated
- Not in initial API coverage sweep
- Lower priority than core CRUD operations

### Impact Assessment

| Impact Area          | Score  | Notes                                                |
| -------------------- | ------ | ---------------------------------------------------- |
| API Parity           | -5%    | 1 of 8 lineage endpoints missing                     |
| User Experience      | Low    | Workaround: call `getLineage()` + serialize manually |
| Compliance Use Cases | Medium | Some users need CSV export for audits                |
| Mission Objective    | Medium | 95% coverage requires this endpoint                  |

---

## Solution Options

### Option A: Add `exportLineage()` to DocumentsResource

```typescript
// Add to src/resources/documents.ts
async exportLineage(
  documentId: string,
  options?: { format?: 'json' | 'csv' }
): Promise<string | Blob>
```

**Pros:**

- Natural grouping with other document methods
- Consistent with `getLineage()` already in this class
- Simple implementation

**Cons:**

- Larger class (already 200+ lines)

### Option B: Add `export()` to LineageResource

```typescript
// Add to src/resources/lineage.ts
async export(
  documentId: string,
  options?: { format?: 'json' | 'csv' }
): Promise<string | Blob>
```

**Pros:**

- Groups all lineage operations together
- Smaller, focused classes

**Cons:**

- Slightly different URL pattern (`/api/v1/documents/{id}/lineage/export`)
- Less discoverable for users expecting document methods

### Option C: Add Dedicated LineageExportResource

```typescript
// src/resources/lineage-export.ts
export class LineageExportResource extends Resource {
  async document(documentId: string, format?: "json" | "csv"): Promise<Blob>;
  async entity(entityName: string, format?: "json" | "csv"): Promise<Blob>;
}
```

**Pros:**

- Future-proof for entity/chunk exports
- Single Responsibility Principle

**Cons:**

- Over-engineering for single endpoint
- More files to maintain

---

## Recommendation

**Option A: Add to DocumentsResource** is the best choice because:

1. **User Mental Model**: Export is a document operation, users expect `documents.exportLineage(id)`
2. **URL Pattern**: Backend path is `/api/v1/documents/{id}/lineage/export`, aligns with document resource
3. **Existing Pattern**: TypeScript SDK already has `documents.getLineage()`, export is next step
4. **Minimal Change**: Single method addition to existing file

---

## Risk Assessment

| Risk            | Probability | Mitigation                                   |
| --------------- | ----------- | -------------------------------------------- |
| Breaking change | Low         | Additive API, no changes to existing methods |
| Type mismatch   | Low         | Use backend DTO types                        |
| Test regression | Low         | Isolated unit test, E2E against live backend |
| Blob handling   | Medium      | Test in browser and Node.js environments     |

---

## Dependencies

1. Backend endpoint is stable and tested (confirmed in `lineage.rs:937`)
2. SDK build system supports Blob responses
3. Unit and E2E test infrastructure exists
