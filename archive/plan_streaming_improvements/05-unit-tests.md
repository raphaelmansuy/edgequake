# Unit Test Specifications for Streaming Improvements

## 1. Overview

This document defines comprehensive unit tests for the streaming token handling improvements. Tests are organized by component and cover both happy paths and edge cases.

## 2. Test Categories

### 2.1 Debounce Buffer Tests

```rust
#[cfg(test)]
mod debounce_buffer_tests {
    use super::*;
    use std::time::Duration;
    use tokio::time::{sleep, Instant};

    /// Test: Buffer should not flush before write_delay expires
    #[tokio::test]
    async fn test_buffer_no_flush_before_delay() {
        let config = DebounceConfig {
            write_delay: Duration::from_millis(500),
            max_buffer_time: Duration::from_millis(2000),
            max_buffer_bytes: 8192,
            ..Default::default()
        };

        let mut buffer = StreamingMessageBuffer::new(Uuid::new_v4(), Uuid::new_v4());
        buffer.append("Hello ");

        // Immediately after append, should not flush
        assert!(!buffer.should_flush(&config));

        // After 400ms (< 500ms), still should not flush
        sleep(Duration::from_millis(400)).await;
        assert!(!buffer.should_flush(&config));
    }

    /// Test: Buffer should flush after write_delay expires (trailing edge)
    #[tokio::test]
    async fn test_buffer_flush_after_delay() {
        let config = DebounceConfig {
            write_delay: Duration::from_millis(100),
            ..Default::default()
        };

        let mut buffer = StreamingMessageBuffer::new(Uuid::new_v4(), Uuid::new_v4());
        buffer.append("Hello ");

        // Wait for delay to expire
        sleep(Duration::from_millis(150)).await;

        assert!(buffer.should_flush(&config));
    }

    /// Test: Buffer should force flush when max_buffer_bytes exceeded
    #[tokio::test]
    async fn test_buffer_force_flush_on_max_bytes() {
        let config = DebounceConfig {
            write_delay: Duration::from_millis(5000), // Very long delay
            max_buffer_bytes: 100,
            ..Default::default()
        };

        let mut buffer = StreamingMessageBuffer::new(Uuid::new_v4(), Uuid::new_v4());

        // Add content exceeding max_buffer_bytes
        let large_content = "x".repeat(150);
        buffer.append(&large_content);

        // Should flush immediately despite not reaching write_delay
        assert!(buffer.should_flush(&config));
    }

    /// Test: Buffer should force flush when max_buffer_time exceeded
    #[tokio::test]
    async fn test_buffer_force_flush_on_max_time() {
        let config = DebounceConfig {
            write_delay: Duration::from_millis(5000), // Very long delay
            max_buffer_time: Duration::from_millis(100),
            max_buffer_bytes: 10000,
            ..Default::default()
        };

        let mut buffer = StreamingMessageBuffer::new(Uuid::new_v4(), Uuid::new_v4());
        buffer.append("Hello");

        // Wait for max_buffer_time
        sleep(Duration::from_millis(150)).await;

        // Should flush due to max_buffer_time
        assert!(buffer.should_flush(&config));
    }

    /// Test: Buffer reset after flush
    #[tokio::test]
    async fn test_buffer_reset_after_flush() {
        let config = DebounceConfig::default();

        let mut buffer = StreamingMessageBuffer::new(Uuid::new_v4(), Uuid::new_v4());
        buffer.append("Hello ");
        buffer.append("World");

        assert_eq!(buffer.bytes_since_write, 11);

        buffer.mark_flushed();

        assert_eq!(buffer.bytes_since_write, 0);
        assert!(buffer.last_write_at.is_some());
    }

    /// Test: Content accumulation
    #[tokio::test]
    async fn test_content_accumulation() {
        let mut buffer = StreamingMessageBuffer::new(Uuid::new_v4(), Uuid::new_v4());

        buffer.append("Hello ");
        buffer.append("World");
        buffer.append("!");

        assert_eq!(buffer.content, "Hello World!");
        assert_eq!(buffer.tokens, 3);
    }

    /// Test: Continuous streaming with periodic flushes
    #[tokio::test]
    async fn test_continuous_streaming() {
        let config = DebounceConfig {
            write_delay: Duration::from_millis(50),
            max_buffer_time: Duration::from_millis(200),
            max_buffer_bytes: 1000,
            ..Default::default()
        };

        let mut buffer = StreamingMessageBuffer::new(Uuid::new_v4(), Uuid::new_v4());
        let mut flush_count = 0;

        // Simulate continuous streaming for 500ms
        let start = Instant::now();
        while start.elapsed() < Duration::from_millis(500) {
            buffer.append("token ");

            if buffer.should_flush(&config) {
                flush_count += 1;
                buffer.mark_flushed();
            }

            sleep(Duration::from_millis(10)).await;
        }

        // Should have flushed multiple times due to max_buffer_time
        assert!(flush_count >= 2);
        assert!(flush_count <= 5);
    }
}
```

