//! Services module for shared business logic.
//!
//! WHY-OODA83: Extracted services follow SRP and DRY principles.
//! Consolidates repeated logic into single, testable modules.

pub mod content_hasher;

pub use content_hasher::ContentHasher;
