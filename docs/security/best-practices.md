# Security Best Practices

> **Securing Your EdgeQuake Deployment**

This guide covers security considerations for production EdgeQuake deployments.

---

## Overview

```
┌─────────────────────────────────────────────────────────────────┐
│                    SECURITY LAYERS                              │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  ┌──────────────────────────────────────────────────────────┐   │
│  │ NETWORK LAYER                                            │   │
│  │ • TLS termination (reverse proxy)                        │   │
│  │ • IP allowlisting                                        │   │
│  │ • DDoS protection                                        │   │
│  └──────────────────────────────────────────────────────────┘   │
│                            ↓                                    │
│  ┌──────────────────────────────────────────────────────────┐   │
│  │ APPLICATION LAYER                                        │   │
│  │ • API key authentication                                 │   │
│  │ • JWT token validation                                   │   │
│  │ • Rate limiting                                          │   │
│  │ • Request validation                                     │   │
│  └──────────────────────────────────────────────────────────┘   │
│                            ↓                                    │
│  ┌──────────────────────────────────────────────────────────┐   │
│  │ DATA LAYER                                               │   │
│  │ • Tenant isolation                                       │   │
│  │ • Workspace boundaries                                   │   │
│  │ • Database encryption                                    │   │
│  │ • Secret management                                      │   │
│  └──────────────────────────────────────────────────────────┘   │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

---

## Network Security

### TLS Configuration

**Always use HTTPS in production.** EdgeQuake doesn't handle TLS directly; use a reverse proxy.

**Caddy (Recommended)**:

```caddyfile
edgequake.example.com {
    reverse_proxy localhost:8080
    # Automatic TLS via Let's Encrypt
}
```

**nginx**:

```nginx
server {
    listen 443 ssl http2;
    server_name edgequake.example.com;

    ssl_certificate /etc/letsencrypt/live/edgequake.example.com/fullchain.pem;
    ssl_certificate_key /etc/letsencrypt/live/edgequake.example.com/privkey.pem;

    # Modern TLS settings
    ssl_protocols TLSv1.3 TLSv1.2;
    ssl_ciphers ECDHE-ECDSA-AES128-GCM-SHA256:ECDHE-RSA-AES128-GCM-SHA256;
    ssl_prefer_server_ciphers on;

    # Security headers
    add_header Strict-Transport-Security "max-age=31536000; includeSubDomains" always;
    add_header X-Content-Type-Options nosniff;
    add_header X-Frame-Options DENY;

    location / {
        proxy_pass http://127.0.0.1:8080;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;
    }
}
```

### IP Allowlisting

Restrict access to trusted networks:

```nginx
# nginx: Allow only trusted IPs
location / {
    allow 10.0.0.0/8;      # Internal network
    allow 192.168.0.0/16;  # VPN range
    deny all;

    proxy_pass http://127.0.0.1:8080;
}
```

### Firewall Rules

```bash
# iptables: Only allow HTTP/HTTPS from load balancer
iptables -A INPUT -p tcp --dport 8080 -s 10.0.1.10 -j ACCEPT  # LB IP
iptables -A INPUT -p tcp --dport 8080 -j DROP

# PostgreSQL: Only from app servers
iptables -A INPUT -p tcp --dport 5432 -s 10.0.0.0/24 -j ACCEPT
iptables -A INPUT -p tcp --dport 5432 -j DROP
```

---

## Authentication

### API Key Authentication

EdgeQuake supports API key authentication via headers:

```bash
# Via X-API-Key header
curl -H "X-API-Key: your-secret-key" http://localhost:8080/api/v1/documents

# Via Authorization Bearer
curl -H "Authorization: Bearer your-secret-key" http://localhost:8080/api/v1/documents
```

**API Key Best Practices**:

| Practice     | Recommendation                         |
| ------------ | -------------------------------------- |
| Key length   | Minimum 32 characters                  |
| Key rotation | Every 90 days                          |
| Scope        | Per-tenant or per-workspace            |
| Storage      | Environment variable or secret manager |
| Logging      | Never log full keys                    |

### External Authentication Proxy

For production, use an authentication proxy:

**OAuth2 Proxy (for SSO)**:

```yaml
# docker-compose.yml
oauth2-proxy:
  image: quay.io/oauth2-proxy/oauth2-proxy
  environment:
    OAUTH2_PROXY_PROVIDER: oidc
    OAUTH2_PROXY_OIDC_ISSUER_URL: https://auth.example.com
    OAUTH2_PROXY_CLIENT_ID: edgequake
    OAUTH2_PROXY_CLIENT_SECRET: ${OAUTH_SECRET}
    OAUTH2_PROXY_COOKIE_SECRET: ${COOKIE_SECRET}
    OAUTH2_PROXY_UPSTREAMS: http://edgequake:8080
  ports:
    - "4180:4180"
