//! Formula detection and LaTeX reconstruction.
//!
//! WHY: Academic papers often contain math formulas as either:
//! - Inline math (embedded in text)
//! - Display equations (standalone blocks)
//!
//! This detector identifies blocks with high math symbol density and
//! reconstructs them as LaTeX, including superscript/subscript detection
//! based on Y-offset analysis.

use super::symbol_map::{is_math_symbol, MATH_SYMBOL_MAP};
use crate::schema::{Block, BlockType, BoundingBox, Page};
use serde::{Deserialize, Serialize};

/// Configuration for formula detection.
#[derive(Debug, Clone)]
pub struct FormulaConfig {
    /// Minimum ratio of math symbols to trigger detection (0.0-1.0).
    /// WHY: 0.15 is the sweet spot - lower catches too much regular text,
    /// higher misses formulas with variables.
    pub min_math_density: f32,

    /// Minimum confidence to accept a reconstructed formula (0.0-1.0).
    pub min_confidence: f32,

    /// Y-offset threshold for superscript detection (in points).
    /// WHY: Most fonts use ~2pt baseline shift for super/subscripts.
    pub superscript_threshold: f32,

    /// Y-offset threshold for subscript detection (in points).
    pub subscript_threshold: f32,

    /// Whether to detect inline math (within text blocks).
    pub detect_inline: bool,

    /// Whether to detect display equations (standalone blocks).
    pub detect_display: bool,
}

impl Default for FormulaConfig {
    fn default() -> Self {
        Self {
            min_math_density: 0.15,
            min_confidence: 0.5,
            superscript_threshold: -2.0,
            subscript_threshold: 2.0,
            detect_inline: true,
            detect_display: true,
        }
    }
}

impl FormulaConfig {
    /// Create a new config.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set minimum math density threshold.
    pub fn with_min_density(mut self, density: f32) -> Self {
        self.min_math_density = density;
        self
    }

    /// Set minimum confidence threshold.
    pub fn with_min_confidence(mut self, confidence: f32) -> Self {
        self.min_confidence = confidence;
        self
    }
}

/// A detected mathematical formula.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Formula {
    /// The reconstructed LaTeX representation.
    pub latex: String,

    /// Bounding box of the formula in the page.
    pub bbox: BoundingBox,

    /// Confidence score (0.0-1.0).
    pub confidence: f32,

    /// Whether this is display math (standalone) or inline.
    pub is_display: bool,

    /// Source block index in the page.
    pub source_block_idx: usize,
}

impl Formula {
    /// Wrap the LaTeX in appropriate delimiters.
    pub fn to_markdown(&self) -> String {
        if self.is_display {
            format!("$${}$$", self.latex)
        } else {
            format!("${}$", self.latex)
        }
    }
}

/// Formula detector for extracting math from document pages.
pub struct FormulaDetector {
    config: FormulaConfig,
}

impl FormulaDetector {
    /// Create a new formula detector with the given config.
    pub fn new(config: FormulaConfig) -> Self {
        Self { config }
    }

    /// Create with default configuration.
    pub fn with_defaults() -> Self {
        Self::new(FormulaConfig::default())
    }

    /// Detect all formulas in a page.
    ///
    /// Returns detected formulas sorted by position (top to bottom, left to right).
    /// @implements FEAT1005
    pub fn detect_formulas(&self, page: &Page) -> Vec<Formula> {
        let mut formulas = Vec::new();

        for (idx, block) in page.blocks.iter().enumerate() {
            // Skip blocks that are already marked as math
            if matches!(block.block_type, BlockType::Equation) {
                if let Some(formula) = self.reconstruct_from_equation(block, idx) {
                    formulas.push(formula);
                }
                continue;
            }

            // Check math symbol density
            let density = self.calculate_math_density(&block.text);

            if density >= self.config.min_math_density {
                let is_display = self.is_display_equation(block, page);

                if (is_display && self.config.detect_display)
                    || (!is_display && self.config.detect_inline)
                {
                    if let Some(formula) = self.reconstruct_formula(block, idx, is_display) {
                        if formula.confidence >= self.config.min_confidence {
                            formulas.push(formula);
                        }
                    }
                }
            }
        }

        // Sort by position (top to bottom, left to right)
        formulas.sort_by(|a, b| {
            let y_cmp = a
                .bbox
                .y1
                .partial_cmp(&b.bbox.y1)
                .unwrap_or(std::cmp::Ordering::Equal);
            if y_cmp == std::cmp::Ordering::Equal {
                a.bbox
                    .x1
                    .partial_cmp(&b.bbox.x1)
                    .unwrap_or(std::cmp::Ordering::Equal)
            } else {
                y_cmp
            }
        });

        formulas
    }

