/// Text element with position and font info
#[derive(Debug, Clone)]
pub struct TextElement {
    pub text: String,
    pub x: f32,
    pub y: f32,
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
