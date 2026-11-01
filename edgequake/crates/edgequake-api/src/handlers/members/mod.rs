//! Organization members directory and lifecycle (SPEC-169).
//!
//! ```text
//!   list · update · invitations · accept     HTTP handlers (thin orchestration)
//!        │
//!   directory                                I/O: memberships × users × IdP links
//!        │
//!   view · guards · types                    pure projection, rules and wire DTOs
//! ```

pub mod accept;
pub mod directory;
pub mod grants;
pub mod guards;
pub mod invitations;
pub mod list;
pub mod types;
pub mod update;
pub mod view;

pub use accept::{accept_invitation, preview_invitation};
pub use grants::{create_grant, delete_grant, list_grants, update_grant};
pub use invitations::{create_invitation, resend_invitation, revoke_invitation};
pub use list::list_members;
pub use update::{remove_member, update_member};
