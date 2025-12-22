# Advanced Query Features

**Specification Version:** 1.0  
**Target Release:** EdgeQuake v1.1.0  
**Priority:** HIGH  
**Status:** Planning

---

## Overview

Enhance query capabilities with token budget controls, conversation history, custom keywords, and advanced retrieval parameters to match LightRAG functionality.

### Goals

1. **Token Budget Control:** Limit entity/relation/total tokens for cost optimization
2. **Conversation History:** Support multi-turn conversations with context
3. **Keyword Control:** High-level and low-level keyword extraction/usage
4. **Custom Prompts:** Allow user-defined prompt templates
5. **Rerank Control:** Explicit control over chunk reranking
6. **Bypass Mode:** Direct LLM queries without RAG retrieval
7. **Context-Only Mode:** Return retrieval context without generation

---

## Enhanced Query Request Model

```rust
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct QueryRequest {
    /// The query text (required)
    pub query: String,
    
    /// Query mode: naive, local, global, hybrid, mix, bypass
    #[serde(default = "default_mode")]
    pub mode: QueryMode,
    
    /// Maximum number of entities/relations to retrieve
    #[serde(default)]
    pub top_k: Option<usize>,
    
    /// Maximum number of text chunks to retrieve
    #[serde(default)]
    pub chunk_top_k: Option<usize>,
    
    /// Maximum tokens for entity context
    #[serde(default)]
    pub max_entity_tokens: Option<usize>,
    
    /// Maximum tokens for relationship context
    #[serde(default)]
    pub max_relation_tokens: Option<usize>,
    
    /// Maximum total tokens (entities + relations + chunks + prompt)
    #[serde(default)]
    pub max_total_tokens: Option<usize>,
    
    /// High-level keywords (optional, auto-extracted if empty)
    #[serde(default)]
    pub hl_keywords: Vec<String>,
    
    /// Low-level keywords (optional, auto-extracted if empty)
    #[serde(default)]
    pub ll_keywords: Vec<String>,
    
    /// Conversation history for multi-turn context
    #[serde(default)]
    pub conversation_history: Option<Vec<ConversationMessage>>,
    
    /// Custom user prompt (overrides default)
    #[serde(default)]
    pub user_prompt: Option<String>,
    
    /// Enable/disable chunk reranking
    #[serde(default = "default_enable_rerank")]
    pub enable_rerank: bool,
    
    /// Include references in response
    #[serde(default = "default_include_references")]
    pub include_references: bool,
    
    /// Include full chunk content in references (for debugging)
    #[serde(default)]
    pub include_chunk_content: bool,
    
    /// Return only context without generation
    #[serde(default)]
    pub context_only: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ConversationMessage {
    pub role: String,  // "user" or "assistant"
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum QueryMode {
    Naive,    // Simple vector search only
    Local,    // Entity-focused retrieval
    Global,   // Relationship-focused retrieval
    Hybrid,   // Combined entity + relationship
    Mix,      // Mixed retrieval strategy
    Bypass,   // Direct LLM without RAG
}

fn default_mode() -> QueryMode {
    QueryMode::Hybrid
}

fn default_enable_rerank() -> bool {
    true
}

fn default_include_references() -> bool {
    true
}
```

---

## Enhanced Query Response

```rust
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct QueryResponse {
    /// Generated answer
    pub answer: String,
    
    /// Query mode used
    pub mode: String,
    
    /// Source references (if include_references=true)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub references: Option<Vec<SourceReference>>,
    
    /// Query statistics
    pub stats: QueryStats,
    
    /// Extracted keywords (if not provided)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extracted_keywords: Option<ExtractedKeywords>,
    
    /// Token usage breakdown
    pub token_usage: TokenUsage,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct ExtractedKeywords {
    pub hl_keywords: Vec<String>,
    pub ll_keywords: Vec<String>,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct TokenUsage {
    pub entity_tokens: usize,
    pub relation_tokens: usize,
    pub chunk_tokens: usize,
    pub prompt_tokens: usize,
    pub total_tokens: usize,
    pub generation_tokens: usize,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct SourceReference {
    pub source_type: String,  // chunk, entity, relationship
    pub id: String,
    pub score: f32,
    pub snippet: Option<String>,
    
    /// Full content (if include_chunk_content=true)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub full_content: Option<String>,
}
```

---

## API Endpoints

### 1. Enhanced Query Endpoint

```http
POST /api/v1/query
Content-Type: application/json
```

