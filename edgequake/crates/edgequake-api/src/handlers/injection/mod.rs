//! Knowledge injection handlers — PUT, GET, LIST, DELETE.
//!
//! @implements SPEC-0002 (Knowledge Injection for Enhanced Search)

pub mod crud;
mod helpers;
pub mod injection_file;

pub use super::injection_types::*;
pub use crud::{delete_injection, get_injection, list_injections, put_injection, update_injection};
pub use injection_file::put_injection_file;
