---
title: "Deployment guide"
description: "Deploy EdgeQuake to production: choose a topology, prepare PostgreSQL, run migrate, start the API, and put a proxy in front. Covers Docker Compose, binary, Kubernetes and GCP."
---

# Deployment guide

This guide is for operators who run EdgeQuake in production. It helps you choose a topology, then follows the same four steps for each one: prepare PostgreSQL, run `edgequake migrate`, start the API with safe settings, and check health. For a quick demo, use the [Docker quickstart](docker-quickstart.md) instead.

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

Users reach the web UI and the API through one proxy. The API stores everything in PostgreSQL and calls the LLM provider you choose. Notice that `migrate` is the only part that changes the schema, and it runs before the API serves traffic.

## Choose a deployment option

| Option | Best for | Guide |
|--------|----------|-------|
| Quickstart file (`make stack`) | Local use and demos | [Docker quickstart](docker-quickstart.md) |
| Prebuilt Compose (`make docker-prebuilt`) | Staging and small production | [Docker options](docker-deployment-options.md) |
| Source build (`make docker-up`) | Custom builds | [Docker options](docker-deployment-options.md) |
| Binary and PostgreSQL | Bare metal and VMs | [Option 1](#option-1-binary-and-postgresql) |
| Kubernetes (Helm) | Scale and high availability | [Option 3](#option-3-kubernetes-helm) |
| GCP (`deploy/gcp`) | A self-managed VM running Compose | [Option 4](#option-4-gcp-spec-148) |

## Prerequisites

- PostgreSQL 16 or newer. The default image uses PostgreSQL 18. Install these extensions:

| Extension | Version |
|-----------|---------|
| `vector` (pgvector) | 0.8.5 |
| `age` (Apache AGE) | 1.6.0 on PG16, 1.7.0 on PG17, 1.8.0 on PG18 |

- Access to an LLM provider (see [Providers](../providers/index.md)).
- A suggested starting size is 4 or more CPU cores, 8 GB RAM (16 GB for large corpora) and SSD storage. Adjust it to your load.

The version pins live in [`edgequake/docker/extension-pins.sh`](../../edgequake/docker/extension-pins.sh). See [Release and CD](release-and-cd.md#postgresql-version-tiers).

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
  Mig->>DB: apply schema
  Mig-->>Op: exit 0
  Op->>Api: start
  Api->>DB: check schema gate
  Api-->>Op: /live 200, then /ready 200
```

With `EDGEQUAKE_SCHEMA_GATE=wait`, the API can start at the same time as `migrate`. Until the schema is current, it answers `/live` and returns 503 on `/ready`. See [Upgrading](upgrading.md#4-what-happens-at-api-boot).

## Settings every production deployment needs

The API refuses to start with unsafe defaults. Set at least these variables:

| Variable | Why |
|----------|-----|
| `DATABASE_URL` | Required. There is no in-memory mode. |
| `EDGEQUAKE_AUTH_ENABLED=true`, `EDGEQUAKE_DEV_MODE=false` | Turns authentication on. |
| `JWT_SECRET` (32 or more bytes) | The API exits if it is the default or shorter than 32 bytes. |
| `EDGEQUAKE_CORS_ORIGINS` | The API exits if it is empty with a remote database. |
| `EDGEQUAKE_BOOTSTRAP_ADMIN_PASSWORD` | Creates the first admin. |
| `EDGEQUAKE_SECRETS_KEY` | Lets you store provider keys in the database. |
| Provider keys (for example `OPENAI_API_KEY`) | Or save them as Connections in the UI. |

Details: [Runtime auth hardening](runtime-auth-hardening.md), [Configuration](configuration.md), [Provider security](../providers/security.md). Run `edgequake doctor` on the target host to check these settings. Add `--json` for machine-readable output.

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

# The AGE branch must match the PostgreSQL major:
# PG16 uses PG16/v1.6.0-rc0, PG17 uses PG17/v1.7.0-rc0, PG18 uses PG18/v1.8.0-rc0
git clone --branch PG17/v1.7.0-rc0 https://github.com/apache/age.git
(cd age && make && make install)
```

### Step 3: Create the database

```sql
-- As a superuser
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

Run `migrate` as a one-shot unit that finishes before the API starts.

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

Put `DATABASE_URL`, `JWT_SECRET`, `EDGEQUAKE_CORS_ORIGINS`, provider keys and the other settings in `/etc/edgequake/edgequake.env`, with mode 600. The server reads `HOST` and `PORT` for its bind address. The defaults are `0.0.0.0` and `8080`.

```bash
sudo systemctl daemon-reload
sudo systemctl enable --now edgequake
```

## Option 2: Docker Compose

Pick a file from [Docker deployment options](docker-deployment-options.md). The prebuilt file is the starting point for a production Compose file:

```bash
cd edgequake/docker
cp .env.example .env     # edit passwords and provider settings
docker compose -f docker-compose.prebuilt.yml up -d
docker compose -f docker-compose.prebuilt.yml ps
curl http://localhost:8080/ready
```

Before you expose the stack, add the auth settings in an override file, as described in [Docker deployment options](docker-deployment-options.md#required-before-the-api-starts). Change `POSTGRES_PASSWORD` from its default and set `EDGEQUAKE_VERSION` to a release.

## Option 3: Kubernetes (Helm)

EdgeQuake ships Helm charts, with an optional in-cluster Langfuse v4. Start with the operator guide: [deploy/kubernetes/README.md](../../deploy/kubernetes/README.md). The spec pack is [specs/138-kubernetes](../../specs/138-kubernetes/README.md).

```bash
make k8s-prereqs     # cert-manager, ClickHouse operator, nginx ingress
make k8s-kind-up     # local kind cluster
make k8s-install     # Langfuse, then EdgeQuake (includes the migrate Job)
make k8s-status
```

What the chart does:

| Topic | Behavior |
|-------|----------|
| Migration | A `migrate` Job runs before the API serves. It is a `pre-install,pre-upgrade` hook with an external database. With the bundled PostgreSQL, it is a normal Job named `edgequake-migrate-r<revision>`. |
| Schema gate | The API runs with `EDGEQUAKE_SCHEMA_GATE=wait`. |
| Probes | Startup and liveness use `/live`. Readiness uses `/ready`. |
| Shutdown | A `preStop` hook runs `edgequake pre-stop <seconds>` (default 15). |
| Auth defaults | `EDGEQUAKE_DEV_MODE=false` and `EDGEQUAKE_AUTH_ENABLED=true`. Supply `JWT_SECRET` through `api.extraSecretEnv` and `EDGEQUAKE_CORS_ORIGINS` through `api.env`, or the API exits at startup. |
| Kind profile | `values-kind.yaml` turns on dev mode and the mock LLM. Do not use it in production. |
| Ingress | Annotations turn off proxy buffering for streaming. |

Copy [`values-production.yaml.example`](../../deploy/kubernetes/helm/edgequake/values-production.yaml.example) as your starting point. Charts: `deploy/kubernetes/helm/edgequake/` (the app) and `edgequake-stack/` (the wrapper). For self-hosted Langfuse 3.1.x, no OTLP setup is needed. See [Langfuse 3.1.x](langfuse-3.1.md).

Probe excerpt for hand-written manifests:

```yaml
startupProbe:   { httpGet: { path: /live,  port: 8080 }, periodSeconds: 5, failureThreshold: 60 }
livenessProbe:  { httpGet: { path: /live,  port: 8080 }, periodSeconds: 30 }
readinessProbe: { httpGet: { path: /ready, port: 8080 }, periodSeconds: 10 }
env:
  - { name: EDGEQUAKE_SCHEMA_GATE, value: wait }
```

Run `edgequake migrate` as a Kubernetes Job before you roll out the API Deployment.

## Option 4: GCP (SPEC-148)

The reference setup runs Docker Compose on a self-managed GCE VM. EdgeQuake needs Apache AGE, so this guide does not use Cloud SQL or AlloyDB. HTTP always redirects to HTTPS. Read [deploy/gcp/README.md](../../deploy/gcp/README.md) and [specs/148-gcloud-hosting](../../specs/148-gcloud-hosting/README.md).

```bash
make spec148-gcp-plan     # terraform plan only; apply is gated
```

## Multi-replica task delivery (SPEC-057)

When more than one API process shares PostgreSQL, tasks must be claimed through leases.

| Variable | Default | Notes |
|----------|---------|-------|
| `EDGEQUAKE_REPLICAS` | `1` | Set this to the replica count. |
| `EDGEQUAKE_TASK_DELIVERY` | `local` | Must be `bridged` or `notify_only` when replicas is above 1. |
| `EDGEQUAKE_TASK_LEASE_TTL_SECS` | `120` | Minimum 30. The worker renews the lease every third of the TTL, and at least every 5 s. |

Boot fails when `EDGEQUAKE_REPLICAS` is above 1 and delivery is `local`. Bridged and notify-only modes are wake signals only. Work is always claimed with `claim_next` and a lease. See [Ingestion cancel and fairness](../ingestion-cancel-and-fairness.md).

## Health checks

| Endpoint | Meaning | Use it for |
|----------|---------|-----------|
| `GET /live` | The process is up. | Liveness probe, Docker healthcheck. |
| `GET /ready` | The API can take traffic. Returns 200, or 503 with blockers. | Readiness probe, load balancer. |
| `GET /health` | Always HTTP 200. Reports `healthy` or `degraded`, with components, schema and `security_posture`. | Dashboards and people. |

The API image is distroless, so it has no `curl`, `wget` or `sh`. Its healthcheck is the binary itself: `/usr/local/bin/edgequake healthcheck` calls `/live`. See [Monitoring](monitoring.md) for more.

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

nginx ingress buffers responses by default. Set these annotations (the Helm chart already does):

```yaml
nginx.ingress.kubernetes.io/proxy-buffering: "off"
nginx.ingress.kubernetes.io/proxy-read-timeout: "86400"
```

Exclude `text/event-stream` from the Traefik `compress` middleware.

## Security checklist

- [ ] Strong PostgreSQL password (not `edgequake_secret`).
- [ ] `EDGEQUAKE_DEV_MODE=false`, `JWT_SECRET`, `EDGEQUAKE_CORS_ORIGINS` and the bootstrap admin password are set.
- [ ] `ALLOW_REGISTRATION=false` and `EDGEQUAKE_RATE_LIMIT_ENABLED=true`.
- [ ] `EDGEQUAKE_SECRETS_KEY` is set and backed up. Losing it makes stored provider keys unreadable.
- [ ] Keys are in a secrets manager, not in an image or in Git.
- [ ] TLS is terminated at the proxy. Only port 443 is public. PostgreSQL is not published.
- [ ] `EDGEQUAKE_STRICT_STARTUP=1` is set once the startup warnings are clean.
- [ ] Database backups (`pg_dump -Fc` or snapshots) exist, and a restore was tested.
- [ ] `EDGEQUAKE_TASK_DELIVERY=bridged` is set when replicas is above 1.

## See also

- [Configuration](configuration.md) and the [env reference](env-reference.md)
- [Monitoring](monitoring.md)
- [Upgrading](upgrading.md)
- [Getting started](../getting-started/index.md)
- [Architecture overview](../architecture/overview.md)
