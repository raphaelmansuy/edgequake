# 10 — Security, sequences and state

[Index](README.md) · Rules: [First principles](01-first-principles.md) / [Policy](04-policy-and-capabilities.md) · Transactions: [DB](08-data-and-db-contract.md) · Failure proof: [Edge cases](11-edge-cases.md) / [Tests](12-e2e-test-matrix.md)

## Upload sequence

```mermaid
sequenceDiagram
    actor U as Uploader
    participant UI as Upload review
    participant API as Admission API
    participant PDP as Clearance domain
    participant DB as PostgreSQL
    participant Q as Outbox worker
    U->>UI: Choose files and access
    UI->>API: Preview canonical per-file labels
    API->>PDP: Check scoped create permission and target label
    PDP-->>UI: Allowed choices and explanation via API
    U->>UI: Upload files
    UI->>API: Bytes plus frozen access and operation key
    API->>DB: Begin and lock workspace policy
    API->>PDP: Recheck current actor, epoch, label and dedup
    alt Invalid or hidden duplicate
        API->>DB: Roll back protected references
        API-->>UI: Safe error without hidden identity
    else Valid
        API->>DB: Insert parent label, original reference, task, audit, receipt and outbox
        API->>DB: Commit
        API-->>UI: Accepted with canonical access version
        Q->>DB: Consume committed outbox once
        Q->>DB: Lock parent and copy current label into each child
    end
```

MCP uses the same final admission: session creation stores the access snapshot and authorized uploader; byte PUT cannot choose its own actor. Finalization rechecks policy and epoch. An expired/revoked session exposes neither staged bytes nor a partially labeled document. A hidden duplicate uses one generic content-conflict response without ID/title/status; it must not trigger delete/reingest of the hidden object. Timing/size side channels are reviewed separately; this specification does not claim constant-time ingestion.

## Edit sequence

```mermaid
sequenceDiagram
    actor E as Editor
    participant UI as Access editor
    participant API as Labels API
    participant DB as PostgreSQL
    participant PDP as Core evaluator
    UI->>API: Read saved access and capabilities
    API-->>UI: Label, version, epoch and explanation
    E->>UI: Change draft
    UI->>API: Preview exact target label
    API-->>UI: Change kind and required reason
    E->>UI: Save access
    UI->>API: PATCH with If-Match, epoch, reason and key
    API->>DB: Begin, lock policy, lock document
    API->>PDP: Recheck current before and after labels
    alt Stale or unauthorized
        API->>DB: Roll back
        API-->>UI: Conflict or scoped error
        UI-->>E: Retain draft and compare latest
    else Authorized
        API->>DB: Update label and children, audit, receipt, epoch
        API->>DB: Commit
        API-->>UI: 204 with version and epoch
        UI->>UI: Purge stale sensitive views and reconcile
        UI->>API: Read canonical saved access
        API-->>UI: Saved access
        UI-->>E: Saved
    end
```

## Editor state machine

```mermaid
stateDiagram-v2
    [*] --> Loading
    Loading --> ReadOnly: no edit capability
    Loading --> Clean: snapshot loaded
    Loading --> Unavailable: policy lookup fails
    Unavailable --> Loading: retry
    Clean --> Dirty: change fields
    Dirty --> Checking: preview requested
    Checking --> Dirty: newer draft
    Checking --> Ready: current preview allows
    Checking --> Blocked: current preview denies
    Blocked --> Dirty: revise draft
    Ready --> Review: broadening and Save
    Ready --> Saving: ordinary Save
    Review --> Saving: steward and valid reason
    Review --> Dirty: return to editing
    Saving --> Clean: committed and reconciled
    Saving --> Conflict: epoch or version changed
    Saving --> Unknown: response lost
    Saving --> Dirty: validation or definite failure
    Unknown --> Clean: receipt confirms commit
    Unknown --> Dirty: receipt confirms failure
    Unknown --> Unknown: still pending
    Conflict --> Dirty: compare and explicitly rebase
    Dirty --> DiscardPrompt: outer dismissal
    Ready --> DiscardPrompt: outer dismissal
    DiscardPrompt --> Dirty: keep editing
    DiscardPrompt --> Closed: discard
    Clean --> Closed: dismiss
    ReadOnly --> Closed: dismiss
    Closed --> [*]
```