```

---

## Authorization

### Multi-Tenant Isolation

EdgeQuake enforces strict tenant boundaries:

```
┌─────────────────────────────────────────────────────────────────┐
│                    TENANT ISOLATION                             │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  ┌───────────────────┐       ┌───────────────────┐              │
│  │    Tenant A       │       │    Tenant B       │              │
│  │ ┌───────────────┐ │       │ ┌───────────────┐ │              │
│  │ │ Workspace 1   │ │       │ │ Workspace 3   │ │              │
│  │ │ - Documents   │ │       │ │ - Documents   │ │              │
│  │ │ - Entities    │ │       │ │ - Entities    │ │              │
│  │ │ - Embeddings  │ │       │ │ - Embeddings  │ │              │
│  │ └───────────────┘ │       │ └───────────────┘ │              │
│  │ ┌───────────────┐ │       │ ┌───────────────┐ │              │
│  │ │ Workspace 2   │ │       │ │ Workspace 4   │ │              │
│  │ │ - Documents   │ │       │ │ - Documents   │ │              │
│  │ │ - Entities    │ │       │ │ - Entities    │ │              │
│  │ │ - Embeddings  │ │       │ │ - Embeddings  │ │              │
│  │ └───────────────┘ │       │ └───────────────┘ │              │
│  └───────────────────┘       └───────────────────┘              │
│           ╲                           ╱                         │
│            ╲   NO DATA SHARING       ╱                          │
│             ╲─────────────────────────                          │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

**Enforcement**:

- All queries include `workspace_id` filter
- All data includes `tenant_id` column
- Cross-tenant access denied at database level

### Role-Based Access (Future)

Planned RBAC roles:

| Role     | Permissions         |
| -------- | ------------------- |
| `admin`  | All operations      |
| `editor` | Upload, query, view |
| `viewer` | Query, view only    |
| `api`    | Programmatic access |

---

## Data Security

### Data at Rest

**PostgreSQL Encryption**:

```sql
-- Enable TDE (Transparent Data Encryption)
-- Requires PostgreSQL Enterprise or managed service

-- For community PostgreSQL, use filesystem encryption:
-- Linux: LUKS, dm-crypt
-- AWS: Encrypted EBS volumes
-- GCP: Default encryption enabled
```

**Environment Variables**:

```bash
# Use encrypted secrets
export OPENAI_API_KEY="$(vault read -field=key secret/openai)"
export DATABASE_URL="$(vault read -field=url secret/database)"
```

### Data in Transit

| Connection             | Encryption        |
| ---------------------- | ----------------- |
| Client → EdgeQuake     | HTTPS (via proxy) |
| EdgeQuake → PostgreSQL | SSL/TLS           |
| EdgeQuake → OpenAI     | HTTPS             |
| EdgeQuake → Ollama     | HTTP (local only) |

**PostgreSQL SSL**:

```bash
# Connection string with SSL
DATABASE_URL="postgresql://user:pass@host:5432/db?sslmode=require"

# With certificate verification
DATABASE_URL="postgresql://user:pass@host:5432/db?sslmode=verify-full&sslrootcert=/path/to/ca.crt"
```

### Secret Management

**Never commit secrets to Git.**

| Secret           | Storage Recommendation     |
| ---------------- | -------------------------- |
| `OPENAI_API_KEY` | Vault, AWS Secrets Manager |
| `DATABASE_URL`   | Vault, Kubernetes Secret   |
| API keys         | Database (hashed)          |
| JWT signing key  | Vault, environment         |

**HashiCorp Vault Example**:

```bash
# Store secrets
vault kv put secret/edgequake \
  openai_key="sk-..." \
  database_url="postgresql://..."

# Retrieve in application
export OPENAI_API_KEY="$(vault kv get -field=openai_key secret/edgequake)"
```

**Kubernetes Secrets**:

```yaml
apiVersion: v1
kind: Secret
metadata:
  name: edgequake-secrets
type: Opaque
stringData:
  OPENAI_API_KEY: sk-your-key-here
  DATABASE_URL: postgresql://...
---
apiVersion: apps/v1
kind: Deployment
spec:
  template:
    spec:
      containers:
        - name: edgequake
          envFrom:
            - secretRef:
                name: edgequake-secrets
```

---

## Input Validation

### Request Validation

EdgeQuake validates all inputs:

| Field          | Validation                              |
| -------------- | --------------------------------------- |
| `workspace_id` | UUID format, exists                     |
| `document_id`  | UUID format, exists, owned by workspace |
| `query`        | Non-empty, max 10,000 chars             |
| `file`         | Size limit, MIME type check             |

### File Upload Security

```rust
// Implemented in EdgeQuake
const MAX_FILE_SIZE: usize = 50 * 1024 * 1024;  // 50 MB
const ALLOWED_TYPES: &[&str] = &["application/pdf", "text/plain", "text/markdown"];
```

**Additional Protections**:

- Content-type sniffing (actual vs declared)
- Filename sanitization
- Path traversal prevention
- Virus scanning (integrate ClamAV)

---

## Rate Limiting

EdgeQuake includes built-in rate limiting:

