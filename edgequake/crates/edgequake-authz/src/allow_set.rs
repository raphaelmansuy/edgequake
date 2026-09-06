//! Allow-set builder (LAW-146-18 / LAW-146-24).

use std::collections::{HashMap, HashSet};
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};

use async_trait::async_trait;
use uuid::Uuid;

use crate::context::AuthzContext;
use crate::error::{AuthzError, AuthzResult};
use crate::principal::PrincipalId;

/// Document IDs the principal may read in this workspace.
#[derive(Debug, Clone, Default)]
pub struct AllowSet {
    pub document_ids: HashSet<Uuid>,
}

/// Default cardinality before switching off `ANY(uuid[])` (LAW-146-21).
pub const ALLOW_SET_ARRAY_THRESHOLD: usize = 2048;

impl AllowSet {
    pub fn empty() -> Self {
        Self {
            document_ids: HashSet::new(),
        }
    }

    pub fn from_ids(ids: impl IntoIterator<Item = Uuid>) -> Self {
        Self {
            document_ids: ids.into_iter().collect(),
        }
    }

    pub fn contains(&self, id: &Uuid) -> bool {
        self.document_ids.contains(id)
    }

    pub fn len(&self) -> usize {
        self.document_ids.len()
    }

    pub fn is_empty(&self) -> bool {
        self.document_ids.is_empty()
    }

    /// Pure intersection with a client filter (LAW-146-24). Never widens.
    pub fn intersect_filter(&self, filter_ids: Option<&[Uuid]>) -> Self {
        match filter_ids {
            None => self.clone(),
            Some(ids) => {
                let set: HashSet<Uuid> = ids.iter().copied().collect();
                Self {
                    document_ids: self
                        .document_ids
                        .iter()
                        .copied()
                        .filter(|id| set.contains(id))
                        .collect(),
                }
            }
        }
    }

    pub fn as_uuid_vec(&self) -> Vec<Uuid> {
        self.document_ids.iter().copied().collect()
    }

    pub fn as_string_vec(&self) -> Vec<String> {
        self.document_ids.iter().map(|u| u.to_string()).collect()
    }

    pub fn fingerprint(&self) -> String {
        use sha2::{Digest, Sha256};
        let mut ids: Vec<_> = self.document_ids.iter().map(|u| u.to_string()).collect();
        ids.sort();
        let mut hasher = Sha256::new();
        for id in ids {
            hasher.update(id.as_bytes());
        }
        hex::encode(hasher.finalize())
    }
}

/// Single allow-set authority (DIP). Implementations: SQL+Cedar or memory tests.
#[async_trait]
pub trait AllowSetProvider: Send + Sync {
    async fn documents_for(&self, ctx: &AuthzContext) -> AuthzResult<AllowSet>;

    /// Load (or initialize) workspace `policy_generation` (LAW-146-22). Default: 1.
    async fn current_policy_generation(&self, _workspace_id: Uuid) -> AuthzResult<u64> {
        Ok(1)
    }

    /// Bump workspace `policy_generation` after label/ACL changes. Default: no-op → 1.
    async fn bump_policy_generation(&self, _workspace_id: Uuid) -> AuthzResult<u64> {
        Ok(1)
    }
}

/// In-memory allow-set for unit tests / flag-off stubs.
pub struct InMemoryAllowSetProvider {
    /// When ABAC disabled, callers should not use this provider.
    map: HashMap<(String, Uuid), HashSet<Uuid>>,
}

impl InMemoryAllowSetProvider {
    pub fn new() -> Self {
        Self {
            map: HashMap::new(),
        }
    }

    pub fn grant(&mut self, principal: &PrincipalId, workspace_id: Uuid, docs: Vec<Uuid>) {
        let key = (format!("{}:{}", principal.kind_str(), principal.id_str()), workspace_id);
        self.map.entry(key).or_default().extend(docs);
    }
}

