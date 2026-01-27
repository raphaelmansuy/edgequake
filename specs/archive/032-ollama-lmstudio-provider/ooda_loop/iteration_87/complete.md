# OODA Iteration 87: Trait Method Default Implementation

## Observe

Verify default implementation for stream_with_options.

## Orient

New providers should work even if they don't implement stream_with_options.

## Decide

Review trait default implementation.

## Act

From traits.rs:

```rust
async fn stream_with_options(
    &self,
    prompt: &str,
    _options: &CompletionOptions,
) -> Result<BoxStream<'static, Result<String>>> {
    // Default: delegate to stream() ignoring options
    self.stream(prompt).await
}
```

This ensures backward compatibility:

- Old providers work without changes
- New providers can override for stop token support

✅ Default implementation provides backward compatibility
