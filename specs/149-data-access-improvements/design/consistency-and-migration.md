# WHY separate providers require durable intent and revision fences

Current `UnitOfWork` and milestone outbox do not provide a shared transaction ([F03–F04](current-state-audit.md)). A remote success followed by a process crash cannot be repaired reliably using an in-memory compensation list alone. This design adopts one relational commit boundary and independently replayable projections.

## Authority and schema changes

D4 in [WHY and principles](why.md) makes this an explicit evolution of SPEC-091. Retain `documents`, `chunks`, graph identity spine, typed embedding tables, serving state and existing migration history. Add fields/relations through newly allocated migrations; never edit historical checksums.

| Logical relation (proposed) | Required key / contents |
|---|---|
| mutation_requests | Unique `(scope, operation, idempotency_key)`; request digest, committed receipt |
| object_revisions | Scoped object key + monotonic revision; canonical graph facts/contributions or immutable payload reference; deletion state |
| embedding_manifests | Subject/model/content revision, canonical ID, digest, required targets and durable embedding payload reference |
| projection_events | Event UUID, schema version, aggregate/revision, operation, manifest reference, scope; appended in same transaction as canonical changes |
| projection_deliveries | Unique `(event_id, target_binding)`; state, lease epoch, attempts, next retry, receipt; individual destination acknowledgment |
| data_bindings | Scope/domain -> provider instance, physical index, model, generation; versioned cutover state |
| projection_visibility | Required targets and acknowledged revisions; serves only current authorized revision |

Existing `outbox_events` may be extended only with a distinct versioned event family and consumer. Existing unknown-event acknowledgment must never consume projection events. Keep best-effort milestone telemetry separate. Reuse lease/quarantine infrastructure where it meets these semantics.

The first implementation uses separate projection relations. W3/J08 introduces this minimum schema before committers; W4 adds its delivery/replay behavior. Concrete constraints, claim/ack SQL and idempotency steps are in [persistence recipes](../implementation/persistence-and-provider-recipes.md). Its separate document-publication/object-serving rules avoid cycles between documents sharing graph facts.

Durable graph facts must contain everything needed to rebuild, including lineage, shared-entity contributions, type/direction, descriptions, weights and deletions. Existing relational rows must be audited for completeness before they become authoritative. Store generated embeddings (or durable immutable payload references) before requesting projection; replay must not silently call a changed embedding model. SQL payload storage is acceptable initially; blob storage is an independent port with retention guarantees.

## Commit and delivery sequence

```ascii
Client          Relational authority       Dispatcher       Graph / Vector
  |                      |                      |                  |
  |-- scoped command --->|                      |                  |
  |                      |-- BEGIN              |                  |
  |                      |-- facts + manifest    |                  |
  |                      |-- events + pending    |                  |
  |                      |-- COMMIT              |                  |
  |<-- durable receipt --|                      |                  |
  |                      |<-- claim + lease ----|                  |
  |                      |                      |-- apply revision>|
  |                      |                      |<-- receipt -------|
  |                      |<-- ack same epoch ---|                  |
  |                      |-- verify manifest    |                  |
  |                      |-- publish ready      |                  |
```

1. Validate scope, canonical IDs, manifest size and expected revision. A reused idempotency key with a different digest fails.
2. Commit canonical bounded batch + pending visibility + every required event together. If event append fails, rollback the batch. No remote call or LLM call is made while this transaction is open.
3. Workers claim indexed delivery pages with leases and a fencing epoch. Dependencies (edge endpoints before edges, graph identity before referenced vectors) are represented in manifests. Per-aggregate revision ordering is retained without serializing unrelated scopes.
4. Apply idempotent revisioned writes under row/byte/deadline limits. Transport acceptance is insufficient: obtain the configured provider completion/visibility receipt, or remain pending.
5. Ack the exact target binding and lease epoch. Once every required batch/target is complete, publish using compare-and-set against current object revision and lifecycle. A delete/cancel invalidates late publication.
6. Retain events, payloads and receipts through the recovery/cutover window. Garbage collection checks all required targets and replay retention before removal.

