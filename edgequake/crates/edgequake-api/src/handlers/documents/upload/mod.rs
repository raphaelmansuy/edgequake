//! Document upload handlers.

pub mod batch_upload;
pub mod document_admission;
pub mod file_upload;
pub mod image_extract;
pub mod text_upload;

pub use batch_upload::*;
pub use document_admission::*;
pub use file_upload::*;
pub use text_upload::*;
