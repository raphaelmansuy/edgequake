# Phase 6: Handoff Documentation

**Phase Duration**: Weeks 11-12 (parallel with Phase 5)  
**Owner**: Tech Lead + DevOps Lead  
**Status**: 🔴 Not Started

---

## Objective

Create comprehensive handoff documentation including deployment guides, maintenance runbooks, architecture decision records (ADRs), and operational procedures for production readiness.

---

## Reference Documentation

| Document | Purpose |
|----------|---------|
| [docs_retro/11-rebuild-checklist.md](../../docs_retro/11-rebuild-checklist.md) | Launch checklist |
| [docs_retro/06-deployment-patterns.md](../../docs_retro/06-deployment-patterns.md) | Deployment options |
| [plan/integration/IMPLEMENTATION_ROADMAP.md](../../plan/integration/IMPLEMENTATION_ROADMAP.md) | Integration plan |
| [plan/integration/MIGRATION_GUIDE.md](../../plan/integration/MIGRATION_GUIDE.md) | Migration details |
| [k8s-deploy/](../../k8s-deploy/) | Kubernetes manifests |

---

## Deliverables Overview

| Deliverable | Purpose |
|-------------|---------|
| Deployment Guide | Step-by-step production deployment |
| Maintenance Runbook | Operational procedures and troubleshooting |
| Architecture Decision Records | Key technical decisions and rationale |
| Migration Playbook | LightRAG → EdgeQuake migration steps |
| Operations Manual | Day-to-day operations guide |

---

## 6.1 Deployment Guide

### edgequake-docs/deployment/README.md

```markdown
# EdgeQuake Deployment Guide

This guide covers deploying EdgeQuake to production environments.

## Deployment Options

| Option | Complexity | Best For |
|--------|------------|----------|
| Docker Compose | Low | Development, small deployments |
| Kubernetes | Medium | Production, scalability |
| Managed Cloud | Low | Quick start, less ops overhead |

## Prerequisites

- PostgreSQL 16+ with AGE and pgvector extensions
- (Optional) Redis for caching
- Container runtime (Docker/Podman) or Kubernetes cluster
- TLS certificates for HTTPS

---

## Option 1: Docker Compose (Recommended for Startups)

### docker-compose.prod.yml

```yaml
version: "3.9"

services:
  edgequake:
    image: ghcr.io/your-org/edgequake:latest
    ports:
      - "8020:8020"
    environment:
      DATABASE_URL: postgres://edgequake:${DB_PASSWORD}@postgres:5432/edgequake
      OPENAI_API_KEY: ${OPENAI_API_KEY}
      EDGEQUAKE_LOG_LEVEL: info
      RUST_LOG: edgequake=info
    depends_on:
      postgres:
        condition: service_healthy
    restart: unless-stopped
    deploy:
      resources:
        limits:
          cpus: '2'
          memory: 4G
        reservations:
          cpus: '1'
          memory: 2G

  postgres:
    image: apache/age:PG16-latest
    environment:
      POSTGRES_DB: edgequake
      POSTGRES_USER: edgequake
      POSTGRES_PASSWORD: ${DB_PASSWORD}
    volumes:
      - postgres_data:/var/lib/postgresql/data
      - ./init-extensions.sql:/docker-entrypoint-initdb.d/init.sql
    healthcheck:
      test: ["CMD-SHELL", "pg_isready -U edgequake"]
      interval: 10s
      timeout: 5s
      retries: 5
    restart: unless-stopped

  caddy:
    image: caddy:latest
    ports:
      - "80:80"
      - "443:443"
    volumes:
      - ./Caddyfile:/etc/caddy/Caddyfile
      - caddy_data:/data
    depends_on:
      - edgequake
    restart: unless-stopped

volumes:
  postgres_data:
  caddy_data:
```

### init-extensions.sql

```sql
-- Enable required extensions
CREATE EXTENSION IF NOT EXISTS age;
CREATE EXTENSION IF NOT EXISTS vector;

