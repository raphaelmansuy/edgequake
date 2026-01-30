# HackerNews Post: Production Deployment

---

**Title:** What I Learned Building Production-Ready Graph-RAG in Rust

---

I've been working on a Graph-RAG system that combines knowledge graphs with vector search. After deploying to production a few times (and experiencing the inevitable incidents), I've collected some patterns that might be useful for others building similar systems.

## The Gap Between Demo and Production

Most RAG frameworks are designed for notebooks. They're excellent for prototyping—you can have something working in an afternoon. But production is a different environment:

- Kubernetes needs health endpoints to probe
- PostgreSQL connections exhaust under load
- SIGTERM during transactions corrupts data
- Your on-call rotation needs a runbook

Every team I've talked to ends up building these patterns from scratch. It's 3-6 months of DevOps work that nobody budgets for.

## What We Built

**Health Probes**: Three endpoints—`/live`, `/ready`, `/health`. Kubernetes liveness probes kill stuck containers. Readiness probes remove pods from service during initialization (extension setup, connection pool warming). The `/health` endpoint returns component-level status.

**Connection Pooling**: We use SQLx with lazy pool initialization. The pool auto-enables pgvector and Apache AGE on first connect. Default max connections is 10, configurable per workload. Acquire timeout is 30s—fail fast if the pool is exhausted.

```rust
PgPoolOptions::new()
    .max_connections(config.max_connections)
    .min_connections(config.min_connections)
    .acquire_timeout(config.connect_timeout)
    .idle_timeout(Some(config.idle_timeout))
```

**Multi-Stage Docker**: Stage 1 builds with `rust:1.78-bookworm`. Stage 2 copies only the binary to `debian:bookworm-slim`. Final image is ~100MB instead of ~2GB. Non-root user by default. Built-in `HEALTHCHECK` for container orchestrators.

**Stateless API**: All state lives in PostgreSQL—graph (AGE), vectors (pgvector), documents. API servers are interchangeable. Scaling is `replicas: N` without session affinity.

**Graceful Shutdown**: SIGTERM triggers connection draining. In-flight requests complete. Database connections close cleanly. No mid-transaction kills.

## What I'm Still Figuring Out

1. **Metrics export**: We use tracing for structured logs, but proper Prometheus metrics would be better. What's the standard approach for Rust services?

2. **Blue-green deployments**: With stateless APIs, this should be straightforward, but graph schema migrations complicate things. How do others handle schema evolution?

3. **Cost observability**: We track LLM costs per document, but production teams want dashboards. What's the integration story for Grafana/Datadog?

## The Stack

- Rust + Axum for the API
- PostgreSQL 16 with pgvector and Apache AGE
- SQLx for connection pooling
- Tracing for structured logging

The codebase is open source if anyone wants to look at the implementation: [github link]

Curious what production patterns others have found essential for AI/ML systems. The gap between "it works in a notebook" and "it runs at 3am without paging anyone" seems consistently underestimated.
