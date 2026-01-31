//! Progress callback trait for PDF extraction.
//!
//! ## Implements
//!
//! - [`SPEC-001-upload-pdf`]: Page-level progress during PDF conversion
//! - [`FEAT0608`]: Progress callbacks for PDF extraction
//!
//! ## Use Cases
//!
//! - [`UC0710`]: User sees page-by-page progress during PDF extraction
//! - [`UC0711`]: System reports errors for specific pages
//!
//! ## WHY Trait Object Pattern?
//!
//! We use a trait object (`Arc<dyn ProgressCallback>`) instead of closures because:
//! 1. **Multiple callbacks**: Trait has 6 lifecycle methods, not just one
//! 2. **State management**: Implementations can hold counters, channels, mutexes
//! 3. **Testability**: Easy to mock with custom implementations
//! 4. **Ergonomics**: Named methods are clearer than multiple closure params
//!
//! ## Example Usage
//!
//! ```rust,ignore
//! use std::sync::Arc;
//! use edgequake_pdf::{PdfExtractor, ProgressCallback, NoopProgress};
//!
//! // Use NoopProgress for no-op (default)
//! let callback = Arc::new(NoopProgress);
//! extractor.extract_with_progress(pdf_bytes, callback).await?;
//!
//! // Or implement your own
//! struct MyProgress { tx: mpsc::Sender<Event> }
//! impl ProgressCallback for MyProgress {
//!     fn on_page_complete(&self, page: usize, md_len: usize) {
//!         self.tx.send(Event::PageDone { page, md_len });
//!     }
//! }
//! ```

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;

/// Callback trait for PDF extraction progress.
///
/// Implementations must be `Send + Sync` for use across async boundaries.
/// All methods have default no-op implementations, so you only need to
/// override the methods you care about.
///
/// ## Lifecycle Order
///
/// ```text
/// on_extraction_start(total_pages)
/// │
/// ├─► on_page_start(1, total)
/// │   ... extraction work ...
/// │   on_page_complete(1, md_len) OR on_page_error(1, "error")
/// │
/// ├─► on_page_start(2, total)
/// │   ...
/// │   on_page_complete(2, md_len)
/// │
/// └─► on_extraction_complete(total_pages, success_count)
/// ```
///
/// The `on_progress` method can be called at any time for general updates.
pub trait ProgressCallback: Send + Sync {
    /// Called when extraction starts, before any pages are processed.
    ///
    /// # Arguments
    /// * `total_pages` - Total number of pages to extract
    fn on_extraction_start(&self, total_pages: usize) {
        let _ = total_pages;
    }

    /// Called before processing a specific page.
    ///
    /// # Arguments
    /// * `page_num` - 1-indexed page number
    /// * `total_pages` - Total pages in document
    fn on_page_start(&self, page_num: usize, total_pages: usize) {
        let _ = (page_num, total_pages);
    }

    /// Called after a page is successfully extracted.
    ///
    /// # Arguments
    /// * `page_num` - 1-indexed page number
    /// * `markdown_len` - Length of extracted markdown in bytes
    fn on_page_complete(&self, page_num: usize, markdown_len: usize) {
        let _ = (page_num, markdown_len);
    }

    /// Called when a page fails to extract.
    ///
    /// Note: Extraction continues with other pages even after errors.
    ///
    /// # Arguments
    /// * `page_num` - 1-indexed page number
    /// * `error` - Error description
    fn on_page_error(&self, page_num: usize, error: &str) {
        let _ = (page_num, error);
    }

    /// Called when extraction is complete (success or partial).
    ///
    /// # Arguments
    /// * `total_pages` - Total pages in document
    /// * `success_count` - Number of successfully extracted pages
    fn on_extraction_complete(&self, total_pages: usize, success_count: usize) {
        let _ = (total_pages, success_count);
    }

    /// Called for general progress updates.
    ///
    /// This can be called at any point during extraction for fine-grained updates.
    ///
    /// # Arguments
    /// * `phase` - Current phase name (e.g., "parsing", "rendering")
    /// * `percent` - Progress percentage (0.0 - 100.0)
    fn on_progress(&self, phase: &str, percent: f32) {
        let _ = (phase, percent);
    }
}