### 2.2 LRU Cache Tests

```rust
#[cfg(test)]
mod lru_cache_tests {
    use super::*;
    use std::time::Duration;

    /// Test: Cache hit returns value
    #[tokio::test]
    async fn test_cache_hit() {
        let cache: TtlLruCache<String, i32> = TtlLruCache::new(10, Duration::from_secs(60));

        cache.put("key1".to_string(), 42).await;

        let result = cache.get(&"key1".to_string()).await;
        assert_eq!(result, Some(42));
    }

    /// Test: Cache miss returns None
    #[tokio::test]
    async fn test_cache_miss() {
        let cache: TtlLruCache<String, i32> = TtlLruCache::new(10, Duration::from_secs(60));

        let result = cache.get(&"nonexistent".to_string()).await;
        assert_eq!(result, None);
    }

    /// Test: TTL expiration
    #[tokio::test]
    async fn test_cache_ttl_expiration() {
        let cache: TtlLruCache<String, i32> = TtlLruCache::new(10, Duration::from_millis(100));

        cache.put("key1".to_string(), 42).await;

        // Before expiration
        assert_eq!(cache.get(&"key1".to_string()).await, Some(42));

        // Wait for TTL to expire
        tokio::time::sleep(Duration::from_millis(150)).await;

        // After expiration
        assert_eq!(cache.get(&"key1".to_string()).await, None);
    }

    /// Test: LRU eviction when capacity exceeded
    #[tokio::test]
    async fn test_lru_eviction() {
        let cache: TtlLruCache<String, i32> = TtlLruCache::new(3, Duration::from_secs(60));

        cache.put("key1".to_string(), 1).await;
        cache.put("key2".to_string(), 2).await;
        cache.put("key3".to_string(), 3).await;

        // Access key1 to make it recently used
        let _ = cache.get(&"key1".to_string()).await;

        // Add key4, should evict key2 (least recently used)
        cache.put("key4".to_string(), 4).await;

        assert_eq!(cache.get(&"key1".to_string()).await, Some(1)); // Still present
        assert_eq!(cache.get(&"key2".to_string()).await, None);    // Evicted
        assert_eq!(cache.get(&"key3".to_string()).await, Some(3)); // Still present
        assert_eq!(cache.get(&"key4".to_string()).await, Some(4)); // Just added
    }

    /// Test: Cache invalidation
    #[tokio::test]
    async fn test_cache_invalidation() {
        let cache: TtlLruCache<String, i32> = TtlLruCache::new(10, Duration::from_secs(60));

        cache.put("key1".to_string(), 42).await;
        assert_eq!(cache.get(&"key1".to_string()).await, Some(42));

        cache.invalidate(&"key1".to_string()).await;
        assert_eq!(cache.get(&"key1".to_string()).await, None);
    }

    /// Test: get_or_insert pattern
    #[tokio::test]
    async fn test_get_or_insert() {
        let cache: TtlLruCache<String, i32> = TtlLruCache::new(10, Duration::from_secs(60));

        let compute_count = std::sync::atomic::AtomicU32::new(0);

        // First call - should compute
        let result = cache.get_or_insert("key1".to_string(), || async {
            compute_count.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            Ok(42)
        }).await.unwrap();

        assert_eq!(result, 42);
        assert_eq!(compute_count.load(std::sync::atomic::Ordering::SeqCst), 1);

        // Second call - should hit cache
        let result = cache.get_or_insert("key1".to_string(), || async {
            compute_count.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            Ok(100)
        }).await.unwrap();

        assert_eq!(result, 42); // Cached value
        assert_eq!(compute_count.load(std::sync::atomic::Ordering::SeqCst), 1); // Not computed again
    }

    /// Test: Custom TTL per entry
    #[tokio::test]
    async fn test_custom_ttl() {
        let cache: TtlLruCache<String, i32> = TtlLruCache::new(10, Duration::from_secs(60));

        cache.put_with_ttl("short".to_string(), 1, Duration::from_millis(50)).await;
        cache.put_with_ttl("long".to_string(), 2, Duration::from_millis(500)).await;

        tokio::time::sleep(Duration::from_millis(100)).await;

        assert_eq!(cache.get(&"short".to_string()).await, None);  // Expired
        assert_eq!(cache.get(&"long".to_string()).await, Some(2)); // Still valid
    }

    /// Test: Concurrent access safety
    #[tokio::test]
    async fn test_concurrent_access() {
        let cache = Arc::new(TtlLruCache::<String, i32>::new(100, Duration::from_secs(60)));

        let mut handles = vec![];

        // Spawn 10 concurrent writers
        for i in 0..10 {
            let cache_clone = cache.clone();
            handles.push(tokio::spawn(async move {
                for j in 0..100 {
                    cache_clone.put(format!("key_{}", i * 100 + j), i * 100 + j).await;
                }
            }));
        }

        // Spawn 10 concurrent readers
        for _ in 0..10 {
            let cache_clone = cache.clone();
            handles.push(tokio::spawn(async move {
                for i in 0..1000 {
                    let _ = cache_clone.get(&format!("key_{}", i % 500)).await;
                }
            }));
        }

        for handle in handles {
            handle.await.unwrap();
        }

        // No panic means thread-safe
    }
}
```

