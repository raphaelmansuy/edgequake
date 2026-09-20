# WHY contracts must specify meaning before adapters

A method named `batch`, `filtered`, or `transaction` is insufficient. The [audit](current-state-audit.md) shows name/behavior mismatches. The following contracts are proposed requirements, not descriptions of today's API.

## Scope, identity, and revision

`AccessScope { tenant: TenantId, workspace: WorkspaceId }` has private fields and is constructed after membership validation. Storage checks both fields on every user-data read/write. Identity-level and fleet-admin operations use separate authorized context types; `None` never means “all tenants.” Workspace-name lookup is scoped by tenant, resolves once at ingress, and returns a UUID. It never performs unscoped `LIMIT 1` resolution.

Every record carries immutable scope and a domain UUID. `GraphNodeKey` contains scope and entity UUID; `GraphEdgeKey` contains scope, source UUID, target UUID, normalized relationship type, and direction. A persisted edge UUID is the transport identity; do not rely on vendor-generated IDs. Existing name-based identifiers are translated by one versioned mapping. Preserve parallel relationship types and self-loops. A node deletion explicitly declares its incident-edge behavior.

`EmbeddingKey` contains scope, family, subject UUID (or mapped report ID), model revision, and content revision. A model descriptor records provider/model/version, dimension, distance metric, normalization and preprocessing revision. Equal dimensions do not imply comparable models. A separate index binding maps that descriptor to physical collections/tables; callers never supply a table name.

Before migration, census aliases, Unicode relation normalization, database collation and UUID/name collisions. Existing PostgreSQL `UPPER` and Rust Unicode uppercasing are not presumed identical. Persist canonical IDs/types at admission; freeze the normalization version. Reject collisions or reconcile them explicitly; never rename existing graph keys opportunistically.

## Narrow port surface

