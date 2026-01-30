# Iteration 12: Production Deployment - OBSERVE

## Topic

**012_production_deployment** - Production Deployment: From Dev to Scale

## Research Sources

### 1. Docker Configuration (`edgequake/docker/`)

#### Dockerfile (Multi-Stage Build)

```dockerfile
# Stage 1: Build
FROM rust:1.78-bookworm AS builder
WORKDIR /app
RUN cargo build --release --locked

# Stage 2: Runtime (minimal image)
FROM debian:bookworm-slim AS runtime
COPY --from=builder /app/target/release/edgequake /usr/local/bin/
USER edgequake  # Non-root user
EXPOSE 8080
HEALTHCHECK --interval=30s --timeout=10s --start-period=5s --retries=3 \
    CMD curl -f http://localhost:8080/health || exit 1
```

**Key Production Patterns:**

- Multi-stage build for minimal image size
- Non-root user for security
- Built-in health check for container orchestrators
- Locked dependencies for reproducibility

#### Docker Compose Stack

```yaml
services:
  edgequake:
    restart: unless-stopped
    ports:
      - "${EDGEQUAKE_PORT:-8080}:8080"
    environment:
      - EDGEQUAKE_DATABASE_URL=postgres://...
      - RUST_LOG=info,edgequake=debug
    depends_on:
      postgres:
        condition: service_healthy
    healthcheck:
      test: ["CMD", "curl", "-f", "http://localhost:8080/health"]
      interval: 30s
      timeout: 10s
      retries: 3
      start_period: 10s

  postgres:
    # Custom image with pgvector + Apache AGE
    volumes:
      - postgres-data:/var/lib/postgresql/data
    healthcheck:
      test: ["CMD-SHELL", "pg_isready -U edgequake -d edgequake"]
```

**Production Features:**

- Health-check dependencies for ordered startup
- Persistent volumes for data
- Environment variable configuration
- Automatic restarts

#### Custom PostgreSQL Image (Dockerfile.postgres)

```dockerfile
FROM postgres:16-bookworm
# Build and install pgvector
RUN git clone --branch v0.7.4 https://github.com/pgvector/pgvector.git && \
    cd pgvector && make && make install

# Build and install Apache AGE
RUN git clone --branch PG16/v1.6.0-rc0 https://github.com/apache/age.git && \
    cd age && make && make install
```

---

### 2. Connection Pool Management (`edgequake-storage/src/adapters/postgres/`)

#### PostgresPool Configuration

```rust
pub struct PostgresPool {
    pool: Arc<RwLock<Option<PgPool>>>,
    config: PostgresConfig,
}

impl PostgresPool {
    pub async fn initialize(&self) -> Result<()> {
        let pool = PgPoolOptions::new()
            .max_connections(self.config.max_connections)
            .min_connections(self.config.min_connections)
            .acquire_timeout(self.config.connect_timeout)
            .idle_timeout(Some(self.config.idle_timeout))
            .connect(&self.config.connection_url())
            .await?;

        // Auto-enable extensions
        self.setup_extensions(&pool).await?;
    }

    pub async fn health_check(&self) -> Result<bool> {
        let pool = self.get().await?;
        let row = sqlx::query("SELECT 1 as health")
            .fetch_one(&pool).await?;
        Ok(true)
    }
}
```

**Connection Pool Features:**

- Lazy initialization
- Configurable min/max connections
- Auto-setup of required extensions (pgvector, AGE, pgcrypto)
- Built-in health check method

---

### 3. Health Endpoints (Kubernetes-Ready)

#### Available Endpoints

| Endpoint      | Purpose         | Response                |
| ------------- | --------------- | ----------------------- |
| `GET /health` | Overall health  | `{"status": "healthy"}` |
| `GET /ready`  | Readiness probe | `{"status": "ready"}`   |
| `GET /live`   | Liveness probe  | `{"status": "live"}`    |

**Kubernetes Mapping:**

- `/live` → livenessProbe (is container alive?)
- `/ready` → readinessProbe (is container ready for traffic?)
- `/health` → Combined health with component status

---

### 4. Runbook Operations (`edgequake/docs/runbook.md`)

#### Alert Thresholds

| Metric          | Warning | Critical |
| --------------- | ------- | -------- |
| API p99 latency | > 500ms | > 2s     |
| Error rate      | > 1%    | > 5%     |
| Memory usage    | > 70%   | > 90%    |
| CPU usage       | > 70%   | > 90%    |
| Storage usage   | > 70%   | > 90%    |

#### Horizontal Scaling

```yaml
services:
  edgequake:
    image: edgequake:latest
    deploy:
      replicas: 3
    environment:
      - DATABASE_URL=postgres://...
```

**Prerequisites:**

- Shared storage backend (PostgreSQL)
- Load balancer in front
- Stateless API instances

#### Vertical Scaling

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

### 5. Configuration (`edgequake/docs/configuration.md`)

#### Environment Variables

| Variable                 | Default   | Description            |
| ------------------------ | --------- | ---------------------- |
| `EDGEQUAKE_HOST`         | `0.0.0.0` | Bind address           |
| `EDGEQUAKE_PORT`         | `8080`    | Server port            |
| `EDGEQUAKE_DATABASE_URL` | -         | PostgreSQL URL         |
| `EDGEQUAKE_NAMESPACE`    | `null`    | Multi-tenant namespace |
| `OPENAI_API_KEY`         | -         | LLM API key            |
| `RUST_LOG`               | `info`    | Log level              |

#### Storage Configuration

