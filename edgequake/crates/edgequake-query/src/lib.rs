//! EdgeQuake Query - SOTA Query Engine for RAG
//!
//! # Implements
//!
//! - **FEAT0007**: Multi-Mode Query Execution
//! - **FEAT0101-0106**: All query mode strategies
//! - **FEAT0107**: LLM-Based Keyword Extraction
//! - **FEAT0108**: Smart Context Truncation
//!
//! # Enforces
//!
//! - **BR0101**: Token budget enforcement (configurable, default 4000)
//! - **BR0102**: Graph context priority over naive chunks
//! - **BR0104**: Conversation history in context
//!
//! This crate provides the query engine that combines:
//! - Vector similarity search
//! - Knowledge graph traversal
//! - LLM-based answer generation
//!
//! # Query Modes
//!
//! | Mode | FEAT | Description |
//! |------|------|-------------|
//! | Naive | FEAT0101 | Simple vector similarity search |
//! | Local | FEAT0102 | Entity-centric search with graph context |
//! | Global | FEAT0103 | Community-based search (relationship focus) |
//! | Hybrid | FEAT0104 | Combines local and global approaches |
//! | Mix | FEAT0105 | Weighted combination of naive + graph |
//! | Bypass | FEAT0106 | Direct LLM, no RAG retrieval |
//!
//! # Architecture
//!
//! The query engine uses a multi-stage retrieval pipeline:
//! 1. Query embedding generation
//! 2. Keyword extraction (FEAT0107)
//! 3. Candidate retrieval (vector + graph)
//! 4. Context aggregation + truncation (FEAT0108)
//! 5. LLM answer generation
//!
//! # Key Components
//!
//! - [`SOTAQueryEngine`]: Main engine implementing LightRAG algorithm
//! - [`QueryMode`]: Enum of all supported query modes
//! - [`QueryContext`]: Retrieved context (entities, relationships, chunks)
//! - [`TruncationConfig`]: Token budget configuration
//!
//! # See Also
//!
//! - [`crate::sota_engine`] for the SOTA implementation
//! - [`crate::keywords`] for keyword extraction
//! - [`crate::truncation`] for token budgeting

pub mod chunk_retrieval;
pub mod context;
pub mod engine;
pub mod error;
pub mod helpers;
pub mod keywords;
pub mod modes;
pub mod sota_engine;
pub mod strategies;
pub mod tokenizer;
pub mod truncation;
pub mod vector_filter;

pub use chunk_retrieval::{
    merge_chunks, retrieve_chunks_from_entities, retrieve_chunks_from_relationships,
    ChunkSelectionMethod,
};
pub use context::{QueryContext, RetrievedContext};
pub use engine::{
    ConversationMessage, QueryEngine, QueryEngineConfig, QueryRequest, QueryResponse,
};
pub use error::{QueryError, Result};
// Re-export keywords module types
#[cfg(feature = "postgres")]
pub use keywords::PostgresKeywordCache;
pub use keywords::{
    CachedKeywordExtractor, ExtractedKeywords, InMemoryKeywordCache, KeywordCache,
    KeywordExtractor, Keywords, LLMKeywordExtractor, MockKeywordExtractor, QueryIntent,
};
pub use modes::QueryMode;
pub use sota_engine::{QueryEmbeddings, SOTAQueryConfig, SOTAQueryEngine};
pub use strategies::{
    create_strategy, GlobalStrategy, HybridStrategy, LocalStrategy, MixStrategy, NaiveStrategy,
    QueryStrategy, StrategyConfig,
};
pub use tokenizer::{MockTokenizer, SimpleTokenizer, Tokenizer};
pub use truncation::{
    balance_context, truncate_chunks, truncate_entities, truncate_relationships, TruncationConfig,
};
pub use vector_filter::{filter_by_type, get_typed_vectors, VectorType};

// Re-export EmbeddingProvider and LLMProvider for workspace-specific query execution
pub use edgequake_llm::traits::EmbeddingProvider;
pub use edgequake_llm::traits::LLMProvider;
