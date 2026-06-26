//! Query entry points — sub-module declarations.
//!
//! Splits the query pipeline entry points into thematic groups:
//! - `query_basic`: Core queries using default vector storage
//! - `query_workspace`: Queries with workspace-specific storage/embedding
//! - `query_stream`: All streaming query variants

mod query_basic;
mod query_pipeline;
mod query_stream;
mod query_workspace;
