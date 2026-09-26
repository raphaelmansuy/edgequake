# WHY end-to-end proof must exercise the actual provider composition

Adapter tests cannot detect a production constructor that still injects a memory job store or a hidden PostgreSQL pool. These scenarios boot the same composition, authorization middleware and workers as the product. They exercise HTTP plus actual persistent providers and inspect durable outcomes. **All new harness paths, fault hooks and scenarios below are implementation requirements, not tests executed by this specification.** Current audit validation remains in [contract validation](edge-cases-and-contract-tests.md).

## Harness to implement in J01 and extend in J07

Proposed layout:

```text
scripts/provider-access/
  compose.yaml                 # isolated pinned provider services
  test-profile.sh              # provision, migrate, boot, run, collect, stop
  required-ports.json          # reviewed production dependency inventory
edgequake/crates/edgequake-api/tests/
  provider_access_e2e.rs       # real HTTP and process lifecycle
  support/provider_access/
    harness.rs                 # production server launch and owned cleanup
    fixtures.rs                # scoped users/docs/known embeddings
    fault_control.rs           # test-build IPC barriers
    assertions.rs              # receipts, scopes, eventual predicates
    manifest.rs                # actual tests/provider versions/results
edgequake/crates/edgequake-storage/tests/
  provider_access_contract.rs
  provider_access_projection_recovery.rs
```

Create one unique run ID, Compose project, scratch relational database/file, graph namespace and vector collections per suite. A run ownership marker must match before cleanup; never drop a shared development database or issue global table purges. Apply real migrations to a fresh store. Use explicit connection configuration supplied by the runner, no implicit developer environment discovery. Record server/client versions, image digests, enabled Cargo features and migration checksums. Start API and worker subprocesses through production bootstrap; use actual HTTP sockets. Restart reopens the same persistent volumes and credentials.

Only external LLM/embedding/vision generation uses a deterministic test server. Configure it through normal provider configuration so generation requests traverse real clients. After ingestion, record generation-call counters; replay/recovery must not increase them. Storage, auth, task queues, migration jobs, graph traversal and vector search are never replaced by memory doubles in certification. A separate pure test layer may use doubles.

Seed bootstrap admin/accounts through the real identity provisioning/migration interface with hashed test credentials; authenticate via `/api/v1/auth/login`, then create authorized tenants/workspaces using the existing routes. If bootstrap requires a non-HTTP admin command, record it as setup rather than inventing a public endpoint. Set `auth_enabled=true`. Every scoped request carries the genuine auth token and `X-Tenant-ID`/`X-Workspace-ID`; forged headers must not override token membership. Never use `X-User-ID` alone as authentication.

The current login body is `{"username":"<fixture-user>","password":"<fixture-password>"}`; use `access_token` from the response as `Authorization: Bearer ...`. Track polling checks each returned document's status: `is_complete` alone also permits failed documents and cannot prove success. Read these shapes from [auth DTOs](../../../edgequake/crates/edgequake-api/src/handlers/auth_types.rs) and [track DTOs](../../../edgequake/crates/edgequake-api/src/handlers/documents_types/tracking.rs).

A test-only Cargo feature may expose a local IPC socket for barriers/clock control. It is absent from release builds and unreachable through public HTTP. It pauses real code at named transitions; it must not implement mutations itself. Crash tests kill only the spawned process whose PID/run marker the harness owns. Provider network faults use a run-owned proxy or service pause. No arbitrary sleep establishes correctness: await a barrier or poll a predicate with a monotonic deadline. Suggested local deadline 120 seconds/scenario, crash suite 300 seconds; timeout fails and saves diagnostics. These are test deadlines, not product latency SLOs.

## Deterministic fixture F0

Use fixed logical labels below, persist actual UUIDs returned by the API, and store the UUID mapping in the run manifest. A label is never assumed to be a server-generated UUID. Test isolation comes from the run namespace.

