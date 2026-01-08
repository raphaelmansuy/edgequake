//! Centralized cache manager for API layer.
//!
//! This module provides a unified cache management system for conversations
//! and messages, reducing database load for frequently accessed data.

use edgequake_core::cache::{CacheStats, TtlLruCache};
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

/// Combined cache statistics.
#[derive(Debug, Clone)]
pub struct CacheManagerStats {
    pub conversation_stats: CacheStats,
    pub message_list_stats: CacheStats,
}

/// Centralized cache manager for conversations and messages.
///
/// This provides an LRU cache with TTL expiration for hot data,
/// reducing pressure on PostgreSQL for frequently accessed conversations.
///
/// # Thread Safety
///
/// The cache manager is thread-safe and can be shared across handlers.
#[derive(Clone)]
pub struct CacheManager {
    conversations: TtlLruCache<Uuid, Conversation>,
    message_lists: TtlLruCache<Uuid, Vec<Message>>,
}

impl CacheManager {
    /// Create a new cache manager with configuration.
    pub fn new(config: CacheConfig) -> Self {
        Self {
            conversations: TtlLruCache::new(config.conversation_capacity, config.conversation_ttl),
            message_lists: TtlLruCache::new(config.message_list_capacity, config.message_list_ttl),
        }
    }

    /// Create a new cache manager with default configuration.
    pub fn with_defaults() -> Self {
        Self::new(CacheConfig::default())
    }

    // ========== Conversation Cache ==========

    /// Get a conversation from cache.
    pub fn get_conversation(&self, id: Uuid) -> Option<Conversation> {
        self.conversations.get(&id)
    }

    /// Cache a conversation.
    pub fn cache_conversation(&self, conversation: Conversation) {
        self.conversations
            .put(conversation.conversation_id, conversation);
    }

    /// Invalidate a conversation cache entry.
    ///
    /// Also invalidates related message list cache.
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

    /// Purge expired entries from all caches.
    ///
    /// Call this periodically to clean up expired entries.
    pub fn purge_expired(&self) {
        self.conversations.purge_expired();
        self.message_lists.purge_expired();
    }
}

