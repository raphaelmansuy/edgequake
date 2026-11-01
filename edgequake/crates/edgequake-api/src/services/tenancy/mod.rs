//! Tenancy services (SPEC-181): creation, slugs, quota, purge.
//!
//! One module per responsibility so the handlers stay thin and the policy stays unit-testable
//! (SRP / DIP in the SPEC-181 ledger). Nothing here opens a transaction directly: the store it
//! calls owns the atomicity, and the database triggers in migration 226 own the invariants.

pub mod create;
pub mod purge;
pub mod purge_exec;
pub mod slug;
