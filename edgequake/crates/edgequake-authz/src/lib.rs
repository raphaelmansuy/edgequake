//! Document-level ABAC / allow-set PDP (SPEC-146).
//!
//! Identity stays in `edgequake-auth`. This crate owns Cedar (classified only),
//! `AuthzContext`, and the single [`AllowSetProvider`] (LAW-146-18).

pub mod allow_set;
pub mod catalog;
pub mod cedar_schema;
pub mod context;
pub mod decision;
pub mod error;
pub mod postgres;
pub mod principal;
pub mod reason;

pub use allow_set::{
    AllowSet, AllowSetProvider, CachedAllowSetProvider, InMemoryAllowSetProvider,
    SharedAllowSetProvider, ALLOW_SET_ARRAY_THRESHOLD,
};
pub use catalog::{
    normalize_classification, normalize_share_mode, CLASSIFICATIONS, DEFAULT_CLASSIFICATION,
    DEFAULT_SHARE_MODE, SHARE_MODES, ZERO_AUTHZ_ANSWER,
};
pub use cedar_schema::{
    compile_default_policy_set, parse_cedar_policy_text, DEFAULT_CEDAR_SCHEMA,
    DEFAULT_CLASSIFIED_POLICY,
};
pub use context::AuthzContext;
pub use decision::{AuthzDecision, DenyReason};
pub use error::{AuthzError, AuthzResult};
pub use postgres::PostgresAllowSetProvider;
pub use principal::PrincipalId;
pub use reason::DenyReasonCode;