impl Default for CacheManager {
    fn default() -> Self {
        Self::with_defaults()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use edgequake_core::types::{ConversationMode, MessageRole};
    use std::collections::HashMap;

    fn create_test_conversation() -> Conversation {
        let now = Utc::now();
        Conversation {
            conversation_id: Uuid::new_v4(),
            tenant_id: Uuid::new_v4(),
            user_id: Uuid::new_v4(),
            workspace_id: None,
            title: "Test Conversation".to_string(),
            mode: ConversationMode::Hybrid,
            folder_id: None,
            is_pinned: false,
            is_archived: false,
            share_id: None,
            meta: HashMap::new(),
            message_count: Some(0),
            last_message_preview: None,
            created_at: now,
            updated_at: now,
        }
    }

    fn create_test_message(conversation_id: Uuid) -> Message {
        let now = Utc::now();
        Message {
            message_id: Uuid::new_v4(),
            conversation_id,
            content: "Test message".to_string(),
            role: MessageRole::User,
            parent_id: None,
            mode: None,
            tokens_used: Some(10),
            duration_ms: Some(100),
            thinking_time_ms: None,
            context: None,
            is_error: false,
            created_at: now,
            updated_at: now,
        }
    }

    #[test]
    fn test_conversation_cache() {
        let cache = CacheManager::with_defaults();
        let conv = create_test_conversation();
        let id = conv.conversation_id;

        // Cache conversation
        cache.cache_conversation(conv.clone());

        // Get from cache
        let cached = cache.get_conversation(id);
        assert!(cached.is_some());
        assert_eq!(cached.unwrap().conversation_id, id);

        // Invalidate
        cache.invalidate_conversation(id);
        assert!(cache.get_conversation(id).is_none());
    }

    #[test]
    fn test_message_list_cache() {
        let cache = CacheManager::with_defaults();
        let conversation_id = Uuid::new_v4();

        let messages = vec![
            create_test_message(conversation_id),
            create_test_message(conversation_id),
        ];

        // Cache messages
        cache.cache_messages(conversation_id, messages.clone());

        // Get from cache
        let cached = cache.get_messages(conversation_id);
        assert!(cached.is_some());
        assert_eq!(cached.unwrap().len(), 2);

        // Invalidate
        cache.invalidate_messages(conversation_id);
        assert!(cache.get_messages(conversation_id).is_none());
    }

    #[test]
    fn test_conversation_invalidation_clears_messages() {
        let cache = CacheManager::with_defaults();
        let conv = create_test_conversation();
        let id = conv.conversation_id;

        let messages = vec![create_test_message(id)];

        cache.cache_conversation(conv);
        cache.cache_messages(id, messages);

        // Invalidating conversation should also clear messages
        cache.invalidate_conversation(id);

        assert!(cache.get_conversation(id).is_none());
        assert!(cache.get_messages(id).is_none());
    }

    #[test]
    fn test_stats() {
        let cache = CacheManager::with_defaults();
        let conv = create_test_conversation();
        let id = conv.conversation_id;

        cache.cache_conversation(conv);

        // Hit
        cache.get_conversation(id);

        // Miss
        cache.get_conversation(Uuid::new_v4());

        let stats = cache.stats();
        assert_eq!(stats.conversation_stats.hits, 1);
        assert_eq!(stats.conversation_stats.misses, 1);
    }

    #[test]
    fn test_clear() {
        let cache = CacheManager::with_defaults();

        let conv1 = create_test_conversation();
        let conv2 = create_test_conversation();

        cache.cache_conversation(conv1.clone());
        cache.cache_conversation(conv2.clone());

        let stats = cache.stats();
        assert_eq!(stats.conversation_stats.size, 2);

        cache.clear();

        let stats = cache.stats();
        assert_eq!(stats.conversation_stats.size, 0);
    }

    #[test]
    fn test_cache_config_default() {
        let config = CacheConfig::default();
        assert_eq!(config.conversation_capacity, 1000);
        assert_eq!(config.conversation_ttl, Duration::from_secs(300));
        assert_eq!(config.message_list_capacity, 500);
        assert_eq!(config.message_list_ttl, Duration::from_secs(60));
    }

    #[test]
    fn test_cache_config_custom() {
        let config = CacheConfig {
            conversation_capacity: 100,
            conversation_ttl: Duration::from_secs(60),
            message_list_capacity: 50,
            message_list_ttl: Duration::from_secs(30),
        };
        assert_eq!(config.conversation_capacity, 100);
        assert_eq!(config.message_list_capacity, 50);
    }

    #[test]
    fn test_cache_manager_with_custom_config() {
        let config = CacheConfig {
            conversation_capacity: 10,
            conversation_ttl: Duration::from_secs(1),
            message_list_capacity: 5,
            message_list_ttl: Duration::from_secs(1),
        };
        let cache = CacheManager::new(config);

        let conv = create_test_conversation();
        let id = conv.conversation_id;
        cache.cache_conversation(conv.clone());

        assert!(cache.get_conversation(id).is_some());
    }

    #[test]
    fn test_cache_manager_default_trait() {
        let cache = CacheManager::default();
        let conv = create_test_conversation();
        let id = conv.conversation_id;

        cache.cache_conversation(conv);
        assert!(cache.get_conversation(id).is_some());
    }

    #[test]
    fn test_cache_miss_returns_none() {
        let cache = CacheManager::with_defaults();
        let nonexistent_id = Uuid::new_v4();

        assert!(cache.get_conversation(nonexistent_id).is_none());
        assert!(cache.get_messages(nonexistent_id).is_none());
    }

    #[test]
    fn test_cache_overwrite() {
        let cache = CacheManager::with_defaults();
        let mut conv = create_test_conversation();
        let id = conv.conversation_id;

        cache.cache_conversation(conv.clone());

        // Update and re-cache
        conv.title = "Updated Title".to_string();
        cache.cache_conversation(conv);

        let cached = cache.get_conversation(id).unwrap();
        assert_eq!(cached.title, "Updated Title");
    }

    #[test]
    fn test_message_list_overwrite() {
        let cache = CacheManager::with_defaults();
        let conversation_id = Uuid::new_v4();

        let messages1 = vec![create_test_message(conversation_id)];
        cache.cache_messages(conversation_id, messages1);
        assert_eq!(cache.get_messages(conversation_id).unwrap().len(), 1);

        let messages2 = vec![
            create_test_message(conversation_id),
            create_test_message(conversation_id),
            create_test_message(conversation_id),
        ];
        cache.cache_messages(conversation_id, messages2);
        assert_eq!(cache.get_messages(conversation_id).unwrap().len(), 3);
    }

    #[test]
    fn test_stats_tracks_message_cache() {
        let cache = CacheManager::with_defaults();
        let conversation_id = Uuid::new_v4();

        let messages = vec![create_test_message(conversation_id)];
        cache.cache_messages(conversation_id, messages);

        // Hit
        cache.get_messages(conversation_id);

        // Miss
        cache.get_messages(Uuid::new_v4());

        let stats = cache.stats();
        assert_eq!(stats.message_list_stats.hits, 1);
        assert_eq!(stats.message_list_stats.misses, 1);
    }

    #[test]
    fn test_purge_expired() {
        let config = CacheConfig {
            conversation_capacity: 10,
            conversation_ttl: Duration::from_millis(1), // Very short TTL
            message_list_capacity: 5,
            message_list_ttl: Duration::from_millis(1),
        };
        let cache = CacheManager::new(config);

        let conv = create_test_conversation();
        let id = conv.conversation_id;
        cache.cache_conversation(conv);

        // Wait for expiration
        std::thread::sleep(Duration::from_millis(10));

        // Purge expired entries
        cache.purge_expired();

        // Entry should be gone
        assert!(cache.get_conversation(id).is_none());
    }

    #[test]
    fn test_cache_manager_clone() {
        let cache = CacheManager::with_defaults();
        let conv = create_test_conversation();
        let id = conv.conversation_id;

        cache.cache_conversation(conv);

        // Clone the cache manager
        let cache_clone = cache.clone();

        // Both should access the same underlying cache
        assert!(cache.get_conversation(id).is_some());
        assert!(cache_clone.get_conversation(id).is_some());
    }

    #[test]
    fn test_invalidate_nonexistent_entry() {
        let cache = CacheManager::with_defaults();
        let nonexistent_id = Uuid::new_v4();

        // Should not panic
        cache.invalidate_conversation(nonexistent_id);
        cache.invalidate_messages(nonexistent_id);

        // Stats should reflect no entries
        let stats = cache.stats();
        assert_eq!(stats.conversation_stats.size, 0);
    }
}
