//! Qdrant vector projection adapter for SPEC-149.
//!
//! The qualified single-node image is `qdrant/qdrant:v1.19.0`. Collections
//! are provisioned explicitly and derive solely from immutable binding UUIDs;
//! search and readiness paths never create provider resources.

mod client;
mod errors;
mod mutate;
mod provision;
mod search;

pub use client::{collection_name, QdrantClient};
pub use mutate::{QdrantPointPayload, QdrantVectorPoint};
pub use provision::{
    drop_qdrant_binding, provision_qdrant_binding, verify_qdrant_binding, REQUIRED_PAYLOAD_INDEXES,
};
pub use search::{compile_metadata_filter, CompiledFilter};
