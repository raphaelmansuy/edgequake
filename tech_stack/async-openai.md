# async-openai LLM Client Guide

**Version**: 0.32+  
**Category**: LLM Integration  
**Use Case**: OpenAI API client for entity extraction and query generation  
**Official Docs**: https://docs.rs/async-openai/latest/async_openai/

---

## Overview

async-openai is a comprehensive Rust client for the OpenAI API, built on Tokio. It provides type-safe access to chat completions, embeddings, and all OpenAI services.

### Key Features

- **Complete API Coverage**: Chat, completions, embeddings, images, audio, etc.
- **Type-Safe**: Strong typing for requests and responses
- **Async**: Built on Tokio + reqwest
- **Configurable**: Support for Azure, custom endpoints
- **Streaming**: SSE support for streaming responses

---

## Installation

### Cargo.toml

```toml
[dependencies]
async-openai = "0.32"
tokio = { version = "1", features = ["full"] }
serde = { version = "1", features = ["derive"] }
```

---

## Progressive Examples

### 1. Basic Setup

```rust
use async_openai::{Client, config::OpenAIConfig};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create client with API key from env var OPENAI_API_KEY
    let client = Client::new();
    
    // Or specify API key explicitly
    let config = OpenAIConfig::new()
        .with_api_key("sk-...");
    let client = Client::with_config(config);
    
    println!("Client initialized");
    
    Ok(())
}
```

**Environment Variable**:
```bash
export OPENAI_API_KEY="sk-..."
```

### 2. Chat Completion

```rust
use async_openai::{
    Client,
    types::{
        ChatCompletionRequestMessage,
        CreateChatCompletionRequestArgs,
        Role,
    },
};

async fn chat_example(client: &Client<OpenAIConfig>) -> Result<String, Box<dyn std::error::Error>> {
    let request = CreateChatCompletionRequestArgs::default()
        .model("gpt-4o-mini")
        .messages(vec![
            ChatCompletionRequestMessage::System {
                content: Some("You are a helpful assistant.".to_string()),
                name: None,
            },
            ChatCompletionRequestMessage::User {
                content: Some("What is Rust?".to_string()),
                name: None,
            },
        ])
        .max_tokens(512u32)
        .build()?;
    
    let response = client.chat().create(request).await?;
    
    let content = response.choices[0]
        .message
        .content
        .as_ref()
        .unwrap()
        .clone();
    
    Ok(content)
}
```

### 3. Embeddings Generation

```rust
use async_openai::types::CreateEmbeddingRequestArgs;

async fn generate_embedding(
    client: &Client<OpenAIConfig>,
    text: &str,
) -> Result<Vec<f32>, Box<dyn std::error::Error>> {
    let request = CreateEmbeddingRequestArgs::default()
        .model("text-embedding-3-small")
        .input(text)
        .build()?;
    
    let response = client.embeddings().create(request).await?;
    
    let embedding = response.data[0]
        .embedding
        .iter()
        .map(|&x| x as f32)
        .collect();
    
    Ok(embedding)
}
```

### 4. Streaming Chat

```rust
use futures::StreamExt;

async fn streaming_chat(
    client: &Client<OpenAIConfig>,
) -> Result<(), Box<dyn std::error::Error>> {
    let request = CreateChatCompletionRequestArgs::default()
        .model("gpt-4o")
        .messages(vec![
            ChatCompletionRequestMessage::User {
                content: Some("Tell me a story".to_string()),
                name: None,
            },
        ])
        .build()?;
    
    let mut stream = client.chat().create_stream(request).await?;
    
    while let Some(result) = stream.next().await {
        match result {
            Ok(response) => {
                if let Some(choice) = response.choices.first() {
                    if let Some(content) = &choice.delta.content {
                        print!("{}", content);
                    }
                }
            }
            Err(e) => eprintln!("Error: {}", e),
        }
    }
    
    println!(); // newline
    
    Ok(())
}
```

---

## Production Pattern: LLM Provider Trait

### Trait Definition

