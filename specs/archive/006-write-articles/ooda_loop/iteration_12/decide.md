# Iteration 12: Production Deployment - DECIDE

## Article Plan

### Topic: 012_production_deployment

**Title**: "Production Deployment: From Dev to Scale"
**Subtitle**: "Why Your RAG Framework Needs a Runbook, Not Just a README"

---

## Medium Article Structure (~2200 words)

### 1. Hook (150 words)

**Start with a story**: An SRE team inherits a RAG prototype that "works perfectly in the demo." First production deployment reveals:

- No health endpoints (Kubernetes can't probe it)
- Unbounded database connections (pool exhaustion at 3am)
- No graceful shutdown (data corruption during deployments)
- No runbook (incident = panic)

**The twist**: "The ML team built an amazing RAG system. The SRE team spent three months making it production-ready. We decided to build production-readiness into the framework itself."

### 2. The Production Readiness Gap (300 words)

- Most RAG frameworks optimize for notebooks
- Production concerns left as "exercise for the reader"
- The hidden cost: 3-6 months of DevOps work per deployment

**ASCII Diagram: Production Readiness Spectrum**

```
┌────────────────────────────────────────────────────────────────┐
│                 PRODUCTION READINESS                           │
├────────────────────────────────────────────────────────────────┤
│ Notebooks ◄──────────────────────────────────► Production     │
│                                                                │
│ LangChain    LlamaIndex    Haystack    EdgeQuake              │
│    │             │            │            │                   │
│    ▼             ▼            ▼            ▼                   │
│ [Prototype]  [Prototype]  [Prototype]  [Production]           │
└────────────────────────────────────────────────────────────────┘
```

### 3. Docker: Multi-Stage Build for Minimal Attack Surface (350 words)

**Why it matters**: Container image size, security, reproducibility

**Code snippet**: EdgeQuake's multi-stage Dockerfile

```dockerfile
# Stage 1: Build
FROM rust:1.78-bookworm AS builder
RUN cargo build --release --locked

# Stage 2: Runtime (minimal)
FROM debian:bookworm-slim
COPY --from=builder /app/target/release/edgequake /usr/local/bin/
USER edgequake  # Non-root
HEALTHCHECK CMD curl -f http://localhost:8080/health || exit 1
```

**Key points**:

- ~100MB final image vs ~2GB with full toolchain
- Non-root user by default
- Built-in health check for container orchestrators
- `--locked` for reproducible builds

### 4. Kubernetes-Ready Health Probes (400 words)

**The three probe types**:

| Probe     | EdgeQuake Endpoint | Purpose                   |
| --------- | ------------------ | ------------------------- |
| Liveness  | `GET /live`        | Is the process alive?     |
| Readiness | `GET /ready`       | Is it ready for traffic?  |
| Startup   | `GET /health`      | Has it finished starting? |

**ASCII Diagram: Probe Architecture**

```
┌─────────────────────────────────────────────────────────┐
│                    Kubernetes                           │
├─────────────────────────────────────────────────────────┤
│ kubelet ──► livenessProbe ──► GET /live                │
│           │                                             │
│           └► readinessProbe ─► GET /ready              │
│                                                         │
│ If /live fails → Kill pod, restart                     │
│ If /ready fails → Remove from Service endpoints        │
└─────────────────────────────────────────────────────────┘
```

**Why readiness matters for RAG**:

- Database migrations need time
- LLM provider connection warm-up
- Extension initialization (pgvector, AGE)

### 5. Connection Pooling: The 3am Page Prevention (350 words)

**The problem**: Unbounded connections exhaust PostgreSQL limits

**The solution**: Built-in connection pooling with sane defaults

```rust
PgPoolOptions::new()
    .max_connections(10)   // Default, tune per workload
    .min_connections(1)    // Maintain at least one
    .acquire_timeout(30s)  // Fail fast on pool exhaustion
    .idle_timeout(600s)    // Clean up unused connections
```

**Configuration**:

```toml
[storage]
max_connections = 20
min_connections = 5
connection_timeout = 30
```

**Profiles**:

- High throughput: `max_connections = 50`
- Memory constrained: `max_connections = 5`

### 6. Horizontal Scaling: Stateless by Design (300 words)

**Why stateless matters**:

- Scale with replica count
- No session affinity required
- Load balancer friendly

**How EdgeQuake achieves this**:

- All state in PostgreSQL (graph, vectors, documents)
- No in-memory caches that require invalidation
- API servers are interchangeable

**Docker Compose scaling**:

```yaml
services:
  edgequake:
    deploy:
      replicas: 3
```

**ASCII Diagram: Horizontal Scaling**

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
                   ┌───────────┐
                   │PostgreSQL │
                   │ + pgvector│
                   │ + AGE     │
                   └───────────┘
```

### 7. The Runbook: Operational Documentation (250 words)

**What's included (316 lines)**:

- Health monitoring procedures
- Alert thresholds (p99 latency, error rate, resource usage)
- Common issues and resolutions
- Backup and recovery procedures
- Security procedures

**Alert thresholds example**:
| Metric | Warning | Critical |
|--------|---------|----------|
| API p99 latency | > 500ms | > 2s |
| Error rate | > 1% | > 5% |
| Memory usage | > 70% | > 90% |

### 8. Graceful Shutdown: Data Integrity (200 words)

**The problem**: Killing processes during deployment corrupts transactions

**The solution**: Signal handling with drain period

```rust
let server = Server::bind(&addr)
    .serve(app)
    .with_graceful_shutdown(shutdown_signal());

async fn shutdown_signal() {
    tokio::signal::ctrl_c().await.unwrap();
    info!("Draining connections...");
}
```

**Kubernetes termination**:

- SIGTERM sent → Start draining
- 30s grace period (configurable)
- SIGKILL if not done

### 9. Conclusion & Call to Action (100 words)

**Summary**: Production readiness is a feature, not an afterthought

- Multi-stage Docker for security
- Health probes for Kubernetes
- Connection pooling for reliability
- Stateless design for scaling
- Runbook for operations

**CTA**: "EdgeQuake: `docker-compose up` to production in minutes, not months."

---

## LinkedIn Post Structure (<3000 chars)

```
Hook → Problem → 5 Solutions → CTA
```

1. **Hook** (1 sentence): Your RAG demo isn't production-ready
2. **Problem** (2 sentences): ML teams build prototypes, SRE teams make them production-ready
3. **5 Solutions** (bullet points):
   - Health endpoints
   - Connection pooling
   - Multi-stage Docker
   - Horizontal scaling
   - Runbook included
4. **CTA**: Link to GitHub

---

## X.com Thread Structure (14 tweets)

1. **Hook**: "Your RAG prototype 'works perfectly' in demos. Here's why it will fail in production 🧵"
2. **Problem stat**: ML frameworks focus on notebooks, not ops
3. **Health probes intro**: Kubernetes needs to know if your app is alive
4. **Code**: `/health`, `/ready`, `/live` endpoints
5. **Connection pooling problem**: Unbounded connections = 3am pages
6. **Connection pooling solution**: Built-in SQLx pooling
7. **Docker problem**: 2GB images with full toolchain
8. **Docker solution**: Multi-stage build, ~100MB final
9. **Scaling problem**: Stateful services don't scale
10. **Scaling solution**: All state in PostgreSQL
11. **Runbook**: 316 lines of operational documentation
12. **Alert thresholds**: Defined SLOs out of the box
13. **Summary**: Production readiness is a feature
14. **CTA**: EdgeQuake GitHub link

---

## HackerNews Post (~700 words)

**Title**: "What I Learned Building Production-Ready Graph-RAG in Rust"

**Structure**:

1. Background (100 words) - Building Graph-RAG, realizing frameworks weren't production-ready
2. Technical decisions (400 words) - Multi-stage Docker, health probes, pooling
3. Open questions (100 words) - What production patterns are we missing?
4. Discussion invite (100 words) - How do others handle RAG in production?

---

## Reddit Post (~800 words)

**Subreddits**: r/devops, r/kubernetes, r/rust

**Title**: "Lessons from deploying Graph-RAG to production (Kubernetes + PostgreSQL + Rust)"

**Structure**:

1. Context (no sales pitch)
2. Technical patterns learned
3. Mistakes made
4. What worked
5. Open to feedback

---

## Substack Newsletter (~1500 words)

**Title**: "The 3am Page That Taught Me RAG Needs a Runbook"

**Structure**:

1. Story: The first production incident
2. What we learned
3. How we fixed it
4. The patterns we now build into every deployment
5. Your takeaways

---

## Validation Checklist

- [x] Starts with WHY (production failures are painful)
- [x] Includes ASCII diagrams (health probes, scaling, Docker)
- [x] Has real code snippets from EdgeQuake
- [x] Cites Kubernetes documentation (pod lifecycle, probes)
- [x] Platform-appropriate length and tone
- [x] Clear call to action

---

## Next: Create Articles

1. `articles/012_production_deployment/medium.md` (~2200 words)
2. `articles/012_production_deployment/linkedin.md` (<3000 chars)
3. `articles/012_production_deployment/xcom.md` (14 tweets)
4. `articles/012_production_deployment/hackernews.md` (~700 words)
5. `articles/012_production_deployment/reddit.md` (~800 words)
6. `articles/012_production_deployment/substack.md` (~1500 words)
