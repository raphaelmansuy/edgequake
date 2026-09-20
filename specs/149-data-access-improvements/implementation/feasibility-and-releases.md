# WHY feasibility requires smaller deliverables and explicit prerequisites

**Assessment: feasible with the sequencing and scope corrections below.** Existing ports, PostgreSQL adapters, graph ISP traits and deterministic provider substitutes offer a usable starting point. This is a persistence refactor with data migration, not a factory-only change. A junior team can implement the bounded steps in [implementation guide](step-by-step-guide.md), with a senior maintainer reviewing transaction, authorization and migration boundaries. No provider is certified by this assessment.

## Gaps found in the first plan and their resolutions

| Gap | Resolution and implementation consequence |
|---|---|
| W1 demanded recovery tests before the ledger/worker existed | W1 closes current contract regressions; recovery gates move to W3/W4. [work packages](work-packages.md) now states the narrower exit. |
| W3 required atomic event append but W4 owned its schema | W3 creates the minimum ledger, receipts and canonical manifests before implementing committers. W4 implements delivery and replay. |
| A leaf crate could depend back on pipeline types | Move driver-free sink DTOs/ports first; re-export them from existing locations. Never add a storage-to-pipeline dependency. |
| Existing integration helpers could make a partial profile look complete | New E2E boots production composition with persistent operational services. Only LLM/embedding generation is substituted; graph/vector/relational services are real. |
| Public and storage empty-filter semantics differ | Preserve public `document_ids: []` as unrestricted; lower-level `Some([])` stays match-none. Translate once at API admission. |
| Lease epochs alone cannot stop late provider writes | Use persisted immutable revision IDs and authoritative visibility checks; garbage collection needs an explicit late-write policy. See [persistence recipes](persistence-and-provider-recipes.md). |
| SQLite was one oversized work package | Separate operational persistence inventory, PostgreSQL port extraction, SQLite adapters and feature closure. Deliver P3 only after all four pass. |
| “Certified or unavailable” could declare the entire project done without substitution | Interim releases may disable future profiles. Full PROVIDER-ACCESS completion requires every composition listed below and [definition of done](../validation/definition-of-done.md). |

## Releases, dependencies and scope

These are delivery boundaries, not calendar promises. Work identifiers stay W0-W9; numbered implementation steps are J01-J24. “P2a/P2b/P4” below are certification aliases for the combinations already required by [architecture](../design/architecture.md).

| Release | Required compositions | Completed steps | User-visible result |
|---|---|---|---|
| R0 safety baseline | P0 = PostgreSQL / AGE / co-located pgvector | J01-J05 | Current provider filters and error semantics are enforceable; no new providers advertised. |
| R1 durable boundary | P0 | J06-J16 | Independent composition, real commits, replay, bounded reads and standalone pgvector adapter contract. |
| R2 vector substitution | P0, P1 = PostgreSQL / AGE / Qdrant | J17-J18 | Vector provider substitution with a demonstrated rollback. |
| R3 graph substitution | P0, P1, P2a = PostgreSQL / Neo4j / pgvector, P2b = PostgreSQL / Neo4j / Qdrant | J19-J20 | Graph and vector axes vary independently. |
| R4 relational substitution | All above, P3 = SQLite / Neo4j / Qdrant, P4 = SQLite / Neo4j / standalone pgvector | J21-J24 | Relational substitution; P3 has no PostgreSQL service or driver feature. P4 intentionally retains PostgreSQL for vectors. |

```ascii
R0 current-provider safety
             |
             v
R1 contracts -> schema -> commits -> delivery -> bounded reads
                                                   |
                                                   v
R2 Qdrant -> R3 Neo4j -> R4 operational ports + SQLite
     |           |                   |
     +-----------+-------------------+
                 |
      accumulated profile regression suite
```

