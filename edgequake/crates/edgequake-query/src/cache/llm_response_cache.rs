//! SPEC-103 — LightRAG-parity LLM response cache (keywords + query).
//!
//! Unified durable cache: L1 memory + L2 `public.llm_cache` via [`KVStorage`].
//! Keys: `{mode}:{cache_type}:{hash}-cache` (LR flattened shape + SPEC-091 suffix).

use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};

use async_trait::async_trait;
use edgequake_storage::kv_keys;
use edgequake_storage::traits::KVStorage;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

/// Field separator for hash inputs (avoids LR delimiter-less collisions).
const HASH_SEP: u8 = 0x1e;

/// LLM response cache family (v1: query path only).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LlmCacheType {
    Keywords,
    Query,
}

impl LlmCacheType {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Keywords => "keywords",
            Self::Query => "query",
        }
    }
}

/// Resolved enable flags (LAW-C6).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LlmCacheFlags {
    pub master: bool,
    pub keywords: bool,
    pub answer: bool,
}

/// Master `EDGEQUAKE_LLM_CACHE` (default **on**).
pub fn master_llm_cache_enabled() -> bool {
    !matches!(
        std::env::var("EDGEQUAKE_LLM_CACHE")
            .ok()
            .as_deref()
            .map(str::trim)
            .map(str::to_ascii_lowercase)
            .as_deref(),
        Some("0") | Some("false") | Some("off") | Some("no")
    )
}

fn env_falsey(name: &str) -> bool {
    matches!(
        std::env::var(name)
            .ok()
            .as_deref()
            .map(str::trim)
            .map(str::to_ascii_lowercase)
            .as_deref(),
        Some("0") | Some("false") | Some("off") | Some("no")
    )
}

fn env_truthy_set(name: &str) -> Option<bool> {
    let raw = std::env::var(name).ok()?;
    let v = raw.trim().to_ascii_lowercase();
    if v.is_empty() {
        return None;
    }
    Some(matches!(v.as_str(), "1" | "true" | "yes" | "on"))
}

/// LAW-C6: master + keyword/answer overrides.
pub fn resolve_llm_cache_flags() -> LlmCacheFlags {
    let master = master_llm_cache_enabled();
    let keywords = master && !env_falsey("EDGEQUAKE_KEYWORD_CACHE");
    let answer = match env_truthy_set("EDGEQUAKE_QUERY_ANSWER_CACHE") {
        Some(explicit) => master && explicit,
        None => master, // unset → follow master (product default ON)
    };
    LlmCacheFlags {
        master,
        keywords,
        answer,
    }
}

/// Answer cache enabled (SPEC-103 / LAW-C6). Replaces 064 default-off when master on.
pub fn answer_cache_enabled_from_env() -> bool {
    resolve_llm_cache_flags().answer
}

/// Keyword cache enabled under master switch.
pub fn keyword_cache_enabled_from_flags() -> bool {
    resolve_llm_cache_flags().keywords
}

/// Delimited SHA-256 of args (LAW-C3).
pub fn compute_args_hash(parts: &[&str]) -> String {
    let mut hasher = Sha256::new();
    for (i, p) in parts.iter().enumerate() {
        if i > 0 {
            hasher.update([HASH_SEP]);
        }
        hasher.update(p.as_bytes());
    }
    format!("{:x}", hasher.finalize())
}

/// SPEC-146: principal + policy_generation + allow fingerprint for cache isolation.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AuthzCacheScope {
    pub principal: String,
    pub policy_generation: u64,
    pub allow_fingerprint: String,
}

impl AuthzCacheScope {
    pub fn from_parts(
        principal: impl Into<String>,
        policy_generation: u64,
        allow_fingerprint: impl Into<String>,
    ) -> Self {
        Self {
            principal: principal.into(),
            policy_generation,
            allow_fingerprint: allow_fingerprint.into(),
        }
    }

    fn append_to_hash_parts<'a>(&'a self, parts: &mut Vec<&'a str>, gen_buf: &'a mut String) {
        parts.push(&self.principal);
        *gen_buf = self.policy_generation.to_string();
        parts.push(gen_buf.as_str());
        parts.push(&self.allow_fingerprint);
    }
}