-- Load AGE
LOAD 'age';
SET search_path = ag_catalog, "$user", public;
```

### Caddyfile

```caddyfile
your-domain.com {
    reverse_proxy edgequake:8020
    
    # Enable compression
    encode gzip
    
    # Rate limiting
    rate_limit {
        zone api {
            key {remote_host}
            events 60
            window 1m
        }
    }
    
    # Logging
    log {
        output file /var/log/caddy/access.log
        format json
    }
}
```

### Deployment Steps

```bash
# 1. Clone configuration
git clone https://github.com/your-org/edgequake-deploy.git
cd edgequake-deploy

# 2. Configure environment
cp .env.example .env
# Edit .env with your values

# 3. Start services
docker compose -f docker-compose.prod.yml up -d

# 4. Run migrations
docker compose exec edgequake edgequake-cli migrate

# 5. Verify deployment
curl https://your-domain.com/health
```

---

## Option 2: Kubernetes (Production)

### Helm Chart Structure

```
charts/edgequake/
├── Chart.yaml
├── values.yaml
├── templates/
│   ├── deployment.yaml
│   ├── service.yaml
│   ├── ingress.yaml
│   ├── configmap.yaml
│   ├── secret.yaml
│   ├── hpa.yaml
│   └── pdb.yaml
└── README.md
```

### values.yaml

```yaml
# EdgeQuake Helm values

replicaCount: 3

image:
  repository: ghcr.io/your-org/edgequake
  tag: "1.0.0"
  pullPolicy: IfNotPresent

service:
  type: ClusterIP
  port: 8020

ingress:
  enabled: true
  className: nginx
  annotations:
    cert-manager.io/cluster-issuer: letsencrypt-prod
  hosts:
    - host: api.edgequake.example.com
      paths:
        - path: /
          pathType: Prefix
  tls:
    - secretName: edgequake-tls
      hosts:
        - api.edgequake.example.com

resources:
  limits:
    cpu: 2000m
    memory: 4Gi
  requests:
    cpu: 500m
    memory: 1Gi

autoscaling:
  enabled: true
  minReplicas: 3
  maxReplicas: 10
  targetCPUUtilizationPercentage: 70
  targetMemoryUtilizationPercentage: 80

postgresql:
  enabled: true  # Use subchart
  auth:
    username: edgequake
    database: edgequake
    existingSecret: edgequake-db-secret
  primary:
    persistence:
      size: 100Gi
    resources:
      limits:
        cpu: 4000m
        memory: 8Gi

config:
  logLevel: info
  llmModel: gpt-4o-mini
  chunkTokenSize: 1200
  defaultQueryMode: hybrid
```

### templates/deployment.yaml

```yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: {{ include "edgequake.fullname" . }}
  labels:
    {{- include "edgequake.labels" . | nindent 4 }}
spec:
  {{- if not .Values.autoscaling.enabled }}
  replicas: {{ .Values.replicaCount }}
  {{- end }}
  selector:
    matchLabels:
      {{- include "edgequake.selectorLabels" . | nindent 6 }}
  template:
    metadata:
      labels:
        {{- include "edgequake.selectorLabels" . | nindent 8 }}
    spec:
      containers:
        - name: {{ .Chart.Name }}
          image: "{{ .Values.image.repository }}:{{ .Values.image.tag }}"
          imagePullPolicy: {{ .Values.image.pullPolicy }}
          ports:
            - name: http
              containerPort: 8020
              protocol: TCP
          env:
            - name: DATABASE_URL
              valueFrom:
                secretKeyRef:
                  name: {{ include "edgequake.fullname" . }}-db
                  key: url
            - name: OPENAI_API_KEY
              valueFrom:
                secretKeyRef:
                  name: {{ include "edgequake.fullname" . }}-llm
                  key: api-key
            - name: EDGEQUAKE_LOG_LEVEL
              value: {{ .Values.config.logLevel }}
          livenessProbe:
            httpGet:
              path: /health
              port: http
            initialDelaySeconds: 10
            periodSeconds: 30
          readinessProbe:
            httpGet:
              path: /health
              port: http
            initialDelaySeconds: 5
            periodSeconds: 10
          resources:
            {{- toYaml .Values.resources | nindent 12 }}
