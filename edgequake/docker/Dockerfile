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
# Use TARGETPLATFORM so QEMU/BuildKit handles cross-compilation transparently.
# For native builds this is a no-op; for multi-platform buildx this enables
# correct arch-native pdfium download inside pdfium-auto.
ARG TARGETPLATFORM
ARG BUILDPLATFORM
FROM --platform=${TARGETPLATFORM} rust:bookworm AS builder

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

# Build release binary with SQLx offline mode (no database needed at build time).
# pdfium-auto downloads and embeds the correct pdfium binary for TARGETPLATFORM.
ENV SQLX_OFFLINE=true
RUN cargo build --release --locked

# ============================================
# Stage 2: Runtime
# ============================================
ARG TARGETPLATFORM
FROM --platform=${TARGETPLATFORM} debian:bookworm-slim AS runtime

WORKDIR /app

# Install runtime dependencies (no pdfium dylib needed — embedded in binary)
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    curl \
    && rm -rf /var/lib/apt/lists/* \
    && useradd -r -s /bin/false edgequake

# Copy binary from builder
COPY --from=builder /app/target/release/edgequake /usr/local/bin/

# Set ownership
RUN chown edgequake:edgequake /usr/local/bin/edgequake

# Switch to non-root user
USER edgequake

# Expose API port
EXPOSE 8080

# Health check
HEALTHCHECK --interval=30s --timeout=10s --start-period=5s --retries=3 \
    CMD curl -f http://localhost:8080/health || exit 1

# Run the server
ENTRYPOINT ["edgequake"]
