//! Document deletion handlers.
//!
//! | Sub-module | Responsibility                                    |
//! |------------|---------------------------------------------------|
//! | `single`   | Delete a single document by ID (cascade cleanup)  |
//! | `batch`    | Selected multi-document delete (SPEC-084 / GH-317)|
//! | `bulk`     | Delete all documents (bulk clear with skip logic)  |
//! | `impact`   | Read-only deletion impact preview                 |

mod batch;
mod bulk;
mod impact;
mod single;

pub use batch::*;
pub use bulk::*;
pub use impact::*;
pub use single::*;
