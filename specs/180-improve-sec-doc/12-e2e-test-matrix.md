# 12 — Test and E2E matrix

[Index](README.md) · Scope: [Requirements](03-product-spec.md) · Risks: [Edge cases](11-edge-cases.md) · Trace: [Matrix](15-traceability.md)

## Evidence levels and fixtures

**Unit/component** tests verify deterministic draft, serializer and interaction behavior. **API/DB integration** tests prove policy, locking, RLS and transactions. **Mocked Playwright** proves screen behavior only. **Live authenticated Playwright + PostgreSQL** proves complete upload/edit/read workflows. No mocked test substitutes for a live security assertion.

Seed two tenants, two workspaces in one tenant, a writer with no clearance, HR-only and Atlas-only writers, a combined-grant writer, an owner without steward, a write-enabled steward, a readonly steward, inactive/revoked members and scoped service principals. Seed reviewed Public/Internal/Confidential/Restricted, unverified with known uploader, null legacy, empty-tag, multi-tag and forbidden documents. Give forbidden fixtures unique canary phrases so title, body, citation, graph and prompt exposure are detectable. Use mock LLMs for deterministic tests; inspect captured prompts for forbidden canaries, not merely final answers.

Application-role DB tests must assert the actual session role and lack of BYPASSRLS. Missing DB, migration head, fixture creation or application role is a **failed live job**, never a passing skip. Keep administrative fixture setup separate from user assertion connections. Repeat the race/fault scenarios with deterministic barriers rather than sleeps. Include worker insert/upsert and deletion/reprocess paths.

## Test catalog

### T-180-01

**Upload intent before network admission** — UI + live E2E. Use a writer, select a PDF, set Confidential plus HR, and assert no upload request until Upload; then verify persisted label and first visible shell.

