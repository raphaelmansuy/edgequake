# Production Deployment: From Dev to Scale

## Why Your RAG Framework Needs a Runbook, Not Just a README

_How EdgeQuake ships production-ready patterns so your SRE team doesn't have to build them_

---

The demo went perfectly.

The ML team had built an incredible Retrieval-Augmented Generation system. It extracted entities from documents, built knowledge graphs, and answered complex multi-hop questions with stunning accuracy. Leadership was impressed. The green light came for production deployment.

Then reality hit.

Three weeks into the production push, the SRE team discovered the framework had no health endpoints. Kubernetes couldn't probe it. The first deployment to staging resulted in pods stuck in `CrashLoopBackOff` because the orchestrator couldn't tell if the application was alive.

Week five brought the 3am page: connection pool exhaustion. The framework opened a new database connection for every request. At scale, PostgreSQL hit its `max_connections` limit and started rejecting queries. Documents stopped ingesting. Users saw errors.

Week eight revealed the shutdown problem. During a routine deployment, SIGTERM killed the process mid-transaction. Entity extraction jobs corrupted. The knowledge graph had orphaned relationships. The team spent a weekend cleaning up data.

**The ML team built an amazing RAG system. The SRE team spent three months making it production-ready.**

This pattern repeats across the industry. RAG frameworks optimize for developer experience in notebooks—rapid prototyping, easy experimentation, quick demos. They leave production concerns as an "exercise for the reader."

We decided to build production-readiness into EdgeQuake from day one.

---

## The Production Readiness Gap

Most RAG frameworks exist on a spectrum:

```
┌────────────────────────────────────────────────────────────────┐
│                 PRODUCTION READINESS SPECTRUM                  │
├────────────────────────────────────────────────────────────────┤
│                                                                │
│ Notebooks ◄──────────────────────────────────────► Production │
│                                                                │
│ LangChain    LlamaIndex    Haystack           EdgeQuake       │
│    │             │            │                   │            │
│    ▼             ▼            ▼                   ▼            │
│ [Prototype]  [Prototype]  [Prototype]       [Production]      │
│                                                                │
│ "Roll your own deployment"              "Ready out of the box"│
└────────────────────────────────────────────────────────────────┘
```

The hidden cost? **3-6 months of DevOps work per deployment.** Every team building production RAG with these frameworks re-invents:

- Dockerfiles with security best practices
- Health endpoints for container orchestrators
- Connection pooling and lifecycle management
- Graceful shutdown handlers
- Runbooks and operational documentation

This isn't a criticism of those frameworks—they're excellent for prototyping. But production is a different game.

---

## Docker: Multi-Stage Builds for Minimal Attack Surface

Container security starts with image size. A smaller image has fewer packages, fewer vulnerabilities, and a smaller attack surface.

EdgeQuake uses a multi-stage Docker build:

```dockerfile
# ============================================
# Stage 1: Build
# ============================================
FROM rust:1.78-bookworm AS builder

WORKDIR /app
COPY Cargo.toml Cargo.lock ./
COPY crates/ crates/
COPY src/ src/

# Build release binary with locked dependencies
RUN cargo build --release --locked

# ============================================
# Stage 2: Runtime (Minimal Image)
# ============================================
FROM debian:bookworm-slim AS runtime

# Install only runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates libssl3 \
    && rm -rf /var/lib/apt/lists/* \
    && useradd -r -s /bin/false edgequake

# Copy only the binary
COPY --from=builder /app/target/release/edgequake /usr/local/bin/

# Run as non-root user
USER edgequake

# Built-in health check
HEALTHCHECK --interval=30s --timeout=10s --start-period=5s --retries=3 \
    CMD curl -f http://localhost:8080/health || exit 1

ENTRYPOINT ["edgequake"]
```

**What this achieves:**

| Aspect          | Before (Full Toolchain) | After (Multi-Stage)           |
| --------------- | ----------------------- | ----------------------------- |
| Image Size      | ~2 GB                   | ~100 MB                       |
| Attack Surface  | Build tools, compilers  | Runtime only                  |
| User            | Root                    | Non-root                      |
| Health Check    | None                    | Built-in                      |
| Reproducibility | Varies                  | `--locked` ensures exact deps |

