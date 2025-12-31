//! Configuration for PDF extraction.

/// Configuration for PDF extraction operations.
#[derive(Clone)]
pub struct PdfConfig {
    /// OCR confidence threshold (0.0-1.0). Below this, AI enhancement is triggered.
    pub ocr_threshold: f32,

    /// Maximum number of pages to process. None means process all.
    pub max_pages: Option<usize>,

    /// Whether to include page numbers in output.
    pub include_page_numbers: bool,

    /// Whether to extract and describe images.
    pub extract_images: bool,

    /// Whether to use AI for table refinement.
    pub enhance_tables: bool,

    /// Temperature for AI calls (0.0 = deterministic, 1.0 = creative).
    pub ai_temperature: f32,
}

impl Default for PdfConfig {
    fn default() -> Self {
        Self {
            ocr_threshold: 0.8,
            max_pages: None,
            include_page_numbers: true,
            extract_images: true,
            enhance_tables: true,
            ai_temperature: 0.1,
        }
    }
}

impl PdfConfig {
    /// Create a new configuration with default values.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the OCR confidence threshold.
    pub fn with_ocr_threshold(mut self, threshold: f32) -> Self {
        self.ocr_threshold = threshold;
        self
    }

    /// Set the maximum number of pages to process.
    pub fn with_max_pages(mut self, max_pages: usize) -> Self {
        self.max_pages = Some(max_pages);
        self
    }

    /// Set whether to include page numbers.
    pub fn with_page_numbers(mut self, include: bool) -> Self {
        self.include_page_numbers = include;
        self
    }

    /// Set whether to extract images.
    pub fn with_image_extraction(mut self, extract: bool) -> Self {
        self.extract_images = extract;
        self
    }

    /// Set whether to enhance tables with AI.
    pub fn with_table_enhancement(mut self, enhance: bool) -> Self {
        self.enhance_tables = enhance;
        self
    }

    /// Set the AI temperature.
    pub fn with_ai_temperature(mut self, temperature: f32) -> Self {
        self.ai_temperature = temperature;
        self
    }
}