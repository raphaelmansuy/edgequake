# LRU Cache Design for Conversations and Generated Text

## 1. Problem Statement

EdgeQuake currently makes direct database queries for every conversation and message access. In high-traffic scenarios with active conversations, this causes:

- Repeated identical queries for hot data
- Database connection pool exhaustion
- Increased latency for frequent operations
- Unnecessary I/O for read-heavy workloads

## 2. Cache Topology

### 2.1 Three-Tier Cache Architecture

```
┌────────────────────────────────────────────────────────────────────┐
│                           CLIENT TIER                               │
│  ┌──────────────┐                                                  │
│  │ Browser/App  │  Local state, session storage                    │
│  └──────────────┘                                                  │
└────────────────────────────────────────────────────────────────────┘
                                    │
                                    ▼
┌────────────────────────────────────────────────────────────────────┐
│                          API SERVER TIER                            │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐             │
│  │ L1: In-Memory│  │ L2: Redis    │  │ L3: PostgreSQL│             │
│  │ LRU Cache    │  │ (Optional)   │  │ (Primary)    │             │
│  │ TTL: 60s     │  │ TTL: 5min    │  │ Persistent   │             │
│  │ Size: 1000   │  │ Size: 10000  │  │              │             │
│  └──────────────┘  └──────────────┘  └──────────────┘             │
└────────────────────────────────────────────────────────────────────┘
```

### 2.2 Cache Keys Structure

```
Conversation:     conv:{tenant_id}:{conversation_id}
Message:          msg:{message_id}
Message List:     msg_list:{conversation_id}:{cursor}:{limit}
User Convs:       user_convs:{tenant_id}:{user_id}:{filter_hash}
Generated Text:   gen:{message_id}  (streaming buffer)
```

## 3. Implementation Design

### 3.1 Core Cache Interface

```rust
use std::time::{Duration, Instant};
use lru::LruCache;
use std::num::NonZeroUsize;
use tokio::sync::RwLock;
use std::sync::Arc;

/// Cache entry with TTL support
#[derive(Clone)]
pub struct CacheEntry<T> {
    value: T,
    created_at: Instant,
    ttl: Duration,
}

impl<T: Clone> CacheEntry<T> {
    pub fn new(value: T, ttl: Duration) -> Self {
        Self {
            value,
            created_at: Instant::now(),
            ttl,
        }
    }

    pub fn is_expired(&self) -> bool {
        self.created_at.elapsed() > self.ttl
    }

    pub fn get(&self) -> Option<T> {
        if self.is_expired() {
            None
        } else {
            Some(self.value.clone())
        }
    }
}

/// Thread-safe LRU cache with TTL support
pub struct TtlLruCache<K, V>
where
    K: std::hash::Hash + Eq + Clone,
    V: Clone,
{
    cache: Arc<RwLock<LruCache<K, CacheEntry<V>>>>,
    default_ttl: Duration,
    max_size: usize,
}

impl<K, V> TtlLruCache<K, V>
where
    K: std::hash::Hash + Eq + Clone + Send + Sync + 'static,
    V: Clone + Send + Sync + 'static,
{
    pub fn new(max_size: usize, default_ttl: Duration) -> Self {
        Self {
            cache: Arc::new(RwLock::new(
                LruCache::new(NonZeroUsize::new(max_size).unwrap())
            )),
            default_ttl,
            max_size,
        }
    }

    /// Get value from cache
    pub async fn get(&self, key: &K) -> Option<V> {
        let mut cache = self.cache.write().await;

        if let Some(entry) = cache.get(key) {
            if entry.is_expired() {
                cache.pop(key);
                None
            } else {
                Some(entry.value.clone())
            }
        } else {
            None
        }
    }

    /// Put value into cache
    pub async fn put(&self, key: K, value: V) {
        self.put_with_ttl(key, value, self.default_ttl).await
    }

    /// Put value with custom TTL
    pub async fn put_with_ttl(&self, key: K, value: V, ttl: Duration) {
        let mut cache = self.cache.write().await;
        cache.put(key, CacheEntry::new(value, ttl));
    }

    /// Invalidate a key
    pub async fn invalidate(&self, key: &K) {
        let mut cache = self.cache.write().await;
        cache.pop(key);
    }

    /// Get or compute: fetch from cache or compute and cache
    pub async fn get_or_insert<F, Fut>(&self, key: K, f: F) -> Result<V, crate::error::Error>
    where
        F: FnOnce() -> Fut,
        Fut: std::future::Future<Output = Result<V, crate::error::Error>>,
    {
        // Try cache first
        if let Some(value) = self.get(&key).await {
            return Ok(value);
        }

        // Compute value
        let value = f().await?;

        // Cache and return
        self.put(key, value.clone()).await;
        Ok(value)
    }
}
```

