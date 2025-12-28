# Debounce Strategy for Streaming Token Storage

## 1. Problem Statement

During LLM streaming, tokens arrive at high frequency (10-100+ per second). Storing each token immediately would:

- Create excessive database pressure (100+ writes per response)
- Cause lock contention on message rows
- Waste I/O bandwidth on tiny updates
- Risk transaction log bloat

## 2. Design Goals

| Goal             | Metric               | Target     |
| ---------------- | -------------------- | ---------- |
| Reduce DB writes | Writes per stream    | ≤5 writes  |
| Preserve data    | Max data loss window | ≤2 seconds |
| Low latency      | Write delay overhead | ≤50ms      |
| Memory efficient | Buffer size          | ≤10KB      |

## 3. Debounce Patterns Analyzed

### 3.1 Fixed Interval Debounce

```
Tokens: T1 T2 T3 T4 T5 ... T50 T51 ... T100
        │                    │              │
        ▼                    ▼              ▼
Writes: ────────[W1]─────────[W2]───────────[W3]
        0ms    500ms       1000ms         1500ms
```

**Pros**: Predictable, simple implementation
**Cons**: May write during active streaming (unnecessary)

### 3.2 Trailing Edge Debounce (Recommended)

```
Tokens: T1 T2 T3 T4 ... [pause 500ms] ... T50 T51 ... [END]
        │                       │                       │
        ▼                       ▼                       ▼
Writes: (accumulate)──────────[W1]──(accumulate)───────[W2]
```

**Pros**: Only writes during natural pauses, final write on completion
**Cons**: Slight complexity in timer management

### 3.3 Hybrid: Trailing + Max Wait (Best)

```
Tokens: T1 T2 ... (continuous for 3s) ... T200 [END]
        │              │                     │
        ▼              ▼                     ▼
Writes: (buffer)─────[W1]─────(buffer)─────[W2]
        0ms         2000ms                Final
```

**Pros**: Guarantees writes even during continuous streaming
**Cons**: More state to manage

## 4. Proposed Implementation

### 4.1 Core Data Structures

```rust
/// Streaming message buffer with debounce state
pub struct StreamingMessageBuffer {
    /// Message ID being streamed
    message_id: Uuid,
    /// Conversation ID
    conversation_id: Uuid,
    /// Accumulated content
    content: String,
    /// Token count (actual, not chunk count)
    tokens: u32,
    /// Last token received timestamp
    last_token_at: Instant,
    /// Last database write timestamp
    last_write_at: Option<Instant>,
    /// Total bytes since last write
    bytes_since_write: usize,
}

/// Debounce configuration
pub struct DebounceConfig {
    /// Minimum time between writes (trailing edge)
    pub write_delay: Duration,          // default: 500ms
    /// Maximum time without write (force write)
    pub max_buffer_time: Duration,      // default: 2000ms
    /// Maximum bytes before force write
    pub max_buffer_bytes: usize,        // default: 8192
    /// Final write delay after stream ends
    pub final_delay: Duration,          // default: 100ms
}

impl Default for DebounceConfig {
    fn default() -> Self {
        Self {
            write_delay: Duration::from_millis(500),
            max_buffer_time: Duration::from_millis(2000),
            max_buffer_bytes: 8192,
            final_delay: Duration::from_millis(100),
        }
    }
}
```

### 4.2 Debounce State Machine

```
                    ┌────────────────┐
                    │    IDLE        │
                    │ (no active     │
                    │  stream)       │
                    └───────┬────────┘
                            │ stream_start()
                            ▼
                    ┌────────────────┐
                    │   BUFFERING    │◀──────────┐
                    │ (accumulating  │           │
                    │  tokens)       │───────────┤ token_received()
                    └───────┬────────┘           │
                            │                    │
            ┌───────────────┼───────────────┐    │
            │               │               │    │
            ▼               ▼               ▼    │
    [timer expires]  [max_time hit]  [max_bytes hit]
            │               │               │
            └───────────────┼───────────────┘
                            │
                            ▼
                    ┌────────────────┐
                    │   FLUSHING     │
                    │ (writing to DB)│
                    └───────┬────────┘
                            │ write_complete()
                            ▼
                    ┌────────────────┐
                    │   BUFFERING    │ (continue streaming)
                    └────────────────┘
                            │
                            │ stream_end()
                            ▼
                    ┌────────────────┐
                    │  FINALIZING    │
                    │ (final write)  │
                    └───────┬────────┘
                            │
                            ▼
                    ┌────────────────┐
                    │    IDLE        │
                    └────────────────┘
```

### 4.3 Core Logic Implementation

```rust
impl StreamingMessageBuffer {
    /// Check if we should flush to database
    pub fn should_flush(&self, config: &DebounceConfig) -> bool {
        let now = Instant::now();

        // Force flush if max bytes exceeded
        if self.bytes_since_write >= config.max_buffer_bytes {
            return true;
        }

        // Force flush if max time exceeded since last write
        if let Some(last_write) = self.last_write_at {
            if now.duration_since(last_write) >= config.max_buffer_time {
                return true;
            }
        } else if now.duration_since(self.created_at) >= config.max_buffer_time {
            // First write after max_buffer_time
            return true;
        }

        // Trailing edge: flush if no tokens for write_delay
        now.duration_since(self.last_token_at) >= config.write_delay
    }

    /// Append a token to the buffer
    pub fn append(&mut self, text: &str) {
        self.content.push_str(text);
        self.bytes_since_write += text.len();
        self.tokens += 1; // This should be actual token count from API
        self.last_token_at = Instant::now();
    }

    /// Reset after successful flush
    pub fn mark_flushed(&mut self) {
        self.last_write_at = Some(Instant::now());
        self.bytes_since_write = 0;
    }
}
```