impl Default for InMemoryAllowSetProvider {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl AllowSetProvider for InMemoryAllowSetProvider {
    async fn documents_for(&self, ctx: &AuthzContext) -> AuthzResult<AllowSet> {
        if !ctx.abac_enabled {
            return Err(AuthzError::Disabled);
        }
        let key = (
            format!("{}:{}", ctx.principal.kind_str(), ctx.principal.id_str()),
            ctx.workspace_id,
        );
        Ok(AllowSet::from_ids(
            self.map.get(&key).cloned().unwrap_or_default(),
        ))
    }
}

struct CacheEntry {
    allow: AllowSet,
    expires_at: Instant,
}

/// Short-TTL cache keyed by (principal, workspace, policy_generation, attr_hash) — G4/R16.
pub struct CachedAllowSetProvider<P: AllowSetProvider> {
    inner: P,
    ttl: Duration,
    cache: RwLock<HashMap<String, CacheEntry>>,
}

impl<P: AllowSetProvider> CachedAllowSetProvider<P> {
    pub fn new(inner: P, ttl: Duration) -> Self {
        Self {
            inner,
            ttl,
            cache: RwLock::new(HashMap::new()),
        }
    }

    pub fn with_default_ttl(inner: P) -> Self {
        Self::new(inner, Duration::from_secs(30))
    }

    fn cache_key(ctx: &AuthzContext) -> String {
        format!(
            "{}:{}:{}:{}:{}",
            ctx.principal.kind_str(),
            ctx.principal.id_str(),
            ctx.workspace_id,
            ctx.policy_generation,
            ctx.attr_hash()
        )
    }

    pub fn invalidate_all(&self) {
        if let Ok(mut g) = self.cache.write() {
            g.clear();
        }
    }
}

#[async_trait]
impl<P: AllowSetProvider> AllowSetProvider for CachedAllowSetProvider<P> {
    async fn documents_for(&self, ctx: &AuthzContext) -> AuthzResult<AllowSet> {
        let key = Self::cache_key(ctx);
        if let Ok(g) = self.cache.read() {
            if let Some(entry) = g.get(&key) {
                if entry.expires_at > Instant::now() {
                    return Ok(entry.allow.clone());
                }
            }
        }
        let allow = self.inner.documents_for(ctx).await?;
        if let Ok(mut g) = self.cache.write() {
            g.insert(
                key,
                CacheEntry {
                    allow: allow.clone(),
                    expires_at: Instant::now() + self.ttl,
                },
            );
        }
        Ok(allow)
    }

    async fn current_policy_generation(&self, workspace_id: Uuid) -> AuthzResult<u64> {
        self.inner.current_policy_generation(workspace_id).await
    }

    async fn bump_policy_generation(&self, workspace_id: Uuid) -> AuthzResult<u64> {
        self.invalidate_all();
        self.inner.bump_policy_generation(workspace_id).await
    }
}

/// Arc wrapper for AppState.
pub type SharedAllowSetProvider = Arc<dyn AllowSetProvider>;

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn intersect_never_widens() {
        let a = Uuid::nil();
        let b = Uuid::from_u128(1);
        let c = Uuid::from_u128(2);
        let allow = AllowSet::from_ids([a, b]);
        let out = allow.intersect_filter(Some(&[b, c]));
        assert!(out.contains(&b));
        assert!(!out.contains(&c));
        assert!(!out.contains(&a));
    }

    #[tokio::test]
    async fn memory_provider_grants() {
        let ws = Uuid::from_u128(9);
        let user = Uuid::from_u128(3);
        let mut p = InMemoryAllowSetProvider::new();
        let principal = PrincipalId::User(user);
        p.grant(&principal, ws, vec![Uuid::from_u128(1)]);
        let ctx = AuthzContext::new(principal, ws, 1, true);
        let set = p.documents_for(&ctx).await.unwrap();
        assert_eq!(set.len(), 1);
    }
}
