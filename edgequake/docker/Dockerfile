# EdgeQuake Dockerfile
# Multi-stage build for minimal image size

# ============================================
# Stage 1: Build
# ============================================
FROM rust:1.88-bookworm AS builder

WORKDIR /app

# Install dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# Copy workspace files
COPY Cargo.toml ./
COPY Cargo.lock* ./
COPY .sqlx/ .sqlx/
COPY crates/ crates/
COPY src/ src/
COPY benches/ benches/
COPY examples/ examples/
COPY migrations/ migrations/

# Enable SQLx offline mode
ENV SQLX_OFFLINE=true

# Build release binary
RUN cargo build --release

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

# Copy models.toml configuration
COPY models.toml /app/models.toml

# Set ownership
RUN chown edgequake:edgequake /usr/local/bin/edgequake && \
    chown edgequake:edgequake /app/models.toml

# Switch to non-root user
USER edgequake

# Expose API port
EXPOSE 8080

# Health check
HEALTHCHECK --interval=30s --timeout=10s --start-period=5s --retries=3 \
    CMD curl -f http://localhost:8080/health || exit 1

# Run the server
ENTRYPOINT ["edgequake"]
