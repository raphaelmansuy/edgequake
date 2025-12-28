# Implementation Proposal: Streaming Improvements

## Executive Summary

This document proposes a concrete implementation plan for improving EdgeQuake's streaming chat generation to match or exceed open-webui's quality. The proposal is grounded in evidence from the codebase analysis and includes precise code changes with line references.

**Honest Assessment**: The current implementation is functional but has critical gaps:

1. **Token Counting**: Line 602-613 in [chat.rs](../edgequake/crates/edgequake-api/src/handlers/chat.rs) - `tokens_used += 1` counts chunks, NOT tokens
2. **No Crash Recovery**: If server crashes during streaming, ALL content is lost (save only happens at line 640-660)
3. **No Caching**: Every conversation fetch hits PostgreSQL
4. **Missing API Metadata**: finish_reason, model_used, response_id not captured

The proposed changes are **pragmatic and incremental** - we won't rebuild everything, just add the missing pieces.

---

## 1. Implementation Order

| Phase | Component          | Effort  | Risk   | Impact                      |
| ----- | ------------------ | ------- | ------ | --------------------------- |
| 1     | StreamAccumulator  | 2 hours | Low    | High - Fixes token counting |
| 2     | TtlLruCache        | 3 hours | Low    | Medium - Improves latency   |
| 3     | StreamFlushManager | 4 hours | Medium | Medium - Crash recovery     |
| 4     | Database Migration | 1 hour  | Low    | High - Enables metadata     |

**Total Estimated Effort**: 10 hours

---

## 2. Phase 1: StreamAccumulator

### 2.1 Problem Statement

