# ADR-0005: Async OpenAI for LLM Integration

## Status

Accepted

## Date

2024-02

## Context

EdgeQuake requires integration with LLM providers for:

1. **Entity extraction**: Parsing entities and relationships from text
2. **Embeddings**: Converting text to vectors for similarity search
3. **Query answering**: Generating responses from retrieved context
4. **Summarization**: Condensing entity descriptions

We need a reliable, async-compatible client library for OpenAI's API.

## Decision

We chose **async-openai** as the primary LLM client library:

### Library Selection

| Library       | Async | Streaming | Types | Maintenance |
| ------------- | ----- | --------- | ----- | ----------- |
| async-openai  | ✅    | ✅        | ✅    | ✅ Active   |
| openai-api-rs | ❌    | ❌        | ⚠️    | ⚠️ Sporadic |
| raw reqwest   | ✅    | ✅        | ❌    | N/A         |

### Why async-openai?

1. **Full async support**: Native tokio integration
2. **Streaming responses**: SSE parsing for chat completions
3. **Type-safe models**: Rust structs for all API types
4. **Active maintenance**: Regular updates for new API features
5. **Retry support**: Built-in retry with exponential backoff

### Provider Abstraction

We wrap async-openai in our own traits for flexibility:

```rust
#[async_trait]
pub trait LLMProvider: Send + Sync {
    async fn chat_completion(&self, messages: Vec<Message>, config: Config)
        -> Result<String>;

    async fn chat_completion_stream(&self, messages: Vec<Message>, config: Config)
        -> Result<impl Stream<Item = Result<String>>>;
}

#[async_trait]
pub trait EmbeddingProvider: Send + Sync {
    async fn embed(&self, texts: &[String]) -> Result<Vec<Vec<f32>>>;
    fn dimension(&self) -> usize;
}
```

### Implementation

```rust
pub struct OpenAIProvider {
    client: Client<OpenAIConfig>,
    model: String,
    embedding_model: String,
}

impl LLMProvider for OpenAIProvider {
    async fn chat_completion(&self, messages: Vec<Message>, config: Config)
        -> Result<String>
    {
        let request = CreateChatCompletionRequestArgs::default()
            .model(&self.model)
            .messages(messages)
            .build()?;

        let response = self.client
            .chat()
            .create(request)
            .await?;

        Ok(response.choices[0].message.content.clone())
    }
}
```

## Consequences

### Positive

- **Type safety**: Compile-time verification of API calls
- **Streaming**: Efficient memory use for long responses
- **Retry logic**: Automatic handling of rate limits
- **OpenAI-compatible**: Works with Azure OpenAI, local models
- **Testable**: Mock providers for unit tests

### Negative

- **OpenAI-centric**: API designed around OpenAI models
- **Dependency lock-in**: Breaking changes in library updates
- **Extra abstraction**: Our traits add complexity
- **Feature lag**: New OpenAI features may lag in library

### Mitigations

- Abstract behind our own traits for flexibility
- Pin library version, controlled upgrades
- MockProvider for testing without API calls
- Monitor async-openai releases for new features

### Future Considerations

- Support for other providers (Anthropic, Google, local)
- Batch embedding API when available
- Function calling / tool use integration
- Vision model support
