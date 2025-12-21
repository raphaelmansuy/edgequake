//! Storage adapters.
//!
//! This module provides various storage backend implementations:
//! - `memory`: In-memory storage for development and testing
//! - `postgres`: PostgreSQL with pgvector and Apache AGE extensions

pub mod memory;

#[cfg(feature = "postgres")]
pub mod postgres;