tokio::task_local! {
    /// SPEC-146: request-scoped authz for keyword cache keying (set by query pipeline).
    static AUTHZ_CACHE_SCOPE: Option<AuthzCacheScope>;
}

/// Run `f` with authz cache scope visible to keyword hash builders.
pub async fn with_authz_cache_scope<F, T>(scope: Option<AuthzCacheScope>, f: F) -> T
where
    F: std::future::Future<Output = T>,
{
    AUTHZ_CACHE_SCOPE.scope(scope, f).await
}

/// Read the current task-local authz scope (if any).
pub fn current_authz_cache_scope() -> Option<AuthzCacheScope> {
    AUTHZ_CACHE_SCOPE.try_with(|s| s.clone()).ok().flatten()
}

/// Flattened LR-shaped id before SPEC-091 suffix.
pub fn flattened_cache_id(mode: &str, cache_type: LlmCacheType, hash: &str) -> String {
    format!("{mode}:{}:{hash}", cache_type.as_str())
}

/// Storage key: `{mode}:{cache_type}:{hash}-cache`.
pub fn llm_cache_storage_key(mode: &str, cache_type: LlmCacheType, hash: &str) -> String {
    kv_keys::llm_cache(&flattened_cache_id(mode, cache_type, hash))
}

pub fn hash_keyword_args(query: &str, mode: &str, model: &str, language: Option<&str>) -> String {
    hash_keyword_args_scoped(query, mode, model, language, None)
}

/// SPEC-146: keyword cache hash includes authz scope when present.
pub fn hash_keyword_args_scoped(
    query: &str,
    mode: &str,
    model: &str,
    language: Option<&str>,
    authz: Option<&AuthzCacheScope>,
) -> String {
    let lang = language.unwrap_or("");
    match authz {
        None => compute_args_hash(&[query, mode, model, lang]),
        Some(scope) => {
            let mut gen = String::new();
            let mut parts: Vec<&str> = vec![query, mode, model, lang];
            scope.append_to_hash_parts(&mut parts, &mut gen);
            compute_args_hash(&parts)
        }
    }
}

/// Query answer hash = full built RAG prompt (context-inclusive).
pub fn hash_query_prompt(prompt: &str) -> String {
    compute_args_hash(&[prompt])
}

/// SPEC-109: include effective reasoning effort when set so effort changes bust cache.
pub fn hash_query_prompt_with_effort(prompt: &str, reasoning_effort: Option<&str>) -> String {
    hash_query_prompt_with_effort_scoped(prompt, reasoning_effort, None)
}

/// SPEC-146: answer cache hash includes authz scope when present.
pub fn hash_query_prompt_with_effort_scoped(
    prompt: &str,
    reasoning_effort: Option<&str>,
    authz: Option<&AuthzCacheScope>,
) -> String {
    let effort = reasoning_effort
        .map(str::trim)
        .filter(|s| !s.is_empty());
    match (effort, authz) {
        (None, None) => hash_query_prompt(prompt),
        (Some(e), None) => compute_args_hash(&[prompt, e]),
        (None, Some(scope)) => {
            let mut gen = String::new();
            let mut parts: Vec<&str> = vec![prompt];
            scope.append_to_hash_parts(&mut parts, &mut gen);
            compute_args_hash(&parts)
        }
        (Some(e), Some(scope)) => {
            let mut gen = String::new();
            let mut parts: Vec<&str> = vec![prompt, e];
            scope.append_to_hash_parts(&mut parts, &mut gen);
            compute_args_hash(&parts)
        }
    }
}

/// Legacy helper used by answer_cache module — SHA of prompt.
pub fn answer_cache_key(prompt: &str) -> String {
    llm_cache_storage_key("mix", LlmCacheType::Query, &hash_query_prompt(prompt))
}

/// Unified LLM response cache port (DIP).
#[async_trait]
pub trait LlmResponseCache: Send + Sync {
    async fn get_return(&self, key: &str) -> Option<String>;
    async fn set_return(
        &self,
        key: &str,
        cache_type: LlmCacheType,
        value: &str,
        original_prompt: Option<&str>,
    );
    fn clear_l1(&self);
}

