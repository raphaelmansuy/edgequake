# WHY failure cases define provider compatibility

Adapters that pass happy-path CRUD can still differ on missing scope, duplicate keys, commit timeouts, model identity and deletion. The following tests define the required behavior and link findings to mitigations. These are planned tests unless explicitly listed under “Audit validation performed.”

## Edge-case and mitigation matrix

| Test | Adversarial fixture / failure | Required behavior and mitigation | Work |
|---|---|---|---|
| PROVIDER-ACCESS-T01 | Same IDs/names across tenants; missing/malformed scope; tenant name collision; scope-null legacy row; pooled connection reused by another tenant | Reject missing scope; mandatory provider predicates; tenant-scoped name resolution; scoped transaction session state/reset where used; quarantine ambiguous legacy data; inspect direct storage and API paths | W1/W3/W7–W9 |
| PROVIDER-ACCESS-T02 | Two relation types between same endpoints; reverse edge; self-loop; Unicode case variants; duplicate batch keys | Preserve full typed identity/direction; delete exactly requested edge; versioned normalization/collision detection; deterministic merge | W1/W6/W8 |
| PROVIDER-ACCESS-T03 | Crash/fault after parent write, after facts, or during required event append | Entire local batch rolls back; retry yields one canonical receipt/IDs; no orphan ready state | W3/W4/W9 |
| PROVIDER-ACCESS-T04 | Remote success then crash before acknowledgment; timeout with unknown outcome; lost response | Replay same immutable revision idempotently; detect digest conflict; never guess rollback or claim exactly-once delivery | W4/W7/W8 |
| PROVIDER-ACCESS-T05 | Delete while ingest/merge in flight; expired worker finishes late; shared entity prune | Tombstone wins at validation point; stale epoch cannot publish; retain other-document contributions; no sibling over-delete | W4/W6/W8 |
| PROVIDER-ACCESS-T06 | Vector outage, graph lag, read-replica lag, mixed manifest target completion | Pending/not-ready is explicit; check current authority; serve only complete current required revisions; bounded wait/deadline | W4/W7/W8 |
| PROVIDER-ACCESS-T07 | Same-dimension different models; zero/NaN/Inf vector; dimension mismatch; same revision different payload | No model mixing; reject malformed inputs before I/O; immutable replay conflict; score metric/tolerance validated | W1/W5/W7 |
| PROVIDER-ACCESS-T08 | Empty batch; repeated IDs; missing hydration rows; n>B; one item>M; arithmetic overflow | No-op empty input, stable positional reads, deterministic dedupe; enforce row+byte caps; reject oversized item; bounded result reporting | W1/W3/W5 |
| PROVIDER-ACCESS-T09 | Typo provider; absent feature; unsupported metrics/filters; partially initialized factory | Fail readiness with actionable reason; release created clients; no provider/default substitution | W2/W9 |
| PROVIDER-ACCESS-T10 | Workspace deleted before vector resolution; target provider unavailable; re-upload after delete | Cleanup uses persisted binding/tombstone revision; stays pending on outage; never default storage; old cleanup cannot touch new content | W4/W5/W7 |
| PROVIDER-ACCESS-T11 | Forged/stale/cross-scope cursor; equal sort keys; concurrent insert/delete during pagination | Authenticated scope/filter/generation cursor; immutable tiebreaker; documented weak/snapshot consistency; no false snapshot guarantee | W3/W9 |
| PROVIDER-ACCESS-T12 | Dense cyclic graph, duplicate incident edges, huge degree, max_nodes reached before max_edges | Frontier batching, visited+edge sets, authorization before expansion, independent depth/node/edge/byte limits and explicit truncation | W1/W6/W8 |
| PROVIDER-ACCESS-T13 | Typed search with document/modality/ID filters; `Some([])`; no-ready chunks; low-selectivity ANN; backend SQL error | Complete predicates preserved before limit semantics; zero matches differ from failure; visibility enforced; capped refill and underfill reason | W1/W5/W7 |
| PROVIDER-ACCESS-T14 | Unknown event version, poison payload, lease stolen, one bad item in batch, duplicate target delivery | Quarantine affected delivery; compare lease epoch; partial outcome bookkeeping; per-target ack; unrelated scopes continue | W4/W7/W8 |
| PROVIDER-ACCESS-T15 | Increasing n/d/metadata size, byte cap crossing, overload and cancellation | Linear expected local transformations; bounded physical request counts/memory; deadline propagation; no per-row calls; report unknown canceled write outcome | W5–W8 |
| PROVIDER-ACCESS-T16 | Two runtimes with separate stores in one process; one shuts down; concurrent tests | Owned injected clients, no leaked global pool or shared selection; independent scope/config/cache state | W3/W9 |
| PROVIDER-ACCESS-T17 | Backfill concurrent writes/deletes; interrupted restart; stale cursor after switch; failed shadow comparison | Capture watermark has no gap; idempotent resume; no switch until canonical digests/tombstones/catch-up match; stale generation rejected | W4/W7–W9 |
| PROVIDER-ACCESS-T18 | Restore canonical database but lose one projection; missing payload retention; lineage capped in display | Rebuild from complete durable facts/payloads; block readiness if required source missing; reverse contributions preserve shared data | W4/W9 |
| PROVIDER-ACCESS-T19 | Missing indexes, schema newer than binary, pool saturation, stopped provider, unsupported admin diagnostics | Read-only startup checks; explicit migrations; bounded cheap probes; no runtime DDL/full count; role isolation/degraded state reported | W2/W9 |
| PROVIDER-ACCESS-T20 | Real-provider CI has no service/credentials; SQLite selected for unsupported multi-replica profile | Certification fails, never silently skips; profile admission rejects unsupported semantics | W0/W7–W9 |

