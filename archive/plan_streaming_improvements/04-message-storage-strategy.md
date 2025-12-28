# Message Storage Strategy: Full API Response vs Streaming Concatenation

## 1. Problem Statement

EdgeQuake currently accumulates streaming tokens in memory and stores the concatenated result. This approach has several issues:

1. **Token count inaccuracy**: Counts chunks, not actual tokens
2. **Missing API metadata**: Usage stats, finish reason, model info lost
3. **No distinction**: Original API response vs reconstructed content
4. **Missing thinking content**: Reasoning models have separate thinking/content

## 2. OpenAI API Response Structure

### 2.1 Streaming Response (SSE chunks)

```json
// First chunk
{"id":"chatcmpl-123","object":"chat.completion.chunk","created":1694268190,
 "model":"gpt-4","choices":[{"index":0,"delta":{"role":"assistant","content":""},"finish_reason":null}]}

// Content chunks
{"id":"chatcmpl-123","object":"chat.completion.chunk","created":1694268190,
 "model":"gpt-4","choices":[{"index":0,"delta":{"content":"Hello"},"finish_reason":null}]}

{"id":"chatcmpl-123","object":"chat.completion.chunk","created":1694268190,
 "model":"gpt-4","choices":[{"index":0,"delta":{"content":" world"},"finish_reason":null}]}

// Final chunk with usage
{"id":"chatcmpl-123","object":"chat.completion.chunk","created":1694268190,
 "model":"gpt-4","choices":[{"index":0,"delta":{},"finish_reason":"stop"}],
 "usage":{"prompt_tokens":10,"completion_tokens":5,"total_tokens":15}}
```

### 2.2 Non-Streaming Response

```json
{
  "id": "chatcmpl-123",
  "object": "chat.completion",
  "created": 1694268190,
  "model": "gpt-4",
  "choices": [
    {
      "index": 0,
      "message": {
        "role": "assistant",
        "content": "Hello world"
      },
      "finish_reason": "stop"
    }
  ],
  "usage": {
    "prompt_tokens": 10,
    "completion_tokens": 5,
    "total_tokens": 15
  }
}
```

### 2.3 Reasoning Model Response (o1, o3)

```json
{
  "id": "chatcmpl-123",
  "object": "chat.completion",
  "model": "o1-preview",
  "choices": [
    {
      "message": {
        "role": "assistant",
        "content": "The answer is 42.",
        "reasoning_content": "Let me think through this step by step..."
      },
      "finish_reason": "stop"
    }
  ],
  "usage": {
    "prompt_tokens": 50,
    "completion_tokens": 200,
    "reasoning_tokens": 150,
    "total_tokens": 250
  }
}
```

## 3. Proposed Data Model

### 3.1 Enhanced Message Schema

```sql
-- Enhanced messages table
CREATE TABLE messages (
    message_id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    conversation_id UUID NOT NULL REFERENCES conversations(conversation_id) ON DELETE CASCADE,
    parent_id UUID REFERENCES messages(message_id),
    role VARCHAR(20) NOT NULL CHECK (role IN ('user', 'assistant', 'system')),

    -- Content fields
    content TEXT NOT NULL,
    reasoning_content TEXT,  -- For reasoning models

    -- API response metadata
    api_response_id VARCHAR(100),  -- chatcmpl-xxx
    model_used VARCHAR(100),       -- Actual model from response
    finish_reason VARCHAR(50),     -- stop, length, content_filter, tool_calls

    -- Token usage (from API)
    prompt_tokens INTEGER,
    completion_tokens INTEGER,
    reasoning_tokens INTEGER,
    total_tokens INTEGER,

    -- Performance metrics
    duration_ms INTEGER,
    thinking_time_ms INTEGER,
    first_token_ms INTEGER,  -- Time to first token

    -- Query context
    mode VARCHAR(20),
    context JSONB,

    -- Streaming state
    is_streaming BOOLEAN DEFAULT FALSE,
    stream_completed_at TIMESTAMPTZ,

    -- Error handling
    is_error BOOLEAN DEFAULT FALSE,
    error_code VARCHAR(50),
    error_message TEXT,

    -- Timestamps
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW()
);

-- Index for streaming state queries
CREATE INDEX idx_messages_streaming ON messages(conversation_id, is_streaming)
    WHERE is_streaming = TRUE;
```

