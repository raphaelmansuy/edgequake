# 07 — API contract

[Index](README.md) · Semantics: [Policy](04-policy-and-capabilities.md) · Persistence: [DB](08-data-and-db-contract.md) · Ordering: [Sequences](10-security-and-sequences.md) · Tests: [Matrix](12-e2e-test-matrix.md)

All endpoints below are **proposed additions/extensions**, except the existing labels PATCH. Paths are relative to `/api/v1`. Tenant/workspace headers are resolved by existing authorization middleware and must match path/target scope. No body-supplied actor, workspace override, steward flag, rank or capability is trusted. Hidden target lookup precedes version/error detail disclosure.

## Read and preview endpoints

| Method/path | Purpose | Authorization |
|---|---|---|
| GET `/workspaces/{workspace_id}/clearance/assignment-options?q=&cursor=&limit=50` | Enforcement, policy epoch, upload capabilities, allowed sensitivities and page of assignable tag IDs/names | Active scoped member; writer sees own assignable choices, not full admin vocabulary |
| GET `/documents/{document_id}/access` | Canonical saved access, label version, ETag, document capabilities, rule explanation | Must be able to read document; indistinguishable 404 otherwise |
| POST `/workspaces/{workspace_id}/clearance/preview` | Evaluate exact proposed create/edit/bulk labels without mutation | Scoped operation permissions; all edit targets visible |
| POST `/documents/labels/batch` | P1 bounded atomic existing-document edit | Full permission and version checks for every document |

Keep existing admin vocabulary/profile routes and profile simulation. Do not weaken `list_clearance_tags` to expose its entire catalog to every writer; the new assignment endpoint is filtered. Query length ≤128 characters, limit 1–100, opaque cursor bound to workspace/principal/epoch/query. Stale cursor returns `POLICY_CHANGED`; restart search without clearing selected tag IDs.

Illustrative document access response (UUIDs abbreviated only in prose; actual payloads require valid UUIDs):

```json
{
  "document_id": "018f1f1f-1111-7111-8111-111111111111",
  "access": {
    "classification": "confidential",
    "compartment_ids": ["018f2f2f-2222-7222-8222-222222222222"]
  },
  "label_version": 7,
  "policy_epoch": 42,
  "enforcement": true,
  "capabilities": {
    "can_edit": true,
    "can_downgrade": false,
    "can_manage_vocabulary": false,
    "can_check_member": false
  },
  "explanation": {
    "kind": "rank_and_all_tags",
    "classification": "confidential",
    "required_tags": [{"id": "018f2f2f-2222-7222-8222-222222222222", "value": "HR"}],
    "steward_exception": true,
    "caller_can_read": true
  }
}
```

Return `ETag: "label-7"` and `Cache-Control: private, no-store` for capabilities, preview, mutation status and sensitive access views. Tenant/document identity remains part of the cache key even when version text is identical. Numeric versions/epochs use the repository's safe integer conventions; if values exceed JavaScript safe integer range, generated contract must use decimal strings consistently.

Preview body: `mode: create|edit|bulk`, `items: [{client_file_id?, document_id?, access, label_version?}]`, `expected_policy_epoch`, optional `check_member_id` only with capability. Create omits document ID; edit requires it. Batch size 1–100. The server returns per-item `allowed`, stable `reason_codes`, `change_kind: initial|unchanged|strengthen|broaden|mixed`, `requires_reason`, `caller_can_read`, normalized access, current versions and epoch. Overall `allowed` requires all items allowed. Only authorized stewards receive member results. Preview does not enumerate users or return corpus titles. Its result is advisory: commit always rechecks current authority and state.

## Existing document PATCH

Extend existing `PATCH /documents/{document_id}/labels`; preserve `classification` and `compartment_ids`:

```http
PATCH /api/v1/documents/018f1f1f-1111-7111-8111-111111111111/labels
If-Match: "label-7"
Idempotency-Key: 9a1b78fc-9ff1-442e-ad02-22b8017168f4
Content-Type: application/json

{
  "classification": "internal",
  "compartment_ids": [],
  "expected_policy_epoch": 42,
  "reason": "Approved for the wider internal audience."
}
```

Success remains 204, with new ETag and `X-Policy-Epoch` headers. Browser fetches the canonical read model after commit; do not fabricate saved state from the submitted draft. Set CORS exposed headers for ETag/epoch if cross-origin deployment uses them. No-op returns current ETag without version/epoch increment or duplicate audit, after permissions and preconditions pass.

## Upload transport contract

JSON upload requests gain optional `access: {classification, compartment_ids, expected_policy_epoch}` and per-file `client_file_id`. Multipart uses exactly one `access` field containing that JSON object; reject repeated/conflicting fields. Form field order must not matter. PDF/image/text/batch adapters deserialize to the same domain type. Batch JSON carries access per item; UI defaults are materialized per file, never ambiguously inherited by the server.

When access is omitted under enforcement, atomically create unverified + uploader + empty required tags. When explicitly supplied, reject null, partial objects, unknown words, malformed IDs, >32 tags or mismatched epoch. Deduplicate and sort IDs before set comparison. Cap decoded access JSON at 16 KiB. Generic metadata keys that impersonate reserved security fields are rejected as `RESERVED_SECURITY_METADATA`, rather than overriding typed values. These are proposed bounds; implementation must keep one constants/schema source.

New-document admission starts label version 1 without incrementing policy epoch. A multi-file review can therefore submit independent files against one unchanged policy snapshot. A genuine intervening membership/profile/vocabulary/relabel change still invalidates that snapshot and requires fresh review. Normal ingestion data invalidation updates lists and retrieval freshness separately.