All releases require regression tests for previously certified compositions. Prototype work may run independently, but no binding cutover precedes the durable boundary. SQLite on a network filesystem, multi-server SQLite, Neo4j clusters and Qdrant clusters are outside the initial single-node certification; profile declarations must say so. Existing supported P0 topology must be regression-tested as deployed, not silently narrowed to a single test process.

## Required design reviews before junior implementation

These are repository review gates, not requests for permission in this documentation task. Record the result in the implementing PR; reject a design that cannot demonstrate the listed experiment.

| Gate | Before | Reviewer verifies | Evidence required |
|---|---|---|---|
| G1 contract/dependency | J06 | Types can move without cycles; API filter compatibility; no provider imports in leaf | Compile a `dyn` port consumer and inspect `cargo tree`; contract truth table tests |
| G2 transaction/schema | J08-J10 | Authority policy D4, locked aggregate order, canonical IDs, same-transaction ledger, replay retention | Concurrent same-key commits and rollback fault demonstrate a single durable receipt or no effects |
| G3 visibility/fencing | J11-J12 | Delayed old writes, shared contribution readiness, graph anchors cannot bridge hidden entities, cleanup completion semantics | Deterministic paused-worker experiment across update and deletion |
| G4 migration | J18/J20/J24 | Snapshot/capture gap closed; restore retains payloads; old binary compatibility explicitly bounded | Interrupted backfill, delete during replay, switch and reverse-switch drill |
| G5 SQLite closure | J21 | Every required operational store has an implementation; feature graph truly excludes PG for P3 | Completed inventory, isolated auth/upload/query/delete/restart test, `cargo tree` artifact |

If a spike fails, keep the affected profile unavailable and fix the prerequisite. Do not let a junior developer invent weaker isolation or silently substitute memory services to pass the gate.

## Sizing and principal risks

J01-J24 are reviewable outcomes; the larger steps explicitly split into child PRs in [implementation guide](step-by-step-guide.md). Target one behavioral change per PR and split any estimated implementation exceeding three engineering days. The three-day rule is a planning heuristic, not an estimate that all steps take three days. Inventory J21 determines the largest unknown: operational PostgreSQL coupling. Set dates only after J02 and G2/G5; allow time for real-provider CI and seven-day rollback soak.

The critical path is canonical authority and replay correctness. Adding a transport client is comparatively small. Risks include legacy graph/spine divergence, incomplete source lineage, feature coupling, stale projection amplification and underestimated relational-service coverage. Mitigations are quarantined migration census, full contribution storage, explicit feature checks, candidate budgets and an exhaustive required-port inventory. A database upgrade is not bundled into this refactor; server pins are qualified independently using [official sources](../references/official-sources.md).

## Grounding for the feasibility corrections

The existing [text ingestion test](../../../edgequake/crates/edgequake-api/tests/e2e_spec024_text_upload_async.rs) establishes HTTP 202 and track polling. [QueryRequest and DocumentFilter](../../../edgequake/crates/edgequake-api/src/handlers/query_types.rs) establish public empty-list compatibility. [Routes](../../../edgequake/crates/edgequake-api/src/routes.rs) establish the endpoints in [E2E specification](../validation/e2e-test-specification.md).

[PostgreSQL deletion tests](../../../edgequake/crates/edgequake-api/tests/e2e_document_deletion_postgres.rs) manually mix PostgreSQL data stores with memory workspace/conversation/task services; they remain useful adapter integration tests, not full-profile certification. [Common worker helpers](../../../edgequake/crates/edgequake-api/tests/common/mod.rs) likewise need a dedicated production-composition harness.

[API Cargo features](../../../edgequake/crates/edgequake-api/Cargo.toml) enable PostgreSQL on the tasks dependency, and the [workspace/root package](../../../edgequake/Cargo.toml) has direct SQLx coupling. Consequently `--no-default-features` by itself does not prove a PostgreSQL-free P3 binary. Preserve all existing tests while adding stronger evidence.
