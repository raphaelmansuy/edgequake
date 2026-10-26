# SPEC-180 live security evidence

[Specification](../README.md) · [Security boundaries](../10-security-and-sequences.md) · [Acceptance matrix](../12-e2e-test-matrix.md) · [Database contract](../08-data-and-db-contract.md) · [Results](results.json) · [Checks](checks.json) · [Screenshots](screenshots) · [Capture manifest](screenshot-manifest.json)

The suite uses real password authentication, a real user-owned MCP API key, HTTP requests, the Rust API, and PostgreSQL 18 with forced RLS. Authorization and persistence responses are not mocked. The lost-response browser case deliberately drops a response after the real server commits, then reconciles the real receipt.

## Isolated environment

Use only a disposable database named `edgequake_spec180`. The seed and live Rust tests reject a different database name. The fixture password in the test source is public, test-only data; the Argon2id fixture hash was generated independently and was not copied from a user's account. Never seed these personas into a production database.

The development run uses container `edgequake-spec180-postgres`, PostgreSQL on `127.0.0.1:15480`, API on `18480`, and Next.js on `3040`. Keep these separate from the normal development stack. The isolated API, frontend and container were stopped after validation; test data is retained. Restart the retained database with `docker start edgequake-spec180-postgres`. The database image is `ghcr.io/raphaelmansuy/edgequake-postgres:latest`; production acceptance should pin its digest.

From the repository root, after provisioning that disposable PostgreSQL instance:

```sh
cd edgequake
cargo build --offline
DATABASE_URL=postgres://edgequake:spec180_test_only@127.0.0.1:15480/edgequake_spec180 \
  cargo run --offline -- migrate
cd ..
docker exec -i edgequake-spec180-postgres psql -U edgequake -d edgequake_spec180 \
  < specs/180-improve-sec-doc/e2e/seed.sql
```

Start the built API with `PORT=18480`, the same `DATABASE_URL`, `EDGEQUAKE_AUTH_ENABLED=true`, `EDGEQUAKE_DEV_MODE=false`, a nondefault test `JWT_SECRET`, and `EDGEQUAKE_CORS_ORIGINS=http://127.0.0.1:3040,http://localhost:3040`. Set LLM/embedding providers to Ollama and `OLLAMA_HOST=http://127.0.0.1:1`; explicitly unset cloud provider credentials. The intentionally unavailable extraction provider keeps this security test independent of cloud credentials or generation quality. Set `ALLOW_REGISTRATION=false` for the isolated run if provisioning through registration is unnecessary.

Start the frontend from `edgequake_webui`:

```sh
EDGEQUAKE_NEXT_DIST_DIR=.next-spec180 NEXT_PUBLIC_AUTH_ENABLED=true \
NEXT_PUBLIC_DISABLE_DEMO_LOGIN=true EDGEQUAKE_API_URL=http://127.0.0.1:18480 \
  node node_modules/next/dist/bin/next dev -p 3040
```

Run browser/API tests serially; fixture resets and intentional epoch changes must not race another suite:

```sh
cd edgequake_webui
node node_modules/@playwright/test/cli.js test -c e2e/spec180/playwright.config.ts
node node_modules/vitest/vitest.mjs run src/lib/clearance/__tests__ \
  src/lib/upload/__tests__/upload-scope-guard.test.ts \
  src/lib/upload/__tests__/perform-file-upload.test.ts
```

After the browser suite has finished, run the live Rust response-fence tests:

```sh
cd edgequake
SPEC180_DATABASE_URL=postgres://edgequake:spec180_test_only@127.0.0.1:15480/edgequake_spec180 \
  cargo test -p edgequake-api --features postgres --offline --lib \
  services::clearance::fence_tests -- --include-ignored --test-threads=1
SPEC180_DATABASE_URL=postgres://edgequake:spec180_test_only@127.0.0.1:15480/edgequake_spec180 \
  cargo test -p edgequake-api --features postgres --offline --lib \
  services::document_access::tests -- --include-ignored --test-threads=1
```

## What is proven by the tests

