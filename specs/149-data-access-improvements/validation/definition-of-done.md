# WHY completion must mean demonstrated behavior

PROVIDER-ACCESS is done when independently selected relational, graph and vector providers preserve the product's contracts under normal operation, failure and migration. A compiled adapter or green memory test is insufficient. This document is the release checklist; [implementation guide](../implementation/step-by-step-guide.md) supplies implementation steps and [E2E specification](e2e-test-specification.md) supplies executable-test requirements.

## Definition of ready for an implementation PR

- Its J step/W package, owning module, invariant and regression case are identified.
- Earlier dependencies and any required G1-G5 review are complete; unresolved authority/identity/transaction decisions are not delegated implicitly to a junior developer.
- The relevant real-provider fixture runs locally/CI, the expected failing behavior is reproducible, and migration rollback compatibility is stated.
- The PR is one reviewable behavior; larger steps split into the child PRs in [implementation guide](../implementation/step-by-step-guide.md).

## Definition of done for every implementation PR

- Required semantics are implemented for every affected current adapter and test double, with no successful no-op defaults or silent provider fallback.
- Unit/property tests cover input boundaries; shared adapter contracts cover real transport/storage behavior; affected E2E scenario passes when its prerequisites exist.
- Changed code is formatted, targeted tests and Clippy pass, and the architecture allowlist has no unexplained additions. Public API compatibility is tested.
- Added schema is additive until the agreed retirement gate, migrations work on fresh and upgrade fixtures, scoped keys/indexes are checked, and old migration checksums are untouched.
- Failure paths preserve typed errors, bounded retries/cancellation, idempotency and safe diagnostics. No secrets or unbounded payload logging.
- The PR contains exact validation commands/results and links to retained artifacts, including explicit unavailable fixtures. Unavailable mandatory CI cannot be marked passed.
- Source docs, capabilities, required-port inventory and run manifests match the final implementation. Proposed names are replaced by actual implemented paths/commands when they differ.

## Release gate checklist

| Release | Must demonstrate before enabling new behavior |
|---|---|
| R0 | J01-J05; P0 filter/isolation/mutation/error regressions, strict test infrastructure, baseline workload and complete dependency inventory |
| R1 | J06-J16; G1-G3; P0 real atomic commits, all crash/replay/visibility cases, bounded graph/vector work, no global runtime state, standalone pgvector adapter conformance |
| R2 | J17-J18; P0/P1 full applicable E2E and vector contracts; vector migration/rollback/restore, pinned Qdrant compatibility, completed soak |
| R3 | J19-J20; P0/P1/P2a/P2b full applicable E2E and graph contracts; graph migration/rollback/restore, pinned Neo4j compatibility, completed soak |
| R4 | J21-J24; all six compositions P0/P1/P2a/P2b/P3/P4; complete operational ports, SQLite contention/restart, PG-free P3 dependency/boot proof, relational cutover/reverse-reconciliation drill |

Earlier releases may explicitly leave later profiles unavailable. **Full PROVIDER-ACCESS completion cannot use “unavailable” to avoid any of the six compositions.** If scope is reduced later, that is a documented scope decision, not completion of this plan.

## Full implementation definition of done

1. **Correctness:** I1-I8 and F01-F14 have passing runtime proof linked through T01-T20 and E2E01-E2E15. Every required profile has real providers and persistent operational services. No certification skip/ignore or zero-test success.
2. **Provider independence:** swapping an axis changes validated configuration/binding and adapter registration, not application policy. P2a/P2b and P3/P4 prove combinations; P3 has no PostgreSQL service or `sqlx-postgres` feature. Standalone pgvector never relies on remote relational FKs/joins.
3. **DRY/SOLID:** one definition of scope/IDs/filter semantics/normalization/batch/retry policy; narrow role ports; no driver dependencies in the contract leaf; no unallowlisted SQL/driver references in policy. Old global pools, fake transaction handles and duplicated vector routing are removed.
4. **Durability:** canonical commit includes every required intent/receipt; crash and ambiguous outcomes reconcile by request key; independent target delivery is replayable; unknown schema quarantines; no LLM/embedding regeneration during replay.
5. **Isolation/visibility:** same-name tenants/workspaces never cross; all filters survive routing; current authority gates serving and graph expansion; pending/deleted/stale contributions never leak, including shared descriptions. Deletion uses recorded bindings and exact revisions.
6. **Lifecycle:** upload, query, graph access, deletion, re-upload, original/PDF/MM retrieval, conversations, auth revocation, tasks, budgets, checkpoints and migrations survive restart on each applicable profile. Required-port inventory has no placeholder or memory replacement.
7. **Performance:** declared local complexity and physical-call budgets verified; bounded rows/bytes/candidates/frontiers/queues/retries; realistic provider query plans retained. ANN recall and p95 meet [algorithms and budgets](../design/algorithms-and-budgets.md), or profile scope/threshold changes receive a documented benchmark-based review before certification. No unsupported universal O(n) claim.
8. **Migration/recovery:** no capture gap, interrupted copy resumes, tombstones/provenance match, generation switch/cursors behave correctly, rollback target is caught up, and restore plus rebuild succeeds. Required payload absence fails honestly. Seven-day proposed soak from [consistency and migration](../design/consistency-and-migration.md) is completed or formally lengthened; it is not waived because tests passed once.
9. **Operations:** readiness includes every required role; startup performs no DDL; schema/capability/version mismatch blocks readiness; bounded health probes and lag/quarantine/unknown-outcome metrics are available. Physical cleanup status never claims erasure before late writers are accounted for.
10. **Evidence:** implementing release stores source commit, feature graph, provider/image/client versions, migration checksums, fixture hashes, exact test counts/results, latency/recall/request metrics and migration/restore evidence. Full workspace tests/lints and existing SPEC-091/098 deletion/migration regression gates pass before release.

## Review procedure and hard blockers

The release reviewer reads the required-port inventory, checks actual CI scenario IDs against [delivery-matrix.json](../evidence/delivery-matrix.json), then samples a scope leak test, crash-after-provider-write test, targeted deletion test and restore drill artifact. Re-run one missing-service test to verify certification fails honestly. Confirm the build being deployed has no test IPC/fault-control feature.

Hard blockers: cross-scope data, deleted/non-current content served, acknowledged lost writes, unknown events treated as success, unresolved provenance divergence, wrong-model search, uncertified required composition, unbounded request fallback, or a rollback path that cannot reconcile post-cutover writes. Fail the gate and keep the affected binding disabled; document the failing artifact and owning J step.

## Documentation assessment completion

This documentation revision is complete when the new execution steps, schema/provider recipes, E2E cases and release DoD are cross-referenced, dependency ordering is consistent, source claims remain grounded, and the local validator passes. That status is separate from all implementation checklists above. No runtime provider, E2E certification, migration or soak has been performed by writing these documents. Actual checks for this revision are appended to [validation evidence](../evidence/validation.txt).
