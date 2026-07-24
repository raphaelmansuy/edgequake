---
title: "Release & CD Cycle"
---

# Release & CD Cycle

> **Product: v0.21.2** · Contract: OpenAPI · Spec ops: [Ingestion cancel & fairness](../ingestion-cancel-and-fairness.md)

This document describes how to cut a release, run quality gates, and verify the published Docker images.

**CD model:** workspace crates are **not** published to crates.io (`cargo-release --no-publish`). Product delivery is **GHCR Docker** via `git tag vX.Y.Z` → `release-docker.yml`. Use `cargo package` / `publish --dry-run` only to prove packaging readiness.

## 1) Local Release Gates (must pass before tag)

```bash
make ops17-smoke            # PG pin SSOT (fast, no Docker)
make spec046-acc            # SPEC-046 Hybrid RAG ACC + JSON artifact
make codegen-openapi-refresh # OpenAPI snapshot + schema.d.ts from ApiDoc
cd edgequake && cargo test -p edgequake-api --test spec027_api_contract && cd ..
make release-gates          # fmt + workspace clippy + SPEC-006/018 + WebUI + version/OpenAPI parity
make test-e2e-lint          # Playwright flake anti-patterns
# Optional deeper proofs:
make spec020-qc-proof-strict # SPEC-020 E2E (migration-038 strict)
make spec020-qc-proof-full    # SPEC-020 + require Ollama (0 skips)
make stop
make spec013-proof-pr
cd edgequake && cargo clippy -p edgequake-pipeline -p edgequake-core -p edgequake-api --all-targets --features postgres -- -D warnings
cd ../edgequake_webui && bunx tsc --noEmit -p tsconfig.release.json
cd .. && make backend-bg frontend-bg && make spec013-proof-ui
```

`make release-gates` uses workspace clippy as SSOT. Set `RELEASE_SKIP_PER_CRATE_CLIPPY=0` locally if you want the slower O(N) per-crate loop. CI always sets `RELEASE_SKIP_LIB_TESTS=1` and `RELEASE_SKIP_PER_CRATE_CLIPPY=1` because `CI.yml` already owns the lib suite.

**OpenAPI / Swagger (required before tag):** regenerate with `make codegen-openapi-refresh`, then run `cargo test -p edgequake-api --test spec027_api_contract`. Live check: `curl -s http://localhost:8080/api-docs/openapi.json | jq -r '.info.version'` must equal `VERSION`.

**Package dry-run (not crates.io upload):**

```bash
cd edgequake
for c in edgequake-observability edgequake-storage edgequake-pdf edgequake-pipeline \
         edgequake-query edgequake-tasks edgequake-auth edgequake-audit \
         edgequake-rate-limiter edgequake-core edgequake-api; do
  cargo package -p "$c" --allow-dirty --no-verify 2>/dev/null || cargo package -p "$c" --list >/dev/null
done
```

## 2) CI Validation (GitHub Actions)

- `CI` (fmt/clippy/nextest lib/docs/build) must be green.
- `Test Quality Gates` (invariants, test-count floor, e2e lint/UI) must be green.
- `Release Gates` must be green (or tag push runs preflight in `release-docker.yml`).
- `SPEC-046 ACC` must be green when query/storage/spec paths change.
- `SPEC-013 PR Proof` and postgres integration tests must be green when those paths change.
- Ignore unrelated external automation failures (for example Dependabot noise) only if all required project gates are green.

**Speed knobs (first principles):** shared Swatinem cache (`shared-key: edgequake-ci`), sparse crates.io, no incremental, `--locked`, cancel-in-progress, no duplicate workspace lib compile in Quality Gates / Release Gates.

**Docker CD anti-flake gates (in `make release-gates` / `scripts/release_gates.sh`):**
- `scripts/check_docker_api_context.sh` — Cargo `[[bench]]`/`[[example]]` paths must exist; Dockerfile must `COPY` them; `.dockerignore` must not exclude them.
- `next.config.ts` SizeLimit guard — `proxyClientMaxBodySize` must be numeric (`DEFAULT_MAX_UPLOAD_BYTES`).
- README badge version must match `VERSION` / Cargo / package.json.
- Per-crate package versions must be `version.workspace = true` or equal `VERSION`.
- `edgequake_webui/openapi/openapi.snapshot.json` `info.version` must equal `VERSION`.

## 3) Cut Release (CD publish)

```bash
# Example (current cut)
git tag v0.21.2
git push origin v0.21.2
```

This triggers `.github/workflows/release-docker.yml`, which:
- builds/publishes multi-arch API, frontend, and **triple-track** postgres images (`:VERSION`, `:VERSION-pg16`, `:VERSION-pg17`, `:VERSION-pg18`) to GHCR
- creates/updates the GitHub Release notes for that tag

## 4) Post-Publish Verification

```bash
gh release view v0.21.2
docker buildx imagetools inspect ghcr.io/raphaelmansuy/edgequake:0.21.2
docker buildx imagetools inspect ghcr.io/raphaelmansuy/edgequake-frontend:0.21.2
docker buildx imagetools inspect ghcr.io/raphaelmansuy/edgequake-postgres:0.21.2
docker buildx imagetools inspect ghcr.io/raphaelmansuy/edgequake-postgres:0.21.2-pg16
docker buildx imagetools inspect ghcr.io/raphaelmansuy/edgequake-postgres:0.21.2-pg17
```

## SPEC-042 Verification (before tag)

```bash
make check-extension-pins          # pg16 + pg17 + pg18 pin SSOT
make spec042-battle-test-all       # docker battle suite (all tiers + #275)
make dev-e2e-proof-all             # dev-stack /health proof per profile
```

## CI/CD — Automated Releases

