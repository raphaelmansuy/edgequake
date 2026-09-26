# WHY concrete persistence rules prevent incompatible implementations

This document resolves implementation choices left abstract in [access contracts](../design/contracts.md) and [consistency and migration](../design/consistency-and-migration.md). Names and SQL below are **proposed**; map them to existing canonical tables where their semantics already match. Do not create a second document/chunk database. Senior gates G2/G3 in [feasibility assessment](feasibility-and-releases.md) review the implementation before enabling authoritative writes.

## Request admission and API compatibility

One API admission layer authenticates, resolves tenant/workspace membership, normalizes legacy identifiers, validates deadlines/budgets, then constructs private-field `AccessScope`. Internal workers reconstruct scope only from validated durable commands; fleet operations use a separate authorized type. Never accept caller-supplied physical table/collection names.

Existing [DocumentFilter](../../../edgequake/crates/edgequake-api/src/handlers/query_types.rs) defines public `document_ids: []` as no filtering. Keep that behavior. Resolve the public union of explicit IDs and title patterns, then intersect date constraints within scope. Distinguish “no membership criterion supplied” from “a supplied criterion matched zero documents”: the former becomes internal `None`; the latter becomes `Some([])` and returns no candidates. This translation happens once, before vendor routing. An unknown foreign-scope ID does not broaden a filter. Bound title/date resolution with indexed pages or provider-side predicates; never hydrate all documents to filter them.

| Access result | HTTP behavior inside existing error envelope | Retry rule |
|---|---|---|
| Invalid dimension/filter/cursor/value | 400, safe validation message | Correct request |
| Unsupported requested search option | 422, capability named | Choose supported option; unsupported startup profile prevents readiness |
| Unauthorized scope | Existing 401/403 policy; cross-scope object read appears absent | No retry with same authorization |
| Absent authorized object | 404 | No implicit creation |
| Expected revision or idempotency digest conflict | 409 | Read current state or choose a new intentional command |
| Explicit strict-read receipt still pending at deadline | 503 with safe pending code and bounded Retry-After | Same receipt within client policy |
| Provider unavailable / transient write outcome unknown | 503; preserve unknown outcome internally, expose request/receipt lookup identifier | Retry same idempotency key only |
| Request deadline exceeded | 504 | Respect total deadline; writes may be unknown |
| Admission concurrency/rate budget exceeded | 429 with Retry-After | Bounded retry |
| Internal invariant violation | 500 with correlation ID | Investigate; do not return empty success |

Preserve existing upload/delete HTTP 202 responses: they acknowledge durable accepted work, not projection completion. Add optional status fields compatibly. Default query mode requires all components used by that mode; an unavailable required graph/vector component yields 503. Optional partial retrieval must be an explicitly requested/supported mode with a degradation field; silently dropping a retrieval arm is prohibited. Empty valid scoped search remains 200. Keep `/health` compatible; add proposed `/ready` for 200/503 readiness and additive per-axis status in health. Provider schema mismatch fails readiness; startup never repairs it implicitly.

## DTO fields to implement before adding an adapter

| DTO | Mandatory fields and rules |
|---|---|
| PreparedIngestionBatch | scope, document ID, ingest generation, batch ordinal, expected revision, idempotency key, schema version, canonical digest, bounded chunks/facts/contributions/embedding payloads; all nested scope agrees |
| CommitReceipt | request key, command digest, document generation, committed canonical IDs/revisions, manifest ID, durable commit token; independent of provider completion |
| FinalizeIngestion | scope, generation, expected batch count, streaming manifest digest, expected document revision; publish only after all required batches and deliveries exist |
| ProjectionEvent | event UUID, event schema version, scope, object kind/ID/revision, operation, immutable manifest/payload reference, payload digest |
| ProjectionDelivery | event UUID + immutable binding UUID, state, epoch, owner token, attempt, due time, lease expiry, provider receipt, completion proof; old binding configuration retained |
| VisibilityRequest | scope, bounded exact object keys/revisions and contribution references; batch result positional, absence explicitly non-serving |
| DeleteReceipt | scoped document ID, tombstone revision, cleanup manifest ID, recorded target binding IDs; logical deletion and physical cleanup separate |

