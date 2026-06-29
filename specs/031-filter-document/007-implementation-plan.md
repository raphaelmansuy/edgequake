# SPEC-031 / 007 — Implementation Plan

> **Lens**: Engineering / PM  
> **Principle**: Vertical slices · Each phase independently deployable · Backend first

---

## 1. Phases Overview

```
Phase 1 (Backend Foundation)
  - Extend DocumentFilter DTO with document_ids
  - Update document_filter_resolver.rs
  - Add GET /api/v1/documents/search endpoint
  - Add unit tests

Phase 2 (Frontend Core)
  - Extend TypeScript types
  - Add searchDocuments() API client
  - Implement useDocumentSearch hook
  - Implement DocumentPickerPopover component
  - Implement QueryScopeBar component

Phase 3 (Integration)
  - Connect QueryScopeBar into QueryInterface
  - Extend QuerySettingsSheet with scope section
  - Extend useQuerySettings / useQueryInterface
  - Wire document_ids into query API calls

Phase 4 (MCP Surface)
  - Update query tool schema
  - Add search_documents MCP tool

Phase 5 (Polish & Tests)
  - Accessibility pass
  - E2E tests
  - Mobile layout verification
  - Performance validation
```

---

## 2. Detailed Task Breakdown

### Phase 1 — Backend Foundation

#### Task B1: Extend `DocumentFilter` DTO

**File**: `edgequake/crates/edgequake-api/src/handlers/query_types.rs`

**Change**:
- Add `document_ids: Option<Vec<String>>` field with `#[serde(default)]`
- Add `is_empty()` method

