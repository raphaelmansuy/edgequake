# Phase 1: Multi-Tenancy Implementation

**Document ID:** 02-PHASE1-MULTI-TENANCY  
**Priority:** 🔴 P0 CRITICAL  
**Effort:** 12 person-days  
**Duration:** Weeks 1-3  
**Dependencies:** None  
**Blocks:** [06-PHASE3-API-FEATURES.md](./06-PHASE3-API-FEATURES.md)

---

## 📋 Overview

This document provides high-precision implementation guidance for multi-tenancy support in EdgeQuake. Multi-tenancy is critical for SaaS deployment, enabling multiple organizations to share the same EdgeQuake instance while maintaining strict data isolation.

### Gaps Addressed

| Gap ID      | Feature               | Severity | Status         |
| ----------- | --------------------- | -------- | -------------- |
| **GAP-003** | Multi-tenancy Support | 🔴 P0    | 🔲 Not started |
| **GAP-004** | Tenant RAG Manager    | 🔴 P0    | 🔲 Not started |
| **GAP-037** | Tenant/KB Isolation   | 🔴 P0    | 🔲 Not started |

### Cross-References

- **Source Analysis:** [../gap-analysis.md](../gap-analysis.md#feature-f-066-multi-tenancy)
- **Source Code:** `lightrag/tenant_rag_manager.py` (reference implementation)
- **Master Plan:** [00-INDEX.md](./00-INDEX.md#phase-1-foundation-weeks-1-3)
- **Testing Plan:** [07-VALIDATION-TESTING.md](./07-VALIDATION-TESTING.md#multi-tenancy-tests)

---

## 🎯 Tenant RAG Manager

### 1.1 Objective

Implement a TenantRAGManager that manages per-tenant/per-KB EdgeQuake instances with LRU caching, proper isolation, and thread-safe initialization.

### 1.2 Source Reference

**Location:** `lightrag/tenant_rag_manager.py` (330 lines)

**LightRAG Features:**

- Per-tenant/KB instance caching with LRU eviction
- Double-check locking for thread-safe initialization
- Template configuration inheritance
- User access verification
- Proper resource cleanup on eviction

### 1.3 Implementation Tasks

#### Task 1.3.1: Create TenantRAGManager

**File:** `edgequake/crates/edgequake-core/src/tenant_manager.rs` (NEW)

```rust
// NEW FILE: edgequake/crates/edgequake-core/src/tenant_manager.rs

//! Tenant-aware EdgeQuake instance manager with caching and isolation.
//!
//! This module manages per-tenant and per-knowledge-base EdgeQuake instances,
//! handling initialization, caching, cleanup, and proper isolation between tenants.

use crate::error::{Error, Result};
use crate::orchestrator::{EdgeQuake, EdgeQuakeConfig};
use lru::LruCache;
use std::collections::HashMap;
use std::num::NonZeroUsize;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Composite key for tenant/KB instances
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TenantKBKey {
    pub tenant_id: String,
    pub kb_id: String,
}

impl TenantKBKey {
    pub fn new(tenant_id: impl Into<String>, kb_id: impl Into<String>) -> Self {
        Self {
            tenant_id: tenant_id.into(),
            kb_id: kb_id.into(),
        }
    }
}

/// Tenant configuration stored in database
#[derive(Debug, Clone)]
pub struct TenantConfig {
    pub tenant_id: String,
    pub is_active: bool,
    pub top_k: usize,
    pub chunk_top_k: usize,
    pub cosine_threshold: f32,
    pub custom_metadata: HashMap<String, serde_json::Value>,
}

impl Default for TenantConfig {
    fn default() -> Self {
        Self {
            tenant_id: "default".to_string(),
            is_active: true,
            top_k: 60,
            chunk_top_k: 40,
            cosine_threshold: 0.2,
            custom_metadata: HashMap::new(),
        }
    }
}

/// Service for retrieving tenant configuration
#[async_trait::async_trait]
pub trait TenantService: Send + Sync {
    /// Get tenant configuration by ID
    async fn get_tenant(&self, tenant_id: &str) -> Result<Option<TenantConfig>>;

    /// Verify user has access to tenant
    async fn verify_user_access(&self, user_id: &str, tenant_id: &str) -> Result<bool>;
}

/// Manages EdgeQuake instances per tenant/KB combination with caching and isolation.
///
/// # Features
/// - Automatic instance caching to avoid repeated initialization
/// - Per-tenant isolation through separate working directories
/// - Configurable max cached instances (LRU eviction)
/// - Async-safe initialization with double-check locking
/// - Proper resource cleanup on instance removal
pub struct TenantRAGManager {
    /// Base directory for all tenant/KB data storage
    base_working_dir: PathBuf,

    /// Service for retrieving tenant configuration
    tenant_service: Arc<dyn TenantService>,

    /// Template configuration for new instances
    template_config: EdgeQuakeConfig,

    /// LRU cache of EdgeQuake instances
    instances: RwLock<LruCache<TenantKBKey, Arc<RwLock<EdgeQuake>>>>,

    /// Maximum number of cached instances
    max_cached_instances: usize,

    /// Whether to require user authentication
    require_auth: bool,
}

impl TenantRAGManager {
    /// Create a new TenantRAGManager.
    ///
    /// # Arguments
    /// * `base_working_dir` - Base directory for all tenant/KB data storage
    /// * `tenant_service` - Service for retrieving tenant configuration
    /// * `template_config` - Template configuration to copy for new instances
    /// * `max_cached_instances` - Maximum number of instances to cache (default: 100)
    pub fn new(
        base_working_dir: impl Into<PathBuf>,
        tenant_service: Arc<dyn TenantService>,
        template_config: EdgeQuakeConfig,
        max_cached_instances: usize,
    ) -> Self {
        let capacity = NonZeroUsize::new(max_cached_instances).unwrap_or(NonZeroUsize::new(100).unwrap());

        Self {
            base_working_dir: base_working_dir.into(),
            tenant_service,
            template_config,
            instances: RwLock::new(LruCache::new(capacity)),
            max_cached_instances,
            require_auth: true,
        }
    }

    /// Set whether user authentication is required
    pub fn with_auth_required(mut self, required: bool) -> Self {
        self.require_auth = required;
        self
    }

    /// Get or create an EdgeQuake instance for a tenant/KB combination.
    ///
    /// This method implements double-check locking to avoid race conditions
    /// when multiple requests try to initialize the same instance concurrently.
    /// Instances are cached and reused across requests for the same tenant/KB.
    ///
    /// # Security
    /// Validates user has access to requested tenant before returning instance.
    ///
    /// # Arguments
    /// * `tenant_id` - The tenant ID (must be valid identifier)
    /// * `kb_id` - The knowledge base ID (must be valid identifier)
    /// * `user_id` - User identifier from JWT token (required for security validation)
    ///
    /// # Errors
    /// - `Error::NotFound` if tenant does not exist or is inactive
    /// - `Error::PermissionDenied` if user does not have access
    /// - `Error::InvalidInput` if tenant_id or kb_id are invalid
    pub async fn get_instance(
        &self,
        tenant_id: &str,
        kb_id: &str,
        user_id: Option<&str>,
    ) -> Result<Arc<RwLock<EdgeQuake>>> {
        // SECURITY: Validate identifier format to prevent injection attacks
        let tenant_id = self.validate_identifier(tenant_id, "tenant_id")?;
        let kb_id = self.validate_identifier(kb_id, "kb_id")?;

        let cache_key = TenantKBKey::new(&tenant_id, &kb_id);

        // First check (fast path - read lock only)
        {
            let cache = self.instances.read().await;
            if let Some(instance) = cache.peek(&cache_key) {
                tracing::debug!(tenant_id = %tenant_id, kb_id = %kb_id, "Cache hit");
                return Ok(Arc::clone(instance));
            }
        }

        // Acquire write lock for initialization
        let mut cache = self.instances.write().await;

        // Second check (double-check locking pattern)
        if let Some(instance) = cache.get(&cache_key) {
            tracing::debug!(tenant_id = %tenant_id, kb_id = %kb_id, "Cache hit (after lock)");
            return Ok(Arc::clone(instance));
        }

        tracing::info!(tenant_id = %tenant_id, kb_id = %kb_id, "Creating new EdgeQuake instance");

        // Get tenant configuration
        let tenant = self.tenant_service
            .get_tenant(&tenant_id)
            .await?
            .ok_or_else(|| Error::not_found(format!("Tenant {} not found", tenant_id)))?;

        if !tenant.is_active {
            return Err(Error::permission_denied(format!("Tenant {} is inactive", tenant_id)));
        }

        // SECURITY: Verify user has access to this tenant
        if let Some(uid) = user_id {
            let has_access = self.tenant_service
                .verify_user_access(uid, &tenant_id)
                .await?;

            if !has_access {
                tracing::warn!(
                    user_id = %uid,
                    tenant_id = %tenant_id,
                    "Access denied: user attempted to access tenant"
                );
                return Err(Error::permission_denied(format!(
                    "Access denied to tenant {}", tenant_id
                )));
            }
        } else if self.require_auth {
            tracing::error!(
                tenant_id = %tenant_id,
                "Access denied: user_id required but not provided"
            );
            return Err(Error::permission_denied(
                "User authentication required for tenant access"
            ));
        } else {
            tracing::warn!("No user_id provided for tenant access - allowing for backward compatibility");
        }

        // SECURITY: Create and validate tenant-specific working directory
        let tenant_working_dir = self.validate_working_directory(&tenant_id, &kb_id)?;
        tokio::fs::create_dir_all(&tenant_working_dir).await
            .map_err(|e| Error::internal(format!("Failed to create tenant directory: {}", e)))?;

        // Create EdgeQuake instance with tenant-specific configuration
        let mut config = self.template_config.clone();
        config.working_dir = tenant_working_dir.to_string_lossy().to_string();
        config.namespace = format!("{}_{}", tenant_id, kb_id);

        // Apply tenant-specific overrides
        // (top_k, cosine_threshold, etc. would come from TenantConfig)

        let mut instance = EdgeQuake::new(config);

        // Note: Providers need to be set by the caller or from a shared pool
        // instance.with_providers(llm, embedding);

        // Initialize the instance
        // instance.initialize().await?;

        let instance = Arc::new(RwLock::new(instance));

        // Check if we need to evict (LruCache handles this automatically)
        if cache.len() >= self.max_cached_instances {
            // LruCache will evict the least recently used entry on put
            // We could add custom cleanup logic here if needed
            tracing::info!("Cache at capacity, LRU eviction will occur");
        }

        // Cache the instance
        cache.put(cache_key, Arc::clone(&instance));

        tracing::info!(
            tenant_id = %tenant_id,
            kb_id = %kb_id,
            cache_size = cache.len(),
            "EdgeQuake instance created and cached"
        );

        Ok(instance)
    }

    /// Clean up and remove a cached instance.
    ///
    /// Call this when a knowledge base is deleted or a tenant is removed
    /// to ensure proper resource cleanup.
    pub async fn cleanup_instance(&self, tenant_id: &str, kb_id: &str) -> Result<()> {
        let cache_key = TenantKBKey::new(tenant_id, kb_id);
        let mut cache = self.instances.write().await;

        if cache.pop(&cache_key).is_some() {
            tracing::info!(
                tenant_id = %tenant_id,
                kb_id = %kb_id,
                "Cleaned up EdgeQuake instance"
            );
            // Note: Could add finalize_storages() call here if EdgeQuake implements it
        }

        Ok(())
    }

    /// Clean up all cached instances for a specific tenant.
    ///
    /// Call this when a tenant is deleted to ensure all its knowledge bases
    /// are properly cleaned up.
    pub async fn cleanup_tenant_instances(&self, tenant_id: &str) -> Result<()> {
        let mut cache = self.instances.write().await;

        // Collect keys to remove (can't modify while iterating)
        let keys_to_remove: Vec<_> = cache
            .iter()
            .filter(|(key, _)| key.tenant_id == tenant_id)
            .map(|(key, _)| key.clone())
            .collect();

        for key in keys_to_remove {
            cache.pop(&key);
            tracing::info!(
                tenant_id = %key.tenant_id,
                kb_id = %key.kb_id,
                "Cleaned up tenant instance"
            );
        }

        Ok(())
    }

    /// Clean up all cached instances.
    ///
    /// Call during application shutdown to ensure all resources are released.
    pub async fn cleanup_all(&self) -> Result<()> {
        let mut cache = self.instances.write().await;
        let count = cache.len();
        cache.clear();
        tracing::info!(count = count, "Cleaned up all cached EdgeQuake instances");
        Ok(())
    }

    /// Get current number of cached instances
    pub async fn instance_count(&self) -> usize {
        self.instances.read().await.len()
    }

    /// Get all currently cached tenant/KB combinations
    pub async fn cached_keys(&self) -> Vec<TenantKBKey> {
        self.instances.read().await
            .iter()
            .map(|(k, _)| k.clone())
            .collect()
    }

    // ============ Private helper methods ============

    /// Validate identifier format to prevent injection attacks
    fn validate_identifier(&self, value: &str, field_name: &str) -> Result<String> {
        // Only allow alphanumeric, hyphens, and underscores
        let valid = value.chars().all(|c| c.is_alphanumeric() || c == '-' || c == '_');

        if !valid || value.is_empty() || value.len() > 128 {
            return Err(Error::invalid_input(format!(
                "Invalid {}: must be 1-128 alphanumeric characters, hyphens, or underscores",
                field_name
            )));
        }

        // Check for path traversal attempts
        if value.contains("..") || value.contains('/') || value.contains('\\') {
            return Err(Error::invalid_input(format!(
                "Invalid {}: path traversal not allowed",
                field_name
            )));
        }

        Ok(value.to_string())
    }

    /// Create and validate tenant-specific working directory
    fn validate_working_directory(&self, tenant_id: &str, kb_id: &str) -> Result<PathBuf> {
        let tenant_dir = self.base_working_dir
            .join(tenant_id)
            .join(kb_id);

        // Verify the path is actually under base_working_dir (canonicalization check)
        // Note: Can't fully canonicalize until directory exists, so we check components
        let base_components: Vec<_> = self.base_working_dir.components().collect();
        let tenant_components: Vec<_> = tenant_dir.components().collect();

        // Tenant dir must start with all base dir components
        if tenant_components.len() <= base_components.len() {
            return Err(Error::internal("Invalid tenant directory construction"));
        }

        for (i, base_comp) in base_components.iter().enumerate() {
            if tenant_components.get(i) != Some(base_comp) {
                return Err(Error::permission_denied("Path traversal attempt detected"));
            }
        }

        Ok(tenant_dir)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct MockTenantService;

    #[async_trait::async_trait]
    impl TenantService for MockTenantService {
        async fn get_tenant(&self, tenant_id: &str) -> Result<Option<TenantConfig>> {
            Ok(Some(TenantConfig {
                tenant_id: tenant_id.to_string(),
                is_active: true,
                ..Default::default()
            }))
        }

        async fn verify_user_access(&self, _user_id: &str, _tenant_id: &str) -> Result<bool> {
            Ok(true)
        }
    }

    #[test]
    fn test_validate_identifier() {
        let manager = TenantRAGManager::new(
            "/tmp/test",
            Arc::new(MockTenantService),
            EdgeQuakeConfig::default(),
            100,
        );

        // Valid identifiers
        assert!(manager.validate_identifier("tenant-123", "test").is_ok());
        assert!(manager.validate_identifier("tenant_123", "test").is_ok());
        assert!(manager.validate_identifier("TENANT123", "test").is_ok());

        // Invalid identifiers
        assert!(manager.validate_identifier("", "test").is_err());
        assert!(manager.validate_identifier("../etc/passwd", "test").is_err());
        assert!(manager.validate_identifier("tenant/kb", "test").is_err());
        assert!(manager.validate_identifier("tenant\\kb", "test").is_err());
    }

    #[test]
    fn test_tenant_kb_key() {
        let key1 = TenantKBKey::new("tenant1", "kb1");
        let key2 = TenantKBKey::new("tenant1", "kb1");
        let key3 = TenantKBKey::new("tenant1", "kb2");

        assert_eq!(key1, key2);
        assert_ne!(key1, key3);
    }
}
```

**Dependencies to add:**

```toml
# Add to edgequake/crates/edgequake-core/Cargo.toml [dependencies]
lru = "0.12"
async-trait = "0.1"
```

---

#### Task 1.3.2: Update Module Exports

**File:** `edgequake/crates/edgequake-core/src/lib.rs`

```rust
// ADD to lib.rs
pub mod tenant_manager;

pub use tenant_manager::{TenantRAGManager, TenantKBKey, TenantConfig, TenantService};
```

---

### 1.4 Tenant RAG Manager Checklist

- [ ] TenantRAGManager struct created
- [ ] LRU caching implemented
- [ ] Double-check locking pattern
- [ ] Identifier validation (security)
- [ ] Path traversal prevention (security)
- [ ] User access verification
- [ ] Instance cleanup methods
- [ ] Unit tests pass
- [ ] lru crate added to dependencies

---

## 🎯 Tenant Isolation

### 2.1 Objective

Implement per-tenant and per-KB data isolation in all storage backends to prevent cross-tenant data access.

### 2.2 Implementation Tasks

#### Task 2.2.1: Add Tenant Context to Storage Traits

**File:** `edgequake/crates/edgequake-storage/src/traits/mod.rs`

```rust
// ADD to traits module

/// Context for multi-tenant storage operations
#[derive(Debug, Clone, Default)]
pub struct TenantContext {
    /// Tenant identifier
    pub tenant_id: String,
    /// Knowledge base identifier
    pub kb_id: String,
}

impl TenantContext {
    pub fn new(tenant_id: impl Into<String>, kb_id: impl Into<String>) -> Self {
        Self {
            tenant_id: tenant_id.into(),
            kb_id: kb_id.into(),
        }
    }

    /// Get storage prefix for this tenant/KB
    pub fn storage_prefix(&self) -> String {
        format!("{}:{}", self.tenant_id, self.kb_id)
    }

    /// Get namespaced key for storage
    pub fn namespaced_key(&self, key: &str) -> String {
        format!("{}:{}:{}", self.tenant_id, self.kb_id, key)
    }
}
```

---

#### Task 2.2.2: Update KVStorage Trait

**File:** `edgequake/crates/edgequake-storage/src/traits/kv.rs`

```rust
// MODIFY KVStorage trait to support tenant context

use super::TenantContext;

#[async_trait::async_trait]
pub trait KVStorage: Send + Sync {
    /// Set tenant context for this storage instance
    fn with_tenant_context(&self, context: TenantContext) -> Box<dyn KVStorage>;

    /// Get a value by key (within current tenant context)
    async fn get(&self, key: &str) -> StorageResult<Option<serde_json::Value>>;

    /// Set a value (within current tenant context)
    async fn set(&self, key: &str, value: serde_json::Value) -> StorageResult<()>;

    /// Delete a key (within current tenant context)
    async fn delete(&self, key: &str) -> StorageResult<()>;

    /// List keys with prefix (within current tenant context)
    async fn list_keys(&self, prefix: &str) -> StorageResult<Vec<String>>;

    /// Check if key exists (within current tenant context)
    async fn exists(&self, key: &str) -> StorageResult<bool>;
}
```

---

#### Task 2.2.3: Update Memory Storage Implementation

**File:** `edgequake/crates/edgequake-storage/src/memory/kv.rs`

```rust
// MODIFY MemoryKVStorage to use tenant prefixing

use crate::traits::TenantContext;

pub struct MemoryKVStorage {
    data: Arc<RwLock<HashMap<String, serde_json::Value>>>,
    tenant_context: Option<TenantContext>,
}

impl MemoryKVStorage {
    fn prefixed_key(&self, key: &str) -> String {
        match &self.tenant_context {
            Some(ctx) => ctx.namespaced_key(key),
            None => key.to_string(),
        }
    }
}

#[async_trait::async_trait]
impl KVStorage for MemoryKVStorage {
    fn with_tenant_context(&self, context: TenantContext) -> Box<dyn KVStorage> {
        Box::new(Self {
            data: Arc::clone(&self.data),
            tenant_context: Some(context),
        })
    }

    async fn get(&self, key: &str) -> StorageResult<Option<serde_json::Value>> {
        let prefixed = self.prefixed_key(key);
        let data = self.data.read().await;
        Ok(data.get(&prefixed).cloned())
    }

    async fn set(&self, key: &str, value: serde_json::Value) -> StorageResult<()> {
        let prefixed = self.prefixed_key(key);
        let mut data = self.data.write().await;
        data.insert(prefixed, value);
        Ok(())
    }

    async fn delete(&self, key: &str) -> StorageResult<()> {
        let prefixed = self.prefixed_key(key);
        let mut data = self.data.write().await;
        data.remove(&prefixed);
        Ok(())
    }

    async fn list_keys(&self, prefix: &str) -> StorageResult<Vec<String>> {
        let prefixed = self.prefixed_key(prefix);
        let data = self.data.read().await;
        let keys: Vec<_> = data.keys()
            .filter(|k| k.starts_with(&prefixed))
            .map(|k| {
                // Remove tenant prefix from returned keys
                match &self.tenant_context {
                    Some(ctx) => k.strip_prefix(&ctx.storage_prefix())
                        .and_then(|s| s.strip_prefix(':'))
                        .unwrap_or(k)
                        .to_string(),
                    None => k.clone(),
                }
            })
            .collect();
        Ok(keys)
    }

    async fn exists(&self, key: &str) -> StorageResult<bool> {
        let prefixed = self.prefixed_key(key);
        let data = self.data.read().await;
        Ok(data.contains_key(&prefixed))
    }
}
```

---

#### Task 2.2.4: Create Tenant Middleware for API

**File:** `edgequake/crates/edgequake-api/src/middleware/tenant.rs` (NEW)

```rust
// NEW FILE: edgequake/crates/edgequake-api/src/middleware/tenant.rs

//! Tenant extraction middleware for multi-tenant API requests.

use axum::{
    extract::{FromRequestParts, Path},
    http::{request::Parts, StatusCode},
    response::{IntoResponse, Response},
};
use serde::Deserialize;

/// Tenant context extracted from request path or headers
#[derive(Debug, Clone)]
pub struct TenantContext {
    pub tenant_id: String,
    pub kb_id: Option<String>,
    pub user_id: Option<String>,
}

/// Error type for tenant extraction failures
pub enum TenantExtractionError {
    MissingTenantId,
    InvalidTenantId(String),
    Unauthorized,
}

impl IntoResponse for TenantExtractionError {
    fn into_response(self) -> Response {
        let (status, message) = match self {
            Self::MissingTenantId => (StatusCode::BAD_REQUEST, "Missing tenant_id"),
            Self::InvalidTenantId(msg) => (StatusCode::BAD_REQUEST, msg.leak()),
            Self::Unauthorized => (StatusCode::UNAUTHORIZED, "Unauthorized"),
        };
        (status, message).into_response()
    }
}

#[derive(Deserialize)]
struct TenantPath {
    tenant_id: String,
    kb_id: Option<String>,
}

#[axum::async_trait]
impl<S> FromRequestParts<S> for TenantContext
where
    S: Send + Sync,
{
    type Rejection = TenantExtractionError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        // Try to extract from path first
        let path_params: Option<Path<TenantPath>> =
            Path::from_request_parts(parts, state).await.ok();

        // Try to extract from headers if not in path
        let tenant_id = path_params
            .as_ref()
            .map(|p| p.tenant_id.clone())
            .or_else(|| {
                parts.headers
                    .get("X-Tenant-ID")
                    .and_then(|v| v.to_str().ok())
                    .map(|s| s.to_string())
            })
            .ok_or(TenantExtractionError::MissingTenantId)?;

        let kb_id = path_params
            .as_ref()
            .and_then(|p| p.kb_id.clone())
            .or_else(|| {
                parts.headers
                    .get("X-KB-ID")
                    .and_then(|v| v.to_str().ok())
                    .map(|s| s.to_string())
            });

        // Extract user_id from JWT claims (if present in extensions)
        let user_id = parts.extensions
            .get::<crate::auth::JwtClaims>()
            .map(|claims| claims.sub.clone());

        // Validate tenant_id format
        if !Self::is_valid_identifier(&tenant_id) {
            return Err(TenantExtractionError::InvalidTenantId(
                "Invalid tenant_id format".to_string()
            ));
        }

        Ok(TenantContext {
            tenant_id,
            kb_id,
            user_id,
        })
    }
}

impl TenantContext {
    fn is_valid_identifier(value: &str) -> bool {
        !value.is_empty()
            && value.len() <= 128
            && value.chars().all(|c| c.is_alphanumeric() || c == '-' || c == '_')
            && !value.contains("..")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_identifiers() {
        assert!(TenantContext::is_valid_identifier("tenant-123"));
        assert!(TenantContext::is_valid_identifier("tenant_123"));
        assert!(TenantContext::is_valid_identifier("TENANT123"));
    }

    #[test]
    fn test_invalid_identifiers() {
        assert!(!TenantContext::is_valid_identifier(""));
        assert!(!TenantContext::is_valid_identifier("../etc"));
        assert!(!TenantContext::is_valid_identifier("tenant/kb"));
    }
}
```

---

#### Task 2.2.5: Update API Routes for Multi-Tenancy

**File:** `edgequake/crates/edgequake-api/src/routes.rs`

Add tenant-scoped routes:

```rust
// ADD tenant-scoped route group to routes.rs

use crate::middleware::tenant::TenantContext;

pub fn tenant_routes() -> Router<AppState> {
    Router::new()
        // Documents scoped to tenant/KB
        .route(
            "/tenants/:tenant_id/kb/:kb_id/documents",
            post(handlers::documents::upload_tenant)
                .get(handlers::documents::list_tenant)
        )
        .route(
            "/tenants/:tenant_id/kb/:kb_id/documents/:doc_id",
            get(handlers::documents::get_tenant)
                .delete(handlers::documents::delete_tenant)
        )
        // Query scoped to tenant/KB
        .route(
            "/tenants/:tenant_id/kb/:kb_id/query",
            post(handlers::query::query_tenant)
        )
        .route(
            "/tenants/:tenant_id/kb/:kb_id/query/stream",
            post(handlers::query::query_stream_tenant)
        )
        // Graph scoped to tenant/KB
        .route(
            "/tenants/:tenant_id/kb/:kb_id/graph",
            get(handlers::graph::get_graph_tenant)
        )
        // Tenant management
        .route(
            "/tenants",
            get(handlers::tenant::list_tenants)
                .post(handlers::tenant::create_tenant)
        )
        .route(
            "/tenants/:tenant_id",
            get(handlers::tenant::get_tenant)
                .delete(handlers::tenant::delete_tenant)
        )
        // Knowledge base management
        .route(
            "/tenants/:tenant_id/kb",
            get(handlers::tenant::list_kbs)
                .post(handlers::tenant::create_kb)
        )
}
```

---

### 2.3 Tenant Isolation Checklist

- [ ] TenantContext struct created
- [ ] Storage traits updated with tenant context
- [ ] Memory storage implements tenant prefixing
- [ ] PostgreSQL storage implements tenant prefixing
- [ ] Tenant middleware extracts context from requests
- [ ] API routes scoped to tenant/KB
- [ ] Cross-tenant access tests (isolation verification)

---

## 📊 Testing Requirements

See [07-VALIDATION-TESTING.md](./07-VALIDATION-TESTING.md#multi-tenancy-tests) for full specifications.

### Unit Tests

```bash
cargo test --package edgequake-core --lib tenant_manager
cargo test --package edgequake-storage --lib tenant_context
cargo test --package edgequake-api --lib tenant_middleware
```

### Integration Tests

```rust
// Test file: edgequake/crates/edgequake-core/tests/tenant_isolation.rs

#[tokio::test]
async fn test_tenant_isolation() {
    let manager = setup_tenant_manager().await;

    // Create instances for two different tenants
    let tenant_a = manager.get_instance("tenant-a", "kb-1", Some("user-a")).await.unwrap();
    let tenant_b = manager.get_instance("tenant-b", "kb-1", Some("user-b")).await.unwrap();

    // Insert document in tenant A
    {
        let mut instance = tenant_a.write().await;
        instance.insert("Secret document for tenant A").await.unwrap();
    }

    // Query in tenant B should NOT find tenant A's data
    {
        let instance = tenant_b.read().await;
        let result = instance.query("secret", QueryParams::default()).await.unwrap();
        assert!(result.context.chunks.is_empty(), "Tenant B should not see tenant A data");
    }
}

#[tokio::test]
async fn test_cross_tenant_access_denied() {
    let manager = setup_tenant_manager().await;

    // User A should not access tenant B
    let result = manager.get_instance("tenant-b", "kb-1", Some("user-a")).await;
    assert!(matches!(result, Err(Error::PermissionDenied(_))));
}
```

---

## 🔗 Cross-References

| Topic        | Document                                                 | Section             |
| ------------ | -------------------------------------------------------- | ------------------- |
| Gap Details  | [../gap-analysis.md](../gap-analysis.md)                 | F-066, F-067, F-068 |
| Testing Plan | [07-VALIDATION-TESTING.md](./07-VALIDATION-TESTING.md)   | Multi-Tenancy Tests |
| Dependencies | [09-DEPENDENCY-GRAPH.md](./09-DEPENDENCY-GRAPH.md)       | Multi-Tenancy       |
| API Features | [06-PHASE3-API-FEATURES.md](./06-PHASE3-API-FEATURES.md) | Tenant Routes       |
| Master Index | [00-INDEX.md](./00-INDEX.md)                             | Phase 1             |

---

## ✅ Completion Criteria

| Criterion                | Target          | Validation          |
| ------------------------ | --------------- | ------------------- |
| TenantRAGManager works   | ✅              | Unit tests pass     |
| LRU caching              | ≤ max instances | Unit test           |
| Cross-tenant isolation   | No data leakage | Integration test    |
| Path traversal blocked   | Security test   | Unit test           |
| User access verified     | Auth required   | Integration test    |
| API routes tenant-scoped | All routes      | Manual verification |

---

_Document Version: 1.0_  
_Last Updated: 2024-12-24_  
_Owner: EdgeQuake Platform Team_