Use typed ID wrappers over existing UUID types, `u64` checked into database signed ranges for revisions/counters, and UTC instants with adapter conversion. Do not use `usize` as a persisted wire type. Serialize event versions as a tagged closed enum plus explicit schema number. Unknown versions quarantine before deserializing arbitrary vendor payloads.

Digest recipe v1: SHA-256 over a versioned, documented canonical binary encoding with fixed field order, explicit lengths, UTF-8 bytes, fixed numeric endianness and stable enum tags. Reject NaN/infinite values before encoding, normalize negative zero, and preserve ordered batch semantics. Sort only maps/set-valued inputs by canonical key; include scope, model/preprocessing revision, operation and expected revision. Persist the exact prepared payload bytes. Do not hash ordinary randomized map serialization or float display strings. Map sorting costs O(m log m); do not describe it as linear. Golden vectors freeze the digest across adapters/languages. Request IDs use existing UUID generation; physical revision UUIDs are allocated once and persisted with a unique logical key, avoiding dependence on hash-derived UUID collision assumptions.

## Minimum schema before any new writer

PostgreSQL types below are normative logical types; SQLite uses BLOB/TEXT UUID encoding chosen once, INTEGER revisions/timestamps and encoded payloads, tested for equivalent ordering and precision. All scoped child relations have composite scoped FKs, not only an unscoped object UUID reference. All mandatory fields are NOT NULL unless called out.

| Relation | Fields beyond existing domain rows | Keys, checks and required indexes |
|---|---|---|
| mutation_requests | tenant/workspace UUID, operation text, idempotency key text, digest bytea, receipt bytea, created_at timestamptz | PK `(tenant,workspace,operation,key)`; digest length=32; bounded key length; receipt never null |
| object_revisions | scope, kind, logical_id UUID, revision bigint, state, physical UUID, digest, payload reference | PK `(scope,kind,logical_id,revision)`; UNIQUE physical UUID; revision>0; scoped FK to canonical identity; immutable contents |
| canonical current state | current revision, lifecycle, serving revision nullable | One row per scoped logical object; revision CAS; serving revision never exceeds current; may extend existing domain tables |
| graph_contributions | scope, fact ID/revision, contribution UUID, source document/generation/chunk ID, payload digest and full source data | Unique source contribution key; indexes `(scope,source_document,generation)` and `(scope,fact_id,revision)`; no truncated lineage |
| embedding_manifests | scope, subject/family/model/content revision, payload bytes/reference, dimension, metric, digest, physical UUID | Unique full EmbeddingKey; source blob retention FK or ownership record; model descriptor immutable |
| ingest_batches/manifests | scope, document/generation, batch ordinal, digest, expected count, state | Unique `(scope,document,generation,ordinal)`; nonnegative count; staged rows non-serving; ordered manifest scan index |
| data_bindings | binding UUID, scope, role, provider instance/config reference, layout, physical index, model descriptor, generation, state | Binding contents immutable; active pointer unique per scope/role/model; generation CAS; secrets stored by reference |
| projection_events | event UUID, scope, object key/revision, schema version, operation, manifest reference, digest, created_at | PK event; unique deterministic event identity for command/batch/object/action; scoped FK to manifest/revision |
| projection_deliveries | event UUID, binding UUID, state, next_attempt_at, lease_until nullable, lease_owner nullable, epoch bigint, attempts integer, receipt nullable | PK `(event_id,binding_id)`; both FKs; nonnegative counters; leased state requires owner/expiry; indexed due queue and expired leases |
| projection_visibility | scope/object/revision/binding, completion receipt and verified generation | Unique target completion; compare against canonical current state at publication; never infer completion from attempt count |

Keep events separate from legacy milestone telemetry in the first implementation. A future schema consolidation must retain distinct consumers/version handling. Retain request receipts at least as long as replay/retry and binding rollback windows; an expired idempotency key is not safe to treat as new without an explicit API retention contract. Retain tombstones until every binding and possible delayed operation is accounted for.

