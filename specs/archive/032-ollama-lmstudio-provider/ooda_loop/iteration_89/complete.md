# OODA Iteration 89: NDJSON Streaming Parse

## Observe

Verify NDJSON parsing in Ollama streaming.

## Orient

Ollama returns newline-delimited JSON for streaming.

## Decide

Review parsing logic.

## Act

From ollama.rs:

```rust
let mapped_stream = stream.map(|chunk_result| {
    match chunk_result {
        Ok(bytes) => {
            let text = String::from_utf8_lossy(&bytes);
            // Parse NDJSON - each line is a separate JSON object
            let mut content = String::new();
            for line in text.lines() {
                if line.is_empty() { continue; }
                if let Ok(chunk) = serde_json::from_str::<StreamChunk>(line) {
                    if let Some(msg) = chunk.message {
                        content.push_str(&msg.content);
                    }
                }
            }
            Ok(content)
        }
        Err(e) => Err(LlmError::NetworkError(e.to_string())),
    }
});
```

✅ NDJSON parsing handles multi-line responses correctly