REST accepts the existing content types/formats; this spec adds no DOCX/Excel support. Label validation happens before content ingestion/LLM calls. Temporary raw upload bytes may be staged under inaccessible server-controlled storage while multipart validation completes; invalid/abandoned uploads are cleaned up by bounded TTL cleanup. Never expose an original before label admission commits.

MCP session creation accepts the same access object; bind the immutable draft and actor/scope to the session, not to the upload URL caller. Revalidate at final admission because permissions may change during byte transfer. Old clients may omit labels; missing uploader identity for enforced unverified admission is rejected rather than creating an ownerless review item. See [Sequence](10-security-and-sequences.md).

## Bulk and idempotency

Batch body: `operation: add_tags|remove_tags|replace_access`, `items: [{document_id, label_version}]`, operation payload, `expected_policy_epoch`, reason where required. Duplicate document IDs are rejected. Server expands operation against each current label; omitted fields in add/remove preserve rank and existing unrelated tags. Replace requires the complete label. Response 200 contains only authorized requested IDs and resulting versions/epoch. A failure changes none; no partial-success semantics in this release.

Idempotency keys are required for label mutations. Receipts bind workspace, actor, operation UUID and the canonical expanded request, including document IDs and versions. Replays are reauthenticated and all targets must remain visible. Same key/body returns the committed outcome; changed body returns 409. A transaction advisory lock represents an in-flight operation without a separately committed pending lease. The receipt commits with the label, descendants, audit and epoch; rollback removes all of them. Scoped GET `/workspaces/{workspace_id}/clearance/operations/{operation_id}` returns `pending`, `committed` or `unknown` without document details. `unknown` means there is no active lock or committed receipt; it is not a successful outcome. Committed receipts are retained without automatic expiry, covering at least the 24-hour retry window. Uploads use existing content-hash/MCP-session deduplication; a different requested label on a duplicate conflicts instead of silently relabeling it.

## P1 library filter extension

Extend the existing document-list request with optional `classification` (one existing wire word), repeated `security_tag_ids` (up to 32 unique governed UUIDs) and `review_state=any|needs_review|reviewed`. Tags in the filter mean the document must contain **all** selected tag IDs; this search predicate is separate from, and intersected with, the caller's clearance. `needs_review` means rank 5; `reviewed` means ranks 1–4. Contradictory classification/review combinations return 422 `INVALID_ACCESS_FILTER`; unknown/foreign filter tag IDs use a generic invalid-filter response without catalog details.

Combine these filters with existing text/date/status filters before count, sort and pagination. Return the existing page shape with a count for that same authorized filtered predicate; never a total for the hidden corpus. Empty results show a clear filter-reset action, not an invitation to change security permissions. Filter cursors carry a policy epoch and are rejected/restarted after policy changes so pages do not mix different access snapshots. Offset-based compatibility callers retain existing paging parameters but receive fresh authorized counts on each request. No new facets may count hidden documents.

## Errors and deterministic precedence

| HTTP | Code | User recovery |
|---|---|---|
| 400 | `INVALID_ACCESS_PAYLOAD` | Fix malformed JSON/duplicate fields/invalid transport |
| 401 | `AUTHENTICATION_REQUIRED` | Reauthenticate, then re-fetch capabilities before explicit retry |
| 403 | `ACCESS_CHANGE_FORBIDDEN`, `STEWARD_REQUIRED` | Change draft or contact authorized steward |
| 404 | ordinary not-found shape | Hidden and nonexistent target are identical; no tag/member hints |
| 409 | `POLICY_CHANGED`, `IDEMPOTENCY_KEY_REUSED`, `OPERATION_IN_PROGRESS` | Reload/compare or reconcile as appropriate |
| 412 | `LABEL_VERSION_CONFLICT` | Fetch current saved label, compare; never silent merge |
| 422 | `UNASSIGNABLE_TAG`, `INVALID_CLASSIFICATION`, `REASON_REQUIRED`, `CLEARANCE_DISABLED`, `RESERVED_SECURITY_METADATA` | Inline actionable field error; no full forbidden catalog echo |
| 422 | `ACCESS_CHANGE_TOO_LARGE` | Select fewer documents or use an approved administrative procedure; no partial change |
| 428 | `LABEL_PRECONDITION_REQUIRED` | Refresh access model, send If-Match and epoch |
| 429 | `ACCESS_RATE_LIMITED` | Retain draft, honor Retry-After |
| 503 | `ACCESS_CHECK_UNAVAILABLE` | Fail closed; retain draft and Retry |

Order: authenticate → resolve scope/target visibility → operation authorization → payload limits/shape → idempotency reconciliation → epoch/version → exact target-label policy → persistence. Unknown/foreign tag IDs share the same external unassignable error. Internal audit/diagnostic logs retain precise cause without leaking it to the caller.

## Compatibility and generated contract

Deploy server read/capability additions before new browser behavior. Existing upload omission remains supported. Current labels PATCH has no version: after server support lands, a short compatibility window may obtain locks and validate current labels without caller preconditions, but must emit deprecation telemetry. This window is **not** the final correctness contract and must close before SPEC-180 release acceptance; then missing preconditions returns 428. Publish a client migration note, regenerate [OpenAPI](../../edgequake/crates/edgequake-api/src/openapi.rs) and [web schema](../../edgequake_webui/openapi/schema.d.ts), update [clearanceApi](../../edgequake_webui/src/lib/clearance/api.ts) and upload types. No new third-party package is needed. See [WP-02](13-implementation-plan.md#wp-02).

## Delivered member check

GET `/workspaces/{workspace_id}/clearance/members?q=...` is steward-only, requires at least two search characters and returns at most 20 active workspace members. Preview accepts optional `check_member_id` and evaluates that member against the exact draft. Read-only stewards may inspect, but cannot change labels. See [the live evidence](e2e/README.md).
