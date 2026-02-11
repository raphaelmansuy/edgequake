#![allow(dead_code)]

//! # EdgeQuake Rust SDK
//!
//! A production-ready Rust client for the EdgeQuake RAG API.
//!
//! ## Quick start
//!
//! ```rust,no_run
//! use edgequake_sdk::EdgeQuakeClient;
//!
//! #[tokio::main]
//! async fn main() -> edgequake_sdk::Result<()> {
//!     let client = EdgeQuakeClient::builder()
//!         .base_url("http://localhost:8080")
//!         .api_key("my-api-key")
//!         .build()?;
//!
//!     let health = client.health().check().await?;
//!     println!("status: {}", health.status);
//!     Ok(())
//! }
//! ```

pub mod client;
pub mod config;
pub mod error;
pub mod resources;
pub mod types;

// Re-exports for ergonomic usage.
pub use client::{ClientBuilder, EdgeQuakeClient};
pub use config::{Auth, ClientConfig, TenantContext};
pub use error::{Error, Result};