### 3.2 Domain-Specific Cache Types

```rust
/// Conversation cache specialization
pub type ConversationCache = TtlLruCache<ConversationCacheKey, Conversation>;

#[derive(Hash, Eq, PartialEq, Clone)]
pub struct ConversationCacheKey {
    pub conversation_id: Uuid,
}

/// Message cache specialization
pub type MessageCache = TtlLruCache<MessageCacheKey, Message>;

#[derive(Hash, Eq, PartialEq, Clone)]
pub struct MessageCacheKey {
    pub message_id: Uuid,
}

/// Message list cache (for pagination)
pub type MessageListCache = TtlLruCache<MessageListCacheKey, Vec<Message>>;

#[derive(Hash, Eq, PartialEq, Clone)]
pub struct MessageListCacheKey {
    pub conversation_id: Uuid,
    pub cursor: Option<String>,
    pub limit: usize,
}

/// User conversation list cache
pub type UserConversationsCache = TtlLruCache<UserConvsCacheKey, Vec<ConversationSummary>>;

#[derive(Hash, Eq, PartialEq, Clone)]
pub struct UserConvsCacheKey {
    pub tenant_id: Uuid,
    pub user_id: Uuid,
    pub filter_hash: u64,  // Hash of ConversationFilter
}

/// Generated text buffer cache (for streaming)
pub type StreamingBufferCache = TtlLruCache<Uuid, StreamingMessageBuffer>;
```

### 3.3 Cache Manager

```rust
/// Central cache manager for all domain caches
pub struct CacheManager {
    /// Conversation cache
    pub conversations: ConversationCache,
    /// Message cache
    pub messages: MessageCache,
    /// Message list cache
    pub message_lists: MessageListCache,
    /// User conversations cache
    pub user_conversations: UserConversationsCache,
    /// Streaming buffer cache
    pub streaming_buffers: StreamingBufferCache,
    /// Statistics
    stats: Arc<RwLock<CacheStats>>,
}

#[derive(Default)]
pub struct CacheStats {
    pub conversation_hits: u64,
    pub conversation_misses: u64,
    pub message_hits: u64,
    pub message_misses: u64,
    pub list_hits: u64,
    pub list_misses: u64,
}

impl CacheManager {
    pub fn new(config: CacheConfig) -> Self {
        Self {
            conversations: TtlLruCache::new(
                config.conversation_cache_size,
                config.conversation_ttl,
            ),
            messages: TtlLruCache::new(
                config.message_cache_size,
                config.message_ttl,
            ),
            message_lists: TtlLruCache::new(
                config.message_list_cache_size,
                config.message_list_ttl,
            ),
            user_conversations: TtlLruCache::new(
                config.user_convs_cache_size,
                config.user_convs_ttl,
            ),
            streaming_buffers: TtlLruCache::new(
                config.streaming_buffer_cache_size,
                config.streaming_buffer_ttl,
            ),
            stats: Arc::new(RwLock::new(CacheStats::default())),
        }
    }

    /// Invalidate all caches for a conversation
    pub async fn invalidate_conversation(&self, conversation_id: Uuid) {
        self.conversations.invalidate(&ConversationCacheKey { conversation_id }).await;
        // Note: message lists for this conversation should also be invalidated
        // This requires tracking which list keys belong to which conversation
    }

    /// Invalidate all caches for a message
    pub async fn invalidate_message(&self, message_id: Uuid, conversation_id: Uuid) {
        self.messages.invalidate(&MessageCacheKey { message_id }).await;
        // Invalidate any message lists that might contain this message
        // For simplicity, invalidate conversation cache too
        self.invalidate_conversation(conversation_id).await;
    }

    /// Get cache statistics
    pub async fn get_stats(&self) -> CacheStats {
        self.stats.read().await.clone()
    }
}

/// Cache configuration
#[derive(Clone)]
pub struct CacheConfig {
    pub conversation_cache_size: usize,
    pub conversation_ttl: Duration,
    pub message_cache_size: usize,
    pub message_ttl: Duration,
    pub message_list_cache_size: usize,
    pub message_list_ttl: Duration,
    pub user_convs_cache_size: usize,
    pub user_convs_ttl: Duration,
    pub streaming_buffer_cache_size: usize,
    pub streaming_buffer_ttl: Duration,
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            conversation_cache_size: 1000,
            conversation_ttl: Duration::from_secs(60),
            message_cache_size: 5000,
            message_ttl: Duration::from_secs(120),
            message_list_cache_size: 500,
            message_list_ttl: Duration::from_secs(30),
            user_convs_cache_size: 200,
            user_convs_ttl: Duration::from_secs(15),
            streaming_buffer_cache_size: 100,
            streaming_buffer_ttl: Duration::from_secs(300),
        }
    }
}
```