**Also update**: `chat_types.rs` if `DocumentFilter` is re-exported or re-defined there (check if it's the same type or a duplicate).

**Acceptance criteria**:
- Existing JSON without `document_ids` deserializes without error
- `document_ids: []` → `is_empty()` returns true
- `document_ids: ["a"]` → `is_empty()` returns false

---

#### Task B2: Update `document_filter_resolver.rs`

**File**: `edgequake/crates/edgequake-api/src/handlers/query/document_filter_resolver.rs`

**Change**:
- Extract `passes_date_filter()` helper (SRP)
- Extract `parse_patterns()` helper (SRP)
- Add fast-path: if `has_explicit_ids && !has_pattern && !has_date_filter` → return IDs immediately
- Refactor the main loop to union explicit IDs + pattern matches
- Keep existing SPEC-005 behavior unchanged

**New tests**:
- `test_empty_document_ids_is_noop`
- `test_explicit_ids_only_no_kv_scan` (mock KV that panics on read — should never be called)
- `test_explicit_ids_with_date_filter`
- `test_explicit_ids_union_with_pattern`
- `test_nonexistent_ids_return_empty`

**Acceptance criteria**:
- All existing 10+ tests still pass
- New tests pass
- `cargo clippy` clean

---

#### Task B3: Add `GET /api/v1/documents/search` Route

**New file**: `edgequake/crates/edgequake-api/src/handlers/documents/query/search.rs`

**New types** (in `documents_types/listing.rs` or new file):
- `DocumentSearchRequest` (query struct)
- `DocumentSearchItem` (response item)
- `DocumentSearchResponse`

**Handler**: `search_documents`
- Require full tenant context (security)
- Load metadata via `load_scoped_document_metadata`
- Filter by `status` if set
- Filter by `q` substring if non-empty
- Sort by `created_at` descending
- Cap at `page_size.min(50)`
- Return `DocumentSearchResponse`

**Route registration** in `routes.rs`:
```
.route("/documents/search", get(handlers::search_documents))
```
Must be registered **before** `/documents/{document_id}` to avoid routing conflicts.

**Update `handlers/mod.rs`** to re-export `search_documents`.

**Tests**:
- `test_search_empty_query_returns_recent`
- `test_search_with_query_filters_by_title`
- `test_search_status_filter_completed_only`
- `test_search_status_all_returns_all`
- `test_search_page_size_cap`
- `test_search_requires_tenant_context`

**Acceptance criteria**:
- Route responds 200 with correct JSON shape
- Empty `q` → returns most recent 20 `completed` docs
- Status filter works
- No cross-workspace data returned

---

#### Task B4: Update OpenAPI Annotations

- Add `document_ids` to `DocumentFilter` `ToSchema` derive
- Add `#[utoipa::path]` annotation to `search_documents`
- Run `cargo build` to verify OpenAPI generation

---

### Phase 2 — Frontend Core

#### Task F1: Extend TypeScript Types

**File**: `edgequake_webui/src/types/query.ts`
- Add `document_ids?: string[]` to `DocumentFilter`
- Export `isEmptyDocumentFilter()` utility

**File**: `edgequake_webui/src/types/index.ts` (or new `types/documents.ts`)
- Add `DocumentSearchItem`
- Add `DocumentSearchResponse`

---

#### Task F2: Add `searchDocuments` API Client Function

**File**: `edgequake_webui/src/lib/api/edgequake/documents.ts`
- Add `searchDocuments(params)` function
- Uses `buildQueryString` + `api.get`

---

#### Task F3: Implement `useDebounce` Hook

**File**: `edgequake_webui/src/hooks/use-debounce.ts`
- Check if already exists; create if not
- Generic `useDebounce<T>(value: T, delay: number): T`

---

#### Task F4: Implement `useDocumentSearch` Hook

**File**: `edgequake_webui/src/hooks/use-document-search.ts`
- Uses `useDebounce` + `useQuery`
- `queryKey: ['documents', 'search', debouncedQuery]`
- `staleTime: 30_000`
- Returns `DocumentSearchItem[]`

---

#### Task F5: Implement `useDocumentTitle` Hook

**File**: `edgequake_webui/src/hooks/use-document-title.ts`
- Reads from React Query cache (no fetch)
- Checks both search results cache and documents list cache

---

#### Task F6: Implement `DocumentPickerPopover` Component

**File**: `edgequake_webui/src/components/query/document-picker-popover.tsx`
- Uses `useDocumentSearch`
- Checkbox-based selection
- Search input with clear button
- Selected items sorted to top
- Footer: count + clear all
- ARIA roles (listbox/option)

**Export from** `edgequake_webui/src/components/query/index.ts`

---

#### Task F7: Implement `QueryScopeBar` Component

**File**: `edgequake_webui/src/components/query/query-scope-bar.tsx`
- `null` when `selectedIds.length === 0`
- Pills with `useDocumentTitle` for labels
- "+N more" for overflow
- Uses `DocumentPickerPopover` for adding
- Horizontally scrollable on mobile

**Export from** `edgequake_webui/src/components/query/index.ts`

---

### Phase 3 — Integration

#### Task I1: Extend `useQuerySettings`

**File**: `edgequake_webui/src/hooks/use-query-settings.ts` (or wherever settings state lives)
- Add `scopedDocumentIds?: string[]` to settings shape
- Persist in `localStorage` (same key or bump version)
- Add `buildDocumentFilter()` helper that merges `documentFilter` + `scopedDocumentIds`

---

#### Task I2: Connect `QueryScopeBar` into `QueryInterface`

**File**: `edgequake_webui/src/components/query/query-interface.tsx`
- Import `QueryScopeBar`
- Render between messages area and text input: `<QueryScopeBar selectedIds={...} onSelectionChange={...} disabled={isLoading} />`
- Source `scopedDocumentIds` from `querySettings`

---

#### Task I3: Wire `document_ids` into Query Submit

**File**: `edgequake_webui/src/hooks/use-query-interface.ts` (or `use-chat.ts`)
- Before submitting, call `buildDocumentFilter(querySettings)` to get merged `DocumentFilter`
- Pass resulting `document_filter` (with `document_ids`) to the API call

---

#### Task I4: Extend `QuerySettingsSheet`

**File**: `edgequake_webui/src/components/query/query-settings-sheet.tsx`
- Add `scopedDocumentIds?: string[]` + `onScopedDocumentIdsChange` to props
- Add "Document Scope" section using `DocumentPickerPopover`
- Update `QueryInterface` to pass the new props

---

### Phase 4 — MCP Surface

#### Task M1: Update MCP Query Tool Schema

**File**: `mcp/src/tools.rs` (or equivalent)
- Add `document_ids` array parameter to `query` tool schema

---

#### Task M2: Add `search_documents` MCP Tool

**File**: `mcp/src/tools.rs`
- Register new `search_documents` tool
- Routes to `GET /api/v1/documents/search` with tenant context forwarding

---

### Phase 5 — Polish & Tests

#### Task P1: Accessibility Pass

- Screen reader test with VoiceOver (macOS)
- Verify all ARIA labels from spec 002 are in place
- Verify keyboard navigation: Tab through pills, Enter to open picker, Space to toggle checkbox, Escape to close popover

#### Task P2: E2E Tests

**File**: `edgequake_webui/e2e/query-scope.spec.ts`

```typescript
test('adds document to scope and sees pill', async ({ page }) => {
  await page.goto('http://localhost:3000/query');
  // Open settings sheet
  await page.click('[aria-label="Settings"]');
  // Click "Add documents to scope"
  await page.click('[aria-label="Add documents to scope"]');
  // Search and select a document
  await page.fill('[aria-label="Search documents by title"]', 'report');
  await page.click('role=option[name*="report"]');
  // Close popover
  await page.keyboard.press('Escape');
  // Close sheet
  await page.keyboard.press('Escape');
  // Verify scope bar visible
  await expect(page.locator('[aria-label="Active query scope"]')).toBeVisible();
  // Verify pill
  await expect(page.locator('[aria-label*="from scope"]')).toBeVisible();
});

test('clearing scope hides scope bar', async ({ page }) => {
  // ... select doc, then click [× All], verify scope bar hidden
});

test('scope persists after page reload', async ({ page }) => {
  // ... select doc, reload, verify scope bar still visible
});
```

#### Task P3: Mobile Layout Verification

- Test at 375px (iPhone SE) viewport
- Verify scope bar scrolls horizontally
- Verify picker popover opens as bottom sheet

#### Task P4: Performance Validation

- Search endpoint: load 1,000 docs into KV, measure p99 latency
- Target: < 100ms response time for 1,000 docs
- Measure from UI: debounce 300ms + request + render should feel < 500ms total

---

## 3. Risk Register

| Risk                                                    | Probability | Impact | Mitigation                                              |
| ------------------------------------------------------- | ----------- | ------ | ------------------------------------------------------- |
| KV scan too slow at 10,000 docs                         | Medium      | High   | Add page_size=20 cap; document future optimization path |
| `document_ids` field breaks existing SPEC-005 clients   | Low         | Medium | `#[serde(default)]` ensures backward compat             |
| Route conflict `/documents/search` vs `/documents/{id}` | Low         | High   | Register search route BEFORE `/{id}` in routes.rs       |
| Pills clutter UI on mobile                              | Medium      | Medium | Horizontal scroll + "+N" chip design                    |
| React Query cache miss for titles                       | High        | Low    | Show truncated ID as fallback — acceptable              |

---

## 4. Acceptance Criteria Summary

### Backend
- [ ] `GET /api/v1/documents/search?q=report` returns `{ items: [...], total: N, has_more: bool }`
- [ ] `POST /api/v1/query` with `document_filter: { document_ids: ["a","b"] }` only returns context from docs "a" and "b"
- [ ] `POST /api/v1/query` with `document_filter: {}` (empty) returns full workspace context
- [ ] `document_ids: []` treated as no-op
- [ ] All existing SPEC-005 filter tests still pass

### Frontend
- [ ] Scope bar hidden when `scopedDocumentIds` is empty
- [ ] Scope bar shows pills when docs selected
- [ ] Pills survive page refresh
- [ ] Clicking `×` on pill removes that document
- [ ] Clicking `× All` removes all pills and hides scope bar
- [ ] Picker search returns results within 500ms perceived time
- [ ] Picker shows "No documents yet" when workspace is empty

### MCP
- [ ] `search_documents` tool returns correct items
- [ ] `query` tool with `document_ids` respects scope

---

## 5. Non-Goals (Explicit Exclusions)

- Saved scope presets / named scopes → future spec
- Cross-workspace queries → out of scope
- Real-time document status updates in the picker → not needed (completed filter sufficient)
- Full-text search on document content → separate feature
- Document scope in conversation history display → future enhancement
