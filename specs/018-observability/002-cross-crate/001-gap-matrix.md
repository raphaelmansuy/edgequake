# Cross-Crate Observability Gap Matrix

**Spec:** 018-observability  
**Date:** 2026-06-05

---

## Current vs Target (Request Path)

```
  Browser (WebUI)                EdgeQuake API                 PostgreSQL / LLM
  ───────────────                ─────────────                 ────────────────

  NO traceparent  ──HTTP──▶  request_id MW ──▶  handlers
  NO x-request-id            (new UUID only)       │
  X-Tenant-ID ✅             request_logging       ├─▶ sqlx (no span)
  X-Workspace-ID ✅          TraceLayer (tower)    └─▶ edgequake-llm
                             NO span.request_id         (extra_headers
                             fmt logs to stdout          ONLY if client
                                                          sends in JSON)

         ╳ gap ╳                    ╳ gap ╳                  ╳ gap ╳
```

---

## Gap Matrix

| ID | P | Gap | Evidence | Impact |
|----|---|-----|----------|--------|
| OBS-P0-001 | P0 | No OTEL SDK | `rg opentelemetry edgequake/` → 0 Rust hits | No distributed traces in Jaeger/Tempo |
| OBS-P0-002 | P0 | Request ID not in spans | `middleware.rs:71-86` — header only | Cannot filter logs by request |
| OBS-P0-003 | P0 | WebUI no correlation headers | `client.ts:146-171` — tenant/workspace only | UI-initiated calls untraceable E2E |
| OBS-P0-004 | P0 | Metrics stub | `metrics.rs:43-147` TODO + static `0` | False sense of monitoring |
| OBS-P0-005 | P0 | Audit crate unwired | `api/Cargo.toml:26` dep; 0 imports in `api/src` | Compliance table empty |
| OBS-P0-006 | P0 | JSON logging documented, not used | `monitoring.md:82-96` vs `main.rs:480` fmt | Loki/ELK parsers fail |
| OBS-P1-001 | P1 | Incoming traceparent ignored | No `traceparent` parse in middleware | Cannot join upstream traces |
| OBS-P1-002 | P1 | LLM headers manual only | `query_types.rs:139` `extra_headers` optional | Operators must duplicate headers in body |
| OBS-P1-003 | P1 | Triple HTTP logging | `server.rs:91-93` + custom MW | Log volume 3×; no shared ID |
| OBS-P1-004 | P1 | `error!` underused | ~15 `error!` in all crates | Failures visible only as HTTP 5xx |
| OBS-P1-005 | P1 | `edgequake-auth` silent | 0 tracing in `edgequake-auth/src` | Auth failures opaque |
| OBS-P1-006 | P1 | Background tasks silent | `edgequake-tasks` 0 tracing in src | Queue stalls invisible |
| OBS-P2-001 | P2 | No `#[instrument]` | 0 across workspace | No function-level spans |
| OBS-P2-002 | P2 | Default RUST_LOG is debug-heavy | `main.rs:478` all crates `debug` | Prod noise if env unset |
| OBS-P2-003 | P2 | WebUI `console.*` (~45) | `rg console.` in `edgequake_webui/src` | No server-side correlation |
| OBS-P2-004 | P2 | Audit query stub | `logger.rs:204-206` returns `vec![]` | Admin audit UI empty |
| OBS-P3-001 | P3 | Emoji log messages | `main.rs`, handlers | Log pipeline breakage |
| OBS-P3-002 | P3 | Next OTEL in lockfile only | `pnpm-lock.yaml` transitive | Unused capability |

---

## What Works Today (Credit)

| Capability | Location | Notes |
|------------|----------|-------|
| Health/readiness | `/health`, `/ready`, `/live` | K8s-ready |
| HTTP access log | `middleware.rs:39-67` | status-based info/warn |
| Response `x-request-id` | `middleware.rs:82-84` | SDKs read it (`sdks/python/_errors.py:199`) |
| Tenant headers → LLM | `safety_limits.rs:452-494` | When `extra_headers` populated |
| Rate-limit visibility | `rate-limiter/middleware.rs:56-60` | WARN + response headers |
| tower `TraceLayer` | `server.rs:93` | Hyper request tracing (not OTEL) |
| Workspace metrics history API | `handlers/workspaces/stats.rs` | Business metrics, not Prometheus |

---

## Priority Actions (Sprint 0)

```
Week 1 ──▶ OBS-P0-002 + OBS-P0-003 + OBS-P0-006  (correlation + JSON logs)
Week 2 ──▶ OBS-P0-001 + OBS-P1-001               (OTEL bootstrap + traceparent)
Week 3 ──▶ OBS-P0-004 + OBS-P1-004               (real metrics + error logging)
Week 4 ──▶ OBS-P0-005                            (wire audit + request_id column)
```

Detail: [003-remediation-roadmap](./003-remediation-roadmap.md).