### 3.4 Integration with Conversation Service

```rust
/// Cached conversation service wrapper
pub struct CachedConversationService {
    /// Inner service (PostgreSQL)
    inner: Arc<dyn ConversationService>,
    /// Cache manager
    cache: Arc<CacheManager>,
}

#[async_trait]
impl ConversationService for CachedConversationService {
    async fn get_conversation(&self, conversation_id: Uuid) -> Result<Option<Conversation>> {
        let key = ConversationCacheKey { conversation_id };

        // Try cache first
        if let Some(conv) = self.cache.conversations.get(&key).await {
            return Ok(Some(conv));
        }

        // Fetch from database
        let conv = self.inner.get_conversation(conversation_id).await?;

        // Cache if found
        if let Some(ref c) = conv {
            self.cache.conversations.put(key, c.clone()).await;
        }

        Ok(conv)
    }

    async fn create_message(
        &self,
        conversation_id: Uuid,
        request: CreateMessageRequest,
    ) -> Result<Message> {
        // Create in database
        let msg = self.inner.create_message(conversation_id, request).await?;

        // Cache the new message
        self.cache.messages.put(
            MessageCacheKey { message_id: msg.message_id },
            msg.clone(),
        ).await;

        // Invalidate conversation cache (message count changed)
        self.cache.invalidate_conversation(conversation_id).await;

        Ok(msg)
    }

    async fn update_message(
        &self,
        message_id: Uuid,
        request: UpdateMessageRequest,
    ) -> Result<Message> {
        // Update in database
        let msg = self.inner.update_message(message_id, request).await?;

        // Update cache
        self.cache.messages.put(
            MessageCacheKey { message_id },
            msg.clone(),
        ).await;

        // Invalidate related caches
        self.cache.invalidate_message(message_id, msg.conversation_id).await;

        Ok(msg)
    }

    // ... other methods with similar cache-through pattern
}
```

## 4. Cache Invalidation Strategy

### 4.1 Invalidation Triggers

| Operation           | Invalidations                                       |
| ------------------- | --------------------------------------------------- |
| Create conversation | User conversations list                             |
| Update conversation | Conversation, User conversations list               |
| Delete conversation | Conversation, All messages, User conversations list |
| Create message      | Conversation, Message lists                         |
| Update message      | Message, Conversation (preview), Message lists      |
| Delete message      | Message, Conversation, Message lists                |

### 4.2 Event-Based Invalidation