Docker images are built and published automatically via GitHub Actions (`.github/workflows/release-docker.yml`) when a version tag is pushed:

```bash
# Tag a release — triggers multi-arch docker build + publish to ghcr.io
git tag v0.21.2 && git push origin v0.21.2
```

Both `linux/amd64` (ubuntu-latest runner) and `linux/arm64` (native ARM64 runner — no QEMU) are built in parallel and merged into a single multi-arch manifest. The same image tag (`ghcr.io/raphaelmansuy/edgequake:0.21.2`) works on x86 servers, Apple Silicon Macs, and AWS Graviton instances.

You can also trigger a manual Docker build + publish without a tag via the `workflow_dispatch` input on GitHub Actions (`Actions -> Release -- Docker (GHCR) -> Run workflow`).

**Republish tip:** `gh workflow run "Release — Docker (GHCR)" --ref release/vX.Y.Z -f tag_name=vX.Y.Z` builds from the release branch (including post-tag CD fixes) while still publishing the `X.Y.Z` / `latest` GHCR tags. Use this when the git tag already exists but Docker CD needs a fix commit.

## Building the Image Locally

The Dockerfile lives at `edgequake/docker/Dockerfile` and uses a two-stage build (Rust builder → Debian slim runtime). **Build context is the monorepo root** — `edgequake-pdf2md` is pulled from crates.io at compile time (no sibling checkout). pdfium is embedded via `pdfium-auto`; no external shared library is needed.

```bash
# Build for host architecture (from repo root)
docker build -f edgequake/docker/Dockerfile . -t edgequake:local

# Multi-platform build (requires docker buildx)
docker buildx build \
  --platform linux/amd64,linux/arm64 \
  -f edgequake/docker/Dockerfile . \
  -t edgequake:local --load
```

## Development Workflow

See [AGENTS.md](../../AGENTS.md) for the full developer workflow, including:
- Make commands for building, testing, and linting
- Database migrations and resource safety
- Agent-driven specification workflow

## Docker Images Published Per Release

| Image | Tags | Description |
|-------|------|-------------|
| `ghcr.io/raphaelmansuy/edgequake` | `VERSION`, `latest` | Backend API server |
| `ghcr.io/raphaelmansuy/edgequake-frontend` | `VERSION`, `latest` | Next.js web UI |
| `ghcr.io/raphaelmansuy/edgequake-postgres` | `VERSION`, `VERSION-pg16`, `VERSION-pg17`, `VERSION-pg18`, `latest` | PostgreSQL with pgvector + AGE |

## PostgreSQL Version Tiers

| Tier | PostgreSQL | pgvector | Apache AGE | Notes |
|------|-----------|----------|-----------|-------|
| PG16 | 16.x | 0.8.5 | 1.6.0 | Legacy, stable |
| PG17 | 17.x | 0.8.5 | 1.7.0 | Modern supported |
| PG18 | 18.x | 0.8.5 | 1.8.0 | Default / recommended (SPEC-068 pin) |

See [PostgreSQL migration guide](../../edgequake/docs/migrations/postgres-triple-track-spec042.md) for tier details.

## Lessons from 0.21.0 cut

- **OpenAPI refresh is mandatory** after `version-bump` — `openapi.snapshot.json` `info.version` must equal `VERSION` or release-gates fail.
- **X-30 typed timeouts** — vision stall messages must carry `Operation timed out` + `failure_class=`; `from_processing_error` must use the timeout factory or the progress-aware breaker never trips.
- **Frontend CD Dockerfile** — `release-docker.yml` builds [`edgequake_webui/Dockerfile`](../../edgequake_webui/Dockerfile) (not `edgequake/docker/Dockerfile.frontend`); must `COPY openapi/` and prefer relative imports for `schema.d.ts` (`@/*` shadows `@/openapi/*`).
- **medical-full raw JSON** — predictions/eval exceed GitHub 100MB; keep scorecards only; gitignore patterns under `artifacts/medical-full*` / `history/medical-full-*`.
- **Do not use `make version-tag`** — it auto-pushes; prefer explicit `git tag` + `git push origin vX.Y.Z` after local gates.

## Lessons from 0.20.2 cut

- **OpenAPI refresh is mandatory** after `version-bump` — `openapi.snapshot.json` `info.version` must equal `VERSION` or release-gates fail.
- **Workspace ≠ crates.io** — bump all members together; dry-run `cargo package`; ship via GHCR tag only.
- Soft-label only — clean opaque AGE node ids still need re-ingest.
- **Do not use `make version-tag`** — it auto-pushes; prefer explicit `git tag` + `git push origin vX.Y.Z` after local gates.

## Lessons from 0.20.1 cut

- **OpenAPI refresh is mandatory** after `version-bump` — `openapi.snapshot.json` `info.version` must equal `VERSION` or release-gates fail.
- **Workspace ≠ crates.io** — bump all members together; dry-run `cargo package`; ship via GHCR tag only.
- **Patch merge with baseline flakes** — AGE neighbor / SPEC-013 AGE `LOAD` can stay red when triaged as baseline; do not merge past SPEC-006 / clippy / workspace lib / release-gates failures.
- **Do not use `make version-tag`** — it auto-pushes; prefer explicit `git tag` + `git push origin vX.Y.Z` after local gates.

## Lessons from 0.20.0 cut

- **Benchmark JSON** — smoke eval/prediction artifacts may be committed when operators want reproduceability; still prefer publish-pack pointers for Acc claims.
- **Acc language** — cite statistical tie / fair cold ~1.01×; do not claim Acc Beat win; warm LR “speed” was cache ([063](../../specs/001-benchmark/001-edgquake-improvements/063-why-lightrag-faster-cache-fairness.md)).