For a co-located PG deployment, adapter-private fast paths may perform compatible writes in the same database transaction, but only if tested for the exact AGE session/extension path. Sharing a pool alone does not prove atomicity. The portable path remains valid even without this optimization. [SQLx transaction semantics](../references/official-sources.md#s03-sqlx-transactions) and [PostgreSQL isolation](../references/official-sources.md#s04-postgresql-isolation) ground local transaction handling.

## Out-of-order writes and unknown outcomes

Use **immutable physical revision IDs** wherever a provider cannot atomically reject stale writes. An expired worker may finish writing revision 7 after revision 8 is current; it cannot overwrite revision 8's physical object. Visibility rejects revision 7 and stale epoch acknowledgment. Cleanup later removes obsolete revisions.

An obsolete upsert can arrive after cleanup and recreate hidden data. [persistence recipes](../implementation/persistence-and-provider-recipes.md#publication-shared-facts-and-stale-cleanup) therefore requires retained tombstones, reconciliation sweeps and a proven writer-drain/barrier before physical erasure is reported complete. Logical deletion does not wait for that physical cleanup.

A mutable provider-side “current” pointer is permitted only with proven monotonic conditional mutation. Merely holding a SQL lease or setting a remote ordering flag cannot fence a delayed network request after lease loss. This is a required capability test, not a vendor assumption. Qdrant ordering/idempotence are useful mechanisms but not a cross-store transaction ([S08/S09](../references/official-sources.md)).

For Neo4j, logical IDs and revisions are properties/constraints, not `elementId`. For Qdrant, derive or persist stable UUID point IDs from the full embedding key and revision; detect mapping/digest conflicts. Scope predicates remain mandatory even when physical indices are dedicated.

## Read and delete visibility

The relational authority owns current lifecycle/revision checks. Candidate hydration batches those checks. Never return content from a missing, deleting, deleted, unauthorized, or non-current revision. Graph traversal validates nodes and edges **before expanding the next frontier**, not merely after constructing a graph, so invisible objects cannot act as traversal bridges.

Provider-side readiness predicates and co-located joins are optimizations; the final validation must use a current authority read for deletion enforcement. Do not check tombstones on a lagging relational replica. The guarantee is relative to the authoritative validation point: a read validated before a concurrent deletion may finish; reads validated after the tombstone commit must exclude it. This is not a global distributed snapshot guarantee.

After filtering stale candidates, bounded refill may return fewer than k with an explicit incomplete/budget reason. Strict requests may wait for a required commit receipt until deadline, then return pending/unavailable. No unlimited retry or whole-index fallback. For multi-entity queries, return current validated revisions; coherent historical snapshots require a separately certified snapshot capability.

Deletion transaction: set tombstone/new generation, revoke serving, cancel obsolete publication, append scoped cleanup intents for **recorded bindings**. Graph cleanup subtracts this document's contributions, retaining shared entities and typed sister edges. Physical cleanup failure leaves deletion pending and observable. A re-upload gets a new revision; a late cleanup must not delete it. Provider outage must not redirect deletion to default storage.

Crash windows, unknown schemas, rate limits, replica lag and partial batches are tested in [contract validation](../validation/edge-cases-and-contract-tests.md). Backoff is bounded with jitter; permanent invalid payloads go to quarantine with affected aggregates non-serving. Unknown event schema blocks/quarantines that delivery, not the entire queue, and is never acknowledged as applied.

## Migration and reversible cutover

1. **Inventory:** record current provider bindings, flags, actual schema versions, scope-null rows, graph/fleet IDs and complete provenance. Identify already-dropped legacy tables; rollback must never depend on them.
2. **Add:** introduce contracts and additive schema. Existing writes remain the serving authority until a durable capture point is established.
3. **Capture:** route every relevant mutation/deletion through the new canonical committer. Briefly fence old writers if necessary; establish a committed watermark only after in-flight legacy writers drain. No snapshot/change-log gap is allowed.
4. **Backfill:** export bounded keyset pages at that watermark (or record a consistent snapshot boundary), remap canonical IDs, preserve scope and model revisions. Stream checksums and persist cursors; restart repeats safely. Scope-null/ambiguous records are quarantined until ownership is resolved.
5. **Catch up:** replay captured events to target bindings. Verify each required manifest, tombstone and contribution. Count equality alone is insufficient; compare per-partition canonical digests, scoped membership, sampled traversal and exact vector fixtures.
6. **Shadow:** execute reads against both bindings under a fixed workload; compare deterministic semantics, recall/score tolerance and performance. Do not serve target results before readiness validation.
7. **Cut over:** atomically change binding generation after backlog and verification gates pass. Invalidate generation-bound caches/cursors; drain in-flight queries or let them finish on their pinned old binding. Return explicit stale-cursor errors.
8. **Soak:** retain and feed the previous binding for an initial 7-day proposed rollback window (operator may lengthen for outage/recovery needs). Include at least one restart and one deletion/replay incident drill. Extend the window if any acceptance gate fails.
9. **Retire:** stop old delivery only after rollback closure. Destructive provider/schema cleanup is a separate, explicitly reviewed operation with export/restore proof; this documentation task performs none.

Rollback is a binding-generation change only when the old target is caught up with all committed revisions/tombstones. Otherwise pause affected writes or replay the retained log to it before switching. Never silently revive retired KV/vector stores. Restoring an older binary after new authoritative writes requires proven schema/event compatibility; a feature-flag flip alone is insufficient.

Backup recovery restores canonical state, binding manifests, events and payloads to a consistent checkpoint, then rebuilds projections. An isolated vector/graph snapshot is not proof of a recoverable product state. Migration and recovery work are paced through existing resumable migration jobs, not serving startup.
