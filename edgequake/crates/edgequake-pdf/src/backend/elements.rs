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
}

/// Graphical line element
#[derive(Debug, Clone)]
pub struct PdfLine {
    pub p1: (f32, f32),
    pub p2: (f32, f32),
    pub width: f32,
}
