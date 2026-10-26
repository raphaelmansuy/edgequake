# 09 — Frontend architecture

[Index](README.md) · Experience: [UX/UI](05-ux-ui-spec.md) / [Screens](06-screens-and-navigation.md) · Protocol: [API](07-api-contract.md) · Delivery: [Plan](13-implementation-plan.md)

## Existing stack and reuse

The [package manifest](../../edgequake_webui/package.json) identifies React 19, Next 16, Radix primitives, TanStack Query 5, Tailwind 4, i18next, Vitest and Playwright. Use installed packages; this pack requests no framework/library installation. Extend [clearance types/API/hooks](../../edgequake_webui/src/lib/clearance/types.ts) and [LabelPopover primitives](../../edgequake_webui/src/components/clearance/label-popover.tsx). Reuse existing Button, Checkbox, Input, Dialog, Popover and Sheet wrappers. Prefer a searchable checkbox group to inventing a nonstandard multi-select combobox.

## Proposed module boundaries

| Proposed module | Single responsibility | Used by |
|---|---|---|
| `lib/clearance/access-draft.ts` | Canonical draft, sort/dedupe, equality, before/after diff; no authorization decisions | Upload, single edit, bulk |
| `lib/clearance/access-contract.ts` | Typed serialization of domain access into JSON/multipart/MCP requests | Existing upload serializers/API client |
| `lib/clearance/access-query-keys.ts` | Tenant/workspace/principal/epoch/document/query cache identity | All clearance queries |
| `hooks/use-document-access-draft.ts` | Draft reducer, snapshot, preview lifecycle, conflicts | Editor containers |
| `components/clearance/document-access-fields.tsx` | Sensitivity + security tag controls, no fetching or submit side effects | Upload review and property editor |
| `components/clearance/security-tag-picker.tsx` | Search/pagination/selected tags/checkbox semantics | Shared fields |
| `components/clearance/access-explanation.tsx` | Localized server explanation DTO | Saved property, preview, upload |
| `components/clearance/access-change-review.tsx` | Before/after, reason and broadening confirmation | Single and bulk edit |
| `components/documents/upload-access-review.tsx` | Queue defaults/overrides, frozen per-file submissions | Dropzone and file chooser |
| Existing `document-access-control.tsx` | Read-model loading and explicit commit orchestration | Preview/property entry point |

These are proposed paths, not present files. Keep form bodies around 150–250 lines as a review heuristic; extract cohesive behavior, not arbitrary fragments. Do not add label logic to the already large DocumentManager.

## SOLID and DRY application

- **SRP:** fields render; hooks manage drafts; adapters handle transport; server policy services authorize; repositories persist.
- **Open/closed:** add another ingestion adapter by supplying the typed access envelope to the same admission service, without another policy implementation.
- **Liskov:** JSON/multipart/MCP adapters must accept equivalent valid access values and yield the same normalized label/error semantics; shared fixtures verify substitutability.
- **Interface segregation:** read-only summary needs only a label/explanation; an uploader does not need admin catalog or member directory APIs.
- **Dependency inversion:** editor fields depend on value/options/callback interfaces, not global stores or network clients. Backend services depend on domain types and transaction/repository interfaces.
- **DRY:** one display formatter for list/detail, one wire vocabulary, one upload access serializer, one server mutation path. Do not reimplement clearance as a browser boolean formula.

## Draft and request lifecycle

Initialize from a server snapshot keyed by tenant/workspace/principal/document/version. On open or target change, reset draft using the latest snapshot; avoid one-time `useState(document)` retaining the previous document. Background refresh while dirty marks a conflict, rather than overwriting user edits. Background refresh while clean can replace the snapshot.

Preview is debounced 200ms, request-cancelable and keyed by canonical draft hash plus scope and policy epoch. Only the latest matching response may enable Save. A canceled/stale preview never flashes another draft's access explanation. No auto-submit in effects, close handlers, unmount cleanup or route transitions. Unknown/retired selected tags are preserved visibly until server validation resolves them.

Save freezes payload/version/epoch/idempotency key. Single-flight the operation; disable repeated clicks. `onSuccess` reconciles canonical state before showing Saved. Await targeted invalidation as [TanStack Query's mutation guidance](https://tanstack.com/query/v5/docs/react/guides/invalidations-from-mutations) describes. Validation/network errors preserve draft and field focus. An unknown commit result uses operation status and the same idempotency key; it never blindly resubmits a new mutation.

## Cache and revocation

Partition assignment options, saved access and previews by tenant/workspace/principal. Browser query keys supplement server checks; they do not enforce access. On principal/workspace change cancel in-flight queries, remove sensitive access/content caches, clear previews and reset drafts. Do not use another principal's placeholder data while the new request loads.

On label/policy changes, purge affected preview/body/citation/retrieval/graph data before refetch; invalidating alone leaves old content rendered until refetch. A scope-safe event carries only epoch/change notice; it must not reveal hidden document IDs or names. Cross-tab BroadcastChannel/focus revalidation may carry an opaque scoped epoch, never content. Server checks remain authoritative even if a tab misses events. Full reload must reconstruct the same saved access.

Use one `accessChanged` reconciliation helper, replacing unbounded new invalidation lists. Inventory all current document/graph/stats/query keys before implementation; [F-180-10](02-current-implementation.md) is the starting point. Preserve unrelated drafts on a same-workspace update but mark their policy epoch stale.

## Upload integration

Replace immediate accepted-files dispatch with local review; preserve existing file validation. Each file item holds `{localId, File, scopeSnapshot, accessDraft, override, operationKey, state}`. Retain File objects in memory only. No filenames or access drafts in localStorage, URL query strings or product telemetry. Switching workspace invalidates the pending queue and requires reselection or explicit rebinding before any upload; never reuse an old target invisibly.

`performFileUpload` accepts the frozen access envelope. `buildPdfUploadFormData` and image multipart serialization call one append helper; text/Markdown/JSON call the JSON adapter. Expected batch count, extraction mode and parser settings stay orthogonal. Per-file results use explicit accepted/failed states; retry an unchanged request with its original operation key. After a definite rejection and an explicitly revised draft/policy snapshot, use a new key; never do this while the old outcome is unknown. New sibling admissions do not change policy epoch. No UI PATCH after upload.

## Bulk behavior

Tri-state display summarizes existing selections, but commit payload always contains an explicit operation. “Mixed” is never serialized as a sensitivity. Preview computes exact diffs per document on the server. Replace requires all values, Add preserves existing required tags, Remove is a steward-gated relaxation. Off-page/all-results selection is out of scope; only up to 100 explicit visible IDs are accepted.

## Frontend verification

Pure tests cover equality under tag permutation, transport equivalence and reducer transitions. Component tests assert no PATCH on dismissal and stale preview exclusion. Playwright verifies keyboard/focus/mobile states; actual backend tests verify authority. See [test matrix](12-e2e-test-matrix.md) and [Front Designer lens](lenses/LENS-front-designer.md).