/// No-op progress callback that does nothing.
///
/// Use this as the default when progress tracking is not needed.
/// All method implementations are empty (inlined away by optimizer).
///
/// # Example
///
/// ```rust,ignore
/// let callback = Arc::new(NoopProgress);
/// extractor.extract_with_progress(pdf_bytes, callback).await?;
/// ```
pub struct NoopProgress;

impl ProgressCallback for NoopProgress {}

/// Progress callback that logs to tracing.
///
/// Useful for debugging extraction progress.
/// Uses `tracing::info!` for page events and `tracing::warn!` for errors.
///
/// # Example
///
/// ```rust,ignore
/// let callback = Arc::new(LoggingProgress::new("pdf-123"));
/// extractor.extract_with_progress(pdf_bytes, callback).await?;
/// // Logs: "pdf-123: Starting extraction of 10 pages"
/// // Logs: "pdf-123: Page 1/10 complete (4523 bytes)"
/// ```
pub struct LoggingProgress {
    /// Identifier for logging (e.g., pdf_id or filename)
    id: String,
}

impl LoggingProgress {
    /// Create a new logging progress callback.
    pub fn new(id: impl Into<String>) -> Self {
        Self { id: id.into() }
    }
}

impl ProgressCallback for LoggingProgress {
    fn on_extraction_start(&self, total_pages: usize) {
        tracing::info!(
            id = %self.id,
            total_pages,
            "Starting extraction"
        );
    }

    fn on_page_start(&self, page_num: usize, total_pages: usize) {
        tracing::debug!(
            id = %self.id,
            page = page_num,
            total = total_pages,
            "Processing page"
        );
    }

    fn on_page_complete(&self, page_num: usize, markdown_len: usize) {
        tracing::info!(
            id = %self.id,
            page = page_num,
            bytes = markdown_len,
            "Page complete"
        );
    }

    fn on_page_error(&self, page_num: usize, error: &str) {
        tracing::warn!(
            id = %self.id,
            page = page_num,
            error,
            "Page extraction failed"
        );
    }

    fn on_extraction_complete(&self, total_pages: usize, success_count: usize) {
        tracing::info!(
            id = %self.id,
            total = total_pages,
            success = success_count,
            failed = total_pages - success_count,
            "Extraction complete"
        );
    }

    fn on_progress(&self, phase: &str, percent: f32) {
        tracing::debug!(
            id = %self.id,
            phase,
            percent = format!("{:.1}%", percent),
            "Progress update"
        );
    }
}

/// Progress callback that counts events for testing.
///
/// Thread-safe counters for each callback method.
/// Use `.get_*()` methods to retrieve counts after extraction.
///
/// # Example
///
/// ```rust
/// use edgequake_pdf::{CountingProgress, ProgressCallback};
///
/// let callback = CountingProgress::new();
/// // Initially all counts are zero
/// assert_eq!(callback.pages_completed(), 0);
/// assert_eq!(callback.pages_failed(), 0);
///
/// // After calling callbacks, counts increment
/// callback.on_page_complete(1, 100);
/// callback.on_page_complete(2, 200);
/// assert_eq!(callback.pages_completed(), 2);
/// ```
pub struct CountingProgress {
    extraction_started: AtomicUsize,
    pages_started: AtomicUsize,
    pages_completed: AtomicUsize,
    pages_failed: AtomicUsize,
    extraction_completed: AtomicUsize,
    progress_calls: AtomicUsize,
    /// Last progress percentage seen
    last_percent: Mutex<f32>,
}

impl CountingProgress {
    /// Create a new counting progress callback.
    pub fn new() -> Self {
        Self {
            extraction_started: AtomicUsize::new(0),
            pages_started: AtomicUsize::new(0),
            pages_completed: AtomicUsize::new(0),
            pages_failed: AtomicUsize::new(0),
            extraction_completed: AtomicUsize::new(0),
            progress_calls: AtomicUsize::new(0),
            last_percent: Mutex::new(0.0),
        }
    }

    /// Get count of extraction starts (should be 1).
    pub fn extraction_started(&self) -> usize {
        self.extraction_started.load(Ordering::Relaxed)
    }

    /// Get count of pages started.
    pub fn pages_started(&self) -> usize {
        self.pages_started.load(Ordering::Relaxed)
    }

    /// Get count of pages completed successfully.
    pub fn pages_completed(&self) -> usize {
        self.pages_completed.load(Ordering::Relaxed)
    }