| Label | Scope / content | Purpose |
|---|---|---|
| tenant-A/workspace-A1 | User Alice authorized; workspace display name `research` | Main scope |
| tenant-A/workspace-A2 | Alice authorized; separate workspace | Same-tenant isolation |
| tenant-B/workspace-B1 | User Bob authorized; same display name `research` | Cross-tenant name collision |
| D-A1 | `ALPHA_ONLY`: Sarah works at Lab; Sarah founded Lab | Two typed relationships on same endpoints |
| D-A2 | `SHARED_ONLY`: Sarah mentors Lee; Sarah works at Lab | Shared Sarah/Lab/WORKS_AT contributions |
| D-A3 | Pending/unpublished document containing `PENDING_SECRET` | Visibility exclusion |
| D-B1 | `BETA_SECRET`, same entity names and query vector as D-A1 | Cross-tenant leak detection |
| D-AW2 | `WORKSPACE_SECRET`, same entity names | Workspace isolation |
| G-cycle | Sarah -> Lab -> Lee -> Sarah plus Sarah -> Sarah | Cycles/self-loop and independent edge limits |

Vectors for exact adapter fixtures: `q=[1,0,0,0]`, v1=`[1,0,0,0]`, v2=`[0.8,0.6,0,0]`, v3=`[0,1,0,0]`, v4=`[-1,0,0,0]`. Expected cosine scores 1, 0.8, 0, -1, tolerance 1e-5. Model M1 and M2 have the same dimension but different immutable identities. Where product dimension policy does not support 4, pad with zeros to the smallest already-supported dimension and record it; do not weaken production dimension validation for tests. Use exact adapter search for this oracle and a separate seeded larger dataset for ANN recall. No exact-ranking assertion on an approximate query.

The generation server returns fixed extracted entities/relations for each fixture marker and deterministic embeddings. Keep outputs accepted by the current extraction parser. Two input documents must remain different bytes to avoid unintended upload deduplication; idempotency tests intentionally reuse prepared commands instead. Add an image/modality fixture and report-generation fixture for all four vector families. Bypass answer generation using `context_only`; test retrieval contents, provenance and IDs, not free-form answer wording.

## HTTP happy-path recipe

Routes are grounded in [routes.rs](../../../edgequake/crates/edgequake-api/src/routes.rs), [query DTOs](../../../edgequake/crates/edgequake-api/src/handlers/query_types.rs), and the existing [async upload test](../../../edgequake/crates/edgequake-api/tests/e2e_spec024_text_upload_async.rs). Preserve those DTOs; generate typed request builders from them when practical.

1. Authenticate and create/resolve a workspace through existing authorized APIs; verify `/health` and proposed `/ready` report expected providers, schemas and binding generation.
2. `POST /api/v1/documents` with the JSON below and valid scope headers. Assert 202 and a nonempty `track_id`. Obtain the canonical document ID from the response or scoped track response; never derive it from the title.
3. Poll `GET /api/v1/documents/track/{track_id}` until terminal success or failure. Success additionally requires the authoritative publication manifest and all required delivery receipts; do not equate an extraction milestone with serving readiness.
4. `POST /api/v1/query` with the second JSON, actual returned document UUID substituted. Assert 200, expected scoped `sources[].document_id`, expected sentinel snippet, finite score and no forbidden sentinel. Repeat local/global/hybrid/mix modes with fixtures activating their expected retrieval arms; naive covers chunk vector retrieval.
5. Read graph via `/api/v1/graph` and existing entity/neighborhood routes; assert typed relationships and scope. API response DTO adapters belong in test assertions, not assumptions about vendor JSON.
6. `DELETE /api/v1/documents/{document_id}` returns 202. After the authority tombstone barrier, repeat query/graph/read checks; no deleted contribution can appear. Poll the cleanup receipt to completed only after physical verification. Restart and repeat reads.

```json
{"title":"provider-access-a1.txt","content":"ALPHA_ONLY: Sarah works at Lab; Sarah founded Lab.","async_processing":true}
```

```json
{"query":"ALPHA_ONLY","mode":"naive","context_only":true,"include_references":true,"enable_rerank":false,"max_results":10,"document_filter":{"document_ids":["REPLACE_WITH_RETURNED_DOCUMENT_UUID"]}}
```

Provider-specific exact mode, physical revision inspection and internal match-none filters are tested through the real adapter contract harness, because the current public QueryRequest has no exact-search switch. Do not add a public vector-array endpoint just to satisfy a test.

