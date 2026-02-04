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
    /// Calculated as char_count * font_size * 0.55 for proportional fonts.
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