Current code in [chat.rs, lines 602-613](../edgequake/crates/edgequake-api/src/handlers/chat.rs#L602-L613):

```rust
while let Some(chunk_result) = stream.next().await {
    match chunk_result {
        Ok(text) => {
            full_content.push_str(&text);
            tokens_used += 1;  // ❌ WRONG: counts chunks, not tokens
            // ...
        }
    }
}
```

This is **fundamentally incorrect**. The LLM API returns chunks of variable size (could be partial words, whole sentences, or multi-word fragments). Counting chunks as tokens gives wildly inaccurate numbers.

### 2.2 Solution: StreamAccumulator

Create a new module at `/edgequake/crates/edgequake-api/src/streaming/mod.rs`:

```rust
//! Streaming utilities for chat completion.
//!
//! This module provides utilities for accumulating streaming responses,
//! tracking token usage accurately, and managing API response metadata.

pub mod accumulator;
pub mod flush_manager;

pub use accumulator::StreamAccumulator;
pub use flush_manager::StreamFlushManager;
```

**File: `/edgequake/crates/edgequake-api/src/streaming/accumulator.rs`**

```rust
//! Stream accumulator for collecting streaming response chunks.

use serde::{Deserialize, Serialize};
use std::time::Instant;

/// Metadata extracted from the final API response.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ApiResponseMetadata {
    /// The model that generated the response
    pub model: Option<String>,

    /// API response ID (e.g., chatcmpl-xxx)
    pub response_id: Option<String>,

    /// Reason the generation stopped
    pub finish_reason: Option<String>,

    /// Token usage from the API
    pub usage: Option<TokenUsage>,
}

/// Token usage statistics from the API.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TokenUsage {
    /// Tokens in the prompt
    pub prompt_tokens: u32,

    /// Tokens generated in completion
    pub completion_tokens: u32,

    /// Total tokens (prompt + completion)
    pub total_tokens: u32,

    /// Reasoning tokens (for o1/o3 models)
    pub reasoning_tokens: Option<u32>,
}

/// Accumulates streaming response chunks with proper tracking.
///
/// This struct properly tracks content, metadata, and timing information
/// during streaming, rather than incorrectly counting chunks as tokens.
#[derive(Debug)]
pub struct StreamAccumulator {
    /// Accumulated content from all chunks
    content: String,

    /// Number of chunks received
    chunk_count: u32,

    /// Estimated character count (for progress)
    char_count: u32,

    /// Start time for duration tracking
    start_time: Instant,

    /// First chunk timestamp (for TTFT - time to first token)
    first_chunk_time: Option<Instant>,

    /// API response metadata (populated from final chunk if available)
    metadata: ApiResponseMetadata,

    /// Whether streaming has completed
    is_complete: bool,
}

impl StreamAccumulator {
    /// Create a new accumulator.
    pub fn new() -> Self {
        Self {
            content: String::with_capacity(4096), // Pre-allocate for typical response
            chunk_count: 0,
            char_count: 0,
            start_time: Instant::now(),
            first_chunk_time: None,
            metadata: ApiResponseMetadata::default(),
            is_complete: false,
        }
    }

    /// Append a content chunk.
    pub fn append_content(&mut self, chunk: &str) {
        if self.first_chunk_time.is_none() {
            self.first_chunk_time = Some(Instant::now());
        }

        self.content.push_str(chunk);
        self.chunk_count += 1;
        self.char_count += chunk.len() as u32;
    }

    /// Set metadata from the API response.
    ///
    /// This should be called when the API provides usage information,
    /// typically in the final chunk or a separate usage chunk.
    pub fn set_metadata(&mut self, metadata: ApiResponseMetadata) {
        self.metadata = metadata;
    }

    /// Update token usage from API response.
    pub fn set_usage(&mut self, usage: TokenUsage) {
        self.metadata.usage = Some(usage);
    }

    /// Mark streaming as complete.
    pub fn complete(&mut self, finish_reason: Option<String>) {
        self.is_complete = true;
        if finish_reason.is_some() {
            self.metadata.finish_reason = finish_reason;
        }
    }

    /// Get the accumulated content.
    pub fn content(&self) -> &str {
        &self.content
    }

    /// Get the owned content (consumes accumulator).
    pub fn into_content(self) -> String {
        self.content
    }

    /// Get the duration since start.
    pub fn duration_ms(&self) -> u64 {
        self.start_time.elapsed().as_millis() as u64
    }

    /// Get time to first token (if received).
    pub fn ttft_ms(&self) -> Option<u64> {
        self.first_chunk_time.map(|t| {
            t.duration_since(self.start_time).as_millis() as u64
        })
    }

    /// Get chunk count (NOT token count).
    pub fn chunk_count(&self) -> u32 {
        self.chunk_count
    }

    /// Estimate token count from content.
    ///
    /// Uses a simple heuristic: ~4 characters per token for English text.
    /// This is reasonably accurate for GPT models.
    ///
    /// For accurate token counts, use the API-provided usage when available.
    pub fn estimated_tokens(&self) -> u32 {
        // Prefer API-provided usage
        if let Some(ref usage) = self.metadata.usage {
            return usage.completion_tokens;
        }

        // Fallback: estimate ~4 chars per token (English average)
        (self.char_count / 4).max(1)
    }

    /// Get actual tokens from API usage (if available).
    pub fn actual_tokens(&self) -> Option<u32> {
        self.metadata.usage.as_ref().map(|u| u.completion_tokens)
    }

    /// Get the metadata.
    pub fn metadata(&self) -> &ApiResponseMetadata {
        &self.metadata
    }

    /// Get current content length.
    pub fn content_len(&self) -> usize {
        self.content.len()
    }

    /// Check if streaming is complete.
    pub fn is_complete(&self) -> bool {
        self.is_complete
    }
}

impl Default for StreamAccumulator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_accumulator_basic() {
        let mut acc = StreamAccumulator::new();

        acc.append_content("Hello ");
        acc.append_content("world!");

        assert_eq!(acc.content(), "Hello world!");
        assert_eq!(acc.chunk_count(), 2);
        assert_eq!(acc.char_count, 12);
    }

    #[test]
    fn test_estimated_tokens() {
        let mut acc = StreamAccumulator::new();

        // 100 characters ≈ 25 tokens (4 chars per token)
        acc.append_content(&"a".repeat(100));

        assert_eq!(acc.estimated_tokens(), 25);
    }

    #[test]
    fn test_actual_tokens_preferred() {
        let mut acc = StreamAccumulator::new();

        acc.append_content(&"a".repeat(100)); // Would estimate 25 tokens

        acc.set_usage(TokenUsage {
            prompt_tokens: 10,
            completion_tokens: 50, // Actual count
            total_tokens: 60,
            reasoning_tokens: None,
        });

        // Should use actual count, not estimate
        assert_eq!(acc.estimated_tokens(), 50);
        assert_eq!(acc.actual_tokens(), Some(50));
    }

    #[test]
    fn test_ttft_tracking() {
        let mut acc = StreamAccumulator::new();

        assert!(acc.ttft_ms().is_none());

        std::thread::sleep(std::time::Duration::from_millis(10));
        acc.append_content("First");

        let ttft = acc.ttft_ms().unwrap();
        assert!(ttft >= 10);
    }
}
```

### 2.3 Integration Changes

**Modify `/edgequake/crates/edgequake-api/src/handlers/chat.rs`**:

1. Add import at top of file:

```rust
use crate::streaming::StreamAccumulator;
```

2. Replace lines 594-614:

```rust
// BEFORE (incorrect):
let mut full_content = String::new();
let mut tokens_used = 0u32;
// ...
full_content.push_str(&text);
tokens_used += 1;

// AFTER (correct):
let mut accumulator = StreamAccumulator::new();
// ...
accumulator.append_content(&text);
```

3. Replace lines 680-690:

```rust
// BEFORE:
tokens_used: Some(tokens_used as i32),
duration_ms: Some(duration_ms as i32),

// AFTER:
tokens_used: Some(accumulator.estimated_tokens() as i32),
duration_ms: Some(accumulator.duration_ms() as i32),
```

---

## 3. Phase 2: TtlLruCache

### 3.1 Problem Statement

Every conversation and message fetch goes directly to PostgreSQL. For active conversations being displayed in the UI, this creates unnecessary load.

### 3.2 Solution: Generic TtlLruCache

**File: `/edgequake/crates/edgequake-core/src/cache.rs`**

````rust
//! Thread-safe LRU cache with TTL expiration.
//!
//! This module provides a generic cache implementation that combines
//! LRU eviction with time-based expiration.

use lru::LruCache;
use std::hash::Hash;
use std::num::NonZeroUsize;
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};

