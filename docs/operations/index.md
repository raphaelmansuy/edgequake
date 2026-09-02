---
title: Operations
description: Deploy, monitor, and tune EdgeQuake in production.
---

> **Product: v0.26.5** · Contract: OpenAPI

Production deployment and operations guides.

## Reliability-first operating model

EdgeQuake now documents and follows a few simple operational invariants:

- pin the Rust toolchain so local results and CI results do not drift
- use readiness probes instead of fixed sleeps when starting services
- cancel superseded CI runs on the same branch to reduce stale signal and wasted minutes
- keep heavyweight coverage and full-E2E flows outside the fastest blocking feedback loop
- fail closed when an explicit workspace context is invalid or missing

## Guides

- **[Docker Quickstart](/docs/operations/docker-quickstart/)** — Full stack from GHCR images (no local build).
- **[Deployment](/docs/operations/deployment/)** — Docker, Kubernetes, and bare-metal deployment.
- **[Configuration](/docs/operations/configuration/)** — Environment variables and runtime settings.
- **[Monitoring](/docs/operations/monitoring/)** — Health checks, metrics, and observability.
- **[Langfuse 3.1.x](/docs/operations/langfuse-3.1/)** — Wire EdgeQuake to self-hosted Langfuse 3.1 (ingestion fallback).
- **[Performance Tuning](/docs/operations/performance-tuning/)** — Optimize throughput and latency.
- **[Metadata Debugging](/docs/operations/metadata-debugging/)** — Inspect and debug extracted metadata.
- **[Runtime auth hardening](/docs/operations/runtime-auth-hardening/)** — Production auth and bootstrap.
- **[Release & CD](/docs/operations/release-and-cd/)** — Tag, gates, and Docker publish cycle.
- **[Ingestion cancel & fairness](/docs/ingestion-cancel-and-fairness/)** — Cancel, lease, fairness, multi-replica.