## Fault barriers

| Barrier | Injection | Required assertion |
|---|---|---|
| B1 after staged parent/chunk insert, before event append | Return an injected transaction error | No rows from that batch or receipt/events survive rollback |
| B2 after authority commit, before any delivery | Kill worker/server process | Receipt survives; restart delivers without regeneration |
| B3 after provider applied, before ledger acknowledgment | Kill worker | Replay same physical revision; one logical result, no duplication |
| B4 after lease claim, before provider request completes | Hold proxy request past lease takeover; later release it | Old ack rejected; late write remains invisible; latest revision retained |
| B5 after tombstone commit, before cleanup | Stop projection provider | Deleted content excluded; cleanup remains pending; no fallback binding |
| B6 after one target receipt, before second target | Stop second provider | Manifest not published prematurely; healthy target not repeatedly rewritten |
| B7 after migration page commit, before checkpoint advance | Kill migration worker | Repeated page idempotent; no scope/revision loss |
| B8 after binding-generation switch | Restart query/API workers | New requests use new generation; old cursors rejected |

Add an ambiguous-COMMIT proxy fault for relational providers where supported; SQLite uses a crash at commit-boundary instrumentation and verifies either complete old or complete new state. Do not claim identical transport failure semantics for a local file database.

## Required scenarios

Scenario IDs use the full `PROVIDER-ACCESS-E2E` prefix. References to E2E01 below abbreviate that prefix. [Delivery matrix](../evidence/delivery-matrix.json) is the machine-readable release/work/test mapping; profile applicability must be explicit, not silent skip logic.

R0 baseline variants do not assert future manifest/factory/bridge-fencing functionality. Name them explicitly as baseline cases in the manifest. From R1 onward, each applicable scenario includes its full durable-boundary assertions; a baseline case cannot satisfy a full scenario. The matrix records the first full gate separately from early baseline work. E2E08 first gates R2; E2E14/E2E15 first gate R4. Relational command fault subcases in J09 are adapter integration tests until the full server/worker recovery path exists.

### PROVIDER-ACCESS-E2E01 — Production composition and strict fixture availability

Boot the selected profile with real operational services and auth enabled. Run the HTTP happy path through query and restart, preserving data. Assert each runtime role/version matches the profile and no memory persistence is selected. Stop one required service before boot: runner exits nonzero with failed readiness and zero claimed certification. P3 additionally boots with no PostgreSQL service or reachable PG address; P4 reports PG only on the vector axis. Minimum gate R0 uses existing P0 bootstrap; future profiles join at their release.

### PROVIDER-ACCESS-E2E02 — Isolation through HTTP and graph expansion

Load F0. As Alice, query/graph/list/download D-B1 and forge Bob's tenant header; cross-scope object reads return 404 or established authorization denial and no BETA_SECRET. Repeat A1 versus A2. Use same graph names to test identity resolution. Run with the actual application PG role, including RLS where configured. At R1 add a deleted/pending bridge between visible nodes; traversal must not reach through it. Assert authorization before expansion using frontier traces, not only the final filtered response. No administrative credentials on application queries.

### PROVIDER-ACCESS-E2E03 — Filter truth table and explicit failure

Query D-A1 with IDs, title pattern, date boundaries and their documented unions/intersections. D-A2 must not leak when excluded. Test public empty array versus omitted filter (same authorized population), unknown/foreign document IDs (empty, never all), and internal `Some([])` (no provider call). Adapter fixtures additionally test allowed subject IDs and modalities on the typed branch. Hold D-A3 non-ready; PENDING_SECRET never appears. Stop vector provider after readiness: required vector query returns 503, not 200 with empty sources. Include k=0 and over-limit k validation in the adapter contract.

### PROVIDER-ACCESS-E2E04 — Atomic commit, replay and unknown outcomes

Run B1/B2/B3 independently in fresh fixtures. Inspect authority rows, receipt, event and per-binding deliveries after crash. B1 has no partial batch; B2/B3 eventually publish the same canonical IDs once. Run 20 concurrent same-key identical internal commands; all resolve to one receipt. Reuse key with altered payload: 409-equivalent conflict. Simulate lost commit response, retry same key, and verify one outcome. Record unchanged LLM/embedding counters during recovery. Minimum gate R1; J09 may run only transactional subcases before delivery exists.

