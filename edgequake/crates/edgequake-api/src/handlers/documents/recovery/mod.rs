//! Document recovery handlers.
//!
//! | Sub-module   | Responsibility                                     |
//! |--------------|----------------------------------------------------|
//! | `reprocess`  | Reprocess failed/cancelled documents (GAP-039)     |
//! | `stuck`      | Recover documents stuck in "processing" status     |
//! | `chunks`     | Retry/list failed chunks (FEAT0408, FEAT0409)      |

mod chunks;
mod enrich;
mod reprocess;
mod stuck;

pub use chunks::*;
pub use enrich::*;
pub use reprocess::*;
pub use stuck::*;
