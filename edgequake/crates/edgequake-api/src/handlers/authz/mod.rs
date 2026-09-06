//! SPEC-146 M1b: workspace authz PAP REST surfaces (roles, members, attrs, policies, ACL, break-glass).

mod attributes;
mod bindings;
mod break_glass;
mod document_acl;
mod document_labels;
mod helpers;
mod policies;
mod roles;
mod types;

pub use attributes::*;
pub use bindings::*;
pub use break_glass::*;
pub use document_acl::*;
pub use document_labels::*;
pub use policies::*;
pub use roles::*;
pub use types::*;
