# WHY each implementation step needs a reproducible exit

Follow the order below. Every step identifies an existing entry point or a **proposed** path, the changes to make, and an observable exit. Proposed files, commands and tests do not exist merely because they appear here. [work packages](work-packages.md) owns W0-W9 scope, [persistence recipes](persistence-and-provider-recipes.md) owns persistence recipes, [E2E specification](../validation/e2e-test-specification.md) owns E2E scenarios, and [definition of done](../validation/definition-of-done.md) owns completion gates.

## Working agreement

1. Read the referenced contract and current callers before changing a trait. Add a failing behavioral regression first for a defect; do not test source strings instead of behavior.
2. Keep new modules focused: contracts, adapter query compilation, command execution and policy are separate. Re-export moved types during migration so one move does not require unrelated caller rewrites.
3. Use the existing async runtime, SQLx 0.8.6 and HTTP client where suitable. Pin any new provider version and record its compatibility experiment. No drive-by dependency upgrades.
4. Execute targeted tests after each child PR. Record exact commands and results, including skips. Run full required CI before release. Never edit old migration checksums.
5. Commit no secrets or generated database volumes. Provision only uniquely owned scratch databases/collections. A failed test retains sanitized diagnostic artifacts and exits nonzero.

## J01 — Establish a strict baseline harness (W0 / R0)

**Read:** [storage contracts](../../../edgequake/crates/edgequake-storage/tests/storage_backend_contract.rs), [PG test configuration](../../../edgequake/crates/edgequake-storage/tests/support/postgres_test_config.rs), [API test helpers](../../../edgequake/crates/edgequake-api/tests/common/mod.rs).

**Build:** proposed `scripts/provider-access/test-profile.sh`, `scripts/provider-access/compose.yaml`, `edgequake/crates/edgequake-api/tests/support/provider_access/`, and Make targets in [E2E specification](../validation/e2e-test-specification.md). First child PR provisions isolated P0 with pinned image digests, fresh migrations and health polling. Second starts the production server/workers and deterministic LLM/embedding endpoints. Third emits the run manifest and rejects missing services/zero tests/skips. Initially use current bootstrap; adopt the factory in J07.

**Exit:** `PROVIDER-ACCESS-E2E01` records a real P0 baseline; deliberately stop its DB and confirm certification fails. Existing memory contract tests still pass. Never fall back to `/tmp/edgequake-db-url`, the development database or memory persistence.

## J02 — Inventory coupling and freeze baseline workloads (W0 / R0)

