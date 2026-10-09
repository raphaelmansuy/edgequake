//! SPEC-163: probe OpenAI / Anthropic / Ollama shapes against the fake LLM.

use edgequake_api::providers::probe::{probe_provider, ProbeErrorKind, ProbeRequest};
use edgequake_fake_llm::{spawn_ephemeral, FakeLlmState};

#[tokio::test]
async fn openai_shape_ok_and_401() {
    let state = FakeLlmState {
        require_key: Some("secret".into()),
        embedding_dimension: 8,
        ..Default::default()
    };
    let (addr, _h) = spawn_ephemeral(state).await.unwrap();
    let base = format!("http://{addr}");

    let ok = probe_provider(ProbeRequest {
        shape: "openai_chat".into(),
        base_url: Some(base.clone()),
        model: Some("fake-chat".into()),
        embedding_model: Some("fake-embed".into()),
        api_key: Some("secret".into()),
        auth_scheme: Some("bearer".into()),
        allow_private_network: Some(true),
        expected_dimension: Some(8),
    })
    .await;
    assert!(ok.ok, "{ok:?}");
    assert_eq!(ok.embedding_dimension, Some(8));

    let unauth = probe_provider(ProbeRequest {
        shape: "openai_chat".into(),
        base_url: Some(base.clone()),
        model: None,
        embedding_model: None,
        api_key: Some("wrong".into()),
        auth_scheme: Some("bearer".into()),
        allow_private_network: Some(true),
        expected_dimension: None,
    })
    .await;
    assert_eq!(unauth.kind, ProbeErrorKind::Unauthorized);

    let anth = probe_provider(ProbeRequest {
        shape: "anthropic_messages".into(),
        base_url: Some(base.clone()),
        model: Some("fake-chat".into()),
        embedding_model: None,
        api_key: Some("secret".into()),
        auth_scheme: Some("x_api_key".into()),
        allow_private_network: Some(true),
        expected_dimension: None,
    })
    .await;
    assert!(anth.chat_ok, "{anth:?}");

    let ollama = probe_provider(ProbeRequest {
        shape: "ollama".into(),
        base_url: Some(base),
        model: Some("fake-chat".into()),
        embedding_model: None,
        api_key: None,
        auth_scheme: Some("none".into()),
        allow_private_network: Some(true),
        expected_dimension: None,
    })
    .await;
    assert!(ollama.list_ok, "{ollama:?}");
}

#[tokio::test]
async fn ssrf_blocks_metadata() {
    let r = probe_provider(ProbeRequest {
        shape: "openai_chat".into(),
        base_url: Some("http://169.254.169.254/".into()),
        model: None,
        embedding_model: None,
        api_key: None,
        auth_scheme: None,
        allow_private_network: Some(true),
        expected_dimension: None,
    })
    .await;
    assert_eq!(r.kind, ProbeErrorKind::SsrfDenied);
}
