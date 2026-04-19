---
title: 'Monitoring Guide'
---

# Monitoring Guide

> **Observability for EdgeQuake Deployments**

This guide covers monitoring, logging, and alerting for EdgeQuake in production environments.

---

## Observability Stack

```
┌─────────────────────────────────────────────────────────────────┐
│                   OBSERVABILITY OVERVIEW                        │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  ┌─────────────┐    ┌─────────────┐    ┌─────────────┐          │
│  │  EdgeQuake  │───▶│   Logs      │───▶│ Log Aggr.   │          │
│  │   Server    │    │  (stdout)   │    │ (Loki/ELK)  │          │
│  └──────┬──────┘    └─────────────┘    └─────────────┘          │
│         │                                                       │
│         ├─────────▶ /health endpoints                           │
│         │                                                       │
│         ├─────────▶ /metrics (planned)                          │
│         │                                                       │
│         └─────────▶ PostgreSQL metrics                          │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

---

## Health Endpoints

EdgeQuake provides built-in health endpoints:

| Endpoint      | Purpose             | Response                  |
| ------------- | ------------------- | ------------------------- |
| `GET /health` | Basic health        | `{ "status": "healthy" }` |
| `GET /ready`  | Readiness check     | Database + LLM status     |
| `GET /live`   | Kubernetes liveness | Process check             |

### Basic Health Check

```bash
curl http://localhost:8080/health
```

```json
{
  "status": "healthy",
  "version": "0.10.x",
  "storage_mode": "postgresql"
}
```

### Readiness Check

```bash
curl http://localhost:8080/ready
```

```json
{
  "status": "ready",
  "checks": {
    "database": "ok",
    "llm_provider": "ok"
  }
}
```

---

## Logging

### Log Format

EdgeQuake uses structured JSON logging via the `tracing` crate:

```json
{
  "timestamp": "2024-01-15T10:30:00.000Z",
  "level": "INFO",
  "target": "edgequake_api::handlers::documents",
  "message": "Document uploaded successfully",
  "fields": {
    "document_id": "doc_123",
    "workspace_id": "ws_456",
    "duration_ms": 1234
  }
}
```

### Log Levels

| Level | `RUST_LOG` Setting | Use Case              |
| ----- | ------------------ | --------------------- |
| Error | `error`            | Critical failures     |
| Warn  | `warn`             | Degraded but working  |
| Info  | `info`             | Production operations |
| Debug | `debug`            | Development debugging |
| Trace | `trace`            | Detailed tracing      |

### Recommended Production Settings

```bash
# Production
RUST_LOG="edgequake=info,tower_http=info,sqlx=warn"

# Development
RUST_LOG="edgequake=debug,tower_http=debug"

# Troubleshooting
RUST_LOG="edgequake=trace,sqlx=debug"
```

### Component-Specific Logging

```bash
# Pipeline debugging
RUST_LOG="edgequake_pipeline=debug"

# Query engine debugging
RUST_LOG="edgequake_query=debug"

# API request tracing
RUST_LOG="tower_http=debug"

# Database query logging
RUST_LOG="sqlx=debug"
```

---

## Log Aggregation

### Loki + Grafana

Docker Compose addition:

```yaml
services:
  loki:
    image: grafana/loki:2.9.0
    ports:
      - "3100:3100"
    volumes:
      - ./loki-config.yaml:/etc/loki/local-config.yaml

  promtail:
    image: grafana/promtail:2.9.0
    volumes:
      - /var/log:/var/log
      - ./promtail-config.yaml:/etc/promtail/config.yml
    command: -config.file=/etc/promtail/config.yml

  grafana:
    image: grafana/grafana:10.0.0
    ports:
      - "3001:3000"
    environment:
      - GF_SECURITY_ADMIN_PASSWORD=admin
```

### ELK Stack

Filebeat configuration:

```yaml
filebeat.inputs:
  - type: container
    paths:
      - "/var/lib/docker/containers/*/*.log"
    processors:
      - add_kubernetes_metadata:
          host: ${NODE_NAME}
          matchers:
            - logs_path:
                logs_path: "/var/lib/docker/containers/"

output.elasticsearch:
  hosts: ["elasticsearch:9200"]
```

---

## Key Metrics to Monitor

### Application Metrics

| Metric                | Source     | Alert Threshold |
| --------------------- | ---------- | --------------- |
| Request latency       | Logs       | p99 > 2s        |
| Error rate            | Logs       | > 1%            |
| Active connections    | PostgreSQL | > 80% pool      |
| Background task queue | Logs       | > 100 pending   |

### PostgreSQL Metrics

| Metric           | Query                  | Alert Threshold |
| ---------------- | ---------------------- | --------------- |
| Connection count | `pg_stat_activity`     | > 80% max       |
| Cache hit ratio  | `pg_stat_database`     | < 95%           |
| Index usage      | `pg_stat_user_indexes` | Unused indexes  |
| Table bloat      | `pgstattuple`          | > 30%           |

### LLM Provider Metrics

| Metric      | Source       | Alert Threshold  |
| ----------- | ------------ | ---------------- |
| Token usage | Provider API | Budget threshold |
| Error rate  | Logs         | > 5%             |
| Latency     | Logs         | > 10s            |
| Rate limits | Provider API | Near limit       |

---

## Alerting Rules

### Prometheus Alerting (Example)

```yaml
groups:
  - name: edgequake
    rules:
      - alert: HighErrorRate
        expr: rate(http_requests_total{status=~"5.."}[5m]) > 0.01
        for: 5m
        labels:
          severity: critical
        annotations:
          summary: High error rate detected

      - alert: SlowQueries
        expr: histogram_quantile(0.99, query_duration_seconds_bucket) > 2
        for: 10m
        labels:
          severity: warning
        annotations:
          summary: Query latency above 2s

      - alert: DatabaseConnectionsHigh
        expr: pg_stat_activity_count > 80
        for: 5m
        labels:
          severity: warning
        annotations:
          summary: PostgreSQL connections high
