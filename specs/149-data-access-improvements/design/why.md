# WHY: provider choice must preserve product behavior

An operator should be able to use one relational engine for durable lifecycle data, another graph engine for traversal, and another vector engine for similarity search. A developer should implement a provider against a conformance suite without editing ingestion, query, authentication, and deletion logic for that vendor.

Today, adding a vector adapter can appear successful while filters are ignored; moving relational storage requires untangling SQL in services; separating stores exposes partial-write and replay gaps. The goal is **safe substitution**, not a larger catalog of adapter names. Evidence: [F01–F14](current-state-audit.md).

## First-principles derivation

1. A document has one identity, scope, lifecycle, and revision. These cannot depend on which database is selected.
2. Relational access protects constraints and atomic domain changes. Graph access answers adjacency and topology questions. Vector access ranks numeric representations. These workloads need different operations and performance contracts.
3. A remote graph/vector service cannot participate in a local SQL transaction merely because Rust wraps both in a trait. Durable intent, replay, and visibility gating are necessary when stores are separate.
4. A successful write must describe what became durable. An empty result must mean no matching data, not an unavailable dependency.
5. Reading or writing n records requires at least proportional work in their bytes. Batching reduces network calls; it does not make data processing constant-time.
6. Abstractions are trustworthy only when substitutions run the same behavioral tests. Memory tests alone cannot establish SQL, transport, locking, or replication semantics.

```ascii
Provider choice
      |
      v
Stable meaning: identity + scope + revision + outcome
      |
      v
Narrow domain ports + shared conformance tests
      |
      +--> Relational: atomic state and durable intent
      +--> Graph: bounded traversal and typed edges
      +--> Vector: scoped ranking and model identity
```

## Required outcomes

| ID | Invariant | Enforcement owner |
|---|---|---|
| PROVIDER-ACCESS-I1 | Every ordinary operation has validated tenant/workspace scope; missing scope fails closed | Scope constructors, ports, adapter predicates |
| PROVIDER-ACCESS-I2 | Success, absence, pending visibility, unsupported behavior, and failure are distinct | Result/error contracts |
| PROVIDER-ACCESS-I3 | A committed mutation and its required delivery intents are atomic in the relational authority | Domain committers |
| PROVIDER-ACCESS-I4 | Only committed, authorized, non-deleted revisions may be served; stale workers cannot resurrect data | Visibility repository + projection revisions |
| PROVIDER-ACCESS-I5 | Graph identity includes relationship type and direction; vector identity includes model revision | Typed keys + shared normalization |
| PROVIDER-ACCESS-I6 | Hot-path work is bounded by requested inputs/outputs; batch limits include bytes | Budgets + conformance instrumentation |
| PROVIDER-ACCESS-I7 | Application policy has one owner; vendor translation remains adapter-local | Dependency rules |
| PROVIDER-ACCESS-I8 | Provider selection is independent, explicit, validated, and observable | Composition root + provider manifests |

These invariants are specified by [contracts](contracts.md), [consistency](consistency-and-migration.md), and [complexity budgets](algorithms-and-budgets.md); [validation](../validation/edge-cases-and-contract-tests.md) makes them falsifiable.

## Design decisions

| ID | Decision | Reason and consequence |
|---|---|---|
| PROVIDER-ACCESS-D1 | Extend existing domain/graph ports; migrate callers and retire compatibility surfaces | Preserve prior investment; prevent permanent duplicate APIs |
| PROVIDER-ACCESS-D2 | Configure relational, graph, and vector independently | `StorageMode` and `VectorBackend` currently represent different, insufficient concepts |
| PROVIDER-ACCESS-D3 | Use narrow atomic domain committers with adapter-private transactions | Replace the placeholder `UnitOfWork` without exposing SQLx or pretending to offer distributed ACID |
| PROVIDER-ACCESS-D4 | For split-provider mode, relational state records canonical versioned graph facts and embedding manifests; graph remains traversal authority | Makes replay and replacement possible without depending on whichever graph is currently online |
| PROVIDER-ACCESS-D5 | Require safety-critical operations; return typed unsupported errors for optional capabilities | No ignored scope/filter/write mode; no successful no-op deletion |
| PROVIDER-ACCESS-D6 | Use at-least-once delivery with immutable revisions and idempotent effects | Handle crash windows and unknown remote outcomes without claiming exactly-once delivery |
| PROVIDER-ACCESS-D7 | Keep PostgreSQL default; qualify alternative providers by deployment profile | SQLite is initially single-node; AGE still requires PostgreSQL even if relational authority changes |
| PROVIDER-ACCESS-D8 | Use measurement-backed complexity and latency gates | ANN recall and planner behavior cannot be guaranteed by Big-O labels |

**D4 is an explicit proposed change to prior authority policy.** [SPEC-091 LD-04](../../091-simplify-data-layer/README.md) calls relational graph models optional projections. PROVIDER-ACCESS makes complete durable graph facts mandatory in split-provider mode after reconciliation; it does not replace graph traversal with recursive SQL. Do not enable that mode until W4 proves complete replay, including provenance, descriptions, and tombstones. Existing single-stack authority stays unchanged before that gate.

## Scope and non-goals

Scope includes documents/chunks, graph facts and provenance, vector families (chunk/entity/relationship/community report), workspace/identity/conversation data, checkpoints/artifacts, scheduling and provider-budget persistence, migrations, and diagnostics. Existing specialized blob/PDF ports remain separate and may be implemented by the relational provider initially.

This work does not introduce a universal SQL/Cypher query language, an ORM rewrite, a cross-vendor transaction coordinator, or a production memory fallback. A vector provider is not an embedding-generation provider. The CLI's current `DATABASE_URL` requirement changes only once the entire selected relational profile is implemented, not when the first alternate repository compiles.

## Success criteria

Adding another graph or vector provider changes its adapter, registration, migration/probe implementation, and tests; it requires **zero provider-name branches in query/ingest/lifecycle services**. The certified alternate relational profile must boot and complete auth, upload, query, delete, restart, and recovery without a PostgreSQL relational authority. Performance and correctness gates are in [contract validation](../validation/edge-cases-and-contract-tests.md); delivery sequencing is in [work packages](../implementation/work-packages.md).