```rust
use async_trait::async_trait;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum LLMError {
    #[error("API error: {0}")]
    ApiError(String),
    
    #[error("Invalid response: {0}")]
    InvalidResponse(String),
    
    #[error("Rate limit exceeded")]
    RateLimit,
}

#[async_trait]
pub trait LLMProvider: Send + Sync {
    async fn chat_completion(
        &self,
        messages: Vec<ChatMessage>,
        model: &str,
    ) -> Result<String, LLMError>;
    
    async fn generate_embedding(
        &self,
        text: &str,
        model: &str,
    ) -> Result<Vec<f32>, LLMError>;
    
    async fn batch_embeddings(
        &self,
        texts: Vec<String>,
        model: &str,
    ) -> Result<Vec<Vec<f32>>, LLMError>;
}

#[derive(Debug, Clone)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}
```

### OpenAI Implementation

```rust
use async_openai::{Client, config::OpenAIConfig};

pub struct OpenAIProvider {
    client: Client<OpenAIConfig>,
    default_model: String,
}

impl OpenAIProvider {
    pub fn new(api_key: Option<String>) -> Self {
        let config = if let Some(key) = api_key {
            OpenAIConfig::new().with_api_key(key)
        } else {
            OpenAIConfig::default()
        };
        
        Self {
            client: Client::with_config(config),
            default_model: "gpt-4o-mini".to_string(),
        }
    }
    
    pub fn with_model(mut self, model: String) -> Self {
        self.default_model = model;
        self
    }
}

#[async_trait]
impl LLMProvider for OpenAIProvider {
    async fn chat_completion(
        &self,
        messages: Vec<ChatMessage>,
        model: &str,
    ) -> Result<String, LLMError> {
        let openai_messages: Vec<_> = messages
            .into_iter()
            .map(|msg| match msg.role.as_str() {
                "system" => ChatCompletionRequestMessage::System {
                    content: Some(msg.content),
                    name: None,
                },
                "user" => ChatCompletionRequestMessage::User {
                    content: Some(msg.content),
                    name: None,
                },
                "assistant" => ChatCompletionRequestMessage::Assistant {
                    content: Some(msg.content),
                    name: None,
                    tool_calls: None,
                },
                _ => ChatCompletionRequestMessage::User {
                    content: Some(msg.content),
                    name: None,
                },
            })
            .collect();
        
        let request = CreateChatCompletionRequestArgs::default()
            .model(model)
            .messages(openai_messages)
            .build()
            .map_err(|e| LLMError::ApiError(e.to_string()))?;
        
        let response = self.client
            .chat()
            .create(request)
            .await
            .map_err(|e| LLMError::ApiError(e.to_string()))?;
        
        response.choices
            .first()
            .and_then(|choice| choice.message.content.clone())
            .ok_or_else(|| LLMError::InvalidResponse("No content in response".to_string()))
    }
    
    async fn generate_embedding(
        &self,
        text: &str,
        model: &str,
    ) -> Result<Vec<f32>, LLMError> {
        let request = CreateEmbeddingRequestArgs::default()
            .model(model)
            .input(text)
            .build()
            .map_err(|e| LLMError::ApiError(e.to_string()))?;
        
        let response = self.client
            .embeddings()
            .create(request)
            .await
            .map_err(|e| LLMError::ApiError(e.to_string()))?;
        
        Ok(response.data[0]
            .embedding
            .iter()
            .map(|&x| x as f32)
            .collect())
    }
    
    async fn batch_embeddings(
        &self,
        texts: Vec<String>,
        model: &str,
    ) -> Result<Vec<Vec<f32>>, LLMError> {
        let request = CreateEmbeddingRequestArgs::default()
            .model(model)
            .input(texts)
            .build()
            .map_err(|e| LLMError::ApiError(e.to_string()))?;
        
        let response = self.client
            .embeddings()
            .create(request)
            .await
            .map_err(|e| LLMError::ApiError(e.to_string()))?;
        
        Ok(response.data
            .iter()
            .map(|data| data.embedding.iter().map(|&x| x as f32).collect())
            .collect())
    }
}
```

---

## LightRAG Use Cases

### 1. Entity Extraction

