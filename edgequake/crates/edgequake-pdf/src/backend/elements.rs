/// A single character with exact bounding box from PDFium.
///
/// WHY character-level extraction:
/// - PDFium provides accurate character positions (unlike lopdf)
/// - Enables pymupdf4llm-style layout analysis algorithms
/// - Character-level precision for multi-column detection
///
/// ## pdfium-render API mapping:
/// - `bounds()` → `x0, y0, x1, y1` (PDF points)
/// - `origin()` → character baseline origin
/// - `font_size()` → size in points
/// - `font_is_italic()` → is_italic flag (from font descriptor flags)
/// - `font_weight()` → is_bold flag (Weight700Bold or higher)
/// - `font_is_fixed_pitch()` → is_monospace flag (from font descriptor flags)
#[derive(Debug, Clone)]
pub struct RawChar {
    /// The character itself
    pub char: char,
    /// Left edge of bounding box (PDF points, origin at bottom-left)
    pub x0: f32,
    /// Bottom edge of bounding box
    pub y0: f32,
    /// Right edge of bounding box
    pub x1: f32,
    /// Top edge of bounding box
    pub y1: f32,
    /// Font size in points
    pub font_size: f32,
    /// Font name (if available)
    pub font_name: Option<String>,
    /// Page number (0-indexed)
    pub page_num: usize,
    /// Bold flag from font descriptor (Weight >= 700)
    /// WHY: Font name matching is unreliable. PDFium provides accurate
    /// font weight from the font descriptor via font_weight().
    pub is_bold: bool,
    /// Italic flag from font descriptor
    /// WHY: Font name matching is unreliable. PDFium provides accurate
    /// italic flag from the font descriptor via font_is_italic().
    pub is_italic: bool,
    /// Monospace (fixed-pitch) flag from font descriptor
    /// WHY: Font name matching ("Mono", "Courier") misses many monospace fonts.
    /// OODA-03: PDFium provides accurate fixed-pitch flag from font descriptor
    /// via font_is_fixed_pitch(). This is the same data PyMuPDF uses.
    pub is_monospace: bool,
}

impl RawChar {
    /// Width of the character bounding box
    #[inline]
    pub fn width(&self) -> f32 {
        self.x1 - self.x0
    }

    /// Height of the character bounding box
    #[inline]
    pub fn height(&self) -> f32 {
        self.y1 - self.y0
    }

    /// Center X coordinate
    #[inline]
    pub fn center_x(&self) -> f32 {
        (self.x0 + self.x1) / 2.0
    }

    /// Center Y coordinate
    #[inline]
    pub fn center_y(&self) -> f32 {
        (self.y0 + self.y1) / 2.0
    }
}

/// Text element with position and font info
#[derive(Debug, Clone)]
pub struct TextElement {
    pub text: String,
    pub x: f32,
    pub y: f32,
    /// Estimated width of the text element in PDF points.
    /// OODA-06: Calculated as char_count * font_size * 0.48 based on PyMuPDF analysis.
    /// Empirical data shows actual char width ratio is 0.43-0.53 (mean ~0.48).
    /// Used for accurate word gap detection in merge_line().
    pub width: f32,
    pub font_size: f32,
    pub font_name: String,
    pub is_bold: bool,
    pub is_italic: bool,
    /// OODA-19: Flag for rotated text (e.g., arXiv watermarks in margins)
    /// Rotated text is detected via CTM matrix analysis:
    /// - Normal text: ctm[0] ≈ 1.0, ctm[1] ≈ 0, ctm[2] ≈ 0, ctm[3] ≈ 1.0
    /// - 90° rotation: ctm[0] ≈ 0, |ctm[1]| ≈ 1 or |ctm[2]| ≈ 1
    pub is_rotated: bool,
}

/// Graphical line element
#[derive(Debug, Clone)]
pub struct PdfLine {
    pub p1: (f32, f32),
    pub p2: (f32, f32),
    pub width: f32,
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper to create a RawChar with minimal required fields
    fn make_char(x0: f32, y0: f32, x1: f32, y1: f32) -> RawChar {
        RawChar {
            char: 'A',
            x0,
            y0,
            x1,
            y1,
            font_size: 12.0,
            font_name: None,
            page_num: 0,
            is_bold: false,
            is_italic: false,
            is_monospace: false,
        }
    }

    #[test]
    fn test_raw_char_dimensions() {
        let c = make_char(10.0, 20.0, 25.0, 32.0);
        // Width = x1 - x0 = 25 - 10 = 15
        assert!((c.width() - 15.0).abs() < 0.001, "width should be 15");
        // Height = y1 - y0 = 32 - 20 = 12
        assert!((c.height() - 12.0).abs() < 0.001, "height should be 12");
    }

    #[test]
    fn test_raw_char_center_point() {
        let c = make_char(10.0, 20.0, 30.0, 40.0);
        // Center X = (10 + 30) / 2 = 20
        assert!((c.center_x() - 20.0).abs() < 0.001, "center_x should be 20");
        // Center Y = (20 + 40) / 2 = 30
        assert!((c.center_y() - 30.0).abs() < 0.001, "center_y should be 30");
    }

    #[test]
    fn test_raw_char_zero_size() {
        // Edge case: point-sized character (zero dimensions)
        let c = make_char(100.0, 200.0, 100.0, 200.0);
        assert!((c.width() - 0.0).abs() < 0.001, "zero-width char");
        assert!((c.height() - 0.0).abs() < 0.001, "zero-height char");
        // Center should still be the point itself
        assert!((c.center_x() - 100.0).abs() < 0.001, "center_x at point");
        assert!((c.center_y() - 200.0).abs() < 0.001, "center_y at point");
    }

    #[test]
    fn test_raw_char_large_coordinates() {
        // Real-world PDF: Letter size is 612x792 points
        let c = make_char(55.0, 700.0, 65.0, 712.0);
        assert!((c.width() - 10.0).abs() < 0.001);
        assert!((c.height() - 12.0).abs() < 0.001);
        assert!((c.center_x() - 60.0).abs() < 0.001);
        assert!((c.center_y() - 706.0).abs() < 0.001);
    }
}