## Traceability: finding -> invariant -> delivery -> proof

| Finding | Invariants | Work packages | Tests |
|---|---|---|---|
| PROVIDER-ACCESS-F01 | I7, I8 | W2, W7, W8, W9 | T09, T19, T20 |
| PROVIDER-ACCESS-F02 | I1, I3, I7 | W3, W9 | T01, T03, T16, T20 |
| PROVIDER-ACCESS-F03 | I3 | W3, W4 | T03, T08 |
| PROVIDER-ACCESS-F04 | I2, I3, I4 | W4 | T03, T04, T06, T14, T18 |
| PROVIDER-ACCESS-F05 | I1, I2 | W1, W5 | T01, T13 |
| PROVIDER-ACCESS-F06 | I1, I2, I4 | W1, W5 | T06, T13 |
| PROVIDER-ACCESS-F07 | I2, I5 | W1, W6 | T02, T05 |
| PROVIDER-ACCESS-F08 | I6 | W1, W6 | T08, T12, T15, T19 |
| PROVIDER-ACCESS-F09 | I2, I5 | W1, W5 | T07, T13 |
| PROVIDER-ACCESS-F10 | I2, I7 | W3, W9 | T16 |
| PROVIDER-ACCESS-F11 | I2, I4, I8 | W4, W5 | T05, T10 |
| PROVIDER-ACCESS-F12 | I5, I7, I8 | W2, W5, W9 | T07, T09, T17 |
| PROVIDER-ACCESS-F13 | I2, I3 | W1 | T03, T04, T08 |
| PROVIDER-ACCESS-F14 | I8 | W0, W7, W8, W9 | T20 |

See [current-state audit](../design/current-state-audit.md) for evidence, [access contracts](../design/contracts.md) for normative semantics and [work packages](../implementation/work-packages.md) for file ownership. Each test must assert observable results/state/physical calls, not merely scan implementation strings.

## Required test layers

[E2E specification](e2e-test-specification.md) turns these requirements into concrete fixtures, real HTTP sequences, B1-B8 crash barriers and 15 scenario specifications. [Delivery matrix](../evidence/delivery-matrix.json) maps them to work/release gates; [definition of done](definition-of-done.md) owns final acceptance.