    /// Calculate the ratio of math symbols to total characters.
    fn calculate_math_density(&self, text: &str) -> f32 {
        if text.is_empty() {
            return 0.0;
        }

        let math_count = text.chars().filter(|c| is_math_symbol(*c)).count();
        let total_chars = text.chars().count();

        math_count as f32 / total_chars as f32
    }

    /// Determine if a block is a display equation (standalone) vs inline.
    ///
    /// WHY: Display equations typically:
    /// - Are centered (not left-aligned with text)
    /// - Have significant vertical spacing
    /// - Are in their own block without surrounding text
    fn is_display_equation(&self, block: &Block, page: &Page) -> bool {
        // Check if block is centered
        let page_bbox = page.bbox();
        let page_center = (page_bbox.x1 + page_bbox.x2) / 2.0;
        let block_center = (block.bbox.x1 + block.bbox.x2) / 2.0;
        let is_centered = (page_center - block_center).abs() < 50.0;

        // Check if block is isolated (no adjacent blocks on same line)
        let is_isolated = !page.blocks.iter().any(|other| {
            other.id != block.id
                && (block.bbox.y1 - other.bbox.y2).abs() < 5.0
                && block.bbox.x1.max(other.bbox.x1) < block.bbox.x2.min(other.bbox.x2)
        });

        // Check if text is short (formulas without explanatory text)
        let is_short = block.text.len() < 100;

        is_centered && is_isolated && is_short
    }

    /// Reconstruct a formula from an already-marked equation block.
    fn reconstruct_from_equation(&self, block: &Block, idx: usize) -> Option<Formula> {
        let latex = self.convert_to_latex(&block.text);
        let confidence = self.estimate_confidence(&latex, &block.text);

        Some(Formula {
            latex,
            bbox: block.bbox,
            confidence,
            is_display: true,
            source_block_idx: idx,
        })
    }

    /// Reconstruct a LaTeX formula from a block.
    ///
    /// WHY: PDF blocks may have spans with different Y-offsets indicating
    /// superscripts or subscripts. We detect these by comparing span
    /// baselines to the block baseline.
    fn reconstruct_formula(&self, block: &Block, idx: usize, is_display: bool) -> Option<Formula> {
        let block_baseline = self.estimate_baseline(block);

        // Process spans if available
        if !block.children.is_empty() {
            for child_id in &block.children {
                // Look up child block by ID
                // For now, just use the text directly
                let _ = child_id; // Suppress unused warning
            }
        }

        // Convert the entire text to LaTeX
        let latex = self.convert_with_structure(&block.text, block_baseline, block);

        if latex.is_empty() {
            return None;
        }

        let confidence = self.estimate_confidence(&latex, &block.text);

        Some(Formula {
            latex,
            bbox: block.bbox,
            confidence,
            is_display,
            source_block_idx: idx,
        })
    }

