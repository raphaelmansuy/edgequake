//! Debounced flush manager for streaming responses.
//!
//! This module provides trailing-edge debouncing for database writes
//! during streaming, ensuring crash recovery while minimizing DB load.
//!
//! ## Implements
//!
//! - [`FEAT0486`]: Trailing-edge debouncing
//! - [`FEAT0487`]: Configurable flush thresholds
//! - [`FEAT0488`]: Background flush task
//!
//! ## Use Cases
//!
//! - [`UC2084`]: System persists streaming content periodically
//! - [`UC2085`]: System recovers conversation on crash
//!
//! ## Enforces
//!
//! - [`BR0486`]: Maximum buffer time limit
//! - [`BR0487`]: Maximum buffer bytes limit

use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::mpsc;
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
pub enum FlushMessage {
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
        let _ = self
            .tx
            .send(FlushMessage::ContentUpdated { content, tokens })
            .await;
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

/// Type alias for the save function used by flush manager.
pub type SaveFn = Arc<
    dyn Fn(String, u32) -> Pin<Box<dyn Future<Output = Result<(), String>> + Send>> + Send + Sync,
>;

/// Manages debounced flushes for a single streaming response.
pub struct StreamFlushManager {
    config: FlushConfig,
    message_id: Uuid,
    save_fn: SaveFn,
}

impl StreamFlushManager {
    /// Create a new flush manager.
    ///
    /// # Arguments
    ///
    /// * `message_id` - The ID of the message being streamed
    /// * `config` - Configuration for debouncing
    /// * `save_fn` - Async function to save content to database
    pub fn new<F, Fut>(message_id: Uuid, config: FlushConfig, save_fn: F) -> Self
    where
        F: Fn(String, u32) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<(), String>> + Send + 'static,
    {
        let save_fn: SaveFn = Arc::new(move |content, tokens| Box::pin(save_fn(content, tokens)));

        Self {
            config,
            message_id,
            save_fn,
        }
    }

    /// Start the flush manager background task.
    ///
    /// Returns a handle for sending updates. The background task
    /// will run until it receives a Complete or Abort message.
    pub fn start(self) -> FlushHandle {
        let (tx, mut rx) = mpsc::channel::<FlushMessage>(100);

        let config = self.config;
        let message_id = self.message_id;
        let save_fn = self.save_fn;

        tokio::spawn(async move {
            let mut last_flush = Instant::now();
            let mut pending_content: Option<(String, u32)> = None;

            loop {
                let timeout = config.write_delay;

                tokio::select! {
                    biased;

                    msg = rx.recv() => {
                        match msg {
                            Some(FlushMessage::ContentUpdated { content, tokens }) => {
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
            async move {
                count.fetch_add(1, Ordering::SeqCst);
                Ok(())
            }
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
            handle
                .content_updated(format!("content_{}", i), i as u32)
                .await;
            sleep(Duration::from_millis(20)).await;
        }

        // Wait for debounce
        sleep(Duration::from_millis(200)).await;

        // Complete
        handle.complete().await;
        sleep(Duration::from_millis(50)).await;

        // Should have far fewer than 10 writes due to debouncing
        let writes = write_count.load(Ordering::SeqCst);
        assert!(
            writes < 5,
            "Expected < 5 writes due to debouncing, got {}",
            writes
        );
    }

    #[tokio::test]
    async fn test_forced_flush_on_size() {
        let write_count = Arc::new(AtomicU32::new(0));
        let write_count_clone = write_count.clone();

        let save_fn = move |_content: String, _tokens: u32| {
            let count = write_count_clone.clone();
            async move {
                count.fetch_add(1, Ordering::SeqCst);
                Ok(())
            }
        };

        let manager = StreamFlushManager::new(
            Uuid::new_v4(),
            FlushConfig {
                write_delay: Duration::from_secs(10), // Long delay
                max_buffer_time: Duration::from_secs(10),
                max_buffer_bytes: 100, // Low threshold
            },
            save_fn,
        );

        let handle = manager.start();

        // Send large content that exceeds threshold
        let large_content = "a".repeat(150);
        handle.content_updated(large_content, 30).await;

        // Give time for forced flush
        sleep(Duration::from_millis(50)).await;

        handle.complete().await;
        sleep(Duration::from_millis(50)).await;

        // Should have at least 1 forced flush
        let writes = write_count.load(Ordering::SeqCst);
        assert!(
            writes >= 1,
            "Expected at least 1 forced flush, got {}",
            writes
        );
    }

    #[tokio::test]
    async fn test_abort_saves_partial() {
        let saved_content = Arc::new(tokio::sync::Mutex::new(String::new()));
        let saved_content_clone = saved_content.clone();

        let save_fn = move |content: String, _tokens: u32| {
            let saved = saved_content_clone.clone();
            async move {
                *saved.lock().await = content;
                Ok(())
            }
        };

        let manager = StreamFlushManager::new(
            Uuid::new_v4(),
            FlushConfig {
                write_delay: Duration::from_secs(10), // Long delay
                max_buffer_time: Duration::from_secs(10),
                max_buffer_bytes: 100000,
            },
            save_fn,
        );

        let handle = manager.start();

        // Send content
        handle
            .content_updated("partial content".to_string(), 5)
            .await;

        // Abort immediately
        handle.abort().await;
        sleep(Duration::from_millis(50)).await;

        // Should have saved partial content
        let content = saved_content.lock().await.clone();
        assert_eq!(content, "partial content");
    }

    #[tokio::test]
    async fn test_complete_flushes_pending() {
        let saved_content = Arc::new(tokio::sync::Mutex::new(String::new()));
        let saved_content_clone = saved_content.clone();

        let save_fn = move |content: String, _tokens: u32| {
            let saved = saved_content_clone.clone();
            async move {
                *saved.lock().await = content;
                Ok(())
            }
        };

        let manager = StreamFlushManager::new(
            Uuid::new_v4(),
            FlushConfig {
                write_delay: Duration::from_secs(10), // Long delay
                max_buffer_time: Duration::from_secs(10),
                max_buffer_bytes: 100000,
            },
            save_fn,
        );

        let handle = manager.start();

        // Send content
        handle.content_updated("final content".to_string(), 5).await;

        // Complete
        handle.complete().await;
        sleep(Duration::from_millis(50)).await;

        // Should have saved content
        let content = saved_content.lock().await.clone();
        assert_eq!(content, "final content");
    }
}
