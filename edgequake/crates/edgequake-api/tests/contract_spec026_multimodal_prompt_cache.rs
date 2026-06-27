//! SPEC-026 Phase 4i — prompt context + KV analysis cache contract.

use std::sync::Arc;

use edgequake_api::services::{
    analysis_cache_enabled, analyze_multimodal_images, table_analysis_messages,
    MultimodalProviders, PromptContext,
};
use edgequake_llm::MockProvider;
use edgequake_storage::adapters::memory::MemoryKVStorage;
use edgequake_storage::traits::KVStorage;

#[test]
#[serial_test::serial]
fn table_prompt_includes_additional_context_block() {
    let ctx = PromptContext {
        language: "English".into(),
        captions: "Revenue breakdown".into(),
        footnotes: "All figures in USD".into(),
        leading: "See quarterly results".into(),
        trailing: "Source: finance team".into(),
    };
    let msgs = table_analysis_messages("<tr><td>100</td></tr>", "html", &ctx).unwrap();
    let user = &msgs[1].content;
    assert!(user.contains("================ ADDITIONAL CONTEXT ================"));
    assert!(user.contains("Revenue breakdown"));
    assert!(user.contains("HTML format"));
}

#[tokio::test]
#[serial_test::serial]
async fn analysis_cache_skips_second_llm_call() {
    std::env::set_var("EDGEQUAKE_MM_ANALYSIS_CACHE", "1");
    std::env::set_var("VLM_PROCESS_ENABLE", "true");
    std::env::set_var("VLM_MIN_IMAGE_PIXEL", "1");

    let b64 = "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mP8z8BQDwAEhQGAhKmMIQAAAABJRU5ErkJggg==";
    let md = format!("Intro ![x](data:image/png;base64,{b64}) end.");
    let mock = MockProvider::new();
    mock.add_response(r#"{"name":"cached_chart","type":"Chart","description":"First call."}"#)
        .await;

    let kv: Arc<dyn KVStorage> = Arc::new(MemoryKVStorage::new("mm_cache_contract"));

    let first = analyze_multimodal_images(
        &md,
        Some("i"),
        "doc.pdf",
        MultimodalProviders::single(&mock),
        None,
        Some(Arc::clone(&kv)),
    )
    .await;
    assert_eq!(first.summary.success, 1);
    assert!(first.markdown.contains("cached chart"));

    let second = analyze_multimodal_images(
        &md,
        Some("i"),
        "doc.pdf",
        MultimodalProviders::single(&mock),
        None,
        Some(kv),
    )
    .await;
    assert_eq!(second.summary.success, 1);
    assert!(second.markdown.contains("cached chart"));
    assert!(analysis_cache_enabled());

    std::env::remove_var("EDGEQUAKE_MM_ANALYSIS_CACHE");
    std::env::remove_var("VLM_MIN_IMAGE_PIXEL");
    std::env::remove_var("VLM_PROCESS_ENABLE");
}