### 4.4 Task-Based Flush Manager

```rust
/// Manages debounced writes for all active streams
pub struct StreamFlushManager {
    /// Active streaming buffers
    buffers: Arc<RwLock<HashMap<Uuid, StreamingMessageBuffer>>>,
    /// Database pool
    pool: PgPool,
    /// Configuration
    config: DebounceConfig,
    /// Background task handle
    flush_task: Option<JoinHandle<()>>,
    /// Shutdown signal
    shutdown: watch::Sender<bool>,
}

impl StreamFlushManager {
    /// Start the background flush task
    pub async fn start(&mut self) {
        let buffers = self.buffers.clone();
        let pool = self.pool.clone();
        let config = self.config.clone();
        let mut shutdown_rx = self.shutdown.subscribe();

        self.flush_task = Some(tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_millis(100));

            loop {
                tokio::select! {
                    _ = interval.tick() => {
                        Self::check_and_flush(&buffers, &pool, &config).await;
                    }
                    _ = shutdown_rx.changed() => {
                        // Final flush on shutdown
                        Self::flush_all(&buffers, &pool).await;
                        break;
                    }
                }
            }
        }));
    }

    async fn check_and_flush(
        buffers: &Arc<RwLock<HashMap<Uuid, StreamingMessageBuffer>>>,
        pool: &PgPool,
        config: &DebounceConfig,
    ) {
        let to_flush: Vec<_> = {
            let guard = buffers.read().await;
            guard.iter()
                .filter(|(_, buf)| buf.should_flush(config))
                .map(|(id, buf)| (*id, buf.content.clone()))
                .collect()
        };

        for (message_id, content) in to_flush {
            if let Err(e) = Self::write_checkpoint(pool, message_id, &content).await {
                tracing::warn!("Checkpoint write failed: {}", e);
            } else {
                let mut guard = buffers.write().await;
                if let Some(buf) = guard.get_mut(&message_id) {
                    buf.mark_flushed();
                }
            }
        }
    }

    async fn write_checkpoint(pool: &PgPool, message_id: Uuid, content: &str) -> Result<()> {
        sqlx::query(
            "UPDATE messages SET content = $1, updated_at = NOW() WHERE message_id = $2"
        )
        .bind(content)
        .bind(message_id)
        .execute(pool)
        .await?;

        Ok(())
    }
}
```

## 5. Integration with Chat Handler

### 5.1 Modified Streaming Flow

```rust
// In chat_completion_stream handler

// 1. Create placeholder message BEFORE streaming
let assistant_message = state
    .conversation_service
    .create_message(
        conversation_id,
        CreateMessageRequest {
            content: String::new(),  // Empty placeholder
            role: MessageRole::Assistant,
            parent_id: Some(user_message_id),
            stream: true,
        },
    )
    .await?;

// 2. Register buffer with flush manager
state.flush_manager.register(
    assistant_message.message_id,
    conversation_id,
).await;

// 3. Stream tokens, appending to buffer
while let Some(chunk_result) = stream.next().await {
    match chunk_result {
        Ok(text) => {
            state.flush_manager.append(
                assistant_message.message_id,
                &text
            ).await;

            // Send to client
            let _ = tx.send(ChatStreamEvent::Token { content: text }).await;
        }
        Err(e) => {
            // Handle error...
        }
    }
}

// 4. Finalize - ensure last content is written
let final_content = state.flush_manager
    .finalize(assistant_message.message_id)
    .await?;

// 5. Update with final metadata (tokens, duration, context)
state.conversation_service.update_message(
    assistant_message.message_id,
    UpdateMessageRequest {
        content: None,  // Already written by flush manager
        tokens_used: Some(actual_token_count),
        duration_ms: Some(duration_ms as i32),
        // ...
    },
).await?;
```

## 6. Benefits Summary

| Aspect               | Before          | After                     |
| -------------------- | --------------- | ------------------------- |
| DB writes per stream | 1 (final only)  | 3-5 (checkpoints + final) |
| Data loss on crash   | Entire response | ≤2 seconds of content     |
| Memory pressure      | Full response   | Rolling buffer            |
| Recovery capability  | None            | Resume from checkpoint    |

## 7. Configuration Recommendations

### Development

```toml
[streaming.debounce]
write_delay = "1000ms"
max_buffer_time = "5000ms"
max_buffer_bytes = 16384
```

### Production (high traffic)

```toml
[streaming.debounce]
write_delay = "500ms"
max_buffer_time = "2000ms"
max_buffer_bytes = 8192
```

### Low-latency (real-time)

```toml
[streaming.debounce]
write_delay = "200ms"
max_buffer_time = "1000ms"
max_buffer_bytes = 4096
```

---

_Document Version: 1.0_
_Created: 2024-12-28_