/// A cached value with expiration timestamp.
#[derive(Clone)]
struct CacheEntry<V> {
    value: V,
    expires_at: Instant,
}

/// Thread-safe LRU cache with TTL expiration.
///
/// # Type Parameters
///
/// - `K`: Key type (must be Clone + Eq + Hash)
/// - `V`: Value type (must be Clone)
///
/// # Example
///
/// ```rust
/// use edgequake_core::cache::TtlLruCache;
/// use std::time::Duration;
///
/// let cache: TtlLruCache<String, String> = TtlLruCache::new(
///     100,                      // Max 100 entries
///     Duration::from_secs(300), // 5 minute TTL
/// );
///
/// cache.put("key".to_string(), "value".to_string());
/// assert_eq!(cache.get(&"key".to_string()), Some("value".to_string()));
/// ```
#[derive(Clone)]
pub struct TtlLruCache<K, V>
where
    K: Clone + Eq + Hash,
    V: Clone,
{
    inner: Arc<RwLock<LruCache<K, CacheEntry<V>>>>,
    ttl: Duration,

    // Metrics (optional, for monitoring)
    hits: Arc<std::sync::atomic::AtomicU64>,
    misses: Arc<std::sync::atomic::AtomicU64>,
}

impl<K, V> TtlLruCache<K, V>
where
    K: Clone + Eq + Hash,
    V: Clone,
{
    /// Create a new cache with specified capacity and TTL.
    ///
    /// # Arguments
    ///
    /// - `capacity`: Maximum number of entries
    /// - `ttl`: Time-to-live for entries
    ///
    /// # Panics
    ///
    /// Panics if capacity is 0.
    pub fn new(capacity: usize, ttl: Duration) -> Self {
        let cap = NonZeroUsize::new(capacity).expect("capacity must be > 0");
        Self {
            inner: Arc::new(RwLock::new(LruCache::new(cap))),
            ttl,
            hits: Arc::new(std::sync::atomic::AtomicU64::new(0)),
            misses: Arc::new(std::sync::atomic::AtomicU64::new(0)),
        }
    }

    /// Get a value from the cache.
    ///
    /// Returns `None` if:
    /// - Key doesn't exist
    /// - Entry has expired (automatically removed)
    ///
    /// If entry exists and is valid, promotes it to most-recently-used.
    pub fn get(&self, key: &K) -> Option<V> {
        let mut cache = self.inner.write().ok()?;

        if let Some(entry) = cache.get(key) {
            if entry.expires_at > Instant::now() {
                self.hits.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                return Some(entry.value.clone());
            }
            // Entry expired - remove it
            cache.pop(key);
        }

        self.misses.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        None
    }

    /// Get a value without updating LRU order.
    ///
    /// Useful for checking existence without affecting eviction priority.
    pub fn peek(&self, key: &K) -> Option<V> {
        let cache = self.inner.read().ok()?;

        if let Some(entry) = cache.peek(key) {
            if entry.expires_at > Instant::now() {
                return Some(entry.value.clone());
            }
        }

        None
    }

    /// Insert a value into the cache.
    ///
    /// If the cache is at capacity, evicts the least-recently-used entry.
    /// Returns the previous value if key existed.
    pub fn put(&self, key: K, value: V) -> Option<V> {
        let mut cache = self.inner.write().ok()?;

        let entry = CacheEntry {
            value,
            expires_at: Instant::now() + self.ttl,
        };

        cache.put(key, entry).map(|e| e.value)
    }

    /// Remove a value from the cache.
    ///
    /// Returns the removed value if it existed.
    pub fn remove(&self, key: &K) -> Option<V> {
        let mut cache = self.inner.write().ok()?;
        cache.pop(key).map(|e| e.value)
    }

    /// Invalidate (remove) an entry from the cache.
    ///
    /// Alias for `remove` with clearer intent.
    pub fn invalidate(&self, key: &K) {
        let _ = self.remove(key);
    }

    /// Get current cache size.
    pub fn len(&self) -> usize {
        self.inner.read().map(|c| c.len()).unwrap_or(0)
    }

    /// Check if cache is empty.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Clear all entries from the cache.
    pub fn clear(&self) {
        if let Ok(mut cache) = self.inner.write() {
            cache.clear();
        }
    }

    /// Get cache hit rate as a percentage.
    pub fn hit_rate(&self) -> f64 {
        let hits = self.hits.load(std::sync::atomic::Ordering::Relaxed);
        let misses = self.misses.load(std::sync::atomic::Ordering::Relaxed);
        let total = hits + misses;

        if total == 0 {
            0.0
        } else {
            (hits as f64 / total as f64) * 100.0
        }
    }

    /// Get cache statistics.
    pub fn stats(&self) -> CacheStats {
        CacheStats {
            size: self.len(),
            hits: self.hits.load(std::sync::atomic::Ordering::Relaxed),
            misses: self.misses.load(std::sync::atomic::Ordering::Relaxed),
            hit_rate: self.hit_rate(),
        }
    }

    /// Remove all expired entries.
    ///
    /// This is O(n) and should be called periodically, not on every access.
    pub fn purge_expired(&self) {
        if let Ok(mut cache) = self.inner.write() {
            let now = Instant::now();
            let expired: Vec<K> = cache
                .iter()
                .filter(|(_, entry)| entry.expires_at <= now)
                .map(|(k, _)| k.clone())
                .collect();

            for key in expired {
                cache.pop(&key);
            }
        }
    }
}

