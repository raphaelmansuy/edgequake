//! EdgeQuake Query - Query Engine for RAG
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
//! - [`QueryEngine`]: Main engine implementing LightRAG algorithm
//! - [`QueryMode`]: Enum of all supported query modes
//! - [`QueryContext`]: Retrieved context (entities, relationships, chunks)
//! - [`TruncationConfig`]: Token budget configuration
//!
//! # See Also
//!
//! - [`crate::engine_impl`] for the engine implementation
//! - [`crate::keywords`] for keyword extraction
//! - [`crate::truncation`] for token budgeting

pub mod bootstrap;
pub mod cache;
pub mod chunk_hydration;
pub mod community_global;
pub mod context;
pub mod context_filter;
pub mod conversation_context;
pub mod engine;
pub mod engine_impl;
pub mod error;
pub mod eval;
pub mod fusion;
pub mod graph_hops;
pub mod helpers;
pub mod hybrid_merge;
pub mod keywords;
pub mod mix_weights;
pub mod modes;
pub mod sparse_retrieval;
pub mod tokenizer;
pub mod truncation;
pub mod types;
pub mod vector_filter;

pub use context::{QueryContext, RetrievedContext};
pub use engine::{ConversationMessage, QueryRequest, QueryResponse, QueryStats};
pub use error::{QueryError, Result};
// Re-export keywords module types
pub use bootstrap::{
    build_production_query_engine, create_production_reranker,
    create_production_reranker_with_embedding,
};
pub use cache::{QueryResultCache, QueryResultCacheInvalidator};
pub use engine_impl::{QueryEmbeddings, QueryEngine, QueryEngineConfig};
#[cfg(feature = "postgres")]
pub use keywords::PostgresKeywordCache;
pub use keywords::{
    CachedKeywordExtractor, ExtractedKeywords, InMemoryKeywordCache, KeywordCache,
    KeywordExtractor, Keywords, LLMKeywordExtractor, MockKeywordExtractor, QueryIntent,
};
pub use mix_weights::MixWeightOverride;
pub use modes::QueryMode;
pub use tokenizer::{MockTokenizer, SimpleTokenizer, Tokenizer};
pub use truncation::{
    balance_context, truncate_chunks, truncate_entities, truncate_relationships, TruncationConfig,
};

// Re-export EmbeddingProvider and LLMProvider for workspace-specific query execution
pub use edgequake_llm::traits::EmbeddingProvider;
pub use edgequake_llm::traits::LLMProvider;