### PROVIDER-ACCESS-E2E05 — Lease takeover and stale revision races

Pause revision r at B4, expire/take over its lease, commit r+1, then release old request. Assert r cannot overwrite r+1 physical data, ack epoch mismatch affects zero rows, and queries show only current authorized content. Repeat with deletion and re-upload in place of r+1. Repeat reverse order (cleanup arrives before stale upsert). Verify old recreated physical data is hidden and later swept; cleanup status remains honest while the request is unresolved. Minimum gate R1; every new projection provider repeats it.

### PROVIDER-ACCESS-E2E06 — Shared provenance and targeted deletion

Load D-A1/D-A2, parallel WORKS_AT/FOUNDED edges and a self-loop. Delete D-A1 at B5, then delete its workspace metadata only after the cleanup binding is durably recorded. Assert no ALPHA_ONLY, no FOUNDED from its sole source, and no deletion of D-A2's surviving WORKS_AT contribution. Shared facts may be temporarily absent until regenerated, then return without deleted provenance. Re-upload a new generation while cleanup is delayed; old cleanup cannot touch it. Inspect exact recorded target namespaces and default-store sentinel to prove no fallback deletion. Minimum gate R1.

### PROVIDER-ACCESS-E2E07 — Partial delivery, poison payload and authority outage

Run B6; pending content stays hidden until both required targets complete. Resume and assert only unfinished delivery is retried. Inject unknown event schema plus one valid event in another scope; unknown event quarantines with reason, valid work proceeds. Stop authority after projection candidates are obtained: query fails safely, never trusts a stale local visibility cache. Restart restores durable job/receipt state. Minimum gate R1.

### PROVIDER-ACCESS-E2E08 — Backfill, switch and rollback under writes

Write/delete while backfilling old to new binding; use B7 and B8. Compare per-partition digests, tombstones, contribution IDs, model descriptors and exact fixtures; counts alone do not pass. Failed comparison blocks switch. Switch invalidates generation-bound cursors; earlier in-flight requests obey their pinned binding plus current authority validation. Rollback refuses a lagging old binding, then succeeds after replay. R2 tests vector switch, R3 graph switch, R4 relational export/import with write fence and receipt reconciliation. Record operator commands and results.

### PROVIDER-ACCESS-E2E09 — Bounded graph/vector work and overload

Generate dense/cyclic graph, huge-degree anchor and many obsolete revisions. Request budgets depth=2, nodes=20, edges=30, small byte cap; output, queued frontier and provider candidate scans respect independent caps and report truncation. Test batch sizes B-1/B/B+1 and oversized item bytes. Instrument provider calls and peak buffered bytes; compare with [algorithms and budgets](../design/algorithms-and-budgets.md) formulas. Cancel mid-query and saturate worker queues; work stops within configured deadlines and returns explicit overload. Pure scaling tests use n=1k/10k/100k; nightly indexed workloads add representative dimensions/filter selectivity and ANN recall. Minimum gate R1.

### PROVIDER-ACCESS-E2E10 — Persistent authorization and runtime ownership

Run two production-composed runtimes with separate stores concurrently; stop one, the other still serves only its own data. Login, create/revoke a session/API key through existing APIs, restart, and verify revocation persists and forged scope fails. Exercise workspace membership change during query admission. No global pool, static selected provider or shared cache may cross runtimes. R1 covers injection ownership and existing PG operational persistence; R4 repeats through SQLite ports.

### PROVIDER-ACCESS-E2E11 — Model identity and four vector families

Use equal-dimension M1/M2 with deliberately different nearest matches. Query each binding directly through the adapter contract and through the product flows producing chunks, entities, relationships and reports. Assert model/preprocessing/content revision remains intact and provenance hydrates from the correct authority. Bad dimension, zero-norm cosine, nonfinite values and same-key different digest fail before provider I/O. Exact fixture order/scores follow F0; ANN recall is measured separately. Minimum gate R1.

### PROVIDER-ACCESS-E2E12 — Configuration, readiness and feature correctness

