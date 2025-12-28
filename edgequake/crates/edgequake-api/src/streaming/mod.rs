//! Streaming utilities for chat completion.
//!
//! This module provides utilities for accumulating streaming responses,
//! tracking token usage accurately, and managing API response metadata.

pub mod accumulator;
pub mod flush_manager;

pub use accumulator::{ApiResponseMetadata, StreamAccumulator, TokenUsage};
pub use flush_manager::{FlushConfig, FlushHandle, StreamFlushManager};
