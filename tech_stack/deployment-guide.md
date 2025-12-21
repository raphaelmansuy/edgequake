# Deployment Guide for EdgeQuake

**Technology Stack**: Rust + Docker + Kubernetes  
**Date**: 2025-12-21  
**Status**: Complete  
**Related**: [technology_choice.md](./technology_choice.md), [surrealdb.md](./surrealdb.md), [axum.md](./axum.md)

---

## Overview

This guide provides comprehensive deployment strategies for EdgeQuake (Rust-based RAG system derived from LightRAG), covering containerization, orchestration, configuration management, monitoring, and production best practices.

**Deployment Targets**:

- Local development (Docker Compose)
- Cloud platforms (AWS, GCP, Azure)
- Kubernetes clusters
- Bare metal servers

---

## Docker Containerization

### Optimized Multi-Stage Dockerfile

```dockerfile
# Stage 1: Build
FROM rust:1.75-slim as builder

WORKDIR /app

# Install system dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# Copy dependency manifests
COPY Cargo.toml Cargo.lock ./

# Create dummy main to cache dependencies
RUN mkdir src && \
    echo "fn main() {}" > src/main.rs && \
    cargo build --release && \
    rm -rf src

# Copy actual source code
COPY src ./src

# Build application (dependencies cached)
RUN touch src/main.rs && \
    cargo build --release

# Stage 2: Runtime
FROM debian:bookworm-slim

WORKDIR /app

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

# Copy binary from builder
COPY --from=builder /app/target/release/lightrag /app/lightrag

# Create non-root user
RUN useradd -m -u 1000 lightrag && \
    chown -R lightrag:lightrag /app

USER lightrag

# Expose API port
EXPOSE 8080

# Health check
HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
    CMD curl -f http://localhost:8080/health || exit 1

# Run application
CMD ["./lightrag"]
```

### Build and Run

```bash
# Build image
docker build -t lightrag:latest .

# Run container
docker run -d \
    --name lightrag \
    -p 8080:8080 \
    -e OPENAI_API_KEY=sk-... \
    -e SURREALDB_URL=ws://surrealdb:8000 \
    -v lightrag-data:/app/data \
    lightrag:latest

# View logs
docker logs -f lightrag

# Stop container
docker stop lightrag

# Remove container
docker rm lightrag
```

---

## Docker Compose Setup

### Development Configuration

```yaml
# docker-compose.yml

version: '3.8'

services:
  lightrag:
    build:
      context: .
      dockerfile: Dockerfile
    ports:
      - "8080:8080"
    environment:
      - RUST_LOG=info
      - OPENAI_API_KEY=${OPENAI_API_KEY}
      - SURREALDB_URL=ws://surrealdb:8000
      - SURREALDB_NAMESPACE=lightrag
      - SURREALDB_DATABASE=main
    volumes:
      - lightrag-data:/app/data
    depends_on:
      surrealdb:
        condition: service_healthy
    networks:
      - lightrag-network
    restart: unless-stopped

  surrealdb:
    image: surrealdb/surrealdb:latest
    command: start --log trace --user root --pass root memory
    ports:
      - "8000:8000"
    volumes:
      - surrealdb-data:/data
    networks:
      - lightrag-network
    healthcheck:
      test: ["CMD", "curl", "-f", "http://localhost:8000/health"]
      interval: 10s
      timeout: 5s
      retries: 5
    restart: unless-stopped

  # Optional: PostgreSQL with AGE extension
  postgres:
    image: apache/age:latest
    environment:
      - POSTGRES_USER=lightrag
      - POSTGRES_PASSWORD=lightrag
      - POSTGRES_DB=lightrag
    ports:
      - "5432:5432"
    volumes:
      - postgres-data:/var/lib/postgresql/data
    networks:
      - lightrag-network
    healthcheck:
      test: ["CMD-SHELL", "pg_isready -U lightrag"]
      interval: 10s
      timeout: 5s
      retries: 5
    restart: unless-stopped

  # Optional: Redis for caching
  redis:
    image: redis:7-alpine
    ports:
      - "6379:6379"
    volumes:
      - redis-data:/data
    networks:
      - lightrag-network
    healthcheck:
      test: ["CMD", "redis-cli", "ping"]
      interval: 10s
      timeout: 3s
      retries: 5
    restart: unless-stopped

  # Optional: Prometheus for monitoring
  prometheus:
    image: prom/prometheus:latest
    ports:
      - "9090:9090"
    volumes:
      - ./prometheus.yml:/etc/prometheus/prometheus.yml
      - prometheus-data:/prometheus
    command:
      - '--config.file=/etc/prometheus/prometheus.yml'
      - '--storage.tsdb.path=/prometheus'
    networks:
      - lightrag-network
    restart: unless-stopped

  # Optional: Grafana for visualization
  grafana:
    image: grafana/grafana:latest
    ports:
      - "3000:3000"
    environment:
      - GF_SECURITY_ADMIN_PASSWORD=admin
    volumes:
      - grafana-data:/var/lib/grafana
    networks:
      - lightrag-network
    restart: unless-stopped

volumes:
  lightrag-data:
  surrealdb-data:
  postgres-data:
  redis-data:
  prometheus-data:
  grafana-data:

networks:
  lightrag-network:
    driver: bridge
```

