//! Node operations split by ISP surface (SPEC-054 / SOLID).
//!
//! - [`read`] — lookups, degrees, batch get ([`GraphStorageReadOps`])
//! - [`mutate`] — upsert/delete + native ON CONFLICT ([`GraphStorageMutateOps`])

mod mutate;
mod read;