```
┌─────────────────────────────────────────────────────────────────┐
│                    RATE LIMITING                                │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  Request → [Per-IP Limiter] → [Per-Key Limiter] → Handler       │
│                  │                    │                         │
│              429 if                429 if                       │
│              exceeded              exceeded                     │
│                                                                 │
│  Default Limits:                                                │
│  ┌─────────────────────────────────────────────────────────┐    │
│  │ Endpoint Category  │ Requests │ Window │ Burst          │    │
│  ├─────────────────────────────────────────────────────────┤    │
│  │ Document upload    │ 10       │ 1 min  │ 3              │    │
│  │ Query              │ 60       │ 1 min  │ 10             │    │
│  │ Graph traversal    │ 100      │ 1 min  │ 20             │    │
│  │ Health checks      │ No limit │ -      │ -              │    │
│  └─────────────────────────────────────────────────────────┘    │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

**Custom Limits** (nginx):

```nginx
# Additional rate limiting at proxy
limit_req_zone $binary_remote_addr zone=api:10m rate=10r/s;
limit_req_zone $http_x_api_key zone=apikey:10m rate=100r/s;

location /api/ {
    limit_req zone=api burst=20 nodelay;
    limit_req zone=apikey burst=50 nodelay;
    proxy_pass http://127.0.0.1:8080;
}
```

---

## Logging & Auditing

### Security Logging

EdgeQuake logs security events:

| Event         | Log Level | Example                              |
| ------------- | --------- | ------------------------------------ |
| Auth success  | INFO      | `user=X authenticated`               |
| Auth failure  | WARN      | `invalid_api_key from IP`            |
| Rate limited  | WARN      | `rate_limit_exceeded user=X`         |
| Access denied | WARN      | `access_denied tenant=X workspace=Y` |
| Admin action  | INFO      | `workspace_deleted by user=X`        |

### Log Aggregation

```yaml
# Ship logs to centralized system
docker-compose.yml:
  edgequake:
    logging:
      driver: "json-file"
      options:
        max-size: "100m"
        max-file: "5"
```

**Recommended Stack**:

- Loki + Grafana (lightweight)
- ELK Stack (feature-rich)
- Datadog/Splunk (managed)

---

## LLM Security

### API Key Protection

```bash
# Don't pass keys in URLs
# BAD: curl "http://api.openai.com?api_key=sk-..."
# GOOD: curl -H "Authorization: Bearer sk-..." http://api.openai.com

# Rotate keys if compromised
# 1. Generate new key in OpenAI dashboard
# 2. Update environment variable
# 3. Revoke old key
```

### Prompt Injection Prevention

EdgeQuake mitigates prompt injection:

| Mitigation        | Implementation                      |
| ----------------- | ----------------------------------- |
| System prompt     | Separate from user input            |
| Context isolation | Retrieved docs in structured format |
| Output validation | Response format checking            |

**Example System Prompt**:

```
You are a helpful assistant answering questions about the provided documents.
Answer based ONLY on the context provided. If the answer is not in the context,
say "I don't have enough information to answer that question."

<context>
{retrieved_documents}
</context>

User question: {user_query}
```

### Data Leakage Prevention

```
┌─────────────────────────────────────────────────────────────────┐
│                 DATA FLOW TO LLM                                │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  Document Upload → Chunking → [PII Detection] → LLM             │
│                                      │                          │
│                               Redact if                         │
│                               configured                        │
│                                                                 │
│  Sensitive Data Handling:                                       │
│  • Never send passwords to LLM                                  │
│  • Optionally redact PII before processing                      │
│  • Use local LLM (Ollama) for sensitive data                    │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

---

## Production Hardening Checklist

### Pre-Deployment

- [ ] TLS enabled (HTTPS)
- [ ] Reverse proxy configured (nginx/Caddy)
- [ ] API keys rotated from defaults
- [ ] Database credentials secure
- [ ] Secrets in secret manager (not env files)
- [ ] Rate limiting configured
- [ ] Firewall rules applied
- [ ] Logging to centralized system

### Runtime

- [ ] Health checks monitored
- [ ] Error rates alerting configured
- [ ] Rate limit violations tracked
- [ ] Auth failure monitoring
- [ ] Database backups verified
- [ ] Log retention policy set

### Periodic

- [ ] API key rotation (90 days)
- [ ] Dependency updates (monthly)
- [ ] Security audit (quarterly)
- [ ] Penetration testing (annually)
- [ ] Incident response plan tested

---

## Security Incidents

### Response Procedure

1. **Detect**: Monitor for anomalies
2. **Contain**: Disable compromised credentials
3. **Investigate**: Review logs
4. **Remediate**: Patch vulnerabilities
5. **Communicate**: Notify affected parties
6. **Document**: Post-incident review

### Contact

For security vulnerabilities, contact: security@edgequake.dev

---

## See Also

- [Deployment Guide](../operations/deployment.md) - Production setup
- [Configuration Reference](../operations/configuration.md) - All settings
- [Monitoring Guide](../operations/monitoring.md) - Observability
