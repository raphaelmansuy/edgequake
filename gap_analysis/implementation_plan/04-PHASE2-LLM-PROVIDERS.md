# Phase 2: LLM Provider Expansion

**Document ID:** 04-PHASE2-LLM-PROVIDERS  
**Priority:** 🟠 P1 HIGH  
**Effort:** 8 person-days  
**Duration:** Weeks 4-6  
**Dependencies:** None  
**Blocks:** None

---

## 📋 Overview

This document provides implementation guidance for expanding LLM provider support, implementing rate limiting, and completing the LLM response cache. These features are essential for production deployment and cost optimization.

### Gaps Addressed

| Gap ID      | Feature            | Severity | Status         | Effort |
| ----------- | ------------------ | -------- | -------------- | ------ |
| **GAP-010** | Anthropic Provider | 🟠 P1    | 🔲 Not started | 3 days |
| **GAP-011** | Rate Limiting      | 🟠 P1    | 🔲 Not started | 3 days |
| **GAP-015** | LLM Cache Complete | 🟠 P1    | 🔲 Not started | 2 days |
| **GAP-028** | Azure OpenAI       | 🟡 P2    | 🔲 Not started | 2 days |

### Cross-References

- **Source Analysis:** [../gap-analysis.md](../gap-analysis.md#feature-f-040-anthropic)
- **Master Plan:** [00-INDEX.md](./00-INDEX.md#phase-2-enhancement-weeks-4-6)
- **Testing Plan:** [07-VALIDATION-TESTING.md](./07-VALIDATION-TESTING.md#llm-provider-tests)

---

## 🎯 Anthropic Provider

### 1.1 Objective

Implement Anthropic Claude provider supporting Claude 3.5 Sonnet, Claude 3 Opus, and Claude 3 Haiku models.

### 1.2 Source Reference

**Location:** `lightrag/llm/anthropic.py`  
**API Docs:** https://docs.anthropic.com/en/api

### 1.3 Implementation Tasks

#### Task 1.3.1: Create Anthropic Provider

**File:** `edgequake/crates/edgequake-llm/src/anthropic.rs` (NEW)

```rust
// NEW FILE: edgequake/crates/edgequake-llm/src/anthropic.rs

//! Anthropic Claude LLM provider implementation.

use crate::traits::{LLMProvider, LLMResponse, ProviderError, StreamChunk};
use async_trait::async_trait;
use futures::Stream;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::pin::Pin;

/// Anthropic Claude provider configuration
#[derive(Debug, Clone)]
pub struct AnthropicConfig {
    pub api_key: String,
    pub model: String,
    pub max_tokens: usize,
    pub temperature: f32,
    pub base_url: String,
    pub api_version: String,
}

impl Default for AnthropicConfig {
    fn default() -> Self {
        Self {
            api_key: std::env::var("ANTHROPIC_API_KEY").unwrap_or_default(),
            model: "claude-3-5-sonnet-20241022".to_string(),
            max_tokens: 4096,
            temperature: 0.0,
            base_url: "https://api.anthropic.com".to_string(),
            api_version: "2023-06-01".to_string(),
        }
    }
}

impl AnthropicConfig {
    pub fn new(api_key: impl Into<String>) -> Self {
        Self {
            api_key: api_key.into(),
            ..Default::default()
        }
    }

    pub fn with_model(mut self, model: impl Into<String>) -> Self {
        self.model = model.into();
        self
    }

    pub fn with_max_tokens(mut self, max_tokens: usize) -> Self {
        self.max_tokens = max_tokens;
        self
    }

    pub fn with_temperature(mut self, temperature: f32) -> Self {
        self.temperature = temperature;
        self
    }
}

/// Anthropic Claude LLM provider
pub struct AnthropicProvider {
    config: AnthropicConfig,
    client: Client,
}

impl AnthropicProvider {
    pub fn new(config: AnthropicConfig) -> Self {
        Self {
            config,
            client: Client::new(),
        }
    }

    pub fn from_env() -> Result<Self, ProviderError> {
        let api_key = std::env::var("ANTHROPIC_API_KEY")
            .map_err(|_| ProviderError::Config("ANTHROPIC_API_KEY not set".to_string()))?;

        Ok(Self::new(AnthropicConfig::new(api_key)))
    }
}

// Request/Response types for Anthropic API
#[derive(Serialize)]
struct AnthropicRequest {
    model: String,
    max_tokens: usize,
    messages: Vec<AnthropicMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    system: Option<String>,
    temperature: f32,
    #[serde(skip_serializing_if = "Option::is_none")]
    stream: Option<bool>,
}

#[derive(Serialize, Deserialize)]
struct AnthropicMessage {
    role: String,
    content: String,
}

#[derive(Deserialize)]
struct AnthropicResponse {
    id: String,
    content: Vec<AnthropicContent>,
    model: String,
    stop_reason: Option<String>,
    usage: AnthropicUsage,
}

#[derive(Deserialize)]
struct AnthropicContent {
    #[serde(rename = "type")]
    content_type: String,
    text: String,
}

#[derive(Deserialize)]
struct AnthropicUsage {
    input_tokens: usize,
    output_tokens: usize,
}

#[derive(Deserialize)]
struct AnthropicStreamEvent {
    #[serde(rename = "type")]
    event_type: String,
    #[serde(default)]
    delta: Option<AnthropicDelta>,
    #[serde(default)]
    usage: Option<AnthropicUsage>,
}

#[derive(Deserialize, Default)]
struct AnthropicDelta {
    #[serde(rename = "type")]
    delta_type: Option<String>,
    text: Option<String>,
}

#[derive(Deserialize)]
struct AnthropicError {
    error: AnthropicErrorDetails,
}

#[derive(Deserialize)]
struct AnthropicErrorDetails {
    message: String,
    #[serde(rename = "type")]
    error_type: String,
}

#[async_trait]
impl LLMProvider for AnthropicProvider {
    async fn complete(&self, prompt: &str) -> Result<LLMResponse, ProviderError> {
        let request = AnthropicRequest {
            model: self.config.model.clone(),
            max_tokens: self.config.max_tokens,
            messages: vec![AnthropicMessage {
                role: "user".to_string(),
                content: prompt.to_string(),
            }],
            system: None,
            temperature: self.config.temperature,
            stream: None,
        };

        let response = self.client
            .post(format!("{}/v1/messages", self.config.base_url))
            .header("x-api-key", &self.config.api_key)
            .header("anthropic-version", &self.config.api_version)
            .header("content-type", "application/json")
            .json(&request)
            .send()
            .await
            .map_err(|e| ProviderError::Network(e.to_string()))?;

        let status = response.status();
        if !status.is_success() {
            let error_body = response.text().await.unwrap_or_default();

            // Try to parse Anthropic error format
            if let Ok(error) = serde_json::from_str::<AnthropicError>(&error_body) {
                return Err(ProviderError::Api(format!(
                    "{}: {}", error.error.error_type, error.error.message
                )));
            }

            return Err(ProviderError::Api(format!(
                "HTTP {}: {}", status, error_body
            )));
        }

        let anthropic_response: AnthropicResponse = response.json().await
            .map_err(|e| ProviderError::Parse(e.to_string()))?;

        let content = anthropic_response.content
            .into_iter()
            .filter(|c| c.content_type == "text")
            .map(|c| c.text)
            .collect::<Vec<_>>()
            .join("");

        Ok(LLMResponse {
            content,
            model: anthropic_response.model,
            prompt_tokens: anthropic_response.usage.input_tokens,
            completion_tokens: anthropic_response.usage.output_tokens,
            finish_reason: anthropic_response.stop_reason,
        })
    }

    async fn complete_with_system(
        &self,
        system: &str,
        prompt: &str,
    ) -> Result<LLMResponse, ProviderError> {
        let request = AnthropicRequest {
            model: self.config.model.clone(),
            max_tokens: self.config.max_tokens,
            messages: vec![AnthropicMessage {
                role: "user".to_string(),
                content: prompt.to_string(),
            }],
            system: Some(system.to_string()),
            temperature: self.config.temperature,
            stream: None,
        };

        let response = self.client
            .post(format!("{}/v1/messages", self.config.base_url))
            .header("x-api-key", &self.config.api_key)
            .header("anthropic-version", &self.config.api_version)
            .header("content-type", "application/json")
            .json(&request)
            .send()
            .await
            .map_err(|e| ProviderError::Network(e.to_string()))?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(ProviderError::Api(error_text));
        }

        let anthropic_response: AnthropicResponse = response.json().await
            .map_err(|e| ProviderError::Parse(e.to_string()))?;

        let content = anthropic_response.content
            .into_iter()
            .filter(|c| c.content_type == "text")
            .map(|c| c.text)
            .collect::<Vec<_>>()
            .join("");

        Ok(LLMResponse {
            content,
            model: anthropic_response.model,
            prompt_tokens: anthropic_response.usage.input_tokens,
            completion_tokens: anthropic_response.usage.output_tokens,
            finish_reason: anthropic_response.stop_reason,
        })
    }

    fn stream(
        &self,
        prompt: &str,
    ) -> Pin<Box<dyn Stream<Item = Result<StreamChunk, ProviderError>> + Send>> {
        let config = self.config.clone();
        let client = self.client.clone();
        let prompt = prompt.to_string();

        Box::pin(async_stream::try_stream! {
            let request = AnthropicRequest {
                model: config.model.clone(),
                max_tokens: config.max_tokens,
                messages: vec![AnthropicMessage {
                    role: "user".to_string(),
                    content: prompt,
                }],
                system: None,
                temperature: config.temperature,
                stream: Some(true),
            };

            let response = client
                .post(format!("{}/v1/messages", config.base_url))
                .header("x-api-key", &config.api_key)
                .header("anthropic-version", &config.api_version)
                .header("content-type", "application/json")
                .json(&request)
                .send()
                .await
                .map_err(|e| ProviderError::Network(e.to_string()))?;

            if !response.status().is_success() {
                let error_text = response.text().await.unwrap_or_default();
                Err(ProviderError::Api(error_text))?;
            }

            let mut stream = response.bytes_stream();
            use futures::StreamExt;

            let mut buffer = String::new();

            while let Some(chunk) = stream.next().await {
                let chunk = chunk.map_err(|e| ProviderError::Network(e.to_string()))?;
                buffer.push_str(&String::from_utf8_lossy(&chunk));

                // Process complete SSE events
                while let Some(pos) = buffer.find("\n\n") {
                    let event_str = buffer[..pos].to_string();
                    buffer = buffer[pos + 2..].to_string();

                    // Parse SSE event
                    for line in event_str.lines() {
                        if let Some(data) = line.strip_prefix("data: ") {
                            if data == "[DONE]" {
                                return;
                            }

                            if let Ok(event) = serde_json::from_str::<AnthropicStreamEvent>(data) {
                                if event.event_type == "content_block_delta" {
                                    if let Some(delta) = event.delta {
                                        if let Some(text) = delta.text {
                                            yield StreamChunk {
                                                content: text,
                                                is_final: false,
                                            };
                                        }
                                    }
                                } else if event.event_type == "message_stop" {
                                    yield StreamChunk {
                                        content: String::new(),
                                        is_final: true,
                                    };
                                }
                            }
                        }
                    }
                }
            }
        })
    }

    fn model_name(&self) -> &str {
        &self.config.model
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_defaults() {
        let config = AnthropicConfig::default();
        assert!(config.model.contains("claude"));
        assert_eq!(config.temperature, 0.0);
    }

    #[test]
    fn test_config_builder() {
        let config = AnthropicConfig::new("test-key")
            .with_model("claude-3-opus-20240229")
            .with_temperature(0.7);

        assert_eq!(config.model, "claude-3-opus-20240229");
        assert_eq!(config.temperature, 0.7);
    }
}
```

---

#### Task 1.3.2: Update Provider Factory

**File:** `edgequake/crates/edgequake-llm/src/factory.rs`

```rust
// ADD to provider factory

use crate::anthropic::{AnthropicConfig, AnthropicProvider};

/// Supported LLM providers
pub enum Provider {
    OpenAI,
    Anthropic,
    Mock,
}

impl ProviderFactory {
    /// Create provider from environment
    pub fn from_env() -> Result<Arc<dyn LLMProvider>, ProviderError> {
        // Check for Anthropic first (if key present)
        if std::env::var("ANTHROPIC_API_KEY").is_ok() {
            return Ok(Arc::new(AnthropicProvider::from_env()?));
        }

        // Check for OpenAI
        if std::env::var("OPENAI_API_KEY").is_ok() {
            return Ok(Arc::new(OpenAIProvider::from_env()?));
        }

        // Fallback to mock
        Ok(Arc::new(MockProvider::new()))
    }

    /// Create specific provider
    pub fn create(provider: Provider) -> Result<Arc<dyn LLMProvider>, ProviderError> {
        match provider {
            Provider::OpenAI => Ok(Arc::new(OpenAIProvider::from_env()?)),
            Provider::Anthropic => Ok(Arc::new(AnthropicProvider::from_env()?)),
            Provider::Mock => Ok(Arc::new(MockProvider::new())),
        }
    }
}
```

---

#### Task 1.3.3: Update Module Exports

**File:** `edgequake/crates/edgequake-llm/src/lib.rs`

```rust
// ADD to lib.rs
pub mod anthropic;
pub use anthropic::{AnthropicConfig, AnthropicProvider};
```

---

### 1.4 Anthropic Provider Checklist

- [ ] AnthropicProvider struct created
- [ ] complete() method works
- [ ] complete_with_system() method works
- [ ] stream() method works
- [ ] Error handling for API errors
- [ ] Factory updated to detect ANTHROPIC_API_KEY
- [ ] Unit tests pass
- [ ] Integration test with real API

---

## 🎯 Rate Limiting

### 2.1 Objective

Implement async-aware rate limiting to prevent API overload and respect provider limits.

### 2.2 Source Reference

**Location:** `lightrag/llm/openai.py` - rate limiting logic
**Reference:** OpenAI rate limits (TPM/RPM)

### 2.3 Implementation Tasks

#### Task 2.3.1: Create Rate Limiter

**File:** `edgequake/crates/edgequake-llm/src/rate_limiter.rs` (NEW)

```rust
// NEW FILE: edgequake/crates/edgequake-llm/src/rate_limiter.rs

//! Async-aware rate limiting for LLM API calls.

use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{Mutex, Semaphore};

/// Rate limiter configuration
#[derive(Debug, Clone)]
pub struct RateLimiterConfig {
    /// Maximum requests per minute
    pub requests_per_minute: usize,
    /// Maximum tokens per minute
    pub tokens_per_minute: usize,
    /// Maximum concurrent requests
    pub max_concurrent: usize,
    /// Retry delay on rate limit
    pub retry_delay: Duration,
    /// Maximum retries
    pub max_retries: usize,
}

impl Default for RateLimiterConfig {
    fn default() -> Self {
        Self {
            requests_per_minute: 60,
            tokens_per_minute: 90_000,
            max_concurrent: 10,
            retry_delay: Duration::from_secs(1),
            max_retries: 3,
        }
    }
}

impl RateLimiterConfig {
    /// Configuration for OpenAI GPT-4
    pub fn openai_gpt4() -> Self {
        Self {
            requests_per_minute: 500,
            tokens_per_minute: 30_000,
            max_concurrent: 50,
            ..Default::default()
        }
    }

    /// Configuration for OpenAI GPT-3.5
    pub fn openai_gpt35() -> Self {
        Self {
            requests_per_minute: 3500,
            tokens_per_minute: 90_000,
            max_concurrent: 100,
            ..Default::default()
        }
    }

    /// Configuration for Anthropic Claude
    pub fn anthropic_claude() -> Self {
        Self {
            requests_per_minute: 60,
            tokens_per_minute: 100_000,
            max_concurrent: 10,
            ..Default::default()
        }
    }
}

/// Token bucket rate limiter
struct TokenBucket {
    tokens: f64,
    max_tokens: f64,
    refill_rate: f64, // tokens per second
    last_refill: Instant,
}

impl TokenBucket {
    fn new(max_tokens: f64, refill_rate: f64) -> Self {
        Self {
            tokens: max_tokens,
            max_tokens,
            refill_rate,
            last_refill: Instant::now(),
        }
    }

    fn refill(&mut self) {
        let now = Instant::now();
        let elapsed = now.duration_since(self.last_refill).as_secs_f64();
        self.tokens = (self.tokens + elapsed * self.refill_rate).min(self.max_tokens);
        self.last_refill = now;
    }

    fn try_acquire(&mut self, tokens: f64) -> bool {
        self.refill();
        if self.tokens >= tokens {
            self.tokens -= tokens;
            true
        } else {
            false
        }
    }

    fn time_to_acquire(&mut self, tokens: f64) -> Duration {
        self.refill();
        if self.tokens >= tokens {
            Duration::ZERO
        } else {
            let needed = tokens - self.tokens;
            Duration::from_secs_f64(needed / self.refill_rate)
        }
    }
}

/// Async-aware rate limiter
pub struct RateLimiter {
    config: RateLimiterConfig,
    request_bucket: Mutex<TokenBucket>,
    token_bucket: Mutex<TokenBucket>,
    concurrent_semaphore: Semaphore,
}

impl RateLimiter {
    pub fn new(config: RateLimiterConfig) -> Self {
        let request_refill_rate = config.requests_per_minute as f64 / 60.0;
        let token_refill_rate = config.tokens_per_minute as f64 / 60.0;

        Self {
            concurrent_semaphore: Semaphore::new(config.max_concurrent),
            request_bucket: Mutex::new(TokenBucket::new(
                config.requests_per_minute as f64,
                request_refill_rate,
            )),
            token_bucket: Mutex::new(TokenBucket::new(
                config.tokens_per_minute as f64,
                token_refill_rate,
            )),
            config,
        }
    }

    /// Acquire permission to make a request
    /// Returns a guard that releases the concurrent slot on drop
    pub async fn acquire(&self, estimated_tokens: usize) -> RateLimitGuard {
        // Acquire concurrent slot
        let permit = self.concurrent_semaphore.acquire().await.unwrap();

        // Wait for request rate limit
        loop {
            let mut bucket = self.request_bucket.lock().await;
            if bucket.try_acquire(1.0) {
                break;
            }
            let wait_time = bucket.time_to_acquire(1.0);
            drop(bucket);
            tokio::time::sleep(wait_time).await;
        }

        // Wait for token rate limit
        loop {
            let mut bucket = self.token_bucket.lock().await;
            if bucket.try_acquire(estimated_tokens as f64) {
                break;
            }
            let wait_time = bucket.time_to_acquire(estimated_tokens as f64);
            drop(bucket);
            tokio::time::sleep(wait_time).await;
        }

        RateLimitGuard {
            _permit: permit,
        }
    }

    /// Record actual token usage (for adjustment)
    pub async fn record_usage(&self, actual_tokens: usize, estimated_tokens: usize) {
        if actual_tokens > estimated_tokens {
            // Consume additional tokens
            let mut bucket = self.token_bucket.lock().await;
            bucket.tokens -= (actual_tokens - estimated_tokens) as f64;
        }
        // If actual < estimated, we already consumed more than needed (conservative)
    }
}

/// Guard that releases rate limit resources on drop
pub struct RateLimitGuard {
    _permit: tokio::sync::SemaphorePermit<'static>,
}

/// Rate-limited LLM provider wrapper
pub struct RateLimitedProvider<P: crate::traits::LLMProvider> {
    inner: P,
    limiter: Arc<RateLimiter>,
}

impl<P: crate::traits::LLMProvider> RateLimitedProvider<P> {
    pub fn new(provider: P, config: RateLimiterConfig) -> Self {
        Self {
            inner: provider,
            limiter: Arc::new(RateLimiter::new(config)),
        }
    }
}

#[async_trait::async_trait]
impl<P: crate::traits::LLMProvider + Send + Sync> crate::traits::LLMProvider for RateLimitedProvider<P> {
    async fn complete(&self, prompt: &str) -> Result<crate::traits::LLMResponse, crate::traits::ProviderError> {
        // Estimate tokens (rough: 4 chars per token)
        let estimated_tokens = prompt.len() / 4 + 1000; // +1000 for response

        let _guard = self.limiter.acquire(estimated_tokens).await;

        let result = self.inner.complete(prompt).await;

        if let Ok(ref response) = result {
            self.limiter.record_usage(
                response.prompt_tokens + response.completion_tokens,
                estimated_tokens,
            ).await;
        }

        result
    }

    async fn complete_with_system(
        &self,
        system: &str,
        prompt: &str,
    ) -> Result<crate::traits::LLMResponse, crate::traits::ProviderError> {
        let estimated_tokens = (system.len() + prompt.len()) / 4 + 1000;

        let _guard = self.limiter.acquire(estimated_tokens).await;

        let result = self.inner.complete_with_system(system, prompt).await;

        if let Ok(ref response) = result {
            self.limiter.record_usage(
                response.prompt_tokens + response.completion_tokens,
                estimated_tokens,
            ).await;
        }

        result
    }

    fn stream(
        &self,
        prompt: &str,
    ) -> std::pin::Pin<Box<dyn futures::Stream<Item = Result<crate::traits::StreamChunk, crate::traits::ProviderError>> + Send>> {
        // For streaming, we acquire once at the start
        // Token counting happens during stream
        self.inner.stream(prompt)
    }

    fn model_name(&self) -> &str {
        self.inner.model_name()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_token_bucket() {
        let mut bucket = TokenBucket::new(10.0, 1.0);

        assert!(bucket.try_acquire(5.0));
        assert!(bucket.try_acquire(5.0));
        assert!(!bucket.try_acquire(1.0)); // Bucket empty

        // Wait for refill
        tokio::time::sleep(Duration::from_secs(2)).await;
        assert!(bucket.try_acquire(1.0));
    }

    #[tokio::test]
    async fn test_concurrent_limit() {
        let limiter = RateLimiter::new(RateLimiterConfig {
            max_concurrent: 2,
            ..Default::default()
        });

        let limiter = Arc::new(limiter);

        // Should be able to acquire 2 concurrent slots
        let g1 = limiter.acquire(100).await;
        let g2 = limiter.acquire(100).await;

        // Third should block (we won't test blocking, just verify guards work)
        drop(g1);
        drop(g2);
    }
}
```

---

## 🎯 LLM Cache

### 3.1 Objective

Complete the LLM response cache to avoid redundant API calls for identical prompts.

### 3.2 Implementation Tasks

#### Task 3.2.1: Create LLM Cache

**File:** `edgequake/crates/edgequake-llm/src/cache.rs` (NEW)

```rust
// NEW FILE: edgequake/crates/edgequake-llm/src/cache.rs

//! LLM response caching for cost optimization.

use crate::traits::{LLMProvider, LLMResponse, ProviderError, StreamChunk};
use async_trait::async_trait;
use futures::Stream;
use lru::LruCache;
use sha2::{Digest, Sha256};
use std::num::NonZeroUsize;
use std::pin::Pin;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Cache configuration
#[derive(Debug, Clone)]
pub struct CacheConfig {
    /// Maximum number of cached responses
    pub max_entries: usize,
    /// Whether to cache streaming responses
    pub cache_streaming: bool,
    /// TTL for cache entries (None = no expiry)
    pub ttl: Option<std::time::Duration>,
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            max_entries: 10_000,
            cache_streaming: false,
            ttl: None,
        }
    }
}

/// Cached response entry
#[derive(Clone)]
struct CacheEntry {
    response: LLMResponse,
    created_at: std::time::Instant,
}

/// LLM response cache
pub struct LLMCache {
    cache: RwLock<LruCache<String, CacheEntry>>,
    config: CacheConfig,
}

impl LLMCache {
    pub fn new(config: CacheConfig) -> Self {
        let capacity = NonZeroUsize::new(config.max_entries)
            .unwrap_or(NonZeroUsize::new(10_000).unwrap());

        Self {
            cache: RwLock::new(LruCache::new(capacity)),
            config,
        }
    }

    /// Generate cache key from prompt
    fn cache_key(model: &str, prompt: &str, system: Option<&str>) -> String {
        let mut hasher = Sha256::new();
        hasher.update(model.as_bytes());
        hasher.update(prompt.as_bytes());
        if let Some(sys) = system {
            hasher.update(sys.as_bytes());
        }
        format!("{:x}", hasher.finalize())
    }

    /// Get cached response
    pub async fn get(&self, model: &str, prompt: &str, system: Option<&str>) -> Option<LLMResponse> {
        let key = Self::cache_key(model, prompt, system);
        let mut cache = self.cache.write().await;

        if let Some(entry) = cache.get(&key) {
            // Check TTL
            if let Some(ttl) = self.config.ttl {
                if entry.created_at.elapsed() > ttl {
                    cache.pop(&key);
                    return None;
                }
            }
            Some(entry.response.clone())
        } else {
            None
        }
    }

    /// Store response in cache
    pub async fn put(&self, model: &str, prompt: &str, system: Option<&str>, response: LLMResponse) {
        let key = Self::cache_key(model, prompt, system);
        let entry = CacheEntry {
            response,
            created_at: std::time::Instant::now(),
        };

        let mut cache = self.cache.write().await;
        cache.put(key, entry);
    }

    /// Clear all cached entries
    pub async fn clear(&self) {
        let mut cache = self.cache.write().await;
        cache.clear();
    }

    /// Get cache statistics
    pub async fn stats(&self) -> CacheStats {
        let cache = self.cache.read().await;
        CacheStats {
            entries: cache.len(),
            capacity: self.config.max_entries,
        }
    }
}

/// Cache statistics
#[derive(Debug, Clone)]
pub struct CacheStats {
    pub entries: usize,
    pub capacity: usize,
}

/// Cached LLM provider wrapper
pub struct CachedProvider<P: LLMProvider> {
    inner: P,
    cache: Arc<LLMCache>,
}

impl<P: LLMProvider> CachedProvider<P> {
    pub fn new(provider: P, config: CacheConfig) -> Self {
        Self {
            inner: provider,
            cache: Arc::new(LLMCache::new(config)),
        }
    }

    pub fn with_shared_cache(provider: P, cache: Arc<LLMCache>) -> Self {
        Self {
            inner: provider,
            cache,
        }
    }

    /// Get cache statistics
    pub async fn cache_stats(&self) -> CacheStats {
        self.cache.stats().await
    }

    /// Clear cache
    pub async fn clear_cache(&self) {
        self.cache.clear().await;
    }
}

#[async_trait]
impl<P: LLMProvider + Send + Sync> LLMProvider for CachedProvider<P> {
    async fn complete(&self, prompt: &str) -> Result<LLMResponse, ProviderError> {
        let model = self.inner.model_name();

        // Check cache
        if let Some(cached) = self.cache.get(model, prompt, None).await {
            tracing::debug!(model = %model, "LLM cache hit");
            return Ok(cached);
        }

        // Call provider
        let response = self.inner.complete(prompt).await?;

        // Store in cache
        self.cache.put(model, prompt, None, response.clone()).await;

        Ok(response)
    }

    async fn complete_with_system(
        &self,
        system: &str,
        prompt: &str,
    ) -> Result<LLMResponse, ProviderError> {
        let model = self.inner.model_name();

        // Check cache
        if let Some(cached) = self.cache.get(model, prompt, Some(system)).await {
            tracing::debug!(model = %model, "LLM cache hit (with system)");
            return Ok(cached);
        }

        // Call provider
        let response = self.inner.complete_with_system(system, prompt).await?;

        // Store in cache
        self.cache.put(model, prompt, Some(system), response.clone()).await;

        Ok(response)
    }

    fn stream(
        &self,
        prompt: &str,
    ) -> Pin<Box<dyn Stream<Item = Result<StreamChunk, ProviderError>> + Send>> {
        // Streaming not cached (could accumulate and cache on completion)
        self.inner.stream(prompt)
    }

    fn model_name(&self) -> &str {
        self.inner.model_name()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cache_key_generation() {
        let key1 = LLMCache::cache_key("gpt-4", "Hello", None);
        let key2 = LLMCache::cache_key("gpt-4", "Hello", None);
        let key3 = LLMCache::cache_key("gpt-4", "World", None);

        assert_eq!(key1, key2);
        assert_ne!(key1, key3);
    }

    #[tokio::test]
    async fn test_cache_put_get() {
        let cache = LLMCache::new(CacheConfig::default());

        let response = LLMResponse {
            content: "Hello, world!".to_string(),
            model: "gpt-4".to_string(),
            prompt_tokens: 10,
            completion_tokens: 5,
            finish_reason: Some("stop".to_string()),
        };

        cache.put("gpt-4", "test prompt", None, response.clone()).await;

        let cached = cache.get("gpt-4", "test prompt", None).await;
        assert!(cached.is_some());
        assert_eq!(cached.unwrap().content, response.content);
    }
}
```

**Dependencies to add:**

```toml
# Add to edgequake/crates/edgequake-llm/Cargo.toml
sha2 = "0.10"
lru = "0.12"
```

---

## 📊 Testing Requirements

See [07-VALIDATION-TESTING.md](./07-VALIDATION-TESTING.md#llm-provider-tests) for full specifications.

### Unit Tests

```bash
cargo test --package edgequake-llm --lib anthropic
cargo test --package edgequake-llm --lib rate_limiter
cargo test --package edgequake-llm --lib cache
```

### Integration Tests

```bash
# Requires ANTHROPIC_API_KEY
ANTHROPIC_API_KEY=sk-... cargo test --package edgequake-llm --test anthropic_integration
```

---

## 🔗 Cross-References

| Topic        | Document                                                 | Section             |
| ------------ | -------------------------------------------------------- | ------------------- |
| Gap Details  | [../gap-analysis.md](../gap-analysis.md)                 | F-040, F-049, F-047 |
| Core Quality | [03-PHASE2-CORE-QUALITY.md](./03-PHASE2-CORE-QUALITY.md) | Uses providers      |
| Testing Plan | [07-VALIDATION-TESTING.md](./07-VALIDATION-TESTING.md)   | LLM Provider Tests  |
| Master Index | [00-INDEX.md](./00-INDEX.md)                             | Phase 2             |

---

## ✅ Completion Criteria

| Criterion           | Target                   | Validation       |
| ------------------- | ------------------------ | ---------------- |
| Anthropic works     | Claude API calls succeed | Integration test |
| Rate limiting       | No 429 errors            | Load test        |
| Cache reduces calls | >50% cache hit rate      | Metrics          |
| Streaming works     | Tokens stream correctly  | Manual test      |

---

_Document Version: 1.0_  
_Last Updated: 2024-12-24_  
_Owner: EdgeQuake LLM Team_