Illustrative Rust signature sketches below omit DTO definitions. Keep existing `async_trait` (or explicit boxed futures) for dynamic dispatch. Native async trait methods alone are not dyn-compatible according to the [Rust reference](../references/official-sources.md#s01-rust-dynamic-traits).

[persistence recipes](../implementation/persistence-and-provider-recipes.md#dto-fields-to-implement-before-adding-an-adapter) specifies the mandatory DTO fields and command encoding; implement those alongside these ports rather than inventing provider-specific request shapes.

```rust
#[async_trait]
pub trait DocumentReader: Send + Sync {
    async fn get_many(&self, scope: &AccessScope, ids: &[DocumentId])
        -> Result<Vec<Option<DocumentView>>, AccessError>;
    async fn list(&self, scope: &AccessScope, request: &DocumentPageRequest)
        -> Result<CursorPage<DocumentView>, AccessError>;
}

#[async_trait]
pub trait IngestionCommitter: Send + Sync {
    async fn commit_batch(&self, command: &PreparedIngestionBatch)
        -> Result<CommitReceipt, AccessError>;
}

#[async_trait]
pub trait LifecycleCommitter: Send + Sync {
    async fn tombstone_document(&self, command: &DeleteDocument)
        -> Result<DeleteReceipt, AccessError>;
}

#[async_trait]
pub trait ScopedVectorSearch: Send + Sync {
    async fn search(&self, request: &VectorSearchRequest)
        -> Result<VectorSearchPage, AccessError>;
}

#[async_trait]
pub trait ScopedGraphRead: Send + Sync {
    async fn incident_edges(&self, request: &IncidentEdgesRequest)
        -> Result<CursorPage<VersionedEdge>, AccessError>;
}
```

Complete the existing repositories behind these role interfaces rather than maintaining a competing store. Atomic commands own scope, request idempotency key, expected revision, bounded rows, and manifests. They expose no transaction object. The relational adapter executes all required writes and durable delivery rows using one private transaction and returns committed IDs/revisions. Repositories called inside it use that same executor. An external caller cannot accidentally pass a transaction from another provider.

A large document is a series of bounded staging commits plus one publication commit verifying the staged manifest. Do not require one unbounded transaction for an entire document. Cancellation rolls back an uncommitted batch; committed staging batches remain non-serving and resumable. [consistency and migration](consistency-and-migration.md) owns this protocol.

## Relational operation coverage

| Port group | Required semantics |
|---|---|
| Document/chunk readers | Scoped ordered batch lookup, keyset page, exact typed absence, bounded hydration |
| Ingestion committer | Document reservation, canonical chunk IDs, graph fact revision, embedding manifest, pending state and durable event append atomically |
| Lifecycle committer | Compare expected revision; tombstone, revoke visibility, append targeted delete intents atomically |
| Graph fact repository | Canonical entities/edges, source contributions, revisioned merge/subtract semantics, bounded rebuild scans |
| Visibility repository | Batch current-state lookup by scoped key/revision; latest authorized state for delete enforcement |
| Projection ledger | Claim/lease/renew/ack/retry/quarantine with fencing; per-target progress and binding generation |
| Workspace/identity/membership | Tenant-bound uniqueness, authorization reads, credentials/session/API-key operations through existing policy services |
| Conversations/artifacts | Existing conversation/PDF/original/MM/layout ports, checkpoint errors separated from absence |
| Job/budget persistence | Durable claim ownership, lease fencing, fairness counters and provider-slot budget; no PostgreSQL-free claim until these work |
| Migration/administration | Explicit schema apply, resumable data movement, provider health and optional diagnostics |

Map existing `TaskStorage`, conversation ports and blob ports into this bundle; do not duplicate them. SQL-specific `SKIP LOCKED`, JSONB and clock expressions belong to PG adapters. SQLite implements equivalent single-node command semantics with its own transactions and clock/lease policy, not copied SQL.

## Graph semantics

Reuse graph ISP separation but require production implementations of bounded batch read, incident-edge pagination, exact-key batch mutation, typed deletion, and scoped scans. Remove safety-critical default bodies. Deprecated full-graph APIs move to explicit export/maintenance capabilities; no request handler may depend on them.

`MergeSources` unions provenance contributions using immutable contribution IDs. `Replace` replaces the chosen revision under an expected-version precondition. Concurrent delete/prune must not erase another document's contribution. Persist per-document/chunk contributions or an equivalently auditable reverse index; aggregate descriptions and weights have one deterministic merge rule in domain policy. A capped display lineage array is not a complete deletion index.

Batch response contains applied/replayed/conflicted keys with bounded error details. No adapter may implement a typed edge delete as endpoint-pair deletion. If an adapter cannot implement a required write mode, it cannot be selected for that profile.

Traversal request contains scope, seed IDs, direction, max depth, max nodes, max edges, page bytes, deadline and continuation. Edges and their endpoints are authorized before expansion. Return truncation reason and an opaque continuation tied to query/scope/binding. Degree semantics are explicit: distinct neighboring nodes, in-edges and out-edges are separate values; self-loops contribute to both in/out counts. Do not alias these metrics across providers.

## Vector semantics

One typed `VectorSearchRequest` carries scope, full model descriptor, family, embedding, `top_k`, allowed subject IDs, document IDs, modalities, threshold, search mode, deadline and scan budget. All routing layers forward it intact. No lossy reconstruction from embedding/workspace alone.

- `None` optional business filter means unrestricted within the mandatory scope; `Some([])` means match nothing. Zero k returns empty with no I/O. Excessive k/bytes fail validation rather than allocate unbounded memory.
- This is the internal storage contract. Preserve the current public API's empty `document_ids` behavior using the admission translation in [persistence recipes](../implementation/persistence-and-provider-recipes.md#request-admission-and-api-compatibility); a filter that resolves to zero authorized documents must still become internal match-none.
- Mandatory tenant/workspace constraints execute at the provider before returning candidates. Missing or malformed scope metadata is rejected. A provider must not expose cross-tenant candidates to application ranking.
- Business predicates constrain the requested search population. ANN may underfill due to its candidate budget; it may not ignore predicates. A post-filter alone is not a claim of equivalent filtered top-k.
- `Exact` requests are rejected if unsupported; `Approximate` returns quality/budget metadata. Published recall is measured against a scoped exact oracle. “Budget exhausted” differs from “no matching data.”
- Cosine score is higher-is-better `1 - cosine_distance`, with valid range [-1, 1] subject to numeric tolerance. L2 and dot-product have tagged metrics; never apply cosine thresholds to them or mix raw scores across metrics. Fuse ranked lists through existing rank fusion where appropriate.
- Validate dimension, finite values, nonzero norm for cosine, and supported quantization. Compare exact fixtures with tolerance; approximate adapters need not return identical neighbors.
- Immutable revision upsert: same key + same digest is replay; same key + different digest is conflict. Replacement gets a new revision. Actual newly-created ownership cannot be guessed with check-then-write.
- Deletion is by scoped versioned key or indexed document/provenance association and returns confirmed effect, pending, or failure. Never use substring matching or route to default storage after a lookup failure.

The provider descriptor includes `requires_relational_colocation` and a storage-layout version. Co-located pgvector may retain typed-table FKs and optimized joins. Standalone pgvector stores canonical projection keys/filter fields without cross-authority FKs; it returns IDs/revisions for hydration through `ChunkReader`/graph fact readers. A remote adapter cannot construct a PostgreSQL chunk repository internally. Factory validation rejects a co-located layout paired with a different relational authority.

A compatibility adapter translates existing `VectorStorage` and legacy citation IDs to this contract during W5. The end state has one query/mutation contract, not permanent duplicated implementations of `VectorStorage`, `EmbeddingIndex`, and fleet logic. Sparse retrieval becomes a separate optional port; PostgreSQL cover-density rank is not declared interchangeable with BM25.

## Capabilities, limits and failures

Factories publish typed descriptors: supported metrics/dimensions, supported predicate operators, exact/approximate search, batch row/byte limits, mutation atomicity, conditional-version support, visibility receipt/barrier behavior, export cursor stability, and optional sparse/analytics support. Required capabilities are checked at startup. Runtime capability drift fails the affected operation; it does not activate an unsafe fallback.

A transport batch is not automatically atomic. Relational commands guarantee local atomic commit. Remote graph/vector batches may partially apply; return per-item outcomes when known, otherwise `UnknownOutcome` with operation identity. The delivery ledger retries idempotently and never marks the entire manifest complete from partial acknowledgment.

`AccessError` distinguishes InvalidInput, ForbiddenScope, NotFound, Conflict, UnsupportedCapability, Unavailable, DeadlineExceeded, RateLimited, SerializationRetry, UnknownOutcome and CorruptData. Keep vendor code and sanitized cause for diagnosis. SQLSTATE/status-code classification replaces constraint-name heuristics. Retry only idempotent operations or whole retriable relational transactions within a budget; timeout after commit submission is not proof of rollback. [Sources S03/S05/S06](../references/official-sources.md).

Batch reads preserve input positions and duplicate IDs using `Vec<Option<T>>`; missing rows occupy `None`. Writes dedupe by complete scoped logical key with an explicit merge/replay rule. Stable order is reconstructed in O(n) expected map work. Keyset cursors include sort tuple plus immutable tiebreaker, scope/filter fingerprint, schema version and binding generation. Authenticate external cursors; reject tampering, foreign scope and stale generation. Cross-page snapshot stability is an explicit capability, not an accidental promise.
