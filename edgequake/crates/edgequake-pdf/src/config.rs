//! Configuration for PDF extraction.

use serde::{Deserialize, Serialize};

/// Extraction mode for PDF processing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum ExtractionMode {
    /// Fast text-based extraction using pdf_oxide.
    #[default]
    Text,

    /// Vision-based extraction using multimodal LLM.
    Vision,

    /// Hybrid mode: use text extraction, fall back to vision for low quality.
    Hybrid,
}

/// Output format for extraction results.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum OutputFormat {
    /// Markdown format.
    #[default]
    Markdown,

    /// JSON format with full block structure.
    Json,

    /// HTML format.
    Html,

    /// Chunked format for RAG pipelines.
    Chunks,
}

/// Configuration for layout detection.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LayoutConfig {
    /// Enable column detection.
    pub detect_columns: bool,

    /// Enable table detection.
    pub detect_tables: bool,

    /// Enable equation detection.
    pub detect_equations: bool,

    /// Minimum gap for column separation (in points).
    pub column_gap_threshold: f32,

    /// Use XY-cut algorithm for layout analysis.
    pub use_xy_cut: bool,
}

impl Default for LayoutConfig {
    fn default() -> Self {
        Self {
            detect_columns: true,
            detect_tables: true,
            detect_equations: true,
            column_gap_threshold: 20.0,
            use_xy_cut: true,
        }
    }
}

/// Configuration for PDF extraction operations.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PdfConfig {
    /// Extraction mode (text, vision, hybrid).
    #[serde(default)]
    pub mode: ExtractionMode,

    /// Output format.
    #[serde(default)]
    pub output_format: OutputFormat,

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

    /// Whether to normalize word spacing (fix concatenated words).
    pub normalize_spacing: bool,

    /// Whether to consolidate broken headers into single lines.
    pub consolidate_headers: bool,

    /// Whether to extract and format figure captions.
    pub extract_figure_captions: bool,

    /// Whether to use AI for full page readability enhancement.
    pub enhance_readability: bool,

    /// Layout detection configuration.
    #[serde(default)]
    pub layout: LayoutConfig,

    /// DPI for vision mode rendering.
    pub vision_dpi: u32,

    /// Quality threshold for hybrid mode (below this, switch to vision).
    pub quality_threshold: f32,
}

