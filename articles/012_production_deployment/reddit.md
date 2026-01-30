# Reddit Post: Production Deployment

---

**Subreddits:** r/devops, r/kubernetes, r/rust

---

**Title:** Lessons from deploying Graph-RAG to production (Kubernetes + PostgreSQL + Rust)

---

I've been working on deploying a Graph-RAG system (combines knowledge graphs with vector search) to production, and wanted to share some patterns that emerged. Not selling anything—genuinely curious what others do differently.

## Background

Built a RAG system in Rust that uses PostgreSQL with pgvector for embeddings and Apache AGE for graph storage. Single database, two extensions. After a few production deployments (and incidents), some patterns became non-negotiable.

## What We Learned

### 1. Health Endpoints Are Not Optional

Kubernetes probes need endpoints. We implemented three:

- `GET /live` → livenessProbe (is the process alive?)
- `GET /ready` → readinessProbe (is it ready for traffic?)
- `GET /health` → detailed component status

The `/ready` endpoint is crucial during startup. Our system needs to enable PostgreSQL extensions (pgvector, AGE) before it can serve traffic. Without readiness checks, Kubernetes routes traffic to pods that aren't ready, and users see errors during deployments.

### 2. Connection Pool Exhaustion is Real

Our first production incident was at 3am. PostgreSQL hit `max_connections` limit. Every request was opening a new connection.

Fix: SQLx connection pooling with sane defaults.

```rust
PgPoolOptions::new()
    .max_connections(10)   // Tune for your workload
    .min_connections(1)    // Keep pool warm
    .acquire_timeout(30s)  // Fail fast
    .idle_timeout(600s)    // Cleanup
```

We also auto-enable extensions on first connect, so you don't need separate migration jobs.

### 3. Multi-Stage Docker Builds Matter

Started with a single-stage Dockerfile (~2GB image). Switched to multi-stage:

- Stage 1: Build with `rust:1.78-bookworm`
- Stage 2: Copy binary to `debian:bookworm-slim`

Final image: ~100MB. Also added non-root user and built-in HEALTHCHECK.

### 4. Stateless API Servers Scale Better

All state lives in PostgreSQL:

- Knowledge graph → Apache AGE
- Vector embeddings → pgvector
- Documents → Standard tables

API servers are interchangeable. No session affinity. Scaling is literally `replicas: N`. Load balancer uses round-robin.

### 5. Graceful Shutdown Prevents Data Corruption

Entity extraction jobs can take seconds. If SIGTERM kills the process mid-transaction, you get orphaned graph relationships and partial documents.

We added a shutdown handler that:

1. Stops accepting new requests
2. Drains in-flight connections
3. Closes database connections cleanly

Kubernetes default grace period is 30s, which is enough for most requests.

### 6. Runbooks Shouldn't Be an Afterthought

We wrote a runbook _after_ our second incident. Should have done it earlier. It includes:

- Alert thresholds (p99 > 500ms warning, > 2s critical)
- Common issues and diagnosis commands
- Backup and recovery procedures
- Connection pool tuning guidance

## What We're Still Working On

1. **Prometheus metrics** - Currently using tracing for logs, want proper metrics export
2. **Graph schema migrations** - How do you handle schema evolution with Apache AGE? Currently doing it manually.
3. **Cost dashboards** - We track LLM costs per document but need better visualization

## Questions for the Community

- What production patterns have I missed?
- How do others handle Kubernetes + PostgreSQL + ML workloads?
- Any recommendations for graph schema migration tooling?

The gap between "works in development" and "runs at 3am without paging anyone" is consistently larger than estimated. Happy to share more details if anyone's interested.

---

_Edit: The system is open source if anyone wants to see the implementation details. Don't want to violate self-promotion rules, so ping me if you want the link._
