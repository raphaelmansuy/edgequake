# EdgeQuake Dockerfile
# Multi-stage build for minimal image size

# ============================================
# Stage 1: Build
# ============================================
# Use rust:bookworm to match runtime glibc version (debian:bookworm-slim)
FROM rust:bookworm AS builder

WORKDIR /app

# Install dependencies
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

# Build release binary with SQLx offline mode (no database needed)
ENV SQLX_OFFLINE=true
RUN cargo build --release --locked

# ============================================
# Stage 2: Runtime
# ============================================
FROM debian:bookworm-slim AS runtime

WORKDIR /app

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
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
