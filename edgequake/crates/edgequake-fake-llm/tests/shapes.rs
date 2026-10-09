use edgequake_fake_llm::{spawn_ephemeral, FakeLlmState};

#[tokio::test]
async fn openai_anthropic_ollama_and_faults() {
    let state = FakeLlmState {
        require_key: Some("test-key".into()),
        ..Default::default()
    };
    let (addr, _h) = spawn_ephemeral(state).await.unwrap();
    let base = format!("http://{addr}");
    let client = reqwest::Client::new();

    let models = client
        .get(format!("{base}/v1/models"))
        .header("x-api-key", "test-key")
        .send()
        .await
        .unwrap();
    assert!(models.status().is_success());

    let chat = client
        .post(format!("{base}/v1/chat/completions"))
        .header("authorization", "Bearer test-key")
        .json(&serde_json::json!({"messages":[{"role":"user","content":"hi"}]}))
        .send()
        .await
        .unwrap()
        .json::<serde_json::Value>()
        .await
        .unwrap();
    assert!(chat["choices"][0]["message"]["content"]
        .as_str()
        .unwrap()
        .contains("echo:hi"));

    let embed = client
        .post(format!("{base}/v1/embeddings"))
        .header("x-api-key", "test-key")
        .json(&serde_json::json!({"input": "x"}))
        .send()
        .await
        .unwrap()
        .json::<serde_json::Value>()
        .await
        .unwrap();
    assert_eq!(embed["data"][0]["embedding"].as_array().unwrap().len(), 8);

    let wrong = client
        .post(format!("{base}/v1/embeddings?mode=wrong-dim"))
        .header("x-api-key", "test-key")
        .json(&serde_json::json!({"input": "x"}))
        .send()
        .await
        .unwrap()
        .json::<serde_json::Value>()
        .await
        .unwrap();
    assert_eq!(wrong["data"][0]["embedding"].as_array().unwrap().len(), 9);

    let unauth = client
        .post(format!("{base}/v1/messages"))
        .json(&serde_json::json!({"messages":[{"role":"user","content":"hi"}]}))
        .send()
        .await
        .unwrap();
    assert_eq!(unauth.status(), 401);

    let anth = client
        .post(format!("{base}/v1/messages"))
        .header("x-api-key", "test-key")
        .json(&serde_json::json!({"messages":[{"role":"user","content":"hi"}]}))
        .send()
        .await
        .unwrap();
    assert!(anth.status().is_success());

    let tags = client.get(format!("{base}/api/tags")).send().await.unwrap();
    assert!(tags.status().is_success());

    let five = client
        .get(format!("{base}/v1/models?mode=500"))
        .header("x-api-key", "test-key")
        .send()
        .await
        .unwrap();
    assert_eq!(five.status(), 500);
}