### Production Configuration

```yaml
# docker-compose.prod.yml

version: '3.8'

services:
  lightrag:
    image: lightrag:${VERSION:-latest}
    ports:
      - "8080:8080"
    environment:
      - RUST_LOG=warn
      - OPENAI_API_KEY=${OPENAI_API_KEY}
      - SURREALDB_URL=${SURREALDB_URL}
      - MAX_WORKERS=4
      - REQUEST_TIMEOUT=30
    volumes:
      - lightrag-data:/app/data
    deploy:
      replicas: 3
      resources:
        limits:
          cpus: '2'
          memory: 4G
        reservations:
          cpus: '1'
          memory: 2G
    healthcheck:
      test: ["CMD", "curl", "-f", "http://localhost:8080/health"]
      interval: 30s
      timeout: 10s
      retries: 3
      start_period: 40s
    restart: unless-stopped
    networks:
      - lightrag-network

  # Production SurrealDB with persistence
  surrealdb:
    image: surrealdb/surrealdb:latest
    command: start file:/data/database.db --log warn --user ${SURREAL_USER} --pass ${SURREAL_PASS}
    volumes:
      - surrealdb-data:/data
    deploy:
      resources:
        limits:
          cpus: '2'
          memory: 8G
    networks:
      - lightrag-network
    restart: unless-stopped

  # Nginx reverse proxy
  nginx:
    image: nginx:alpine
    ports:
      - "80:80"
      - "443:443"
    volumes:
      - ./nginx.conf:/etc/nginx/nginx.conf:ro
      - ./ssl:/etc/nginx/ssl:ro
    depends_on:
      - lightrag
    networks:
      - lightrag-network
    restart: unless-stopped

volumes:
  lightrag-data:
  surrealdb-data:

networks:
  lightrag-network:
    driver: bridge
```

### Start Services

```bash
# Development
docker-compose up -d

# Production
docker-compose -f docker-compose.yml -f docker-compose.prod.yml up -d

# Scale replicas
docker-compose up -d --scale lightrag=3

# View logs
docker-compose logs -f lightrag

# Stop services
docker-compose down

# Stop and remove volumes
docker-compose down -v
```

---

## Kubernetes Deployment

### Namespace

```yaml
# k8s/namespace.yaml

apiVersion: v1
kind: Namespace
metadata:
  name: lightrag
```

### ConfigMap

```yaml
# k8s/configmap.yaml

apiVersion: v1
kind: ConfigMap
metadata:
  name: lightrag-config
  namespace: lightrag
data:
  RUST_LOG: "info"
  SURREALDB_NAMESPACE: "lightrag"
  SURREALDB_DATABASE: "main"
  MAX_WORKERS: "4"
  REQUEST_TIMEOUT: "30"
```

### Secret

```yaml
# k8s/secret.yaml

apiVersion: v1
kind: Secret
metadata:
  name: lightrag-secrets
  namespace: lightrag
type: Opaque
stringData:
  OPENAI_API_KEY: "sk-..."
  SURREALDB_USER: "root"
  SURREALDB_PASSWORD: "root"
```

Create secret from command line:

```bash
kubectl create secret generic lightrag-secrets \
  --from-literal=OPENAI_API_KEY=sk-... \
  --from-literal=SURREALDB_USER=root \
  --from-literal=SURREALDB_PASSWORD=root \
  -n lightrag
```

### Deployment