```rust
async fn extract_entities(
    provider: &dyn LLMProvider,
    chunk: &str,
) -> Result<Vec<Entity>, LLMError> {
    let prompt = format!(
        r#"Extract entities from the following text. 
        
Format each entity as: entity_name|entity_type|description
Use <|#|> as delimiter between fields.
Use <|COMPLETE|> to end the list.

Text:
{}

Entities:"#,
        chunk
    );
    
    let messages = vec![
        ChatMessage {
            role: "system".to_string(),
            content: "You are an entity extraction expert.".to_string(),
        },
        ChatMessage {
            role: "user".to_string(),
            content: prompt,
        },
    ];
    
    let response = provider
        .chat_completion(messages, "gpt-4o-mini")
        .await?;
    
    parse_entities(&response)
}

fn parse_entities(response: &str) -> Result<Vec<Entity>, LLMError> {
    let mut entities = Vec::new();
    
    for line in response.lines() {
        if line.contains("<|COMPLETE|>") {
            break;
        }
        
        let parts: Vec<&str> = line.split("<|#|>").collect();
        if parts.len() >= 3 {
            entities.push(Entity {
                name: parts[0].trim().to_uppercase(),
                entity_type: parts[1].trim().to_lowercase(),
                description: parts[2].trim().to_string(),
            });
        }
    }
    
    Ok(entities)
}
```

### 2. Query Generation with Context

```rust
async fn generate_answer(
    provider: &dyn LLMProvider,
    question: &str,
    context: &[String],
) -> Result<String, LLMError> {
    let context_text = context.join("\n\n");
    
    let messages = vec![
        ChatMessage {
            role: "system".to_string(),
            content: "You are a helpful assistant that answers questions based on provided context.".to_string(),
        },
        ChatMessage {
            role: "user".to_string(),
            content: format!(
                "Context:\n{}\n\nQuestion: {}\n\nAnswer:",
                context_text, question
            ),
        },
    ];
    
    provider.chat_completion(messages, "gpt-4o").await
}
```

### 3. Caching Layer

```rust
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

pub struct CachedLLMProvider<T: LLMProvider> {
    inner: T,
    cache: Arc<RwLock<HashMap<String, String>>>,
}

impl<T: LLMProvider> CachedLLMProvider<T> {
    pub fn new(provider: T) -> Self {
        Self {
            inner: provider,
            cache: Arc::new(RwLock::new(HashMap::new())),
        }
    }
    
    fn cache_key(messages: &[ChatMessage], model: &str) -> String {
        use std::hash::{Hash, Hasher};
        use std::collections::hash_map::DefaultHasher;
        
        let mut hasher = DefaultHasher::new();
        for msg in messages {
            msg.role.hash(&mut hasher);
            msg.content.hash(&mut hasher);
        }
        model.hash(&mut hasher);
        
        format!("{:x}", hasher.finish())
    }
}

#[async_trait]
impl<T: LLMProvider> LLMProvider for CachedLLMProvider<T> {
    async fn chat_completion(
        &self,
        messages: Vec<ChatMessage>,
        model: &str,
    ) -> Result<String, LLMError> {
        let key = Self::cache_key(&messages, model);
        
        // Check cache
        {
            let cache = self.cache.read().await;
            if let Some(cached) = cache.get(&key) {
                return Ok(cached.clone());
            }
        }
        
        // Call inner provider
        let result = self.inner.chat_completion(messages, model).await?;
        
        // Store in cache
        {
            let mut cache = self.cache.write().await;
            cache.insert(key, result.clone());
        }
        
        Ok(result)
    }
    
    async fn generate_embedding(
        &self,
        text: &str,
        model: &str,
    ) -> Result<Vec<f32>, LLMError> {
        // Embeddings are deterministic, so always cache
        let key = format!("{}:{}", model, text);
        
        // Implementation similar to chat_completion
        self.inner.generate_embedding(text, model).await
    }
    
    async fn batch_embeddings(
        &self,
        texts: Vec<String>,
        model: &str,
    ) -> Result<Vec<Vec<f32>>, LLMError> {
        self.inner.batch_embeddings(texts, model).await
    }
}
```

