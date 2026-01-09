//! Streaming utilities for chat completion.
//!
//! This module provides utilities for accumulating streaming responses,
//! tracking token usage accurately, and managing API response metadata.
//!
//! ## Implements
//!
//! - [`FEAT0480`]: Streaming response accumulation
//! - [`FEAT0481`]: Token usage tracking
//! - [`FEAT0482`]: Periodic flush management
//!
//! ## Use Cases
//!
//! - [`UC2080`]: System streams LLM response to client
//! - [`UC2081`]: System tracks token usage for billing
//!
//! ## Enforces
//!
//! - [`BR0480`]: Accurate token counting from API metadata
//! - [`BR0481`]: Debounced database writes during streaming
//!
//! # Architecture
//!
//! The streaming system consists of two main components:
//!
//! - **StreamAccumulator**: Accumulates partial responses from LLM providers,
//!   tracking tokens, timing, and metadata
//! - **StreamFlushManager**: Manages periodic flushing of accumulated data to
//!   storage with debouncing and backpressure control
//!
//! # Usage Example
//!
//! ```rust,ignore
//! use edgequake_api::streaming::{StreamAccumulator, StreamFlushManager, FlushConfig};
//!
//! // Create accumulator for a conversation
//! let mut accumulator = StreamAccumulator::new(
//!     conversation_id,
//!     message_id,
//!     user_id,
//!     Some(workspace_id),
//! );
//!
//! // Process streaming chunks
//! for chunk in llm_stream {
//!     accumulator.add_chunk(&chunk.content);
//!     
//!     if let Some(usage) = chunk.usage {
//!         accumulator.add_usage(usage.input_tokens, usage.output_tokens);
//!     }
//! }
//!
//! // Get final result
//! let result = accumulator.finalize();
//! ```
//!
//! # Flush Management
//!
//! ```rust,ignore
//! // Configure periodic flushing
//! let config = FlushConfig {
//!     debounce_ms: 500,        // Wait 500ms after last change
//!     min_tokens_for_flush: 50, // Flush when 50+ tokens accumulated
//!     max_pending_flushes: 10,  // Limit concurrent flush operations
//! };
//!
//! let manager = StreamFlushManager::new(config, kv_storage);
//! let handle = manager.start_tracking(conversation_id, message_id);
//!
//! // Update periodically
//! handle.update_content(accumulated_content.clone()).await;
//!
//! // Final flush
//! handle.force_flush().await?;
//! ```
//!
//! # Performance Characteristics
//!
//! - **Memory**: O(n) where n is accumulated content length
//! - **Latency**: Debounced flushes reduce database writes by 5-10x
//! - **Throughput**: Handles 100+ concurrent streams per instance
//!
//! # Thread Safety
//!
//! Both components are designed for concurrent access:
//! - StreamAccumulator: Not thread-safe (use per-stream)
//! - StreamFlushManager: Thread-safe (shared across handlers)

pub mod accumulator;
pub mod flush_manager;

pub use accumulator::{ApiResponseMetadata, StreamAccumulator, TokenUsage};
pub use flush_manager::{FlushConfig, FlushHandle, StreamFlushManager};