fn envelope(cache_type: LlmCacheType, value: &str, original_prompt: Option<&str>) -> Value {
    let mut obj = json!({
        "return": value,
        "cache_type": cache_type.as_str(),
    });
    if let Some(p) = original_prompt {
        obj.as_object_mut()
            .unwrap()
            .insert("original_prompt".into(), json!(p));
    }
    obj
}

fn parse_return(v: &Value) -> Option<String> {
    v.get("return").and_then(|x| x.as_str()).map(str::to_string)
}

/// In-process L1 LRU+TTL.
pub struct MemoryLlmResponseCache {
    max_size: usize,
    ttl: Duration,
    cache: RwLock<HashMap<String, (String, Instant, Instant)>>,
}

impl MemoryLlmResponseCache {
    pub fn new(max_size: usize, ttl: Duration) -> Self {
        Self {
            max_size,
            ttl,
            cache: RwLock::new(HashMap::new()),
        }
    }

    pub fn with_defaults() -> Self {
        Self::new(1_000, Duration::from_secs(3600))
    }
}

#[async_trait]
impl LlmResponseCache for MemoryLlmResponseCache {
    async fn get_return(&self, key: &str) -> Option<String> {
        let now = Instant::now();
        let mut cache = self.cache.write().unwrap();
        if let Some((val, expires, accessed)) = cache.get_mut(key) {
            if *expires > now {
                *accessed = now;
                return Some(val.clone());
            }
            cache.remove(key);
        }
        None
    }

    async fn set_return(
        &self,
        key: &str,
        _cache_type: LlmCacheType,
        value: &str,
        _original_prompt: Option<&str>,
    ) {
        let now = Instant::now();
        let mut cache = self.cache.write().unwrap();
        cache.retain(|_, (_, exp, _)| *exp > now);
        while cache.len() >= self.max_size {
            let oldest = cache
                .iter()
                .min_by_key(|(_, (_, _, a))| *a)
                .map(|(k, _)| k.clone());
            match oldest {
                Some(k) => {
                    cache.remove(&k);
                }
                None => break,
            }
        }
        cache.insert(key.to_string(), (value.to_string(), now + self.ttl, now));
    }

    fn clear_l1(&self) {
        self.cache.write().unwrap().clear();
    }
}

/// L2 via KV / typed `public.llm_cache`.
pub struct KvLlmResponseCache {
    kv: Arc<dyn KVStorage>,
}

impl KvLlmResponseCache {
    pub fn new(kv: Arc<dyn KVStorage>) -> Self {
        Self { kv }
    }
}

#[async_trait]
impl LlmResponseCache for KvLlmResponseCache {
    async fn get_return(&self, key: &str) -> Option<String> {
        match self.kv.get_by_id(key).await {
            Ok(Some(v)) => parse_return(&v),
            Ok(None) => None,
            Err(e) => {
                tracing::warn!(error = %e, key = %key, "SPEC-103 L2 cache get failed");
                None
            }
        }
    }

    async fn set_return(
        &self,
        key: &str,
        cache_type: LlmCacheType,
        value: &str,
        original_prompt: Option<&str>,
    ) {
        let entry = envelope(cache_type, value, original_prompt);
        if let Err(e) = self.kv.upsert(&[(key.to_string(), entry)]).await {
            tracing::warn!(error = %e, key = %key, "SPEC-103 L2 cache set failed");
        }
    }

    fn clear_l1(&self) {}
}

/// L1 then L2 (LAW-C5).
pub struct TieredLlmResponseCache {
    l1: MemoryLlmResponseCache,
    l2: KvLlmResponseCache,
}

impl TieredLlmResponseCache {
    pub fn new(l1: MemoryLlmResponseCache, l2: KvLlmResponseCache) -> Self {
        Self { l1, l2 }
    }

    pub fn with_kv(kv: Arc<dyn KVStorage>) -> Self {
        Self::new(
            MemoryLlmResponseCache::with_defaults(),
            KvLlmResponseCache::new(kv),
        )
    }
}

