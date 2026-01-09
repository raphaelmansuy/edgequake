# EdgeQuake Multi-Tenancy Guide

> Namespace-based data isolation for multi-tenant deployments

**Version**: 2.0.0 | **Last Updated**: January 2026

> **Implements**: [FEAT0040](features.md#feat0040) Multi-Tenancy | [FEAT0041](features.md#feat0041) Namespace Isolation
> **Business Rules**: [BR0040](business_rules.md#br0040) Data Isolation | [BR0041](business_rules.md#br0041) Cross-Tenant Prevention
> **Use Cases**: [UC0010](use_cases.md#uc0010) Enterprise SaaS Deployment
> **Code Reference**: See [edgequake/examples/multi_tenant.rs](../edgequake/examples/multi_tenant.rs) for a working example

---

## Quick Reference

| I want to...                | Go to                                                   |
| --------------------------- | ------------------------------------------------------- |
| Understand isolation model  | [Namespace-Based Isolation](#namespace-based-isolation) |
| Implement in Rust code      | [Implementation](#implementation)                       |
| Use PostgreSQL multi-tenant | [PostgreSQL Multi-Tenancy](#postgresql-multi-tenancy)   |
| Set up via API              | [API Usage](#api-usage)                                 |
| Configure RBAC              | [RBAC and Permissions](#rbac-and-permissions)           |
| See security best practices | [Security Considerations](#security-considerations)     |

---

## Isolation Model Summary

| Isolation Type | Mechanism                         | Data Affected                              |
| -------------- | --------------------------------- | ------------------------------------------ |
| **Namespace**  | String prefix in all storage      | Documents, Chunks, Entities, Relationships |
| **PostgreSQL** | `namespace` column + WHERE clause | All tables include namespace filtering     |
| **API**        | `X-Workspace-ID` header           | Per-request tenant routing                 |
| **RBAC**       | Role + Permission matrix          | Action authorization                       |

### Security Guarantees

✅ **Complete data isolation** - No tenant can access another's data  
✅ **Query-level enforcement** - All queries include namespace filter  
✅ **API-level validation** - Workspace ID validated on every request  
✅ **No cross-tenant joins** - Graph queries scoped to namespace

---

## Table of Contents

1. [Overview](#overview)
2. [Namespace-Based Isolation](#namespace-based-isolation)
3. [Implementation](#implementation)
4. [PostgreSQL Multi-Tenancy](#postgresql-multi-tenancy)
5. [API Usage](#api-usage)
6. [RBAC and Permissions](#rbac-and-permissions)
7. [Security Considerations](#security-considerations)
8. [Best Practices](#best-practices)

---

## Overview

EdgeQuake provides namespace-based data isolation for multi-tenant deployments. Each tenant gets isolated storage through namespaced storage backends.

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                       NAMESPACE-BASED ISOLATION                              │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                              │
│  ┌──────────────────────────────────────────────────────────────────────┐   │
│  │                      EdgeQuake API Server                             │   │
│  └────────────────────────────┬─────────────────────────────────────────┘   │
│                               │                                              │
│       ┌───────────────────────┼───────────────────────┐                     │
│       │                       │                       │                     │
│       ▼                       ▼                       ▼                     │
│  ┌────────────┐         ┌────────────┐         ┌────────────┐              │
│  │ Namespace  │         │ Namespace  │         │ Namespace  │              │
│  │  tenant_a  │         │  tenant_b  │         │  tenant_c  │              │
│  ├────────────┤         ├────────────┤         ├────────────┤              │
│  │ KV Storage │         │ KV Storage │         │ KV Storage │              │
│  │ Vector DB  │         │ Vector DB  │         │ Vector DB  │              │
│  │ Graph DB   │         │ Graph DB   │         │ Graph DB   │              │
│  └────────────┘         └────────────┘         └────────────┘              │
│                                                                              │
└─────────────────────────────────────────────────────────────────────────────┘
```

### Key Concepts

| Concept             | Description                                                  |
| ------------------- | ------------------------------------------------------------ |
| **Namespace**       | String identifier that isolates data across storage backends |
| **Tenant**          | Logical organization with its own namespace                  |
| **Storage Backend** | KV, Vector, or Graph storage scoped to a namespace           |

---

## Namespace-Based Isolation

Each storage backend in EdgeQuake accepts a namespace parameter that isolates data:

### Storage Trait Namespaces

```rust
// All storage traits include namespace()
pub trait KVStorage: Send + Sync {
    fn namespace(&self) -> &str;
    // ... other methods
}

pub trait VectorStorage: Send + Sync {
    fn namespace(&self) -> &str;
    // ... other methods
}

pub trait GraphStorage: Send + Sync {
    fn namespace(&self) -> &str;
    // ... other methods
}
```

### Data Isolation

- Documents, entities, and relationships are scoped to namespace
- Vector embeddings are isolated per namespace
- Graph nodes and edges are namespace-specific
- No cross-namespace data leakage

---

## Implementation

### Creating Tenant-Isolated Storage

```rust
use edgequake_storage::adapters::memory::{
    MemoryKVStorage, MemoryVectorStorage, MemoryGraphStorage
};
use std::sync::Arc;

/// Create isolated storage for a tenant
fn create_tenant_storage(tenant_id: &str) -> (
    Arc<MemoryKVStorage>,
    Arc<MemoryVectorStorage>,
    Arc<MemoryGraphStorage>,
) {
    let namespace = format!("tenant_{}", tenant_id);

    (
        Arc::new(MemoryKVStorage::new(&namespace)),
        Arc::new(MemoryVectorStorage::new(&namespace, 1536)),
        Arc::new(MemoryGraphStorage::new(&namespace)),
    )
}
```

### TenantRAG Example

```rust
use std::sync::Arc;
use edgequake_llm::MockProvider;
use edgequake_storage::{MemoryKVStorage, MemoryVectorStorage, MemoryGraphStorage};

/// A tenant-isolated RAG instance.
struct TenantRAG {
    tenant_id: String,
    kv_storage: Arc<MemoryKVStorage>,
    vector_storage: Arc<MemoryVectorStorage>,
    graph_storage: Arc<MemoryGraphStorage>,
}

impl TenantRAG {
    /// Create a new tenant-isolated RAG instance.
    fn new(tenant_id: &str) -> Self {
        let namespace = format!("tenant_{}", tenant_id);

        Self {
            tenant_id: tenant_id.to_string(),
            kv_storage: Arc::new(MemoryKVStorage::new(&namespace)),
            vector_storage: Arc::new(MemoryVectorStorage::new(&namespace, 1536)),
            graph_storage: Arc::new(MemoryGraphStorage::new(&namespace)),
        }
    }

    /// Initialize storage backends.
    async fn initialize(&self) -> anyhow::Result<()> {
        self.kv_storage.initialize().await?;
        self.vector_storage.initialize().await?;
        self.graph_storage.initialize().await?;
        Ok(())
    }
}
```

### Multi-Tenant Manager

```rust
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Manages multiple tenant RAG instances.
struct TenantManager {
    tenants: RwLock<HashMap<String, Arc<TenantRAG>>>,
}

impl TenantManager {
    fn new() -> Self {
        Self {
            tenants: RwLock::new(HashMap::new()),
        }
    }

    /// Get or create a tenant instance.
    async fn get_tenant(&self, tenant_id: &str) -> Arc<TenantRAG> {
        // Check if already exists
        {
            let tenants = self.tenants.read().await;
            if let Some(tenant) = tenants.get(tenant_id) {
                return tenant.clone();
            }
        }

        // Create new tenant
        let tenant = Arc::new(TenantRAG::new(tenant_id));
        tenant.initialize().await.expect("Failed to initialize tenant");

        // Store
        {
            let mut tenants = self.tenants.write().await;
            tenants.insert(tenant_id.to_string(), tenant.clone());
        }

        tenant
    }
}
```

---

## PostgreSQL Multi-Tenancy

For production deployments, use PostgreSQL storage with namespace isolation:

### Namespace in PostgreSQL

> **Code Reference**: See [edgequake/crates/edgequake-storage/src/adapters/postgres/config.rs](../edgequake/crates/edgequake-storage/src/adapters/postgres/config.rs)

```rust
use edgequake_storage::{PostgresConfig, PostgresKVStorage, PgVectorStorage, PostgresAGEGraphStorage};

// Create tenant-specific config
let config = PostgresConfig {
    host: "localhost".to_string(),
    port: 5432,
    database: "edgequake".to_string(),
    user: "postgres".to_string(),
    password: "password".to_string(),
    namespace: format!("tenant_{}", tenant_id),  // Tenant-specific namespace
    ..Default::default()
};

// Each storage adapter uses the namespace from config
let kv_storage = Arc::new(PostgresKVStorage::new(config.clone()));
let vector_storage = Arc::new(PgVectorStorage::new(config.clone()));
let graph_storage = Arc::new(PostgresAGEGraphStorage::new(config));
```

### Database Schema Isolation

Data is isolated via namespace column in all tables:

```sql
-- All tables include namespace column
CREATE TABLE edgequake_documents (
    id UUID PRIMARY KEY,
    namespace VARCHAR(255) NOT NULL DEFAULT 'default',
    -- ... other columns
);

CREATE TABLE edgequake_chunks (
    id UUID PRIMARY KEY,
    namespace VARCHAR(255) NOT NULL DEFAULT 'default',
    -- ... other columns
);

-- Queries are always scoped by namespace
SELECT * FROM edgequake_documents WHERE namespace = 'tenant_abc';
```

---

## API Usage

### Workspace-Based Multi-Tenancy

The EdgeQuake API uses workspaces for multi-tenancy:

```bash
# Create workspace (tenant)
POST /api/v1/workspaces
{
    "name": "Acme Corp",
    "description": "Production workspace"
}

# All subsequent operations are scoped to workspace
POST /api/v1/workspaces/{workspace_id}/documents
GET /api/v1/workspaces/{workspace_id}/query
```

### Configuration

```bash
# Set default namespace
export EDGEQUAKE_NAMESPACE=production

# Or per-request via API
curl -X POST "http://localhost:8080/api/v1/query" \
  -H "X-Workspace-ID: tenant_abc" \
  -d '{"query": "What is EdgeQuake?"}'
```

---

## RBAC and Permissions

EdgeQuake implements a granular Role-Based Access Control (RBAC) system to manage access within and across tenants.

### Roles

| Role       | Description                                                 |
| :--------- | :---------------------------------------------------------- |
| `admin`    | Full system access, including tenant and user management.   |
| `user`     | Regular user with read/write access to documents and graph. |
| `readonly` | Read-only access to documents and graph.                    |

### Permissions

Permissions are grouped by resource type:

| Resource          | Permissions                                                                          |
| :---------------- | :----------------------------------------------------------------------------------- |
| **Documents**     | `DocumentRead`, `DocumentCreate`, `DocumentUpdate`, `DocumentDelete`                 |
| **Entities**      | `EntityRead`, `EntityCreate`, `EntityUpdate`, `EntityDelete`                         |
| **Relationships** | `RelationshipRead`, `RelationshipCreate`, `RelationshipUpdate`, `RelationshipDelete` |
| **Query**         | `QueryExecute`, `QueryAdvanced`                                                      |
| **Admin**         | `TenantManage`, `UserManage`, `SystemMaintenance`                                    |

---

## Security Considerations

### Threat Model

| Threat                     | Mitigation                                | Status              |
| -------------------------- | ----------------------------------------- | ------------------- |
| Cross-tenant data access   | All queries include `WHERE namespace = ?` | ✅ Enforced         |
| Tenant ID manipulation     | Validate workspace header server-side     | ✅ API validation   |
| SQL injection in namespace | Parameterized queries only                | ✅ No string concat |
| Privilege escalation       | RBAC enforcement on all endpoints         | ✅ Role checks      |
| Mass data export           | Rate limiting per tenant                  | ⚙️ Configurable     |

### Namespace Injection Prevention

```rust
// GOOD: Parameterized query
sqlx::query("SELECT * FROM documents WHERE namespace = $1")
    .bind(&namespace)
    .fetch_all(&pool)
    .await?;

// BAD: String concatenation (NEVER DO THIS)
// sqlx::query(&format!("SELECT * FROM documents WHERE namespace = '{}'", namespace))
```

### Audit Logging

For compliance, enable audit logging for cross-tenant operations:

```bash
# Enable audit logging
export EDGEQUAKE_AUDIT_LOG=true
export EDGEQUAKE_AUDIT_PATH=/var/log/edgequake/audit.log
```

```json
// Audit log entry format
{
  "timestamp": "2026-01-15T10:30:00Z",
  "action": "document.create",
  "tenant_id": "acme",
  "user_id": "user_123",
  "resource_id": "doc_abc",
  "ip_address": "192.168.1.100",
  "success": true
}
```

### Compliance Checklist

- [ ] **Data residency**: Tenant data stored in correct region
- [ ] **Encryption at rest**: All storage encrypted (PostgreSQL TDE)
- [ ] **Encryption in transit**: TLS 1.3 for all connections
- [ ] **Audit trail**: All operations logged with tenant context
- [ ] **Data retention**: Per-tenant retention policies
- [ ] **Right to deletion**: Tenant deletion cascades to all data

---

## Best Practices

### 1. Consistent Namespace Naming

```rust
// Good: Clear, consistent naming
let namespace = format!("tenant_{}", tenant_id);
let namespace = format!("org_{}", org_id);

// Bad: Inconsistent or unclear
let namespace = tenant_id.clone();  // No prefix
```

### 2. Namespace Validation

```rust
fn validate_namespace(namespace: &str) -> Result<(), Error> {
    if namespace.is_empty() {
        return Err(Error::InvalidNamespace("Empty namespace"));
    }
    if namespace.len() > 255 {
        return Err(Error::InvalidNamespace("Namespace too long"));
    }
    if !namespace.chars().all(|c| c.is_alphanumeric() || c == '_') {
        return Err(Error::InvalidNamespace("Invalid characters"));
    }
    Ok(())
}
```

### 3. Resource Cleanup

```rust
impl TenantManager {
    /// Clean up tenant resources on deletion.
    async fn delete_tenant(&self, tenant_id: &str) -> Result<()> {
        if let Some(tenant) = self.tenants.write().await.remove(tenant_id) {
            // Finalize storage
            tenant.kv_storage.finalize().await?;
            tenant.vector_storage.finalize().await?;
            tenant.graph_storage.finalize().await?;
        }
        Ok(())
    }
}
```

### 4. Connection Pooling

```rust
// Share connection pool across tenants
let pool = PostgresPool::connect(&config).await?;

// Each tenant uses same pool but different namespace
let tenant_a = PostgresKVStorage::new(pool.clone(), "tenant_a");
let tenant_b = PostgresKVStorage::new(pool.clone(), "tenant_b");
```

---

## Running the Example

```bash
# Run the multi-tenant example
cargo run --example multi_tenant

# Output:
# Tenant 'acme' storage initialized
# Tenant 'globex' storage initialized
# Tenant 'acme': Ingesting document 'doc1' (3 chunks)
# Tenant 'globex': Ingesting document 'doc2' (2 chunks)
# Tenant 'acme': Found 3 relevant chunks
```

---

## Troubleshooting

| Symptom                     | Likely Cause             | Solution                                  |
| --------------------------- | ------------------------ | ----------------------------------------- |
| Data visible across tenants | Missing namespace filter | Check all queries include `namespace = ?` |
| "Invalid namespace" error   | Special characters       | Use alphanumeric + underscore only        |
| Empty query results         | Wrong tenant context     | Verify `X-Workspace-ID` header            |
| Connection pool exhausted   | Too many tenants         | Share pool with namespace routing         |
| Slow tenant switching       | Creating new connections | Use connection pooling                    |

---

## Next Steps

| Document                                                   | When to Read                           |
| ---------------------------------------------------------- | -------------------------------------- |
| [Storage Backends](0004-storage-backends.md)               | Configure PostgreSQL for multi-tenancy |
| [Configuration Reference](0007-configuration-reference.md) | All namespace-related config options   |
| [Deployment Guide](0006-deployment-guide.md)               | Production deployment with isolation   |
| [API Reference](0003-api-reference.md)                     | Workspace API endpoints                |
| [Algorithms Reference](0009-algorithms-reference.md)       | Query algorithms across namespaces     |

---

**Document Navigation**: [← Configuration Reference](0007-configuration-reference.md) | [README](README.md) | [Algorithms Reference →](0009-algorithms-reference.md)
