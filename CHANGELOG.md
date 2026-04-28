# Changelog

All notable changes to this project will be documented in this file.

## [0.11.1] - 2026-04-28

### Fixed

- **CRITICAL — Migration checksum mismatch on 0.10.12→0.11.0 upgrade** ([#195](https://github.com/raphaelmansuy/edgequake/issues/195)):
  `001_init_database.sql` was inadvertently mutated in v0.11.0 (commit `e91108df`):
  `SET search_path = public` was changed to `SET LOCAL search_path = public` and
  4 comment lines were added, changing the SHA-384 checksum stored in `_sqlx_migrations`.
  Any persistent PostgreSQL database that ran 0.10.12 migrations had the original
  checksum stored; the v0.11.0 binary found a mismatch and refused to start with:
  `migration 1 was previously applied but has been modified`.
  Fix: restored `001_init_database.sql` to its byte-exact pre-v0.11.0 content.
  The `SET LOCAL` concern is already addressed at the `DATABASE_URL` connection level
  (`?options=-c%20search_path%3Dpublic`). No data loss. No manual DB intervention required.
  Verified: `sha384sum` of the restored file matches `bb40c61f…` — the value stored
  in all 0.10.12 production databases.

### Upgrade notes

Users on 0.10.12 upgrading to 0.11.0 who hit the startup failure should upgrade
directly to 0.11.1. No database changes required — just update the container image.
Users who already applied the workaround (manual checksum update) are also safe
to upgrade; the fix is idempotent.

## [0.11.0] - 2026-04-27

### Added — Mistral as First-Class LLM Provider

- **edgequake-llm upgraded to v0.6.14** — the external `edgequake-llm` crate now ships full Mistral La Plateforme support including chat, vision (`pixtral-large-latest`), and embeddings (`mistral-embed`, 1024 dims).
- **Mistral provider fully integrated at every layer:**
  - `models.toml` — 11 new Mistral model cards (chat, vision, embedding families).
  - `workspace.rs` — `default_embedding_model_for_provider("mistral")` returns `"mistral-embed"`; `known_embedding_dimension` maps all Mistral embedding model variants to 1024.
  - `pdf_upload/types.rs` — `default_vision_model_for_provider("mistral")` returns `"pixtral-large-latest"` for PDF ingestion.
  - `safety_limits.rs` — Mistral model family detection added; `"mistral-small-latest"` set as safe default.
  - `state/provider_setup.rs` — provider-aware embedding model and dimension resolution; `MISTRAL_API_KEY` detection; `mistral-embed` registered at 1024 dims.
- **Makefile** — `backend-bg` detects `MISTRAL_API_KEY` first (priority over OpenAI/Ollama) and exports the full Mistral environment block including `EDGEQUAKE_LLM_PROVIDER=mistral`, `EDGEQUAKE_EMBEDDING_PROVIDER=mistral`, `EDGEQUAKE_VISION_PROVIDER=mistral`, `EDGEQUAKE_VISION_MODEL=pixtral-large-latest`, `EDGEQUAKE_EMBEDDING_BATCH_SIZE=16` (critical — Mistral allows ≤16 embeddings per request).
- **`.env.example`** — Option 5 (Mistral La Plateforme) documented with all required and optional env vars.

### Fixed

- **Mistral embedding batch size limit** — Mistral API rejects requests with > 16 texts in a single embedding call (HTTP 400, code 3210). Fixed by exporting `EDGEQUAKE_EMBEDDING_BATCH_SIZE=16` in Makefile Mistral startup; `edgequake-llm`'s `embed_batched()` respects `max_batch_size()` which reads this env var.
- **Embedding model bleed-through in hybrid mode** — `OLLAMA_EMBEDDING_MODEL` env var was silently overriding Mistral embedding selection. Fixed by provider-aware env key resolution in `provider_setup.rs` (`provider_specific_embedding_env_key()`).

### Verified E2E

- Full pipeline tested with real Mistral API keys:
  - Health endpoint confirms `llm_provider_name: "mistral"`, `model: "mistral-small-latest"`, `embedding.model: "mistral-embed"`, `embedding.dimension: 1024`.
  - PDF ingestion: `AI_Services__Elitizon.pdf` → **45 entities extracted** (status: Completed).
  - PDF ingestion: `national-capitals.pdf` → **570 entities extracted** (status: Completed).
  - RAG query: "What are the main AI services and technologies described in these documents?" → **2,916 tokens, 224.5 tok/s, 2 Sources, 97 Topics, 100% confidence** using `mistral-small-latest` + `mistral-embed`.
- All 550 Rust workspace tests pass (`cargo test --workspace --lib`).
- `cargo clippy --workspace --lib -- -D warnings` passes with zero warnings.
- `cargo fmt --all -- --check` passes.

## [0.10.14] - 2026-04-27

### Fixed

- **Issue #189 & #192 — WebUI cannot reach API in any non-default deployment.** The `frontend` Docker service had no `EDGEQUAKE_API_URL` environment variable. The Next.js image bakes `http://localhost:8080` into the JS bundle at build time via `NEXT_PUBLIC_API_URL` (a Next.js build-time constant). With no runtime override, all deployments used the baked URL regardless of `EDGEQUAKE_PORT` — breaking custom-port setups (#189) and causing "Connection Error" when `apiUrl` fell back to empty (relative `/api/v1` hits Next.js with no proxy, #192).
  - **Fix 1:** `runtime-config.ts` now reads `process.env.EDGEQUAKE_API_URL` (a plain env var, read at request time by the server component) before falling back to `NEXT_PUBLIC_API_URL`. This allows the API URL to be set at container startup without rebuilding the image.
  - **Fix 2:** `docker-compose.quickstart.yml` frontend service now passes `EDGEQUAKE_API_URL: http://localhost:${EDGEQUAKE_PORT:-8080}`, so custom port deployments work automatically.
  - **Fix 3:** `websocket-manager.ts` SSR fallback changed from hardcoded `ws://localhost:8080/ws/pipeline/progress` to the relative path `/ws/pipeline/progress`, eliminating the port-8080 assumption in SSR context.

### Verified

- TypeScript compile check: `tsc --noEmit --strict` passes (exit 0).
- 31/31 WebSocket client tests pass unchanged.
- Logic proof: `EDGEQUAKE_PORT=8081` scenario correctly injects `http://localhost:8081` into `window.__EDGEQUAKE_RUNTIME_CONFIG__` at runtime.
- `window.__EDGEQUAKE_RUNTIME_CONFIG__` override (highest priority) still works; `NEXT_PUBLIC_API_URL` (local dev `.env.local`) still works as fallback.

## [0.10.13] - 2026-04-27

### Fixed

- **Bug 1 (CRITICAL) — Migration 019 checksum mismatch on v0.10.1 → v0.10.12 upgrade.** Commit `6f3d0204` modified `019_add_tenant_workspace_to_tasks.sql` after it had been deployed in v0.10.1, adding `ALTER TABLE tasks ALTER COLUMN tenant_id/workspace_id SET DEFAULT` statements that changed the SHA-384 checksum. Any database that had run v0.10.1 stored the old checksum; the v0.10.12 binary shipped the mutated file, causing sqlx to abort at startup with *"migration 19 was previously applied but has been modified"*. The file has been restored to its byte-for-byte v0.10.1 content. The DEFAULT-value logic correctly lives in the separately-created migration 035.
- **Bug 2 — sqlx-cli schema ambiguity causing duplicate-key error on every restart.** When the PostgreSQL user is named `edgequake`, the default search_path `"$user",public` resolves to `edgequake,public` once migration 001 creates the `edgequake` schema. On a second `sqlx migrate run` invocation, sqlx-cli resolved `_sqlx_migrations` to the `edgequake` schema (empty), then migration 001's session-level `SET search_path = public` redirected tracking writes back to `public._sqlx_migrations` (already fully populated), producing a duplicate-key constraint violation. Fixed by: (1) adding `?options=-c%20search_path%3Dpublic` to `DEFAULT_DATABASE_URL` in the Makefile, `.env`, and `.env.example` so sqlx-cli always connects with `search_path=public`; (2) changing `SET search_path = public` to `SET LOCAL search_path = public` in migration 001 so the path override is transaction-scoped, not session-scoped. The compiled application binary was not affected (its connection pool already sets `search_path TO public` via an `after_connect` hook).

### Documentation

- Added `specs/fix-migration-db-issue/` documentation suite: root cause analysis with SHA-384 proof, step-by-step reproduction guide (Docker-based), prevention playbook with golden rules for migration immutability, and a runnable Python checksum proof script.

### Verified

- Restored migration 019: `sha384sum` output matches the golden v0.10.1 checksum (`1f538faa…`).
- Two-run proof on a fresh Docker PostgreSQL container: first `sqlx migrate run` applies all 35 migrations (exit 0); second run exits 0 silently with no duplicate-key error and no `edgequake._sqlx_migrations` ghost table.

## [0.10.12] - 2026-04-19

### Fixed

- Restored localhost:3000 as the default WebUI URL for `make dev` and `make dev-bg`, matching the documented local startup path and preventing confusion when developers opened the UI on the wrong port.
- Improved the startup messaging so when the UI must move to a different free port, the Makefile now prints the exact browser URL to use for that session.

### Verified

- Reproduced the local startup regression, restarted the stack with the fix, and confirmed via live browser clicks that the dashboard and documents flow load cleanly on localhost:3000 with no console connection errors.

## [0.10.11] - 2026-04-19

### Changed

- Canonical clean main-branch release published after the squash merge of PR #183 so the default branch and published Docker artifacts stay aligned.

### Verified

- Re-ran the published Docker validation plan for both OpenAI and local Ollama or Gemma flows from the clean release baseline.

## [0.10.10] - 2026-04-19

### Fixed

- Published Docker quickstart validation for local Ollama or Gemma now reports provider availability correctly by probing the real HTTP health endpoints instead of relying on brittle socket-level host resolution.
- Added a regression test covering the Ollama health endpoint path so Docker-hosted local model setups stay verifiable across future releases.

### Verified

- Revalidated the public quickstart flow against both OpenAI and local Ollama or Gemma setups after the fix.

## [0.10.9] - 2026-04-19

### Fixed

- Local startup is now deterministic and matches the documentation. make dev always runs in unauthenticated local mode, while make dev-auth explicitly enables backend and frontend authentication flags.
- The main Rust server entrypoint now uses concise contextual error propagation instead of boxed top-level errors and oversized fatal-print blocks.
- Frontend workspace tooling now consistently uses bun or pnpm for local commands, avoiding npm lockfile drift in the main WebUI workspace.
- Workspace release metadata now points at the correct repository origin and matches the new fix release.

### Verified

- Fresh backend auth regression suite: 28 passed, 0 failed.
- Fresh frontend production build completed successfully.
- Live browser proof confirmed protected dashboard routes redirect to the login screen when started with make dev-auth.

## [0.10.8] - 2026-04-19

### Fixed

- **Issue #180 — runtime auth and deployment hardening is now fail-closed.** Prebuilt WebUI deployments no longer depend exclusively on build-time `NEXT_PUBLIC_*` variables; runtime API/auth values are injected centrally so one image can move safely across environments.

- **Protected dashboard routes are now actually protected.** When authentication is enabled or demo login is intentionally disabled, direct navigation to pages like Graph, Documents, Pipeline, Query, Workspace, Costs, Knowledge, API Explorer, and Settings redirects to the login screen instead of rendering the dashboard shell.

- **Sensitive auth endpoints now enforce a single runtime source of truth.** Environment-based auth flags, bootstrap master API keys, public registration policy, and API-key creation rules are now applied consistently across middleware and handlers.

- **Settings and provider panels now use the shared authenticated client.** This removes duplicated fetch logic, fixes authenticated-mode 401 noise, and keeps the frontend aligned with DRY/SOLID principles.

### Changed

- Added a runtime-config helper, shared auth guard, and shared request-auth helpers to make the deployment/auth model explicit and auditable.
- Added backend auth regressions and verified the fix with fresh Rust E2E checks plus live browser validation in both unauthenticated and authenticated modes.
- Added implementation notes in `specs/issue-180-runtime-auth-hardening.md` and deployment guidance in `docs/operations/runtime-auth-hardening.md`.

## [0.10.7] - 2026-04-19

### Fixed

- **Default-workspace uploads and queries now use the same canonical workspace identity.** The legacy default alias is resolved through shared helpers across middleware, ingestion, pipeline creation, and vector storage so browser uploads no longer drift into the wrong embedding configuration.

- **Workspace-scoped document ingestion no longer hits hidden embedding-dimension mismatches.** File and text uploads now run through the same workspace-aware pipeline that later serves queries, preventing the 768-vs-1536 split that previously produced empty answers in the UI.

- **Query verification remains deterministic in mock-backed environments.** Mock keyword extraction now uses a rule-based path instead of brittle fallback parsing, keeping release validation stable and non-flaky.

- **The query UI self-heals stale provider selections.** Persisted local model choices are sanitized against live provider health so invalid environments cleanly fall back to the server default.

### Changed

- Added shared default tenant/workspace UUID resolvers and regression tests to keep the multitenant contract DRY and auditable.
- Verified the release end to end with fresh workspace integration tests, local stack health checks, and a live browser proof returning EQ-10642.

## [0.10.6] - 2026-04-19

### Fixed

- **Deleted documents no longer come back after restart.** Single-document delete, bulk delete, and workspace delete now purge persisted task rows and in-flight progress state before removing document data, preventing stale recovery jobs from resurrecting work the user already removed.

- **Destructive and recovery flows now stay inside the active workspace.** Delete, bulk clear, stuck-recovery, and reprocess handlers now re-check document workspace ownership before acting, so cross-workspace scans cannot accidentally touch unrelated data.

- **Large PDF ingestion is more memory-stable under local and cloud vision providers.** The pipeline now computes a safe per-file resource profile, lowers DPI/concurrency for heavy PDFs, and drops raw PDF bytes as soon as conversion is complete to reduce spikes and flaky retries.

- **PDF retries now resume safely from saved checkpoints.** Reprocessing and recovery flows keep the existing document and continue conversion progress instead of restarting from page 1 unless a full reset is explicitly requested.

- **Vision PDF failures now degrade gracefully to EdgeParse.** Timeout and provider-setup failures no longer leave documents stuck in a flaky failed loop when a deterministic text path is available.

- **Pipeline cancellation and destructive UI actions are more robust and accessible.** Cancel is now idempotent when the queue is already idle, destructive buttons use correct contrast, and keyboard focus stays on real app controls during browser navigation.

- **Query endpoints now fail closed on invalid workspace context.** If a client explicitly sends an invalid or nonexistent workspace ID, the API now returns a clear error instead of silently falling back to the default workspace. This closes a multi-tenant isolation gap in the query path.

- **Service health checks are gentler on Docker/OrbStack.** Development commands now prefer lightweight port probes for PostgreSQL readiness and skip repeated Docker exec retries when the daemon is unavailable, reducing local environment instability during E2E work.

- **Local development now avoids interfering with other stacks.** `make dev` and `make dev-bg` now use listener-only checks, prefer the UI on port 3001, and automatically shift the API/UI to the next safe free ports when 8080 or 3001 are already in use.

- **Release branches stay clean during E2E verification.** Generated screenshot and test artifact folders are now ignored so publication commits and PRs are not polluted by transient PNG output.

### Changed

- Added explicit WHY-comments around task lifecycle cleanup, workspace-scope enforcement, and development-overlay accessibility so the reliability invariants are easy to audit and maintain.
- Verified the hardening with fresh Rust E2E coverage (`78 passed`) plus Playwright browser regression coverage (`16 passed, 2 skipped`).
- Aligned release metadata and publish defaults to `0.10.6` across the workspace, frontend package, quickstart surfaces, and Docker workflow defaults.
- Regenerated `Cargo.lock` so Docker publication with `cargo build --release --locked` stays deterministic and succeeds on tagged releases.
- Verified the release path with fresh build, health, and Docker-oriented publication checks.

## [0.10.3] - 2026-04-17

### Fixed

- **Entity merge UX is now searchable and accessible for large graphs.** The merge dialog now uses a ranked search combobox so users can quickly find the correct canonical entity even when the workspace contains many concepts and people.

- **Human-readable entity merges now resolve correctly in the backend.** Merge requests using labels such as person and organization names are now resolved robustly instead of depending on brittle transient IDs or over-normalized keys.

- **Graph merge semantics are preserved during deduplication.** When duplicate entities are merged, relationships are rewired onto the canonical node while preserving provenance and relation meaning through merged metadata.

- **Provider/model diagnostics remain actionable and safe.** The effective configuration chain and compatibility checks now make it much easier to understand and fix mismatched vision or LLM settings.

### Changed

- Verified release dependency alignment with published crates: `edgequake-llm` v0.6.2 and `edgequake-pdf2md` v0.8.1.
- Release metadata, quickstart examples, and pinned version references are now aligned to `0.10.3`.

## [0.10.2] - 2026-04-16

### Fixed

- **Ollama Cloud returns 401 Unauthorized (#162).** Upgraded to `edgequake-llm`
  v0.6.0 which adds `OLLAMA_API_KEY` environment variable for Bearer token
  authentication with Ollama Cloud and authenticated Ollama endpoints.

- **Embedding deserialization fails with HuggingFace TEI (#164).** Upgraded to
  `edgequake-llm` v0.6.0 which replaces async-openai's strict embedding types
  with lenient HTTP deserialization, supporting HuggingFace TEI, Infinity,
  FastEmbed, and other OpenAI-compatible embedding servers.

- **Embedding batch size not configurable (#165).** Upgraded to `edgequake-llm`
  v0.6.0 and switched pipeline helpers to `embed_batched()`. Large embedding
  requests are now auto-chunked per `EDGEQUAKE_EMBEDDING_BATCH_SIZE` (default
  2048), preventing 422 errors from servers with batch limits.

### Changed

- Upgraded `edgequake-llm` from 0.5.1 to 0.6.0.
- Upgraded `edgequake-pdf2md` from 0.8.0 to 0.8.1 (diamond dependency resolution).
- Pipeline embedding helpers (`helpers.rs`) now use `embed_batched()` for all
  chunk, entity, and relationship embeddings.
- `SafetyLimitedEmbeddingProviderWrapper` forwards `max_batch_size()`.

## [0.10.1] - 2026-04-11

### Fixed

- **Release candidate promoted to a fully green publication**

  The exact GitHub CI formatting gate (`cargo fmt --all -- --check`) caught two
  Rust files that had not been normalized with the same invocation used in CI.
  The official publish tag now advances to `0.10.1`, ensuring the default
  branch checks, release workflow, and published install surface all describe
  the same green release.

## [0.10.0] - 2026-04-11

### Fixed

- **Sigma graph viewer performance hardening**

  The knowledge-graph UI was doing redundant heavy work in the browser:

  1. Layout changes could rebuild the entire Sigma instance instead of only
     animating node positions.
  2. Selection highlighting used a perpetual 20fps pulse loop that refreshed
     the whole canvas indefinitely.
  3. Layout logic was duplicated across multiple components with inconsistent
     parameters.
  4. Hover highlighting mutated large portions of the graph instead of using
     Sigma's reducer model and scheduled refresh flow.

  **Fix applied:**

  - Extracted a shared graph layout engine with adaptive ForceAtlas2/Noverlap
    profiles and shared graph performance thresholds.
  - Extracted deterministic graph edge-key helpers so store, renderer, and
    streaming updates use the same identity rules.
  - Refactored the Sigma renderer to keep the instance alive across layout
    changes and to use render-time reducers for hover/selection emphasis.
  - Replaced eager refresh calls with scheduled refreshes on the external-state
    paths where Sigma recommends debouncing.
  - Removed the perpetual selected-node pulse loop.

- **Mermaid label sanitizer no longer strips angle-bracket labels**

  The Mermaid fallback sanitizer treated `A[a<b>c]` like HTML and collapsed the
  label to `ac`, which broke the existing frontend unit suite. HTML stripping is
  now limited to the real tag shapes we need to neutralize, so Mermaid labels
  containing literal angle brackets remain intact.

- **Frontend publication lint gate aligned with shipped surface area**

  The default `edgequake_webui` lint command now ignores exploratory Playwright
  specs under `e2e/` and enforces a clean gate over shipped application code and
  committed unit-test support files. This keeps the release signal focused on
  publishable artifacts while leaving audit scripts available for manual use.

- **Embedding error: "input length exceeds context length" for scientific PDFs (Ollama)**

  Scientific papers with dense tables, gene IDs, p-values, and numeric data have an actual
  tokenizer density of ~2 chars/true-token — roughly 2× worse than the chunker's 4 chars/token
  assumption. A 1200-estimated-token chunk (4800 chars) therefore becomes ~2400 true tokens,
  exceeding `embeddinggemma`'s hard 2048-token limit and causing a 400 Bad Request error that
  aborted the entire document ingestion.

  **Three-layer fix applied (defense in depth):**

  1. **Reduce default `chunk_size` 1200 → 800** (`ChunkerConfig::default`).
     At 2 chars/true-token: 800 × 4 = 3200 chars → 1600 true tokens, comfortably within 2048
     (80% margin). Prior value produced 4800-char chunks → 2400 true tokens → 400 error.

  2. **Adaptive cap in `Pipeline::with_embedding_provider`** (`pipeline/mod.rs`).
     When the embedding provider reports `max_tokens > 0`, the pipeline caps `chunk_size` to
     `max_tokens / 2`, accounting for worst-case tokenizer density divergence. For embeddinggemma
     (2048 tokens): cap = 1024 est-tokens → 4096 chars.

  3. **Pre-embedding truncation guard** (`pipeline/helpers.rs`).
     Before every `embed()` call (chunks, entities, relationships), texts are truncated to
     `max_tokens × 2.5 × 0.85` chars and a WARNING is logged. This catches edge cases where
     the chunker cannot split (e.g., an entire 10 000-char markdown table with no sentence
     boundaries) and prevents hard 400 failures. A partial embedding is more useful than an
     aborted pipeline.

- **Large PDF ingestion fails with local Ollama models (timeout loop circuit-breaker) — Issue #90**

  `SafetyLimitedProviderWrapper` hard-capped every per-page LLM call at `MAXIMUM_TIMEOUT_SECS`
  (600 s) even for local providers. A 120-page document at 30 s/page needs ≥ 3 600 s but hit
  a 660 s outer timeout three times, triggering the circuit-breaker.

  - Added `create_safe_vision_provider()` with provider-aware per-page timeout:
    600 s/page for Ollama/LM Studio, 120 s/page for cloud APIs.
  - Fixed outer vision-conversion timeout formula: `120 + (page_count × secs_per_page)`.
    Replaces the old `max(60 + pages×5, 600)` which assumed cloud API speed.
  - Fixed default OCR concurrency for local providers: now 2 (was 8), preventing VRAM thrashing
    on single-GPU inference.

### Added

- **Embedding context safety** — new private helpers in `edgequake-pipeline`:
  - `embed_max_chars(max_tokens)` — converts a provider's token limit to a safe char cap
  - `guard_for_embedding(texts, max_chars)` — truncates and logs oversized inputs
  - `truncate_at_char_boundary(s, max_bytes)` — UTF-8-safe truncation utility

- **New environment variables for tuning large-PDF local inference:**
  - `EDGEQUAKE_PDF_SECS_PER_PAGE` — estimated seconds per page (default: 30 local / 8 cloud)
  - `EDGEQUAKE_VISION_PAGE_TIMEOUT_SECS` — per-page LLM call timeout (default: 600 s local /
    120 s cloud)
  - `EDGEQUAKE_PDF_CONCURRENCY` — parallel page workers (default: 2 local)

### Changed

- **Release metadata and quickstart surfaces aligned to `0.10.0`**

  The workspace version, frontend package metadata, pinned Docker examples, and
  release workflow defaults now all point to the same official release number.
  This removes version drift between the changelog, the tag-triggered GHCR
  publication flow, and the `curl | sh` quickstart path.

## [0.9.18] - 2026-04-09

### Fixed

- **Issue #144 fully closed: prebuilt Docker path, env compatibility aliases, and OSS onboarding**.

  **Implementation**
  1. Added deterministic compatibility aliases for LightRAG-style environment names:
     `MODEL_PROVIDER`, `CHAT_MODEL`, `EMBEDDING_PROVIDER`, `EMBEDDING_MODEL`,
     `EMBEDDING_DIMENSION`.
  2. Aliases are normalized into canonical `EDGEQUAKE_*` variables at startup and in app-state
     constructors, so the rest of the provider stack keeps a single code path.
  3. Workspace default resolution now honors the same aliases directly with explicit precedence:
     `EDGEQUAKE_DEFAULT_*` → `EDGEQUAKE_*` → compatibility alias → compiled default.
  4. Added unit and E2E coverage for the alias path to prevent regressions and empty-string /
     Docker Compose edge cases.

  **Docker / release**
  1. `edgequake/docker/docker-compose.prebuilt.yml` now pulls the published
     `ghcr.io/raphaelmansuy/edgequake-postgres` image instead of building PostgreSQL locally.
  2. The documented prebuilt path is now truly all-prebuilt: API, frontend, and PostgreSQL are
     all versioned GHCR images.

  **Documentation**
  1. Rewrote `CONTRIBUTING.md` to standard open-source contribution flow.
  2. Updated Docker and configuration docs with the compatibility alias mapping and the versioned
     prebuilt-image workflow.
  3. Updated the LightRAG migration guide with alias examples and explicit tenant/workspace
     guidance: for 1,000 businesses, use 1,000 tenants and then add workspaces inside each tenant.
  4. Normalized project-owned release metadata and SDK manifests to Apache-2.0 only so published
     artifacts match the repository `LICENSE`.

## [0.9.17] - 2026-04-09

### Fixed

- **Vision model sent as empty string to Ollama — `{"error":"model is required"}`** — After
  the v0.9.16 fix, vision provider correctly resolved to Ollama, but Ollama rejected every
  page with a 400 error because the model name was an empty string.

  **Root cause:** Docker Compose `${VAR:-}` maps an unset host variable to the literal empty
  string `""` inside the container. `std::env::var("EDGEQUAKE_VISION_MODEL")` returns `Ok("")`
  for that case — not an error — so every `.or_else(|_| ...)` fallback chain silently short-
  circuits and the caller receives `""` as the model. Ollama then gets
  `{"model":"","messages":[...]}` and responds with `400 model is required`.

  **First-Principle fix — defence in depth across all resolution layers:**

  1. **`types.rs` — `default_vision_model_for_provider()`:** Switched from
     `std::env::var(X).or_else(|_| std::env::var(Y))` to
     `std::env::var(X).ok().filter(|s| !s.is_empty()).or_else(|| ...)` so empty strings are
     treated identically to unset variables. This is the single source of truth for vision
     model defaults.
  2. **`types.rs` — `vision_model()`:** Added `.filter(|s| !s.is_empty())` before the
     `unwrap_or_else` so an empty `self.vision_model` falls through to the provider default.
  3. **`types.rs` — `resolved_vision_provider()`:** Same empty-string filter applied so an
     explicitly empty vision provider also falls through to `EDGEQUAKE_LLM_PROVIDER` / "ollama".
  4. **`pdf_processing.rs` safety-net:** Added `.filter(|s| !s.is_empty())` on `data.vision_model`
     before the `unwrap_or_else` branch so a task stored with `""` still resolves correctly.
  5. **`reprocess.rs` — PDF retry path:** Changed env var reads to use
     `.ok().filter(|s| !s.is_empty()).or_else(|| ...)` for both `vision_provider` and
     `vision_model`.

  **Infrastructure fix — docker-compose.quickstart.yml:**
  - Changed `EDGEQUAKE_VISION_PROVIDER: ${EDGEQUAKE_VISION_PROVIDER:-}` to
    `${EDGEQUAKE_VISION_PROVIDER:-${EDGEQUAKE_LLM_PROVIDER:-ollama}}` so the container always
    receives a non-empty provider derived from the main LLM setting.
  - Changed `EDGEQUAKE_VISION_MODEL: ${EDGEQUAKE_VISION_MODEL:-}` to
    `${EDGEQUAKE_VISION_MODEL:-${EDGEQUAKE_LLM_MODEL:-}}` so Ollama users automatically get
    their configured model (e.g. `gemma4:e4b`) for vision without any extra configuration.

  **Invariant now enforced end-to-end:**
  ```
  vision_model = EDGEQUAKE_VISION_MODEL (non-empty)
             ?? EDGEQUAKE_LLM_MODEL     (non-empty)
             ?? "gemma3:latest"         (Ollama) / "gpt-4.1-nano" (OpenAI)
  ```

## [0.9.16] - 2026-04-09

### Fixed

- **Vision LLM hardcoded to OpenAI even when Ollama is selected** — PDF → Markdown
  extraction (vision LLM) was always attempting to create an OpenAI provider regardless of
  which provider the user configured. Root cause: `resolved_vision_provider()` returned a
  hardcoded `"openai"` when no explicit `vision_llm_provider` workspace setting existed.

  **First-Principle fix across all five code paths:**
  1. `types.rs` — `resolved_vision_provider()` now reads `EDGEQUAKE_LLM_PROVIDER` env var as
     fallback (default `"ollama"`) instead of hardcoding `"openai"`.
  2. `upload.rs` — When workspace has no `vision_llm_provider`, falls back to
     `workspace.llm_provider` (the main LLM) rather than the hardcoded default.
  3. `helpers.rs` — Passes `vision_model: Some(options.vision_model())` so the
     provider-specific default model is always stored in the task (no surprise fallback at
     execution time).
  4. `pdf_processing.rs` — Uses `default_vision_model_for_provider()` for the rare case
     where `vision_model` is None at processing time (safety net).
  5. `reprocess.rs` / `bulk_ops/mod.rs` — Retry/rebuild paths also fall back to
     `EDGEQUAKE_LLM_PROVIDER` / `workspace.llm_provider` instead of hardcoded strings.

  **Infrastructure fix:**
  - `docker-compose.quickstart.yml`: added `EDGEQUAKE_VISION_PROVIDER` and
    `EDGEQUAKE_VISION_MODEL` pass-through env vars with safe empty defaults.
  - `quickstart.sh`: `start_stack()` now explicitly exports
    `EDGEQUAKE_VISION_PROVIDER="$LLM_PROVIDER"` so there is zero ambiguity — the vision
    provider always matches the selected provider unless the user overrides it.

## [0.9.15] - 2026-04-09

### Fixed

- **Ollama unreachable inside Docker — `http://localhost:11434` fails** — Docker containers
  have their own network namespace: `localhost` inside a container refers to the container
  itself, not the host machine where Ollama runs. `quickstart.sh` now auto-translates any
  loopback address (`localhost`, `127.x.x.x`) in `OLLAMA_HOST` to `host.docker.internal`
  before passing it to Docker Compose. The translation is deterministic and covers all edge
  cases:
  - `http://localhost:11434` → `http://host.docker.internal:11434`
  - `http://127.0.0.1:PORT`  → `http://host.docker.internal:PORT`
  - Custom remote hosts (e.g. `http://my-ollama-server:11434`) are passed unchanged.
  - `host.docker.internal` resolves natively on macOS/Windows (Docker Desktop) and via
    `extra_hosts: host-gateway` on Linux (already in `docker-compose.quickstart.yml`).
- **Validation feedback**: `quickstart.sh` now shows the Docker-side address when it differs
  from the host-side address, so users understand what address the container will use.
- **docker-compose.quickstart.yml**: Expanded `OLLAMA_HOST` comment to explain why
  `localhost` must not be used and document the `extra_hosts` Linux mechanism.

## [0.9.14] - 2026-04-09

### Fixed

- **Query UI default model now matches the extraction model (First Principle)** — The
  `/api/models/llm` and `/api/models` endpoints previously returned the static `models.toml`
  default (`ollama/gemma4:e4b`) as `default_model` / `default_provider`, regardless of the
  runtime-configured provider (e.g. `openai/gpt-5.4-mini` set via env vars). Both handlers now
  read `state.llm_provider.name()` and `state.llm_provider.model()` from the live `AppState`
  so the dropdown always pre-selects the same model that entity extraction uses.

## [0.9.13] - 2026-04-09

### Fixed

- **Reasoning-model token exhaustion → "0 Sources" regression (ADR-006)** — Models such as
  `gpt-5-mini` and `gpt-5-nano` are reasoning-only: they allocate their entire
  `completion_tokens` budget to internal chain-of-thought, leaving zero tokens for visible JSON
  output. The extractor received an empty string, raised "Invalid JSON: EOF while parsing a
  value", and the chunk was never stored — so **all vector tables stayed empty** and every
  document query returned "0 Sources".
  The fix is two-pronged:
  1. **quickstart.sh** now offers `gpt-5.4-mini` (default) and `gpt-5.4-nano` — models from the
     GPT-5.4 adjustable-reasoning family that support `reasoning_effort=none`.
  2. **`sota.rs` and `llm.rs`** now always pass `reasoning_effort="none"` and an explicit
     `max_tokens` cap to every entity-extraction LLM call. Non-reasoning models silently ignore
     the field; reasoning-capable models disable CoT and emit direct JSON output.
  Documented in `specs/install_script/ADR-006-reasoning-model-exclusion.md`.

- **Empty-response diagnostic message** — The "LLM returned EMPTY response" error in `sota.rs`
  now prominently names the reasoning-model root cause (reasoning_tokens = completion_tokens → 0
  net output) and recommends the correct model alternatives, speeding up operator diagnosis.

## [0.9.12] - 2026-04-09

### Added

- **Interactive setup wizard** — `quickstart.sh` is a fully redesigned step-by-step terminal
  wizard that guides users through provider selection, model choice, and validation before
  starting the stack. Replaces the heuristic-based auto-detect approach.

- **Explicit provider selection (ADR-001)** — The wizard always asks whether to use OpenAI or
  Ollama; it never auto-detects from `OPENAI_API_KEY`. The environment variable is shown as an
  informational hint only, preventing "flaky heuristic" surprises when the variable happens to be
  set for an unrelated purpose.

- **In-wizard model catalogue (ADR-003)** — Users choose from a curated, priced menu:
  - OpenAI LLM: `gpt-5.4-mini` (default), `gpt-5.4-nano`, `gpt-5.4`, `gpt-5.4-mini`
  - OpenAI Embeddings: `text-embedding-3-small` (default), `text-embedding-3-large`
  - Ollama LLM: `gemma4:e4b` (default), `gemma4:e2b`, `gemma4:26b`, `qwen2.5:latest`, `llama3.2:latest`
  - Ollama Embeddings: `embeddinggemma:latest` (default), `nomic-embed-text:latest`
  Selected models are exported to Docker Compose; no manual editing of `.env` files required.

- **Three-state volume lifecycle detection (ADR-002)** — On re-run the wizard detects:
  - Running containers → offer "Update & Reconfigure" or "Quit"
  - Stopped containers / orphaned volumes → offer "Restart & Reconfigure", "Fresh Start", or "Quit"
  - No prior installation → fresh install path
  Data volumes are explicitly listed so users know exactly what will be preserved or destroyed.

- **Irreversible fresh-start gate** — Choosing "Fresh Start" requires the user to type `DELETE`
  verbatim. Any other input cancels the destructive wipe and falls back to restart.

- **`/dev/tty` reads for `curl | sh` compatibility (ADR-004)** — All interactive input uses
  `read < /dev/tty`, keeping the wizard fully functional when the script body is piped from curl
  (stdin is the shell pipe, TTY remains available for keystrokes). When `/dev/tty` is unavailable
  (CI mode) the wizard exits immediately with env-var instructions for headless installs.

- **Premium terminal UX (ADR-005)** — Design tokens (8 semantic colors, `C_BOLD`, `C_DIM`),
  consistent component library (`ui_banner`, `ui_section`, `ui_ok/info/warn/fail`, `ui_menu`,
  `ui_confirm`), and POSIX-safe 90-second health polling with animated dots.

- **Spec documents** — Five Architecture Decision Records in `specs/install_script/` document
  every design choice, rejected alternative, edge case, and mitigation in the new wizard:
  ADR-001 (provider selection), ADR-002 (volume lifecycle), ADR-003 (model catalogue),
  ADR-004 (TTY/POSIX compatibility), ADR-005 (UX design system).

### Fixed

- **`grep -c || echo 0` double-output under `set -e`** — When container/volume counts were
  computed with `$(grep -c "…" || echo 0)`, a zero-match `grep` exited 1 and printed "0", then
  `|| echo 0` ran, producing the string `"0\n0"` and breaking `-eq 0` integer tests. Replaced
  with `| grep "…" | wc -l | tr -d ' '` which always exits 0 and emits a single clean integer.

- **`printf "…\\\n"` renders literal `\n` (no newline)** — Double-quoted backslash-newline escape
  sequences `"\\\n"` were shell-collapsed to `\n` before reaching `printf`, which then printed a
  literal two-character `\n` instead of a backslash followed by a newline. Fixed by switching to
  single-quoted format strings (`'\\\n'`) so the shell passes them unchanged to `printf`.

## [0.9.11] - 2026-04-09

### Fixed

- **"LLM error: Network error: builder error" on every document when using OpenAI** —
  Docker Compose evaluates `OPENAI_BASE_URL: ${OPENAI_BASE_URL:-}` to an empty string `""` when
  the variable is not set on the host. The OpenAI provider reads this empty string via
  `std::env::var("OPENAI_BASE_URL")` (which returns `Ok("")`) and passes it as the API base URL,
  causing every `reqwest` request to fail immediately with "builder error". The API now strips
  empty-string env vars (`OPENAI_BASE_URL`, `OPENAI_API_KEY`) at startup before any provider is
  initialised so that the provider library falls back to its built-in defaults.

- **Prior installation not detected by quickstart** — Re-running `curl | sh` on a machine that
  already has EdgeQuake installed no longer silently force-restarts containers. The script now
  detects running or stopped containers and volumes, shows a clear summary of what was found, and
  asks the user whether to update or exit. Existing data is preserved in both cases.

- **Ollama not reachable — silent failure** — When `EDGEQUAKE_LLM_PROVIDER=ollama` (the
  default), the quickstart now checks whether Ollama is reachable before starting the stack. If
  not, it prints actionable instructions (`ollama serve`, model pull command, or OpenAI switch)
  and asks the user whether to continue or abort.

### Changed

- Quickstart `_compose_env` no longer explicitly forwards `OPENAI_API_KEY` and `OPENAI_BASE_URL`
  as prefixed env var assignments (they were always forwarded as empty strings when unset). Docker
  Compose now picks them up directly from the host shell environment, which is the correct
  behaviour.
- Quickstart success message now shows which LLM provider is active and, for Ollama, reminds the
  user to pull a model (`ollama pull gemma4:latest`).
- Added `Update: sh quickstart.sh` to the management commands in the success footer.

## [0.9.10] - 2026-04-09

### Fixed

- **Empty embedding provider crashes document ingestion** — When `EDGEQUAKE_EMBEDDING_PROVIDER` (or its `DEFAULT_` variant) is set to an empty string in Docker Compose via `${VAR:-}` expansion, `std::env::var` returns `Ok("")` rather than `Err`, causing the empty string to silently override the hard-coded Ollama default. The workspace provider resolution chain now filters out empty strings at every step so that `${VAR:-}` and unset variables are treated identically. Symptoms: `"Unknown embedding provider: ''"` error on all document uploads after a fresh quickstart.

- **Existing DB workspaces retain empty embedding_provider after upgrade** — Workspaces created with an old Docker image had `embedding_provider=""` stored in their metadata JSONB column. `WorkspaceRow::into_workspace()` did not filter these empty strings, so even after the env-var fix the loaded workspace struct still carried an empty provider string. Added `.filter(|s| !s.is_empty())` before the `unwrap_or` fallback for all four provider/model fields so that empty-string DB values fall back to the env-var-aware runtime defaults.

- **Quickstart stale compose file causes silent misconfiguration on re-runs** — The quickstart script previously skipped downloading `docker-compose.quickstart.yml` when the file already existed. If a user had re-run the script after an EdgeQuake update, the old compose file (missing new env vars like `EDGEQUAKE_EMBEDDING_MODEL`) was reused silently. The script now always downloads a fresh copy, backing up the previous file to `docker-compose.quickstart.yml.bak`.

- **Quickstart ignores embedding provider when detecting OpenAI API key** — When `OPENAI_API_KEY` was set, the script configured `EDGEQUAKE_LLM_PROVIDER=openai` but left `EDGEQUAKE_EMBEDDING_PROVIDER` unset, causing the workspace to use Ollama embeddings even though no local Ollama instance was running inside Docker. The script now sets both providers to `openai` and picks sensible model defaults (`gpt-5-mini` + `text-embedding-3-small`) when an OpenAI key is detected.

- **`docker compose up -d` reuses stale containers on re-run** — Without `--force-recreate`, Docker Compose left existing containers running even when environment variables had changed (e.g. switching from Ollama to OpenAI). The quickstart now passes `--force-recreate --remove-orphans` so every run applies the current configuration.

## [0.9.8] - 2026-04-09

### Added

- **GPT-5.4 model family** — Added `gpt-5.4` (1M ctx, $2.50/$15/MTok, 128K out), `gpt-5.4-mini` (400K ctx, $0.75/$4.50, fast), and `gpt-5.4-nano` (400K ctx, $0.20/$1.25, ultra-cheap) to the OpenAI provider in `models.toml`. GPT-4 Turbo now points to `gpt-5.4` as its replacement.
- **Claude Sonnet 4.6** — Added `claude-sonnet-4-6` (1M ctx, $3/$15/MTok, 64K out) as Anthropic's latest recommended model. Fixed `claude-opus-4-6` context window from 200K → 1M tokens.
- **Gemma 4 model family (Ollama)** — Added `gemma4:latest`, `gemma4:e2b` (7.2GB, 128K ctx), `gemma4:e4b` (9.6GB, 128K ctx), `gemma4:26b` (MoE, 256K ctx), `gemma4:31b` (dense, 256K ctx). Gemma 4 models support multimodal input including audio.
- **Default model updated** — Changed default Ollama LLM from `gemma3:12b` → `gemma4:e4b` for better multimodal support and performance.

### Fixed

- **"LLM error: Network error: builder error"** — When selecting a cloud LLM provider (OpenAI, Anthropic, etc.) without the corresponding API key configured, the system now returns a clear `ConfigError` before attempting to build the HTTP client, instead of the cryptic reqwest `builder error`. The error message now instructs the user to set the required environment variable or switch to the Ollama provider.

## [0.9.7] - 2026-04-08

### Fixed

- **Document status constraint prevents `partial_failure` state** — Migration 017 (`add_processing_substates`) replaced the status constraint but dropped the wrong constraint name: it dropped `valid_document_status` while the original was named `documents_valid_status`. Both constraints coexisted, and `partial_failure` was absent from both. Result: any document that had successful entity extraction but failed entity-embedding storage would panic with `new row for relation "documents" violates check constraint "documents_valid_status"`. **Fix:** Migration 032 drops both old constraints and creates a single canonical `documents_valid_status` constraint that includes all valid status values including `partial_failure`.

## [0.9.6] - 2026-04-08

### Fixed

- **API container crashes on every restart** — `duplicate key value violates unique constraint "_sqlx_migrations_pkey"` panic on startup. Root cause: migration 001 creates the `edgequake` schema; PostgreSQL's default `"$user",public` search_path then resolves `$user="edgequake"` to that schema first. Without a fixed search_path, SQLx's `migrate!()` created a fresh empty `_sqlx_migrations` in the `edgequake` schema, saw no applied migrations, ran migration 001 (which contains `SET search_path = public`), then tried to INSERT version=1 into `public._sqlx_migrations` — which already existed from the previous install — causing a duplicate key panic on every restart. **Fix:** both the API migration pool and the storage connection pool now use `PgPoolOptions::after_connect` to pin `search_path TO public` on every connection, making `_sqlx_migrations` consistently read/written in the correct schema regardless of pool connection assignment order.

- **Spurious "Unknown embedding provider" warning on startup** — when `EDGEQUAKE_EMBEDDING_PROVIDER` is set to an empty string in Docker Compose (via `${EDGEQUAKE_EMBEDDING_PROVIDER:-}`), the code logged a noisy warning and fell through to defaults. Empty string is now treated as "not set", silencing the warning.

## [0.9.5] - 2026-04-08

### Fixed

- **Stale `edgequake.tasks` VIEW** — Migration 001 created schema views with `SELECT *` which bakes the column list at creation time. Migration 020 added `error`, `consecutive_timeout_failures`, `circuit_breaker_tripped` columns to `public.tasks` but the view was never refreshed. This caused `column "error" does not exist` errors when listing tasks. Migration 031 recreates all edgequake schema views.

- **Workspace LLM provider ignores environment variables** — When workspace metadata in the database was empty (`{}`), `WorkspaceRow::into_workspace()` fell back to hardcoded `DEFAULT_LLM_PROVIDER = "ollama"` instead of reading `EDGEQUAKE_LLM_PROVIDER` / `OPENAI_API_KEY` from environment. This caused entity extraction to fail in Docker deployments where OpenAI was the intended provider, because the pipeline tried to connect to a non-existent Ollama at `localhost:11434`. The same fix was applied to `TenantRow::into_tenant()` for consistency.

- **Stale default model names** — Updated `default_model_for_provider()` defaults: `gpt-4o-mini` → `gpt-5-mini`, `claude-3-haiku` → `claude-sonnet-4`, `gemini-1.5-flash` → `gemini-2.5-flash`, `grok-beta` → `grok-3-mini`.

## [0.9.4] - 2026-04-08

### Added

#### ⚡ One-Command Full Stack — Zero Build Time (~30 seconds)

The entire EdgeQuake stack (API, Web UI + PostgreSQL) is now available as three prebuilt multi-arch GHCR images. No Rust toolchain, no Node.js, no `cargo build` needed.

```bash
# Clone and start everything
git clone https://github.com/raphaelmansuy/edgequake.git
cd edgequake
make stack          # pulls images, starts all services, waits for health
```

Or one-liner without `git clone` (just Docker required):

```bash
# Pipe compose file direct to docker compose — no files saved locally
curl -fsSL https://raw.githubusercontent.com/raphaelmansuy/edgequake/edgequake-main/docker-compose.quickstart.yml \
  | docker compose -f - up -d

# Or with the helper shell script (auto-detects LLM provider):
curl -fsSL https://raw.githubusercontent.com/raphaelmansuy/edgequake/edgequake-main/quickstart.sh | sh
```

- **New `quickstart.sh`** at repo root — one command that downloads the compose file, auto-detects LLM provider (OpenAI if `OPENAI_API_KEY` set, else Ollama), pulls images, starts services, waits for health, and prints access URLs
- **New `docker-compose.quickstart.yml`** at repo root — pulls all three images from GHCR:
  - `ghcr.io/raphaelmansuy/edgequake:latest` (API, amd64 + arm64)
  - `ghcr.io/raphaelmansuy/edgequake-frontend:latest` (Next.js Web UI, amd64 + arm64)
  - `ghcr.io/raphaelmansuy/edgequake-postgres:latest` (PostgreSQL + pgvector + Apache AGE, **new**, amd64 + arm64)
- **New `make stack` target** (and siblings):
  - `make stack` — pull + start API, Web UI, PostgreSQL from GHCR
  - `make stack-down` — stop and remove containers
  - `make stack-logs` — tail all container logs
  - `make stack-status` — show container status
  - `make stack-pull` — update images without starting
  - `make stack-restart` — pull + restart
- **`release-docker.yml`** CI: added `build-postgres` job that builds and publishes the custom PostgreSQL image (`ghcr.io/raphaelmansuy/edgequake-postgres`) on every `v*.*.*` tag push. Also added `release` job that automatically creates a GitHub Release with image table + quickstart notes after all images are published.

#### Bug Fix — Zombie Documents (cannot delete/reprocess)

Documents whose workspace record was missing from the PostgreSQL `workspaces` table were permanently undeleteable ("zombie documents"). The strict vector storage resolver returned `404 Workspace not found` before any rows were cleaned up.

**Fix:** `delete/single.rs` now calls `get_workspace_vector_storage_for_delete` which gracefully falls back to default storage when the workspace record is absent. All KV, graph, and PostgreSQL rows are cleaned up correctly; any orphaned vector rows in the missing workspace are an acceptable trade-off vs. a permanently stuck document.

#### Multi-Provider Workspace Defaults (fixes #147, #145)

Two-tier environment variable resolution for `default_llm_config()`:
- `EDGEQUAKE_DEFAULT_LLM_PROVIDER` → `EDGEQUAKE_LLM_PROVIDER` → `"ollama"` (provider)
- `EDGEQUAKE_DEFAULT_LLM_MODEL` → `EDGEQUAKE_LLM_MODEL` → provider default model

`OPENAI_BASE_URL` passthrough added to `docker-compose.yml` for Azure, vLLM, and other OpenAI-compatible endpoints.

### Fixed

- **#92** Docker `--platform=${TARGETPLATFORM}` build error on plain `docker build` / `docker compose build`
- **#100** PDFium cache `Permission denied` on container startup — writable `/tmp/edgequake-pdfium-cache` + `ENV PDFIUM_AUTO_CACHE_DIR` set in Dockerfile
- **#147** Ollama workspace defaults not respected when `OPENAI_API_KEY` is set alongside `EDGEQUAKE_LLM_PROVIDER=ollama`
- **#145** Workspace default model config not reading `EDGEQUAKE_LLM_MODEL` env var
- **Delete pending/processing documents** — deleting a document with status `pending` or `processing` no longer returns `409 Conflict`. The handler now cancels the in-flight task via `CancellationRegistry` (best-effort) and proceeds with cascade delete unconditionally. First Principle: a user must always be able to delete their own document. SRP: the delete handler's only responsibility is data removal; lifecycle management belongs to the processor (which handles the no-op gracefully when KV keys are gone).
- **Default LLM/embedding models updated** — `gemma3` → `gemma4:latest` (LLM + vision); `embeddinggemma` → `embeddinggemma:latest` (embedding) across `workspace.rs`, Makefile, `.env.example`, WebUI provider card, and API Explorer

### Infrastructure

- Version bumped `0.9.1` → `0.9.4` across all crates and `VERSION` file
- CI `Check` jobs (fmt + clippy) fixed: `workspace.rs` test reformatted + `Cargo.lock` updated

---

## [0.9.3] - 2026-04-08

### Added

#### Docker — Frontend Image Published to GHCR
- **`ghcr.io/raphaelmansuy/edgequake-frontend`** multi-arch image (amd64 + arm64) now published on every `v*.*.*` tag alongside the API image. Bakes `NEXT_PUBLIC_API_URL=http://localhost:8080` for standard local + docker-compose use.
- **`docker-compose.prebuilt.yml`** updated: added `frontend` service pulling `ghcr.io/raphaelmansuy/edgequake-frontend:${EDGEQUAKE_VERSION:-latest}`. Full stack (API + Web UI + PostgreSQL) now starts from three published images — no Rust toolchain, no Node.js required.
- **`make docker-prebuilt`** updated: always pulls latest GHCR images before starting (`docker compose pull`), auto-creates `.env` from `.env.example` on first run, waits for API health, then prints access URLs.
- **New Makefile targets**: `make docker-prebuilt-logs`, `make docker-ps-prebuilt` for log tailing and status inspection of the prebuilt stack.
- **`release-docker.yml`** CI: added `build-frontend` job building the Next.js frontend image with QEMU multi-arch and pushing to GHCR.

#### Quick Start (zero local build)
```bash
cd edgequake/docker
make docker-prebuilt   # pulls latest API + Web UI + starts postgres
# → http://localhost:3000  (Web UI)
# → http://localhost:8080  (API)
```
Pin to a specific release: `EDGEQUAKE_VERSION=0.9.3 make docker-prebuilt`

### Changed
- `make docker-prebuilt` now performs `docker compose pull` before `up -d` so re-running the command always fetches the latest published images without needing `--pull always`.

## [Unreleased]

### Added — Docker CI/CD, Prebuilt Deployment & Multi-Provider Support

#### Docker — Three Deployment Options
- **New `docker-compose.prebuilt.yml`** (Option B): pulls the EdgeQuake API from GHCR (`ghcr.io/raphaelmansuy/edgequake:latest`); builds only the PostgreSQL service locally (required for Apache AGE + pgvector). No Rust toolchain needed. Use `EDGEQUAKE_VERSION=x.y.z` to pin to a specific release.
- **`docker-compose.api-only.yml`** (Option A): lightweight single-service compose using the GHCR image for bring-your-own-PostgreSQL scenarios.
- **`docker-compose.yml`** (Option C): unchanged — full build-from-source including the Next.js frontend.
- **`docker/.env.example`**: annotated environment template for all three compose options.

#### Docker CI/CD — Multi-arch GHCR Publishing
- New `.github/workflows/release-docker.yml`: native `linux/amd64` + `linux/arm64` Docker builds published to GHCR on every `v*.*.*` tag push or `workflow_dispatch`. Native ARM64 runner used (no QEMU). Images merged into a single multi-arch manifest with both `version` and `latest` tags.

#### README — Docker Deployment section rewritten
- Option A (API only): `docker run` one-liner + `docker-compose.api-only.yml`.
- Option B (prebuilt full stack): **new** — `docker compose -f docker-compose.prebuilt.yml up -d` pulls the prebuilt GHCR image.
- Option C (build from source): `docker compose up -d` with all three services.
- Full environment variable reference table with all 13 variables.
- "Building Locally", "CI/CD — Automated Releases" subsections.

#### New LLM Providers (edgequake-llm 0.5.1)
- `provider_types.rs`: Added `ProviderInfo` for **Mistral AI** (`mistral-small-latest`, `mistral-embed`, 1024-dim) and **Google Vertex AI** (`gemini-2.5-flash`, `gemini-embedding-001`, 3072-dim). Both appear in `/api/v1/providers` and the provider selector UI.
- `provider_setup.rs` docs: updated embedding-provider table with Azure and Mistral env vars.

#### TypeScript E2E Fix
- Created `edgequake_webui/e2e/global.d.ts` to fix `TS2339` (`Window.__requestUrls` not on `Window`).

### Changed — Dependency Bump

#### edgequake-llm 0.3.0 → 0.5.1
New capabilities added by upstream:
- Azure OpenAI chat + embedding provider
- Mistral AI chat + embedding provider (`mistral-small-latest`, `mistral-embed`)
- Google Vertex AI provider (`gemini-2.5-flash`, `gemini-embedding-001`)
- Streaming usage reporting fixes
- Image generation API extension points

#### edgequake-pdf2md =0.7.0 → =0.8.0
- Declares `edgequake-llm = "0.5.1"` — resolves the previous diamond dependency that caused `E0308` type mismatch when building the workspace with `edgequake-llm 0.5.1`.

**Build verified:** `cargo build --workspace --lib` ✅  `cargo clippy --workspace -- -D warnings` ✅  `cargo fmt --all -- --check` ✅

## [0.9.1] - 2026-04-03

### Fixed

#### Graph Edge Labels (Relation Types) Now Display — Closes [#91](https://github.com/raphaelmansuy/edgequake/issues/91)

- **Root cause fixed**: Sigma 3.x only draws edge labels automatically when *both* endpoint nodes have their node-labels visible (limited by `labelDensity` / `labelRenderedSizeThreshold`). Edge labels therefore never appeared even when "Show Edge Labels" was enabled in Settings.
- **Fix**: all graph edges now receive `forceLabel: true` when the "Show Edge Labels" setting is on. The `forceLabel` flag bypasses the endpoint-label requirement and always draws the edge label canvas text.
- **Explicit edge-label styling** added to the Sigma constructor: `edgeLabelSize: 10`, `edgeLabelFont` matching the app font, `edgeLabelWeight: '500'`, and `edgeLabelColor` using high-contrast colours (`#e2e8f0` dark / `#334155` light) instead of the default edge colour (which had low contrast).
- **Streaming path** (`addEdgesToGraph` callback): same `forceLabel` logic applied so progressively-streamed edges also render their labels.
- **Node-expansion path** (`use-graph-expansion.ts`): edges added via the "expand node" action also receive `forceLabel`, so labels stay consistent after expanding a neighbourhood.
- **New E2E test** (`e2e/issue-91-edge-labels.spec.ts`): 3 Playwright tests — settings toggle presence, edgeLabels canvas attachment, and forceLabel DOM verification.

#### Mermaid Renderer: Curly-Brace Labels, Forward Slashes, and Graceful Fallback — Closes [#141](https://github.com/raphaelmansuy/edgequake/issues/141)

- **Bare curly-brace node expressions** (`{label}` with no preceding node ID) are now detected and rewritten to a valid quoted rectangular node (`_bare_N["label"]`). This eliminates the `DIAMOND_START` parse error that crashed the renderer when the LLM emitted `People --> {Personnes/Gens}`.
- **Forward slashes `/` and backslashes `\`** inside unquoted square-bracket labels are now included in the sanitiser's special-character class and will be auto-quoted (e.g. `A[yes/no]` → `A["yes/no"]`).
- **Rhombus/diamond labels** (`NodeId{label}`) containing `/`, `\`, `|`, `<`, or `>` are now quoted in-place while keeping the rhombus shape (e.g. `B{yes/no}` → `B{"yes/no"}`).
- **Error fallback hardened**: when `mermaid.render()` throws after sanitisation the component now renders a prominent `<pre>` block with the raw Mermaid source and a friendly error message instead of propagating the exception to the Next.js overlay. A collapsible "Show sanitized version" section is shown when the code was rewritten before the failed render.
- `sanitizeMermaidCode()` exported for direct unit-testing.
- **23 new unit tests** (`src/components/query/markdown/__tests__/MermaidBlock.test.ts`) covering: bare `{label}` rewrite, forward-slash quoting, backslash quoting, pipe quoting, angle-bracket quoting, rhombus-label quoting, no-regression on already-valid diagrams, code-block stripping, and completeness detection.



- Added `NEXT_PUBLIC_DISABLE_DEMO_LOGIN` environment variable to the Next.js frontend. When set to `true` at build time, the **"Continue without login (Demo)"** button and its separator are hidden on the login page, preventing unintentional unauthenticated access in production deployments.
- Updated `.env.example` and `docs/operations/configuration.md` with documentation for the new variable.

#### Separate LLM and Embedding Provider Hosts — Closes [#140](https://github.com/raphaelmansuy/edgequake/issues/140)

- Added **hybrid provider mode**: the embedding provider can now be configured independently from the LLM provider without editing code.
- New `OLLAMA_EMBEDDING_HOST` environment variable: routes embedding requests to a dedicated Ollama instance (e.g. a separate GPU node), while LLM chat continues to use `OLLAMA_HOST` or whatever the main provider is.
- New `EDGEQUAKE_EMBEDDING_PROVIDER` environment variable: explicitly selects a different provider type (e.g. `openai`, `ollama`) for embeddings in hybrid setups.
- New `EDGEQUAKE_EMBEDDING_MODEL` / `EDGEQUAKE_EMBEDDING_DIMENSION` variables for finer control.
- Implemented in a new `state/provider_setup.rs` module (`resolve_embedding_provider()`), applied in both `AppState::new_postgres()` and `AppState::new_memory()`. The helper is non-fatal: misconfigured overrides warn and fall back to the default provider so the server never fails to start due to an embedding config error.
- Updated `.env.example` with examples for the hybrid mode configuration.
- Updated `docs/operations/configuration.md` with a dedicated "Hybrid Provider Mode" table and examples.



### Added

#### Custom Entity Configuration per Workspace (SPEC-085) — Closes [#85](https://github.com/raphaelmansuy/edgequake/issues/85)

- **`entity_types` field in `CreateWorkspaceRequest`**: Users can now specify a custom list of entity types when creating a workspace. Types are normalized (trimmed, uppercased, spaces/hyphens replaced with underscores, deduplicated) and capped at **50 types** (increased from 20).
- **Entity types persisted in workspace metadata**: Custom entity types are stored as a JSONB array in the `metadata` column of the `workspaces` table — no schema migration required.
- **Pipeline reads workspace entity types**: The document ingestion pipeline reads custom entity types from workspace metadata at upload time and configures the extractor accordingly. Falls back to `default_entity_types()` (9 general types) when no custom config is present.
- **`normalize_entity_types()` backend function**: Validates and normalizes entity type input (uppercase, underscore format, deduplication, max-50 cap).
- **Entity type selector UI** (`EntityTypeSelector` component): Preset buttons (General, Manufacturing, Healthcare, Legal, Research, Finance), chip display with individual remove, custom type input (normalized on add), live count badge, and collapsible Advanced tab.
- **6 domain presets**: General, Manufacturing, Healthcare, Legal, Research, Finance — each with 9–12 curated entity types.
- **`MAX_ENTITY_TYPES = 50`** constant mirrored in both backend (`workspace_service_impl.rs`) and frontend (`entity-presets.ts`).
- **Interactive Playwright E2E tests** (`spec-085-entity-ui-interactive.spec.ts`): 8 tests covering preset selection, custom type input, chip removal, max-50 enforcement, empty state, accessibility (aria-pressed, aria-live), and API round-trip verification.
- **Dialog layout improvements**: Workspace/tenant creation dialogs now use `overflow-hidden + grid-rows` to prevent entity-type selector from overflowing the viewport.

### Changed

- Entity type limit raised from **20 to 50** per workspace. This allows richer domain-specific vocabularies without prompt-length concerns. At 50 types, the prompt overhead is ~250 additional tokens — acceptable relative to chunk content.
- `LLMExtractor` defaults now aligned with `default_entity_types()` (9 types: PERSON, ORGANIZATION, LOCATION, EVENT, CONCEPT, TECHNOLOGY, PRODUCT, DATE, DOCUMENT) — previously used 7 types missing DATE and DOCUMENT.

## [0.8.0] - 2026-04-03

### Added

#### Knowledge Injection — Domain Glossaries & Synonym Enrichment (SPEC-0002) — Closes [#131](https://github.com/raphaelmansuy/edgequake/issues/131)

- **`PUT /api/v1/workspaces/:workspace_id/injection`**: Create or replace a named knowledge injection entry. Content is processed through the standard entity-extraction pipeline with `source_type = "injection"` tagging so entities enrich the knowledge graph without surfacing as document citations.
- **`POST /api/v1/workspaces/:workspace_id/injection/upload`**: Upload a `.txt`/`.md`/plain-text file as a knowledge injection entry. Supports multipart form upload with the same pipeline semantics as text injection.
- **`GET /api/v1/workspaces/:workspace_id/injection`**: List all injection entries for a workspace with name, status (`processing` / `completed` / `failed`), entity count, and timestamps.
- **`GET /api/v1/workspaces/:workspace_id/injection/:injection_id`**: Fetch a single injection entry including full content.
- **`PATCH /api/v1/workspaces/:workspace_id/injection/:injection_id`**: Update the name and/or content of an existing injection entry; re-triggers pipeline processing when content changes.
- **`DELETE /api/v1/workspaces/:workspace_id/injection/:injection_id`**: Delete a named injection entry and cascade-remove all its entities, vectors, and graph nodes/edges.
- **`injection_types.rs`**: New `PutInjectionRequest`, `PutInjectionResponse`, `PatchInjectionRequest`, `InjectionListItem`, `InjectionDetail`, and `InjectionListResponse` types with full OpenAPI annotations.
- **`injection.rs`** — DRY primitives: `workspace_id_from_tenant()`, `validate_name()`, `run_pipeline_for_injection()` shared across all handlers to eliminate duplication.
- **Citation exclusion**: All query modes (`/api/v1/query`, `/api/v1/query/stream`, `/api/v1/chat/completions`, `/api/v1/chat/completions/stream`) filter out `source_type = "injection"` from `SourceReference` arrays. Injection knowledge enriches answers but is never listed as a source.
- **Injection entries excluded from document list**: `GET /api/v1/documents` no longer returns injection KV entries; they are only visible via the `/injection` endpoints.
- **`query_with_vector_storage()`** on `SOTAQueryEngine`: New convenience method for workspaces that share the server's default embedding model — delegates to `query_with_workspace_config()` without requiring callers to replicate embedding provider access.
- **`/knowledge` frontend page** (`app/(dashboard)/knowledge/page.tsx`): Dedicated UI for managing injection entries. Features: add/edit/delete dialogs, text and file upload tabs, status badges, entity count display, search filtering, and pagination.
- **Knowledge detail page** (`app/(dashboard)/knowledge/[id]/page.tsx`): Full-detail view with inline editing, retry, and delete confirmation dialog.
- **`useInjection` hook** (`hooks/use-injection.ts`): React Query–powered hook for all injection CRUD operations with optimistic updates and cache invalidation.
- **`edgequake.ts` API client**: 8 new typed methods — `listInjections`, `getInjection`, `putInjection`, `patchInjection`, `deleteInjection`, `uploadInjectionFile`, `pollInjectionStatus`, `queryWithExpansion`.
- **Sidebar navigation**: New "Knowledge" entry (BookOpen icon) between Documents and Pipeline.
- **1 000+ line Rust E2E test suite** (`tests/e2e_injection.rs`): Covers create, read, list, PATCH, DELETE, file upload, citation exclusion, document-list exclusion, concurrent operations, large content, unicode names, and edge cases.
- **5 Playwright E2E tests** (`e2e/knowledge-injection-crud.spec.ts`): Add, edit, delete, API verification, and query-retrieval of injected terms.
- **Source citation deep-link E2E tests** (`e2e/source-citations-deep-linking.spec.ts`): Verify citation links open document detail pages.

### Changed

- `storage_helpers::cleanup_document_graph_data` visibility widened from `pub(super)` to `pub(crate)` so the injection delete handler can reuse it without duplication.
- `documents/mod.rs`: `storage_helpers` module visibility widened to `pub(crate)` accordingly.
- `source-citations.tsx`: Component now filters out injection-tagged sources from the displayed reference list.

### Internal

- Total tests: **1 122+ passing** (526 core, 123 storage, 179 query, 92 pipeline, 34 PDF, 72 worker, 12 graph, 79 LLM, 5 PDF-crate).

---

## [0.7.0] - 2026-03-18

### Added

#### Vector Storage Optimization — Tier 2 & Tier 3 (SPEC-007)

- **`MetadataFilter` type** (`vector.rs`): New filter struct with `document_ids`, `tenant_id`, and `workspace_id` fields for SQL-level pre-filtering. Builder `from_tenant_workspace()` and `is_empty()` provided for ergonomic construction.
- **`query_filtered()` trait method** on `VectorStorage`: Pushes metadata filtering to the storage layer (SQL WHERE) instead of post-filtering in application code. Default implementation delegates to `query()` for backward compatibility.
- **`PgVectorStorage::query_filtered()`**: Dynamic SQL WHERE clause generation with parameterized queries. Supports Tier 3 materialized columns with Tier 2 JSONB fallback (`document_id = ANY($N) OR metadata->>'document_id' = ANY($N)`).
- **`MemoryVectorStorage::query_filtered()`**: In-memory metadata filtering with identical semantics to SQL implementation. Lenient matching for `tenant_id`/`workspace_id` (missing field passes), strict matching for `document_ids` (missing field excludes).
- **GIN index on metadata JSONB** (migration 027): Accelerates Tier 2 JSONB WHERE clauses for all vector tables.
- **Materialized columns** (migration 028): `document_id`, `tenant_id`, `workspace_id` columns added to vector tables with automatic JSONB backfill.
- **B-tree indexes** (migration 029): Dedicated indexes on materialized columns for fast equality lookups.
- **Dual-write on upsert**: `PgVectorStorage::upsert()` writes metadata to both JSONB blob and materialized columns, using `COALESCE(document_id, source_document_id)` normalization.
- **30+ edge-case tests**: Comprehensive coverage including null metadata, missing fields, empty filters, empty document ID lists, wrong dimensions, score ordering, top-k after filter, all-fields-combined, AND semantics for filter_ids + metadata_filter, source_document_id-only matching, empty storage, and top-k=0.

### Changed

- **Query pipeline fully wired**: All 18 vector query call sites (10 in `query_modes.rs`, 8 in `vector_queries.rs`) migrated from `query()` to `query_filtered()` with `MetadataFilter::from_tenant_workspace()`.
- **Removed 14 `matches_tenant_filter` post-filter calls**: Tenant/workspace filtering now happens at the SQL layer, reducing wasted vector scans by up to 90% for multi-tenant deployments.
- **Migrations safe for fresh databases**: Dynamic table discovery (`pg_tables WHERE tablename LIKE 'eq_%_vectors'`) instead of hardcoded table names.

### Performance

| Document Count | Vectors (est.) | Tier 1 Waste | Tier 2+3 Savings |
| :------------: | :------------: | :----------: | :--------------: |
|      100       |      ~5K       |     <5%      |    Negligible    |
|     1,000      |      ~50K      |    10-40%    |       ~30%       |
|     10,000     |     ~500K      |    40-80%    |       ~60%       |
|    100,000     |      ~5M       |    80-95%    |       ~90%       |

## [0.6.0] - 2026-03-18

### Added

#### Unified Streaming Response Protocol (SPEC-006) — Closes [#56](https://github.com/raphaelmansuy/edgequake/issues/56)

- **Structured SSE events** for `/api/v1/query/stream`: New `context`, `token`, `thinking`, `done`, and `error` event types replace raw text streaming (v2 format). Backward-compatible `v1` format available via `stream_format: "v1"` parameter.
- **`QueryStreamEvent` enum** (`query_types.rs`): Five tagged variants with full context (sources, query mode, retrieval timing) and statistics (tokens/sec, generation time, total time).
- **`QueryStreamStats` struct**: Comprehensive streaming statistics including `embedding_time_ms`, `retrieval_time_ms`, `generation_time_ms`, `total_time_ms`, `sources_retrieved`, `tokens_used`, `tokens_per_second`, and `query_mode`.
- **Enriched `SourceReference`**: Added `entity_type`, `degree`, and `source_chunk_ids` fields to source references across all API responses, enabling richer entity display and provenance tracking.
- **Enriched `ChatStreamEvent::Context`**: Added `query_mode` and `retrieval_time_ms` fields to chat streaming context events for consistent timing feedback.
- **`StreamQueryRequest` expansion**: New `document_filter`, `llm_provider`, `llm_model`, and `stream_format` parameters for query stream endpoint.
- **Full provider resolution**: Query stream endpoint now supports workspace-specific embedding providers, vector storage, and LLM provider overrides (same dispatch logic as chat streaming).
- **Frontend TypeScript**: Updated `SourceReference`, `ChatStreamEvent`, `StreamingState`, `QueryStreamChunk`, and `QueryRequest` types. Added `QueryStreamStats` interface. Updated `reduceStreamingEvent()` and `mapSourcesToContext()` for enriched fields.
- **Rust SDK**: Updated `SourceReference` (both chat and query types), `ChatStreamChunk`, `QueryStreamChunk`, and `QueryRequest` with SPEC-006 fields. Added `QueryStreamStats` struct.
- **API documentation**: Updated `/api/v1/query/stream` and `/api/v1/chat/completions` SSE event documentation with v2 structured format examples.

### Changed

- Query stream handler rewritten from 99 lines to full workspace-aware implementation with mpsc channel-based event dispatch (matching chat streaming architecture).
- `build_sources()` visibility changed from private to `pub(crate)` to allow reuse across query and chat handlers.

## [0.5.6] - 2026-03-17

### Added

#### Document Query Filters (SPEC-005) — Closes [#75](https://github.com/raphaelmansuy/edgequake/issues/75)

- **`document_filter` field** on `QueryRequest`, `ChatCompletionRequest`, and `ListDocumentsRequest`: Optional filter restricting RAG context and document listings by date range (`date_from`, `date_to`) and document name pattern (`document_pattern`). All criteria are AND-ed.
- **`context_filter.rs`** (`edgequake-query`): Post-retrieval context filtering by `allowed_document_ids`. Chunks use strict matching; entities and relationships use lenient matching (kept if no provenance metadata).
- **`document_filter_resolver.rs`** (`edgequake-api`): Resolves `DocumentFilter` → `Option<Vec<String>>` of matching document IDs via KV metadata scan. Supports tenant/workspace scoping, ISO 8601 date comparison, and case-insensitive comma-separated OR pattern matching on titles.
- **All 10 query entry points wired**: Filter applied in `query()`, `query_with_embedding_provider()`, `query_with_llm_provider()`, `get_context()`, `query_with_workspace_config()`, `query_with_full_config()`, `query_stream()`, and `query_stream_with_full_config()`.
- **All 4 API handlers wired**: `/api/v1/query`, `/api/v1/query/stream`, `/api/v1/chat/completions`, and `/api/v1/chat/completions/stream` accept and resolve `document_filter`.
- **Document listing**: `GET /api/v1/documents` accepts `date_from`, `date_to`, `document_pattern` query parameters for server-side filtering.
- **Frontend**: `QueryDocumentFilter` popover component with date range pickers and pattern input, integrated into the query interface toolbar. Active filter count badge on the filter button.
- **12 unit tests**: 4 for `context_filter` and 8 for `document_filter_resolver` covering date ranges, patterns, tenant scoping, combined filters, and edge cases.

## [0.5.5] - 2026-02-27

### Added

#### System Prompt Extension Point (SPEC-004)

- **`system_prompt` field** on `QueryRequest` and `ChatCompletionRequest`: Optional free-text prompt prepended to the LLM context as `---Additional Instructions---`. Allows callers to steer tone, persona, language, and output format per-query without modifying the global pipeline.
- **Backend** (`engine.rs`, `prompt.rs`): `QueryRequest.system_prompt` is threaded through the SOTA engine and injected before the RAG context in `build_prompt()`.
- **All API handlers**: `/api/v1/query`, `/api/v1/query/stream`, `/api/v1/chat/completions`, and `/api/v1/chat/completions/stream` accept and forward `system_prompt`.
- **Frontend** (`query-settings-sheet.tsx`): New "System Prompt" textarea in the query settings panel with active indicator.
- **10 SDK updates**: TypeScript, Python, Rust, Go, Java, Kotlin, C#, Swift, PHP, and Ruby SDKs now expose `system_prompt` on query and chat request types.
- **6 unit tests** covering prompt injection, empty/None handling, and builder API.

#### Cooperative Pipeline Cancellation

- **`CancellationRegistry`** (`edgequake-tasks/src/cancellation.rs`): New per-task cooperative cancellation using `tokio_util::sync::CancellationToken`. Each running task gets a unique token registered at worker start and deregistered on completion.
- **`cancel_task` handler** (`handlers/tasks.rs`): `POST /tasks/{track_id}/cancel` now triggers the token, causing the pipeline to exit at the next cancellation gate instead of waiting for the current LLM call to finish.
- **6 cancellation gates in `text_insert.rs`**: Before chunking, after chunking, before extraction, after extraction, before embedding, and after storage — each calls `check_cancelled()`.
- **2 cancellation gates in `pdf_processing.rs`**: After PDF-to-markdown conversion and after vision extraction.
- **Per-chunk + per-retry cancellation** (`extraction.rs`, `processing.rs`): Entity extraction loop and resilience retry loop check the token between iterations.
- **Shared registry** (`main.rs`): `CancellationRegistry` is shared between `WorkerPool` and `AppState` so the cancel API endpoint and the worker pool use the same token store.

### Fixed

#### Undeletable Documents with KV Key/ID Mismatch

- **`resolve_kv_key_prefix()`** (`delete/single.rs`): New two-phase resolution — fast path checks `{id}-metadata` directly, slow path scans all metadata keys for matching JSON `id` field. Handles historical data where the KV key prefix diverged from the metadata JSON `id`.
- **Comprehensive key cleanup** (`delete/single.rs`): Delete now collects ALL keys under both the resolved KV prefix and the JSON id prefix (catches lineage, checkpoint, and other auxiliary keys).
- **Source prefix matching** (`delete/single.rs`): Graph entity/edge source filtering uses both prefixes in mismatch cases to prevent orphaned graph data.
- **Postgres cascade** (`delete/single.rs`): `delete_document_record` tries both UUIDs (KV prefix and JSON id) when they differ.
- **6 unit tests** covering fast-path resolution, mismatch resolution, not-found, full cascade with mismatch, lineage key cleanup, and 404 for truly nonexistent documents.

#### Clippy

- **`pipeline_checkpoint.rs`**: Fixed 3 `cloned_ref_to_slice_refs` warnings by using `std::slice::from_ref()` instead of `&[key.clone()]`.

## [0.5.4] - 2026-02-26

### Fixed

#### Dashboard KPIs: Accurate Document/Entity/Relationship Counts (closes #81)

- **Phase 1 — Stats endpoint** (`stats.rs`): Removed PostgreSQL-first fallback in `fetch_workspace_stats_uncached`. The endpoint now always uses KV storage for document counts and Apache AGE for entity/relationship counts, eliminating the premature short-circuit at `if stats.document_count > 0` that skipped the accurate data path.
- **Phase 2 — Dual-write** (`text_upload.rs`, `file_upload.rs`, `text_insert.rs`): Added `ensure_document_record` calls after document processing completes, so text/markdown/file uploads also populate the PostgreSQL `documents` table for consistency. Previously only PDF uploads called this function.

### Added

- **14 E2E test cases** (`e2e_dashboard_stats_issue81.rs`): Comprehensive regression tests covering empty workspace, mixed document types, entity/relationship counts, workspace isolation, cache contamination, orphan documents, chunk counts, storage bytes aggregation, response shape validation, and stress test (50 documents).

### Infrastructure

- Bumped version `0.5.3` → `0.5.4` in `Cargo.toml`, `VERSION`, and `package.json`.

## [0.5.3] - 2026-02-26

### Fixed

#### WebUI: Consistent API Base URL (closes #79)

- **`getPdfDownloadUrl()`** (`edgequake.ts`): Replaced incorrect `NEXT_PUBLIC_API_BASE_URL` env var with `SERVER_BASE_URL` (derived from `NEXT_PUBLIC_API_URL`), fixing PDF downloads that failed with `ERR_CONNECTION_REFUSED` in production when the non-standard env var was unset.
- **`exportDocumentLineage()`** (`edgequake.ts`): Same fix applied — lineage export downloads now use the same base URL as the rest of the API client.

### Infrastructure

- Bumped version `0.5.2` → `0.5.3` in `Cargo.toml`, `VERSION`, and `package.json`.

## [0.5.2] - 2026-02-26

### Fixed

#### Document Lifecycle & Cascade Delete (closes #73, #74)

- **FK constraint on PDF upload** (`pdf_processing.rs`): `ensure_document_record` now inserts into the `documents` table _before_ `pdf_documents`, preventing the foreign key violation that caused uploads to silently fail.
- **Cascade delete** (`single.rs`): Deleting a document now also removes the associated `pdf_documents` row and lets `ON DELETE CASCADE` clean up chunks and graph edges.
- **Status CHECK constraint** (`pdf_processing.rs`): Changed status value from `"completed"` (invalid) to `"indexed"` to satisfy the `documents_valid_status` CHECK constraint in migration 001/003.
- **UTF-8 boundary panic** (`pdf_processing.rs`): Markdown preview truncation (`&markdown[..65_536]`) now uses `char_indices()` to find a safe byte boundary, preventing panics on multi-byte characters.

#### Table Preprocessor Quality (SRP / DRY / Edge Cases)

- **Refactored `table_preprocessor.rs`** for single-responsibility: extracted `ParsedTable::from_lines()`, `group_rows_by_first_column()`, and `emit_grouped_sections()` as focused helper functions.
- **DRY**: Added `PreprocessResult::passthrough()` constructor to eliminate four identical block constructions.
- **Configurable title**: Replaced hard-coded `"Glossary / Data Dictionary"` with `document_title: Option<String>` field on `TablePreprocessorConfig`.
- **Separator false-positive fix**: `is_separator_line("| |")` no longer incorrectly returns `true` (guarded against `.all()` on empty iterators).
- **Test coverage**: Expanded from 9 → 30 tests covering: unicode grouping, deduplication toggle, truncation boundary, threshold semantics, alphabetical ordering, summary statistics, empty/whitespace inputs, mixed content, and more.

### Infrastructure

- Bumped version `0.5.1` → `0.5.2` in `Cargo.toml`, `VERSION`, and `package.json`.

## [0.5.1] - 2026-02-24

### Security

#### Tenant / Workspace Isolation (full audit)

- **`verify_workspace_tenant_access` helper** (`handlers/workspaces/helpers.rs`): Centralised guard that fetches a workspace by ID, checks that `workspace.tenant_id` matches the `X-Tenant-ID` request header, and returns **404** (not 403) on mismatch to prevent cross-tenant UUID enumeration. Access is permissive when the header is absent for backward-compat with admin/direct-API use.
- **Workspace CRUD** (`workspace_crud.rs`): `get_workspace`, `update_workspace`, and `delete_workspace` now require the workspace to belong to the requesting tenant before serving or mutating data.
- **Stats & metrics** (`stats.rs`): `get_workspace_stats` verifies tenant ownership **before** consulting the in-memory cache — cross-tenant requests never receive cached data from workspaces they do not own. Same check applied to `get_metrics_history` and `trigger_metrics_snapshot`.
- **Bulk operations** (`rebuild_embeddings`, `rebuild_knowledge_graph`, `reprocess_all_documents`): Inline `BR0201` guard added to all three destructive/long-running handlers.

### Fixed

#### Workspace / Tenant UX

- **Auto-select after creation** (`tenant-workspace-selector.tsx`, `use-tenant-context.ts`): When a new workspace or tenant is created, it is immediately pushed into the Zustand store (`setWorkspaces` / `setTenants`) before `selectWorkspace()` / `selectTenant()` is called. This eliminates the race-condition window where the Select dropdown showed "Select workspace…" until the async React Query refetch delivered the new item. The fix is applied in both the sidebar `TenantWorkspaceSelector` component and the `useTenantContext` hook so all call-sites are consistent.

### Infrastructure

- Bumped `[workspace.package] version` in `edgequake/Cargo.toml`, `VERSION`, and `edgequake_webui/package.json` from `0.5.0` → `0.5.1`.

## [0.5.0] - 2026-02-25

### Added

#### Query UX Enhancements

- **Wider query/answer layout** (`query-interface.tsx`): Message area widened from `max-w-3xl` to `max-w-4xl lg:max-w-5xl`; assistant message container set to `max-w-full` for long tables and code blocks
- **Response language support** (full-stack): Backend detects the `language` field sent by the frontend (`ChatCompletionRequest`) and appends `[IMPORTANT: You MUST respond in {Language}]` to the query before the LLM call via `enrich_query_with_language()` — frontend passes `i18n.language` automatically
- **Mermaid syntax sanitization** (`MermaidBlock.tsx`): `sanitizeMermaidCode()` fully rewritten — auto-quotes labels that contain `(){}|><` or non-ASCII characters (e.g., `A[label (note)]` → `A["label (note)"]`), maps non-ASCII node IDs, and shows the sanitized source in the error view

#### Source Citations & Deep-Links

- **Chunk deep-link on sidebar selection**: Clicking a source chunk in the sidebar navigates to the exact file location via deep-link
- **Auto-resolve chunk line range on deep-link**: Content highlights auto-open and the Data Hierarchy panel reveals the referenced section
- **Improved source-citations UX**: Uniform scroll height, per-document chunk expand/collapse, count badges, and better contrast for citations

#### Developer / Architecture

- **Centralised tenant isolation** (`handlers/isolation.rs`): DRY/SRP refactor — all workspace/tenant security checks route through a single `isolation.rs` module, reducing duplication across handlers
- **Workspace-scoped rebuild**: Exclude cross-workspace documents from incremental rebuild scope

### Changed

- **Streaming markdown UX**: Larger text rendering, light/dark theme consistency, full-view dialogs for long code and Mermaid blocks
- **Router history on deep-link**: `router.push()` used for chunk navigation to preserve browser back-button history
- **Yellow chunk highlight + source-citations contrast**: Selected chunks highlighted in amber; citation rows have improved foreground/background contrast ratio

### Fixed

- **Chunk deep-link propagation**: `chunk_id` correctly propagated from query citations to URL parameters
- **`chunk_id` in `convertServerMessage`**: Historical messages now carry `chunk_id` so citations reopen correctly after page reload
- **Source-mapper `chunk_id` propagation**: Fixed `isolation.rs`, `lineage.rs`, and source-mapper to consistently pass `chunk_id` through pipeline
- **Table streaming flicker**: Eliminated double-render and flicker when a streamed response block transitions from partial to complete table
- **Accessibility, responsive design, smooth display**: ARIA labels, keyboard navigation, reduced-motion preference, and mobile breakpoints across the query UI

## [0.4.1] - 2026-02-23

### Added

#### Tenant & Workspace Model Configuration (SPEC-041 / SPEC-032)

- **Vision LLM selector in Create Tenant form**: Users can now set a default Vision LLM (filtered to vision-capable models) when creating a tenant — inherited by all new workspaces
- **Vision LLM selector in Create Workspace form**: Per-workspace override for the Vision LLM used in PDF-to-Markdown extraction
- **`filterVision` prop on `LLMModelSelector` and `ModelSelector`**: Restricts the dropdown to models with `supports_vision === true`
- **`vision_llm_model` / `vision_llm_provider` in `CreateWorkspaceRequest`** type: Workspace creation API now accepts Vision LLM fields (SPEC-041)

### Changed

- **LLM Model, Embedding Model, and Vision LLM are now required** in both Create Tenant and Create Workspace forms; the Create button is disabled until all three are selected and labels show a red `*`

## [0.4.0] - 2026-02-19

### Added

#### PDF → LLM Vision Pipeline (SPEC-040)

- **Vision-Based PDF Extraction** (FEAT1010): Multimodal LLM reads PDF page images directly — handles scanned docs, complex layouts, and tables where text extraction fails
- **Multi-Page Image Extraction** (FEAT1011): Each PDF page rendered to high-resolution images (up to 2048px), encoded as base64 and streamed to the vision LLM
- **LLM-Powered Layout Understanding** (FEAT1012): GPT-4o / Claude / Gemini vision models interpret page structure, resolve multi-column text, reconstruct tables
- **Automatic Fallback** (BR1010): If vision extraction fails (quota, timeout, no vision model), the pipeline gracefully falls back to pdfium text extraction
- **Resolution Capping** (BR1011): Image DPI capped at 300 / max-side 2048px to balance quality vs. token cost
- **Zero-Config pdfium**: Switched to `edgequake-pdf2md` 0.4.1 – pdfium binary now embedded; no `PDFIUM_DYNAMIC_LIB_PATH` env var required
- **ExtractionMethod field on Block**: Each extracted block carries `vision`, `text`, or `ocr` metadata for traceability
- **Config flag `use_vision_llm`**: Opt-in per-request; set on `PdfExtractConfig` or pass `X-Use-Vision: true` HTTP header

#### Improved Developer Experience

- `cargo build` now works out-of-the-box without downloading pdfium — CI shaved ~40 s
- `vision`, `image_ocr`, and `formula` sub-modules extracted into focused files for maintainability
- `ProgressCallback` wired through vision pipeline for live extraction progress in WebUI

### Changed

- Workspace version bumped to `0.4.0` across all crates
- `edgequake-pdf` crate internal refactor: layout, processors, renderers grouped into sub-modules
- Default extraction mode is still text (`use_vision_llm = false`); vision is opt-in to avoid unexpected LLM cost
- README "Experimental" PDF warning upgraded to "Production Ready (vision mode optional)"

### Fixed

- PDF pipeline `block_in_place` / `spawn` issues that caused `Send` bound errors with async trait are fully resolved in 0.4.0
- PDFIUM path resolution in Docker images now works without manual env var

## [0.3.0] - 2025-02-17

### Added

#### Multi-Provider Support Expansion

- **9 Active Providers**: OpenAI, Anthropic, Google Gemini, xAI, OpenRouter, Ollama, LM Studio, Azure OpenAI, Mock
- **26 Model Configurations**: Comprehensive pricing data across all providers
- **Latest Model Support**:
  - Anthropic: Claude Opus 4.6, Sonnet 4.5, Haiku 4.5 (200K context, 128K max output)
  - xAI: Grok 4.1 Fast, Grok 4.0, Grok 3, Grok 3 Mini (up to 2M context)
  - Google Gemini: 2.5 Pro, 2.5 Flash, 2.5 Flash Lite, 2.0 Experimental (thinking capabilities)
  - OpenAI: o4-mini (reasoning model), o4, o1-2024-12-17

#### Cost Tracking Enhancements

- Updated pricing for 26 models (Feb 2025 verified rates)
- Expanded `default_model_pricing()` from 10 to 26 entries
- Added pricing for embedding models: text-embedding-3-small, gemini-embedding-001
- Cost tracking infrastructure fully seeded with latest pricing data

#### Provider Configuration

- Updated default models for all providers in safety limits
- Enhanced provider metadata with latest model information
- Improved WebUI configuration snippets with current models
- Auto-detection priority order for cloud providers

#### Lineage Tracking & Metadata (OODA-01 through OODA-25)

- Chunk position metadata: `start_line`, `end_line`, `start_offset`, `end_offset` fields (OODA-01)
- Chunk model tracking: `llm_model`, `embedding_model`, `embedding_dimension` fields (OODA-02)
- Document lineage metadata: `document_type`, `file_size`, `sha256_checksum`, `pdf_id`, `processed_at` fields (OODA-03)
- PDF↔Document bidirectional linking with `pdf_id` in document metadata (OODA-04)
- Lineage tracking enabled by default (`enable_lineage_tracking = true`)
- `GET /api/v1/chunks/{id}/lineage` — Chunk lineage with parent refs (OODA-08)
- `GET /api/v1/documents/{id}/lineage/export?format=json|csv` — Download lineage as file (OODA-22)
- In-memory TTL cache (120s, 500 entries max) for lineage queries (OODA-23)
- Enhanced metadata component with KV storage fields (OODA-12)
- Document hierarchy tree: Document → Chunks → Entities (OODA-13)
- Lineage export buttons (JSON/CSV download) in metadata sidebar (OODA-24)
- **TypeScript SDK**: `documents.getLineage()`, `getMetadata()`, `chunks.getLineage()` (OODA-15)
- **Python SDK**: Same methods on sync and async resource classes (OODA-16)
- E2E tests for lineage/metadata in all 3 SDKs (OODA-21)
- `docs/operations/metadata-debugging.md` — Diagnostics & repair guide (~260 lines) (OODA-20)
- "Unfiled" filter for conversations: displays all conversations not assigned to a folder
- Frontend and backend support for filtering by unfiled conversations

### Changed

#### Model Catalog Updates

- **Anthropic**: Updated to Claude 4.x series (Opus 4.6, Sonnet 4.5, Haiku 4.5)
- **xAI**: Updated to Grok 4.x/3.x series with 2M context models
- **Gemini**: Updated to 2.5 series with thinking capabilities
- **OpenAI**: Added o4-mini reasoning model, updated context limits
- **LM Studio**: Changed default from gemma2-9b-it to gemma-3n-e4b-it
- **OpenRouter**: Updated model references to latest versions

#### Default Model Changes

- Anthropic: claude-3-5-sonnet-20241022 → claude-sonnet-4-5-20250929
- xAI: grok-beta → grok-4-1-fast
- Gemini: gemini-1.5-pro → gemini-2.5-flash
- LM Studio: gemma2-9b-it → gemma-3n-e4b-it

#### Other Changes

- `sources_to_message_context()` uses `file_path` (then `document_id`) for source title instead of `source_type`
- Added `resolve_chunk_file_paths()` helper in query handler for reusable document name resolution from KV metadata
- **SDK updates**: Added `file_path` field to Rust, Java, and Kotlin SDK source reference types (Python and TypeScript already had it)
- Updated workspace version to 0.2.4
- Improved PATCH semantics for nullable fields in API and storage layers
- Refactored embedding batch calculation to use `.div_ceil()` (clippy compliance)
- Fixed consecutive `str::replace` calls in build scripts (clippy compliance)

### Fixed

- **Query/Chat source references show "chunk" instead of document name**: `sources_to_message_context()` was using `source_type` (always `"chunk"`) as the title. Now resolves `document_id` to actual document title from KV metadata. Affects `/api/v1/query`, `/api/v1/chat/completions`, and streaming endpoints
- **WebUI stored conversations**: Frontend `convertServerMessage` now uses `title` as fallback for `file_path` when displaying source citations from persisted conversations
- PATCH API for conversations now correctly distinguishes between "no change", "set to null", and "set to value" for folder assignment using `Option<Option<Uuid>>` pattern
- Moving conversations to/from folders now works reliably (E2E tested)
- Test assertions for LM Studio default model
- Provider status card configuration snippets
- Cost tracking consistency across all providers
- TypeScript build error in dashboard: removed non-existent `entity_type_count` property reference
- Visual feedback for tenant/workspace switching in the knowledge graph view

### Deprecated

- **gpt-4-turbo**: Superseded by gpt-4o and o4-mini (still functional, marked deprecated)
- **gpt-3.5-turbo**: Superseded by gpt-4o-mini (still functional, marked deprecated)

### Removed

- **gpt-oss:20b**: Removed from default model catalog

### Migration Notes

- No database migrations required for multi-provider support - cost tracking infrastructure already in place
- Existing cost data remains valid
- New pricing automatically applies to future operations
- Provider configurations are backwards compatible
- Lineage/metadata KV keys (`{id}-lineage`, `{id}-metadata`) only populated for newly processed documents
- Existing documents continue to work; lineage data appears after reprocessing

### Breaking Changes

None - all changes are additive or deprecations with backwards compatibility

## [v0.2.1] - 2026-02-12

### Fixed

- Fixed TypeScript build error in dashboard: removed non-existent `entity_type_count` property reference
- Visual feedback for tenant/workspace switching in the knowledge graph view

## [v0.2.4] - 2026-02-17

### Added

- "Unfiled" filter for conversations: displays all conversations not assigned to a folder
- Frontend and backend support for filtering by unfiled conversations

### Fixed

- PATCH API for conversations now correctly distinguishes between "no change", "set to null", and "set to value" for folder assignment using `Option<Option<Uuid>>` pattern
- Moving conversations to/from folders now works reliably (E2E tested)

### Changed

- Updated workspace version to 0.2.4
- Improved PATCH semantics for nullable fields in API and storage layers

- Loading overlay with minimum 800ms duration during workspace/tenant transitions
- Toast notifications for tenant and workspace switch confirmation
- Early return guard for same tenant/workspace selection (no-op)
- Toast deduplication using IDs to prevent duplicate notifications
- Loading overlay now always appears during workspace/tenant switch, even for empty/fast workspaces
- Only one toast notification is shown per switch (no duplicates)
- No notification or reload when selecting the same tenant/workspace
- See [SDKs documentation](sdks/) and [SDK changelogs](sdks/python/CHANGELOG.md, sdks/typescript/CHANGELOG.md, etc.) for language-specific updates.

---

## SDKs

EdgeQuake provides official SDKs for multiple languages. See the following for details and changelogs:

- [Python SDK](sdks/python/README.md) ([Changelog](sdks/python/CHANGELOG.md))
- [TypeScript SDK](sdks/typescript/README.md) ([Changelog](sdks/typescript/CHANGELOG.md))
- [Other SDKs](sdks/) for C#, Go, Java, Kotlin, PHP, Ruby, Rust, Swift

---

For a full project history, see the [README.md](README.md) and documentation in [docs/].