1. **Pure/property tests:** key normalization, duplicate/permutation behavior, filter truth tables, positional batch reads, cursor validation, score conversion and budget splitting. Use current proptest infrastructure where useful.
2. **Adapter contracts:** run shared fixtures against memory, PostgreSQL, SQLite, AGE, Neo4j, pgvector and Qdrant according to role. Test real transport outcomes as well as mocks.
3. **Composition/E2E:** P0–P3 plus standalone-pgvector composition from [architecture](../design/architecture.md); auth -> workspace -> ingest -> graph/vector query -> deletion -> restart. Include chunk/entity/relationship/report families and existing PDF/original/MM artifact access.
4. **Chaos:** kill after every numbered transition in [consistency and migration](../design/consistency-and-migration.md); inject delayed stale writes, ambiguous commits, concurrent deletes, poison events and replica lag. Verify durable state after restart.
5. **Performance:** [algorithms and budgets](../design/algorithms-and-budgets.md) fixtures, operation counts, EXPLAIN/query profiles and retrieval quality. Exact-vs-ANN comparisons use fixed synthetic vectors, not paid LLM calls.
6. **Architecture:** fail unallowlisted SQL/driver imports in policy crates; compile contracts with no vendor features; compile each supported provider feature set. Verify dynamic trait-object construction. Audit source scans allow only real composition/migration/adapter exceptions.

## Commands and acceptance gates

Existing baseline commands, run from `edgequake/`:

```bash
cargo test -p edgequake-storage --no-default-features --test storage_backend_contract
cargo test -p edgequake-storage --features postgres --test contract_spec098_edge_delete_arbiter
cargo test -p edgequake-storage --features postgres --test e2e_spec098_saturated_spine_ensure
cargo test -p edgequake-storage --features postgres --test e2e_spec098_cypher_edge_multigraph
cargo test -p edgequake-storage --features postgres --test e2e_spec130_rel_identity_map
cargo test -p edgequake-storage --lib
cargo clippy -p edgequake-storage --all-targets --features postgres -- -D warnings
cargo fmt --check
```

PostgreSQL commands require isolated provisioned fixtures. They are future implementation regression gates, not claims that they ran for this documentation audit. Use the repository Makefile setup for service-backed runs; never point destructive conformance fixtures at a user's production database.

W0 adds proposed `provider_access_contract`, `provider_access_projection_recovery` and a profile-matrix runner; exact command names must be wired into CI before certification. Every run emits provider/server/client versions, image digests, schema revisions, test counts and skips, topology, workload hash and results. Full workspace tests/lints and existing 091/098 migration/deletion suites remain required before implementing releases.

**Release gate:** no failed/uncertified required profile, no skipped certification cases, no unresolved isolation/identity/recovery mismatch, all manifests complete, performance thresholds satisfied or explicitly narrowed profile documented, and one demonstrated rollback plus restore/rebuild.

## Audit validation performed

This change creates specifications and validation artifacts only. It does not implement providers, repair the identified runtime defects, run a production migration, or establish a new latency/recall benchmark.

- Local document/source validation: run `python3 specs/149-data-access-improvements/scripts/validate_spec.py` from repository root; verifies local links/headings, 34 source hashes/line anchors, ASCII box alignment and finding/invariant/work/test coverage.
- Existing memory contract baseline: **3 passed, 0 failed, 0 ignored** with `CARGO_TARGET_DIR=<isolated-temporary-target> cargo test -p edgequake-storage --no-default-features --test storage_backend_contract --offline` from `edgequake/`.
- The first Cargo attempt could not write the configured `/Volumes/CargoBuild/target/debug/.cargo-lock`; the isolated temporary target above resolved that filesystem restriction without changing repository configuration.
- Storage library Clippy: **passed** with `--no-default-features --lib --offline -- -D warnings` and the same temporary target; command arguments and results are in [validation evidence](../evidence/validation.txt). Historical temporary target paths are elided after the naming revision; the placeholder is not a literal runnable path.
- No live PostgreSQL/AGE/pgvector, Neo4j, Qdrant or SQLite certification/chaos/benchmark run was performed. The three baseline tests cannot certify the proposed design.
