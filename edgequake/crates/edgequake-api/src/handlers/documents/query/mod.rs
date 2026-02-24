//! Document query handlers — split by SRP.

pub mod detail;
pub mod list;
pub mod scan;
pub mod track_status;

pub use detail::*;
pub use list::*;
pub use scan::*;
pub use track_status::*;
