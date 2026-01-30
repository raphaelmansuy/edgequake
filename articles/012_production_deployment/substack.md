# Substack Newsletter: Production Deployment

---

# The 3am Page That Taught Me RAG Needs a Runbook

_Why your ML demo and production deployment are fundamentally different_

---

The first 3am page came on a Tuesday.

I was two months into deploying our Graph-RAG system—the one that had dazzled leadership in the demo three months prior. The one where leadership said "let's get this into production immediately."

The PagerDuty alert was cryptic: "connection pool exhausted, query failures exceeding threshold."

I stumbled to my laptop, half-asleep, trying to remember what connection pool we were even talking about. The RAG framework we'd used didn't have connection pooling. Every database query opened a new connection. At 2am, our batch processing job kicked off, tried to open 200 connections simultaneously, and PostgreSQL threw up its hands.

The fix took 15 minutes once I understood the problem. Finding the problem took two hours of bleary-eyed log diving.

That night taught me something: **RAG frameworks optimize for demos. Production needs something else entirely.**

---

## The Demo-Production Gap

Here's what the demo environment had:

- A Jupyter notebook running on my laptop
- A small PostgreSQL database with 50 documents
- An audience of five people who were impressed by the multi-hop query answering

Here's what production needed:

- Docker containers running on Kubernetes
- Health endpoints so the orchestrator knows the app is alive
- Connection pooling so we don't exhaust the database
- Graceful shutdown so deployments don't corrupt transactions
- A runbook so the next 3am incident doesn't take two hours

The demo took a week to build. Production-readiness took three months.

This pattern repeats across the industry. I've talked to teams at other companies, and they tell similar stories. The ML team builds something amazing. The SRE team inherits it. Months pass before it's actually production-ready.

---

## What I Wish I'd Known

After that incident—and the two that followed—I started documenting what production RAG systems actually need. Here's the list I wish someone had given me:

### 1. Health Endpoints (Three of Them)

Kubernetes uses probes to manage containers:

- **Liveness probe**: Is the process alive? If not, kill it and restart.
- **Readiness probe**: Is it ready for traffic? If not, don't send requests yet.
- **Startup probe**: Has it finished starting? Don't check liveness until startup completes.

Without these, you get pods stuck in `CrashLoopBackOff`. Deployments fail. Users see errors. The orchestrator doesn't know if your app is healthy.

Three endpoints. Maybe 50 lines of code. Would have saved us two deployment incidents.

### 2. Connection Pooling

The framework we used opened a fresh database connection for every query. Fine for a notebook. Disaster at scale.

PostgreSQL has a `max_connections` limit (default 100). Under load, we hit it. New queries fail. The backlog grows. Eventually, everything times out.

Connection pooling solves this:

- Maintain a pool of reusable connections
- Set a maximum (tune for your database)
- Fail fast if the pool is exhausted (better than hanging)
- Clean up idle connections (release resources)

We switched to SQLx with pooling. Problem solved. Should have been the default.

### 3. Graceful Shutdown

Here's what happens when Kubernetes deploys a new version:

1. Send SIGTERM to old pods
2. Wait grace period (default 30s)
3. Send SIGKILL if still running

If your process doesn't handle SIGTERM, it gets killed mid-transaction. Entity extraction jobs leave partial results. Graph relationships become orphaned. You spend your weekend cleaning up data.

Graceful shutdown means:

1. Receive SIGTERM
2. Stop accepting new requests
3. Finish in-flight requests
4. Close database connections cleanly
5. Exit

Maybe 20 lines of code. Would have saved a weekend.

### 4. Docker, Done Right

Our first Docker image was 2GB. It included the entire Rust toolchain because we copied the approach from a tutorial.

Multi-stage builds fix this:

- Stage 1: Build with full toolchain
- Stage 2: Copy only the binary to a minimal image

Final image: ~100MB instead of ~2GB. Faster pulls. Smaller attack surface. We also added a non-root user (security) and a built-in HEALTHCHECK (visibility).

### 5. The Runbook

I wrote the runbook after our third incident. Should have written it before the first.

It now includes:

- Alert thresholds (p99 latency > 500ms = warning, > 2s = critical)
- Common issues and diagnosis commands
- Recovery procedures for each failure mode
- Backup and restore instructions
- Security procedures (key rotation, audit logs)

316 lines. Living document. Gets updated after every incident.

---

## What Changed

After building these patterns into our system, deployments went from anxiety-inducing to routine. The 3am pages stopped (mostly). New team members could look at the runbook instead of asking me how to diagnose problems.

The lesson: **production readiness isn't an afterthought. It's a feature.**

When we eventually rewrote the system in Rust (EdgeQuake), we started from production requirements. Health endpoints first. Connection pooling built-in. Graceful shutdown from day one. Runbook included in the repo.

The result? Teams can go from `docker-compose up` to production-ready without the three-month detour. Their SRE teams don't have to build the infrastructure patterns—they're already there.

---

## Your Takeaways

If you're deploying an ML system to production—RAG or otherwise—here's my checklist:

1. **Add health endpoints** before the first deployment, not after the first incident
2. **Use connection pooling** for any database, especially under load
3. **Handle shutdown signals** gracefully, or enjoy data corruption
4. **Optimize your Docker build** for size and security
5. **Write a runbook** before you need it, not after

The gap between "works on my laptop" and "runs at 3am without paging anyone" is larger than anyone budgets for. Plan for it.

---

_If you're building RAG systems and want to skip the three-month production detour, EdgeQuake ships these patterns out of the box. Open source on GitHub._

_Until next week,_
_Raphael_

---

_P.S. — The framework that caused the 3am page was fine for what it was designed for: prototyping. The mistake was assuming prototyping tools work in production. Different problems, different solutions._