    /// Get count of pages that failed.
    pub fn pages_failed(&self) -> usize {
        self.pages_failed.load(Ordering::Relaxed)
    }

    /// Get count of extraction completions (should be 1).
    pub fn extraction_completed(&self) -> usize {
        self.extraction_completed.load(Ordering::Relaxed)
    }

    /// Get count of progress update calls.
    pub fn progress_calls(&self) -> usize {
        self.progress_calls.load(Ordering::Relaxed)
    }

    /// Get last progress percentage seen.
    pub fn last_percent(&self) -> f32 {
        *self.last_percent.lock().unwrap()
    }
}

impl Default for CountingProgress {
    fn default() -> Self {
        Self::new()
    }
}

impl ProgressCallback for CountingProgress {
    fn on_extraction_start(&self, _total_pages: usize) {
        self.extraction_started.fetch_add(1, Ordering::Relaxed);
    }

    fn on_page_start(&self, _page_num: usize, _total_pages: usize) {
        self.pages_started.fetch_add(1, Ordering::Relaxed);
    }

    fn on_page_complete(&self, _page_num: usize, _markdown_len: usize) {
        self.pages_completed.fetch_add(1, Ordering::Relaxed);
    }

    fn on_page_error(&self, _page_num: usize, _error: &str) {
        self.pages_failed.fetch_add(1, Ordering::Relaxed);
    }

    fn on_extraction_complete(&self, _total_pages: usize, _success_count: usize) {
        self.extraction_completed.fetch_add(1, Ordering::Relaxed);
    }

    fn on_progress(&self, _phase: &str, percent: f32) {
        self.progress_calls.fetch_add(1, Ordering::Relaxed);
        *self.last_percent.lock().unwrap() = percent;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    #[test]
    fn test_noop_progress_does_nothing() {
        // Verify NoopProgress can be called without panic
        let callback = NoopProgress;
        callback.on_extraction_start(10);
        callback.on_page_start(1, 10);
        callback.on_page_complete(1, 500);
        callback.on_page_error(2, "test error");
        callback.on_extraction_complete(10, 9);
        callback.on_progress("test", 50.0);
        // No assertions needed - just verify no panic
    }

    #[test]
    fn test_noop_progress_is_send_sync() {
        // Verify Send + Sync bounds are satisfied
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<NoopProgress>();
    }

    #[test]
    fn test_counting_progress_records_calls() {
        let callback = CountingProgress::new();

        callback.on_extraction_start(5);
        assert_eq!(callback.extraction_started(), 1);

        callback.on_page_start(1, 5);
        callback.on_page_start(2, 5);
        assert_eq!(callback.pages_started(), 2);

        callback.on_page_complete(1, 100);
        callback.on_page_complete(2, 200);
        callback.on_page_complete(3, 300);
        assert_eq!(callback.pages_completed(), 3);

        callback.on_page_error(4, "corrupt page");
        assert_eq!(callback.pages_failed(), 1);

        callback.on_extraction_complete(5, 3);
        assert_eq!(callback.extraction_completed(), 1);

        callback.on_progress("test", 75.5);
        assert_eq!(callback.progress_calls(), 1);
        assert!((callback.last_percent() - 75.5).abs() < 0.001);
    }

    #[test]
    fn test_counting_progress_is_thread_safe() {
        let callback = Arc::new(CountingProgress::new());

        // Simulate concurrent calls
        let handles: Vec<_> = (0..10)
            .map(|i| {
                let cb = Arc::clone(&callback);
                std::thread::spawn(move || {
                    cb.on_page_start(i, 10);
                    cb.on_page_complete(i, i * 100);
                })
            })
            .collect();

        for handle in handles {
            handle.join().unwrap();
        }

        assert_eq!(callback.pages_started(), 10);
        assert_eq!(callback.pages_completed(), 10);
    }

    #[test]
    fn test_logging_progress_creation() {
        let callback = LoggingProgress::new("test-pdf-123");
        // Just verify creation works
        callback.on_extraction_start(5);
        callback.on_page_complete(1, 100);
        // Logging goes to tracing subscriber (may be no-op in tests)
    }

    #[test]
    fn test_trait_object_usage() {
        // Verify trait can be used as Arc<dyn ProgressCallback>
        let callback: Arc<dyn ProgressCallback> = Arc::new(CountingProgress::new());
        callback.on_extraction_start(5);
        callback.on_page_complete(1, 100);
    }
}
