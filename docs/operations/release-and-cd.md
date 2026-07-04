# Release & CD Cycle

This document describes how to cut a release, run quality gates, and verify the published Docker images.

## 1) Local Release Gates (must pass before tag)

```bash
make release-gates          # fmt + clippy (all crates) + resource + observability proofs
make spec020-qc-proof-strict # SPEC-020 E2E (migration-038 strict)
make spec020-qc-proof-full    # SPEC-020 + require Ollama (0 skips)
make stop
make spec013-proof-pr
cd edgequake && cargo clippy -p edgequake-pipeline -p edgequake-core -p edgequake-api --all-targets --features postgres -- -D warnings
cd ../edgequake_webui && bunx tsc --noEmit -p tsconfig.release.json
cd .. && make backend-bg frontend-bg && make spec013-proof-ui
```

## 2) CI Validation (GitHub Actions)

- `Release Gates` workflow must be green (or tag push runs preflight in `release-docker.yml`).
- `SPEC-013 PR Proof`, `CI`, `Test Quality Gates`, and integration tests must be green.
- Ignore unrelated external automation failures (for example Dependabot noise) only if all required project gates are green.

## 3) Cut Release (CD publish)

```bash
# Example
git tag v0.14.0
git push origin v0.14.0
```

This triggers `.github/workflows/release-docker.yml`, which:
- builds/publishes multi-arch API, frontend, and **triple-track** postgres images (`:VERSION`, `:VERSION-pg16`, `:VERSION-pg17`, `:VERSION-pg18`) to GHCR
- creates/updates the GitHub Release notes for that tag

## 4) Post-Publish Verification

```bash
gh release view v0.14.0
docker buildx imagetools inspect ghcr.io/raphaelmansuy/edgequake:0.14.0
docker buildx imagetools inspect ghcr.io/raphaelmansuy/edgequake-frontend:0.14.0
docker buildx imagetools inspect ghcr.io/raphaelmansuy/edgequake-postgres:0.14.0
docker buildx imagetools inspect ghcr.io/raphaelmansuy/edgequake-postgres:0.14.0-pg16
docker buildx imagetools inspect ghcr.io/raphaelmansuy/edgequake-postgres:0.14.0-pg17
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
git tag v0.14.0 && git push origin v0.14.0
```

Both `linux/amd64` (ubuntu-latest runner) and `linux/arm64` (native ARM64 runner — no QEMU) are built in parallel and merged into a single multi-arch manifest. The same image tag (`ghcr.io/raphaelmansuy/edgequake:0.14.0`) works on x86 servers, Apple Silicon Macs, and AWS Graviton instances.

You can also trigger a manual Docker build + publish without a tag via the `workflow_dispatch` input on GitHub Actions (`Actions -> Release -- Docker (GHCR) -> Run workflow`).

## Building the Image Locally

The Dockerfile lives at `edgequake/docker/Dockerfile` and uses a two-stage build (Rust builder -> Debian slim runtime). pdfium is embedded at compile time via `pdfium-auto` -- no external shared library is needed.

```bash
# Build for host architecture
docker build -f edgequake/docker/Dockerfile edgequake -t edgequake:local

# Multi-platform build (requires docker buildx)
docker buildx build \
  --platform linux/amd64,linux/arm64 \
  -f edgequake/docker/Dockerfile edgequake \
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
| PG16 | 16.x | 0.8.3 | 1.6.0 | Legacy, stable |
| PG17 | 17.x | 0.8.3 | 1.7.0 | Modern, recommended for most users |
| PG18 | 18.x | 0.8.3 | 1.7.0 | Default, includes `uuidv7()` |

See [PostgreSQL migration guide](../migrations/postgres-triple-track-spec042.md) for tier details.