The `--locked` flag ensures the exact same dependencies build every time. No surprise updates between builds.

---

## Kubernetes-Ready Health Probes

Kubernetes uses probes to manage container lifecycle. Without them, the orchestrator can't tell if your application is alive, ready for traffic, or still starting up.

EdgeQuake implements all three:

| Probe Type    | Endpoint      | Purpose                   | Failure Action           |
| ------------- | ------------- | ------------------------- | ------------------------ |
| **Liveness**  | `GET /live`   | Is the process alive?     | Kill and restart         |
| **Readiness** | `GET /ready`  | Ready for traffic?        | Remove from Service      |
| **Startup**   | `GET /health` | Has it finished starting? | Don't check liveness yet |

```
┌─────────────────────────────────────────────────────────────────┐
│                       Kubernetes Cluster                        │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│   kubelet ──► livenessProbe ──► GET /live ──► 200 OK           │
│           │                                                     │
│           └─► readinessProbe ─► GET /ready ──► 200 OK          │
│                                                                 │
│   If /live fails  → Kill pod, restart                          │
│   If /ready fails → Remove from Service endpoints              │
│                     (stop sending traffic)                      │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

**Why readiness matters for RAG:**

Graph-RAG systems need initialization time:

- Database migrations must complete
- PostgreSQL extensions (pgvector, Apache AGE) need enabling
- Connection pools need warming
- LLM provider connections need establishing

A proper readiness probe ensures traffic only arrives after initialization completes. Without it, users see errors during deployments.

EdgeQuake's `/health` endpoint checks component status:

```json
{
  "status": "healthy",
  "components": [
    { "name": "storage", "healthy": true },
    { "name": "llm", "healthy": true },
    { "name": "graph", "healthy": true }
  ]
}
```

---

## Connection Pooling: The 3am Page Prevention

The most common production RAG failure we've seen? Connection pool exhaustion.

**The problem:** Naive database access opens a new connection per request. Under load, this exhausts PostgreSQL's `max_connections` limit (default 100). Queries start failing. Documents stop ingesting. The pager goes off at 3am.

**The solution:** Built-in connection pooling with sane defaults.

```rust
// EdgeQuake's connection pool initialization
let pool = PgPoolOptions::new()
    .max_connections(self.config.max_connections)  // Default: 10
    .min_connections(self.config.min_connections)  // Default: 1
    .acquire_timeout(self.config.connect_timeout)  // Default: 30s
    .idle_timeout(Some(self.config.idle_timeout))  // Default: 600s
    .connect(&self.config.connection_url())
    .await?;

// Auto-enable required extensions
sqlx::query("CREATE EXTENSION IF NOT EXISTS vector").execute(&pool).await?;
sqlx::query("CREATE EXTENSION IF NOT EXISTS age CASCADE").execute(&pool).await?;
```

**Configuration for different workloads:**

```toml
# High Throughput Profile
[storage]
max_connections = 50
min_connections = 10

[pipeline]
concurrency = 16

# Memory Constrained Profile
[storage]
max_connections = 5
min_connections = 1

[pipeline]
concurrency = 2
```

The pool automatically:

- Reuses connections across requests
- Maintains a minimum pool for responsiveness
- Times out requests waiting for connections (fail fast)
- Cleans up idle connections (release resources)
- Initializes PostgreSQL extensions on first connect

---

## Horizontal Scaling: Stateless by Design

Scaling RAG systems shouldn't require a PhD in distributed systems. EdgeQuake achieves horizontal scaling through stateless API servers.

```
                    Load Balancer
                         │
           ┌─────────────┼─────────────┐
           ▼             ▼             ▼
      ┌─────────┐   ┌─────────┐   ┌─────────┐
      │ API #1  │   │ API #2  │   │ API #3  │
      └────┬────┘   └────┬────┘   └────┬────┘
           │             │             │
           └─────────────┼─────────────┘
                         ▼
               ┌───────────────────┐
               │    PostgreSQL     │
               │  ┌─────┐ ┌─────┐  │
               │  │pgvec│ │ AGE │  │
               │  │ tor │ │     │  │
               │  └─────┘ └─────┘  │
               └───────────────────┘