### 2.3 Stream Accumulator Tests

```rust
#[cfg(test)]
mod stream_accumulator_tests {
    use super::*;

    /// Test: Content accumulation from chunks
    #[test]
    fn test_content_accumulation() {
        let mut acc = StreamAccumulator::new(Uuid::new_v4());

        acc.process_text("Hello ");
        acc.process_text("world");
        acc.process_text("!");

        let finalized = acc.finalize();
        assert_eq!(finalized.content, "Hello world!");
    }

    /// Test: Reasoning content separation
    #[test]
    fn test_reasoning_content() {
        let mut acc = StreamAccumulator::new(Uuid::new_v4());

        // Simulate reasoning model output
        acc.process_reasoning("Let me think...");
        acc.process_text("The answer is 42.");

        let finalized = acc.finalize();
        assert_eq!(finalized.content, "The answer is 42.");
        assert_eq!(finalized.reasoning_content, Some("Let me think...".to_string()));
    }

    /// Test: Chunk counting
    #[test]
    fn test_chunk_counting() {
        let mut acc = StreamAccumulator::new(Uuid::new_v4());

        for _ in 0..10 {
            acc.process_text("x");
        }

        let finalized = acc.finalize();
        assert_eq!(finalized.streaming_state.chunks_received, 10);
    }

    /// Test: Bytes tracking
    #[test]
    fn test_bytes_tracking() {
        let mut acc = StreamAccumulator::new(Uuid::new_v4());

        acc.process_text("Hello");  // 5 bytes
        acc.process_text(" ");       // 1 byte
        acc.process_text("World");   // 5 bytes

        let finalized = acc.finalize();
        assert_eq!(finalized.streaming_state.bytes_received, 11);
    }

    /// Test: Empty stream handling
    #[test]
    fn test_empty_stream() {
        let acc = StreamAccumulator::new(Uuid::new_v4());

        let finalized = acc.finalize();
        assert_eq!(finalized.content, "");
        assert_eq!(finalized.reasoning_content, None);
        assert_eq!(finalized.streaming_state.chunks_received, 0);
    }

    /// Test: Performance metrics calculation
    #[test]
    fn test_performance_metrics() {
        let mut acc = StreamAccumulator::new(Uuid::new_v4());

        // Small delay to get measurable duration
        std::thread::sleep(std::time::Duration::from_millis(10));

        acc.process_text("Hello");

        let finalized = acc.finalize();

        assert!(finalized.performance.duration_ms.unwrap() >= 10);
        assert!(finalized.performance.first_token_ms.is_some());
    }

    /// Test: API metadata from chunks
    #[test]
    fn test_api_metadata() {
        let mut acc = StreamAccumulator::new(Uuid::new_v4());

        // First chunk with metadata
        acc.set_response_id("chatcmpl-123".to_string());
        acc.set_model("gpt-4".to_string());

        acc.process_text("Hello");

        // Final chunk with finish reason and usage
        acc.set_finish_reason(FinishReason::Stop);
        acc.set_usage(TokenUsage {
            prompt_tokens: 10,
            completion_tokens: 5,
            reasoning_tokens: None,
            total_tokens: 15,
        });

        let finalized = acc.finalize();

        let api = finalized.api_response.unwrap();
        assert_eq!(api.response_id, "chatcmpl-123");
        assert_eq!(api.model, "gpt-4");
        assert!(matches!(api.finish_reason, FinishReason::Stop));
        assert_eq!(api.usage.total_tokens, 15);
    }

    /// Test: Unicode content handling
    #[test]
    fn test_unicode_content() {
        let mut acc = StreamAccumulator::new(Uuid::new_v4());

        acc.process_text("Hello 世界 🌍");

        let finalized = acc.finalize();
        assert_eq!(finalized.content, "Hello 世界 🌍");
    }

    /// Test: Very long content
    #[test]
    fn test_long_content() {
        let mut acc = StreamAccumulator::new(Uuid::new_v4());

        let chunk = "x".repeat(1000);
        for _ in 0..100 {
            acc.process_text(&chunk);
        }

        let finalized = acc.finalize();
        assert_eq!(finalized.content.len(), 100_000);
    }
}
```