```

### Deployment Steps

```bash
# 1. Add Helm repository
helm repo add edgequake https://charts.edgequake.io
helm repo update

# 2. Create namespace
kubectl create namespace edgequake

# 3. Create secrets
kubectl create secret generic edgequake-db-secret \
  --from-literal=url="postgres://user:pass@host:5432/db" \
  -n edgequake

kubectl create secret generic edgequake-llm-secret \
  --from-literal=api-key="sk-..." \
  -n edgequake

# 4. Install chart
helm install edgequake edgequake/edgequake \
  --namespace edgequake \
  --values custom-values.yaml

# 5. Verify
kubectl get pods -n edgequake
kubectl logs -l app.kubernetes.io/name=edgequake -n edgequake
```

---

## Post-Deployment Verification

### Health Checks

```bash
# API health
curl https://api.edgequake.example.com/health

# Database connectivity
curl https://api.edgequake.example.com/health/db

# OpenAPI spec
curl https://api.edgequake.example.com/api-docs/openapi.json
```

### Smoke Tests

```bash
# Insert test document
curl -X POST https://api.edgequake.example.com/documents \
  -H "Content-Type: application/json" \
  -d '{"content": ["Deployment verification test."]}'

# Query
curl -X POST https://api.edgequake.example.com/query \
  -H "Content-Type: application/json" \
  -d '{"query": "What was the test?", "mode": "naive"}'
```
```

---

## 6.2 Maintenance Runbook

### edgequake-docs/ops/runbook.md

```markdown
# EdgeQuake Operations Runbook

## Table of Contents

1. [Common Operations](#common-operations)
2. [Monitoring & Alerting](#monitoring--alerting)
3. [Troubleshooting](#troubleshooting)
4. [Disaster Recovery](#disaster-recovery)
5. [Scaling](#scaling)

---

## Common Operations

### Restart Service

**Docker Compose:**
```bash
docker compose restart edgequake
```

**Kubernetes:**
```bash
kubectl rollout restart deployment/edgequake -n edgequake
```

### View Logs

**Docker Compose:**
```bash
# Follow logs
docker compose logs -f edgequake

# Last 100 lines
docker compose logs --tail 100 edgequake

# Filter by level
docker compose logs edgequake 2>&1 | grep -i error
```

**Kubernetes:**
```bash
# All pods
kubectl logs -l app.kubernetes.io/name=edgequake -n edgequake -f

# Specific pod
kubectl logs edgequake-abc123 -n edgequake

# Previous container (after restart)
kubectl logs edgequake-abc123 -n edgequake --previous
```

### Check Service Health

```bash
# Health endpoint
curl -s http://localhost:8020/health | jq

# Expected response:
{
  "status": "healthy",
  "version": "1.0.0",
  "uptime_seconds": 86400,
  "database": "connected",
  "llm_provider": "available"
}
```

### Database Operations

**Run Migrations:**
```bash
# Docker
docker compose exec edgequake edgequake-cli migrate

# Kubernetes
kubectl exec -it deployment/edgequake -n edgequake -- edgequake-cli migrate
```

**Backup Database:**
```bash
# Full backup
pg_dump -h localhost -U edgequake -d edgequake -Fc > backup_$(date +%Y%m%d).dump

# Schema only
pg_dump -h localhost -U edgequake -d edgequake --schema-only > schema.sql
```

**Restore Database:**
```bash
pg_restore -h localhost -U edgequake -d edgequake -c backup.dump
```

---

## Monitoring & Alerting

### Key Metrics

| Metric | Description | Alert Threshold |
|--------|-------------|-----------------|
| `edgequake_requests_total` | Total API requests | N/A (informational) |
| `edgequake_request_duration_seconds` | Request latency | P95 > 500ms |
| `edgequake_errors_total` | Error count | > 10/min |
| `edgequake_active_connections` | DB connections | > 80% pool |
| `edgequake_queue_depth` | Pending documents | > 1000 |

### Prometheus Queries

```promql
# Request rate (5 min window)
rate(edgequake_requests_total[5m])

# Error rate percentage
sum(rate(edgequake_errors_total[5m])) / sum(rate(edgequake_requests_total[5m])) * 100