Use separate indexes on deliveries `(next_attempt_at,event_id,binding_id)` for pending/retry rows and `(lease_until,event_id,binding_id)` for leased rows. Claim due work and reclaim expired leases in bounded operations; avoid an unindexed OR over the entire table. Add scoped document list indexes `(tenant,workspace,created_at,id)` and contribution reverse indexes before enabling affected queries. Validate with realistic EXPLAIN and SQLite query plans, including many completed rows.

## Transaction and idempotency recipe

The PostgreSQL committer owns a real SQLx transaction. Its helpers receive the same executor (`&mut **tx` when the helper holds `&mut Transaction`, or `&mut *tx` for an owned local transaction), not a pool. This distinction is checked by rollback tests, not parameter names.

```text
prepare outside transaction: validate -> normalize -> bound -> digest
BEGIN
  read committed receipt by (scope, operation, idempotency key)
  if present: verify digest; return same receipt after ending transaction
  lock canonical document row, then touched fact rows in stable key order
  verify scope, lifecycle and expected revision
  reserve/resolve canonical chunk IDs using database RETURNING/readback
  insert immutable staged rows, contributions, embeddings and manifest
  insert projection events AND one delivery per required binding
  construct receipt from committed IDs; insert mutation_requests receipt
COMMIT
return receipt; network loss during COMMIT means outcome unknown
```

Reserve a new document via its scoped uniqueness arbiter before locking it. For fact rows absent initially, use insert-on-conflict then lock in stable order. All writers, including delete/reprocess, acquire document locks first and fact locks in the same order. Multi-document maintenance locks document IDs in stable order too. Sorting lock keys is O(n log n) and deliberate deadlock avoidance; other transformations remain expected linear. Serialization/deadlock retries replay the entire prepared command within a total deadline.

Concurrent commands using the same request key can race at the final receipt insertion: its unique violation rolls back **all** effects of the loser, which then reads the winner's committed receipt in a new transaction and compares digests. Never catch the violation and continue in an aborted transaction. Different commands with the same expected revision conflict. Do not generate different payload/IDs on retries; reuse prepared state and resolve canonical IDs again as needed.

On ambiguous commit, query the receipt against the writer authority using the same key; if absent and the first transaction may still be finishing, repeat the same command under uniqueness/locking, not a new request key. A rolled-back attempt leaves no receipt. A committed attempt has the full receipt/events together. Quota/permission decisions requiring transactional invariants participate in the same command using existing policy inputs plus locked/conditional authority state.

Large ingestion uses staged batch commits and a bounded finalization command. Store expected batch count/digest in the manifest; finalize checks durable count/digest and delivery completeness using indexed aggregation, never loads every chunk into memory. Reprocessing creates a new generation. Until it is published, new content is non-serving; initial implementation may withhold the previous generation too, favoring correctness over continuous availability. This availability tradeoff must be visible in status and tested.

## Worker claim and acknowledgment recipe

One worker policy owns retries: at most five inline attempts bounded by the request deadline; a durable delivery can be scheduled again later according to an operator-visible retry/quarantine policy. Rate-limit retry-after is capped by the deadline. Do not quarantine a transient outage merely because five inline attempts elapsed. Permanent invalid payload/schema errors quarantine immediately. Initial proposed lease=30 seconds, renew every 10 seconds, provider timeout=10 seconds; calibrate and record these, rather than assuming they bound remote execution after a timeout.

PostgreSQL due-claim statement shape, inside a short transaction (column aliases abbreviate the schema above):

```sql
WITH due AS (
  SELECT event_id, binding_id
  FROM projection_deliveries
  WHERE state IN ('pending', 'retry') AND next_attempt_at <= now()
  ORDER BY next_attempt_at, event_id, binding_id
  LIMIT $1
  FOR UPDATE SKIP LOCKED
)
UPDATE projection_deliveries AS d
SET state = 'leased', lease_owner = $2, lease_until = now() + $3,
    epoch = d.epoch + 1, attempts = d.attempts + 1
FROM due
WHERE d.event_id = due.event_id AND d.binding_id = due.binding_id
RETURNING d.*;
```

