//! API request handlers.

pub mod auth;
pub mod documents;
pub mod entities;
pub mod graph;
pub mod health;
pub mod metrics;
pub mod pipeline;
pub mod query;
pub mod relationships;
pub mod tasks;

pub use auth::*;
pub use documents::*;
pub use entities::*;
pub use graph::*;
pub use health::*;
pub use metrics::*;
pub use pipeline::*;
pub use query::*;
pub use relationships::*;
pub use tasks::*;
