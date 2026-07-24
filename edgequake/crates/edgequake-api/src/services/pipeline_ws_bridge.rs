//! SPEC-048: Bridge PipelineEvent → ProgressEvent for WebSocket clients (DEF-02).

use edgequake_tasks::PipelineEvent;

use crate::handlers::websocket_types::{ProgressBroadcaster, ProgressEvent};

/// Map internal pipeline events to WS ProgressEvent variants worth forwarding.
pub fn pipeline_event_to_progress(event: &PipelineEvent) -> Option<ProgressEvent> {
    match event {
        PipelineEvent::ChunkProgress {
            document_id,
            task_id,
            chunk_index,
            total_chunks,
            chunk_preview,
            time_ms,
            eta_seconds,
            tokens_in,
            tokens_out,
            cost_usd,
        } => Some(ProgressEvent::ChunkProgress {
            document_id: document_id.clone(),
            task_id: task_id.clone(),
            chunk_index: *chunk_index,
            total_chunks: *total_chunks,
            chunk_preview: chunk_preview.clone(),
            time_ms: *time_ms,
            eta_seconds: *eta_seconds,
            tokens_in: *tokens_in,
            tokens_out: *tokens_out,
            cost_usd: *cost_usd,
        }),
        PipelineEvent::GraphStorageProgress {
            track_id,
            document_id,
            sub_phase,
            sub_phase_label,
            entities_processed,
            entities_total,
            entities_created,
            entities_updated,
            relationships_processed,
            relationships_total,
            relationships_created,
            relationships_updated,
            elapsed_ms,
            eta_ms,
        } => Some(ProgressEvent::GraphStorageProgress {
            track_id: track_id.clone(),
            document_id: document_id.clone(),
            sub_phase: sub_phase.clone(),
            sub_phase_label: sub_phase_label.clone(),
            entities_processed: *entities_processed,
            entities_total: *entities_total,
            entities_created: *entities_created,
            entities_updated: *entities_updated,
            relationships_processed: *relationships_processed,
            relationships_total: *relationships_total,
            relationships_created: *relationships_created,
            relationships_updated: *relationships_updated,
            elapsed_ms: *elapsed_ms,
            eta_ms: *eta_ms,
        }),
        PipelineEvent::ChunkFailure {
            document_id,
            task_id,
            chunk_index,
            total_chunks,
            error_message,
            was_timeout,
            retry_attempts,
        } => Some(ProgressEvent::ChunkFailure {
            document_id: document_id.clone(),
            task_id: task_id.clone(),
            chunk_index: *chunk_index,
            total_chunks: *total_chunks,
            error_message: error_message.clone(),
            was_timeout: *was_timeout,
            retry_attempts: *retry_attempts,
        }),
        PipelineEvent::PdfPageProgress {
            pdf_id,
            task_id,
            page_num,
            total_pages,
            phase,
            markdown_len,
            success,
            error,
        } => Some(ProgressEvent::PdfPageProgress {
            pdf_id: pdf_id.clone(),
            task_id: task_id.clone(),
            page_num: *page_num,
            total_pages: *total_pages,
            phase: phase.clone(),
            markdown_len: *markdown_len,
            success: *success,
            error: error.clone(),
        }),
        PipelineEvent::StageTransition {
            document_id,
            task_id,
            stage,
            stage_message,
            stage_progress,
        } => Some(ProgressEvent::StageTransition {
            document_id: document_id.clone(),
            task_id: task_id.clone(),
            stage: stage.clone(),
            stage_message: stage_message.clone(),
            stage_progress: *stage_progress,
        }),
        _ => None,
    }
}

/// Spawn a background task that forwards PipelineState events to ProgressBroadcaster.
pub fn spawn_pipeline_ws_bridge(
    pipeline_state: edgequake_tasks::PipelineState,
    broadcaster: ProgressBroadcaster,
) {
    let mut rx = pipeline_state.subscribe();
    tokio::spawn(async move {
        loop {
            match rx.recv().await {
                Ok(event) => {
                    if let Some(progress) = pipeline_event_to_progress(&event) {
                        broadcaster.broadcast(progress);
                    }
                }
                Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => continue,
                Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_chunk_progress() {
        let ev = PipelineEvent::ChunkProgress {
            document_id: "d".into(),
            task_id: "t".into(),
            chunk_index: 1,
            total_chunks: 10,
            chunk_preview: "hi".into(),
            time_ms: 1,
            eta_seconds: 2,
            tokens_in: 3,
            tokens_out: 4,
            cost_usd: 0.01,
        };
        let mapped = pipeline_event_to_progress(&ev).expect("mapped");
        match mapped {
            ProgressEvent::ChunkProgress {
                chunk_index,
                total_chunks,
                ..
            } => {
                assert_eq!(chunk_index, 1);
                assert_eq!(total_chunks, 10);
            }
            _ => panic!("expected ChunkProgress"),
        }
    }

    #[test]
    fn maps_stage_transition() {
        let ev = PipelineEvent::StageTransition {
            document_id: "d".into(),
            task_id: "insert-1".into(),
            stage: "chunking".into(),
            stage_message: "Chunking document".into(),
            stage_progress: Some(0.0),
        };
        let mapped = pipeline_event_to_progress(&ev).expect("mapped");
        match mapped {
            ProgressEvent::StageTransition {
                task_id,
                stage,
                stage_message,
                stage_progress,
                ..
            } => {
                assert_eq!(task_id, "insert-1");
                assert_eq!(stage, "chunking");
                assert_eq!(stage_message, "Chunking document");
                assert_eq!(stage_progress, Some(0.0));
            }
            _ => panic!("expected StageTransition"),
        }
    }
}