Bind `$1` positive batch cap, `$2` worker-instance UUID, `$3` validated interval; no string concatenation. A separate bounded expired-lease reclamation increments epoch, clears owner and makes the delivery due again. Renew/ack compare event, binding, leased state, owner, epoch and unexpired lease using the authority clock. An ack updates the completion receipt in the same transaction as the visibility row. A zero-row ack is lost ownership, never success. Publication separately checks current object revision and all required binding receipts. [PostgreSQL locking guidance](../references/official-sources.md#s15-postgresql-queue-claim-locking) supports queue claims, not a general consistent scan.

```ascii
pending -> leased -> applied
              |         |
              |         +-> conditional publication
              +-> retry -> pending
              +-> quarantined
expired lease -> retry (new epoch; old ack rejected)
```

Adapter responses preserve partial item outcomes when possible. With an unknown batch outcome, replay the same immutable IDs/payload; never invent new IDs. Graph endpoint prerequisites are explicit delivery dependencies; retries cannot create dangling logical endpoints. Unknown event versions remain inspectable in quarantine and never pass through the legacy acknowledge-unknown consumer.

## Publication, shared facts and stale cleanup

Use two distinct readiness levels. **Document publication** checks its current generation, completed staging manifest and exact required delivery receipts. **Object serving** checks current fact/vector revision, required target receipts and source-document visibility. Receipt completion is durable even if a newer fact revision supersedes it; document publication does not wait for another document's visibility. This avoids readiness cycles between documents sharing an entity.

For the initial implementation, a projected aggregate description/edge carries an immutable complete contribution manifest. If any source contribution's document/generation is not currently serving, suppress the entire aggregate until a replacement excluding that contribution is projected. Never display a description partly derived from deleted or pending content. The surviving document remains published, but shared graph results may temporarily underfill. Regeneration uses complete canonical contributions, without LLM re-extraction; aggregate generation policy and any required generated description are persisted as canonical revision data. Final authority checks include every contributing document in bounded batches. Very large contribution sets require cached authority-owned validity metadata invalidated transactionally on delete, or bounded suppression; never silently cap the security check to the display lineage length.

Graph expansion validates the candidate edge revision, its source/target logical identities and source-document visibility **before** adding the next frontier. Projection-only anchors hold no user content and cannot authorize traversal. A deleted bridge therefore cannot connect two otherwise visible nodes.

Immutable physical IDs prevent old upserts overwriting new revisions. Deletes name exact recorded physical revisions, never all points/edges for a logical ID after re-upload. Late old upserts can recreate an obsolete physical object; authority validation still hides it. A periodic sweep reconciles obsolete revisions and tombstones. Report physical cleanup as pending while any unknown/in-flight write could recreate data. To declare physical erasure complete, the adapter must prove a drained/revoked writer boundary or a qualified provider barrier, then purge and verify. A timeout or expired SQL lease alone is insufficient. Keep a tombstone/reconciliation record for the full retention window; privacy erasure deadlines need an operational escalation path if a provider remains unavailable.

## Provider recipes

**PostgreSQL relational / AGE / pgvector.** Start with the existing migrations and transaction executor. Keep AGE session setup private to graph connections. Pass every filter to parameterized SQL; dynamic dimension/index expressions come only from validated binding metadata. Co-located vector queries may use local joins; standalone projections use only local filter payload plus injected authoritative hydration. Exact-vector tests use the distance operator with index-compatible ORDER BY; approximate search exposes bounded underfill. Do not claim atomic AGE plus domain transactions until the exact connection/session path is tested.

**Qdrant.** First qualify a single-node image and typed HTTP DTOs over the existing client. Provision collections by immutable model/dimension/metric binding; a model change creates another binding. Required payload: tenant, workspace, family, subject ID, document/contribution reference, model revision, content revision, manifest ID and digest. Provision indexes for scope and business filters. A write uses the persisted UUID point ID and `wait=true`; parse the operation result and validate completion/visibility before acknowledging. Repeated point IDs overwrite, so immutable payload identity must already be enforced by the authority. [Official upsert API](../references/official-sources.md#s17-qdrant-completion-options) is the transport contract. Search compiles scope AND business predicates, includes exact-mode configuration only when certified, tags score metric and respects candidate/deadline caps. Delete explicit UUIDs in bounded batches. Collection names derive only from binding UUIDs; no user-provided names.

**Neo4j.** First qualify single-node Query API; existing HTTP client avoids assuming an official Rust Bolt driver. Parse `errors` even on accepted HTTP responses; preserve the database/transaction routing endpoint and completion bookmark as applicable. Use parameterized `UNWIND $rows`, stable labels and static relationship types. Never interpolate arbitrary relation text as Cypher syntax.

Initial portable graph mapping: `EntityKey` anchor with unique scoped application ID; immutable `EntityRevision` nodes with unique physical ID; immutable `EdgeRevision` nodes with unique physical ID, logical edge ID, revision, relation type, direction, digest and contribution manifest; static `FROM`/`TO` relationships connect each edge revision to its anchors. This edge-as-node mapping avoids requiring edition-specific relationship uniqueness constraints. A batch transaction MERGEs a revision ID, verifies digest equivalence, then ensures exactly one FROM and one TO. Failed digest/endpoint equivalence rolls back. Index anchor scope/logical ID and revision physical ID. Store canonical current content in revision nodes, not mutable anchors.

Incident-edge reads seek anchor IDs, page EdgeRevision candidates by stable physical ID, and validate current revisions through the authority. In/out/self-loop semantics derive from FROM/TO and stored direction; count a self-loop once as a logical returned edge. Native graph work and indexes are verified with PROFILE. Budget two structural relationships per logical edge and bound old-revision scans; stale-revision excess may return truncation and trigger cleanup, never an unbounded scan. Delete only exact revision nodes and their structural relationships. Remove an anchor only when no retained revision needs it. AGE may retain native edges with equivalent key/revision semantics; adapters need not share a physical layout. [Neo4j Query API](../references/official-sources.md#s12-neo4j-query-api) remains the authoritative transport reference.

**SQLite.** Add the SQLite feature to the pinned SQLx dependency without upgrading it. Qualify a local persistent file, FK enforcement on every connection, WAL and finite busy timeout. Use `SqlitePool::begin_with("BEGIN IMMEDIATE")` to obtain an owned SQLx transaction; [SQLx 0.8.6 documents this API](../references/official-sources.md#s18-sqlx-owned-custom-begin-and-sqlite-connection-settings). Test rollback-on-error/drop before reusing the connection; never issue raw BEGIN on a pooled connection then return it with an open transaction. Begin/commit busy errors follow [SQLite transaction rules](../references/official-sources.md#s16-sqlite-write-transactions); COMMIT BUSY can leave a transaction active, so resolve or roll back through the owned transaction lifecycle before retrying the whole command. Because SQLx commit consumes the guard, test its error/drop cleanup explicitly rather than assuming a second commit call is available.

One process owns a bounded writer queue; readers use short snapshots. Claim/renew/ack run inside a short immediate transaction and use the same CAS predicates; no SKIP LOCKED imitation. Clock readings come from one injected authority-clock implementation, persisted UTC expiry; restart reclaims expired leases using a new worker-instance token. Test backward/forward clock adjustments conservatively: premature retry cannot publish stale work, and backward shifts cause an observable lag alert rather than data loss. Integer timestamps, UUID encoding, NULL uniqueness, booleans, JSON and collation have explicit contract fixtures. Refuse unsupported filesystem/replica topology in deployment configuration and document that the application cannot reliably auto-detect every filesystem.

## Implementation checks before declaring a recipe complete

Use [E2E specification](../validation/e2e-test-specification.md) scenarios plus adapter contract tests for: conflicting duplicate key/digest, scope mismatch, empty batch, large payload, concurrent revision change, both sides of each crash boundary, unknown commit, lease stealing, shared-fact deletion, unavailable authority, unavailable projection and interrupted migration. Inspect actual provider rows/receipts as well as API responses. The SQL snippets are design recipes; executing them successfully in isolated integration tests is an implementation requirement, not a claim made by this documentation.