Test provider typo, missing feature, missing required index, schema newer than binary, exhausted role pool and stopped provider. Startup rejects invalid composition and releases created clients; `/ready` is 503 while required components cannot serve. `/health` retains its existing fields plus safe per-axis diagnostics. Trace startup to assert no DDL/full table counts. P3 rejects multi-replica config and compiles without `sqlx-postgres`; P4 accepts the standalone vector layout and rejects a co-located-only layout. R0 covers existing baseline failure reporting; full new factory assertions begin R1.

### PROVIDER-ACCESS-E2E13 — Restore authority and rebuild lost projections

Back up canonical facts, source contributions, generated embeddings, bindings, ledger and receipts at a consistent checkpoint. Restore into a new run-owned authority, provision empty projections, replay and compare fixture digests/traversal/scoped retrieval. Generation counters stay unchanged. Repeat with a missing retained payload: readiness/rebuild fails explicitly instead of silently regenerating. R1 tests P0 rebuild; each later release repeats its new provider combination. Verify restored auth revocations and tombstones, not just positive data.

### PROVIDER-ACCESS-E2E14 — Operational persistence closure and artifacts

Create a conversation/message, durable queued task, provider-budget reservation, checkpoint and migration job through existing services/API. Upload a checked-in tiny PDF and multipart original with deterministic vision output. Before/after restart, verify original bytes/digest via `/documents/{id}/download/original`, PDF download/content, page layout and configured MM asset retrieval; verify membership isolation on every path. Jobs resume once logically despite duplicate delivery; expired reservations reclaim safely. Delete/revoke then restart again; no resurrection. Requires all J21 inventory groups, and gates R4 on P0/P3/P4. Existing operational suites remain mandatory in earlier releases.

### PROVIDER-ACCESS-E2E15 — SQLite writer contention and relational cutover

On P3/P4 issue concurrent ingest/delete/job claims, inject busy errors and restart between commit steps. Every receipt is all-or-nothing, tasks have one current owner, retries bounded, readers cannot see uncommitted changes. Test NULL/UUID/time/JSON/collation contract fixtures and clock skew. Execute PG-to-SQLite migration under a final write fence, preserving IDs, receipts, sessions and cleanup bindings. After target writes, blind switch back is rejected until explicit reverse reconciliation. No PostgreSQL relational connection appears in P3 telemetry/dependency evidence. Minimum gate R4.

## CI invocation contract and evidence

J01 adds these **proposed repository-root Make targets**; until implemented they are not runnable commands:

```bash
make provider-access-test PROFILE=P0 SUITE=smoke
make provider-access-test PROFILE=P1 SUITE=contracts
make provider-access-test PROFILE=P2b SUITE=recovery
make provider-access-test PROFILE=P3 SUITE=full
make provider-access-test PROFILE=P4 SUITE=full
make provider-access-test-matrix RELEASE=R4
```

`provider-access-test` provisions its owned environment, applies migrations, builds the required features, runs selected tests, saves artifacts and stops its processes; failed state may be retained with an explicit local debug option. `smoke` covers E2E01-E2E03; `contracts` runs role conformance plus HTTP smoke; `recovery` covers E2E04-E2E08/E2E13; `full` adds all scenarios applicable to that release/profile. Test selection is emitted as planned IDs before launch and actual IDs after completion. Add tests for the runner's missing-service/empty-selection rejection.

PR CI runs changed-role contracts and P0 smoke/recovery as applicable; adapter PRs add their new compositions. Nightly runs the accumulated full matrix and performance suite. Release CI requires the complete supported matrix from [feasibility assessment](../implementation/feasibility-and-releases.md), no skips/ignored scenarios or “passed” empty selection. Deliberately unsupported cluster tests are outside the declared matrix, not skipped entries within it.

Save proposed `artifacts/provider-access/<run-id>/manifest.json`, JUnit, sanitized logs/traces, exact commands/features, profile names, actual scenario IDs/results, versions/digests, migrations, dataset hash, operation counts, peak memory, latency/recall reports, crash-barrier history and rollback/restore evidence. Include source commit and dirty-tree status. No credentials, access tokens or customer content. Implementing CI links these artifacts to [definition of done](definition-of-done.md); the docs validator cannot substitute for this run.