```

**All state lives in PostgreSQL:**

- Knowledge graph: Apache AGE
- Vector embeddings: pgvector
- Documents and metadata: Standard tables
- Task queues: PostgreSQL-backed

**What this means for operations:**

- No session affinity required
- Any API server can handle any request
- Scale with `replicas: N` in your deployment
- Load balancer can use round-robin

```yaml
# docker-compose.yml
services:
  edgequake:
    image: edgequake:latest
    deploy:
      replicas: 3
    environment:
      - DATABASE_URL=postgres://...
```

For Kubernetes, scale is a single command:

```bash
kubectl scale deployment edgequake --replicas=5
```

---

## The Runbook: Operational Documentation Included

EdgeQuake ships with a 316-line runbook covering:

1. **Health Monitoring** - Endpoints, metrics, what to watch
2. **Alert Thresholds** - When to page, when to warn
3. **Common Issues** - Diagnosis and resolution procedures
4. **Scaling Procedures** - Horizontal and vertical guidance
5. **Backup and Recovery** - Database backup and DR procedures
6. **Security Procedures** - Key rotation, audit logs, incident response

**Example alert thresholds:**

| Metric          | Warning | Critical |
| --------------- | ------- | -------- |
| API p99 latency | > 500ms | > 2s     |
| Error rate      | > 1%    | > 5%     |
| Memory usage    | > 70%   | > 90%    |
| CPU usage       | > 70%   | > 90%    |
| Storage usage   | > 70%   | > 90%    |

**Example resolution procedure:**

````markdown
### Issue: Connection Pool Exhaustion

**Symptoms**: "connection pool exhausted" errors

**Diagnosis**:

```bash
psql -c "SELECT count(*), state FROM pg_stat_activity GROUP BY state;"
```
````

**Resolution**:

1. Increase pool size in configuration
2. Check for connection leaks
3. Implement connection timeouts
4. Restart service to reset pool

````

Most frameworks leave runbook creation to the deploying team. By the time they've written it, they've already experienced the incidents that required it.

---

## Graceful Shutdown: Data Integrity Under Deployment

The final production pattern: graceful shutdown.

**The problem:** Killing a process during a transaction corrupts data. Entity extraction jobs leave partial results. Graph relationships become orphaned.

**The solution:** Signal handling with connection draining.

```rust
let server = Server::bind(&addr)
    .serve(app)
    .with_graceful_shutdown(shutdown_signal());

async fn shutdown_signal() {
    tokio::signal::ctrl_c().await.expect("CTRL+C handler");
    info!("Received shutdown signal, draining connections...");
}
````

Kubernetes sends SIGTERM when stopping a pod. EdgeQuake:

1. Stops accepting new requests
2. Finishes in-flight requests
3. Closes database connections cleanly
4. Exits with status 0

The default grace period is 30 seconds. For long-running extraction jobs, configure `terminationGracePeriodSeconds` in your pod spec.

---

## Putting It All Together

Production readiness isn't a single feature—it's a collection of patterns that work together:

| Pattern            | Why It Matters                                |
| ------------------ | --------------------------------------------- |
| Multi-stage Docker | Security, smaller attack surface              |
| Health probes      | Kubernetes integration, zero-downtime deploys |
| Connection pooling | Reliability under load                        |
| Stateless API      | Horizontal scaling                            |
| Runbook            | Faster incident resolution                    |
| Graceful shutdown  | Data integrity                                |

EdgeQuake ships all of these out of the box. Your SRE team can focus on their domain expertise instead of reinventing production patterns.

---

## Getting Started

```bash
# Clone and start
git clone https://github.com/raphaelmansuy/edgequake
cd edgequake
docker-compose up -d

# Verify health
curl http://localhost:8080/health
# {"status": "healthy", "components": [...]}

# Scale to 3 replicas
docker-compose up -d --scale edgequake=3
```

**Production deployment in minutes, not months.**

---

## References

- **LightRAG Paper**: arXiv:2410.05779 - The graph-based RAG architecture EdgeQuake implements
- **Kubernetes Pod Lifecycle**: kubernetes.io/docs/concepts/workloads/pods/pod-lifecycle/
- **Microsoft GraphRAG**: Production patterns for enterprise graph retrieval

---

_EdgeQuake is an open-source Graph-RAG framework in Rust. Production-ready patterns included._

**GitHub**: github.com/raphaelmansuy/edgequake
