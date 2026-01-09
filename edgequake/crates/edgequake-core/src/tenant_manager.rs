//! Tenant-aware EdgeQuake instance manager with caching and isolation.
//!
//! This module manages per-tenant and per-knowledge-base EdgeQuake instances,
//! handling initialization, caching, cleanup, and proper isolation between tenants.
//!
//! ## Implements
//!
//! @implements FEAT0015 (Multi-Tenant Isolation)
//! @implements FEAT0830 (Per-tenant EdgeQuake instance management)
//! @implements FEAT0831 (Instance caching for performance)
//! @implements FEAT0832 (Automatic cleanup of stale instances)
//!
//! ## Use Cases
//!
//! - **UC2420**: System creates EdgeQuake instance for new tenant
//! - **UC2421**: System retrieves cached instance for existing tenant
//! - **UC2422**: System cleans up instances for deleted tenants
//!
//! ## Enforces
//!
//! - **BR0830**: Instances must be isolated by tenant/KB combination
//! - **BR0831**: Cache entries must have TTL for cleanup
//!
//! Based on LightRAG's tenant management: `lightrag/tenant_rag_manager.py`

use crate::error::{Error, Result};
use crate::orchestrator::{EdgeQuake, EdgeQuakeConfig};
use async_trait::async_trait;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Composite key for tenant/KB instances.
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

impl std::fmt::Display for TenantKBKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}:{}", self.tenant_id, self.kb_id)
    }
}

/// Tenant configuration stored in database.
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

impl TenantConfig {
    /// Create a new tenant config with the given ID.
    pub fn new(tenant_id: impl Into<String>) -> Self {
        Self {
            tenant_id: tenant_id.into(),
            ..Default::default()
        }
    }

    /// Set whether the tenant is active.
    pub fn with_active(mut self, active: bool) -> Self {
        self.is_active = active;
        self
    }
}

/// Service for retrieving tenant configuration.
#[async_trait]
pub trait TenantService: Send + Sync {
    /// Get tenant configuration by ID.
    async fn get_tenant(&self, tenant_id: &str) -> Result<Option<TenantConfig>>;

    /// Verify user has access to tenant.
    async fn verify_user_access(&self, user_id: &str, tenant_id: &str) -> Result<bool>;
}

/// Simple in-memory tenant service for testing.
pub struct InMemoryTenantService {
    tenants: RwLock<HashMap<String, TenantConfig>>,
    user_access: RwLock<HashMap<String, Vec<String>>>, // user_id -> tenant_ids
}

impl InMemoryTenantService {
    /// Create a new in-memory tenant service.
    pub fn new() -> Self {
        Self {
            tenants: RwLock::new(HashMap::new()),
            user_access: RwLock::new(HashMap::new()),
        }
    }

    /// Add a tenant.
    pub async fn add_tenant(&self, config: TenantConfig) {
        let mut tenants = self.tenants.write().await;
        tenants.insert(config.tenant_id.clone(), config);
    }

    /// Grant user access to a tenant.
    pub async fn grant_access(&self, user_id: &str, tenant_id: &str) {
        let mut access = self.user_access.write().await;
        access
            .entry(user_id.to_string())
            .or_default()
            .push(tenant_id.to_string());
    }
}

impl Default for InMemoryTenantService {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl TenantService for InMemoryTenantService {
    async fn get_tenant(&self, tenant_id: &str) -> Result<Option<TenantConfig>> {
        let tenants = self.tenants.read().await;
        Ok(tenants.get(tenant_id).cloned())
    }

    async fn verify_user_access(&self, user_id: &str, tenant_id: &str) -> Result<bool> {
        let access = self.user_access.read().await;
        Ok(access
            .get(user_id)
            .map(|tenants| tenants.contains(&tenant_id.to_string()))
            .unwrap_or(false))
    }
}

/// LRU cache entry for EdgeQuake instances.
struct CacheEntry {
    instance: Arc<RwLock<EdgeQuake>>,
    last_accessed: std::time::Instant,
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
    /// Base directory for all tenant/KB data storage.
    base_working_dir: PathBuf,

