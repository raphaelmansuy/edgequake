# `GH-319` — Failed document filter shows zero results

> **Priority**: P0  
> **Audit status**: FIXED  
> **Sprint**: 0  
> **Laws**: LAW-10, LAW-3, LAW-8  
> **GitHub**: https://github.com/raphaelmansuy/edgequake/issues/319  
> **Verified against**: v0.21.0 / `19477c2d`

---

## 1. WHY

Users see `Failed (N)` in the filter chip but selecting Failed returns an empty table. They cannot find or retry failed uploads. Filed at ~199 docs on v0.19.0; still reproducible on HEAD whenever failures fall outside the newest page.

---

## 2. Audit (code is law)

| Field | Value |
|-------|-------|
| Primary locus (BE) | `ListDocumentsRequest` — **no `status` field** ([`listing.rs`](../../../edgequake/crates/edgequake-api/src/handlers/documents_types/listing.rs)) |
| Counts | Computed on full list **before** pagination ([`list.rs`](../../../edgequake/crates/edgequake-api/src/handlers/documents/query/list.rs)) |
| Page clamp | `MAX_PAGE_SIZE = 100` ([`budget.rs`](../../../edgequake/crates/edgequake-core/src/resource/budget.rs)) |
| FE | Sends `page_size: 500` + `status=failed`; then **client-filters** returned items ([`document-manager.tsx`](../../../edgequake_webui/src/components/documents/document-manager.tsx), [`use-document-filtering.ts`](../../../edgequake_webui/src/hooks/use-document-filtering.ts)) |
| Enum mismatch? | **No** — both sides use lowercase `"failed"` |
| Verdict | **CONFIRMED** |

```
Backend: all docs → status_counts.failed = N ✓ → clamp page_size 500→100 → newest 100
Frontend: show Failed (N) from status_counts ✓ → filter items where status==="failed" → often []
```

---

## 3. Root cause (first principles)

**LAW-10**: Filter universe must equal the universe that produced the count. Counts are global; the Failed filter runs on a truncated newest-100 window. FE believes `page_size=500` fetches all (virtual scroll intent) but API silently clamps. `?status=` is ignored because the DTO has no field.

---

## 4. Multi-lens analysis

### Product Owner

- Acceptance: If Failed chip shows N>0, the table lists those N docs (paginated if needed), newest or oldest per sort SSOT.
- Retry Failed must operate on the same set the user sees.

### Full Stack

| Layer | Bug |
|-------|-----|
| OpenAPI / DTO | No `status` query param |
| Handler | Counts then paginate; no status filter stage |
| FE hooks | Passes `status` but also re-filters client-side |
| Virtual scroll | `VIRTUAL_PAGE_SIZE=500` conflicts with `MAX_PAGE_SIZE` |

### AI Engineer

- N/A (pure list/filter SSOT).

### O(n) / Systems

- Client filter is O(page) not O(workspace) — appears “correct” in small demos (<100 docs), fails in production.
- Server-side status filter is O(workspace) once then paginate — acceptable; avoid loading all rows into FE.

### Postgres Expert

- Status lives in KV/SQL document store, not AGE. No GIN concern.
- Prefer SQL/KV filter + LIMIT/OFFSET (or keyset) over load-all-in-memory if workspace grows past tens of thousands (future); for #319, in-memory filter-before-paginate matching existing list architecture is enough if status is applied to the full vec before `paginate_vec`.

---

## 5. ASCII causal diagram

```
  status_counts over ALL docs (failed=N)
            |
            v
  page_size clamped to 100 (newest)
            |
            v
  FE filters page only  -->  0 rows while chip shows N
```

---

## 6. Solution (SOLID + DRY)

| Principle | Application |
|-----------|-------------|
| S | List handler owns filter+paginate; FE does not re-derive status membership when server filtered |
| O | `status` enum query param alongside date/pattern |
| L | SDK + OpenAPI + handler share same request shape |
| I | Narrow: optional `status: Option<DocumentStatusFilter>` |
| D | `status_counts` SSOT remains server; FE displays server counts |
| DRY | One `apply_document_filters` used by list (date, pattern, status) before counts/paginate order is documented |

### Implementation steps

1. Add `status: Option<String>` (or typed enum) to `ListDocumentsRequest` + utoipa + codegen OpenAPI refresh.
2. In `list.rs`: after date/pattern filters, if status set → filter docs; compute `status_counts` on the **pre-status** set (or document product choice: counts always workspace-global — **locked: counts remain global**, list items honor status).
3. Paginate **after** status filter.
4. FE: when `statusFilter !== "all"`, trust server `items`; remove redundant client status filter **or** keep as defense-in-depth only if server filter active.
5. Fix virtual-scroll contract: either paginate with `has_more` or raise intentional “fetch all pages” loop — do not pretend page_size=500 works.
6. SDK Rust/TS: add `status` query field.

**Locked product rule**: `status_counts` always reflect the full workspace (after date/pattern only). Selecting Failed filters the list; chip counts stay global.

---

## 7. Edge cases

| ID | Case | Mitigation |
|----|------|------------|
| EC-1 | >100 docs, all failures older than newest 100 | Server filter returns them on page 1 |
| EC-2 | `partial_failure` vs `failed` | Separate buckets; filter exact match |
| EC-3 | `display_status` vs raw `status` (SPEC-057) | Filter on canonical `status`; badge may use display_status |
| EC-4 | Unknown / NULL status | `unknown` count bucket; filter value `unknown` if exposed |
| EC-5 | Case variants `Failed` | Normalize to lowercase at filter boundary |
| EC-6 | status + date_from + pattern | Apply all before paginate; counts = global after date/pattern only |
| EC-7 | Empty workspace | counts 0; empty list |

---

## 8. E2E / contract tests

| Test | Assertion |
|------|-----------|
| `issue319_failed_filter_beyond_page_size` | Seed 120 completed + 5 failed with older `created_at`; `GET ?status=failed` returns 5; `status_counts.failed==5` |
| `issue319_fe_failed_filter_lists_rows` | Playwright: Failed chip N>0 ⇒ table rows == N (or paginated subset with total N) |
| `issue319_status_query_honored_openapi` | Contract test: `ListDocumentsRequest` schema includes `status` |

---

## 9. Cross-refs

- SPEC-027 pagination / IMP-020  
- SPEC-057 display_status / cancelled vs failed  
- SPEC-029 virtual scrolling notes (page_size 500 conflict)
