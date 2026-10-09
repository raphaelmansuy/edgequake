use edgequake_fake_llm::{app, FakeLlmState};
use std::net::SocketAddr;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt().with_env_filter("info").init();
    let port: u16 = std::env::var("FAKE_LLM_PORT")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(18080);
    let dim: usize = std::env::var("FAKE_LLM_EMBED_DIM")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(8);
    let key = std::env::var("FAKE_LLM_API_KEY")
        .ok()
        .filter(|s| !s.is_empty());
    let state = FakeLlmState {
        embedding_dimension: dim,
        model: std::env::var("FAKE_LLM_MODEL").unwrap_or_else(|_| "fake-chat".into()),
        require_key: key,
        hits: Default::default(),
    };
    let addr = SocketAddr::from(([127, 0, 0, 1], port));
    tracing::info!(%addr, dim, "SPEC-163 fake LLM listening");
    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .expect("bind fake llm");
    axum::serve(listener, app(state)).await.expect("serve");
}
