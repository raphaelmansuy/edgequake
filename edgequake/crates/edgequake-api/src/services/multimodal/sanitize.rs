//! VLM/Extract text sanitization (LightRAG `sanitize_text_for_encoding` subset).

/// Strip control characters except tab/newline/carriage-return.
/// Restores LaTeX backslashes from form-feed/backspace + letter (LightRAG subset).
pub fn sanitize_text_for_encoding(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '\t' | '\n' | '\r' => out.push(c),
            '\x0c' | '\x08' if chars.peek().is_some_and(|n| n.is_ascii_alphabetic()) => {
                out.push('\\');
                if let Some(next) = chars.next() {
                    out.push(next);
                }
            }
            c if c.is_control() => {}
            c => out.push(c),
        }
    }
    out.trim().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strips_null_and_preserves_newline() {
        let s = "line1\x00line2\nline3";
        assert_eq!(sanitize_text_for_encoding(s), "line1line2\nline3");
    }

    #[test]
    fn restores_formfeed_latex() {
        // "\frac" decoded as formfeed + "rac"
        let corrupted = "\x0crac{a}{b}";
        assert_eq!(sanitize_text_for_encoding(corrupted), "\\rac{a}{b}");
    }
}
