# EdgeQuake Maintenance Runbook

This document provides operational procedures for maintaining EdgeQuake in production.

## Table of Contents

1. [Health Monitoring](#health-monitoring)
2. [Common Issues](#common-issues)
3. [Scaling Procedures](#scaling-procedures)
4. [Backup and Recovery](#backup-and-recovery)
5. [Performance Tuning](#performance-tuning)
6. [Security Procedures](#security-procedures)

---

## Health Monitoring

### Health Endpoints

| Endpoint      | Purpose         | Expected Response       |
| ------------- | --------------- | ----------------------- |
| `GET /health` | Overall health  | `{"status": "healthy"}` |
| `GET /ready`  | Readiness probe | `{"status": "ready"}`   |
| `GET /live`   | Liveness probe  | `{"status": "live"}`    |

### Key Metrics to Monitor

```bash
# API response times
curl -w "@curl-format.txt" http://localhost:8080/health

# Memory usage
docker stats edgequake

# Connection pool status (if using PostgreSQL)
psql -c "SELECT count(*) FROM pg_stat_activity WHERE datname='edgequake';"
```

### Alert Thresholds

| Metric          | Warning | Critical |
| --------------- | ------- | -------- |
| API p99 latency | > 500ms | > 2s     |
| Error rate      | > 1%    | > 5%     |
| Memory usage    | > 70%   | > 90%    |
| CPU usage       | > 70%   | > 90%    |
| Storage usage   | > 70%   | > 90%    |

---

## Common Issues

### Issue: High Memory Usage

**Symptoms**: Memory climbing, OOM kills

**Diagnosis**:

```bash
# Check container memory
docker stats edgequake

# Check for memory leaks in logs
docker logs edgequake | grep -i "memory\|oom"
```

**Resolution**:

1. Check for large document uploads
2. Verify batch sizes in configuration
3. Restart service if immediate relief needed
4. Increase memory limits if sustained high usage

### Issue: Slow Query Performance

**Symptoms**: High latency on /api/v1/query

**Diagnosis**:

```bash
# Check query timing
curl -w "Total: %{time_total}s\n" -X POST \
  http://localhost:8080/api/v1/query \
  -H "Content-Type: application/json" \
  -d '{"query": "test", "mode": "naive"}'

# Check graph size
curl http://localhost:8080/api/v1/graph/stats
```

**Resolution**:

1. Use simpler query mode (naive vs hybrid)
2. Reduce `max_tokens` in configuration
3. Check LLM provider latency
4. Consider adding vector index if using PostgreSQL

### Issue: Document Upload Failures

**Symptoms**: 500 errors on POST /api/v1/documents

**Diagnosis**:

```bash
# Check logs for errors
docker logs edgequake --tail 100 | grep -i error

# Check document size
wc -c < document.txt
```

**Resolution**:

1. Verify document is under size limit (default 10MB)
2. Check LLM API key is valid
3. Verify storage backend is accessible
4. Check for malformed content

### Issue: Connection Pool Exhaustion

**Symptoms**: "connection pool exhausted" errors

**Diagnosis**:

```bash
# PostgreSQL connections
psql -c "SELECT count(*), state FROM pg_stat_activity GROUP BY state;"
```

**Resolution**:

1. Increase pool size in configuration
2. Check for connection leaks
3. Implement connection timeouts
4. Restart service to reset pool

---

## Scaling Procedures

### Horizontal Scaling

EdgeQuake supports horizontal scaling with stateless API instances:

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

**Prerequisites**:

- Shared storage backend (PostgreSQL)
- Load balancer in front

### Vertical Scaling

Adjust resource limits:

```yaml
services:
  edgequake:
    deploy:
      resources:
        limits:
          memory: 4G
          cpus: "2"
```

---

## Backup and Recovery

### Database Backup

```bash
# PostgreSQL backup
pg_dump -h localhost -U edgequake -d edgequake > backup.sql

# Restore
psql -h localhost -U edgequake -d edgequake < backup.sql
```

### Vector Storage Backup

If using pgvector, vectors are included in PostgreSQL backup.

### Configuration Backup

```bash
# Backup configuration
cp .env .env.backup
cp config.toml config.toml.backup
```

### Disaster Recovery Procedure

1. **Assess damage**: Determine what data is affected
2. **Restore database**: From most recent backup
3. **Verify integrity**: Check document counts
4. **Re-index if needed**: Trigger reprocessing of affected documents
5. **Validate**: Run health checks

---

## Performance Tuning

### Chunking Configuration

```toml
[chunking]
chunk_size = 1200      # Tokens per chunk
chunk_overlap = 100    # Token overlap
```

**Guidelines**:

- Larger chunks = better context, slower processing
- Smaller chunks = faster, but may lose context
- Overlap prevents context loss at boundaries

### Query Configuration

```toml
[query]
top_k = 10             # Number of results to retrieve
max_tokens = 4000      # Max tokens in response
temperature = 0.7      # LLM creativity
```

### Connection Pool Tuning

```toml
[database]
max_connections = 20
min_connections = 5
connection_timeout = 30
```

---

## Security Procedures

### API Key Rotation

1. Generate new API key
2. Update `.env` file
3. Restart service
4. Update all clients with new key
5. Revoke old key after transition

### Audit Log Review

```bash
# Review access logs
docker logs edgequake | grep "api/v1" | tail -1000

# Check for suspicious patterns
docker logs edgequake | grep -E "(401|403|500)" | tail -100
```

### Dependency Updates

```bash
# Check for vulnerabilities
cargo audit

# Update dependencies
cargo update

# Test after updates
cargo test --all
```

### Incident Response

1. **Contain**: Isolate affected systems
2. **Assess**: Determine scope of incident
3. **Mitigate**: Apply immediate fixes
4. **Document**: Record timeline and actions
5. **Review**: Post-incident analysis
6. **Improve**: Update procedures based on learnings

---

## Appendix

### Useful Commands

```bash
# View logs
docker logs -f edgequake

# Execute command in container
docker exec -it edgequake /bin/sh

# Check disk usage
docker system df

# Prune unused resources
docker system prune -f
```

### Contact Information

| Role             | Contact              | Escalation           |
| ---------------- | -------------------- | -------------------- |
| On-call engineer | ops@example.com      | PagerDuty            |
| Database admin   | dba@example.com      | Slack #dba           |
| Security team    | security@example.com | security@example.com |
