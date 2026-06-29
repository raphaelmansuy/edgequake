//! Document query handlers — split by SRP.

pub mod detail;
pub mod list;
pub mod scan;
pub mod search;
pub mod track_status;

pub use detail::*;
pub use list::*;
pub use scan::*;
pub use search::*;
pub use track_status::*;