| Config Field              | Default | Description          |
| ------------------------- | ------- | -------------------- |
| `storage.max_connections` | `10`    | Max pool connections |
| `storage.min_connections` | `1`     | Min pool connections |
| `storage.connect_timeout` | `30s`   | Connection timeout   |

#### Performance Tuning Profiles

**High Throughput:**

```toml
[storage]
max_connections = 50
min_connections = 10

[pipeline]
concurrency = 16
chunk_size = 800
```

**Memory Constrained:**

```toml
[storage]
max_connections = 5
min_connections = 1

[pipeline]
concurrency = 2
chunk_size = 1500
```

---

### 6. Observability (main.rs)

#### Structured Logging

```rust
tracing_subscriber::registry()
    .with(EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| "edgequake=debug,tower_http=debug".into()))
    .with(tracing_subscriber::fmt::layer())
    .init();
```

**Log Levels:**

```bash
RUST_LOG=info                     # General info
RUST_LOG=edgequake=debug         # Debug EdgeQuake only
RUST_LOG=edgequake_pipeline=trace # Trace pipeline
```

---

### 7. Disaster Recovery

#### Database Backup

```bash
# PostgreSQL backup (includes vectors via pgvector)
pg_dump -h localhost -U edgequake -d edgequake > backup.sql

# Restore
psql -h localhost -U edgequake -d edgequake < backup.sql
```

#### Recovery Procedure

1. Assess damage - Determine affected data
2. Restore database - From most recent backup
3. Verify integrity - Check document counts
4. Re-index if needed - Trigger reprocessing
5. Validate - Run health checks

---

### 8. Security Practices

#### API Key Authentication

```bash
EDGEQUAKE_API_KEYS=key1,key2,key3

curl -H "Authorization: Bearer key1" http://localhost:8080/api/v1/query
# or
curl -H "X-API-Key: key1" http://localhost:8080/api/v1/query
```

#### CORS Configuration

```toml
[api]
cors_enabled = true
cors_origins = ["https://app.example.com"]
```

---

### 9. Kubernetes Pod Lifecycle (from kubernetes.io)

#### Three Types of Probes

1. **livenessProbe** - Is the container running? If fails, kubelet kills container
2. **readinessProbe** - Is the container ready for traffic? If fails, removed from endpoints
3. **startupProbe** - Has the app started? Disables other probes until success

#### Probe Mechanisms

- `httpGet` - HTTP GET request (EdgeQuake uses this)
- `tcpSocket` - TCP port check
- `exec` - Command execution
- `grpc` - gRPC health check

#### Pod Restart Policy

- `Always` (default) - Automatically restart after any termination
- `OnFailure` - Only restart on non-zero exit
- `Never` - Never restart

---

## Key Metrics for Articles

| Metric                      | Value                            | Source             |
| --------------------------- | -------------------------------- | ------------------ |
| Multi-stage build           | 2 stages                         | Dockerfile         |
| Health endpoints            | 3 (`/health`, `/ready`, `/live`) | handlers/mod.rs    |
| Alert thresholds defined    | 5 metrics                        | runbook.md         |
| Max connections default     | 10                               | configuration.md   |
| High throughput connections | 50                               | configuration.md   |
| Restart backoff cap         | 300s (5 min)                     | kubernetes.io      |
| Termination grace period    | 30s default                      | kubernetes.io      |
| Health check interval       | 30s                              | docker-compose.yml |

---

## Differentiators vs Other RAG Frameworks

| Feature            | EdgeQuake      | LangChain  | LlamaIndex |
| ------------------ | -------------- | ---------- | ---------- |
| Production Docker  | ✅ Multi-stage | ❌ DIY     | ❌ DIY     |
| Health endpoints   | ✅ 3 K8s-ready | ❌ None    | ❌ None    |
| Connection pooling | ✅ Built-in    | ❌ DIY     | ❌ DIY     |
| Runbook            | ✅ 316 lines   | ❌ None    | ❌ None    |
| Alert thresholds   | ✅ Documented  | ❌ None    | ❌ None    |
| Horizontal scaling | ✅ Stateless   | ⚠️ Complex | ⚠️ Complex |

---

## Code Snippets for Articles

### Health Check Handler

```rust
// edgequake-api/src/handlers/health.rs
pub async fn health_check(State(state): State<AppState>) -> impl IntoResponse {
    let storage_ok = state.graph_storage.health_check().await.unwrap_or(false);
    let llm_ok = state.llm_provider.health_check().await.unwrap_or(false);

    Json(HealthResponse {
        status: if storage_ok && llm_ok { "healthy" } else { "degraded" },
        components: vec![
            Component { name: "storage", healthy: storage_ok },
            Component { name: "llm", healthy: llm_ok },
        ],
    })
}
```

### Graceful Shutdown

```rust
// main.rs
let server = Server::bind(&addr)
    .serve(app)
    .with_graceful_shutdown(shutdown_signal());

async fn shutdown_signal() {
    tokio::signal::ctrl_c().await.expect("Failed to install CTRL+C handler");
    info!("Received shutdown signal, draining connections...");
}
```

---

## Cited Research

- **LightRAG Paper**: arXiv:2410.05779 - Graph-based RAG architecture foundation
- **Microsoft GraphRAG**: Production patterns for enterprise graph retrieval
- **Kubernetes Documentation**: Pod lifecycle, health probes, graceful shutdown

---

## Next Steps

1. **orient.md**: Audience analysis (DevOps, SREs, Platform engineers)
2. **decide.md**: Article structure for production deployment
3. **Articles**: Medium, LinkedIn, X.com, HackerNews, Reddit, Substack
