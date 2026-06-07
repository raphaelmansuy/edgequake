//! Shared conversation CRUD contract (SPEC-017 P1).

use edgequake_storage::ConversationStorage;
use uuid::Uuid;

/// Exercise core conversation lifecycle with explicit tenant/user (postgres needs seeded FK rows).
pub async fn assert_conversation_crud_contract_with_ids<S: ConversationStorage + ?Sized>(
    storage: &S,
    tenant: Uuid,
    user: Uuid,
) {
    let folder = storage
        .create_folder(tenant, user, None, "Inbox", None)
        .await
        .unwrap();

    let conv = storage
        .create_conversation(
            tenant,
            user,
            None,
            "Contract chat".into(),
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
            "hello contract",
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

    let (listed, total) = storage
        .list_conversations(
            tenant,
            user,
            None,
            None,
            None,
            None,
            None,
            "created_at",
            false,
            10,
            0,
        )
        .await
        .unwrap();
    assert_eq!(total, 1);
    assert_eq!(listed[0].conversation_id, conv.conversation_id);

    let (_, msg_total) = storage
        .list_messages(conv.conversation_id, 10, 0)
        .await
        .unwrap();
    assert_eq!(msg_total, 1);
}

/// Memory adapters: random tenant/user IDs (no FK).
pub async fn assert_conversation_crud_contract<S: ConversationStorage + ?Sized>(storage: &S) {
    assert_conversation_crud_contract_with_ids(storage, Uuid::new_v4(), Uuid::new_v4()).await;
}