/// Cache statistics.
#[derive(Debug, Clone)]
pub struct CacheStats {
    pub size: usize,
    pub hits: u64,
    pub misses: u64,
    pub hit_rate: f64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_operations() {
        let cache: TtlLruCache<String, i32> = TtlLruCache::new(
            10,
            Duration::from_secs(60),
        );

        cache.put("a".to_string(), 1);
        cache.put("b".to_string(), 2);

        assert_eq!(cache.get(&"a".to_string()), Some(1));
        assert_eq!(cache.get(&"b".to_string()), Some(2));
        assert_eq!(cache.get(&"c".to_string()), None);
    }

    #[test]
    fn test_lru_eviction() {
        let cache: TtlLruCache<String, i32> = TtlLruCache::new(
            2, // Only 2 entries
            Duration::from_secs(60),
        );

        cache.put("a".to_string(), 1);
        cache.put("b".to_string(), 2);

        // Access "a" to make it recently used
        cache.get(&"a".to_string());

        // Add "c" - should evict "b" (least recently used)
        cache.put("c".to_string(), 3);

        assert_eq!(cache.get(&"a".to_string()), Some(1));
        assert_eq!(cache.get(&"c".to_string()), Some(3));
        assert_eq!(cache.get(&"b".to_string()), None); // Evicted
    }