### 3.2 Rust Type Definitions

```rust
/// Enhanced message with full API response data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub message_id: Uuid,
    pub conversation_id: Uuid,
    pub parent_id: Option<Uuid>,
    pub role: MessageRole,

    // Content
    pub content: String,
    pub reasoning_content: Option<String>,

    // API Response Metadata
    pub api_response: Option<ApiResponseMetadata>,

    // Performance
    pub performance: MessagePerformance,

    // Context
    pub mode: Option<ConversationMode>,
    pub context: Option<MessageContext>,

    // Streaming State
    pub streaming_state: StreamingState,

    // Error
    pub error: Option<MessageError>,

    // Timestamps
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Metadata from LLM API response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiResponseMetadata {
    /// API response ID (e.g., chatcmpl-xxx)
    pub response_id: String,
    /// Actual model used (may differ from requested)
    pub model: String,
    /// How the response ended
    pub finish_reason: FinishReason,
    /// Token usage from API
    pub usage: TokenUsage,
}

/// Token usage statistics from API
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenUsage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub reasoning_tokens: Option<u32>,
    pub total_tokens: u32,
}

/// Why the model stopped generating
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FinishReason {
    Stop,
    Length,
    ContentFilter,
    ToolCalls,
    FunctionCall,
    Unknown,
}

/// Performance metrics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MessagePerformance {
    pub duration_ms: Option<u32>,
    pub thinking_time_ms: Option<u32>,
    pub first_token_ms: Option<u32>,
    pub tokens_per_second: Option<f32>,
}

/// Streaming state tracking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamingState {
    pub is_streaming: bool,
    pub stream_started_at: Option<DateTime<Utc>>,
    pub stream_completed_at: Option<DateTime<Utc>>,
    pub chunks_received: u32,
    pub bytes_received: usize,
}

impl Default for StreamingState {
    fn default() -> Self {
        Self {
            is_streaming: false,
            stream_started_at: None,
            stream_completed_at: None,
            chunks_received: 0,
            bytes_received: 0,
        }
    }
}

/// Error information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageError {
    pub code: String,
    pub message: String,
    pub retryable: bool,
}
```

## 4. Streaming Handler Improvements

### 4.1 Stream Accumulator