```rust
/// Cache invalidation events
pub enum CacheInvalidation {
    Conversation(Uuid),
    Message(Uuid, Uuid),  // message_id, conversation_id
    UserConversations(Uuid, Uuid),  // tenant_id, user_id
    AllForConversation(Uuid),
}

impl CacheManager {
    /// Process an invalidation event
    pub async fn process_invalidation(&self, event: CacheInvalidation) {
        match event {
            CacheInvalidation::Conversation(id) => {
                self.conversations.invalidate(&ConversationCacheKey { conversation_id: id }).await;
            }
            CacheInvalidation::Message(msg_id, conv_id) => {
                self.messages.invalidate(&MessageCacheKey { message_id: msg_id }).await;
                self.invalidate_conversation(conv_id).await;
            }
            CacheInvalidation::UserConversations(tenant_id, user_id) => {
                // Invalidate all user conversation caches for this user
                // This requires pattern matching or separate tracking
            }
            CacheInvalidation::AllForConversation(conv_id) => {
                self.invalidate_conversation(conv_id).await;
                // Also invalidate all messages - would need tracking
            }
        }
    }
}
```

## 5. Performance Considerations

### 5.1 Memory Estimation

| Cache             | Max Entries | Avg Entry Size | Max Memory |
| ----------------- | ----------- | -------------- | ---------- |
| Conversations     | 1000        | 2 KB           | 2 MB       |
| Messages          | 5000        | 4 KB           | 20 MB      |
| Message Lists     | 500         | 20 KB          | 10 MB      |
| User Convs        | 200         | 10 KB          | 2 MB       |
| Streaming Buffers | 100         | 32 KB          | 3.2 MB     |
| **Total**         |             |                | **~37 MB** |

### 5.2 TTL Tuning Guidelines

| Workload       | Conversation TTL | Message TTL | List TTL |
| -------------- | ---------------- | ----------- | -------- |
| High traffic   | 30s              | 60s         | 15s      |
| Medium traffic | 60s              | 120s        | 30s      |
| Low traffic    | 120s             | 300s        | 60s      |

### 5.3 Monitoring Metrics

```rust
/// Metrics to expose
pub struct CacheMetrics {
    /// Hit rate percentage
    pub hit_rate: f64,
    /// Current cache size
    pub size: usize,
    /// Eviction count
    pub evictions: u64,
    /// Average get latency
    pub avg_get_latency_us: u64,
}
```

## 6. Redis L2 Cache (Future Enhancement)

For distributed deployments, add Redis as L2 cache:

```rust
pub struct DistributedCacheManager {
    /// L1: In-memory LRU
    l1: CacheManager,
    /// L2: Redis
    redis: Option<redis::Client>,
}

impl DistributedCacheManager {
    pub async fn get_conversation(&self, id: Uuid) -> Option<Conversation> {
        // L1 check
        if let Some(conv) = self.l1.conversations.get(&ConversationCacheKey { conversation_id: id }).await {
            return Some(conv);
        }

        // L2 check (Redis)
        if let Some(ref redis) = self.redis {
            let key = format!("conv:{}", id);
            if let Ok(Some(data)) = redis.get::<_, Option<Vec<u8>>>(&key).await {
                if let Ok(conv) = serde_json::from_slice::<Conversation>(&data) {
                    // Promote to L1
                    self.l1.conversations.put(
                        ConversationCacheKey { conversation_id: id },
                        conv.clone(),
                    ).await;
                    return Some(conv);
                }
            }
        }

        None
    }
}
```

## 7. Configuration Example

```toml
[cache]
# Enable caching
enabled = true

# L1 (in-memory) configuration
[cache.l1]
conversation_size = 1000
conversation_ttl = "60s"
message_size = 5000
message_ttl = "120s"
message_list_size = 500
message_list_ttl = "30s"

# L2 (Redis) configuration - optional
[cache.l2]
enabled = false
url = "redis://localhost:6379"
ttl_multiplier = 5  # L2 TTL = L1 TTL * multiplier
```

---

_Document Version: 1.0_
_Created: 2024-12-28_
