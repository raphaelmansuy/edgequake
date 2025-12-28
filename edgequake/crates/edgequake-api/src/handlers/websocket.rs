//! WebSocket handlers for real-time progress streaming.
//!
//! This module provides WebSocket endpoints for streaming pipeline progress events
//! to connected clients in real-time.

use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        State,
    },
    response::IntoResponse,
};
use futures_util::{SinkExt, StreamExt};
use serde::Serialize;
use std::time::Duration;
use tokio::sync::broadcast;
use tracing::{debug, error, info, warn};

use crate::state::AppState;

/// Configuration for WebSocket connections.
const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(30);

/// Progress event types sent over WebSocket.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", content = "data")]
pub enum ProgressEvent {
    /// Pipeline job started.
    JobStarted {
        job_name: String,
        total_documents: u32,
        total_batches: u32,
    },
    /// Document processing progress.
    DocumentProgress {
        document_id: String,
        entities_extracted: usize,
        processed: u32,
        total: u32,
    },
    /// Document processing failed.
    DocumentFailed {
        document_id: String,
        error: String,
        processed: u32,
        total: u32,
    },
    /// Batch completed.
    BatchCompleted { batch: u32, total_batches: u32 },
    /// Pipeline job finished.
    JobFinished {
        total_processed: u32,
        duration_ms: u64,
    },
    /// Pipeline message (info, warn, error).
    Message {
        level: String,
        message: String,
        timestamp: String,
    },
    /// Status snapshot (for initial sync or periodic updates).
    StatusSnapshot {
        is_busy: bool,
        job_name: Option<String>,
        processed_documents: u32,
        total_documents: u32,
        current_batch: u32,
        total_batches: u32,
    },
    /// Heartbeat/ping for connection keepalive.
    Heartbeat { timestamp: String },
    /// Connection established confirmation.
    Connected { message: String },
    /// Cancellation requested.
    CancellationRequested,
}

/// WebSocket connection for pipeline progress streaming.
///
/// Upgrades an HTTP connection to a WebSocket for real-time progress events.
///
/// # WebSocket Messages
///
/// The server sends JSON-encoded `ProgressEvent` messages:
/// - `JobStarted`: When a pipeline job begins
/// - `DocumentProgress`: Progress update for each document
/// - `DocumentFailed`: When document processing fails
/// - `BatchCompleted`: When a batch finishes
/// - `JobFinished`: When the entire job completes
/// - `Message`: Pipeline log messages
/// - `StatusSnapshot`: Full status at connection start
/// - `Heartbeat`: Periodic keepalive
///
/// # Example Client Usage
///
/// ```javascript
/// const ws = new WebSocket('ws://localhost:8020/ws/pipeline/progress');
/// ws.onmessage = (event) => {
///     const data = JSON.parse(event.data);
///     switch (data.type) {
///         case 'DocumentProgress':
///             console.log(`Processed ${data.data.processed}/${data.data.total}`);
///             break;
///         case 'JobFinished':
///             console.log('Pipeline complete!');
///             break;
///     }
/// };
/// ```
#[utoipa::path(
    get,
    path = "/ws/pipeline/progress",
    tag = "WebSocket",
    responses(
        (status = 101, description = "WebSocket upgrade successful"),
        (status = 400, description = "WebSocket upgrade failed")
    )
)]
pub async fn ws_pipeline_progress(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
) -> impl IntoResponse {
    info!("WebSocket connection requested for pipeline progress");
    ws.on_upgrade(move |socket| handle_pipeline_socket(socket, state))
}

