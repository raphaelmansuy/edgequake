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

pub mod context;
pub mod engine;
pub mod error;
pub mod modes;
pub mod strategies;

pub use context::{QueryContext, RetrievedContext};
pub use engine::{QueryEngine, QueryEngineConfig, QueryRequest, QueryResponse};
pub use error::{QueryError, Result};
pub use modes::QueryMode;
pub use strategies::{
    create_strategy, GlobalStrategy, HybridStrategy, LocalStrategy, MixStrategy, NaiveStrategy,
    QueryStrategy, StrategyConfig,
};