    #[test]
    fn test_ttl_expiration() {
        let cache: TtlLruCache<String, i32> = TtlLruCache::new(
            10,
            Duration::from_millis(50), // Very short TTL
        );

        cache.put("a".to_string(), 1);
        assert_eq!(cache.get(&"a".to_string()), Some(1));

        // Wait for expiration
        std::thread::sleep(Duration::from_millis(60));

        assert_eq!(cache.get(&"a".to_string()), None); // Expired
    }

    #[test]
    fn test_invalidation() {
        let cache: TtlLruCache<String, i32> = TtlLruCache::new(
            10,
            Duration::from_secs(60),
        );

        cache.put("a".to_string(), 1);
        assert_eq!(cache.get(&"a".to_string()), Some(1));

        cache.invalidate(&"a".to_string());
        assert_eq!(cache.get(&"a".to_string()), None);
    }

    #[test]
    fn test_hit_rate() {
        let cache: TtlLruCache<String, i32> = TtlLruCache::new(
            10,
            Duration::from_secs(60),
        );

        cache.put("a".to_string(), 1);

        // 2 hits
        cache.get(&"a".to_string());
        cache.get(&"a".to_string());

        // 1 miss
        cache.get(&"b".to_string());

        // 2/3 = 66.67%
        let stats = cache.stats();
        assert_eq!(stats.hits, 2);
        assert_eq!(stats.misses, 1);
        assert!((stats.hit_rate - 66.67).abs() < 1.0);
    }
}
````

### 3.3 CacheManager

**File: `/edgequake/crates/edgequake-api/src/cache_manager.rs`**

```rust
//! Centralized cache manager for API layer.

use edgequake_core::cache::TtlLruCache;
use edgequake_core::types::{Conversation, Message};
use std::time::Duration;
use uuid::Uuid;

/// Configuration for cache manager.
#[derive(Debug, Clone)]
pub struct CacheConfig {
    /// Conversation cache capacity
    pub conversation_capacity: usize,

    /// Conversation TTL
    pub conversation_ttl: Duration,

    /// Message list cache capacity
    pub message_list_capacity: usize,

    /// Message list TTL
    pub message_list_ttl: Duration,
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            conversation_capacity: 1000,
            conversation_ttl: Duration::from_secs(300), // 5 minutes
            message_list_capacity: 500,
            message_list_ttl: Duration::from_secs(60), // 1 minute
        }
    }
}

/// Centralized cache manager for conversations and messages.
#[derive(Clone)]
pub struct CacheManager {
    conversations: TtlLruCache<Uuid, Conversation>,
    message_lists: TtlLruCache<Uuid, Vec<Message>>,
}

impl CacheManager {
    /// Create a new cache manager with configuration.
    pub fn new(config: CacheConfig) -> Self {
        Self {
            conversations: TtlLruCache::new(
                config.conversation_capacity,
                config.conversation_ttl,
            ),
            message_lists: TtlLruCache::new(
                config.message_list_capacity,
                config.message_list_ttl,
            ),
        }
    }

    /// Create a new cache manager with default configuration.
    pub fn default_config() -> Self {
        Self::new(CacheConfig::default())
    }

    // ========== Conversation Cache ==========

    /// Get a conversation from cache.
    pub fn get_conversation(&self, id: Uuid) -> Option<Conversation> {
        self.conversations.get(&id)
    }

    /// Cache a conversation.
    pub fn cache_conversation(&self, conversation: Conversation) {
        self.conversations.put(conversation.conversation_id, conversation);
    }

    /// Invalidate a conversation cache entry.
    pub fn invalidate_conversation(&self, id: Uuid) {
        self.conversations.invalidate(&id);
        // Also invalidate related message list
        self.message_lists.invalidate(&id);
    }

    // ========== Message List Cache ==========

    /// Get messages for a conversation from cache.
    pub fn get_messages(&self, conversation_id: Uuid) -> Option<Vec<Message>> {
        self.message_lists.get(&conversation_id)
    }

    /// Cache messages for a conversation.
    pub fn cache_messages(&self, conversation_id: Uuid, messages: Vec<Message>) {
        self.message_lists.put(conversation_id, messages);
    }

    /// Invalidate message list cache for a conversation.
    pub fn invalidate_messages(&self, conversation_id: Uuid) {
        self.message_lists.invalidate(&conversation_id);
    }

    // ========== Utilities ==========

    /// Get cache statistics for monitoring.
    pub fn stats(&self) -> CacheManagerStats {
        CacheManagerStats {
            conversation_stats: self.conversations.stats(),
            message_list_stats: self.message_lists.stats(),
        }
    }

    /// Clear all caches.
    pub fn clear(&self) {
        self.conversations.clear();
        self.message_lists.clear();
    }
}

/// Combined cache statistics.
#[derive(Debug, Clone)]
pub struct CacheManagerStats {
    pub conversation_stats: edgequake_core::cache::CacheStats,
    pub message_list_stats: edgequake_core::cache::CacheStats,
}
```