Scope/auth change cancels local requests and returns to Loading with a new identity; it cannot auto-replay Saving. During Saving, dismissal may hide the editor but does not cancel the committed server operation; reconcile by operation ID before reopening. An inner picker closes independently without changing this outer state machine.

## Document access lifecycle

```mermaid
stateDiagram-v2
    [*] --> SelectedLocally
    SelectedLocally --> Validating: Upload
    Validating --> Rejected: invalid access or revoked scope
    Validating --> NeedsReview: omitted label and enforcement on
    Validating --> Reviewed: authorized explicit initial label
    Validating --> LegacyOff: enforcement off
    NeedsReview --> Reviewed: steward review with reason
    Reviewed --> Reviewed: authorized versioned relabel
    LegacyOff --> Reviewed: explicit settings enable with reviewed backfill
    LegacyOff --> NeedsReview: explicit strict enable
    NeedsReview --> Deleted: authorized delete
    Reviewed --> Deleted: authorized delete
    Deleted --> [*]
```

Ingestion state (queued/processing/failed/completed) is an independent axis. A completed document can still need security review. A failed document's original and errors remain label-protected. No automatic policy transition is triggered by successful OCR/extraction.

## Threat model and obligations

| Threat | Boundary | Required control / proof |
|---|---|---|
| Browser forges tag/steward/actor | Browser → API | Server capability + exact domain decision; T-180-05/06 |
| Hidden title/member/profile inference | Lookup/preview/dedup | Authorized target lookup, filtered catalogs, safe 404/error shapes; T-180-03/17/24 |
| Lost update permits broadening | Concurrent mutations | Policy lock + document version + recheck inside transaction; T-180-09/10 |
| Worker inserts a stale-labeled child | Relabel → async writes | Parent shared/exclusive lock protocol; T-180-12/28 |
| Stale context/answer leaks after restriction | Retrieval/cache/stream/history | Scope/epoch fencing and descendant checks; T-180-12/29 |
| Cross-workspace association | FK/API validation | Scope-aware constraints and tag-kind checks; T-180-16/30 |
| Tenant-scoped API key exceeds grant | REST/MCP/system | Operation scopes plus clearance and attributed actor; T-180-08/32 |
| XSS or data leakage in names/reasons | Rendering/analytics/logs | Text rendering, bounded strings, content-free analytics, protected audit; T-180-11/23/31 |
| Misleading off/break-glass UI | Capability read model | Explicit status and audit, no protected claim; T-180-13/33 |

## Revocation contract

A read authorized before a label transaction commits may already have sent bytes. Those bytes cannot be recalled. New reads after commit and any buffered response not yet released must evaluate the latest policy epoch. For ongoing query streams, check epoch before each output frame; if changed, cancel generation/retrieval and terminate with a generic policy-changed event. Do not emit new frames based on stale context. Synchronize the final epoch check and frame admission with the workspace policy serialization boundary, or prove an equivalent atomic fence; a best-effort TTL is insufficient.

Revalidate stored conversation context, citations and retrieval handles when fetched; do not replay inaccessible document fragments from history. Clear browser-visible stale text on policy change notice, focus revalidation or access failure. This cannot guarantee removal from screenshots, clipboard or another offline tab; server authorization must never depend on browser cache clearing.

Disable new UX rollout until live tests prove this boundary across REST, MCP, originals, markdown, pages, multimodal assets, tasks, graph contributions, vector retrieval, citations, context handles and saved conversations. See [T-180-24](12-e2e-test-matrix.md#t-180-24), [T-180-29](12-e2e-test-matrix.md#t-180-29) and [WP-07](13-implementation-plan.md#wp-07).
