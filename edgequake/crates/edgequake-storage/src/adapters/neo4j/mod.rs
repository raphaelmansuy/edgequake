//! Neo4j single-node graph projection adapter (SPEC-149 J19-J20).
//!
//! The adapter uses Query API v2 over HTTP, immutable application UUIDs, and
//! an edge-as-node layout with fixed `FROM` / `TO` structural relationships.

mod client;
mod errors;
mod mapping;
#[cfg(feature = "postgres")]
mod migration;
mod mutate;
mod provision;
mod read;

pub use client::{Neo4jClient, Neo4jConfig};
pub use mapping::{EdgeRevision, EntityRevision, GraphObjectPayload};
#[cfg(feature = "postgres")]
pub use migration::{
    AgeRollbackReadiness, GraphDigestMismatch, GraphDigestReport, GraphReplayReport,
    PgNeo4jGraphMigration,
};
pub use mutate::Neo4jMutationReceipt;
pub use provision::Neo4jProvisionReport;
pub use read::{Neo4jTraversalRequest, Neo4jTraversalResult, RevisionDigest};
