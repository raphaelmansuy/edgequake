//! Back-compat inline enrich — delegates to [`super::analyzer`].

use edgequake_llm::traits::LLMProvider;

use super::analyzer::analyze_multimodal_images;
use super::providers::MultimodalProviders;

/// Enrich converted PDF markdown with VLM analysis for inline images.
pub async fn enrich_markdown_with_vlm(
    markdown: &str,
    process_options: Option<&str>,
    filename: &str,
    llm: &dyn LLMProvider,
) -> String {
    analyze_multimodal_images(
        markdown,
        process_options,
        filename,
        MultimodalProviders::single(llm),
        None,
        None,
    )
    .await
    .markdown
}

#[cfg(test)]
mod tests {
    use super::*;
    use edgequake_llm::MockProvider;

    #[tokio::test]
    async fn skips_when_images_flag_disabled() {
        let md = "<drawing id=\"x\" />";
        let mock = MockProvider::new();
        let out = enrich_markdown_with_vlm(md, Some(""), "doc.md", &mock).await;
        assert_eq!(out, md);
    }

    #[tokio::test]
    #[serial_test::serial]
    async fn replaces_data_uri_when_images_enabled() {
        std::env::set_var("VLM_PROCESS_ENABLE", "true");
        std::env::set_var("VLM_MIN_IMAGE_PIXEL", "1");
        let b64 = "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mP8z8BQDwAEhQGAhKmMIQAAAABJRU5ErkJggg==";
        let md = format!("Text ![x](data:image/png;base64,{b64}) end.");
        let mock = MockProvider::new();
        mock.add_response(
            r#"{"name":"inline_chart","type":"Chart","description":"A tiny chart."}"#,
        )
        .await;
        let out = enrich_markdown_with_vlm(&md, Some("i"), "doc.md", &mock).await;
        assert!(out.contains("# inline chart"));
        assert!(out.contains("Chart"));
        assert!(!out.contains("data:image/png;base64"));
        std::env::remove_var("VLM_MIN_IMAGE_PIXEL");
        std::env::remove_var("VLM_PROCESS_ENABLE");
    }

    #[tokio::test]
    #[serial_test::serial]
    async fn keeps_data_uri_when_below_min_pixels() {
        std::env::set_var("VLM_PROCESS_ENABLE", "true");
        std::env::set_var("VLM_MIN_IMAGE_PIXEL", "64");
        let b64 = "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mP8z8BQDwAEhQGAhKmMIQAAAABJRU5ErkJggg==";
        let md = format!("Text ![x](data:image/png;base64,{b64}) end.");
        let mock = MockProvider::new();
        mock.add_response(r#"{"name":"x","type":"Chart","description":"y"}"#)
            .await;
        let out = enrich_markdown_with_vlm(&md, Some("i"), "doc.md", &mock).await;
        assert!(out.contains("data:image/png;base64"));
        std::env::remove_var("VLM_MIN_IMAGE_PIXEL");
        std::env::remove_var("VLM_PROCESS_ENABLE");
    }
}
