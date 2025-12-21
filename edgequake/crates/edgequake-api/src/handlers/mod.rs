//! API request handlers.

pub mod documents;
pub mod graph;
pub mod health;
pub mod query;

pub use documents::*;
pub use graph::*;
pub use health::*;
pub use query::*;
