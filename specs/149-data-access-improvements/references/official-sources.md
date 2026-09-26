# WHY provider claims need current primary sources

Sources below were opened and reviewed on **2026-09-20**, including relevant linked pages on consistency, transaction handling, indexing and releases. Repository behavior is evidenced separately in [current-state audit](../design/current-state-audit.md). Provider recommendations and the proposed architecture are engineering conclusions, not claims endorsed by upstream maintainers.

No callable tool named `fetch_webpage` was available. Research used the available web search/open tools; opening the requested Google-search URL failed, so official pages were located through web search and opened directly. No packages were installed. “Current”/“latest” URLs are moving references; implementation must pin server images, clients and compatible feature floors in W0/W7–W9.

## S01 Rust dynamic traits

[The Rust Reference — dyn compatibility](https://doc.rust-lang.org/reference/items/traits.html#dyn-compatibility) describes restrictions on dynamically dispatched methods, including native async methods and opaque return types. **Implication:** preserve a dyn-compatible async strategy for runtime provider handles; do not mechanically remove `async_trait`. Applies to [access contracts](../design/contracts.md).

## S02 async-trait

[async-trait crate documentation](https://docs.rs/async-trait/latest/async_trait/) documents the boxed future transformation used to support asynchronous trait objects. **Implication:** the repository's existing approach is a viable compatibility bridge; allocation cost should be measured if it matters. This is a mechanism choice, not a performance guarantee.

## S03 SQLx transactions

[SQLx 0.8.6 Transaction](https://docs.rs/sqlx/0.8.6/sqlx/struct.Transaction.html) is the version in the inspected lockfile; [latest documentation](https://docs.rs/sqlx/latest/sqlx/struct.Transaction.html) was checked as a separate moving reference. Transactions expose commit/rollback and rollback on drop when unfinished. **Implication:** carry the actual transaction executor inside a domain committer; a label-only handle cannot provide atomicity. Do not infer that a connection loss during commit proves rollback. Applies to [consistency and migration](../design/consistency-and-migration.md).

## S04 PostgreSQL isolation

[PostgreSQL current transaction isolation](https://www.postgresql.org/docs/current/transaction-iso.html), resolving to PostgreSQL 18 during this audit, explains statement snapshots under Read Committed and retry requirements for stronger isolation failures. **Implication:** use explicit transaction boundaries and whole-command retries with idempotency keys; a shared pool is not a shared transaction. The chosen isolation/locking policy must protect expected revisions and quota/lease invariants.

## S05 PostgreSQL row security

[PostgreSQL row-security policies](https://www.postgresql.org/docs/current/ddl-rowsecurity.html) documents default-deny policy behavior and bypass by superusers, BYPASSRLS roles and normally table owners. **Implication:** scope contracts and adapter predicates remain necessary; exercise the actual application role in tests. RLS is defense in depth, not a portable graph/vector authorization mechanism. Applies to [access contracts](../design/contracts.md) and T01.

## S06 PostgreSQL error codes

[PostgreSQL SQLSTATE appendix](https://www.postgresql.org/docs/current/errcodes-appendix.html) advises checking codes rather than message text. Examples relevant here include unique violation 23505, serialization failure 40001 and deadlock 40P01. **Implication:** preserve classification and retry/unknown-outcome semantics across adapters instead of inferring constraint type from its name. Applies to F13 and W1.

## S07 pgvector filtered search

[pgvector official README — filtering and iterative scans](https://github.com/pgvector/pgvector#filtering) explains filtered approximate-search underfill, iterative scans introduced in 0.8.0, and scan limits. [Index troubleshooting](https://github.com/pgvector/pgvector#troubleshooting) explains distance-operator ordering and index eligibility. **Implication:** predicates and typed dimension expressions must survive routing; benchmark selective filters and expose bounded underfill. Iterative scans do not promise perfect recall or a universal logarithmic bound. Repository [088](../../088-data-layer/README.md) records a 0.8.5 deployment pin; this audit does not verify the running server. Applies to [algorithms and budgets](../design/algorithms-and-budgets.md).

## S08 Qdrant consistency

[Qdrant consistency guarantees](https://qdrant.tech/documentation/scaling/consistency-guarantees/), reached from [distributed deployment](https://qdrant.tech/documentation/scaling/distributed_deployment/), separates write consistency, read consistency and write ordering. Stronger ordering can reduce availability when its leader is unavailable. **Implication:** describe the selected policy in a provider manifest; it does not create a transaction with PostgreSQL or fence all delayed cross-store effects. Immutable revisions and authority validation remain application requirements.

## S09 Qdrant points and payload indexes

[Qdrant points](https://qdrant.tech/documentation/manage-data/points/) documents point identity and idempotent repeated point loading; re-uploading an ID overwrites that point. [Payload indexing](https://qdrant.tech/documentation/manage-data/payload/#payload-indexing) describes field indexes for filtered queries. **Implication:** use stable scoped revision IDs and required scope/document/model payload indexes. Basic idempotence does not imply stale-revision conflict detection. Qualify server/client versions before relying on newly added update modes. Applies to W7.

## S10 SQLite isolation

[SQLite isolation](https://www.sqlite.org/isolation.html) documents serialized writes, a single writer at a time and WAL read behavior. **Implication:** SQLite is a concrete second relational implementation for a bounded single-node profile, not a transparent replacement for PostgreSQL multi-replica queue concurrency. Enforce the topology restriction and test restart/lease behavior. Applies to W9.

## S11 Neo4j database transactions

[Neo4j transaction management](https://neo4j.com/docs/operations-manual/current/database-internals/transaction-management/) documents transaction scope and warns that large transactions retain modifications in memory. **Implication:** batch by bytes as well as records, set deadlines, and qualify read-your-write behavior through the selected transport. Neo4j-local ACID does not cover a separate relational/vector engine. Applies to W8 and [algorithms and budgets](../design/algorithms-and-budgets.md).

## S12 Neo4j Query API

[Neo4j Query API transactions](https://neo4j.com/docs/query-api/current/transactions/) documents transaction affinity, response-body errors despite HTTP acceptance, and the limited stability of `elementId` beyond a transaction. **Implication:** use application-owned UUIDs; preserve transaction routing and inspect body-level errors if this transport is selected. A community Rust Bolt client is not assumed to be an official supported driver. Implementation must document the chosen transport and compatibility matrix.

## S13 Apache AGE setup and release conflict

[Apache AGE setup](https://age.apache.org/age-manual/master/intro/setup.html) documents per-connection AGE setup and search path. Its listed PostgreSQL compatibility range is older than the [official release page](https://github.com/apache/age/releases), which lists a PG18 1.8.0 release entry with tag `PG18/v1.8.0-rc0` at review time. **Implication:** do not infer compatibility or stable release status from the manual label alone; pin the exact build/tag and validate catalog/session behavior. Keep AGE setup and agtype handling inside its adapter. Applies to W2/W6.

## S14 PostgreSQL pagination and index shape

[PostgreSQL LIMIT/OFFSET](https://www.postgresql.org/docs/current/queries-limit.html) explains that skipped rows still require work and ordering must be predictable. [Multicolumn indexes](https://www.postgresql.org/docs/current/indexes-multicolumn.html), followed from pgvector guidance, explains how column constraints affect index scans. **Implication:** use indexed keyset tuples with immutable tiebreakers and verify plans for actual scoped filters. Keyset pagination alone does not create a stable cross-request snapshot. Applies to [algorithms and budgets](../design/algorithms-and-budgets.md) and T11.

## S15 PostgreSQL queue claim locking

[PostgreSQL SELECT locking clauses](https://www.postgresql.org/docs/current/sql-select.html#SQL-FOR-UPDATE-SHARE) explains that SKIP LOCKED is useful for queue consumers but does not provide a general consistent view. **Implication:** the bounded claim SQL in [persistence recipes](../implementation/persistence-and-provider-recipes.md#worker-claim-and-acknowledgment-recipe) uses it only to allocate work, then commits before provider I/O. Lease/epoch checks still govern completion.

## S16 SQLite write transactions

[SQLite transaction documentation](https://www.sqlite.org/lang_transaction.html) distinguishes deferred/immediate transactions, immediate-write contention and COMMIT busy behavior. A busy COMMIT can leave the transaction active. **Implication:** use a real owned transaction and verify cleanup before retrying; do not return a connection with an unresolved transaction to a pool. J22 tests both contention and restart outcomes.

## S17 Qdrant completion options

[Official Qdrant upsert API](https://api.qdrant.tech/api-reference/points/upsert-points) documents overwrite-by-ID, the wait option, ordering choices and operation results. **Implication:** use persisted immutable revision IDs, request completion, inspect the response and qualify visibility. HTTP acceptance alone is not the projection publication condition. Cluster ordering/consistency settings require their own certification rather than borrowing single-node results.

## S18 SQLx owned custom begin and SQLite connection settings

[SQLx 0.8.6 Pool::begin_with](https://docs.rs/sqlx/0.8.6/sqlx/struct.Pool.html#method.begin_with) returns an owned transaction after executing the supplied begin statement. [SQLite foreign-key PRAGMA](https://www.sqlite.org/pragma.html#pragma_foreign_keys) documents connection enforcement and the restriction on changing it during a transaction. **Implication:** begin SQLite writes through `begin_with("BEGIN IMMEDIATE")`, configure connection settings on creation and test error/drop cleanup. No custom transaction guard or SQLx upgrade is needed merely to choose immediate transactions.

## S19 Qdrant filter semantics

[Qdrant filtering documentation](https://qdrant.tech/documentation/search/filtering/) defines structured field predicates and OR membership via Match Any. **Implication:** compile the validated typed predicate tree with mandatory tenant/workspace clauses; do not treat the public API's empty document list as a vendor-specific empty filter. Test truth tables and malformed payloads against the pinned server. Array membership does not prove that every contributor to a shared aggregate is authorized; authoritative contribution validation remains necessary.

## Applicability and unresolved implementation choices

Official documentation establishes available mechanisms, not this product's throughput or deployment support. P0 service versions require runtime probes; Qdrant/Neo4j/SQLite profiles remain unimplemented. The feasibility revision chooses existing-client HTTP transports for initial Qdrant/Neo4j and pinned SQLx for SQLite; server pins and benchmark calibration are recorded in their work packages before release. No new dependency/version claim in this plan is a substitute for a tested compatibility manifest. S15-S19 were additionally reviewed during the implementation-readiness assessment on 2026-09-20.
