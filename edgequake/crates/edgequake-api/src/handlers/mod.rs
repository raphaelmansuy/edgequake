//! API request handlers.

pub mod documents;
pub mod entities;
pub mod graph;
pub mod health;
pub mod query;
pub mod relationships;
pub mod tasks;

pub use documents::*;
pub use entities::*;
pub use graph::*;
pub use health::*;
pub use query::*;
pub use relationships::*;
pub use tasks::*;