Covers [R-180-01](03-product-spec.md#r-180-01) and [EC-180-01](11-edge-cases.md#ec-180-01); delivered by WP-03/04 in [Plan](13-implementation-plan.md).

### T-180-02

**Dismissal is not a commit** — Component + UI E2E. Change a tag then exercise every dismissal path; assert zero mutations, saved version unchanged, and focus returns correctly.

Covers [R-180-02](03-product-spec.md#r-180-02) and [EC-180-02](11-edge-cases.md#ec-180-02); delivered by WP-05 in [Plan](13-implementation-plan.md).

### T-180-03

**Privacy-safe audience explanation** — API + live E2E. Compare writer/steward previews, submit foreign and hidden IDs, and verify response bodies contain no unauthorized names or counts.

Covers [R-180-03](03-product-spec.md#r-180-03) and [EC-180-03](11-edge-cases.md#ec-180-03); delivered by WP-02 in [Plan](13-implementation-plan.md).

### T-180-04

**AND semantics and scoped Public** — Domain + live E2E. Seed HR-only, Atlas-only, HR+Atlas and no-membership users; only the qualified combined subject reads Confidential+HR+Atlas; anonymous Public read is denied.

Covers [R-180-04](03-product-spec.md#r-180-04) and [EC-180-04](11-edge-cases.md#ec-180-04); delivered by WP-02/07 in [Plan](13-implementation-plan.md).

### T-180-05

**RBAC and clearance capability intersection** — Domain + API + UI. Exercise the full capability matrix with direct API calls and UI controls; no role name alone bypasses scope or clearance.

Covers [R-180-05](03-product-spec.md#r-180-05) and [EC-180-05](11-edge-cases.md#ec-180-05); delivered by WP-02 in [Plan](13-implementation-plan.md).

### T-180-06

**Catalog loading, empty and unavailable** — Component + UI E2E. Delay and fail catalog calls, search 10k tags, paginate, clear search and toggle values; no selection is silently lost and failure never becomes empty.

Covers [R-180-06](03-product-spec.md#r-180-06) and [EC-180-06](11-edge-cases.md#ec-180-06); delivered by WP-02/04 in [Plan](13-implementation-plan.md).

### T-180-07

**Initial exposure window** — DB race + live E2E. Pause admission at transaction barriers, attempt list/download/task reads, and release worker; observe no unlabeled or permissive intermediate resource.

Covers [R-180-07](03-product-spec.md#r-180-07) and [EC-180-07](11-edge-cases.md#ec-180-07); delivered by WP-03 in [Plan](13-implementation-plan.md).

### T-180-08

**Transport and metadata equivalence** — Unit + API + MCP E2E. Apply shared valid/invalid fixtures across all transports, including multipart order permutations and conflicting metadata; compare persisted labels and stable error codes.

Covers [R-180-08](03-product-spec.md#r-180-08) and [EC-180-08](11-edge-cases.md#ec-180-08); delivered by WP-03 in [Plan](13-implementation-plan.md).

### T-180-09

**Stale edit and policy epoch** — DB concurrency + UI. Race two identical versions using barriers; one commits and one returns 412/409 with no partial child writes; revoked capability never commits.

Covers [R-180-09](03-product-spec.md#r-180-09) and [EC-180-09](11-edge-cases.md#ec-180-09); delivered by WP-01/02 in [Plan](13-implementation-plan.md).

### T-180-10

**Mixed broadening and review** — Domain + API + UI. Try lower-rank, tag-removal, rank-up/tag-removal, unverified-to-reviewed and equal-set permutations for steward/non-steward; validate reason bounds and exact audit.

Covers [R-180-10](03-product-spec.md#r-180-10) and [EC-180-10](11-edge-cases.md#ec-180-10); delivered by WP-02/05 in [Plan](13-implementation-plan.md).

### T-180-11

**Audit and reason privacy** — DB fault + API. Inject audit failure and lost response, replay same operation, then inspect audit metadata and analytics; exactly one event and no protected content in analytics.

Covers [R-180-11](03-product-spec.md#r-180-11) and [EC-180-11](11-edge-cases.md#ec-180-11); delivered by WP-01/02 in [Plan](13-implementation-plan.md).

### T-180-12

**Child propagation and rollback** — DB integration. Seed all seven child tables plus vector/graph references, fail each copy step, and compare labels/versions/epoch/audit before and after rollback.

Covers [R-180-12](03-product-spec.md#r-180-12) and [EC-180-12](11-edge-cases.md#ec-180-12); delivered by WP-01/07 in [Plan](13-implementation-plan.md).

### T-180-13

**Off, null and unknown policy** — API + UI. Render each state and call APIs directly; off-mode uploads preserve compatibility, off-mode label writes reject, enforced NULL reads are hidden.

Covers [R-180-13](03-product-spec.md#r-180-13) and [EC-180-13](11-edge-cases.md#ec-180-13); delivered by WP-02/05 in [Plan](13-implementation-plan.md).

### T-180-14

**Filtered counts and pagination** — DB + live UI. Seed 250 visible and hidden mixed-label documents; page/search/sort with tag all-of filters; counts and rows match, hidden counts never appear.

Covers [R-180-14](03-product-spec.md#r-180-14) and [EC-180-14](11-edge-cases.md#ec-180-14); delivered by WP-06 in [Plan](13-implementation-plan.md).

### T-180-15

**Bulk mixed state and atomic failure** — API + live UI. Exercise Add/Remove/Replace, then fail one member or version; verify zero changes on failure and only intended fields change on success.

Covers [R-180-15](03-product-spec.md#r-180-15) and [EC-180-15](11-edge-cases.md#ec-180-15); delivered by WP-06 in [Plan](13-implementation-plan.md).

### T-180-16

**Foreign or invalid vocabulary reference** — DB + API. Attempt forged UUIDs through API and application-role SQL; reference tag only in a profile rule and attempt delete; verify denial and no projection drift.

Covers [R-180-16](03-product-spec.md#r-180-16) and [EC-180-16](11-edge-cases.md#ec-180-16); delivered by WP-01/02 in [Plan](13-implementation-plan.md).

### T-180-17

**Scope changes and dedup** — API + live UI. Switch workspace while queued and during preview, submit stale requests, upload hidden duplicate bytes; no wrong-scope writes or hidden IDs/titles/status.

Covers [R-180-17](03-product-spec.md#r-180-17) and [EC-180-17](11-edge-cases.md#ec-180-17); delivered by WP-03/05/07 in [Plan](13-implementation-plan.md).

### T-180-18

**List/detail/reload consistency** — Component + live UI. Save once, inspect list/preview/detail, reload and refresh mid-draft; all saved summaries agree and the dirty draft is retained with conflict notice.

Covers [R-180-18](03-product-spec.md#r-180-18) and [EC-180-18](11-edge-cases.md#ec-180-18); delivered by WP-05 in [Plan](13-implementation-plan.md).

### T-180-19

**Accessibility and responsive state** — A11y + visual + manual. Run axe plus manual keyboard/screen-reader checklist at 320/375/768/1440px, 200%/400% zoom, RTL, dark/light, reduced-motion and forced-colors; verify sticky footer does not obscure focus.

Covers [R-180-19](03-product-spec.md#r-180-19) and [EC-180-19](11-edge-cases.md#ec-180-19); delivered by WP-04/05/07 in [Plan](13-implementation-plan.md).

### T-180-20

**High-cardinality performance** — Performance. Measure 100 warm/cold samples with seeded cardinalities; report p50/p95 against product budgets, query plans, lock wait and client interaction delay.

Covers [R-180-20](03-product-spec.md#r-180-20) and [EC-180-20](11-edge-cases.md#ec-180-20); delivered by WP-01/07 in [Plan](13-implementation-plan.md).

### T-180-21

**Network errors and unknown commit** — Fault + UI E2E. Drop responses at each boundary; same key/body yields one outcome, different body conflicts, auth change requires new preview and explicit retry.

Covers [R-180-21](03-product-spec.md#r-180-21) and [EC-180-21](11-edge-cases.md#ec-180-21); delivered by WP-02/05 in [Plan](13-implementation-plan.md).

### T-180-22

**Old clients and upgrade ordering** — API compatibility. Test old request fixtures through server-first rollout, then strict mode; omitted upload is unverified and preconditionless edit is rejected after cutoff.

Covers [R-180-22](03-product-spec.md#r-180-22) and [EC-180-22](11-edge-cases.md#ec-180-22); delivered by WP-02/08 in [Plan](13-implementation-plan.md).

### T-180-23

**Trust metrics without data leakage** — Contract + analytics. Capture outbound analytics for every flow; assert no filenames/tag IDs/reasons and identical security decisions under UX variants.

Covers [R-180-23](03-product-spec.md#r-180-23) and [EC-180-23](11-edge-cases.md#ec-180-23); delivered by WP-08 in [Plan](13-implementation-plan.md).

### T-180-24

**False green security gate** — Live release gate. Run against real app role and authenticated users with known allowed/forbidden fixtures; deliberately break enforcement in isolated test to prove negative tests detect it.

Covers [R-180-24](03-product-spec.md#r-180-24) and [EC-180-24](11-edge-cases.md#ec-180-24); delivered by WP-07 in [Plan](13-implementation-plan.md).

### T-180-25

**Batch defaults and retry snapshots** — Unit + live UI. Select three formats, override one, change shared defaults, partially fail and retry; assert exact request labels and one admitted identity per operation. Verify the first admission does not invalidate sibling files' policy epoch; an actual policy change does. Changed intent after definite rejection gets a new key, while unknown outcomes keep the original key.

Covers [R-180-01](03-product-spec.md#r-180-01) and [EC-180-25](11-edge-cases.md#ec-180-25); delivered by WP-03/04 in [Plan](13-implementation-plan.md).

### T-180-26

**Set normalization and duplicates** — Unit + domain. Property-test permutations/duplicates and compare browser/server fixtures; equal sets are unchanged, including legacy duplicate-array repair fixtures.

Covers [R-180-06](03-product-spec.md#r-180-06) and [EC-180-26](11-edge-cases.md#ec-180-26); delivered by WP-01/02 in [Plan](13-implementation-plan.md).

### T-180-27

**Lock ordering and concurrent bulk** — DB concurrency. Use reversed overlapping ID lists plus membership revocation barriers; verify no deadlock escape, no stale-authority commit and deterministic rollback/retry behavior.

Covers [R-180-09](03-product-spec.md#r-180-09) and [EC-180-27](11-edge-cases.md#ec-180-27); delivered by WP-01/02 in [Plan](13-implementation-plan.md).

### T-180-28

**Worker writes after relabel** — DB race + worker E2E. Barrier a child insert/upsert against relabel in both orders for every child writer; final descendants match parent and no stale visibility window occurs.

Covers [R-180-12](03-product-spec.md#r-180-12) and [EC-180-28](11-edge-cases.md#ec-180-28); delivered by WP-03/07 in [Plan](13-implementation-plan.md).

### T-180-29

**Revocation during RAG and streams** — Live retrieval E2E. Warm every cache then restrict label while query streams; after commit no newly admitted frame/read returns forbidden facts; verify MCP fetch/history/citations/originals too.

Covers [R-180-12](03-product-spec.md#r-180-12) and [EC-180-29](11-edge-cases.md#ec-180-29); delivered by WP-07 in [Plan](13-implementation-plan.md).

### T-180-30

**RLS role and pooled-session contamination** — DB security. Alternate principals/workspaces on one connection pool and compare API/SQL decisions; verify the runner rejects owner/superuser/BYPASSRLS execution.

Covers [R-180-16](03-product-spec.md#r-180-16) and [EC-180-30](11-edge-cases.md#ec-180-30); delivered by WP-01/07 in [Plan](13-implementation-plan.md).

### T-180-31

**Long names, hostile strings and locales** — Unit + UI + a11y. Test NFC/case collisions, HTML-looking names, 64/65 characters, 40% translation expansion and screen-reader chip names; no execution or clipped critical state.

Covers [R-180-19](03-product-spec.md#r-180-19) and [EC-180-31](11-edge-cases.md#ec-180-31); delivered by WP-02/04 in [Plan](13-implementation-plan.md).

### T-180-32

**MCP/API key attribution** — API + MCP live. Exercise scoped keys and delegated actors; revoke during session upload and finalize; reject ownerless unverified admission, no inaccessible original leaks.

Covers [R-180-08](03-product-spec.md#r-180-08) and [EC-180-32](11-edge-cases.md#ec-180-32); delivered by WP-03/07 in [Plan](13-implementation-plan.md).

### T-180-33

**Break-glass and uploader exception** — Domain + API + UI. Test deny-limited steward, uploader of unverified, non-uploader and emergency flag; UI explanations agree with actual policy and all bypass reads are audited.

Covers [R-180-13](03-product-spec.md#r-180-13) and [EC-180-33](11-edge-cases.md#ec-180-33); delivered by WP-02/07 in [Plan](13-implementation-plan.md).

### T-180-34

**Duplicate reingestion changes identity** — API + live ingestion. Upload same bytes with changed requested access while queued/completed/failed; preserve original access and reject implicit relabel; hidden object is neither named nor deleted.

Covers [R-180-17](03-product-spec.md#r-180-17) and [EC-180-34](11-edge-cases.md#ec-180-34); delivered by WP-03/07 in [Plan](13-implementation-plan.md).

### T-180-35

**Migration and dual representation drift** — Migration integration. Migrate clean and intentionally drifted fixtures, restart migration and rollback UI; no silent privilege changes, collisions reported and constraints validated.

Covers [R-180-16](03-product-spec.md#r-180-16) and [EC-180-35](11-edge-cases.md#ec-180-35); delivered by WP-01 in [Plan](13-implementation-plan.md).

### T-180-36

**Malformed payload and limits** — Contract fuzz + API. Table-test all invalid variants, field orders, empty sets, mixed types and 16KiB boundary; no task, original reference, document or LLM call on rejection.

Covers [R-180-08](03-product-spec.md#r-180-08) and [EC-180-36](11-edge-cases.md#ec-180-36); delivered by WP-02/03 in [Plan](13-implementation-plan.md).

### T-180-37

**No-op and idempotency lifecycle** — DB + API. Replay same/different requests, concurrent pending receipt, expired lease after process death, post-retention request and revoked visibility; verify recovery under receipt locks, response contract and exactly-once effective mutation.

Covers [R-180-21](03-product-spec.md#r-180-21) and [EC-180-37](11-edge-cases.md#ec-180-37); delivered by WP-01/02 in [Plan](13-implementation-plan.md).

### T-180-38

**Tag rename, retirement and deletion** — DB + UI. Race catalog changes with draft commit; conflict/revalidate, stable IDs, no automatic requirement removal; manager deletion fails for rule-only reference.

Covers [R-180-16](03-product-spec.md#r-180-16) and [EC-180-38](11-edge-cases.md#ec-180-38); delivered by WP-01/02/05 in [Plan](13-implementation-plan.md).

### T-180-39

**Oversized bulk and child count** — API + performance. Test 0/1/100/101 document inputs and aggregate child cap around boundary; above-cap returns ACCESS_CHANGE_TOO_LARGE and leaves all state unchanged.

Covers [R-180-15](03-product-spec.md#r-180-15) and [EC-180-39](11-edge-cases.md#ec-180-39); delivered by WP-06/07 in [Plan](13-implementation-plan.md).

### T-180-40

**Admission/outbox/object failure** — DB fault + worker E2E. Inject failures at each boundary and restart workers; no visible unlabeled original, orphan public reference or duplicate task; cleanup preserves committed originals.

Covers [R-180-07](03-product-spec.md#r-180-07) and [EC-180-40](11-edge-cases.md#ec-180-40); delivered by WP-03/07 in [Plan](13-implementation-plan.md).

## Proposed test locations

These paths are planned, not files claimed to exist:

| Layer | Proposed location | Main coverage |
|---|---|---|
| Domain | `edgequake/crates/edgequake-core/src/clearance/tests.rs` (extend) | Existing semantics preserved; mixed changes and normalization |
| API | `edgequake/crates/edgequake-api/tests/spec180_access_contract.rs` | Capability, error and transport contract |
| PostgreSQL | `edgequake/crates/edgequake-api/tests/spec180_access_concurrency.rs` | Locks, version/epoch, audit rollback, worker races |
| Ingestion | `edgequake/crates/edgequake-api/tests/spec180_access_admission.rs` | All transports, originals, outbox, dedup |
| Web unit | `edgequake_webui/src/lib/clearance/__tests__/access-draft.test.ts` | Equality, serialization, fixtures |
| Web component | `edgequake_webui/src/components/clearance/__tests__/document-access-editor.test.tsx` | Explicit save, focus, stale preview |
| Mock browser | `edgequake_webui/e2e/spec180/access-editor.spec.ts` | Screens and states, tagged `@spec180` |
| Live browser | `edgequake_webui/e2e/spec180/access-live.spec.ts` | Real auth, upload/edit/read, tagged `@spec180live` |
| Security retrieval | `edgequake/crates/edgequake-api/tests/spec180_access_retrieval.rs` | Query/graph/context/MCP/history revocation |

## Runner integration and commands

The current [Playwright config](../../edgequake_webui/playwright.config.ts) has an explicit `mock-api` grep through @spec179; new tests tagged @spec180 would not run there today. WP-07 MUST add @spec180 to mock-api selection and exclude it from the live/default project. Use a distinct live project/grep for @spec180live (avoid prefix regex accidentally matching both). Add explicit project patterns rather than assuming a filename runs. Validate with `--list` before running.

Future implementation verification, from repository root:

```bash
cd edgequake
cargo test -p edgequake-core --lib clearance
cargo test -p edgequake-api --features postgres --test spec180_access_contract
cargo test -p edgequake-api --features postgres --test spec180_access_concurrency
cargo test -p edgequake-api --features postgres --test spec180_access_admission
cargo test -p edgequake-api --features postgres --test spec180_access_retrieval
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
```

From `edgequake_webui`, after planned test files/config changes exist:

```bash
pnpm exec vitest run src/lib/clearance/__tests__/access-draft.test.ts src/components/clearance/__tests__/document-access-editor.test.tsx
pnpm exec playwright test --project=mock-api --grep '@spec180\b' --list
pnpm exec playwright test --project=mock-api --grep '@spec180\b'
pnpm run typecheck
pnpm run lint
```

For live E2E, use the repository [makefile dev workflow](../../.github/skills/makefile-dev-workflow/SKILL.md) with a dedicated disposable test database and mock providers, enable authentication and clearance, and start the fixture backend/frontend explicitly. The proposed live project must target those exact URLs. Run live browser tests with `E2E_LIVE_STACK=1` and required live-fixture checks; never silently run them against an unrelated developer database. There is no reason to incur live LLM cost for label workflow acceptance.

Use role/name locators and retrying assertions as [Playwright recommends](https://playwright.dev/docs/best-practices). Negative assertions must inspect actual HTTP bodies, downloads, captured model prompts, database rows and emitted frames; absence from the DOM alone is insufficient.

## Release evidence bundle

Attach to each implementation PR: requirement/test IDs; exact commands and versions; passed/failed/skipped counts; seed manifest; database role; migrations; before/after screenshots for all nine wireframes in light/dark and small viewport; axe plus manual keyboard/AT results; race/fault traces; p50/p95 performance results; cross-channel forbidden-canary checks. Scrub sensitive fixtures and credentials. A failed disclosure test blocks rollout.

## Assessment validation, distinct from future acceptance

During authoring, the existing `src/lib/clearance/rules.test.ts` was executed with the installed Vitest runner: seven tests passed. The initial pnpm wrapper could not verify its pinned executable because registry access failed; direct `node node_modules/vitest/vitest.mjs` avoided any install. This is regression evidence for existing rule formatting only. It is not proof of the proposed workflows or live RLS behavior. Documentation links, identifier coverage, ASCII grids and Mermaid syntax are validated separately before completion.

The attempted `cargo test -p edgequake-core --lib clearance --offline` did not run tests: Cargo was configured to write `/Volumes/OWCSSD/builds/debug/.cargo-lock`, outside the writable sandbox, and returned Operation not permitted. No Rust test pass is claimed. No application source or schema was changed by this documentation task.

Authoring checks: [the standard-library validator](scripts/validate.py) checks local links/anchors, canonical IDs, traceability, JSON examples and ASCII grids. All six Mermaid blocks also parsed successfully using the installed Mermaid 11.15.0 package. Re-run after edits:

```bash
python3 specs/180-improve-sec-doc/scripts/validate.py
git diff --check
```