---

## Best Practices (2025)

### Do's

✅ **Use builder pattern for requests**
```rust
let request = CreateChatCompletionRequestArgs::default()
    .model("gpt-4o-mini")
    .messages(messages)
    .temperature(0.7)
    .max_tokens(512u32)
    .build()?;
```

✅ **Handle rate limits gracefully**
```rust
use tokio::time::{sleep, Duration};

async fn with_retry<F, T>(mut f: F) -> Result<T, LLMError>
where
    F: FnMut() -> Pin<Box<dyn Future<Output = Result<T, LLMError>>>>,
{
    let mut retries = 3;
    loop {
        match f().await {
            Ok(result) => return Ok(result),
            Err(LLMError::RateLimit) if retries > 0 => {
                retries -= 1;
                sleep(Duration::from_secs(2)).await;
            }
            Err(e) => return Err(e),
        }
    }
}
```

✅ **Cache responses**
```rust
// Use CachedLLMProvider wrapper
let provider = CachedLLMProvider::new(OpenAIProvider::new(None));
```

✅ **Use appropriate models**
```rust
// For entity extraction (fast, cheap)
"gpt-4o-mini"

// For complex reasoning (slow, expensive)
"gpt-4o"

// For embeddings
"text-embedding-3-small" or "text-embedding-3-large"
```

### Don'ts

❌ **Don't expose API keys**
```rust
// Bad
let config = OpenAIConfig::new().with_api_key("sk-hardcoded");

// Good
let config = OpenAIConfig::default(); // Uses env var
```

❌ **Don't ignore errors**
```rust
// Bad
let response = client.chat().create(request).await.unwrap();

// Good
let response = client.chat().create(request).await
    .map_err(|e| LLMError::ApiError(e.to_string()))?;
```

❌ **Don't send sensitive data without encryption**
```rust
// Ensure HTTPS is used (default for OpenAI)
// For self-hosted: use TLS
```

---

## Testing

### Mock Provider

```rust
pub struct MockLLMProvider {
    responses: HashMap<String, String>,
}

impl MockLLMProvider {
    pub fn new() -> Self {
        Self {
            responses: HashMap::new(),
        }
    }
    
    pub fn with_response(mut self, key: &str, response: &str) -> Self {
        self.responses.insert(key.to_string(), response.to_string());
        self
    }
}

#[async_trait]
impl LLMProvider for MockLLMProvider {
    async fn chat_completion(
        &self,
        messages: Vec<ChatMessage>,
        _model: &str,
    ) -> Result<String, LLMError> {
        let key = messages.last().unwrap().content.clone();
        self.responses
            .get(&key)
            .cloned()
            .ok_or_else(|| LLMError::InvalidResponse("No mock response".to_string()))
    }
    
    async fn generate_embedding(
        &self,
        _text: &str,
        _model: &str,
    ) -> Result<Vec<f32>, LLMError> {
        Ok(vec![0.1; 1536])
    }
    
    async fn batch_embeddings(
        &self,
        texts: Vec<String>,
        _model: &str,
    ) -> Result<Vec<Vec<f32>>, LLMError> {
        Ok(texts.iter().map(|_| vec![0.1; 1536]).collect())
    }
}

#[tokio::test]
async fn test_entity_extraction() {
    let provider = MockLLMProvider::new()
        .with_response(
            "Extract entities...",
            "Rust<|#|>language<|#|>Systems programming\n<|COMPLETE|>"
        );
    
    let entities = extract_entities(&provider, "test").await.unwrap();
    assert_eq!(entities.len(), 1);
    assert_eq!(entities[0].name, "RUST");
}
```

---

## Official Resources

- **Documentation**: https://docs.rs/async-openai/latest/async_openai/
- **GitHub**: https://github.com/64bit/async-openai
- **Examples**: https://github.com/64bit/async-openai/tree/main/examples
- **OpenAI API Docs**: https://platform.openai.com/docs/api-reference

---

**Last Updated**: December 20, 2025  
**Version**: 1.0