/// Handle the WebSocket connection for pipeline progress.
async fn handle_pipeline_socket(socket: WebSocket, state: AppState) {
    let (mut sender, mut receiver) = socket.split();

    info!("WebSocket connection established for pipeline progress");

    // Send initial connected message
    let connected_event = ProgressEvent::Connected {
        message: "Connected to pipeline progress stream".to_string(),
    };
    if let Err(e) = send_event(&mut sender, &connected_event).await {
        error!("Failed to send connected event: {}", e);
        return;
    }

    // Send initial status snapshot
    let status = state.pipeline_state.get_status().await;
    let snapshot_event = ProgressEvent::StatusSnapshot {
        is_busy: status.is_busy,
        job_name: status.job_name.clone(),
        processed_documents: status.processed_documents,
        total_documents: status.total_documents,
        current_batch: status.current_batch,
        total_batches: status.total_batches,
    };
    if let Err(e) = send_event(&mut sender, &snapshot_event).await {
        error!("Failed to send status snapshot: {}", e);
        return;
    }

    // Subscribe to progress broadcast channel
    let mut progress_rx = state.progress_broadcaster.subscribe();

    // Create heartbeat interval
    let mut heartbeat_interval = tokio::time::interval(HEARTBEAT_INTERVAL);

    // Main event loop
    loop {
        tokio::select! {
            // Handle incoming messages from client
            Some(msg) = receiver.next() => {
                match msg {
                    Ok(Message::Text(text)) => {
                        debug!("Received text message: {}", text);
                        // Handle client commands if needed
                        if text.trim() == "status" {
                            let status = state.pipeline_state.get_status().await;
                            let snapshot = ProgressEvent::StatusSnapshot {
                                is_busy: status.is_busy,
                                job_name: status.job_name.clone(),
                                processed_documents: status.processed_documents,
                                total_documents: status.total_documents,
                                current_batch: status.current_batch,
                                total_batches: status.total_batches,
                            };
                            if let Err(e) = send_event(&mut sender, &snapshot).await {
                                error!("Failed to send status snapshot: {}", e);
                                break;
                            }
                        }
                    }
                    Ok(Message::Ping(data)) => {
                        debug!("Received ping, sending pong");
                        if sender.send(Message::Pong(data)).await.is_err() {
                            break;
                        }
                    }
                    Ok(Message::Close(_)) => {
                        info!("WebSocket client disconnected");
                        break;
                    }
                    Err(e) => {
                        warn!("WebSocket error: {}", e);
                        break;
                    }
                    _ => {}
                }
            }

            // Handle broadcast progress events
            result = progress_rx.recv() => {
                match result {
                    Ok(event) => {
                        if let Err(e) = send_event(&mut sender, &event).await {
                            error!("Failed to send progress event: {}", e);
                            break;
                        }
                    }
                    Err(broadcast::error::RecvError::Lagged(n)) => {
                        warn!("WebSocket client lagged behind {} events", n);
                        // Continue processing, but client missed some events
                    }
                    Err(broadcast::error::RecvError::Closed) => {
                        warn!("Progress broadcast channel closed");
                        break;
                    }
                }
            }

            // Send periodic heartbeats
            _ = heartbeat_interval.tick() => {
                let heartbeat = ProgressEvent::Heartbeat {
                    timestamp: chrono::Utc::now().to_rfc3339(),
                };
                if let Err(e) = send_event(&mut sender, &heartbeat).await {
                    error!("Failed to send heartbeat: {}", e);
                    break;
                }
            }
        }
    }

    info!("WebSocket connection closed for pipeline progress");
}

/// Send a progress event as JSON over the WebSocket.
async fn send_event(
    sender: &mut futures_util::stream::SplitSink<WebSocket, Message>,
    event: &ProgressEvent,
) -> Result<(), axum::Error> {
    let json = serde_json::to_string(event).map_err(|e| {
        error!("Failed to serialize event: {}", e);
        axum::Error::new(e)
    })?;
    sender
        .send(Message::Text(json.into()))
        .await
        .map_err(axum::Error::new)
}

/// Broadcast channel for progress events.
///
/// This struct manages the broadcast channel that distributes progress events
/// to all connected WebSocket clients.
#[derive(Clone)]
pub struct ProgressBroadcaster {
    sender: broadcast::Sender<ProgressEvent>,
}

impl Default for ProgressBroadcaster {
    fn default() -> Self {
        Self::new(1024)
    }
}

impl ProgressBroadcaster {
    /// Create a new progress broadcaster with specified channel capacity.
    pub fn new(capacity: usize) -> Self {
        let (sender, _) = broadcast::channel(capacity);
        Self { sender }
    }

