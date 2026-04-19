---
title: Operations
description: Deploy, monitor, and tune EdgeQuake in production.
---

Production deployment and operations guides.

## Reliability-first operating model

EdgeQuake now documents and follows a few simple operational invariants:

- pin the Rust toolchain so local results and CI results do not drift
- use readiness probes instead of fixed sleeps when starting services
- cancel superseded CI runs on the same branch to reduce stale signal and wasted minutes
- keep heavyweight coverage and full-E2E flows outside the fastest blocking feedback loop
- fail closed when an explicit workspace context is invalid or missing

## Guides

- **[Deployment](/docs/operations/deployment/)** — Docker, Kubernetes, and bare-metal deployment.
- **[Configuration](/docs/operations/configuration/)** — Environment variables and runtime settings.
- **[Monitoring](/docs/operations/monitoring/)** — Health checks, metrics, and observability.
- **[Performance Tuning](/docs/operations/performance-tuning/)** — Optimize throughput and latency.
- **[Metadata Debugging](/docs/operations/metadata-debugging/)** — Inspect and debug extracted metadata.