# P95 latency
histogram_quantile(0.95, rate(edgequake_request_duration_seconds_bucket[5m]))

# Memory usage
container_memory_usage_bytes{container="edgequake"} / container_spec_memory_limit_bytes{container="edgequake"} * 100
```

### Alert Rules

```yaml
# prometheus/alerts.yaml
groups:
  - name: edgequake
    rules:
      - alert: HighErrorRate
        expr: |
          sum(rate(edgequake_errors_total[5m])) / 
          sum(rate(edgequake_requests_total[5m])) > 0.05
        for: 5m
        labels:
          severity: critical
        annotations:
          summary: "High error rate detected"
          description: "Error rate is {{ $value | humanizePercentage }}"

      - alert: HighLatency
        expr: |
          histogram_quantile(0.95, 
            rate(edgequake_request_duration_seconds_bucket[5m])) > 0.5
        for: 5m
        labels:
          severity: warning
        annotations:
          summary: "High P95 latency"
          description: "P95 latency is {{ $value }}s"

      - alert: QueueBacklog
        expr: edgequake_queue_depth > 1000
        for: 10m
        labels:
          severity: warning
        annotations:
          summary: "Document queue backlog"
          description: "{{ $value }} documents waiting"
```

---

## Troubleshooting

### Issue: API Returns 500 Errors

**Symptoms:**
- API requests return HTTP 500
- Error logs show database connection errors

**Resolution:**
1. Check database connectivity:
   ```bash
   psql $DATABASE_URL -c "SELECT 1"
   ```

2. Check connection pool:
   ```bash
   curl http://localhost:8020/metrics | grep connections
   ```

3. Restart if pool exhausted:
   ```bash
   kubectl rollout restart deployment/edgequake -n edgequake
   ```

### Issue: Slow Query Response

**Symptoms:**
- Query latency > 2 seconds
- Users reporting timeouts

**Resolution:**
1. Check LLM provider status:
   ```bash
   curl https://status.openai.com/api/v2/status.json
   ```

2. Review query complexity:
   ```bash
   # Get query metrics
   curl http://localhost:8020/metrics | grep query_duration
   ```

3. Consider reducing `top_k` or switching to `naive` mode

### Issue: Document Processing Stuck

**Symptoms:**
- Documents stuck in "queued" state
- Queue depth increasing

**Resolution:**
1. Check worker status:
   ```bash
   curl http://localhost:8020/health/workers
   ```

2. Check for failed extractions:
   ```bash
   docker compose logs edgequake 2>&1 | grep -i "extraction failed"
   ```

3. Restart workers:
   ```bash
   kubectl rollout restart deployment/edgequake-worker -n edgequake
   ```

### Issue: Out of Memory

**Symptoms:**
- Pods being OOMKilled
- Memory usage at 100%

**Resolution:**
1. Check memory usage pattern:
   ```bash
   kubectl top pods -n edgequake
   ```

2. Reduce batch sizes:
   ```yaml
   # Update config
   embedding.batch_size: 50  # Down from 100
   ```

3. Increase memory limits if justified

---

## Disaster Recovery

### Database Recovery

**From Backup:**
```bash
# Stop application
kubectl scale deployment edgequake --replicas=0 -n edgequake

# Restore database
pg_restore -h $DB_HOST -U $DB_USER -d $DB_NAME -c latest_backup.dump

# Verify data
psql $DATABASE_URL -c "SELECT COUNT(*) FROM ag_catalog.cypher('edgequake', \$\$ MATCH (n) RETURN count(n) \$\$) as (count agtype);"

# Restart application
kubectl scale deployment edgequake --replicas=3 -n edgequake
```

### Full System Recovery

1. Provision new infrastructure
2. Deploy PostgreSQL with AGE
3. Restore database from backup
4. Deploy EdgeQuake
5. Run verification tests
6. Update DNS

### Data Export

```bash
# Export all data
edgequake-cli export --format json --output export.json