**Request Example:**
```json
{
  "query": "What are the latest developments in AGI research?",
  "mode": "hybrid",
  "top_k": 60,
  "chunk_top_k": 5,
  "max_entity_tokens": 1000,
  "max_relation_tokens": 1000,
  "max_total_tokens": 4000,
  "hl_keywords": ["AGI", "artificial general intelligence"],
  "ll_keywords": ["neural networks", "transformer architecture"],
  "conversation_history": [
    {
      "role": "user",
      "content": "Tell me about AI research trends"
    },
    {
      "role": "assistant",
      "content": "Recent AI research has focused on large language models..."
    }
  ],
  "enable_rerank": true,
  "include_references": true,
  "include_chunk_content": false
}
```

**Response (200 OK):**
```json
{
  "answer": "Recent developments in AGI research include...",
  "mode": "hybrid",
  "references": [
    {
      "source_type": "entity",
      "id": "AGI_RESEARCH",
      "score": 0.95,
      "snippet": "Artificial General Intelligence research focuses on..."
    },
    {
      "source_type": "relationship",
      "id": "AGI_RESEARCH->NEURAL_NETWORKS",
      "score": 0.92,
      "snippet": "AGI research relies heavily on neural network architectures"
    },
    {
      "source_type": "chunk",
      "id": "doc-123-chunk-5",
      "score": 0.89,
      "snippet": "The latest paper on AGI by DeepMind discusses..."
    }
  ],
  "stats": {
    "embedding_time_ms": 45,
    "retrieval_time_ms": 120,
    "rerank_time_ms": 80,
    "generation_time_ms": 850,
    "total_time_ms": 1095,
    "sources_retrieved": 25,
    "sources_after_rerank": 10
  },
  "extracted_keywords": {
    "hl_keywords": ["AGI", "artificial general intelligence", "AI research"],
    "ll_keywords": ["neural networks", "transformer", "deep learning"]
  },
  "token_usage": {
    "entity_tokens": 850,
    "relation_tokens": 920,
    "chunk_tokens": 1200,
    "prompt_tokens": 450,
    "total_tokens": 3420,
    "generation_tokens": 320
  }
}
```

### 2. Context-Only Endpoint

```http
POST /api/v1/query/context
Content-Type: application/json
```

**Request:**
```json
{
  "query": "quantum computing applications",
  "mode": "hybrid",
  "top_k": 30,
  "max_total_tokens": 2000
}
```

**Response (200 OK):**
```json
{
  "query": "quantum computing applications",
  "mode": "hybrid",
  "context": {
    "entities": [
      {
        "id": "QUANTUM_COMPUTING",
        "name": "Quantum Computing",
        "entity_type": "TECHNOLOGY",
        "description": "Computing paradigm using quantum mechanics...",
        "score": 0.96
      }
    ],
    "relationships": [
      {
        "source": "QUANTUM_COMPUTING",
        "target": "CRYPTOGRAPHY",
        "relation_type": "APPLIES_TO",
        "description": "Quantum computing has applications in cryptography",
        "score": 0.94
      }
    ],
    "chunks": [
      {
        "id": "doc-456-chunk-12",
        "content": "Quantum computing applications include...",
        "document_id": "doc-456",
        "score": 0.91
      }
    ]
  },
  "extracted_keywords": {
    "hl_keywords": ["quantum computing", "quantum algorithms"],
    "ll_keywords": ["qubits", "superposition", "entanglement"]
  },
  "token_usage": {
    "entity_tokens": 650,
    "relation_tokens": 720,
    "chunk_tokens": 580,
    "total_tokens": 1950
  },
  "stats": {
    "embedding_time_ms": 42,
    "retrieval_time_ms": 115,
    "total_time_ms": 157,
    "entities_retrieved": 15,
    "relationships_retrieved": 22,
    "chunks_retrieved": 8
  }
}
```

---

## Implementation

### Token Budget Controller

