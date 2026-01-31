//! WebSocket handlers for real-time progress streaming.
//!
//! This module provides WebSocket endpoints for streaming pipeline progress events
//! to connected clients in real-time.
//!
//! ## Implements
//!
//! - **FEAT0520**: WebSocket connection upgrade for progress streaming
//! - **FEAT0521**: Real-time pipeline progress events
//! - **FEAT0522**: Initial status snapshot on connection
//! - **FEAT0523**: Heartbeat keepalive mechanism
//!
//! ## Use Cases
//!
//! - **UC2120**: Client connects to WebSocket for live pipeline updates
//! - **UC2121**: Client receives document progress events during processing
//! - **UC2122**: Client gets notified when pipeline job completes
//! - **UC2123**: Connection stays alive via periodic heartbeats
//!
//! ## Enforces
//!
//! - **BR0520**: WebSocket must send initial status snapshot on connect
//! - **BR0521**: Heartbeat interval must be 30 seconds
//! - **BR0522**: All progress events must be JSON-encoded

use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        Path, State,
    },
    response::IntoResponse,
};
use futures_util::{SinkExt, StreamExt};
use std::time::Duration;
use tokio::sync::broadcast;
use tracing::{debug, error, info, warn};

use crate::state::AppState;

// Re-export DTOs from websocket_types for backwards compatibility
pub use crate::handlers::websocket_types::{ProgressBroadcaster, ProgressEvent};

/// Configuration for WebSocket connections.
const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(30);

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

// ============================================================================
// OODA-15: Filtered WebSocket for specific track_id
// ============================================================================

/// WebSocket connection for filtered PDF upload progress.
///
/// @implements SPEC-001-upload-pdf: Filtered progress streaming
/// @implements OODA-15: Track-specific WebSocket endpoint
///
/// Upgrades an HTTP connection to a WebSocket that streams only events
/// for the specified `track_id`. This allows clients to subscribe to
/// progress updates for their specific upload without receiving
/// unrelated events.
///
/// # Path Parameters
///
/// * `track_id` - Upload tracking ID (returned from upload response)
///
/// # WebSocket Messages (Server → Client)
///
/// - `PdfPageProgress`: Page-by-page extraction progress
/// - `StatusSnapshot`: Initial progress snapshot on connection
/// - `Heartbeat`: Periodic keepalive (every 30 seconds)
/// - `Connected`: Connection established confirmation
///
/// # Example Client Usage
///
/// ```javascript
/// const trackId = uploadResponse.track_id;
/// const ws = new WebSocket(`ws://localhost:8020/ws/progress/${trackId}`);
/// ws.onmessage = (event) => {
///     const data = JSON.parse(event.data);
///     if (data.type === 'PdfPageProgress') {
///         console.log(`Page ${data.data.page_num}/${data.data.total_pages}`);
///     }
/// };
/// ```
#[utoipa::path(
    get,
    path = "/ws/progress/{track_id}",
    params(
        ("track_id" = String, Path, description = "Upload tracking ID")
    ),
    tag = "WebSocket",
    responses(
        (status = 101, description = "WebSocket upgrade successful"),
        (status = 400, description = "WebSocket upgrade failed")
    )
)]
pub async fn ws_progress_by_track_id(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
    Path(track_id): Path<String>,
) -> impl IntoResponse {
    info!("WebSocket connection requested for track_id={}", track_id);
    ws.on_upgrade(move |socket| handle_filtered_progress_socket(socket, state, track_id))
}

/// Handle the filtered WebSocket connection for PDF progress.
async fn handle_filtered_progress_socket(socket: WebSocket, state: AppState, track_id: String) {
    let (mut sender, mut receiver) = socket.split();

    info!("WebSocket connection established for track_id={}", track_id);

    // Send initial connected message
    let connected_event = ProgressEvent::Connected {
        message: format!("Connected to progress stream for {}", track_id),
    };
    if let Err(e) = send_event(&mut sender, &connected_event).await {
        error!("Failed to send connected event: {}", e);
        return;
    }

    // Send initial progress snapshot if available
    if let Some(progress) = state.pipeline_state.get_pdf_progress(&track_id).await {
        // Serialize progress as a special event
        if let Ok(json) = serde_json::to_value(&progress) {
            let snapshot_msg = serde_json::json!({
                "type": "ProgressSnapshot",
                "data": json
            });
            if let Ok(json_str) = serde_json::to_string(&snapshot_msg) {
                if sender.send(Message::Text(json_str.into())).await.is_err() {
                    error!("Failed to send progress snapshot");
                    return;
                }
            }
        }
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
                        debug!("Received text message from track_id={}: {}", track_id, text);
                        // Handle client commands if needed
                        if text.trim() == "status" {
                            // Send current progress snapshot
                            if let Some(progress) = state.pipeline_state.get_pdf_progress(&track_id).await {
                                if let Ok(json) = serde_json::to_value(&progress) {
                                    let snapshot_msg = serde_json::json!({
                                        "type": "ProgressSnapshot",
                                        "data": json
                                    });
                                    if let Ok(json_str) = serde_json::to_string(&snapshot_msg) {
                                        if sender.send(Message::Text(json_str.into())).await.is_err() {
                                            break;
                                        }
                                    }
                                }
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
                        info!("WebSocket client disconnected for track_id={}", track_id);
                        break;
                    }
                    Err(e) => {
                        warn!("WebSocket error for track_id={}: {}", track_id, e);
                        break;
                    }
                    _ => {}
                }
            }

            // Handle broadcast progress events (filtered)
            result = progress_rx.recv() => {
                match result {
                    Ok(event) => {
                        // Only forward events matching this track_id
                        if matches_track_id(&event, &track_id) {
                            if let Err(e) = send_event(&mut sender, &event).await {
                                error!("Failed to send progress event: {}", e);
                                break;
                            }
                        }
                    }
                    Err(broadcast::error::RecvError::Lagged(n)) => {
                        warn!("WebSocket client lagged behind {} events for track_id={}", n, track_id);
                    }
                    Err(broadcast::error::RecvError::Closed) => {
                        warn!("Progress broadcast channel closed for track_id={}", track_id);
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

    info!("WebSocket connection closed for track_id={}", track_id);
}

/// Check if a ProgressEvent matches the specified track_id.
///
/// Only PdfPageProgress events contain a task_id field that can be matched.
/// Other event types are not relevant to specific PDF uploads.
fn matches_track_id(event: &ProgressEvent, track_id: &str) -> bool {
    match event {
        ProgressEvent::PdfPageProgress { task_id, .. } => task_id == track_id,
        ProgressEvent::ChunkFailure { task_id, .. } => task_id == track_id,
        _ => false, // Other events don't have task_id, skip them
    }
}
