//! Configuration for PDF extraction.
//!
//! ## Implements
//!
//! - **FEAT1020**: Configurable extraction modes
//! - **FEAT1021**: Output format selection
//! - **FEAT1022**: Image processing options
//!
//! ## Enforces
//!
//! - **BR1020**: Default to text mode for speed
//! - **BR1021**: Validate configuration parameters

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

/// Configuration for LLM-based image OCR.
///
/// When enabled, embedded images and figures in the PDF are sent to a
/// multimodal LLM (like GPT-4o) for text extraction and description.
/// This is more accurate than traditional OCR for complex diagrams,
/// charts, and handwritten text.
///
/// # Cost Warning
/// Each image processed incurs LLM API costs based on image dimensions.
/// A typical 1024x1024 image costs ~765 tokens with GPT-4o.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageOcrConfig {
    /// Enable LLM-based OCR for images. **Disabled by default**.
    ///
    /// When enabled, images are extracted from the PDF and sent to the
    /// configured LLM provider for text extraction and description.
    pub enabled: bool,

    /// Extract text from images (OCR functionality).
    ///
    /// When true, the LLM will attempt to read any text visible in images.
    pub extract_text: bool,

    /// Generate descriptions for images and figures.
    ///
    /// When true, the LLM will provide a natural language description
    /// of the image content, useful for accessibility and indexing.
    pub generate_descriptions: bool,

    /// Analyze charts and diagrams for structured data.
    ///
    /// When true, the LLM will attempt to extract data from charts,
    /// graphs, and diagrams into markdown tables.
    pub analyze_charts: bool,

    /// Model to use for image OCR. Defaults to vision-capable model.
    ///
    /// Examples: "gpt-4o", "gpt-4o-mini", "claude-3-5-sonnet-latest"
    pub model: String,

    /// Minimum image size (in pixels) to process.
    ///
    /// Images smaller than this threshold are skipped to avoid
    /// processing icons and decorative elements.
    pub min_image_size: u32,

    /// Maximum number of images to process per page.
    ///
    /// Limits cost and processing time for image-heavy documents.
    pub max_images_per_page: usize,

    /// Detail level for OpenAI vision API ("low", "high", "auto").
    ///
    /// - "low": 85 tokens per image, faster and cheaper
    /// - "high": More tokens, better for text-heavy images
    /// - "auto": Let the model decide
    pub detail_level: String,
}

impl Default for ImageOcrConfig {
    fn default() -> Self {
        Self {
            enabled: false, // Disabled by default
            extract_text: true,
            generate_descriptions: true,
            analyze_charts: false,
            model: "gpt-4o-mini".to_string(),
            min_image_size: 50, // Skip very small images
            max_images_per_page: 10,
            detail_level: "auto".to_string(),
        }
    }
}

impl ImageOcrConfig {
    /// Create a new config with image OCR enabled.
    pub fn enabled() -> Self {
        Self {
            enabled: true,
            ..Default::default()
        }
    }

    /// Enable OCR mode.
    pub fn with_enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// Set the model for image OCR.
    pub fn with_model(mut self, model: impl Into<String>) -> Self {
        self.model = model.into();
        self
    }

    /// Set detail level for vision API.
    pub fn with_detail(mut self, detail: impl Into<String>) -> Self {
        self.detail_level = detail.into();
        self
    }

    /// Enable chart analysis.
    pub fn with_chart_analysis(mut self, enabled: bool) -> Self {
        self.analyze_charts = enabled;
        self
    }
}