```rust
/// Accumulates streaming response with full metadata
pub struct StreamAccumulator {
    message_id: Uuid,

    // Content accumulation
    content: String,
    reasoning_content: String,

    // Metadata from first chunk
    response_id: Option<String>,
    model: Option<String>,

    // Final chunk data
    finish_reason: Option<FinishReason>,
    usage: Option<TokenUsage>,

    // Performance tracking
    start_time: Instant,
    first_token_time: Option<Instant>,
    chunk_count: u32,
    bytes_received: usize,
}

impl StreamAccumulator {
    pub fn new(message_id: Uuid) -> Self {
        Self {
            message_id,
            content: String::new(),
            reasoning_content: String::new(),
            response_id: None,
            model: None,
            finish_reason: None,
            usage: None,
            start_time: Instant::now(),
            first_token_time: None,
            chunk_count: 0,
            bytes_received: 0,
        }
    }

    /// Process an SSE chunk
    pub fn process_chunk(&mut self, chunk: &StreamChunk) -> Result<(), Error> {
        self.chunk_count += 1;

        // Capture first chunk metadata
        if self.response_id.is_none() {
            self.response_id = Some(chunk.id.clone());
            self.model = Some(chunk.model.clone());
        }

        // Track first token timing
        if self.first_token_time.is_none() && !chunk.delta_content().is_empty() {
            self.first_token_time = Some(Instant::now());
        }

        // Accumulate content
        if let Some(content) = &chunk.choices[0].delta.content {
            self.content.push_str(content);
            self.bytes_received += content.len();
        }

        // Accumulate reasoning content (for o1, o3 models)
        if let Some(reasoning) = &chunk.choices[0].delta.reasoning_content {
            self.reasoning_content.push_str(reasoning);
            self.bytes_received += reasoning.len();
        }

        // Capture final chunk data
        if let Some(reason) = &chunk.choices[0].finish_reason {
            self.finish_reason = Some(parse_finish_reason(reason));
        }

        if let Some(usage) = &chunk.usage {
            self.usage = Some(TokenUsage {
                prompt_tokens: usage.prompt_tokens,
                completion_tokens: usage.completion_tokens,
                reasoning_tokens: usage.reasoning_tokens,
                total_tokens: usage.total_tokens,
            });
        }

        Ok(())
    }

    /// Finalize and build the complete message data
    pub fn finalize(self) -> FinalizedMessage {
        let duration = self.start_time.elapsed();
        let first_token_ms = self.first_token_time
            .map(|t| t.duration_since(self.start_time).as_millis() as u32);

        let tokens_per_second = self.usage.as_ref().map(|u| {
            let secs = duration.as_secs_f32();
            if secs > 0.0 {
                u.completion_tokens as f32 / secs
            } else {
                0.0
            }
        });

        FinalizedMessage {
            content: self.content,
            reasoning_content: if self.reasoning_content.is_empty() {
                None
            } else {
                Some(self.reasoning_content)
            },
            api_response: self.response_id.map(|id| ApiResponseMetadata {
                response_id: id,
                model: self.model.unwrap_or_default(),
                finish_reason: self.finish_reason.unwrap_or(FinishReason::Unknown),
                usage: self.usage.unwrap_or_default(),
            }),
            performance: MessagePerformance {
                duration_ms: Some(duration.as_millis() as u32),
                thinking_time_ms: None,
                first_token_ms,
                tokens_per_second,
            },
            streaming_state: StreamingState {
                is_streaming: false,
                stream_started_at: None,
                stream_completed_at: Some(Utc::now()),
                chunks_received: self.chunk_count,
                bytes_received: self.bytes_received,
            },
        }
    }
}

/// Result of stream finalization
pub struct FinalizedMessage {
    pub content: String,
    pub reasoning_content: Option<String>,
    pub api_response: Option<ApiResponseMetadata>,
    pub performance: MessagePerformance,
    pub streaming_state: StreamingState,
}
```

### 4.2 Updated Chat Handler

