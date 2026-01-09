//! Storage adapters.
//!
//! This module provides various storage backend implementations:
//! - `memory`: In-memory storage for development and testing
//! - `postgres`: PostgreSQL with pgvector and Apache AGE extensions
//!
//! ## Implements
//!
//! - [`FEAT0201`]: In-memory storage adapter
//! - [`FEAT0202`]: PostgreSQL with pgvector adapter
//! - [`FEAT0203`]: Apache AGE graph storage
//!
//! ## Use Cases
//!
//! - [`UC0601`]: System stores documents and chunks
//! - [`UC0602`]: System stores entities and relationships
//! - [`UC0603`]: System performs vector similarity search
//!
//! ## Enforces
//!
//! - [`BR0201`]: Memory adapter for testing isolation
//! - [`BR0202`]: PostgreSQL adapter for production persistence

pub mod memory;

#[cfg(feature = "postgres")]
pub mod postgres;