    /// Subscribe to progress events.
    pub fn subscribe(&self) -> broadcast::Receiver<ProgressEvent> {
        self.sender.subscribe()
    }

    /// Broadcast a progress event to all subscribers.
    pub fn broadcast(&self, event: ProgressEvent) {
        // Ignore send errors (no subscribers)
        let _ = self.sender.send(event);
    }

    /// Broadcast job started event.
    pub fn job_started(&self, job_name: &str, total_documents: u32, total_batches: u32) {
        self.broadcast(ProgressEvent::JobStarted {
            job_name: job_name.to_string(),
            total_documents,
            total_batches,
        });
    }

    /// Broadcast document progress event.
    pub fn document_progress(
        &self,
        document_id: &str,
        entities_extracted: usize,
        processed: u32,
        total: u32,
    ) {
        self.broadcast(ProgressEvent::DocumentProgress {
            document_id: document_id.to_string(),
            entities_extracted,
            processed,
            total,
        });
    }

    /// Broadcast document failed event.
    pub fn document_failed(&self, document_id: &str, error: &str, processed: u32, total: u32) {
        self.broadcast(ProgressEvent::DocumentFailed {
            document_id: document_id.to_string(),
            error: error.to_string(),
            processed,
            total,
        });
    }

    /// Broadcast batch completed event.
    pub fn batch_completed(&self, batch: u32, total_batches: u32) {
        self.broadcast(ProgressEvent::BatchCompleted {
            batch,
            total_batches,
        });
    }

    /// Broadcast job finished event.
    pub fn job_finished(&self, total_processed: u32, duration_ms: u64) {
        self.broadcast(ProgressEvent::JobFinished {
            total_processed,
            duration_ms,
        });
    }

    /// Broadcast a message event.
    pub fn message(&self, level: &str, message: &str) {
        self.broadcast(ProgressEvent::Message {
            level: level.to_string(),
            message: message.to_string(),
            timestamp: chrono::Utc::now().to_rfc3339(),
        });
    }

    /// Broadcast cancellation requested event.
    pub fn cancellation_requested(&self) {
        self.broadcast(ProgressEvent::CancellationRequested);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_progress_broadcaster_creation() {
        let broadcaster = ProgressBroadcaster::new(100);
        let mut rx = broadcaster.subscribe();

        broadcaster.job_started("test-job", 10, 2);

        let event = rx.recv().await.unwrap();
        match event {
            ProgressEvent::JobStarted {
                job_name,
                total_documents,
                total_batches,
            } => {
                assert_eq!(job_name, "test-job");
                assert_eq!(total_documents, 10);
                assert_eq!(total_batches, 2);
            }
            _ => panic!("Unexpected event type"),
        }
    }

    #[tokio::test]
    async fn test_progress_broadcaster_multiple_subscribers() {
        let broadcaster = ProgressBroadcaster::new(100);
        let mut rx1 = broadcaster.subscribe();
        let mut rx2 = broadcaster.subscribe();

        broadcaster.document_progress("doc-1", 5, 1, 10);

        let event1 = rx1.recv().await.unwrap();
        let event2 = rx2.recv().await.unwrap();

        match (&event1, &event2) {
            (
                ProgressEvent::DocumentProgress {
                    document_id: id1, ..
                },
                ProgressEvent::DocumentProgress {
                    document_id: id2, ..
                },
            ) => {
                assert_eq!(id1, "doc-1");
                assert_eq!(id2, "doc-1");
            }
            _ => panic!("Unexpected event types"),
        }
    }

    #[tokio::test]
    async fn test_progress_broadcaster_all_event_types() {
        let broadcaster = ProgressBroadcaster::new(100);
        let mut rx = broadcaster.subscribe();

        broadcaster.job_started("test", 10, 2);
        broadcaster.document_progress("doc-1", 5, 1, 10);
        broadcaster.document_failed("doc-2", "error", 2, 10);
        broadcaster.batch_completed(1, 2);
        broadcaster.message("info", "test message");
        broadcaster.cancellation_requested();
        broadcaster.job_finished(10, 5000);

        // Verify all events are received
        for _ in 0..7 {
            let _ = rx.recv().await.unwrap();
        }
    }
}