/// Configuration for PDF extraction operations.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct PdfConfig {
    /// Extraction mode (text, vision, hybrid).
    pub mode: ExtractionMode,

    /// Output format.
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

    /// LLM-based image OCR configuration. **Disabled by default**.
    ///
    /// When enabled, images and figures in the PDF are processed by a
    /// multimodal LLM for text extraction and description generation.
    #[serde(default)]
    pub image_ocr: ImageOcrConfig,
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
            enhance_tables: false, // WHY: Disabled by default - LLM enhancement can corrupt lattice-generated tables
            ai_temperature: 0.1,
            normalize_spacing: true,
            consolidate_headers: true,
            extract_figure_captions: true,
            enhance_readability: false,
            layout: LayoutConfig::default(),
            vision_dpi: 150,
            quality_threshold: 0.5,
            image_ocr: ImageOcrConfig::default(),
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

    /// Set the image OCR configuration.
    ///
    /// # Example
    ///
    /// ```rust
    /// use edgequake_pdf::{PdfConfig, ImageOcrConfig};
    ///
    /// let config = PdfConfig::new()
    ///     .with_image_ocr(ImageOcrConfig::enabled());
    /// ```
    pub fn with_image_ocr(mut self, config: ImageOcrConfig) -> Self {
        self.image_ocr = config;
        self
    }

    /// Enable LLM-based image OCR with default settings.
    ///
    /// This is a convenience method to quickly enable image OCR.
    /// For more control, use `with_image_ocr()` with a custom config.
    pub fn with_image_ocr_enabled(mut self) -> Self {
        self.image_ocr = ImageOcrConfig::enabled();
        self
    }

    /// Load configuration from a TOML file.
    ///
    /// # Example
    ///
    /// ```toml
    /// mode = "Text"
    /// output_format = "Markdown"
    /// ocr_threshold = 0.8
    /// include_page_numbers = true
    ///
    /// [layout]
    /// detect_columns = true
    /// detect_tables = true
    /// column_gap_threshold = 20.0
    /// ```
    pub fn from_toml_file(path: impl AsRef<std::path::Path>) -> Result<Self, ConfigError> {
        let contents = std::fs::read_to_string(path.as_ref())
            .map_err(|e| ConfigError::IoError(e.to_string()))?;
        Self::from_toml(&contents)
    }

    /// Load configuration from a TOML string.
    pub fn from_toml(toml_str: &str) -> Result<Self, ConfigError> {
        toml::from_str(toml_str).map_err(|e| ConfigError::ParseError(e.to_string()))
    }

    /// Save configuration to a TOML file.
    pub fn to_toml_file(&self, path: impl AsRef<std::path::Path>) -> Result<(), ConfigError> {
        let toml_str = self.to_toml()?;
        std::fs::write(path.as_ref(), toml_str).map_err(|e| ConfigError::IoError(e.to_string()))?;
        Ok(())
    }

    /// Serialize configuration to a TOML string.
    pub fn to_toml(&self) -> Result<String, ConfigError> {
        toml::to_string_pretty(self).map_err(|e| ConfigError::SerializeError(e.to_string()))
    }

    /// Validate the configuration.
    ///
    /// Returns errors for invalid combinations or values.
    pub fn validate(&self) -> Result<(), ConfigError> {
        // Validate OCR threshold
        if !(0.0..=1.0).contains(&self.ocr_threshold) {
            return Err(ConfigError::ValidationError(
                "ocr_threshold must be between 0.0 and 1.0".to_string(),
            ));
        }

        // Validate quality threshold
        if !(0.0..=1.0).contains(&self.quality_threshold) {
            return Err(ConfigError::ValidationError(
                "quality_threshold must be between 0.0 and 1.0".to_string(),
            ));
        }

        // Validate AI temperature
        if !(0.0..=2.0).contains(&self.ai_temperature) {
            return Err(ConfigError::ValidationError(
                "ai_temperature must be between 0.0 and 2.0".to_string(),
            ));
        }

        // Validate vision DPI
        if self.vision_dpi < 72 || self.vision_dpi > 600 {
            return Err(ConfigError::ValidationError(
                "vision_dpi must be between 72 and 600".to_string(),
            ));
        }

        // Validate layout column gap
        if self.layout.column_gap_threshold < 0.0 {
            return Err(ConfigError::ValidationError(
                "column_gap_threshold must be non-negative".to_string(),
            ));
        }

        Ok(())
    }
}

/// Configuration errors.
#[derive(Debug, Clone, thiserror::Error)]
pub enum ConfigError {
    #[error("IO error: {0}")]
    IoError(String),

    #[error("Parse error: {0}")]
    ParseError(String),

    #[error("Serialize error: {0}")]
    SerializeError(String),

    #[error("Validation error: {0}")]
    ValidationError(String),
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

        let modes = vec![
            ExtractionMode::Text,
            ExtractionMode::Vision,
            ExtractionMode::Hybrid,
        ];
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

    // TOML configuration tests (Phase 5)

    #[test]
    fn test_config_to_toml() {
        let config = PdfConfig::default();
        let toml_str = config.to_toml().expect("Should serialize to TOML");

        assert!(toml_str.contains("mode = \"Text\""));
        assert!(toml_str.contains("output_format = \"Markdown\""));
        assert!(toml_str.contains("ocr_threshold = 0.8"));
    }