```yaml
# k8s/deployment.yaml

apiVersion: apps/v1
kind: Deployment
metadata:
  name: lightrag
  namespace: lightrag
  labels:
    app: lightrag
spec:
  replicas: 3
  selector:
    matchLabels:
      app: lightrag
  template:
    metadata:
      labels:
        app: lightrag
    spec:
      containers:
      - name: lightrag
        image: lightrag:latest
        imagePullPolicy: Always
        ports:
        - containerPort: 8080
          name: http
        env:
        - name: SURREALDB_URL
          value: "ws://surrealdb-service:8000"
        envFrom:
        - configMapRef:
            name: lightrag-config
        - secretRef:
            name: lightrag-secrets
        resources:
          requests:
            cpu: "500m"
            memory: "1Gi"
          limits:
            cpu: "2"
            memory: "4Gi"
        livenessProbe:
          httpGet:
            path: /health
            port: 8080
          initialDelaySeconds: 30
          periodSeconds: 10
          timeoutSeconds: 5
          failureThreshold: 3
        readinessProbe:
          httpGet:
            path: /health
            port: 8080
          initialDelaySeconds: 10
          periodSeconds: 5
          timeoutSeconds: 3
          failureThreshold: 3
        volumeMounts:
        - name: data
          mountPath: /app/data
      volumes:
      - name: data
        persistentVolumeClaim:
          claimName: lightrag-pvc
```

### Service

```yaml
# k8s/service.yaml

apiVersion: v1
kind: Service
metadata:
  name: lightrag-service
  namespace: lightrag
spec:
  selector:
    app: lightrag
  ports:
  - protocol: TCP
    port: 80
    targetPort: 8080
  type: LoadBalancer
```

### Ingress

```yaml
# k8s/ingress.yaml

apiVersion: networking.k8s.io/v1
kind: Ingress
metadata:
  name: lightrag-ingress
  namespace: lightrag
  annotations:
    cert-manager.io/cluster-issuer: "letsencrypt-prod"
    nginx.ingress.kubernetes.io/rate-limit: "100"
spec:
  ingressClassName: nginx
  tls:
  - hosts:
    - lightrag.example.com
    secretName: lightrag-tls
  rules:
  - host: lightrag.example.com
    http:
      paths:
      - path: /
        pathType: Prefix
        backend:
          service:
            name: lightrag-service
            port:
              number: 80
```

### Persistent Volume Claim

```yaml
# k8s/pvc.yaml

apiVersion: v1
kind: PersistentVolumeClaim
metadata:
  name: lightrag-pvc
  namespace: lightrag
spec:
  accessModes:
    - ReadWriteOnce
  resources:
    requests:
      storage: 20Gi
  storageClassName: standard
```

### HorizontalPodAutoscaler

```yaml
# k8s/hpa.yaml

apiVersion: autoscaling/v2
kind: HorizontalPodAutoscaler
metadata:
  name: lightrag-hpa
  namespace: lightrag
spec:
  scaleTargetRef:
    apiVersion: apps/v1
    kind: Deployment
    name: lightrag
  minReplicas: 2
  maxReplicas: 10
  metrics:
  - type: Resource
    resource:
      name: cpu
      target:
        type: Utilization
        averageUtilization: 70
  - type: Resource
    resource:
      name: memory
      target:
        type: Utilization
        averageUtilization: 80
```

### Apply Kubernetes Manifests

```bash
# Create namespace
kubectl apply -f k8s/namespace.yaml

# Apply configurations
kubectl apply -f k8s/configmap.yaml
kubectl apply -f k8s/secret.yaml

# Deploy application
kubectl apply -f k8s/pvc.yaml
kubectl apply -f k8s/deployment.yaml
kubectl apply -f k8s/service.yaml
kubectl apply -f k8s/ingress.yaml
kubectl apply -f k8s/hpa.yaml

# Check status
kubectl get all -n lightrag

# View logs
kubectl logs -f deployment/lightrag -n lightrag

# Scale deployment
kubectl scale deployment/lightrag --replicas=5 -n lightrag

# Update image
kubectl set image deployment/lightrag lightrag=lightrag:v1.1.0 -n lightrag

# Rollback
kubectl rollout undo deployment/lightrag -n lightrag
```

---

## Configuration Management

### Environment Variables