### 3.4 Integration

Add `CacheManager` to `AppState` in `/edgequake/crates/edgequake-api/src/state.rs`:

```rust
pub struct AppState {
    // ... existing fields ...
    pub cache_manager: CacheManager,
}
```

---

## 4. Phase 3: StreamFlushManager (Debounce)

### 4.1 Problem Statement

Current flow saves ONLY at the end of streaming. If server crashes during a long response, ALL content is lost.

### 4.2 Solution: Debounced Checkpoint Saves

**File: `/edgequake/crates/edgequake-api/src/streaming/flush_manager.rs`**

```rust
//! Debounced flush manager for streaming responses.
//!
//! This module provides trailing-edge debouncing for database writes
//! during streaming, ensuring crash recovery while minimizing DB load.

use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{mpsc, Mutex};
use tokio::time::sleep;
use tracing::{debug, error, warn};
use uuid::Uuid;

/// Configuration for flush manager.
#[derive(Debug, Clone)]
pub struct FlushConfig {
    /// Delay after last chunk before flushing
    pub write_delay: Duration,

    /// Maximum time between flushes
    pub max_buffer_time: Duration,

    /// Maximum bytes before forced flush
    pub max_buffer_bytes: usize,
}

impl Default for FlushConfig {
    fn default() -> Self {
        Self {
            write_delay: Duration::from_millis(500),
            max_buffer_time: Duration::from_secs(2),
            max_buffer_bytes: 8192,
        }
    }
}

/// Message to the flush background task.
#[derive(Debug)]
enum FlushMessage {
    /// Content was updated
    ContentUpdated { content: String, tokens: u32 },

    /// Stream completed normally
    Complete,

    /// Stream aborted (client disconnect, error, etc.)
    Abort,
}

/// Handle for interacting with a running flush manager.
#[derive(Clone)]
pub struct FlushHandle {
    tx: mpsc::Sender<FlushMessage>,
}

impl FlushHandle {
    /// Notify that content was updated.
    pub async fn content_updated(&self, content: String, tokens: u32) {
        let _ = self.tx.send(FlushMessage::ContentUpdated { content, tokens }).await;
    }

    /// Signal stream completion.
    pub async fn complete(&self) {
        let _ = self.tx.send(FlushMessage::Complete).await;
    }

    /// Signal stream abort.
    pub async fn abort(&self) {
        let _ = self.tx.send(FlushMessage::Abort).await;
    }
}

/// Manages debounced flushes for a single streaming response.
pub struct StreamFlushManager<F>
where
    F: Fn(String, u32) -> futures::future::BoxFuture<'static, Result<(), String>> + Send + Sync + 'static,
{
    config: FlushConfig,
    message_id: Uuid,
    save_fn: Arc<F>,
}

impl<F> StreamFlushManager<F>
where
    F: Fn(String, u32) -> futures::future::BoxFuture<'static, Result<(), String>> + Send + Sync + 'static,
{
    /// Create a new flush manager.
    pub fn new(message_id: Uuid, config: FlushConfig, save_fn: F) -> Self {
        Self {
            config,
            message_id,
            save_fn: Arc::new(save_fn),
        }
    }

    /// Start the flush manager background task.
    ///
    /// Returns a handle for sending updates and a join handle for the task.
    pub fn start(self) -> FlushHandle {
        let (tx, mut rx) = mpsc::channel::<FlushMessage>(100);

        let config = self.config;
        let message_id = self.message_id;
        let save_fn = self.save_fn;

        tokio::spawn(async move {
            let mut last_flush = Instant::now();
            let mut pending_content: Option<(String, u32)> = None;
            let mut pending_flush_task: Option<tokio::task::JoinHandle<()>> = None;

            loop {
                let timeout = config.write_delay;

                tokio::select! {
                    msg = rx.recv() => {
                        match msg {
                            Some(FlushMessage::ContentUpdated { content, tokens }) => {
                                // Cancel any pending flush
                                if let Some(task) = pending_flush_task.take() {
                                    task.abort();
                                }

                                let should_force_flush =
                                    content.len() >= config.max_buffer_bytes ||
                                    last_flush.elapsed() >= config.max_buffer_time;

                                if should_force_flush {
                                    // Immediate flush
                                    let save_fn_clone = save_fn.clone();
                                    match save_fn_clone(content.clone(), tokens).await {
                                        Ok(()) => {
                                            last_flush = Instant::now();
                                            debug!(message_id = %message_id, "Forced flush completed");
                                        }
                                        Err(e) => {
                                            error!(message_id = %message_id, error = %e, "Flush failed");
                                        }
                                    }
                                    pending_content = None;
                                } else {
                                    // Schedule delayed flush
                                    pending_content = Some((content, tokens));
                                }
                            }
                            Some(FlushMessage::Complete) => {
                                // Final flush with whatever we have
                                if let Some((content, tokens)) = pending_content.take() {
                                    let save_fn_clone = save_fn.clone();
                                    let _ = save_fn_clone(content, tokens).await;
                                }
                                debug!(message_id = %message_id, "Stream completed, final flush done");
                                break;
                            }
                            Some(FlushMessage::Abort) | None => {
                                // Save whatever we have before exiting
                                if let Some((content, tokens)) = pending_content.take() {
                                    let save_fn_clone = save_fn.clone();
                                    let _ = save_fn_clone(content, tokens).await;
                                    warn!(message_id = %message_id, "Stream aborted, saved partial content");
                                }
                                break;
                            }
                        }
                    }

                    _ = sleep(timeout), if pending_content.is_some() => {
                        // Debounce timeout reached - flush pending content
                        if let Some((content, tokens)) = pending_content.take() {
                            let save_fn_clone = save_fn.clone();
                            match save_fn_clone(content, tokens).await {
                                Ok(()) => {
                                    last_flush = Instant::now();
                                    debug!(message_id = %message_id, "Debounced flush completed");
                                }
                                Err(e) => {
                                    error!(message_id = %message_id, error = %e, "Debounced flush failed");
                                }
                            }
                        }
                    }
                }
            }
        });

        FlushHandle { tx }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU32, Ordering};

    #[tokio::test]
    async fn test_debounce_coalesces_writes() {
        let write_count = Arc::new(AtomicU32::new(0));
        let write_count_clone = write_count.clone();

        let save_fn = move |_content: String, _tokens: u32| {
            let count = write_count_clone.clone();
            Box::pin(async move {
                count.fetch_add(1, Ordering::SeqCst);
                Ok(())
            }) as futures::future::BoxFuture<'static, Result<(), String>>
        };

        let manager = StreamFlushManager::new(
            Uuid::new_v4(),
            FlushConfig {
                write_delay: Duration::from_millis(100),
                max_buffer_time: Duration::from_secs(10),
                max_buffer_bytes: 100000,
            },
            save_fn,
        );

        let handle = manager.start();

        // Send 10 rapid updates
        for i in 0..10 {
            handle.content_updated(format!("content_{}", i), i).await;
            sleep(Duration::from_millis(20)).await;
        }

        // Wait for debounce
        sleep(Duration::from_millis(200)).await;

        // Complete
        handle.complete().await;
        sleep(Duration::from_millis(50)).await;

        // Should have far fewer than 10 writes due to debouncing
        let writes = write_count.load(Ordering::SeqCst);
        assert!(writes < 5, "Expected < 5 writes due to debouncing, got {}", writes);
    }
}
```