```rust
pub struct TokenBudgetController {
    tokenizer: Arc<dyn Tokenizer>,
}

impl TokenBudgetController {
    pub fn enforce_budget(
        &self,
        context: RetrievalContext,
        config: TokenBudgetConfig,
    ) -> RetrievalContext {
        let mut entity_tokens = 0;
        let mut relation_tokens = 0;
        let mut chunk_tokens = 0;
        
        let mut filtered_entities = Vec::new();
        let mut filtered_relations = Vec::new();
        let mut filtered_chunks = Vec::new();
        
        // Process entities (sorted by score)
        for entity in context.entities {
            let tokens = self.tokenizer.count(&entity.description);
            if let Some(max) = config.max_entity_tokens {
                if entity_tokens + tokens > max {
                    break;
                }
            }
            entity_tokens += tokens;
            filtered_entities.push(entity);
        }
        
        // Process relationships
        for rel in context.relationships {
            let tokens = self.tokenizer.count(&rel.description);
            if let Some(max) = config.max_relation_tokens {
                if relation_tokens + tokens > max {
                    break;
                }
            }
            relation_tokens += tokens;
            filtered_relations.push(rel);
        }
        
        // Process chunks
        for chunk in context.chunks {
            let tokens = self.tokenizer.count(&chunk.content);
            if let Some(max) = config.max_total_tokens {
                let current_total = entity_tokens + relation_tokens + chunk_tokens;
                if current_total + tokens > max {
                    break;
                }
            }
            chunk_tokens += tokens;
            filtered_chunks.push(chunk);
        }
        
        RetrievalContext {
            entities: filtered_entities,
            relationships: filtered_relations,
            chunks: filtered_chunks,
            token_usage: TokenUsage {
                entity_tokens,
                relation_tokens,
                chunk_tokens,
                prompt_tokens: 0,  // Calculated later
                total_tokens: entity_tokens + relation_tokens + chunk_tokens,
                generation_tokens: 0,
            },
        }
    }
}
```

### Conversation History Manager

```rust
pub struct ConversationHistoryManager {
    storage: Arc<dyn ConversationStorage>,
}

impl ConversationHistoryManager {
    pub async fn get_session_history(
        &self,
        session_id: &str,
        max_messages: usize,
    ) -> Result<Vec<ConversationMessage>, Error> {
        self.storage
            .get_recent_messages(session_id, max_messages)
            .await
    }
    
    pub async fn add_message(
        &self,
        session_id: &str,
        message: ConversationMessage,
    ) -> Result<(), Error> {
        self.storage.insert_message(session_id, message).await
    }
    
    pub fn format_history_for_prompt(
        &self,
        history: &[ConversationMessage],
    ) -> String {
        history
            .iter()
            .map(|msg| format!("{}: {}", msg.role, msg.content))
            .collect::<Vec<_>>()
            .join("\n")
    }
}

// Database schema
CREATE TABLE conversation_history (
    id SERIAL PRIMARY KEY,
    session_id VARCHAR(100) NOT NULL,
    role VARCHAR(20) NOT NULL,  -- user, assistant
    content TEXT NOT NULL,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    metadata JSONB,
    CONSTRAINT valid_role CHECK (role IN ('user', 'assistant'))
);

CREATE INDEX idx_conv_session ON conversation_history(session_id, created_at);
```

### Keyword Extractor

```rust
pub struct KeywordExtractor {
    llm: Arc<dyn LLMProvider>,
}

impl KeywordExtractor {
    pub async fn extract_keywords(
        &self,
        query: &str,
    ) -> Result<ExtractedKeywords, Error> {
        let prompt = format!(
            r#"Extract keywords from this query:
Query: "{}"

Return two types of keywords:
1. High-level keywords (broad concepts, 3-5 keywords)
2. Low-level keywords (specific terms, 5-10 keywords)

Format as JSON:
{{
  "hl_keywords": ["keyword1", "keyword2", ...],
  "ll_keywords": ["term1", "term2", ...]
}}"#,
            query
        );
        
        let response = self.llm.generate(&prompt, None).await?;
        let keywords: ExtractedKeywords = serde_json::from_str(&response)?;
        
        Ok(keywords)
    }
    
    pub fn use_keywords_in_retrieval(
        &self,
        context: &mut RetrievalContext,
        hl_keywords: &[String],
        ll_keywords: &[String],
    ) {
        // Boost entities/relations matching high-level keywords
        for entity in &mut context.entities {
            if self.matches_keywords(&entity.name, hl_keywords) {
                entity.score *= 1.2;  // 20% boost
            }
        }
        
        // Boost chunks matching low-level keywords
        for chunk in &mut context.chunks {
            if self.matches_keywords(&chunk.content, ll_keywords) {
                chunk.score *= 1.1;  // 10% boost
            }
        }
        
        // Re-sort by updated scores
        context.entities.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap());
        context.chunks.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap());
    }
    
    fn matches_keywords(&self, text: &str, keywords: &[String]) -> bool {
        let text_lower = text.to_lowercase();
        keywords.iter().any(|kw| text_lower.contains(&kw.to_lowercase()))
    }
}
```

