//! Query engine module — re-exports the shared query protocol types.
//!
//! # WHY THIS IS NOW A SHIM (P-G6a / RC-11)
//!
//! Historically this module defined both the query *protocol* (request /
//! response / stats) and a legacy `QueryEngine` struct. The legacy struct was
//! dead code: production routes exclusively through `QueryEngine`
//! (`crate::engine_impl`), and no handler reads the legacy engine. The
//! protocol types, however, are the contract between the API layer and any
//! engine implementation, so they were hoisted into [`crate::types`].
//!
//! This file remains as a thin re-export so existing `use crate::engine::*`
//! and `use edgequake_query::engine::*` paths keep resolving during the
//! rename transition (P-G6b). New code should import from [`crate::types`]
//! directly.
//!
//! The legacy `QueryEngine` struct, its config-consuming impl, and the
//! `strategies/` module that only it used have been deleted alongside this
//! shim. There is now exactly one query engine implementation in the crate.

pub use crate::types::{ConversationMessage, QueryRequest, QueryResponse, QueryStats};