# Export graph only
edgequake-cli export --graph-only --output graph.json
```

---

## Scaling

### Horizontal Scaling

**When to Scale:**
- CPU utilization > 70% sustained
- Request queue growing
- P95 latency degrading

**How to Scale:**
```bash
# Manual scaling
kubectl scale deployment edgequake --replicas=5 -n edgequake

# Update HPA
kubectl patch hpa edgequake -n edgequake -p '{"spec":{"maxReplicas":15}}'
```

### Vertical Scaling

**When Needed:**
- Memory-intensive operations
- Large document batches

**How to Scale:**
```yaml
# Update resource limits
resources:
  limits:
    cpu: 4000m
    memory: 8Gi
  requests:
    cpu: 1000m
    memory: 4Gi
```

### Database Scaling

**Connection Pool:**
```toml
[database.pool]
max_connections = 50  # Increase from 20
```

**Read Replicas:**
Configure read replicas for query-heavy workloads.
```

---

## 6.3 Architecture Decision Records

### ADR-001: Rust for Core Implementation

```markdown
# ADR-001: Use Rust for EdgeQuake Core

## Status
Accepted

## Context
We need to rebuild LightRAG with improved performance, safety, and maintainability.

## Decision
Use Rust as the primary implementation language.

## Rationale
1. **Performance**: Zero-cost abstractions, no GC pauses
2. **Safety**: Memory safety without garbage collection
3. **Concurrency**: Fearless concurrency with ownership model
4. **Ecosystem**: Mature async ecosystem (Tokio)
5. **Deployment**: Single binary, easy containerization

## Consequences
- Steeper learning curve for Python developers
- Longer initial development time
- Better long-term performance and reliability
- Easier production deployment
```

### ADR-002: PostgreSQL with AGE for Graph Storage

```markdown
# ADR-002: PostgreSQL with Apache AGE for Graph Storage

## Status
Accepted

## Context
LightRAG supports multiple storage backends (Neo4j, NetworkX, etc.). 
We need a unified storage strategy for EdgeQuake.

## Decision
Use PostgreSQL with Apache AGE extension for graph storage, 
with pgvector for vector similarity search.

## Rationale
1. **Unified Stack**: Single database for graph, vector, and relational data
2. **Operational Simplicity**: One database to manage
3. **Maturity**: PostgreSQL is battle-tested
4. **Cost**: No additional licensing fees
5. **Cypher Support**: AGE supports openCypher query language

## Alternatives Considered
- **Neo4j**: Better graph features, but adds operational complexity
- **SurrealDB**: Unified solution, but less mature
- **Separate Vector DB**: Adds another service to manage

## Consequences
- Some graph features may be less optimized than dedicated graph DB
- Need to manage PostgreSQL extensions
- Simpler operations and deployment
```

### ADR-003: Axum for HTTP Framework

```markdown
# ADR-003: Use Axum for HTTP Framework

## Status
Accepted

## Context
Need to choose a web framework for the REST API.

## Decision
Use Axum with Tower middleware ecosystem.

## Rationale
1. **Tokio Native**: Built by Tokio team, guaranteed compatibility
2. **Type Safety**: Leverages Rust's type system for route handlers
3. **Performance**: Among the fastest Rust web frameworks
4. **Middleware**: Tower ecosystem for observability, auth, etc.
5. **Active Development**: Well-maintained, growing community

## Alternatives Considered
- **Actix-web**: Faster in some benchmarks, but different async model
- **Warp**: Good, but less flexible routing
- **Rocket**: More opinionated, requires nightly for some features

## Consequences
- Strong integration with Tokio async ecosystem
- Need to learn Tower service pattern
- Excellent performance characteristics
```

---

## 6.4 Migration Playbook

### edgequake-docs/migration/playbook.md

