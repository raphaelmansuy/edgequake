# WHY: make data providers replaceable without changing application correctness

EdgeQuake already has storage traits. The missing boundary is a **complete, enforceable contract**: selecting another provider must preserve tenant isolation, document lifecycle, graph identity, retrieval filters, and failure reporting. A new adapter must not inherit successful no-ops or require PostgreSQL pools in application services.

**Initiative: Provider-Independent Data Access.** Evolve the existing domain ports into independently configured relational, graph, and vector access, with one durable relational commit boundary and replayable graph/vector projections. Keep PostgreSQL + AGE + pgvector as the default deployment. Prove substitution with SQLite, Neo4j, and Qdrant in explicitly supported profiles.

**Status (specification):** audited design and implementation plan. Audit date: **2026-09-20**. Baseline: **`c8a5af76f3dc7c1bbfa124514c13a18e5afdb86b`**, workspace version **0.26.10**, Rust MSRV **1.95**, SQLx lockfile **0.8.6**. Spec findings are static source evidence unless superseded by [evidence](evidence/).

**Status (branch implementation — honest):** **P0 production-correct hot path** is closed on `feat/149-data-access-improvements` (immutable bindings + auto-provision, durable `IngestionCommitter`, fenced lease/renew/ack projection, exact-binding tombstone cleanup, managed `ProjectionWorkerRuntime`). Product serving remains **P0 only**. **P1–P4 are unavailable** via `assert_product_serving_allowed`. Full PROVIDER-ACCESS R1 certification (**E2E01–07**, including process-kill **E2E04**) and six-profile DoD remain **open**. Cert recovery suite reports **`SPEC149-P0-HOTPATH-REPLAY`**, not E2E04. See [evidence/implementation-status.md](evidence/implementation-status.md).

Use **PROVIDER-ACCESS** for all identifiers in this directory. [SPEC-149 realtime updates](../149-fix-real-time-update/README.md) and migration `149` already exist; do not reuse their identifiers or rename them.

## Directory structure

```ascii
149-data-access-improvements/
|-- README.md
|-- design/
|   |-- why.md
|   |-- current-state-audit.md
|   |-- architecture.md
|   |-- contracts.md
|   |-- consistency-and-migration.md
|   `-- algorithms-and-budgets.md
|-- implementation/
|   |-- feasibility-and-releases.md
|   |-- work-packages.md
|   |-- step-by-step-guide.md
|   `-- persistence-and-provider-recipes.md
|-- validation/
|   |-- edge-cases-and-contract-tests.md
|   |-- e2e-test-specification.md
|   `-- definition-of-done.md
|-- references/
|   `-- official-sources.md
|-- evidence/
|   |-- code-evidence.json
|   |-- delivery-matrix.json
|   |-- implementation-status.md
|   `-- validation.txt
`-- scripts/
    `-- validate_spec.py
```

Keep requirements and rationale in `design`, delivery instructions in `implementation`, acceptance criteria in `validation`, upstream facts in `references`, and reproducible records in `evidence`. Each topic has one owner; link to it instead of copying requirements. Proposed runtime fixtures belong to their production crate/script locations, not this specification directory.

## Reading order and ownership

| Document | Owns |
|---|---|
| [WHY and principles](design/why.md) | Business outcomes, invariants, scope and explicit decisions |
| [Current-state audit](design/current-state-audit.md) | Findings F01–F14 and source evidence E01–E34 |
| [Target architecture](design/architecture.md) | Dependency direction, composition, provider profiles, DRY/SOLID ownership |
| [Access contracts](design/contracts.md) | Scope, IDs, ports, capabilities, error and batch semantics |
| [Consistency and migration](design/consistency-and-migration.md) | Commit boundary, replay, deletion, visibility, rollout and rollback |
| [Algorithms and budgets](design/algorithms-and-budgets.md) | Complexity assumptions, algorithms, resource and round-trip budgets |
| [Implementation plan](implementation/work-packages.md) | Ordered work packages W0–W9, file changes, acceptance and rollback |
| [Edge cases and validation](validation/edge-cases-and-contract-tests.md) | Tests T01–T20, mitigation, traceability and release gates |
| [Official sources](references/official-sources.md) | Dated upstream facts, applicability and documentation conflicts |
| [Feasibility and delivery](implementation/feasibility-and-releases.md) | Corrected prerequisites, releases R0-R4, senior review gates and realistic scope |
| [Junior implementation guide](implementation/step-by-step-guide.md) | 24 ordered implementation steps, file ownership, child PRs and concrete exits |
| [Persistence and provider recipes](implementation/persistence-and-provider-recipes.md) | DTO fields, schema/indexes, transactions, worker SQL, API compatibility and adapter details |
| [E2E test specification](validation/e2e-test-specification.md) | Real-provider harness, fixtures, HTTP requests, crash barriers and 15 E2E scenarios |
| [Definition of done](validation/definition-of-done.md) | Per-PR, per-release and full implementation acceptance with required evidence |
| [Implementation status (honest)](evidence/implementation-status.md) | P0 production-correct hot path closed; full R1/E2E04 and P1–P4 DoD open |

**Implementation starting point:** read [feasibility assessment](implementation/feasibility-and-releases.md), then execute [implementation guide](implementation/step-by-step-guide.md) against the contracts in [persistence recipes](implementation/persistence-and-provider-recipes.md). The plan is feasible as staged releases; operational relational extraction and SQLite are a separate release-sized effort. [Delivery matrix](evidence/delivery-matrix.json) records dependencies and proof coverage. Full completion requires all six provider compositions, not merely making unfinished profiles unavailable.

[Code evidence](evidence/code-evidence.json) stores exact source anchors and hashes. [Validator](scripts/validate_spec.py) checks local links, anchors, evidence, diagrams, and traceability. Run from repository root:

```bash
python3 specs/149-data-access-improvements/scripts/validate_spec.py
```

The performance goal is **linear application work in input/output size wherever possible**, bounded memory and batched I/O. It is not a universal O(n) database guarantee: sorted top-k, index maintenance, graph output size, and approximate retrieval have different bounds. [algorithms and budgets](design/algorithms-and-budgets.md) defines those bounds without hiding dimension, payload bytes, or network cost.