### Bypass Mode Implementation

```rust
pub async fn execute_bypass_query(
    llm: &Arc<dyn LLMProvider>,
    query: &str,
    conversation_history: Option<Vec<ConversationMessage>>,
    custom_prompt: Option<String>,
) -> Result<QueryResponse, Error> {
    // Build prompt without RAG context
    let mut prompt = String::new();
    
    // Add conversation history if provided
    if let Some(history) = conversation_history {
        prompt.push_str("Conversation History:\n");
        for msg in history {
            prompt.push_str(&format!("{}: {}\n", msg.role, msg.content));
        }
        prompt.push_str("\n");
    }
    
    // Add custom or default prompt
    if let Some(custom) = custom_prompt {
        prompt.push_str(&custom);
    } else {
        prompt.push_str("Please answer the following question:");
    }
    
    prompt.push_str(&format!("\n\nQuestion: {}\n\nAnswer:", query));
    
    // Generate response directly from LLM
    let answer = llm.generate(&prompt, None).await?;
    
    Ok(QueryResponse {
        answer,
        mode: "bypass".to_string(),
        references: None,  // No RAG retrieval
        stats: QueryStats {
            embedding_time_ms: 0,
            retrieval_time_ms: 0,
            rerank_time_ms: 0,
            generation_time_ms: 0,  // Filled by caller
            total_time_ms: 0,
            sources_retrieved: 0,
            sources_after_rerank: 0,
        },
        extracted_keywords: None,
        token_usage: TokenUsage {
            entity_tokens: 0,
            relation_tokens: 0,
            chunk_tokens: 0,
            prompt_tokens: 0,  // Estimated from prompt length
            total_tokens: 0,
            generation_tokens: 0,  // From LLM response
        },
    })
}
```

---

## Configuration

```toml
[query]
# Default values
default_mode = "hybrid"
default_top_k = 60
default_chunk_top_k = 5

# Token budgets
default_max_entity_tokens = 1000
default_max_relation_tokens = 1000
default_max_total_tokens = 4000

# Reranking
enable_rerank_by_default = true
rerank_top_k = 10

# Conversation history
max_conversation_history_messages = 10
session_timeout_minutes = 60

# Keyword extraction
auto_extract_keywords = true
max_hl_keywords = 5
max_ll_keywords = 10

# Performance
query_timeout_seconds = 30
max_concurrent_queries = 100
```

---

## Testing

```rust
#[tokio::test]
async fn test_token_budget_enforcement() {
    let controller = TokenBudgetController::new(Arc::new(MockTokenizer::new()));
    
    let context = RetrievalContext {
        entities: vec![
            // 10 entities, 100 tokens each = 1000 tokens
            create_entity("E1", 100),
            create_entity("E2", 100),
            // ...
        ],
        relationships: vec![],
        chunks: vec![],
    };
    
    let config = TokenBudgetConfig {
        max_entity_tokens: Some(500),  // Limit to 500 tokens
        max_relation_tokens: None,
        max_total_tokens: None,
    };
    
    let filtered = controller.enforce_budget(context, config);
    
    // Should keep only first 5 entities (500 tokens)
    assert_eq!(filtered.entities.len(), 5);
}

#[tokio::test]
async fn test_conversation_history() {
    let manager = ConversationHistoryManager::new(Arc::new(MockStorage::new()));
    
    let session_id = "session-123";
    
    // Add messages
    manager.add_message(session_id, ConversationMessage {
        role: "user".to_string(),
        content: "What is AI?".to_string(),
    }).await.unwrap();
    
    manager.add_message(session_id, ConversationMessage {
        role: "assistant".to_string(),
        content: "AI is...".to_string(),
    }).await.unwrap();
    
    // Retrieve history
    let history = manager.get_session_history(session_id, 10).await.unwrap();
    
    assert_eq!(history.len(), 2);
    assert_eq!(history[0].role, "user");
    assert_eq!(history[1].role, "assistant");
}

#[tokio::test]
async fn test_bypass_mode() {
    let llm = Arc::new(MockLLM::new());
    
    let response = execute_bypass_query(
        &llm,
        "What is 2+2?",
        None,
        None,
    ).await.unwrap();
    
    assert_eq!(response.mode, "bypass");
    assert!(response.references.is_none());
    assert_eq!(response.stats.sources_retrieved, 0);
}
```

---

**Status:** ✅ Specification Complete  
**Dependencies:** None (extends existing query engine)  
**Next:** Implement token budget controller and conversation history