```markdown
# LightRAG to EdgeQuake Migration Playbook

## Overview

This playbook guides the migration from Python LightRAG to Rust EdgeQuake.

## Pre-Migration Checklist

- [ ] EdgeQuake deployed and verified
- [ ] Database backup completed
- [ ] Migration window scheduled
- [ ] Rollback plan documented
- [ ] Stakeholders notified

## Migration Approaches

### Option A: Fresh Start (Recommended for <1GB data)

1. Deploy EdgeQuake
2. Re-ingest all documents
3. Switch traffic to EdgeQuake
4. Decommission LightRAG

**Pros**: Clean slate, no compatibility issues  
**Cons**: Requires re-processing time

### Option B: Data Migration (For large datasets)

1. Export LightRAG data
2. Transform to EdgeQuake format
3. Import into EdgeQuake
4. Validate data integrity
5. Switch traffic

**Pros**: Faster for large datasets  
**Cons**: More complex, potential data issues

---

## Option A: Fresh Start Migration

### Step 1: Deploy EdgeQuake

```bash
# Deploy alongside LightRAG
kubectl apply -f edgequake-deployment.yaml

# Verify health
curl https://edgequake.example.com/health
```

### Step 2: Extract Document Sources

```python
# scripts/export_documents.py
import json
from lightrag import LightRAG

rag = LightRAG(working_dir="./rag_storage")

# Get all document content from KV store
documents = []
for key in rag.full_docs.keys():
    doc = rag.full_docs.get(key)
    documents.append({
        "id": key,
        "content": doc,
        "file_path": rag.doc_meta.get(key, {}).get("file_path")
    })

with open("documents.json", "w") as f:
    json.dump(documents, f)
```

### Step 3: Ingest into EdgeQuake

```bash
# Bulk insert via API
cat documents.json | jq -c '.[]' | while read doc; do
    curl -X POST https://edgequake.example.com/documents \
        -H "Content-Type: application/json" \
        -d "$doc"
    sleep 0.1  # Rate limit
