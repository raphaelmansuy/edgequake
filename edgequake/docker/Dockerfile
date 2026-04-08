# EdgeQuake Dockerfile
# Multi-stage build for minimal image size
# Multi-platform: supports linux/amd64, linux/arm64 (macOS Apple Silicon, AWS Graviton, etc.)
# Build: docker buildx build --platform linux/amd64,linux/arm64 -t edgequake .
# Or:    docker compose build  (uses DOCKER_DEFAULT_PLATFORM or host default)
#
# SPEC-040 v0.4.1: pdfium is embedded at compile time via pdfium-auto — no external
# libpdfium.dylib or PDFIUM_DYNAMIC_LIB_PATH is needed. The build downloads and
# embeds pdfium for the target platform automatically.

# ============================================
# Stage 1: Build
# ============================================
# TARGETPLATFORM / BUILDPLATFORM are populated automatically by Docker BuildKit
# when using `docker buildx build --platform ...`.  In a plain `docker build`
# or `docker compose build` the variables are empty, so we must NOT interpolate
# them into `FROM --platform=` (an empty value is rejected by the daemon with an
# "invalid component" error – issue #92).  We therefore let Docker choose the
# base-image platform from the manifest, which gives native builds for free and
# keeps cross-compilation working via buildx without extra flags.
ARG TARGETPLATFORM
ARG BUILDPLATFORM
FROM rust:bookworm AS builder

WORKDIR /app

# Install build dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# Copy workspace files
COPY Cargo.toml Cargo.lock ./
COPY crates/ crates/
COPY src/ src/
COPY migrations/ migrations/
COPY .sqlx .sqlx/
# WHY: models.toml defines available LLM providers/models (gpt-5.4, claude-sonnet-4-6,
# gemma4, etc.). The binary falls back to built-in defaults (edgequake-llm v0.5.1) which
# only include gpt-4o and gpt-4o-mini — the new models are only in models.toml.
# We copy it into the build layer so Stage 2 can retrieve it via --from=builder.
COPY models.toml models.toml

# Build release binary with SQLx offline mode (no database needed at build time).
# pdfium-auto downloads and embeds the correct pdfium binary for TARGETPLATFORM.
ENV SQLX_OFFLINE=true
RUN cargo build --release --locked

# ============================================
# Stage 2: Runtime
# ============================================
# See builder-stage comment above for why --platform is omitted.
FROM debian:bookworm-slim AS runtime

WORKDIR /app

# Install runtime dependencies (no pdfium dylib needed — embedded in binary)
# Also create the pdfium-auto cache directory that is writable by the non-root
# edgequake user at runtime (fix #100: "Permission denied" when pdfium-auto
# tries to extract its embedded library on first use).
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    curl \
    && rm -rf /var/lib/apt/lists/* \
    && useradd -r -s /bin/false edgequake \
    && mkdir -p /tmp/edgequake-pdfium-cache \
    && chmod 1777 /tmp/edgequake-pdfium-cache

# Tell pdfium-auto where to write its extracted shared library.  /tmp is always
# writable by all users; the sticky-bit above prevents other processes from
# removing files they do not own.  Users may override via PDFIUM_AUTO_CACHE_DIR.
ENV PDFIUM_AUTO_CACHE_DIR=/tmp/edgequake-pdfium-cache

# Copy binary from builder
COPY --from=builder /app/target/release/edgequake /usr/local/bin/
# Copy models.toml so the runtime binary finds up-to-date models at ./models.toml
# (CWD=/app, see WORKDIR above). Without this, the binary falls back to the compiled-in
# defaults which lack the v0.9.8 models (gpt-5.4, claude-sonnet-4-6, gemma4).
COPY --from=builder /app/models.toml /app/models.toml

# Set ownership
RUN chown edgequake:edgequake /usr/local/bin/edgequake /app/models.toml

# Switch to non-root user
USER edgequake

# Expose API port
EXPOSE 8080

# Health check
HEALTHCHECK --interval=30s --timeout=10s --start-period=5s --retries=3 \
    CMD curl -f http://localhost:8080/health || exit 1

# Run the server
ENTRYPOINT ["edgequake"]
