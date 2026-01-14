# Iteration 132 – Orient

## Analysis

### Provider/Model Lineage in API

From OODA 122, QueryStats was extended with:
```rust
pub struct QueryStats {
    pub tokens_used: Option<usize>,
    pub tokens_per_second: Option<f32>,
    pub llm_provider: Option<String>,  // NEW
    pub llm_model: Option<String>,     // NEW
}
```

### UI Display

Found in [chat-message.tsx](edgequake_webui/src/components/query/chat-message.tsx) (lines 273-299):

```tsx
{/* SPEC-032: Show tokens per second with model name for performance insight */}
{tokensUsed && durationMs && durationMs > 0 && (
  <span className="flex items-center gap-1 text-emerald-600">
    <Gauge className="h-3 w-3" />
    {((tokensUsed / durationMs) * 1000).toFixed(1)}/s
    {/* REQ-22: Display model after tokens/second */}
    {(llmProvider || llmModel) && (
      <span className="text-muted-foreground">
        • {llmProvider && llmModel ? `${llmProvider}/${llmModel}` : llmProvider || llmModel}
      </span>
    )}
  </span>
)}
```

### Data Flow

1. **query-interface.tsx** (lines 496-497, 672-674):
   - Receives `llm_provider` and `llm_model` from API response
   - Passes to ChatMessage component

2. **Tooltip Content**:
   - Shows generation speed
   - Shows model used

### Display Format

```
58.5/s • ollama/gemma3:12b
```

## Conclusion

**Item 15 (Provider/Model lineage): VERIFIED COMPLETE**

- Stored in QueryStats
- Returned via API
- Displayed in WebUI with tokens/second
