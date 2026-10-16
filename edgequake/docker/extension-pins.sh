#!/usr/bin/env bash
# SSOT — PostgreSQL extension version pins for EdgeQuake (SPEC-042 / Issue #161).
#
# Profiles (multi-major — SPEC-042-C):
#   pg16 / pg17 / pg18 — product images: pgvector 0.8.5, no Apache AGE
#   pg18-vectorscale — study image only; still builds AGE 1.8.0
#
# Usage:
#   source extension-pins.sh                    # defaults to pg18 (recommended)
#   EQ_POSTGRES_PROFILE=pg16 source extension-pins.sh
#   make dev-pg16 | make dev-pg17 | make dev-pg18
#
# Consumers:
#   - Dockerfile.postgres / .pg17 / .pg18
#   - verify-postgres-extensions.sh
#   - Makefile postgres-image-build*
#   - scripts/migrate_postgres_major.sh
#   - specs/042-update-age-pgvector/e2e/*
set -euo pipefail

_profile="${EQ_POSTGRES_PROFILE:-pg18}"

case "$_profile" in
  pg18)
    export EQ_POSTGRES_MAJOR="18"
    export EQ_POSTGRES_IMAGE="postgres:18-bookworm"
    export EQ_PGVECTOR_VERSION="v0.8.5"
    export EQ_PGVECTOR_MIN="0.8.5"
    export EQ_AGE_GIT_REF=""
    export EQ_AGE_MIN=""
    export EQ_POSTGRES_DOCKERFILE="Dockerfile.postgres.pg18"
    export EQ_POSTGRES_IMAGE_TAG="edgequake-postgres:local"
    export EQ_POSTGRES_GHCR_SUFFIX="pg18"
    export EQ_PGVECTORSCALE_VERSION=""
    export EQ_PGVECTORSCALE_MIN=""
    ;;
  pg18-vectorscale)
    # SPEC-070 — opt-in DiskANN study image (not product default)
    export EQ_POSTGRES_MAJOR="18"
    export EQ_POSTGRES_IMAGE="postgres:18-bookworm"
    export EQ_PGVECTOR_VERSION="v0.8.5"
    export EQ_PGVECTOR_MIN="0.8.5"
    export EQ_AGE_GIT_REF="PG18/v1.8.0-rc0"
    export EQ_AGE_MIN="1.8.0"
    export EQ_PGVECTORSCALE_VERSION="0.9.0"
    export EQ_PGVECTORSCALE_MIN="0.9.0"
    export EQ_POSTGRES_DOCKERFILE="Dockerfile.postgres.pg18-vectorscale"
    export EQ_POSTGRES_IMAGE_TAG="edgequake-postgres:pg18-vectorscale"
    export EQ_POSTGRES_GHCR_SUFFIX="pg18-vectorscale"
    ;;
  pg17)
    export EQ_POSTGRES_MAJOR="17"
    export EQ_POSTGRES_IMAGE="postgres:17-bookworm"
    export EQ_PGVECTOR_VERSION="v0.8.5"
    export EQ_PGVECTOR_MIN="0.8.5"
    export EQ_AGE_GIT_REF=""
    export EQ_AGE_MIN=""
    export EQ_POSTGRES_DOCKERFILE="Dockerfile.postgres.pg17"
    export EQ_POSTGRES_IMAGE_TAG="edgequake-postgres:pg17"
    export EQ_POSTGRES_GHCR_SUFFIX="pg17"
    export EQ_PGVECTORSCALE_VERSION=""
    export EQ_PGVECTORSCALE_MIN=""
    ;;
  pg16)
    export EQ_POSTGRES_MAJOR="16"
    export EQ_POSTGRES_IMAGE="postgres:16-bookworm"
    export EQ_PGVECTOR_VERSION="v0.8.5"
    export EQ_PGVECTOR_MIN="0.8.5"
    export EQ_AGE_GIT_REF=""
    export EQ_AGE_MIN=""
    export EQ_POSTGRES_DOCKERFILE="Dockerfile.postgres"
    export EQ_POSTGRES_IMAGE_TAG="edgequake-postgres:pg16"
    export EQ_POSTGRES_GHCR_SUFFIX="pg16"
    export EQ_PGVECTORSCALE_VERSION=""
    export EQ_PGVECTORSCALE_MIN=""
    ;;
  *)
    echo "Unknown EQ_POSTGRES_PROFILE: $_profile (expected pg16|pg17|pg18|pg18-vectorscale)" >&2
    exit 1
    ;;
esac

unset _profile
