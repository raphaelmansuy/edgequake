---
title: "Deployment guide"
description: "Deploy EdgeQuake to production: choose a topology, prepare PostgreSQL, run migrate, start the API, and put a proxy in front. Covers Docker Compose, binary, Kubernetes and GCP."
---

# Deployment guide

This page is for operators who put EdgeQuake into production. It helps you choose a topology, then walks the same four steps for every one: prepare PostgreSQL, run `edgequake migrate`, start the API with safe settings, and check health. For a quick demo use the [Docker quickstart](docker-quickstart.md).

## The topology

Every deployment has the same parts. Only the packaging changes.

```mermaid
%%{init: {"theme":"base","themeVariables":{"primaryColor":"#E0E7FF","primaryBorderColor":"#6366F1","primaryTextColor":"#1E1B4B","secondaryColor":"#D1FAE5","secondaryBorderColor":"#10B981","secondaryTextColor":"#064E3B","tertiaryColor":"#FEF3C7","tertiaryBorderColor":"#F59E0B","tertiaryTextColor":"#6B7A90","lineColor":"#7A889C","clusterBkg":"rgba(99,102,241,0.07)","clusterBorder":"#A5B4FC","noteBkgColor":"#FEF9C3","noteTextColor":"#422006","textColor":"#6B7A90","titleColor":"#6B7A90","signalColor":"#7A889C","signalTextColor":"#6B7A90","loopTextColor":"#6B7A90","edgeLabelBackground":"#F1F5F9","actorLineColor":"#94A3B8"}}}%%
%% eq-theme:v1
flowchart LR
  U["Users"] --> P["Reverse proxy: TLS"]
  P --> W["Web UI: port 3000"]
  P --> A["API: port 8080"]
  W --> A
  A --> D[("PostgreSQL: pgvector + AGE")]
  A --> L["LLM provider"]
  M["migrate: one-shot"] --> D
%% eq-classes
classDef eqActor fill:#FCE7F3,stroke:#EC4899,color:#500724
classDef eqStore fill:#D1FAE5,stroke:#10B981,color:#064E3B
classDef eqLlm fill:#FEF3C7,stroke:#F59E0B,color:#451A03
class W eqActor
class D eqStore
class L eqLlm
```

How to read it: users reach the web UI and API through one proxy. The API stores everything in PostgreSQL and calls an LLM provider you choose. `migrate` is the only part that changes the schema, and it runs before the API serves traffic.

## Choose a deployment option

| Option | Cold start | Best for |
|--------|-----------|----------|
| `make stack` / quickstart file | about 30 s | Local use, demos. See [Docker quickstart](docker-quickstart.md). |
| Prebuilt Compose (`make docker-prebuilt`) | about 45 s | Staging and small production. See [Docker options](docker-deployment-options.md). |
| Source build (`make docker-up`) | 5 to 15 min | Custom builds. |
| Binary and PostgreSQL | n/a | Bare metal and VMs. |
| Kubernetes (Helm) | n/a | Scale and high availability. |
| GCP (`deploy/gcp`) | 5 to 10 min | The cheapest managed-VM setup. Cloud SQL and AlloyDB do not ship Apache AGE. |

## Prerequisites

- PostgreSQL 16 or newer (PG18 is the default image) with these extensions:

| Extension | Version |
|-----------|---------|
| `vector` (pgvector) | 0.8.5 |
| `age` (Apache AGE) | 1.6.0 on PG16, 1.7.0 on PG17, 1.8.0 on PG18 |

- Access to an LLM provider (see [Providers](../providers/index.md)).
- Recommended: 4 or more CPU cores, 8 GB RAM (16 GB for large corpora), SSD storage.