done
```

### Step 4: Validate

```bash
# Compare document counts
lightrag_count=$(python -c "from lightrag import LightRAG; print(len(LightRAG('./rag_storage').full_docs.keys()))")
edgequake_count=$(curl -s https://edgequake.example.com/stats | jq '.document_count')

echo "LightRAG: $lightrag_count documents"
echo "EdgeQuake: $edgequake_count documents"
```

### Step 5: Switch Traffic

```bash
# Update load balancer / DNS
# Route traffic to EdgeQuake

# Monitor for errors
kubectl logs -l app=edgequake -f
```

---

## Option B: Data Migration

### Step 1: Export LightRAG Data

```python
# scripts/export_full.py
import json
import os
from lightrag import LightRAG

rag = LightRAG(working_dir="./rag_storage")

export = {
    "documents": {},
    "chunks": {},
    "entities": {},
    "relationships": {},
    "embeddings": {}
}

# Export documents
for key in rag.full_docs.keys():
    export["documents"][key] = rag.full_docs.get(key)

# Export chunks
for key in rag.text_chunks.keys():
    export["chunks"][key] = rag.text_chunks.get(key)

# Export entities
for key in rag.entities.keys():
    export["entities"][key] = rag.entities.get(key)

# Export relationships  
for key in rag.relationships.keys():
    export["relationships"][key] = rag.relationships.get(key)

with open("lightrag_export.json", "w") as f:
    json.dump(export, f)
```

### Step 2: Transform Data

```python
# scripts/transform.py
import json

with open("lightrag_export.json") as f:
    lightrag = json.load(f)

edgequake = {
    "documents": [],
    "graph_nodes": [],
    "graph_edges": [],
    "embeddings": []
}

# Transform entities
for entity_key, entity_data in lightrag["entities"].items():
    edgequake["graph_nodes"].append({
        "id": entity_key,
        "entity_name": entity_data.get("entity_name"),
        "entity_type": entity_data.get("entity_type"),
        "description": entity_data.get("description"),
        "source_id": entity_data.get("source_id")
    })

# Transform relationships
for rel_key, rel_data in lightrag["relationships"].items():
    edgequake["graph_edges"].append({
        "source": rel_data.get("src_id"),
        "target": rel_data.get("tgt_id"),
        "description": rel_data.get("description"),
        "weight": rel_data.get("weight", 1.0)
    })

with open("edgequake_import.json", "w") as f:
    json.dump(edgequake, f)
```

### Step 3: Import into EdgeQuake

```bash
# Use EdgeQuake import CLI
edgequake-cli import --file edgequake_import.json --validate

# Or via API
curl -X POST https://edgequake.example.com/admin/import \
    -H "Content-Type: application/json" \
    -d @edgequake_import.json
```

### Step 4: Validate Migration

```bash
# Run validation script
edgequake-cli validate-migration \
    --lightrag-export lightrag_export.json \
    --report validation_report.json
```

---

## Rollback Procedure

If migration fails:

1. **Immediate** (< 5 min since cutover):
   - Revert DNS/LB to LightRAG
   - No data loss

2. **Short-term** (< 1 hour):
   - Stop EdgeQuake
   - Verify LightRAG still healthy
   - Route traffic back

3. **Long-term**:
   - Investigate failure
   - Fix issues
   - Retry migration
```

---

## 6.5 Operations Manual

### edgequake-docs/ops/manual.md

```markdown
# EdgeQuake Operations Manual

## Daily Operations

### Morning Checklist

- [ ] Check health dashboard
- [ ] Review overnight alerts
- [ ] Verify queue depth is normal
- [ ] Check error rate trends

### Monitoring Dashboard URLs

| Dashboard | URL | Purpose |
|-----------|-----|---------|
| Health | /health | Basic health check |
| Metrics | /metrics | Prometheus metrics |
| Grafana | grafana.example.com | Visualizations |
| Alerts | alertmanager.example.com | Alert status |

## Weekly Operations

### Performance Review

1. Review P95 latency trends
2. Check resource utilization
3. Analyze query patterns
4. Plan capacity if needed

### Security Updates

1. Run `cargo audit` for vulnerabilities
2. Update dependencies if needed
3. Review access logs

## Monthly Operations

### Capacity Planning

1. Project growth for next 3 months
2. Estimate resource requirements
3. Plan scaling actions

### Disaster Recovery Test

1. Restore from backup to staging
2. Verify data integrity
3. Document any issues

## Release Procedures

### Standard Release

1. Merge PR to main
2. Wait for CI to pass
3. Create release tag
4. Automated deployment to staging
5. Run smoke tests
6. Promote to production

### Hotfix Release

1. Branch from production tag
2. Apply fix
3. Expedited code review
4. Direct deploy to production
5. Monitor closely
6. Backport to main

## Contact Information

| Role | Contact | Escalation |
|------|---------|------------|
| On-call | PagerDuty | Automatic |
| Platform Lead | platform@example.com | High severity |
| Security | security@example.com | Security issues |
```

---

## Week-by-Week Tasks

### Week 11: Deployment & Runbook

| Task | Description | Owner | Status |
|------|-------------|-------|--------|
| 6.1.1 | Write Docker Compose deployment guide | DevOps | ⬜ |
| 6.1.2 | Create Helm chart | DevOps | ⬜ |
| 6.1.3 | Document Kubernetes deployment | DevOps | ⬜ |
| 6.1.4 | Write maintenance runbook | SRE | ⬜ |
| 6.1.5 | Create monitoring dashboards | SRE | ⬜ |
| 6.1.6 | Define alert rules | SRE | ⬜ |

### Week 12: ADRs & Migration

| Task | Description | Owner | Status |
|------|-------------|-------|--------|
| 6.2.1 | Document all ADRs | Tech Lead | ⬜ |
| 6.2.2 | Write migration playbook | Tech Lead | ⬜ |
| 6.2.3 | Create migration scripts | Backend | ⬜ |
| 6.2.4 | Test migration on staging | QA | ⬜ |
| 6.2.5 | Write operations manual | SRE | ⬜ |
| 6.2.6 | Conduct handoff meeting | Tech Lead | ⬜ |

---

## Acceptance Criteria

- [ ] Deployment works on Docker Compose and Kubernetes
- [ ] All runbook procedures tested
- [ ] ADRs document all major decisions
- [ ] Migration playbook validated on staging
- [ ] Operations manual reviewed by SRE team
- [ ] Handoff meeting completed

---

## Related Documents

- [Phase 5: Quality Assurance](phase-5-quality-assurance.md) - Parallel phase
- [master.md](../master.md) - Overall plan
- [plan_progress.md](../plan_progress.md) - Progress tracker
