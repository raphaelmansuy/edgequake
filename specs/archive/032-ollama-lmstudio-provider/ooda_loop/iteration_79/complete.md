# OODA Iteration 79: ChatOptions Serialization

## Observe

Verify ChatOptions correctly serializes for Ollama API.

## Orient

ChatOptions struct needs:

- temperature (optional)
- num_predict (max tokens)
- stop (stop sequences)

## Decide

Review serialization with serde.

## Act

From ollama.rs:

```rust
#[derive(Debug, Serialize)]
struct ChatOptions {
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    num_predict: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    stop: Option<Vec<String>>,
}
```

✅ ChatOptions correctly serializes with optional fields