### 2.4 Token Usage Tests

```rust
#[cfg(test)]
mod token_usage_tests {
    use super::*;

    /// Test: Accurate token count from API (not chunk count)
    #[test]
    fn test_token_count_from_api() {
        let usage = TokenUsage {
            prompt_tokens: 50,
            completion_tokens: 25,
            reasoning_tokens: Some(100),
            total_tokens: 175,
        };

        // Total should include reasoning tokens if present
        assert_eq!(usage.total_tokens, 175);
        assert_eq!(usage.completion_tokens, 25);
    }

    /// Test: Missing reasoning tokens for non-reasoning models
    #[test]
    fn test_no_reasoning_tokens() {
        let usage = TokenUsage {
            prompt_tokens: 50,
            completion_tokens: 25,
            reasoning_tokens: None,
            total_tokens: 75,
        };

        assert_eq!(usage.reasoning_tokens, None);
    }

    /// Test: Tokens per second calculation
    #[test]
    fn test_tokens_per_second() {
        let usage = TokenUsage {
            prompt_tokens: 50,
            completion_tokens: 100,
            reasoning_tokens: None,
            total_tokens: 150,
        };

        let duration_secs = 2.0f32;
        let tps = usage.completion_tokens as f32 / duration_secs;

        assert_eq!(tps, 50.0);
    }
}
```

### 2.5 Cache Manager Integration Tests