    #[test]
    fn test_config_from_toml() {
        let toml_str = r#"
            mode = "Vision"
            output_format = "Json"
            ocr_threshold = 0.9
            include_page_numbers = false
            include_metadata = true
            include_styles = false
            quality_threshold = 0.7
            vision_dpi = 200
            ai_temperature = 0.5

            [layout]
            detect_columns = true
            detect_tables = true
            detect_equations = false
            column_gap_threshold = 25.0
            use_xy_cut = true
        "#;

        let config = PdfConfig::from_toml(toml_str).expect("Should parse TOML");
        assert_eq!(config.mode, ExtractionMode::Vision);
        assert_eq!(config.output_format, OutputFormat::Json);
        assert!((config.ocr_threshold - 0.9).abs() < 0.001);
        assert!(!config.include_page_numbers);
        assert_eq!(config.vision_dpi, 200);
    }

    #[test]
    fn test_config_toml_roundtrip() {
        let original = PdfConfig::new()
            .with_mode(ExtractionMode::Hybrid)
            .with_output_format(OutputFormat::Html)
            .with_max_pages(50)
            .with_vision_dpi(300);

        let toml_str = original.to_toml().expect("Should serialize");
        let parsed = PdfConfig::from_toml(&toml_str).expect("Should parse");

        assert_eq!(parsed.mode, original.mode);
        assert_eq!(parsed.output_format, original.output_format);
        assert_eq!(parsed.max_pages, original.max_pages);
        assert_eq!(parsed.vision_dpi, original.vision_dpi);
    }

    #[test]
    fn test_config_validate_valid() {
        let config = PdfConfig::default();
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_config_validate_invalid_ocr_threshold() {
        let mut config = PdfConfig::default();
        config.ocr_threshold = 1.5; // Invalid: > 1.0

        let result = config.validate();
        assert!(result.is_err());
        if let Err(ConfigError::ValidationError(msg)) = result {
            assert!(msg.contains("ocr_threshold"));
        }
    }

    #[test]
    fn test_config_validate_invalid_quality_threshold() {
        let mut config = PdfConfig::default();
        config.quality_threshold = -0.1; // Invalid: < 0.0

        let result = config.validate();
        assert!(result.is_err());
        if let Err(ConfigError::ValidationError(msg)) = result {
            assert!(msg.contains("quality_threshold"));
        }
    }

    #[test]
    fn test_config_validate_invalid_ai_temperature() {
        let mut config = PdfConfig::default();
        config.ai_temperature = 2.5; // Invalid: > 2.0

        let result = config.validate();
        assert!(result.is_err());
        if let Err(ConfigError::ValidationError(msg)) = result {
            assert!(msg.contains("ai_temperature"));
        }
    }

    #[test]
    fn test_config_validate_invalid_vision_dpi() {
        let mut config = PdfConfig::default();
        config.vision_dpi = 50; // Invalid: < 72

        let result = config.validate();
        assert!(result.is_err());
        if let Err(ConfigError::ValidationError(msg)) = result {
            assert!(msg.contains("vision_dpi"));
        }
    }

    #[test]
    fn test_config_validate_invalid_column_gap() {
        let mut config = PdfConfig::default();
        config.layout.column_gap_threshold = -5.0; // Invalid: < 0.0

        let result = config.validate();
        assert!(result.is_err());
        if let Err(ConfigError::ValidationError(msg)) = result {
            assert!(msg.contains("column_gap_threshold"));
        }
    }

    #[test]
    fn test_config_from_toml_invalid() {
        let invalid_toml = "this is not valid toml {{{{";
        let result = PdfConfig::from_toml(invalid_toml);
        assert!(matches!(result, Err(ConfigError::ParseError(_))));
    }

    #[test]
    fn test_config_toml_file_roundtrip() {
        use std::fs;

        let config = PdfConfig::new()
            .with_mode(ExtractionMode::Vision)
            .with_max_pages(25);

        let temp_path = std::env::temp_dir().join("test_config.toml");

        // Write to file
        config.to_toml_file(&temp_path).expect("Should write file");

        // Read back
        let loaded = PdfConfig::from_toml_file(&temp_path).expect("Should read file");

        assert_eq!(loaded.mode, config.mode);
        assert_eq!(loaded.max_pages, config.max_pages);

        // Cleanup
        fs::remove_file(&temp_path).ok();
    }

    #[test]
    fn test_config_error_display() {
        let io_err = ConfigError::IoError("file not found".to_string());
        let display = format!("{}", io_err);
        assert!(display.contains("IO error"));

        let parse_err = ConfigError::ParseError("invalid syntax".to_string());
        let display = format!("{}", parse_err);
        assert!(display.contains("Parse error"));

        let validation_err = ConfigError::ValidationError("threshold out of range".to_string());
        let display = format!("{}", validation_err);
        assert!(display.contains("Validation error"));
    }
}