The pin matrix lives in [`edgequake/docker/extension-pins.sh`](../../edgequake/docker/extension-pins.sh). See [Release and CD](release-and-cd.md#postgresql-version-tiers).

## The boot sequence

Follow this order on every platform.

```mermaid
%%{init: {"theme":"base","themeVariables":{"primaryColor":"#E0E7FF","primaryBorderColor":"#6366F1","primaryTextColor":"#1E1B4B","secondaryColor":"#D1FAE5","secondaryBorderColor":"#10B981","secondaryTextColor":"#064E3B","tertiaryColor":"#FEF3C7","tertiaryBorderColor":"#F59E0B","tertiaryTextColor":"#6B7A90","lineColor":"#7A889C","clusterBkg":"rgba(99,102,241,0.07)","clusterBorder":"#A5B4FC","noteBkgColor":"#FEF9C3","noteTextColor":"#422006","textColor":"#6B7A90","titleColor":"#6B7A90","signalColor":"#7A889C","signalTextColor":"#6B7A90","loopTextColor":"#6B7A90","edgeLabelBackground":"#F1F5F9","actorLineColor":"#94A3B8"}}}%%
%% eq-theme:v1
sequenceDiagram
  participant Op as Operator
  participant Mig as migrate
  participant DB as PostgreSQL
  participant Api as API
  Op->>Mig: start
  Mig->>DB: apply safe schema
  Mig-->>Op: exit 0
  Op->>Api: start
  Api->>DB: check schema gate
  Api-->>Op: /live 200, then /ready 200
```

How to read it: time flows downward. With `EDGEQUAKE_SCHEMA_GATE=wait` the API may start at the same time as `migrate`. It answers `/live` and returns 503 on `/ready` until the schema is current. See [Upgrading](upgrading.md#4-what-happens-at-api-boot).

## Settings every production deployment needs

The API refuses to start with unsafe defaults. Set at least these:

| Variable | Why |
|----------|-----|
| `DATABASE_URL` | Required. There is no in-memory mode. |
| `EDGEQUAKE_AUTH_ENABLED=true`, `EDGEQUAKE_DEV_MODE=false` | Authentication on. |
| `JWT_SECRET` (32 or more bytes) | Fatal if default or short. |
| `EDGEQUAKE_CORS_ORIGINS` | Fatal if empty with a remote database. |
| `EDGEQUAKE_BOOTSTRAP_ADMIN_PASSWORD` | Creates the first admin. |
| `EDGEQUAKE_SECRETS_KEY` | Lets you store provider keys in the database. |
| Provider keys (for example `OPENAI_API_KEY`) | Or save them as Connections. |

Details: [Runtime auth hardening](runtime-auth-hardening.md), [Configuration](configuration.md), [Provider security](../providers/security.md). Run `edgequake doctor` to check them from a shell on the target host.

## Option 1: Binary and PostgreSQL

### Step 1: Build

```bash
cd edgequake
cargo build --release          # binary: target/release/edgequake
```

### Step 2: Install PostgreSQL and extensions

```bash
# Example: PG17 on macOS
brew install postgresql@17 && brew services start postgresql@17

git clone --branch v0.8.5 https://github.com/pgvector/pgvector.git
(cd pgvector && make && make install)

# AGE branch must match the PG major: PG16/v1.6.0-rc0, PG17/v1.7.0-rc0, PG18/v1.8.0-rc0
git clone --branch PG17/v1.7.0-rc0 https://github.com/apache/age.git
(cd age && make && make install)
```

### Step 3: Create the database

```sql
-- as superuser
CREATE USER edgequake WITH PASSWORD 'your_secure_password';
CREATE DATABASE edgequake OWNER edgequake;
\c edgequake
ALTER USER edgequake SET search_path TO public;
CREATE EXTENSION IF NOT EXISTS vector;
CREATE EXTENSION IF NOT EXISTS age;
CREATE EXTENSION IF NOT EXISTS pg_trgm;
```

`edgequake migrate` creates the tables and the AGE graph. Create the extensions yourself if the application user is not a superuser.

### Step 4: Migrate, then run

```bash
export DATABASE_URL="postgresql://edgequake:your_secure_password@localhost:5432/edgequake"
./target/release/edgequake migrate
./target/release/edgequake
```

### Step 5: systemd

Run migrate as a one-shot unit that runs before the API.

```ini
# /etc/systemd/system/edgequake-migrate.service
[Unit]
Description=EdgeQuake schema migrate
After=network.target postgresql.service
Requires=postgresql.service

[Service]
Type=oneshot
RemainAfterExit=yes
User=edgequake
EnvironmentFile=/etc/edgequake/edgequake.env
ExecStart=/opt/edgequake/edgequake migrate
```

```ini
# /etc/systemd/system/edgequake.service
[Unit]
Description=EdgeQuake API
After=edgequake-migrate.service
Requires=edgequake-migrate.service

[Service]
Type=simple
User=edgequake
WorkingDirectory=/opt/edgequake
EnvironmentFile=/etc/edgequake/edgequake.env
ExecStart=/opt/edgequake/edgequake
Restart=on-failure
RestartSec=5

[Install]
WantedBy=multi-user.target
```

Put `DATABASE_URL`, `JWT_SECRET`, `EDGEQUAKE_CORS_ORIGINS`, provider keys and the rest in `/etc/edgequake/edgequake.env` (mode 600). The server reads `HOST` and `PORT` for its bind address (defaults `0.0.0.0` and `8080`).

```bash
sudo systemctl daemon-reload
sudo systemctl enable --now edgequake
```

## Option 2: Docker Compose

Pick a file from [Docker deployment options](docker-deployment-options.md). The prebuilt file, started from `edgequake/docker`, is a good production base:

```bash
cd edgequake/docker
cp .env.example .env     # edit passwords and provider settings
docker compose -f docker-compose.prebuilt.yml up -d
docker compose -f docker-compose.prebuilt.yml ps
curl http://localhost:8080/ready
```

Remember the override file for auth settings described in [Docker deployment options](docker-deployment-options.md#required-before-the-api-starts). Change `POSTGRES_PASSWORD` from its default and set `EDGEQUAKE_VERSION`.

## Option 3: Kubernetes (Helm)

EdgeQuake ships Helm charts with an optional in-cluster Langfuse v4. Start with the operator guide: [deploy/kubernetes/README.md](../../deploy/kubernetes/README.md). The spec pack is [specs/138-kubernetes](../../specs/138-kubernetes/README.md).

```bash
make k8s-prereqs     # cert-manager, ClickHouse operator, nginx ingress
make k8s-kind-up     # local kind cluster
make k8s-install     # Langfuse, then EdgeQuake (includes the migrate Job)
make k8s-status
```

What the chart does:

| Topic | Behavior |
|-------|----------|
| Migration | A `migrate` Job runs before the API serves. It is a `pre-install,pre-upgrade` hook with an external database, and a normal Job named `edgequake-migrate-r<revision>` with the bundled PostgreSQL. |
| Schema gate | The API runs with `EDGEQUAKE_SCHEMA_GATE=wait`. |
| Probes | Startup and liveness use `/live`. Readiness uses `/ready`. |
| Shutdown | A `preStop` hook runs `edgequake pre-stop <seconds>` (default 15). |
| Auth defaults | `EDGEQUAKE_DEV_MODE=false` and `EDGEQUAKE_AUTH_ENABLED=true`. Supply `JWT_SECRET` through `api.extraSecretEnv` and `EDGEQUAKE_CORS_ORIGINS` through `api.env`, or the API exits at startup. |
| Kind profile | `values-kind.yaml` turns on dev mode and the mock LLM. Do not use it in production. |
| Ingress | Annotations turn off proxy buffering for streaming. |

Copy [`values-production.yaml.example`](../../deploy/kubernetes/helm/edgequake/values-production.yaml.example) as your starting point. Charts: `deploy/kubernetes/helm/edgequake/` (app) and `edgequake-stack/` (wrapper). Self-hosted Langfuse 3.1.x needs no OTLP: see [Langfuse 3.1.x](langfuse-3.1.md).

Probe excerpt for hand-written manifests:

```yaml
startupProbe:   { httpGet: { path: /live,  port: 8080 }, periodSeconds: 5, failureThreshold: 60 }
livenessProbe:  { httpGet: { path: /live,  port: 8080 }, periodSeconds: 30 }
readinessProbe: { httpGet: { path: /ready, port: 8080 }, periodSeconds: 10 }
env:
  - { name: EDGEQUAKE_SCHEMA_GATE, value: wait }
```

Run `edgequake migrate` as a Kubernetes Job before you roll the API Deployment.

## Option 4: GCP (SPEC-148)

A self-managed GCE VM running Compose, because Cloud SQL and AlloyDB do not include Apache AGE. HTTP always redirects to HTTPS. Read [deploy/gcp/README.md](../../deploy/gcp/README.md) and [specs/148-gcloud-hosting](../../specs/148-gcloud-hosting/README.md).

```bash
make spec148-gcp-plan     # terraform plan only; apply is gated
```

## Multi-replica task delivery (SPEC-057)

When more than one API process shares PostgreSQL, tasks must be claimed through leases.

| Variable | Default | Notes |
|----------|---------|-------|
| `EDGEQUAKE_REPLICAS` | `1` | Set to the replica count. |
| `EDGEQUAKE_TASK_DELIVERY` | `local` | Must be `bridged` or `notify_only` when replicas is above 1. |
| `EDGEQUAKE_TASK_LEASE_TTL_SECS` | `120` | Minimum 30. The worker renews the lease every third of the TTL (at least 5 s). |

Boot fails when `EDGEQUAKE_REPLICAS` is above 1 and delivery is `local`. Bridged and notify-only are wake signals. Work is always claimed with `claim_next` and a lease. See [Ingestion cancel and fairness](../ingestion-cancel-and-fairness.md).

## Health checks

| Endpoint | Meaning | Use it for |
|----------|---------|-----------|
| `GET /live` | The process is up. | Liveness probe, Docker healthcheck. |
| `GET /ready` | The API can take traffic. 200 or 503 with blockers. | Readiness probe, load balancer. |
| `GET /health` | Always HTTP 200. Reports `healthy` or `degraded` with components, schema and `security_posture`. | Dashboards and humans. |

The API image is distroless (no `curl`, `wget` or `sh`). Its healthcheck is the binary itself: `/usr/local/bin/edgequake healthcheck` (it calls `/live`). More in [Monitoring](monitoring.md).

## Reverse proxy

Streaming endpoints use server-sent events (SSE): `/api/v1/query/stream`, `/api/v1/chat/completions/stream` and `/api/v1/graph/stream`. The proxy must not buffer or gzip them. The API sends `X-Accel-Buffering: no` on SSE responses.

### Nginx

```nginx
upstream edgequake { server localhost:8080; keepalive 32; }

server {
    listen 443 ssl http2;
    server_name rag.yourdomain.com;
    ssl_certificate     /etc/ssl/certs/your-cert.pem;
    ssl_certificate_key /etc/ssl/private/your-key.pem;

    location / {
        proxy_pass http://edgequake;
        proxy_http_version 1.1;
        proxy_set_header Upgrade $http_upgrade;
        proxy_set_header Connection "upgrade";
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;
        proxy_buffering off;      # SSE
        proxy_cache off;
        proxy_read_timeout 86400;
        gzip off;                 # gzip buffers text/event-stream
    }
}
```

### Caddy

```caddy
rag.yourdomain.com {
    reverse_proxy localhost:8080 {
        header_up X-Real-IP {remote_host}
        flush_interval -1
    }
}
```

Do not use `encode gzip` on the SSE paths.

### Traefik and Kubernetes ingress

nginx ingress buffers by default. Set these annotations (the Helm chart already does):

```yaml
nginx.ingress.kubernetes.io/proxy-buffering: "off"
nginx.ingress.kubernetes.io/proxy-read-timeout: "86400"
```

Exclude `text/event-stream` from the Traefik `compress` middleware.

## Security checklist

- [ ] Strong PostgreSQL password (not `edgequake_secret`).
- [ ] `EDGEQUAKE_DEV_MODE=false`, `JWT_SECRET`, `EDGEQUAKE_CORS_ORIGINS`, bootstrap admin set.
- [ ] `ALLOW_REGISTRATION=false` and `EDGEQUAKE_RATE_LIMIT_ENABLED=true`.
- [ ] `EDGEQUAKE_SECRETS_KEY` set and backed up. Losing it makes stored provider keys unreadable.
- [ ] Keys in a secrets manager, not in image or Git.
- [ ] TLS at the proxy. Expose only 443. Do not publish PostgreSQL.
- [ ] `EDGEQUAKE_STRICT_STARTUP=1` once warnings are clean.
- [ ] Database backups (`pg_dump -Fc` or snapshots) and a tested restore.
- [ ] `EDGEQUAKE_TASK_DELIVERY=bridged` when replicas is above 1.

## See also

- [Configuration](configuration.md) and [env reference](env-reference.md)
- [Monitoring](monitoring.md)
- [Upgrading](upgrading.md)
- [Getting started](../getting-started/index.md)
- [Architecture overview](../architecture/overview.md)