```rust
pub async fn chat_completion_stream(
    State(state): State<AppState>,
    tenant_ctx: TenantContext,
    Json(request): Json<ChatCompletionRequest>,
) -> ApiResult<Sse<impl futures::Stream<Item = Result<Event, Infallible>>>> {
    // ... validation and setup ...

    // Create placeholder message
    let assistant_message = state
        .conversation_service
        .create_message(
            conversation_id,
            CreateMessageRequest {
                content: String::new(),
                role: MessageRole::Assistant,
                parent_id: Some(user_message_id),
                stream: true,
            },
        )
        .await?;

    let (tx, rx) = mpsc::channel::<ChatStreamEvent>(100);

    tokio::spawn(async move {
        // Initialize accumulator
        let mut accumulator = StreamAccumulator::new(assistant_message.message_id);

        // Stream from query engine
        match state.query_engine.query_stream(engine_request).await {
            Ok(mut stream) => {
                while let Some(chunk_result) = stream.next().await {
                    match chunk_result {
                        Ok(chunk) => {
                            // Process the chunk (extract content)
                            let content = chunk.content.clone();

                            // For full API response tracking, we'd need the raw chunk
                            // This is a simplified version
                            accumulator.process_text(&content);

                            // Send to client
                            if tx.send(ChatStreamEvent::Token { content }).await.is_err() {
                                break;
                            }
                        }
                        Err(e) => {
                            // Handle error...
                            break;
                        }
                    }
                }
            }
            Err(e) => {
                // Handle error...
                return;
            }
        }

        // Finalize and store full message
        let finalized = accumulator.finalize();

        // Update message with complete data
        match state.conversation_service
            .update_message(
                assistant_message.message_id,
                UpdateMessageRequest {
                    content: Some(finalized.content),
                    reasoning_content: finalized.reasoning_content,
                    tokens_used: finalized.api_response
                        .as_ref()
                        .map(|r| r.usage.completion_tokens as i32),
                    prompt_tokens: finalized.api_response
                        .as_ref()
                        .map(|r| r.usage.prompt_tokens as i32),
                    duration_ms: finalized.performance.duration_ms.map(|d| d as i32),
                    first_token_ms: finalized.performance.first_token_ms.map(|d| d as i32),
                    api_response_id: finalized.api_response
                        .as_ref()
                        .map(|r| r.response_id.clone()),
                    model_used: finalized.api_response
                        .as_ref()
                        .map(|r| r.model.clone()),
                    finish_reason: finalized.api_response
                        .as_ref()
                        .map(|r| r.finish_reason.to_string()),
                    is_streaming: Some(false),
                    stream_completed_at: Some(Utc::now()),
                    is_error: None,
                    context: None,
                },
            )
            .await
        {
            Ok(_) => {
                let _ = tx.send(ChatStreamEvent::Done {
                    assistant_message_id: assistant_message.message_id,
                    tokens_used: finalized.api_response
                        .map(|r| r.usage.completion_tokens)
                        .unwrap_or(0),
                    duration_ms: finalized.performance.duration_ms.unwrap_or(0) as u64,
                }).await;
            }
            Err(e) => {
                let _ = tx.send(ChatStreamEvent::Error {
                    message: e.to_string(),
                    code: "SAVE_FAILED".to_string(),
                }).await;
            }
        }
    });

    // Return SSE stream...
}
```

## 5. Benefits Summary

| Aspect              | Before                   | After                      |
| ------------------- | ------------------------ | -------------------------- |
| Token count         | Chunk count (inaccurate) | API usage (accurate)       |
| Model info          | Request model only       | Actual model from response |
| Finish reason       | None                     | stop/length/content_filter |
| Reasoning           | Not captured             | Separate field             |
| First token latency | Not tracked              | Measured                   |
| Tokens/second       | Not tracked              | Calculated                 |
| API response ID     | Not stored               | Stored for debugging       |

## 6. Migration Strategy

### 6.1 Database Migration

```sql
-- Add new columns to messages table
ALTER TABLE messages
    ADD COLUMN reasoning_content TEXT,
    ADD COLUMN api_response_id VARCHAR(100),
    ADD COLUMN model_used VARCHAR(100),
    ADD COLUMN finish_reason VARCHAR(50),
    ADD COLUMN prompt_tokens INTEGER,
    ADD COLUMN reasoning_tokens INTEGER,
    ADD COLUMN first_token_ms INTEGER,
    ADD COLUMN is_streaming BOOLEAN DEFAULT FALSE,
    ADD COLUMN stream_completed_at TIMESTAMPTZ;

-- Rename existing column for clarity
ALTER TABLE messages RENAME COLUMN tokens_used TO completion_tokens;

-- Add computed column for total tokens
ALTER TABLE messages
    ADD COLUMN total_tokens INTEGER
    GENERATED ALWAYS AS (COALESCE(prompt_tokens, 0) + COALESCE(completion_tokens, 0)) STORED;
```

### 6.2 Backward Compatibility

```rust
impl Message {
    /// Get tokens_used for backward compatibility
    pub fn tokens_used(&self) -> Option<i32> {
        self.api_response
            .as_ref()
            .map(|r| r.usage.completion_tokens as i32)
            .or(self.legacy_tokens_used)
    }
}
```

---

_Document Version: 1.0_
_Created: 2024-12-28_
