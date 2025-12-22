//! EdgeQuake Query - Query Engine for RAG
//!
//! This crate provides the query engine that combines:
//! - Vector similarity search
//! - Knowledge graph traversal
//! - LLM-based answer generation
//!
//! # Query Modes
//!
//! - **Naive**: Simple vector similarity search
//! - **Local**: Entity-centric search with local graph context
//! - **Global**: Community-based search using graph structure
//! - **Hybrid**: Combines local and global approaches
//! - **Mix**: Weighted combination of naive and graph-based search
//!
//! # Architecture
//!
//! The query engine uses a multi-stage retrieval pipeline:
//! 1. Query embedding generation
//! 2. Candidate retrieval (vector + graph)
//! 3. Context aggregation
//! 4. LLM answer generation

pub mod chunk_retrieval;
pub mod context;
pub mod engine;
pub mod error;
pub mod keywords;
pub mod modes;
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
pub use keywords::{KeywordExtractor, Keywords, LLMKeywordExtractor, MockKeywordExtractor};
pub use modes::QueryMode;
pub use strategies::{
    create_strategy, GlobalStrategy, HybridStrategy, LocalStrategy, MixStrategy, NaiveStrategy,
    QueryStrategy, StrategyConfig,
};
pub use tokenizer::{MockTokenizer, SimpleTokenizer, Tokenizer};
pub use truncation::{
    balance_context, truncate_chunks, truncate_entities, truncate_relationships, TruncationConfig,
};
pub use vector_filter::{filter_by_type, get_typed_vectors, VectorType};
