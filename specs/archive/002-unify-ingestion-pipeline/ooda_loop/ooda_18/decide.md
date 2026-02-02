# OODA-18: Decide - Action Plan

## Date: 2026-02-01

## Mission Re-Read ✓

---

## Decisions

### Decision 1: Keep react-pdf

**Rationale:**

- 2.8M weekly downloads
- Active maintenance
- Already integrated and working
- Based on Mozilla pdf.js (industry standard)

**Action:** No change needed.

### Decision 2: Focus on Testing

**Rationale:**

- Mission requires "evidence that all tests are passing"
- No E2E tests exist for document viewer
- Testing validates multi-tenancy compliance

**Action:** Create comprehensive E2E tests.

### Decision 3: Minor UX Polish

**Rationale:**

- Mission requires "scrolling / border / margin well handled"
- Current implementation works but can be refined
- Low effort, high visibility improvement

**Action:** Review and refine CSS for scroll container styling.

### Decision 4: Verify OpenAPI Documentation

**Rationale:**

- Mission requires "Swagger / OpenAPI documentation updated"
- utoipa annotations present but need verification

**Action:** Verify endpoints documented in OpenAPI spec.

---

## Priority Order

```
1. Verify existing implementation works (manual test)
2. Create E2E tests for document viewer
3. Review and improve scroll/border UX if needed
4. Verify OpenAPI documentation completeness
5. Document final state
```

---

## Specific Changes Planned

### E2E Tests (High Priority)

Create: `edgequake_webui/e2e/document-viewer.spec.ts`

Test scenarios:

1. Open PDF viewer dialog
2. Navigate pages
3. Zoom in/out
4. Toggle view modes (PDF only, Markdown only, side-by-side)
5. Copy markdown content
6. Download PDF
7. Error handling for missing document

### UX Review (Medium Priority)

Files to review:

- `components/documents/pdf-viewer.tsx` - scroll container
- `components/documents/side-by-side-viewer.tsx` - panel borders
- `components/documents/markdown-viewer.tsx` - content padding

### OpenAPI Verification (Medium Priority)

Endpoints to verify:

- GET `/api/v1/documents/pdf/{id}/download`
- GET `/api/v1/documents/pdf/{id}/content`

---

## Success Criteria

| Criterion          | Measure                       |
| ------------------ | ----------------------------- |
| E2E tests pass     | All scenarios green           |
| PDF viewer renders | Page displays correctly       |
| Side-by-side works | Both panels visible           |
| View modes toggle  | All 3 modes functional        |
| Download works     | PDF downloads correctly       |
| Multi-tenancy      | Forbidden for other workspace |
| OpenAPI complete   | Endpoints documented          |

---

## Risk Mitigation

1. **Test flakiness**: Use Playwright's built-in waits
2. **PDF loading slowness**: Set appropriate timeouts
3. **Server dependency**: Use mocked backend if needed
