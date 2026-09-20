# WHY composition and policy need separate owners

A provider can change only when application services depend on semantics rather than pools, table names, or vendor switches. Preserve the existing batch-first ports, complete them, and inject narrow capabilities. [Findings F01/F02/F10/F12](current-state-audit.md) motivate this boundary.

## Proposed dependency direction

```ascii
+-------------------------------------------------------------+
| API / core / pipeline / query / task policy                 |
| validated scope, domain commands, bounded reads             |
+-------------------------------------------------------------+
                              |
                              v
+-------------------------------------------------------------+
| edgequake-storage-contracts (proposed small leaf crate)     |
| IDs | scope | revisions | errors | ports | capability types |
+-------------------------------------------------------------+
          ^                   ^                    ^
          | implements        | implements         | implements
+------------------+ +------------------+ +--------------------+
| Relational       | | Graph            | | Vector             |
| PostgreSQL       | | AGE              | | pgvector           |
| SQLite           | | Neo4j            | | Qdrant             |
+------------------+ +------------------+ +--------------------+
          ^                   ^                    ^
          +-------------------+--------------------+
                              |
                  Composition root owns clients
```

Initially re-export contracts from `edgequake-storage` so callers migrate incrementally. Extract only driver-free types/ports/shared pure key semantics into the leaf crate; keep SQL helpers, drivers, migrations, and data movement out. Avoid one crate per tiny repository. Start adapter modules in the existing storage crate; move heavyweight vendor dependencies to provider crates only if compile/feature isolation requires it.

The graph mutation intent flows **from the relational commit to projection delivery**; runtime dependencies do not flow from a graph adapter into the relational implementation. Shared state visibility is applied by a retrieval service using the visibility port, with an optional co-located optimization proven equivalent.

## Runtime construction

Proposed `DataAccessConfig` contains separate tagged relational/graph/vector configurations plus role budgets and a versioned binding identifier. Each factory returns typed capability bundles; services receive only the handles they use.

```text
DataAccessFactory::build(validated_config)
  -> RelationalRuntime { document_reads, chunk_reads, ingest_commits,
       lifecycle_commits, graph_facts, visibility, delivery_ledger,
       identity, workspaces, conversations, artifacts, jobs, migrations }
  -> GraphRuntime { reads, mutations, scans, optional_analytics }
  -> VectorRuntime { search, mutations, index_catalog, optional_sparse }
```

These are composition bundles, not a giant trait every caller must implement. Construction sequence: parse strictly -> resolve secret references -> build role-limited clients -> read-only schema/version probes -> validate required capabilities/profile -> inject services -> start workers -> report ready. Failure closes already-created clients and prevents partial runtime publication. Shutdown drains bounded work, cancels requests, and closes clients owned by the runtime.

Proposed configuration, **not available today**:

```toml
[data_access.relational]
provider = "postgres"
connection_env = "DATABASE_URL"

[data_access.graph]
provider = "neo4j"
connection_env = "EDGEQUAKE_GRAPH_URL"

[data_access.vector]
provider = "qdrant"
connection_env = "EDGEQUAKE_VECTOR_URL"
```

Registration is explicit at build time plus runtime selection among compiled providers. Unknown providers, missing Cargo features, inconsistent scopes, unsupported filters, and insufficient durability capabilities fail startup with actionable errors. No `as_any`, downcasts, raw SQL callbacks, or provider-name matches in domain services. Full registry labels distinguish provider identity from `EDGEQUAKE_VECTOR_BACKEND`, which remains a temporary PG schema-cutover concern.

Existing config resolves to PostgreSQL/AGE/pgvector with identical defaults. No automatic live provider switch from an environment-variable change: binding changes use the controlled migration in [consistency and migration](consistency-and-migration.md).

## Deployment profiles and actual independence

| Profile | Relational | Graph | Vector | Admission gate |
|---|---|---|---|---|
| P0 default | PostgreSQL | AGE | pgvector | Current behavior retained; new contracts pass |
| P1 vector substitution | PostgreSQL | AGE | Qdrant | W7 recovery/filter certification |
| P2 graph substitution | PostgreSQL | Neo4j | pgvector or Qdrant | W8 graph identity/provenance certification |
| P3 relational substitution | SQLite | Neo4j | Qdrant | W9 single-node operational-port certification |

AGE or pgvector still needs a PostgreSQL service even with SQLite relational authority. Independence means separately selectable bindings and semantics, not removal of a provider's own infrastructure dependency. P3 is the proof that application relational access can operate without SQLx Postgres. SQLite's single-writer constraint restricts P3 to a qualified single-node profile; unsupported multi-replica scheduling is rejected rather than silently emulated. [Official SQLite source](../references/official-sources.md#s10-sqlite-isolation).

The pgvector adapter must declare whether it requires co-location with the relational authority. Its existing typed tables/FKs and hydration joins are a P0 optimization, not a universal provider contract. W5 adds an adapter-owned standalone projection layout for an external relational authority: canonical scoped IDs, revision/model fields and filter payload are stored locally without foreign keys into another provider. Hydration/visibility run through relational ports. No runtime may claim independent selection while silently relying on those co-located joins. Qualify this layout with an additional SQLite + Neo4j + standalone-pgvector composition fixture; it still runs a PostgreSQL vector service.

Memory adapters implement conformance fixtures only. Additional providers (for example MySQL) can be added after the same contracts; no support claim is made here.

## DRY and SOLID made concrete

| Principle | One owner / enforcement |
|---|---|
| SRP | Scope authorizer validates access; committer owns atomic domain changes; adapter translates/storage-executes; delivery worker retries; lifecycle service decides visibility |
| OCP | New provider implements ports and factory registration; no changes to ingest/query policy or conformance assertions |
| LSP | Every provider honors empty filters, exact edge identity, revision ordering and error outcomes; capabilities reject unsupported optional behavior |
| ISP | Reuse graph read/mutate/scan/analytics split; separate vector query/mutation/index lifecycle and sparse search; split relational domain roles |
| DIP | Policy crates depend on contract types; concrete drivers owned by composition/adapters |
| DRY | One normalization module, scope model, batch planner, retry policy and conformance suite; adapter-specific SQL/Cypher/payload compilation remains distinct |

Do not conflate DRY with a single untyped JSON repository. Shared semantic rules need one definition; SQL UNNEST, Cypher UNWIND, and Qdrant point batches are different mechanisms.

Preserve existing `EntityId`, graph dedupe, dimension policy, and DATA-* registry where semantics match. Move legacy IDs/table naming into compatibility adapters with removal gates. Share DTOs; do not share a global database connection across runtimes.

## Operational boundaries

Expose provider/version/schema/binding/health/capabilities per axis, plus projection lag and whether serving is degraded. Keep query/ingest/queue/admin budget separation from `PgPoolBundle`; other providers map these roles to their own client limits. Health probes are bounded connectivity checks, never full counts. Diagnostics may expose provider-specific detail behind an explicitly privileged optional interface; ordinary product routes never require PG diagnostics.

Schema creation belongs to explicit migration/provisioning commands. Startup verifies; it must not recreate retired tables through `initialize()`. Provider migration histories are separate and checksummed. Do not allocate a new migration number based on the specification number.

Detailed signatures and capability semantics: [access contracts](contracts.md). Durable state ownership: [consistency and migration](consistency-and-migration.md). Concrete file moves: [work packages](../implementation/work-packages.md).