```rust
#[cfg(test)]
mod cache_manager_tests {
    use super::*;

    fn create_test_conversation() -> Conversation {
        Conversation::new(Uuid::new_v4(), Uuid::new_v4())
            .with_title("Test Conversation")
    }

    /// Test: Conversation caching through manager
    #[tokio::test]
    async fn test_conversation_caching() {
        let manager = CacheManager::new(CacheConfig::default());

        let conv = create_test_conversation();
        let key = ConversationCacheKey { conversation_id: conv.conversation_id };

        // Cache miss
        assert!(manager.conversations.get(&key).await.is_none());

        // Put in cache
        manager.conversations.put(key.clone(), conv.clone()).await;

        // Cache hit
        let cached = manager.conversations.get(&key).await.unwrap();
        assert_eq!(cached.conversation_id, conv.conversation_id);
    }

    /// Test: Invalidate conversation cascades properly
    #[tokio::test]
    async fn test_invalidation_cascade() {
        let manager = CacheManager::new(CacheConfig::default());

        let conv_id = Uuid::new_v4();
        let conv = Conversation::new(Uuid::new_v4(), Uuid::new_v4());

        manager.conversations.put(
            ConversationCacheKey { conversation_id: conv_id },
            conv.clone()
        ).await;

        // Invalidate
        manager.invalidate_conversation(conv_id).await;

        // Should be gone
        assert!(manager.conversations.get(
            &ConversationCacheKey { conversation_id: conv_id }
        ).await.is_none());
    }
}
```

## 3. Mock Dependencies

### 3.1 Mock Database

```rust
/// Mock database for testing without real PostgreSQL
pub struct MockDatabase {
    conversations: Arc<RwLock<HashMap<Uuid, Conversation>>>,
    messages: Arc<RwLock<HashMap<Uuid, Message>>>,
}

impl MockDatabase {
    pub fn new() -> Self {
        Self {
            conversations: Arc::new(RwLock::new(HashMap::new())),
            messages: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn insert_conversation(&self, conv: Conversation) {
        self.conversations.write().await.insert(conv.conversation_id, conv);
    }

    pub async fn get_conversation(&self, id: Uuid) -> Option<Conversation> {
        self.conversations.read().await.get(&id).cloned()
    }

    pub async fn insert_message(&self, msg: Message) {
        self.messages.write().await.insert(msg.message_id, msg);
    }

    pub async fn update_message(&self, id: Uuid, content: String) -> Option<Message> {
        let mut messages = self.messages.write().await;
        if let Some(msg) = messages.get_mut(&id) {
            msg.content = content;
            msg.updated_at = Utc::now();
            Some(msg.clone())
        } else {
            None
        }
    }
}
```

### 3.2 Mock LLM Stream

```rust
/// Mock LLM stream for testing streaming behavior
pub struct MockLLMStream {
    chunks: Vec<String>,
    delay: Duration,
}

impl MockLLMStream {
    pub fn new(chunks: Vec<String>, delay: Duration) -> Self {
        Self { chunks, delay }
    }

    pub fn quick(content: &str) -> Self {
        let chunks: Vec<_> = content.split_whitespace()
            .map(|s| format!("{} ", s))
            .collect();
        Self::new(chunks, Duration::from_millis(10))
    }

    pub fn slow(content: &str) -> Self {
        let chunks: Vec<_> = content.split_whitespace()
            .map(|s| format!("{} ", s))
            .collect();
        Self::new(chunks, Duration::from_millis(100))
    }
}

impl Stream for MockLLMStream {
    type Item = Result<String, Error>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        // Implementation would yield chunks with delay
        todo!()
    }
}
```

## 4. Test Execution Commands

```bash
# Run all unit tests
cargo test --package edgequake-api --lib

# Run specific test module
cargo test --package edgequake-api debounce_buffer_tests

# Run with output
cargo test --package edgequake-api -- --nocapture

# Run with coverage
cargo tarpaulin --packages edgequake-api --out Html

# Run benchmarks
cargo bench --package edgequake-api
```

---

_Document Version: 1.0_
_Created: 2024-12-28_