impl Default for PdfConfig {
    fn default() -> Self {
        Self {
            mode: ExtractionMode::Text,
            output_format: OutputFormat::Markdown,
            ocr_threshold: 0.8,
            max_pages: None,
            include_page_numbers: true,
            extract_images: true,
            enhance_tables: true,
            ai_temperature: 0.1,
            normalize_spacing: true,
            consolidate_headers: true,
            extract_figure_captions: true,
            enhance_readability: false,
            layout: LayoutConfig::default(),
            vision_dpi: 150,
            quality_threshold: 0.5,
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

    /// Set whether to normalize word spacing.
    pub fn with_spacing_normalization(mut self, normalize: bool) -> Self {
        self.normalize_spacing = normalize;
        self
    }

    /// Set whether to consolidate broken headers.
    pub fn with_header_consolidation(mut self, consolidate: bool) -> Self {
        self.consolidate_headers = consolidate;
        self
    }

    /// Set whether to extract figure captions.
    pub fn with_figure_captions(mut self, extract: bool) -> Self {
        self.extract_figure_captions = extract;
        self
    }

    /// Set whether to enhance readability with AI.
    pub fn with_readability_enhancement(mut self, enhance: bool) -> Self {
        self.enhance_readability = enhance;
        self
    }

    /// Set the extraction mode.
    pub fn with_mode(mut self, mode: ExtractionMode) -> Self {
        self.mode = mode;
        self
    }

    /// Set the output format.
    pub fn with_output_format(mut self, format: OutputFormat) -> Self {
        self.output_format = format;
        self
    }

    /// Set the layout configuration.
    pub fn with_layout(mut self, layout: LayoutConfig) -> Self {
        self.layout = layout;
        self
    }

    /// Set the vision DPI.
    pub fn with_vision_dpi(mut self, dpi: u32) -> Self {
        self.vision_dpi = dpi;
        self
    }

    /// Set the quality threshold for hybrid mode.
    pub fn with_quality_threshold(mut self, threshold: f32) -> Self {
        self.quality_threshold = threshold;
        self
    }

    /// Enable vision mode.
    pub fn with_vision_mode(mut self) -> Self {
        self.mode = ExtractionMode::Vision;
        self
    }

    /// Enable hybrid mode.
    pub fn with_hybrid_mode(mut self) -> Self {
        self.mode = ExtractionMode::Hybrid;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_defaults() {
        let config = PdfConfig::default();
        assert_eq!(config.mode, ExtractionMode::Text);
        assert_eq!(config.output_format, OutputFormat::Markdown);
        assert_eq!(config.ocr_threshold, 0.8);
        assert!(config.include_page_numbers);
    }

    #[test]
    fn test_extraction_mode_serialization() {
        let mode = ExtractionMode::Vision;
        let json = serde_json::to_string(&mode).unwrap();
        assert_eq!(json, "\"Vision\"");

        let parsed: ExtractionMode = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, mode);
    }

    #[test]
    fn test_output_format_serialization() {
        let format = OutputFormat::Json;
        let json = serde_json::to_string(&format).unwrap();
        assert_eq!(json, "\"Json\"");
    }

    #[test]
    fn test_layout_config_defaults() {
        let layout = LayoutConfig::default();
        assert!(layout.detect_columns);
        assert!(layout.detect_tables);
        assert!(layout.use_xy_cut);
    }

    #[test]
    fn test_config_builder() {
        let config = PdfConfig::new()
            .with_mode(ExtractionMode::Hybrid)
            .with_output_format(OutputFormat::Json)
            .with_max_pages(10)
            .with_vision_dpi(300);

        assert_eq!(config.mode, ExtractionMode::Hybrid);
        assert_eq!(config.output_format, OutputFormat::Json);
        assert_eq!(config.max_pages, Some(10));
        assert_eq!(config.vision_dpi, 300);
    }

    #[test]
    fn test_config_serialization() {
        let config = PdfConfig::default();
        let json = serde_json::to_string(&config).unwrap();
        assert!(json.contains("\"mode\":\"Text\""));
        assert!(json.contains("\"output_format\":\"Markdown\""));

        let parsed: PdfConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.mode, config.mode);
    }

    // Additional config tests for Phase 4.1

    #[test]
    fn test_all_extraction_modes() {
        assert_eq!(ExtractionMode::default(), ExtractionMode::Text);

        let modes = vec![ExtractionMode::Text, ExtractionMode::Vision, ExtractionMode::Hybrid];
        for mode in modes {
            let json = serde_json::to_string(&mode).unwrap();
            let parsed: ExtractionMode = serde_json::from_str(&json).unwrap();
            assert_eq!(parsed, mode);
        }
    }

    #[test]
    fn test_all_output_formats() {
        assert_eq!(OutputFormat::default(), OutputFormat::Markdown);

        let formats = vec![
            OutputFormat::Markdown,
            OutputFormat::Json,
            OutputFormat::Html,
            OutputFormat::Chunks,
        ];
        for fmt in formats {
            let json = serde_json::to_string(&fmt).unwrap();
            let parsed: OutputFormat = serde_json::from_str(&json).unwrap();
            assert_eq!(parsed, fmt);
        }
    }

    #[test]
    fn test_layout_config_builder() {
        let layout = LayoutConfig {
            detect_columns: false,
            detect_tables: true,
            detect_equations: false,
            column_gap_threshold: 30.0,
            use_xy_cut: false,
        };
        assert!(!layout.detect_columns);
        assert!(layout.detect_tables);
        assert_eq!(layout.column_gap_threshold, 30.0);
    }

    #[test]
    fn test_config_debug_display() {
        let config = PdfConfig::default();
        // Ensure Debug is implemented
        let debug_str = format!("{:?}", config);
        assert!(debug_str.contains("PdfConfig"));
        assert!(debug_str.contains("mode"));
    }

    #[test]
    fn test_config_no_max_pages() {
        let config = PdfConfig::new();
        assert!(config.max_pages.is_none());
    }

    #[test]
    fn test_config_with_all_options() {
        let config = PdfConfig::new()
            .with_mode(ExtractionMode::Vision)
            .with_output_format(OutputFormat::Html)
            .with_max_pages(100);

        assert_eq!(config.mode, ExtractionMode::Vision);
        assert_eq!(config.output_format, OutputFormat::Html);
        assert_eq!(config.max_pages, Some(100));
    }
}