#[async_trait]
impl LlmResponseCache for TieredLlmResponseCache {
    async fn get_return(&self, key: &str) -> Option<String> {
        if let Some(v) = self.l1.get_return(key).await {
            return Some(v);
        }
        if let Some(v) = self.l2.get_return(key).await {
            self.l1.set_return(key, LlmCacheType::Query, &v, None).await;
            return Some(v);
        }
        None
    }

    async fn set_return(
        &self,
        key: &str,
        cache_type: LlmCacheType,
        value: &str,
        original_prompt: Option<&str>,
    ) {
        self.l1
            .set_return(key, cache_type, value, original_prompt)
            .await;
        self.l2
            .set_return(key, cache_type, value, original_prompt)
            .await;
    }

    fn clear_l1(&self) {
        self.l1.clear_l1();
    }
}

pub type SharedLlmResponseCache = Arc<dyn LlmResponseCache>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn key_format_and_hash_pins() {
        let h = hash_keyword_args("What is BRCA1?", "default", "mistral-small-latest", None);
        let key = llm_cache_storage_key("default", LlmCacheType::Keywords, &h);
        assert!(key.ends_with("-cache"), "{key}");
        assert!(key.contains(":keywords:"), "{key}");
        assert_ne!(
            hash_keyword_args("a", "default", "m1", None),
            hash_keyword_args("a", "default", "m2", None)
        );
        // Adjacent-field collision guard
        assert_ne!(
            compute_args_hash(&["ab", "c"]),
            compute_args_hash(&["a", "bc"])
        );
        let qk = answer_cache_key("full prompt with context");
        assert!(qk.contains(":query:"));
        assert!(qk.ends_with("-cache"));
    }

    #[test]
    fn hash_query_prompt_includes_effort_when_set() {
        let base = hash_query_prompt("prompt");
        assert_eq!(hash_query_prompt_with_effort("prompt", None), base);
        assert_eq!(hash_query_prompt_with_effort("prompt", Some("")), base);
        assert_ne!(
            hash_query_prompt_with_effort("prompt", Some("low")),
            hash_query_prompt_with_effort("prompt", Some("high"))
        );
        assert_ne!(hash_query_prompt_with_effort("prompt", Some("low")), base);
    }

    #[test]
    fn master_off_disables_answer_and_keywords() {
        let _g = ENV_LOCK.lock().unwrap();
        std::env::set_var("EDGEQUAKE_LLM_CACHE", "0");
        std::env::remove_var("EDGEQUAKE_QUERY_ANSWER_CACHE");
        std::env::remove_var("EDGEQUAKE_KEYWORD_CACHE");
        let f = resolve_llm_cache_flags();
        assert!(!f.master && !f.keywords && !f.answer);
        std::env::remove_var("EDGEQUAKE_LLM_CACHE");
    }

    #[test]
    fn master_on_answer_follows_unless_override() {
        let _g = ENV_LOCK.lock().unwrap();
        std::env::set_var("EDGEQUAKE_LLM_CACHE", "1");
        std::env::remove_var("EDGEQUAKE_QUERY_ANSWER_CACHE");
        assert!(resolve_llm_cache_flags().answer);
        std::env::set_var("EDGEQUAKE_QUERY_ANSWER_CACHE", "0");
        assert!(!resolve_llm_cache_flags().answer);
        std::env::remove_var("EDGEQUAKE_LLM_CACHE");
        std::env::remove_var("EDGEQUAKE_QUERY_ANSWER_CACHE");
    }

    #[tokio::test]
    async fn memory_round_trip() {
        let c = MemoryLlmResponseCache::with_defaults();
        let key = llm_cache_storage_key("mix", LlmCacheType::Query, &hash_query_prompt("p"));
        assert!(c.get_return(&key).await.is_none());
        c.set_return(&key, LlmCacheType::Query, "ans", Some("p"))
            .await;
        assert_eq!(c.get_return(&key).await.as_deref(), Some("ans"));
        c.clear_l1();
        assert!(c.get_return(&key).await.is_none());
    }

    static ENV_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
}