```

---

## Dashboard Examples

### Key Panels for Grafana

1. **Request Overview**
   - Requests per second
   - Error rate
   - Latency percentiles (p50, p95, p99)

2. **Document Processing**
   - Documents indexed per minute
   - Processing time distribution
   - Queue depth

3. **Query Performance**
   - Query latency by mode
   - Context retrieval time
   - LLM generation time

4. **Resource Usage**
   - CPU usage
   - Memory usage
   - PostgreSQL connections
   - Disk I/O

### Sample Query Panel

```
# Loki query for request latency
{app="edgequake"} |= "request completed" | json | duration_ms > 1000
```

---

## Tracing (Future)

OpenTelemetry integration is planned:

```rust
// Future: Distributed tracing
#[tracing::instrument]
async fn process_query(query: &str) -> Result<Response> {
    // Automatic span creation
    let chunks = retrieve_chunks(query).await?;
    let response = generate_response(chunks).await?;
    Ok(response)
}
```

---

## PostgreSQL Monitoring

### Essential Views

```sql
-- Active connections
SELECT count(*) as connections,
       state,
       wait_event_type
FROM pg_stat_activity
WHERE datname = 'edgequake'
GROUP BY state, wait_event_type;

-- Long-running queries
SELECT pid,
       now() - pg_stat_activity.query_start AS duration,
       query
FROM pg_stat_activity
WHERE (now() - pg_stat_activity.query_start) > interval '5 minutes'
  AND state != 'idle';

-- Table sizes
SELECT schemaname,
       relname,
       pg_size_pretty(pg_total_relation_size(relid)) as total_size
FROM pg_catalog.pg_statio_user_tables
ORDER BY pg_total_relation_size(relid) DESC
LIMIT 10;

-- Index usage
SELECT schemaname,
       relname,
       indexrelname,
       idx_scan,
       idx_tup_read
FROM pg_stat_user_indexes
ORDER BY idx_scan ASC
LIMIT 10;
```

### Vector Storage Metrics

```sql
-- Vector index stats (pgvector)
SELECT indexname,
       pg_size_pretty(pg_relation_size(indexname::regclass)) as size
FROM pg_indexes
WHERE indexdef LIKE '%vector%';

-- Chunk count per workspace
SELECT workspace_id,
       count(*) as chunk_count
FROM chunks
GROUP BY workspace_id
ORDER BY chunk_count DESC;
```

### Graph Storage Metrics

```sql
-- Entity count
SELECT count(*) FROM ag_catalog.cypher('edgequake_graph', $$
  MATCH (n) RETURN count(n)
$$) AS (count agtype);

-- Relationship count
SELECT count(*) FROM ag_catalog.cypher('edgequake_graph', $$
  MATCH ()-[r]->() RETURN count(r)
$$) AS (count agtype);
```

---

## Troubleshooting

### High Memory Usage

1. Check background task queue
2. Review connection pool size
3. Analyze PostgreSQL memory settings

```bash
# Check process memory
ps aux | grep edgequake

# Check PostgreSQL memory
psql -c "SHOW shared_buffers; SHOW work_mem;"
```

### Slow Queries

1. Enable query logging

```bash
RUST_LOG="edgequake_query=debug,sqlx=debug"
```

2. Check PostgreSQL slow query log

```sql
-- Enable slow query logging
ALTER SYSTEM SET log_min_duration_statement = 1000;  -- 1 second
SELECT pg_reload_conf();
```

### LLM Errors

1. Check provider status
2. Review rate limits
3. Verify API keys

```bash
# Test OpenAI connectivity
curl https://api.openai.com/v1/models \
  -H "Authorization: Bearer $OPENAI_API_KEY"

# Test Ollama connectivity
curl http://localhost:11434/api/tags
```

---

## Backup Monitoring

### PostgreSQL Backups

```bash
# Check last backup time
pg_dump --version-only edgequake

# Verify backup size
ls -lh /backups/edgequake-*.sql.gz
```

### Backup Alert Rule

```yaml
- alert: BackupTooOld
  expr: time() - backup_last_success_timestamp > 86400
  for: 1h
  labels:
    severity: critical
  annotations:
    summary: No successful backup in 24 hours
```

---

## Summary

```
┌─────────────────────────────────────────────────────────────────┐
│                   MONITORING CHECKLIST                           │
├─────────────────────────────────────────────────────────────────┤
│                                                                   │
│  ✅ Health endpoints configured for load balancer               │
│  ✅ Structured logging enabled                                   │
│  ✅ Log aggregation set up (Loki/ELK)                           │
│  ✅ Key metrics identified and dashboarded                      │
│  ✅ Alert rules defined for critical conditions                 │
│  ✅ PostgreSQL monitoring enabled                               │
│  ✅ LLM provider usage tracked                                  │
│  ✅ Backup verification automated                               │
│                                                                   │
└─────────────────────────────────────────────────────────────────┘
```

---

## See Also

- [Deployment Guide](/docs/operations/deployment/) - Production deployment
- [Configuration Reference](/docs/operations/configuration/) - All settings
- [Troubleshooting Guide](/docs/troubleshooting/common-issues/) - Problem solving