---

## 5. Phase 4: Database Migration

### 5.1 New Migration File

**File: `/edgequake/migrations/YYYYMMDD_add_api_metadata.sql`**

```sql
-- Add API response metadata columns to messages table

ALTER TABLE messages
ADD COLUMN IF NOT EXISTS prompt_tokens INTEGER;

ALTER TABLE messages
ADD COLUMN IF NOT EXISTS completion_tokens INTEGER;

ALTER TABLE messages
ADD COLUMN IF NOT EXISTS reasoning_tokens INTEGER;

ALTER TABLE messages
ADD COLUMN IF NOT EXISTS model_used VARCHAR(100);

ALTER TABLE messages
ADD COLUMN IF NOT EXISTS response_id VARCHAR(100);

ALTER TABLE messages
ADD COLUMN IF NOT EXISTS finish_reason VARCHAR(50);

ALTER TABLE messages
ADD COLUMN IF NOT EXISTS ttft_ms INTEGER;

-- Add index for performance
CREATE INDEX IF NOT EXISTS idx_messages_model_used ON messages (model_used);

-- Add comment for documentation
COMMENT ON COLUMN messages.prompt_tokens IS 'Number of tokens in the prompt';
COMMENT ON COLUMN messages.completion_tokens IS 'Number of tokens in the completion';
COMMENT ON COLUMN messages.reasoning_tokens IS 'Reasoning tokens for o1/o3 models';
COMMENT ON COLUMN messages.model_used IS 'Model ID that generated this response';
COMMENT ON COLUMN messages.response_id IS 'API response ID (e.g., chatcmpl-xxx)';
COMMENT ON COLUMN messages.finish_reason IS 'Reason generation stopped (stop, length, etc.)';
COMMENT ON COLUMN messages.ttft_ms IS 'Time to first token in milliseconds';
```

