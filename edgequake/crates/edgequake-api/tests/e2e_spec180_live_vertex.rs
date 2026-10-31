//! SPEC-180 — live Vertex `gemini-embedding-2`, text and image, against Google.
//!
//! ```bash
//! gcp-use-mobioos                     # activates vertex-express@mobioos-506015
//! cargo test -p edgequake-api --features postgres \
//!   --test e2e_spec180_live_vertex -- --ignored --nocapture
//! ```
//!
//! These tests are `#[ignore]` because CI must not call Google (16-e2e-test-matrix). They
//! pin [`LIVE_PROJECT`] themselves: a shell that exports `GOOGLE_CLOUD_PROJECT=saas-app-001`
//! must not leak in and turn a working client into a 403 (LAW-180-15).
//!
//! The token is minted by `gcloud auth print-access-token` on the server side and is never
//! read, printed or stored here.

use edgequake_core::multimodal_embed::{EmbedInput, MultimodalEmbedder};

/// SPEC-180 LAW-180-15: the project is pinned here, never inherited from a developer shell.
/// `saas-app-001` is a different project that answers 403 for this model, so an inherited
/// export would make a live test fail for a reason that has nothing to do with the code.
const LIVE_PROJECT: &str = "mobioos-506015";

/// Widths the model must serve (LAW-180-5).
const DIMENSIONS: [usize; 3] = [768, 1536, 3072];

/// The backend pins the project; this pins it for the client under test too.
fn pin_project() -> Result<(), String> {
    // WHY set_var before building the client: `VertexGeminiConfig::from_env` reads the project
    // once, at construction. Setting it afterwards would silently test the inherited value.
    std::env::set_var("GOOGLE_CLOUD_PROJECT", LIVE_PROJECT);
    std::env::remove_var("EDGEQUAKE_VERTEX_BASE_URL");
    Ok(())
}

fn a_png() -> Vec<u8> {
    let image = image::RgbImage::from_pixel(24, 24, image::Rgb([30, 90, 200]));
    let mut bytes = Vec::new();
    image
        .write_to(
            &mut std::io::Cursor::new(&mut bytes),
            image::ImageFormat::Png,
        )
        .expect("png");
    bytes
}

#[test]
fn t_180_live_project_is_the_one_that_may_call_vertex() {
    // The constant is the contract; a typo here fails every live test confusingly.
    assert_eq!(LIVE_PROJECT, "mobioos-506015");
}

#[tokio::test]
#[ignore = "live Vertex: run with gcp-use-mobioos, --ignored"]
async fn t_180_text_embed_live() {
    pin_project().expect("project");

    for dimension in DIMENSIONS {
        let embedder = edgequake_api::embeddings::VertexGeminiEmbedder::from_env(
            "gemini-embedding-2",
            dimension,
        )
        .unwrap_or_else(|e| panic!("client at {dimension}: {} {}", e.code, e.message));

        let vectors = embedder
            .embed(&[EmbedInput::text(
                "title: refund | text: refunds within 30 days",
            )])
            .await
            .unwrap_or_else(|e| panic!("embed text at {dimension}: {} {}", e.code, e.message));

        assert_eq!(vectors.len(), 1);
        assert_eq!(vectors[0].len(), dimension, "width at {dimension}");
        let norm: f32 = vectors[0].iter().map(|v| v * v).sum::<f32>().sqrt();
        assert!(
            (norm - 1.0).abs() < 1e-2,
            "vector must be L2-normalised, got {norm}"
        );
    }
}

#[tokio::test]
#[ignore = "live Vertex: run with gcp-use-mobioos, --ignored"]
async fn t_180_image_embed_live() {
    pin_project().expect("project");

    let embedder =
        edgequake_api::embeddings::VertexGeminiEmbedder::from_env("gemini-embedding-2", 1536)
            .expect("client");

    let vectors = embedder
        .embed(&[EmbedInput::image("image/png", a_png())])
        .await
        .unwrap_or_else(|e| panic!("embed image: {} {}", e.code, e.message));

    assert_eq!(vectors[0].len(), 1536);
}

#[tokio::test]
#[ignore = "live Vertex: run with gcp-use-mobioos, --ignored"]
async fn t_180_image_and_text_share_one_space_live() {
    pin_project().expect("project");

    let embedder =
        edgequake_api::embeddings::VertexGeminiEmbedder::from_env("gemini-embedding-2", 1536)
            .expect("client");

    // The same picture twice must be closer than the picture and an unrelated sentence.
    let picture = a_png();
    let vectors = embedder
        .embed(&[
            EmbedInput::image("image/png", picture.clone()),
            EmbedInput::image_with_text("image/png", picture, "a blue square"),
            EmbedInput::text("task: search result | query: quarterly revenue by region"),
        ])
        .await
        .unwrap_or_else(|e| panic!("embed mixed: {} {}", e.code, e.message));

    let cosine = |a: &[f32], b: &[f32]| -> f32 { a.iter().zip(b).map(|(x, y)| x * y).sum() };
    let same_picture = cosine(&vectors[0], &vectors[1]);
    let picture_vs_sentence = cosine(&vectors[0], &vectors[2]);
    assert!(
        same_picture > picture_vs_sentence,
        "a picture should be nearer its own caption ({same_picture}) than an unrelated \
         sentence ({picture_vs_sentence})"
    );
}

#[tokio::test]
#[ignore = "live Vertex: run with gcp-use-mobioos, --ignored"]
async fn t_180_probe_reports_success_for_the_live_project() {
    pin_project().expect("project");
    edgequake_api::services::embed_capability_probe::forget_all();

    let outcome = edgequake_api::services::embed_capability_probe::probe_image(
        "vertexai",
        "gemini-embedding-2",
        1536,
    )
    .await
    .expect("the pair is one this build can probe");

    // The create-time probe and this test call the same function, so a pass here means the
    // model step would also let an image workspace through.
    assert!(
        outcome.is_ok(),
        "probe failed for {LIVE_PROJECT}: {:?}",
        outcome.err().map(|e| (e.code, e.message))
    );
}

#[tokio::test]
#[ignore = "live Vertex: run with gcp-use-mobioos, --ignored"]
async fn t_180_probe_names_a_bad_project() {
    // The failure path must be as useful as the success path: this is the 403 the wizard used
    // to hide until Review. A project with no Vertex access must produce `embedding_auth` and
    // a message naming the project.
    std::env::set_var("GOOGLE_CLOUD_PROJECT", "saas-app-001");
    std::env::remove_var("EDGEQUAKE_VERTEX_BASE_URL");
    edgequake_api::services::embed_capability_probe::forget_all();

    let outcome = edgequake_api::services::embed_capability_probe::probe_image(
        "vertexai",
        "gemini-embedding-2",
        1536,
    )
    .await
    .expect("probeable pair");

    std::env::set_var("GOOGLE_CLOUD_PROJECT", LIVE_PROJECT);

    let error = outcome.expect_err("saas-app-001 must not serve gemini-embedding-2");
    assert_eq!(error.code, "embedding_auth");
    let message = edgequake_api::services::embed_capability_probe::action_message(&error);
    assert!(
        message.contains("saas-app-001"),
        "the message must name the project: {message}"
    );
    assert!(
        message.contains("aiplatform.endpoints.predict"),
        "the message must name the fix: {message}"
    );
}
