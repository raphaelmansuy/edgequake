# X.com Thread: Production Deployment

---

## Tweet 1 (Hook)

Your RAG prototype "works perfectly" in demos.

Here's why it will fail in production—and how we fixed it. 🧵

---

## Tweet 2 (The Problem)

Most RAG frameworks optimize for notebooks:
• Rapid prototyping ✓
• Easy experimentation ✓
• Quick demos ✓

Production concerns?
"Exercise for the reader."

The result: 3-6 months of DevOps work before your first real deployment.

---

## Tweet 3 (Health Probes)

Problem #1: Kubernetes can't tell if your app is alive.

No health endpoints = pods stuck in CrashLoopBackOff during deployment.

EdgeQuake ships three endpoints:
• /live → livenessProbe
• /ready → readinessProbe  
• /health → component status

Zero-downtime deploys. Out of the box.

---

## Tweet 4 (Health Probe Diagram)

How it works:

```
kubelet ──► GET /live ──► 200 OK
        │
        └► GET /ready ─► 200 OK

If /live fails → Kill pod, restart
If /ready fails → Remove from Service
```

Kubernetes integration shouldn't be a 2-week project.

---

## Tweet 5 (Connection Pooling Problem)

Problem #2: Connection pool exhaustion.

Naive approach: New database connection per request.
Under load: PostgreSQL hits max_connections.
Result: 3am page.

Seen this pattern at three different companies. Same failure mode every time.

---

## Tweet 6 (Connection Pooling Solution)

EdgeQuake's fix: Built-in SQLx connection pooling.

```rust
PgPoolOptions::new()
    .max_connections(10)   // Tunable
    .min_connections(1)    // Always warm
    .acquire_timeout(30s)  // Fail fast
    .idle_timeout(600s)    // Clean up
```

Auto-enables pgvector and Apache AGE on first connect.

---

## Tweet 7 (Docker Problem)

Problem #3: 2GB Docker images with full toolchain.

More packages = more vulnerabilities
Root user = security risk
No health check = orchestrator blind

---

## Tweet 8 (Docker Solution)

EdgeQuake's multi-stage build:

Stage 1: Build with Rust toolchain
Stage 2: Copy only the binary

Result:
• ~100MB final image (not 2GB)
• Non-root user by default
• Built-in HEALTHCHECK
• --locked for reproducibility

---

## Tweet 9 (Scaling Problem)

Problem #4: Stateful services don't scale.

In-memory caches need invalidation.
Session affinity limits load balancing.
Adding replicas becomes a distributed systems problem.

---

## Tweet 10 (Scaling Solution)

EdgeQuake: Stateless by design.

All state lives in PostgreSQL:
• Graph → Apache AGE
• Vectors → pgvector
• Documents → Standard tables

Scaling = `replicas: N`

Any API server can handle any request. Load balancer uses round-robin.

---

## Tweet 11 (Runbook)

Problem #5: No operational documentation.

First incident = panic.
Every incident = reinventing diagnosis procedures.

EdgeQuake ships a 316-line runbook:
• Alert thresholds (p99, error rate, resources)
• Common issues + resolutions
• Backup/recovery procedures
• Security procedures

---

## Tweet 12 (Graceful Shutdown)

Problem #6: SIGTERM kills mid-transaction.

Entity extraction jobs corrupt.
Graph relationships orphan.
Weekend cleanup ensues.

EdgeQuake: Graceful shutdown handler drains connections before exit.

Data integrity isn't optional.

---

## Tweet 13 (Summary)

Production readiness checklist:

✅ Health probes (3 endpoints)
✅ Connection pooling
✅ Multi-stage Docker
✅ Stateless design
✅ Runbook included
✅ Graceful shutdown

EdgeQuake ships all of these.

Your SRE team shouldn't have to build them.

---

## Tweet 14 (CTA)

From demo to production in minutes, not months.

```bash
docker-compose up -d
curl http://localhost:8080/health
# {"status": "healthy"}
```

Open source: github.com/raphaelmansuy/edgequake

Build RAG systems, not production infrastructure. 🚀