```bash
# .env file for Docker Compose

# LLM Configuration
OPENAI_API_KEY=sk-...
ANTHROPIC_API_KEY=sk-ant-...
LLM_MODEL=gpt-4
LLM_TEMPERATURE=0.7
MAX_TOKENS=4000

# Database Configuration
SURREALDB_URL=ws://surrealdb:8000
SURREALDB_NAMESPACE=lightrag
SURREALDB_DATABASE=main
SURREALDB_USER=root
SURREALDB_PASSWORD=root

# Server Configuration
RUST_LOG=info
SERVER_HOST=0.0.0.0
SERVER_PORT=8080
MAX_WORKERS=4
REQUEST_TIMEOUT=30

# Feature Flags
ENABLE_CACHING=true
ENABLE_METRICS=true
```

### Configuration File

```toml
# config.toml

[server]
host = "0.0.0.0"
port = 8080
workers = 4
request_timeout = 30

[llm]
provider = "openai"
model = "gpt-4"
temperature = 0.7
max_tokens = 4000

[storage]
backend = "surrealdb"
url = "ws://localhost:8000"
namespace = "lightrag"
database = "main"

[embedding]
model = "text-embedding-ada-002"
dimension = 1536

[features]
caching = true
metrics = true
tracing = true
```

Load configuration in Rust:

```rust
use serde::Deserialize;
use config::{Config, ConfigError, File};

#[derive(Debug, Deserialize)]
pub struct Settings {
    pub server: ServerConfig,
    pub llm: LLMConfig,
    pub storage: StorageConfig,
    pub embedding: EmbeddingConfig,
    pub features: Features,
}

impl Settings {
    pub fn new() -> Result<Self, ConfigError> {
        let config = Config::builder()
            .add_source(File::with_name("config"))
            .add_source(config::Environment::with_prefix("LIGHTRAG"))
            .build()?;
        
        config.try_deserialize()
    }
}
```

---

## Monitoring and Observability

### Prometheus Metrics

```rust
use prometheus::{Counter, Gauge, Histogram, Registry};

lazy_static! {
    static ref REGISTRY: Registry = Registry::new();
    
    static ref HTTP_REQUESTS_TOTAL: Counter = Counter::new(
        "lightrag_http_requests_total",
        "Total HTTP requests"
    ).unwrap();
    
    static ref HTTP_REQUEST_DURATION: Histogram = Histogram::new(
        "lightrag_http_request_duration_seconds",
        "HTTP request duration in seconds"
    ).unwrap();
    
    static ref ACTIVE_CONNECTIONS: Gauge = Gauge::new(
        "lightrag_active_connections",
        "Active connections"
    ).unwrap();
}

pub fn init_metrics() {
    REGISTRY.register(Box::new(HTTP_REQUESTS_TOTAL.clone())).unwrap();
    REGISTRY.register(Box::new(HTTP_REQUEST_DURATION.clone())).unwrap();
    REGISTRY.register(Box::new(ACTIVE_CONNECTIONS.clone())).unwrap();
}

// Expose metrics endpoint
async fn metrics_handler() -> String {
    use prometheus::Encoder;
    let encoder = prometheus::TextEncoder::new();
    let metric_families = REGISTRY.gather();
    let mut buffer = vec![];
    encoder.encode(&metric_families, &mut buffer).unwrap();
    String::from_utf8(buffer).unwrap()
}
```

### Prometheus Configuration

```yaml
# prometheus.yml

global:
  scrape_interval: 15s
  evaluation_interval: 15s

scrape_configs:
  - job_name: 'lightrag'
    static_configs:
      - targets: ['lightrag:8080']
    metrics_path: '/metrics'
```

### Grafana Dashboard

Import dashboard JSON or create custom dashboard with panels:

- HTTP request rate (requests/sec)
- Request duration (p50, p95, p99)
- Error rate (%)
- Active connections
- Database query performance
- Memory usage
- CPU usage

---

## Health Checks

### Health Check Endpoint

```rust
use axum::{Json, http::StatusCode};
use serde_json::json;

pub async fn health_check() -> (StatusCode, Json<serde_json::Value>) {
    (
        StatusCode::OK,
        Json(json!({
            "status": "healthy",
            "version": env!("CARGO_PKG_VERSION"),
            "timestamp": chrono::Utc::now().to_rfc3339(),
        }))
    )
}

pub async fn readiness_check(
    State(state): State<Arc<AppState>>,
) -> (StatusCode, Json<serde_json::Value>) {
    // Check database connectivity
    let db_healthy = state.storage.health_check().await.is_ok();
    
    if db_healthy {
        (
            StatusCode::OK,
            Json(json!({
                "status": "ready",
                "database": "connected",
            }))
        )
    } else {
        (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(json!({
                "status": "not ready",
                "database": "disconnected",
            }))
        )
    }
}
```