---

## 6. Dependency Updates

**Modify `/edgequake/crates/edgequake-core/Cargo.toml`**:

```toml
[dependencies]
lru = "0.12"  # Add LRU cache support
```

**Modify `/edgequake/crates/edgequake-api/Cargo.toml`**:

```toml
[dependencies]
# ... existing deps ...
```

---

## 7. Implementation Checklist

### Phase 1: StreamAccumulator

- [ ] Create `/edgequake/crates/edgequake-api/src/streaming/mod.rs`
- [ ] Create `/edgequake/crates/edgequake-api/src/streaming/accumulator.rs`
- [ ] Add `mod streaming;` to `/edgequake/crates/edgequake-api/src/lib.rs`
- [ ] Modify `chat.rs` to use `StreamAccumulator`
- [ ] Run tests: `cargo test --package edgequake-api`

### Phase 2: TtlLruCache

- [ ] Add `lru = "0.12"` to `edgequake-core/Cargo.toml`
- [ ] Create `/edgequake/crates/edgequake-core/src/cache.rs`
- [ ] Add `pub mod cache;` to `edgequake-core/src/lib.rs`
- [ ] Create `/edgequake/crates/edgequake-api/src/cache_manager.rs`
- [ ] Add `CacheManager` to `AppState`
- [ ] Integrate cache into conversation handlers
- [ ] Run tests: `cargo test --package edgequake-core`

### Phase 3: StreamFlushManager

- [ ] Create `/edgequake/crates/edgequake-api/src/streaming/flush_manager.rs`
- [ ] Integrate `StreamFlushManager` into `chat_completion_stream`
- [ ] Run tests

### Phase 4: Database Migration

- [ ] Create migration file
- [ ] Run migration: `sqlx migrate run`
- [ ] Update Rust types for new columns
- [ ] Update PostgreSQL adapter

---

## 8. Risk Assessment

| Risk                             | Likelihood | Impact | Mitigation                                         |
| -------------------------------- | ---------- | ------ | -------------------------------------------------- |
| LRU cache increases memory usage | Medium     | Low    | Monitor with metrics, tune capacity                |
| Debounce adds complexity         | Low        | Low    | Comprehensive tests, fallback to simple save       |
| Token estimation inaccurate      | Medium     | Low    | Use API usage when available, estimate is fallback |
| Migration breaks existing data   | Low        | High   | Add columns as nullable, no data loss              |

---

## 9. Honest Assessment

**What's Good:**

- The design is pragmatic and incremental
- Each phase can be delivered independently
- Tests are comprehensive
- Fallbacks exist for edge cases

**What's Concerning:**

- Token estimation (4 chars/token) is a heuristic, not accurate for all languages
- Debouncing adds complexity that may be overkill for current load
- Cache invalidation during updates needs careful handling

**Recommendation**: Proceed with Phase 1 and 2 immediately. Phase 3 (debouncing) can be deferred until we observe actual crash recovery issues. Phase 4 is prerequisite for full Phase 1 benefits.

---

_Document Version: 1.0_
_Created: 2024-12-28_