Run `rg -n 'sqlx::|PgPool|PgConnection|SIDECAR_POOL|register_.*pool' edgequake/crates edgequake/src` from repository root. Review matches rather than counting comments as dependencies. Create proposed `scripts/provider-access/required-ports.json`: each service has owning crate, existing trait, production constructor, PG implementation, SQLite status, scope rules, transaction participation and test ID. Include all groups in [access contracts](../design/contracts.md#relational-operation-coverage).

Add an architecture allowlist with a reason and removal step for every temporary policy-layer SQL dependency. Record physical request counts, workload digest and p95 baseline per [algorithms and budgets](../design/algorithms-and-budgets.md). **Exit:** no required service marked “unknown”; capture auth/tasks/budgets/artifacts as well as documents. G5 later checks this inventory, rather than rediscovering it.

## J03 — Fix filtered search forwarding (W1 / R0)

Start in [typed_read.rs](../../../edgequake/crates/edgequake-storage/src/adapters/postgres/vector/typed_read.rs) and [storage_impl.rs](../../../edgequake/crates/edgequake-storage/src/adapters/postgres/vector/storage_impl.rs). Reproduce the typed early-return path with every filter and non-ready rows. Pass the full typed filter into SQL before LIMIT, enforce scope and serving predicates, and propagate database failure. Translate the public empty document list as specified in [persistence recipes](persistence-and-provider-recipes.md#request-admission-and-api-compatibility).

**Exit:** T01/T13 and E2E02/E2E03 pass on P0; a provider outage is an error, not an empty successful answer. This step does not introduce projection recovery.

## J04 — Remove successful unsafe trait defaults (W1 / R0)

In existing vector/graph read/mutate traits, require safety-critical methods and update each implementer/test double explicitly. Preserve existing correct typed-edge overrides. Add separate tests for Replace, MergeSources, typed sibling edges, self-loops and empty batches. Optional operations return `UnsupportedCapability`.

**Exit:** T02/T07/T08/T12 contract cases applicable to current adapters pass; compilation locates every missing implementation. Do not require future revision-fencing cases yet.

## J05 — Normalize errors and HTTP failure mapping (W1 / R0)

Update [storage error translation](../../../edgequake/crates/edgequake-storage/src/error.rs) and API error mapping. Classify PG errors by SQLSTATE, then constraint identity when a named constraint is relevant; preserve safe source diagnostics internally. Implement the mapping table in [persistence recipes](persistence-and-provider-recipes.md#request-admission-and-api-compatibility) behind the existing response envelope.

**Exit:** uniqueness, deadline, unavailable provider, invalid vector and unsupported capability tests pass; public responses expose no SQL, credentials or connection URLs. R0 gate is now eligible.

## J06 — Extract driver-free contracts without cycles (W2 / R1)

Add proposed `edgequake/crates/edgequake-storage-contracts/{Cargo.toml,src/lib.rs}` with focused `scope`, `ids`, `error`, `relational`, `graph`, `vector`, `projection` modules. Move one type/port family per child PR. Inventory `RelationalEntitySink` and its DTO dependencies in pipeline; move pure definitions into the leaf before storage implements them. Existing locations re-export the identical type, not duplicate wrapper definitions.

**Exit:** G1 passes; leaf compiles without vendor features; a compile test constructs `Arc<dyn ScopedVectorSearch>` and `Arc<dyn IngestionCommitter>`. Existing public imports remain valid. Do not make the leaf depend on API/core/pipeline/tasks/storage.

## J07 — Wire independent composition (W2 / R1)

Add proposed `edgequake-api/src/state/data_access_config.rs` and `data_access_factory.rs`; update the existing PG state constructors and CLI bootstrap listed in [work packages](work-packages.md). Parse tagged configurations strictly, apply old-config P0 defaults, probe versions/schema/capabilities read-only, create narrow bundles, then start workers. Inject owned clients; failure cleans partially built runtimes. Update J01 to invoke this same bootstrap.

**Exit:** E2E01/E2E12; legacy P0 config works, unknown/disabled provider fails before readiness, absent index fails with an actionable migration requirement, startup executes no DDL. Reserve future providers as unavailable rather than falling back.

## J08 — Add the minimum durable schema (W3 / R1)

After G2 schema review, allocate fresh migration IDs by inspecting the current migration registry. Implement the tables/constraints/indexes in [persistence recipes](persistence-and-provider-recipes.md#minimum-schema-before-any-new-writer), including receipts, facts/contributions, bindings, events, deliveries and manifests. Split into schema and schema-test PRs if needed. Existing milestone events keep their consumer.

**Exit:** fresh install and upgrade-from-P0 fixtures both pass; scoped foreign keys reject cross-scope relationships; delivery uniqueness and claim indexes verified; malformed/null revisions fail. Old binary compatibility is tested only for the stated pre-authority-change window. No worker or new writer is enabled yet.

## J09 — Implement one transactional ingestion command (W3 / R1)

Replace the label-only `UnitOfWork` for document/chunk writes with adapter-private SQLx transactions. Start with one bounded document/chunk batch, then add graph facts and persisted embedding payloads in a second child PR. Follow receipt lookup, lock order, canonical-ID readback and event append in [persistence recipes](persistence-and-provider-recipes.md#transaction-and-idempotency-recipe). Repository helpers accept the real connection executor, never silently use the pool.

**Exit:** T03/T08; injected failure before event append leaves no batch rows/receipt/events; 20 concurrent identical requests return one receipt; changed body with same key conflicts; unknown commit outcome resolves by the same key. No network/LLM work occurs inside the transaction.

## J10 — Move sidecar and lifecycle writes behind ports (W3 / R1)

Refactor sidecar/stage-mirror services and PG entity sink listed in W3. Replace global registration and `Box::leak` with injected handles. Add bounded staging/finalization and tombstone commands using the J09 executor. A delete records all target bindings before the workspace or original metadata can disappear. Move workspace document operations first; leave remaining operational services explicitly assigned to J21.

**Exit:** E2E04 rollback section and E2E10; two runtimes in one process never share clients/state. Missing row differs from read error. Delete admission atomically revokes authority visibility even while projections are offline.

## J11 — Implement leased delivery and immutable writes (W4 / R1)

Use proposed `edgequake-storage/src/projection/{ledger,worker,payload}.rs` or equivalently small existing modules. First child PR adds claim/renew/ack/quarantine against the real relational store. Second routes PG graph/vector writes through immutable physical revisions. Third persists visibility receipts and conditional publication. Reuse shared retry/batch policy; do not let each adapter add another retry loop.

**Exit:** G3 and E2E04/E2E05/E2E07; lost lease causes zero-row ack, unknown event version quarantines, graph/vector delivery progress is independent, restart needs no generation calls. Claim transaction finishes before any provider call.

## J12 — Enforce authoritative visibility and shared lineage (W4 / R1)

Add batched visibility/hydration at the retrieval boundary and before every graph frontier expansion. Apply the exact shared-contribution policy in [persistence recipes](persistence-and-provider-recipes.md#publication-shared-facts-and-stale-cleanup). Add targeted cleanup reconciler, periodic obsolete-revision sweeps, quarantine inspection and lag diagnostics.

**Exit:** E2E05/E2E06/E2E07; a deleted document cannot remain a graph bridge or appear in an aggregate description. Shared surviving facts return after regeneration; stale deliveries cannot republish. Physical-cleanup completion is not claimed while old writes may still arrive.

## J13 — Unify vector requests and identity (W5 / R1)

Adapt FleetEmbeddingIndex, EmbeddingIndex, workspace registry and document-vector resolver through one full request DTO. Keep legacy ID hydration in one compatibility module. All four families (chunk/entity/relationship/report) carry authoritative model revision and binding. Remove deletion fallback to the default store. Add pure validation, byte-aware splitting and typed score conversion tests.

**Exit:** T01/T07/T10/T13; E2E03/E2E06/E2E11. An explicit empty internal filter performs no search, and equal dimensions never select another model.

## J14 — Add standalone pgvector layout (W5 / R1)

Add migration-owned adapter tables keyed by scope/family/subject/model/content revision with persisted physical ID, dimension-specific index definitions and filter payload. No FK/join to a remote relational authority. Use injected visibility/hydration ports; declare `requires_relational_colocation=false` only for this layout. Co-located layout remains separate and tested.

**Exit:** standalone adapter contract with an isolated relational authority fixture passes; EXPLAIN shows index-compatible scoped search. Full SQLite composition P4 waits until J23. Do not silently switch an existing collection/table layout.

## J15 — Bound graph reads and mutations (W6 / R1)

Replace production inherited full scans/per-node RPCs with scoped incident-edge pages, frontier batches and exact-key mutations. Reuse graph dedupe helpers. Track unique nodes and edge identities separately; limits apply before enqueue and serialization. Validate current authority before expanding any neighbor. Keep exports behind paged maintenance ports.

**Exit:** T02/T05/T12, E2E02/E2E06/E2E09; dense/cyclic graph stays within depth/node/edge/byte limits and reports why it stopped. Cold and stale-revision-heavy fixtures stay bounded.

## J16 — Add algorithm and operational gates (W6 / R1)

Optimize memory exact top-k with partial selection plus sorted k or bounded heap. Instrument physical calls, serialized bytes, retries and candidate rejection; cap queues/concurrency. Add architecture import checks using the J02 reviewed allowlist.

**Exit:** T15 and E2E09/E2E12; measured call counts fit declared batch formulas; pending backlog does not allocate unbounded memory. R1 requires recovery E2E gates, not only unit tests.

## J17 — Build the Qdrant adapter (W7 / R2)

Split into provisioning/health, mutation, filtered search and error translation child PRs. Use existing HTTP client with typed DTOs unless a qualified client has a demonstrated benefit. Apply the concrete recipe in [persistence recipes](persistence-and-provider-recipes.md#provider-recipes); validate body-level completion and full payload predicates. Pin server image/transport contract at this step.

**Exit:** all shared vector contracts plus E2E01-E2E07/E2E11/E2E12 on P1. Use real Qdrant, including delayed response and unavailable service cases. No hot-path collection creation.

## J18 — Certify vector migration and rollback (W7 / R2)

Implement resumable P0-to-P1 backfill/capture/shadow/switch using binding generations, existing migration jobs and durable cursors. Test tombstones and model revisions while copying. Complete G4 and retain the old binding for the defined soak.

**Exit:** E2E08/E2E13 and applicable performance suite pass on P0/P1; the rollback script refuses an uncaught-up target. R2 may ship; P2/P3 remain explicitly unavailable.

## J19 — Build the Neo4j adapter (W8 / R3)

Initial transport decision: **single-node Neo4j Query API over existing HTTP client**, pending version qualification. Split provisioning/constraints, revisioned mutations, bounded traversal and body-error mapping into separate PRs. Use application UUIDs and the edge-as-node mapping in [persistence recipes](persistence-and-provider-recipes.md#provider-recipes). Preserve portable graph semantics; do not copy AGE SQL/agtype parsing.

**Exit:** all shared graph contracts and E2E02/E2E05/E2E06/E2E09 pass on P2a and P2b. HTTP success containing Neo4j errors fails delivery. Cluster support requires separate affinity/bookmark certification and is not implied.

## J20 — Certify graph migration and independent combinations (W8 / R3)

Replay complete canonical facts into Neo4j; compare scoped identity/contributions and bounded traversal. Demonstrate rollback to a caught-up AGE binding. Test Neo4j with both Qdrant and co-located pgvector; no shared client assumption between axes.

**Exit:** E2E01-E2E13 applicable scenarios pass for P2a/P2b, previous P0/P1 remain green, G4 closes. R3 may ship.

## J21 — Complete operational relational port extraction (W9 / R4)

Reconcile J02 inventory against current code before starting. Create separate child PRs for (a) identity/session/membership/API keys, (b) workspaces/conversations/audit, (c) tasks/leases/provider budgets, (d) checkpoints/artifacts/original/PDF/MM/migration jobs. Reuse existing traits; keep policy in services and move SQL into PG adapters. Each child PR must preserve current P0 behavior before adding SQLite.

**Exit:** G5 inventory has an injectable port for every required service; no unreviewed driver dependency remains in policy. E2E10/E2E14/E2E15 run against production persistent stores. This step can be many PRs and is not estimated as one junior ticket.

## J22 — Implement SQLite persistence and migrations (W9 / R4)

Implement one inventory group per PR; compare it against PG using the same contract fixture. Add SQLite-specific migrations, FKs on each connection, WAL/local-file deployment, finite busy timeout and short single-writer transactions. Follow lease/idempotency/rollback rules in [persistence recipes](persistence-and-provider-recipes.md#provider-recipes); never translate PG SQL by string replacement.

**Exit:** T01-T20 applicable relational contracts; restart retains jobs, revocations, manifests and artifacts. Concurrent writers either complete within bounds or return a retryable error; no partial commit. SQLite admission rejects multiple server replicas/network-file topology.

## J23 — Remove feature coupling and qualify P3/P4 (W9 / R4)

Make root SQLx use and API/tasks PG features conditional; split PG-only bootstrap/diagnostics. Record `cargo tree -e features` for P3 and assert no `sqlx-postgres` dependency. Do not just invoke `--no-default-features`. Compile exact feature sets from the runner manifest and boot P3 with no PostgreSQL address/service reachable. Boot P4 separately with a PostgreSQL vector service and standalone layout.

**Exit:** E2E01-E2E15 applicable scenarios pass on P3/P4; missing required operational port fails boot. P4 diagnostics distinguish the SQLite authority from the PG vector client.

## J24 — Rehearse data migration, restore and remove bridges (W9 / R4)

Use explicit export/import at a capture watermark for the first PG-to-SQLite relational migration; fence writes for final reconciliation and switch one authority, never dual-master. Retain PG read-only until rollback reconciliation is proven. Remove old UnitOfWork/global pool/facade call sites and temporary import exceptions only after P0-P4 coverage proves replacement. Keep compatibility event readers for the retained replay window.

**Exit:** E2E08/E2E13/E2E14/E2E15, G4/G5 and the entire [definition of done](../validation/definition-of-done.md) pass. A retained old schema alone is not a runnable rollback target; demonstrate receipt/tombstone reconciliation or block reversal.

## Commands per PR and stopping rules

From `edgequake/`, run changed-crate unit/contract tests, `cargo fmt --check` and targeted `cargo clippy -p <changed-crate> --all-targets --features <profile-features> -- -D warnings`. The profile runner owns exact feature sets; angle-bracket text here is a placeholder for that checked-in manifest, not a literal shell command. From repository root, run the documentation validator and the appropriate proposed Make profile target from [E2E specification](../validation/e2e-test-specification.md).

Stop the affected PR when a contract cannot be met, a migration loses provenance, an unknown outcome cannot be reconciled or a required profile fixture is unavailable. File the failing test/evidence and repair the prerequisite. Never redefine success as an empty result, skip, no-op mutation or memory fallback.