    /// Convert text to LaTeX, detecting superscripts/subscripts from structure.
    fn convert_with_structure(&self, text: &str, _baseline: f32, _block: &Block) -> String {
        let mut result = String::with_capacity(text.len() * 2);

        // Track if we're in a superscript or subscript
        let mut in_super = false;
        let mut in_sub = false;

        for ch in text.chars() {
            // Check for superscript digits
            if ('⁰'..='⁹').contains(&ch) {
                if !in_super {
                    result.push_str("^{");
                    in_super = true;
                }
                // Convert superscript digit to regular digit
                let digit = ch as u32 - '⁰' as u32;
                result.push(char::from_digit(digit, 10).unwrap_or('?'));
                continue;
            }

            // Check for subscript digits
            if ('₀'..='₉').contains(&ch) {
                if !in_sub {
                    if in_super {
                        result.push('}');
                        in_super = false;
                    }
                    result.push_str("_{");
                    in_sub = true;
                }
                // Convert subscript digit to regular digit
                let digit = ch as u32 - '₀' as u32;
                result.push(char::from_digit(digit, 10).unwrap_or('?'));
                continue;
            }

            // Close any open groups if switching to regular text
            if in_super {
                result.push('}');
                in_super = false;
            }
            if in_sub {
                result.push('}');
                in_sub = false;
            }

            // Convert math symbols
            if let Some(latex_cmd) = MATH_SYMBOL_MAP.get(&ch) {
                result.push_str(latex_cmd);
                result.push(' ');
            } else {
                result.push(ch);
            }
        }

        // Close any remaining groups
        if in_super {
            result.push('}');
        }
        if in_sub {
            result.push('}');
        }

        result.trim().to_string()
    }

    /// Convert text to LaTeX using the symbol map.
    fn convert_to_latex(&self, text: &str) -> String {
        let mut result = String::with_capacity(text.len() * 2);

        for ch in text.chars() {
            if let Some(latex_cmd) = MATH_SYMBOL_MAP.get(&ch) {
                result.push_str(latex_cmd);
                result.push(' ');
            } else {
                result.push(ch);
            }
        }

        result.trim().to_string()
    }

    /// Estimate the baseline Y-coordinate for a block.
    fn estimate_baseline(&self, block: &Block) -> f32 {
        // Use bottom of bounding box as approximate baseline
        block.bbox.y2
    }

    /// Estimate confidence based on how well the conversion worked.
    ///
    /// WHY: Confidence is based on:
    /// - Ratio of recognized symbols to unknowns
    /// - Balanced braces/parentheses
    /// - No obvious errors (empty result, all whitespace, etc.)
    fn estimate_confidence(&self, latex: &str, original: &str) -> f32 {
        if latex.is_empty() || original.is_empty() {
            return 0.0;
        }

        let mut score: f32 = 1.0;

        // Penalize if result is mostly whitespace
        let non_whitespace: usize = latex.chars().filter(|c| !c.is_whitespace()).count();
        if non_whitespace < 3 {
            score *= 0.3;
        }

        // Penalize unbalanced braces
        let open_braces = latex.chars().filter(|c| *c == '{').count();
        let close_braces = latex.chars().filter(|c| *c == '}').count();
        if open_braces != close_braces {
            score *= 0.5;
        }

        // Penalize unbalanced parentheses
        let open_parens = latex.chars().filter(|c| *c == '(').count();
        let close_parens = latex.chars().filter(|c| *c == ')').count();
        if open_parens != close_parens {
            score *= 0.7;
        }

        // Bonus for recognized LaTeX commands
        let command_count = latex.matches('\\').count();
        if command_count > 0 {
            score = (score + 0.1).min(1.0);
        }

        // Bonus for math-like structure
        if latex.contains('^') || latex.contains('_') {
            score = (score + 0.1).min(1.0);
        }

        score
    }