| Boundary | Executable evidence |
|---|---|
| RBAC intersects clearance; read-only stewards cannot write | `access-api.spec.ts`, read-only browser case |
| Hidden and missing documents have the same 404 behavior; hidden content, originals, chunks, status, lineage, PDF metadata/download/list entries and MCP catalog entries are absent | `access-api.spec.ts`, `access-security.spec.ts` |
| Actual application DB role cannot read a hidden row | `SET LOCAL ROLE edgequake_tenant_access` test, not a superuser-only assertion |
| Exact labels are committed before extraction on JSON, multipart, PDF and MCP admission | transport cases, immediate parent/PDF assertions, real UI upload |
| Omitted labels stay uploader-private; malformed or reserved metadata cannot bypass admission | malformed/null/partial and omission cases |
| A duplicate's different label intent cannot silently relabel it | JSON and PDF deduplication cases |
| Optimistic versions, epochs, reasons and steward checks are authoritative | precondition, broadening, concurrent-edit and revoked-membership cases |
| Label, child, audit, epoch and receipt roll back together | injected audit failure and stale bulk cases |
| Late worker labels derive from current parent, including after relabel | direct stale chunk update plus guards installed on all seven protected parent tables |
| Operation receipts cannot be read by another actor; expanded bulk intents replay across the epoch change they committed | cross-actor status case; replay and lost-response reconciliation cases |
| A revoked stream cannot emit another body chunk; asynchronous MCP tools bind their own fresh clearance context and recheck read outcomes | live PostgreSQL response-fence test and real HTTP streaming test with membership revocation |
| UI interactions remain explicit and accessible | Save/Cancel/discard, nested Escape, draft retention, member check, focused WCAG checks, 1440/768/375 screenshots |

The full Playwright report is regenerated by the command above. Screenshots are saved by the tests in this directory and inspected as part of delivery. Focused accessibility checks include the access dialog; they do not claim that every unrelated application screen is conformant.

## Scope and limits

This is evidence for the listed threat scenarios, not a mathematical proof of all possible attacks. It does not validate successful LLM extraction, live object-store outage recovery, production SSO deployment, every retrieval algorithm, a penetration test, or the large-corpus latency targets in the specification. The offline-model warning visible behind some dialogs truthfully reflects the test environment. Existing protected-parent/task reconciliation is reused; cross-storage admission is not claimed to be a distributed ACID transaction.

Clean database migration was also exercised separately through migration 225, including forced RLS, the actor/workspace receipt key, parent preservation and child inheritance functions. The new migration is pinned in `checksums.lock`; no previously deployed migration was rewritten.

Comparative market leadership and usability-study targets remain hypotheses under [the benchmark protocol](../14-research-and-benchmark.md). No claim of universal superiority or defect-free security follows from these tests.

## Design decisions found through live tests

The live tests found and corrected three related channel gaps: PDF-ID routes used workspace checks without a label check, progress/task metadata needed document visibility checks, and spawned MCP tasks did not inherit the request task-local clearance session. PDF-ID routes now guard before metadata/bytes lookup; PDF and Runs list totals/pages use a single RLS transaction. Shared task lookup now checks the canonical parent identity before details, progress, retry or cancellation. MCP tasks bind a newly compiled session, use the same document inventory predicate as REST, and recheck read outcomes before returning content. MCP workspace stats count visible documents/chunks and explicitly mark graph totals unavailable rather than disclose unscoped aggregate values.

Visual review covered every saved capture through a contact sheet, with individual inspection of the mobile dark editor, upload overrides, bulk selection, member check and failure recovery states. The inspected dialogs contain no clipped form fields or overlapping actions at the tested sizes. Desktop, tablet and mobile use the existing semantic design tokens and focus indicators, informed by the supplied gr8r-studio reference.

## Final executed checks

The final report contains **42 passing live API/browser tests**, zero skipped, zero flaky and zero unexpected failures. A further 49 focused unit, compatibility and boundary tests passed, including the real HTTP revocation fence and enforcement-off legacy admission. All 21 final screenshots were visually inspected.

Rust build, formatting, strict Clippy across API/storage/tasks and all their targets, release TypeScript, shared UI lint, documentation links/diagrams and diff checks passed. Every changed frontend source file was compared with its baseline lint diagnostics: no new diagnostics remain. Existing repository lint debt is retained explicitly in [the comparison](frontend-lint-baseline.json), rather than misreported as a clean whole-repository lint run.
