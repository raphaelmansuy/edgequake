//! Memory PDF + conversation adapter parity smoke tests (SPEC-017 P1).

use edgequake_storage::{
    calculate_pdf_checksum, CreatePdfRequest, MemoryConversationStorage, MemoryPdfStorage,
    PdfDocumentStorage, PdfProcessingStatus,
};
use uuid::Uuid;

#[tokio::test]
async fn memory_pdf_full_lifecycle() {
    let storage = MemoryPdfStorage::new();
    let ws = Uuid::new_v4();
    let pdf_data = b"%PDF-1.4\n";
    let checksum = calculate_pdf_checksum(pdf_data);

    let pdf_id = storage
        .create_pdf(CreatePdfRequest {
            workspace_id: ws,
            filename: "proof.pdf".into(),
            content_type: "application/pdf".into(),
            file_size_bytes: pdf_data.len() as i64,
            sha256_checksum: checksum,
            page_count: Some(1),
            pdf_data: pdf_data.to_vec(),
            vision_model: None,
        })
        .await
        .unwrap();

    storage
        .update_pdf_status(&pdf_id, PdfProcessingStatus::Processing)
        .await
        .unwrap();
    assert_eq!(storage.count_pdfs(&ws, None).await.unwrap(), 1);

    let doc_id = Uuid::new_v4();
    storage
        .ensure_document_record(&doc_id, &ws, None, "proof", "# md", "ready")
        .await
        .unwrap();
    storage
        .link_pdf_to_document(&pdf_id, &doc_id)
        .await
        .unwrap();

    let listed = storage
        .list_pdfs(edgequake_storage::ListPdfFilter {
            workspace_id: Some(ws),
            ..Default::default()
        })
        .await
        .unwrap();
    assert_eq!(listed.total_count, 1);
    assert_eq!(listed.items[0].document_id, Some(doc_id));
}

#[tokio::test]
async fn memory_conversation_crud_smoke() {
    let storage = MemoryConversationStorage::new();
    let tenant = Uuid::new_v4();
    let user = Uuid::new_v4();

    let folder = storage
        .create_folder(tenant, user, None, "Inbox", None)
        .await
        .unwrap();

    let conv = storage
        .create_conversation(
            tenant,
            user,
            None,
            "Parity chat".into(),
            "hybrid".into(),
            Some(folder.folder_id),
        )
        .await
        .unwrap();

    storage
        .create_message(
            conv.conversation_id,
            None,
            "user",
            "workspace stats?",
            Some("hybrid"),
            None,
            None,
            None,
            None,
            false,
        )
        .await
        .unwrap();

    let share = storage
        .share_conversation(conv.conversation_id)
        .await
        .unwrap();
    assert!(share.starts_with("share_"));
    assert_eq!(
        storage
            .list_messages(conv.conversation_id, 10, 0)
            .await
            .unwrap()
            .1,
        1
    );
}
