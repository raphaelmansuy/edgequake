//! Unified cancel entry for HTTP / WebSocket (SPEC-057 DRY + SPEC-059 retract).
//!
//! Owns: task-row cancel + doc KV sync + Convert/Insert pdf_id chain + index retract.

use std::sync::Arc;

use edgequake_storage::traits::{GraphStorage, KVStorage, VectorStorage};
use edgequake_tasks::{CancellationRegistry, SharedTaskStorage};

use super::cancel_retract::{retract_indexes_for_document, retract_indexes_for_task};
use super::task_cancel::{
    apply_cancel_pdf_pipeline_tasks, apply_task_row_cancel, TaskCancelApplyResult,
};
use super::task_document_sync::sync_doc_cancelled_for_task;

/// Cancel a track_id, sync linked document KV, retract indexes, and cancel sibling
/// Convert/Insert tasks for the same `pdf_id` when present.
pub async fn cancel_track_with_doc_and_pdf_chain(
    storage: &SharedTaskStorage,
    registry: &CancellationRegistry,
    kv: Arc<dyn KVStorage>,
    graph: &Arc<dyn GraphStorage>,
    vector: &Arc<dyn VectorStorage>,
    track_id: &str,
) -> Result<TaskCancelApplyResult, String> {
    let applied = apply_task_row_cancel(storage, registry, track_id).await?;

    if applied.conflict_indexed {
        return Ok(applied);
    }

    if applied.cancelled {
        if let Some(ref task) = applied.task {
            if let Err(e) =
                sync_doc_cancelled_for_task(Arc::clone(&kv), None, task, "Task cancelled by user")
                    .await
            {
                tracing::warn!(
                    track_id = %track_id,
                    error = %e,
                    "cancel facade: doc KV sync failed"
                );
            }
            // SPEC-059: unindex immediately — do not wait for worker checkpoint.
            retract_indexes_for_task(graph, vector, task).await;

            if let Some(pdf_id) = task.pdf_id() {
                match apply_cancel_pdf_pipeline_tasks(storage, registry, pdf_id, task.workspace_id)
                    .await
                {
                    Ok(linked) => {
                        for linked_applied in linked {
                            if let Some(ref linked_task) = linked_applied.task {
                                let _ = sync_doc_cancelled_for_task(
                                    Arc::clone(&kv),
                                    None,
                                    linked_task,
                                    "Task cancelled by user",
                                )
                                .await;
                                retract_indexes_for_task(graph, vector, linked_task).await;
                            }
                        }
                    }
                    Err(e) => {
                        tracing::warn!(
                            track_id = %track_id,
                            pdf_id = %pdf_id,
                            error = %e,
                            "cancel facade: linked PDF pipeline cancel failed"
                        );
                    }
                }
            }
        }
    }

    Ok(applied)
}

/// Retract by document id when PDF cancel syncs via `pdf.document_id` only.
pub async fn retract_indexes_for_document_id(
    graph: &Arc<dyn GraphStorage>,
    vector: &Arc<dyn VectorStorage>,
    document_id: &str,
) {
    retract_indexes_for_document(graph, vector, document_id).await;
}

#[cfg(test)]
mod tests {
    use super::*;
    use edgequake_storage::{
        MemoryGraphStorage, MemoryKVStorage, MemoryVectorStorage, VectorStorage,
    };
    use edgequake_tasks::{memory::MemoryTaskStorage, Task, TaskStatus, TaskType};
    use uuid::Uuid;

    #[tokio::test]
    async fn cancel_convert_also_cancels_linked_insert() {
        let storage: SharedTaskStorage = Arc::new(MemoryTaskStorage::new());
        let registry = CancellationRegistry::new();
        let kv: Arc<dyn KVStorage> = Arc::new(MemoryKVStorage::new("cancel-facade"));
        let graph: Arc<dyn GraphStorage> = Arc::new(MemoryGraphStorage::new("cancel-facade"));
        let vector: Arc<dyn VectorStorage> = Arc::new(MemoryVectorStorage::new("cancel-facade", 4));
        graph.initialize().await.unwrap();
        vector.initialize().await.unwrap();
        let tenant = Uuid::new_v4();
        let workspace = Uuid::new_v4();
        let pdf_id = Uuid::new_v4();

        let convert = Task::new(
            tenant,
            workspace,
            TaskType::PdfProcessing,
            serde_json::json!({ "pdf_id": pdf_id.to_string() }),
        );
        let convert_track = convert.track_id.clone();
        storage.create_task(&convert).await.unwrap();

        let insert = Task::new(
            tenant,
            workspace,
            TaskType::Insert,
            serde_json::json!({
                "text": "x",
                "file_source": "a.pdf",
                "workspace_id": workspace.to_string(),
                "metadata": { "pdf_id": pdf_id.to_string() },
            }),
        );
        let insert_track = insert.track_id.clone();
        storage.create_task(&insert).await.unwrap();

        let applied = cancel_track_with_doc_and_pdf_chain(
            &storage,
            &registry,
            kv,
            &graph,
            &vector,
            &convert_track,
        )
        .await
        .unwrap();
        assert!(applied.cancelled);

        assert_eq!(
            storage
                .get_task(&convert_track)
                .await
                .unwrap()
                .unwrap()
                .status,
            TaskStatus::Cancelled
        );
        assert_eq!(
            storage
                .get_task(&insert_track)
                .await
                .unwrap()
                .unwrap()
                .status,
            TaskStatus::Cancelled
        );
    }

    #[tokio::test]
    async fn cancel_retracts_document_vectors() {
        let storage: SharedTaskStorage = Arc::new(MemoryTaskStorage::new());
        let registry = CancellationRegistry::new();
        let kv: Arc<dyn KVStorage> = Arc::new(MemoryKVStorage::new("cancel-retract"));
        let graph: Arc<dyn GraphStorage> = Arc::new(MemoryGraphStorage::new("cancel-retract"));
        let vector: Arc<dyn VectorStorage> =
            Arc::new(MemoryVectorStorage::new("cancel-retract", 4));
        graph.initialize().await.unwrap();
        vector.initialize().await.unwrap();
        kv.initialize().await.unwrap();

        let doc_id = "doc-cancel-retract";
        vector
            .upsert(&[(
                format!("{doc_id}-chunk-0"),
                vec![0.1, 0.2, 0.3, 0.4],
                serde_json::json!({"type": "chunk", "document_id": doc_id}),
            )])
            .await
            .unwrap();

        let tenant = Uuid::new_v4();
        let workspace = Uuid::new_v4();
        let task = Task::new(
            tenant,
            workspace,
            TaskType::Insert,
            serde_json::json!({
                "text": "x",
                "existing_document_id": doc_id,
            }),
        );
        let track = task.track_id.clone();
        storage.create_task(&task).await.unwrap();

        let applied =
            cancel_track_with_doc_and_pdf_chain(&storage, &registry, kv, &graph, &vector, &track)
                .await
                .unwrap();
        assert!(applied.cancelled);
        assert!(
            vector
                .get_by_id(&format!("{doc_id}-chunk-0"))
                .await
                .unwrap()
                .is_none(),
            "SPEC-059: cancel facade must retract chunk vectors"
        );
    }
}