---

## Security Best Practices

### 1. Use TLS/SSL

```yaml
# nginx.conf

server {
    listen 443 ssl http2;
    server_name lightrag.example.com;
    
    ssl_certificate /etc/nginx/ssl/cert.pem;
    ssl_certificate_key /etc/nginx/ssl/key.pem;
    ssl_protocols TLSv1.2 TLSv1.3;
    ssl_ciphers HIGH:!aNULL:!MD5;
    
    location / {
        proxy_pass http://lightrag:8080;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;
    }
}
```

### 2. Rate Limiting

```rust
use tower_governor::{governor::GovernorConfigBuilder, GovernorLayer};

let governor_conf = Box::new(
    GovernorConfigBuilder::default()
        .per_second(10)
        .burst_size(20)
        .finish()
        .unwrap()
);

let app = Router::new()
    .route("/query", post(query_handler))
    .layer(GovernorLayer {
        config: Box::leak(governor_conf),
    });
```

### 3. API Key Authentication

```rust
use axum::middleware;

async fn auth_middleware(
    headers: HeaderMap,
    request: Request<Body>,
    next: Next<Body>,
) -> Result<Response, StatusCode> {
    let api_key = headers
        .get("X-API-Key")
        .and_then(|v| v.to_str().ok())
        .ok_or(StatusCode::UNAUTHORIZED)?;
    
    if !validate_api_key(api_key).await {
        return Err(StatusCode::UNAUTHORIZED);
    }
    
    Ok(next.run(request).await)
}

let app = Router::new()
    .route("/query", post(query_handler))
    .layer(middleware::from_fn(auth_middleware));
```

---

## Backup and Recovery

### Database Backup

```bash
# SurrealDB export
surreal export --conn ws://localhost:8000 \
    --user root --pass root \
    --ns lightrag --db main \
    backup.surql

# PostgreSQL backup
pg_dump -U lightrag -d lightrag -F c -f backup.dump

# Automated backup script
#!/bin/bash
DATE=$(date +%Y%m%d_%H%M%S)
BACKUP_DIR="/backups"

# Backup SurrealDB
surreal export --conn ws://surrealdb:8000 \
    --user $SURREAL_USER --pass $SURREAL_PASS \
    --ns lightrag --db main \
    $BACKUP_DIR/lightrag_$DATE.surql

# Upload to S3
aws s3 cp $BACKUP_DIR/lightrag_$DATE.surql \
    s3://lightrag-backups/

# Cleanup old backups (keep last 7 days)
find $BACKUP_DIR -name "lightrag_*.surql" -mtime +7 -delete
```

### Restore

```bash
# SurrealDB import
surreal import --conn ws://localhost:8000 \
    --user root --pass root \
    --ns lightrag --db main \
    backup.surql

# PostgreSQL restore
pg_restore -U lightrag -d lightrag backup.dump
```

---

## Deployment Checklist

### Pre-Deployment

- [ ] Run all tests (`cargo test`)
- [ ] Check code quality (`cargo clippy`)
- [ ] Build production image
- [ ] Tag image with version
- [ ] Push to container registry
- [ ] Update configuration
- [ ] Backup production database
- [ ] Review resource limits
- [ ] Test health checks

### Deployment

- [ ] Deploy to staging environment
- [ ] Run smoke tests
- [ ] Monitor metrics and logs
- [ ] Deploy to production (canary/blue-green)
- [ ] Verify health checks
- [ ] Monitor error rates
- [ ] Check performance metrics

### Post-Deployment

- [ ] Verify all services healthy
- [ ] Check database connectivity
- [ ] Test critical user flows
- [ ] Monitor for 24 hours
- [ ] Update documentation
- [ ] Notify team of deployment

---

## Conclusion

This deployment guide provides comprehensive strategies for deploying EdgeQuake in various environments. By following these patterns, you can ensure reliable, scalable, and secure deployments.

**Key Takeaways**:

1. Use multi-stage Docker builds for optimal image size
2. Orchestrate with Docker Compose (dev) or Kubernetes (prod)
3. Implement comprehensive health checks
4. Enable monitoring and observability
5. Secure with TLS, rate limiting, and authentication
6. Automate backups and disaster recovery

**Next Steps**:

- Set up CI/CD pipeline
- Configure monitoring dashboards
- Implement automated backups
- Establish runbook for operations

---

**Status**: ✅ COMPLETE - Deployment guide ready for production use