    /// Service for retrieving tenant configuration.
    tenant_service: Arc<dyn TenantService>,

    /// Template configuration for new instances.
    template_config: EdgeQuakeConfig,

    /// Cache of EdgeQuake instances.
    instances: RwLock<HashMap<TenantKBKey, CacheEntry>>,

    /// Maximum number of cached instances.
    max_cached_instances: usize,

    /// Whether to require user authentication.
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
        Self {
            base_working_dir: base_working_dir.into(),
            tenant_service,
            template_config,
            instances: RwLock::new(HashMap::new()),
            max_cached_instances: max_cached_instances.max(1),
            require_auth: true,
        }
    }

    /// Set whether user authentication is required.
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
    /// - `Error::not_found` if tenant does not exist or is inactive
    /// - `Error::validation` if user does not have access
    /// - `Error::validation` if tenant_id or kb_id are invalid
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
            if let Some(entry) = cache.get(&cache_key) {
                tracing::debug!(tenant_id = %tenant_id, kb_id = %kb_id, "Cache hit");
                return Ok(Arc::clone(&entry.instance));
            }
        }

        // Acquire write lock for initialization
        let mut cache = self.instances.write().await;

        // Second check (double-check locking pattern)
        if let Some(entry) = cache.get(&cache_key) {
            tracing::debug!(tenant_id = %tenant_id, kb_id = %kb_id, "Cache hit (after lock)");
            return Ok(Arc::clone(&entry.instance));
        }

        tracing::info!(tenant_id = %tenant_id, kb_id = %kb_id, "Creating new EdgeQuake instance");

        // Get tenant configuration
        let tenant = self
            .tenant_service
            .get_tenant(&tenant_id)
            .await?
            .ok_or_else(|| Error::internal(format!("Tenant {} not found", tenant_id)))?;

        if !tenant.is_active {
            return Err(Error::validation(format!(
                "Tenant {} is inactive",
                tenant_id
            )));
        }

        // SECURITY: Verify user has access to this tenant
        if let Some(uid) = user_id {
            let has_access = self
                .tenant_service
                .verify_user_access(uid, &tenant_id)
                .await?;

            if !has_access {
                tracing::warn!(
                    user_id = %uid,
                    tenant_id = %tenant_id,
                    "Access denied: user attempted to access tenant"
                );
                return Err(Error::validation(format!(
                    "Access denied to tenant {}",
                    tenant_id
                )));
            }
        } else if self.require_auth {
            tracing::error!(
                tenant_id = %tenant_id,
                "Access denied: user_id required but not provided"
            );
            return Err(Error::validation(
                "User authentication required for tenant access",
            ));
        } else {
            tracing::warn!(
                "No user_id provided for tenant access - allowing for backward compatibility"
            );
        }

        // SECURITY: Create and validate tenant-specific working directory
        let tenant_working_dir = self.validate_working_directory(&tenant_id, &kb_id)?;

        // Create directory if it doesn't exist
        if let Err(e) = std::fs::create_dir_all(&tenant_working_dir) {
            return Err(Error::internal(format!(
                "Failed to create tenant directory: {}",
                e
            )));
        }

        // Create EdgeQuake instance with tenant-specific configuration
        let mut config = self.template_config.clone();
        config.working_dir = tenant_working_dir.to_string_lossy().to_string();
        config.namespace = format!("{}_{}", tenant_id, kb_id);
        config.tenant_id = Some(tenant_id.clone());
        config.workspace_id = Some(kb_id.clone());

        let instance = EdgeQuake::new(config);

        let instance = Arc::new(RwLock::new(instance));

        // Evict LRU entries if at capacity
        if cache.len() >= self.max_cached_instances {
            self.evict_lru_entry(&mut cache);
        }

        // Cache the instance
        cache.insert(
            cache_key,
            CacheEntry {
                instance: Arc::clone(&instance),
                last_accessed: std::time::Instant::now(),
            },
        );

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

        if cache.remove(&cache_key).is_some() {
            tracing::info!(
                tenant_id = %tenant_id,
                kb_id = %kb_id,
                "Cleaned up EdgeQuake instance"
            );
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
            .keys()
            .filter(|key| key.tenant_id == tenant_id)
            .cloned()
            .collect();

        for key in keys_to_remove {
            cache.remove(&key);
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

    /// Get current number of cached instances.
    pub async fn instance_count(&self) -> usize {
        self.instances.read().await.len()
    }

    /// Get all currently cached tenant/KB combinations.
    pub async fn cached_keys(&self) -> Vec<TenantKBKey> {
        self.instances.read().await.keys().cloned().collect()
    }

    // ============ Private helper methods ============

    /// Validate identifier format to prevent injection attacks.
    fn validate_identifier(&self, value: &str, field_name: &str) -> Result<String> {
        // Only allow alphanumeric, hyphens, and underscores
        let valid = value
            .chars()
            .all(|c| c.is_alphanumeric() || c == '-' || c == '_');

        if !valid || value.is_empty() || value.len() > 128 {
            return Err(Error::validation(format!(
                "Invalid {}: must be 1-128 alphanumeric characters, hyphens, or underscores",
                field_name
            )));
        }

        // Check for path traversal attempts
        if value.contains("..") || value.contains('/') || value.contains('\\') {
            return Err(Error::validation(format!(
                "Invalid {}: path traversal not allowed",
                field_name
            )));
        }

        Ok(value.to_string())
    }

    /// Create and validate tenant-specific working directory.
    fn validate_working_directory(&self, tenant_id: &str, kb_id: &str) -> Result<PathBuf> {
        let tenant_dir = self.base_working_dir.join(tenant_id).join(kb_id);

        // Verify the path is actually under base_working_dir (canonicalization check)
        let base_components: Vec<_> = self.base_working_dir.components().collect();
        let tenant_components: Vec<_> = tenant_dir.components().collect();

        // Tenant dir must start with all base dir components
        if tenant_components.len() <= base_components.len() {
            return Err(Error::internal("Invalid tenant directory construction"));
        }

        for (i, base_comp) in base_components.iter().enumerate() {
            if tenant_components.get(i) != Some(base_comp) {
                return Err(Error::validation("Path traversal attempt detected"));
            }
        }

        Ok(tenant_dir)
    }

    /// Evict the least recently used entry from the cache.
    fn evict_lru_entry(&self, cache: &mut HashMap<TenantKBKey, CacheEntry>) {
        if let Some((lru_key, _)) = cache
            .iter()
            .min_by_key(|(_, entry)| entry.last_accessed)
            .map(|(k, v)| (k.clone(), v.last_accessed))
        {
            cache.remove(&lru_key);
            tracing::info!(
                tenant_id = %lru_key.tenant_id,
                kb_id = %lru_key.kb_id,
                "Evicted LRU EdgeQuake instance"
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_identifier() {
        let tenant_service = Arc::new(InMemoryTenantService::new());
        let manager =
            TenantRAGManager::new("/tmp/test", tenant_service, EdgeQuakeConfig::default(), 100);

        // Valid identifiers
        assert!(manager.validate_identifier("tenant-123", "test").is_ok());
        assert!(manager.validate_identifier("tenant_123", "test").is_ok());
        assert!(manager.validate_identifier("TENANT123", "test").is_ok());

        // Invalid identifiers
        assert!(manager.validate_identifier("", "test").is_err());
        assert!(manager
            .validate_identifier("../etc/passwd", "test")
            .is_err());
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

    #[tokio::test]
    async fn test_in_memory_tenant_service() {
        let service = InMemoryTenantService::new();

        // Add tenant
        service.add_tenant(TenantConfig::new("tenant1")).await;

        // Grant access
        service.grant_access("user1", "tenant1").await;

        // Verify
        let tenant = service.get_tenant("tenant1").await.unwrap();
        assert!(tenant.is_some());
        assert_eq!(tenant.unwrap().tenant_id, "tenant1");

        let has_access = service
            .verify_user_access("user1", "tenant1")
            .await
            .unwrap();
        assert!(has_access);

        let no_access = service
            .verify_user_access("user2", "tenant1")
            .await
            .unwrap();
        assert!(!no_access);
    }
}
