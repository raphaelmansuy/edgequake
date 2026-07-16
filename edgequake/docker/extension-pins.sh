#!/usr/bin/env bash
# SSOT — PostgreSQL extension version pins for EdgeQuake (SPEC-042 / Issue #161).
#
# Profiles (multi-major — SPEC-042-C):
#   pg16  — legacy supported: AGE 1.6.0 (existing deployments)
#   pg17  — modern supported: AGE 1.7.0 (managed PG17, full #161)
#   pg18  — recommended:      AGE 1.7.0 (new installs, longest support runway)
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
    export EQ_AGE_GIT_REF="PG18/v1.7.0-rc0"
    export EQ_AGE_MIN="1.7.0"
    export EQ_POSTGRES_DOCKERFILE="Dockerfile.postgres.pg18"
    export EQ_POSTGRES_IMAGE_TAG="edgequake-postgres:local"
    export EQ_POSTGRES_GHCR_SUFFIX="pg18"
    ;;
  pg17)
    export EQ_POSTGRES_MAJOR="17"
    export EQ_POSTGRES_IMAGE="postgres:17-bookworm"
    export EQ_PGVECTOR_VERSION="v0.8.5"
    export EQ_PGVECTOR_MIN="0.8.5"
    export EQ_AGE_GIT_REF="PG17/v1.7.0-rc0"
    export EQ_AGE_MIN="1.7.0"
    export EQ_POSTGRES_DOCKERFILE="Dockerfile.postgres.pg17"
    export EQ_POSTGRES_IMAGE_TAG="edgequake-postgres:pg17"
    export EQ_POSTGRES_GHCR_SUFFIX="pg17"
    ;;
  pg16)
    export EQ_POSTGRES_MAJOR="16"
    export EQ_POSTGRES_IMAGE="postgres:16-bookworm"
    export EQ_PGVECTOR_VERSION="v0.8.5"
    export EQ_PGVECTOR_MIN="0.8.5"
    export EQ_AGE_GIT_REF="PG16/v1.6.0-rc0"
    export EQ_AGE_MIN="1.6.0"
    export EQ_POSTGRES_DOCKERFILE="Dockerfile.postgres"
    export EQ_POSTGRES_IMAGE_TAG="edgequake-postgres:pg16"
    export EQ_POSTGRES_GHCR_SUFFIX="pg16"
    ;;
  *)
    echo "Unknown EQ_POSTGRES_PROFILE: $_profile (expected pg16|pg17|pg18)" >&2
    exit 1
    ;;
esac

unset _profile