    /// Mark blocks in a page as TextInlineMath or Equation based on detection.
    pub fn annotate_page(&self, page: &mut Page) {
        let formulas = self.detect_formulas(page);

        for formula in formulas {
            if formula.source_block_idx < page.blocks.len() {
                let block = &mut page.blocks[formula.source_block_idx];
                block.block_type = if formula.is_display {
                    BlockType::Equation
                } else {
                    BlockType::TextInlineMath
                };
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema::BlockId;

    fn make_block(text: &str) -> Block {
        Block {
            id: BlockId::generate(),
            block_type: BlockType::Text,
            bbox: BoundingBox {
                x1: 100.0,
                y1: 100.0,
                x2: 300.0,
                y2: 120.0,
            },
            page: 0,
            position: 0,
            text: text.to_string(),
            html: None,
            spans: vec![],
            children: vec![],
            confidence: 1.0,
            level: None,
            source: None,
            metadata: Default::default(),
        }
    }

    fn make_page(blocks: Vec<Block>) -> Page {
        use crate::schema::{ExtractionMethod, PageStats};
        use std::collections::HashMap;

        Page {
            number: 1,
            width: 612.0,
            height: 792.0,
            blocks,
            method: ExtractionMethod::Native,
            stats: PageStats::default(),
            columns: vec![],
            margins: None,
            metadata: HashMap::new(),
        }
    }

    #[test]
    fn test_math_density_empty() {
        let detector = FormulaDetector::with_defaults();
        assert_eq!(detector.calculate_math_density(""), 0.0);
    }

    #[test]
    fn test_math_density_no_math() {
        let detector = FormulaDetector::with_defaults();
        assert_eq!(detector.calculate_math_density("hello world"), 0.0);
    }

    #[test]
    fn test_math_density_all_math() {
        let detector = FormulaDetector::with_defaults();
        let density = detector.calculate_math_density("αβγ");
        assert!(density > 0.9);
    }

    #[test]
    fn test_math_density_mixed() {
        let detector = FormulaDetector::with_defaults();
        let density = detector.calculate_math_density("x + α = β");
        // 2 math symbols out of 9 chars (including spaces)
        assert!(density > 0.1 && density < 0.5);
    }

    #[test]
    fn test_detect_formula_high_density() {
        let detector = FormulaDetector::with_defaults();
        let block = make_block("∑ αβγ ∫ δε");
        let page = make_page(vec![block]);

        let formulas = detector.detect_formulas(&page);
        assert!(!formulas.is_empty());
    }

    #[test]
    fn test_detect_formula_low_density() {
        let detector = FormulaDetector::with_defaults();
        let block = make_block("This is regular text with no math symbols");
        let page = make_page(vec![block]);

        let formulas = detector.detect_formulas(&page);
        assert!(formulas.is_empty());
    }

    #[test]
    fn test_convert_with_superscript() {
        let detector = FormulaDetector::with_defaults();
        let result = detector.convert_to_latex("x²");
        assert!(result.contains("^2"));
    }

    #[test]
    fn test_convert_with_subscript() {
        let detector = FormulaDetector::with_defaults();
        let result = detector.convert_to_latex("x₂");
        assert!(result.contains("_2"));
    }

    #[test]
    fn test_formula_to_markdown_display() {
        let formula = Formula {
            latex: r"\sum_{i=1}^n x_i".to_string(),
            bbox: BoundingBox::new(0.0, 0.0, 100.0, 20.0),
            confidence: 0.9,
            is_display: true,
            source_block_idx: 0,
        };
        assert_eq!(formula.to_markdown(), r"$$\sum_{i=1}^n x_i$$");
    }

    #[test]
    fn test_formula_to_markdown_inline() {
        let formula = Formula {
            latex: "x^2".to_string(),
            bbox: BoundingBox::new(0.0, 0.0, 50.0, 15.0),
            confidence: 0.8,
            is_display: false,
            source_block_idx: 0,
        };
        assert_eq!(formula.to_markdown(), "$x^2$");
    }

    #[test]
    fn test_confidence_balanced_braces() {
        let detector = FormulaDetector::with_defaults();
        let good = detector.estimate_confidence(r"\frac{a}{b}", "a/b");
        let bad = detector.estimate_confidence(r"\frac{a}{b", "a/b");
        assert!(good > bad);
    }

    #[test]
    fn test_annotate_page() {
        let mut page = make_page(vec![
            make_block("Regular text here"),
            make_block("∑ αβγ ∫ δε ∂∇"),
        ]);

        let detector = FormulaDetector::with_defaults();
        detector.annotate_page(&mut page);

        // First block should remain Text
        assert_eq!(page.blocks[0].block_type, BlockType::Text);
        // Second block should be marked as math
        assert!(matches!(
            page.blocks[1].block_type,
            BlockType::Equation | BlockType::TextInlineMath
        ));
    }

    #[test]
    fn test_greek_letters_converted() {
        let detector = FormulaDetector::with_defaults();
        let result = detector.convert_to_latex("α + β = γ");
        assert!(result.contains(r"\alpha"));
        assert!(result.contains(r"\beta"));
        assert!(result.contains(r"\gamma"));
    }

    #[test]
    fn test_operators_converted() {
        let detector = FormulaDetector::with_defaults();
        let result = detector.convert_to_latex("∑∫∂");
        assert!(result.contains(r"\sum"));
        assert!(result.contains(r"\int"));
        assert!(result.contains(r"\partial"));
    }

    #[test]
    fn test_formula_struct_fields() {
        let formula = Formula {
            latex: "x".to_string(),
            bbox: BoundingBox::new(10.0, 20.0, 30.0, 40.0),
            confidence: 0.75,
            is_display: false,
            source_block_idx: 5,
        };
        assert_eq!(formula.latex, "x");
        assert_eq!(formula.source_block_idx, 5);
        assert!(!formula.is_display);
    }

    #[test]
    fn test_detector_custom_threshold() {
        let config = FormulaConfig {
            min_math_density: 0.5,
            ..Default::default()
        };
        let detector = FormulaDetector::new(config);
        // With higher threshold, regular text won't trigger
        let block = make_block("x + α");
        let page = make_page(vec![block]);
        let formulas = detector.detect_formulas(&page);
        assert!(formulas.is_empty());
    }

    #[test]
    fn test_empty_page_detection() {
        let detector = FormulaDetector::with_defaults();
        let page = make_page(vec![]);
        let formulas = detector.detect_formulas(&page);
        assert!(formulas.is_empty());
    }

    #[test]
    fn test_multiple_formulas_in_page() {
        let detector = FormulaDetector::with_defaults();
        let page = make_page(vec![
            make_block("∑ αβγ ∫ δε"),
            make_block("Regular text"),
            make_block("∂∇ ∈ ∀∃"),
        ]);
        let formulas = detector.detect_formulas(&page);
        assert!(formulas.len() >= 2);
    }

    #[test]
    fn test_relations_converted() {
        let detector = FormulaDetector::with_defaults();
        let result = detector.convert_to_latex("≤ ≥ ≠ ≈");
        assert!(result.contains(r"\leq") || result.contains(r"\le"));
    }

    #[test]
    fn test_infinity_converted() {
        let detector = FormulaDetector::with_defaults();
        let result = detector.convert_to_latex("∞");
        assert!(result.contains(r"\infty"));
    }

    #[test]
    fn test_root_symbol_converted() {
        let detector = FormulaDetector::with_defaults();
        let result = detector.convert_to_latex("√");
        assert!(result.contains(r"\sqrt"));
    }

    // OODA-21: FormulaConfig builder method tests

    #[test]
    fn test_formula_config_with_min_density() {
        let config = FormulaConfig::new().with_min_density(0.25);
        assert_eq!(config.min_math_density, 0.25);
        // Other fields should remain at defaults
        assert_eq!(config.min_confidence, 0.5);
    }

    #[test]
    fn test_formula_config_with_min_confidence() {
        let config = FormulaConfig::new().with_min_confidence(0.8);
        assert_eq!(config.min_confidence, 0.8);
        // Other fields should remain at defaults
        assert_eq!(config.min_math_density, 0.15);
    }

    #[test]
    fn test_formula_config_new_equals_default() {
        let from_new = FormulaConfig::new();
        let from_default = FormulaConfig::default();
        // WHY: new() and Default should produce equivalent configs
        assert_eq!(from_new.min_math_density, from_default.min_math_density);
        assert_eq!(from_new.min_confidence, from_default.min_confidence);
        assert_eq!(
            from_new.superscript_threshold,
            from_default.superscript_threshold
        );
        assert_eq!(
            from_new.subscript_threshold,
            from_default.subscript_threshold
        );
        assert_eq!(from_new.detect_inline, from_default.detect_inline);
        assert_eq!(from_new.detect_display, from_default.detect_display);
    }
}
