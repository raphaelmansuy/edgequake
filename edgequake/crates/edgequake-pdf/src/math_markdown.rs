//! Canonical math spans in vision markdown.
//!
//! A display formula is one object: `$$` for the viewer and an
//! `<equation id="eq-p{page}-{n}">` tag for the multimodal scan. Inline
//! `\(...\)` becomes `$...$` and is not tagged.

use crate::page_marker::{PAGE_MARKER_PREFIX, PAGE_MARKER_SUFFIX};

/// Normalize delimiters and wrap each display formula in an equation tag.
///
/// Idempotent: an existing `<equation ...>` element is copied through.
/// Fenced code is copied through. Pages without formulas are unchanged.
pub fn canonicalize_math_markdown(markdown: &str) -> String {
    if markdown.is_empty() {
        return String::new();
    }
    let chars: Vec<char> = markdown.chars().collect();
    let mut out = String::with_capacity(markdown.len() + 32);
    let mut page: u32 = 1;
    let mut seq: u32 = 0;
    let mut i = 0usize;
    while i < chars.len() {
        if let Some((end, next_page)) = match_page_marker(&chars, i) {
            page = next_page;
            seq = 0;
            push_slice(&chars, i, end, &mut out);
            i = end;
            continue;
        }
        if starts_with_fence(&chars, i) {
            i = copy_fence(&chars, i, &mut out);
            continue;
        }
        if let Some(end) = match_equation_tag(&chars, i) {
            push_slice(&chars, i, end, &mut out);
            i = end;
            continue;
        }
        if let Some((end, body)) = match_display_dollars(&chars, i) {
            if !body.trim().is_empty() {
                push_equation(&mut out, page, &mut seq, body.trim());
                i = end;
                continue;
            }
        }
        if let Some((end, body)) = match_escaped_delim(&chars, i, '[', ']') {
            push_equation(&mut out, page, &mut seq, body.trim());
            i = end;
            continue;
        }
        if let Some((end, body)) = match_equation_env(&chars, i) {
            push_equation(&mut out, page, &mut seq, body.trim());
            i = end;
            continue;
        }
        if let Some((end, body)) = match_escaped_delim(&chars, i, '(', ')') {
            out.push('$');
            out.push_str(body.trim());
            out.push('$');
            i = end;
            continue;
        }
        if let Some(end) = match_inline_dollars(&chars, i) {
            push_slice(&chars, i, end, &mut out);
            i = end;
            continue;
        }
        if let Some((end, body)) = match_bare_display_line(&chars, i) {
            push_equation(&mut out, page, &mut seq, &body);
            i = end;
            continue;
        }
        out.push(chars[i]);
        i += 1;
    }
    out
}

fn push_equation(out: &mut String, page: u32, seq: &mut u32, body: &str) {
    *seq += 1;
    out.push_str(&format!(
        "<equation id=\"eq-p{page}-{seq}\" format=\"latex\">\n$$\n{body}\n$$\n</equation>"
    ));
}

fn push_slice(chars: &[char], start: usize, end: usize, out: &mut String) {
    for c in &chars[start..end] {
        out.push(*c);
    }
}

fn line_start(chars: &[char], i: usize) -> bool {
    i == 0 || chars[i - 1] == '\n'
}

fn starts_with_fence(chars: &[char], i: usize) -> bool {
    i + 2 < chars.len() && chars[i] == '`' && chars[i + 1] == '`' && chars[i + 2] == '`'
}

fn copy_fence(chars: &[char], i: usize, out: &mut String) -> usize {
    out.push_str("```");
    let mut j = i + 3;
    while j < chars.len() {
        if starts_with_fence(chars, j) {
            out.push_str("```");
            return j + 3;
        }
        out.push(chars[j]);
        j += 1;
    }
    j
}

fn match_page_marker(chars: &[char], i: usize) -> Option<(usize, u32)> {
    if !line_start(chars, i) {
        return None;
    }
    let prefix: Vec<char> = PAGE_MARKER_PREFIX.chars().collect();
    if i + prefix.len() > chars.len() || chars[i..i + prefix.len()] != prefix[..] {
        return None;
    }
    let mut j = i + prefix.len();
    let num_start = j;
    while j < chars.len() && chars[j].is_ascii_digit() {
        j += 1;
    }
    if j == num_start {
        return None;
    }
    let suffix: Vec<char> = PAGE_MARKER_SUFFIX.chars().collect();
    if j + suffix.len() > chars.len() || chars[j..j + suffix.len()] != suffix[..] {
        return None;
    }
    let num: String = chars[num_start..j].iter().collect();
    let page = num.parse::<u32>().ok()?;
    let mut end = j + suffix.len();
    if chars.get(end) == Some(&'\n') {
        end += 1;
    }
    Some((end, page))
}

fn match_equation_tag(chars: &[char], i: usize) -> Option<usize> {
    const OPEN: &str = "<equation";
    const CLOSE: &str = "</equation>";
    let open: Vec<char> = OPEN.chars().collect();
    if i + open.len() > chars.len() || chars[i..i + open.len()] != open[..] {
        return None;
    }
    let close: Vec<char> = CLOSE.chars().collect();
    let mut j = i + open.len();
    while j + close.len() <= chars.len() {
        if chars[j..j + close.len()] == close[..] {
            return Some(j + close.len());
        }
        j += 1;
    }
    None
}

fn match_display_dollars(chars: &[char], i: usize) -> Option<(usize, String)> {
    if chars.get(i) != Some(&'$') || chars.get(i + 1) != Some(&'$') {
        return None;
    }
    let mut j = i + 2;
    while j + 1 < chars.len() {
        if chars[j] == '$' && chars[j + 1] == '$' && (j == 0 || chars[j - 1] != '\\') {
            let body: String = chars[i + 2..j].iter().collect();
            return Some((j + 2, body));
        }
        j += 1;
    }
    None
}

fn match_inline_dollars(chars: &[char], i: usize) -> Option<usize> {
    if chars.get(i) != Some(&'$') || chars.get(i + 1) == Some(&'$') {
        return None;
    }
    if i > 0 && chars[i - 1] == '\\' {
        return None;
    }
    let mut j = i + 1;
    while j < chars.len() && chars[j] != '\n' {
        if chars[j] == '$' && chars[j - 1] != '\\' && chars.get(j + 1) != Some(&'$') {
            return Some(j + 1);
        }
        j += 1;
    }
    None
}

fn match_escaped_delim(
    chars: &[char],
    i: usize,
    open: char,
    close: char,
) -> Option<(usize, String)> {
    if chars.get(i) != Some(&'\\') || chars.get(i + 1) != Some(&open) {
        return None;
    }
    let mut j = i + 2;
    while j + 1 < chars.len() {
        if chars[j] == '\\' && chars[j + 1] == close {
            let body: String = chars[i + 2..j].iter().collect();
            if body.trim().is_empty() {
                return None;
            }
            return Some((j + 2, body));
        }
        j += 1;
    }
    None
}

fn match_equation_env(chars: &[char], i: usize) -> Option<(usize, String)> {
    const BEGIN: &str = "\\begin{equation";
    let begin: Vec<char> = BEGIN.chars().collect();
    if i + begin.len() > chars.len() || chars[i..i + begin.len()] != begin[..] {
        return None;
    }
    let mut j = i + begin.len();
    if chars.get(j) == Some(&'*') {
        j += 1;
    }
    if chars.get(j) != Some(&'}') {
        return None;
    }
    j += 1;
    let body_start = j;
    const END: &str = "\\end{equation";
    let end_tag: Vec<char> = END.chars().collect();
    while j + end_tag.len() <= chars.len() {
        if chars[j..j + end_tag.len()] == end_tag[..] {
            let mut k = j + end_tag.len();
            if chars.get(k) == Some(&'*') {
                k += 1;
            }
            if chars.get(k) == Some(&'}') {
                let body: String = chars[body_start..j].iter().collect();
                return Some((k + 1, body));
            }
        }
        j += 1;
    }
    None
}

fn match_bare_display_line(chars: &[char], i: usize) -> Option<(usize, String)> {
    if !line_start(chars, i) {
        return None;
    }
    let mut j = i;
    while j < chars.len() && chars[j] != '\n' {
        j += 1;
    }
    let line: String = chars[i..j].iter().collect();
    if !is_bare_display_line(&line) {
        return None;
    }
    let end = if chars.get(j) == Some(&'\n') {
        j + 1
    } else {
        j
    };
    Some((end, line.trim().to_string()))
}

fn is_bare_display_line(line: &str) -> bool {
    let trimmed = line.trim();
    if trimmed.len() < 3 || !trimmed.starts_with('\\') || !contains_latex_command(trimmed) {
        return false;
    }
    let chars: Vec<char> = trimmed.chars().collect();
    let mut residual = String::new();
    let mut i = 0usize;
    while i < chars.len() {
        if chars[i] == '\\' && i + 1 < chars.len() && chars[i + 1].is_ascii_alphabetic() {
            i += 2;
            while i < chars.len() && chars[i].is_ascii_alphabetic() {
                i += 1;
            }
            continue;
        }
        if chars[i] == '{' {
            let mut depth = 1usize;
            i += 1;
            while i < chars.len() && depth > 0 {
                if chars[i] == '{' {
                    depth += 1;
                } else if chars[i] == '}' {
                    depth -= 1;
                }
                i += 1;
            }
            continue;
        }
        residual.push(chars[i]);
        i += 1;
    }
    let mut run = 0usize;
    for c in residual.chars() {
        if c.is_ascii_alphabetic() {
            run += 1;
            if run >= 4 {
                return false;
            }
        } else {
            run = 0;
        }
    }
    true
}

fn contains_latex_command(text: &str) -> bool {
    let bytes = text.as_bytes();
    let mut i = 0usize;
    while i + 2 < bytes.len() {
        if bytes[i] == b'\\'
            && bytes[i + 1].is_ascii_alphabetic()
            && bytes[i + 2].is_ascii_alphabetic()
        {
            return true;
        }
        i += 1;
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn page5_route_formula_is_tagged_and_emoji_line_stays_prose() {
        let md = "<!-- edgequake-page:5 -->\nA proof route has the form\n\n$$\n\\Gamma_r \\vdash \\left( \\bigwedge_{o \\in \\text{Prem}(r)} q_o \\right) \\implies q_r. \\tag{1}\n$$\n\n(p') = Reconcile(\u{1F4DC}, \u{1F4C4}),\n\n\\[\nE = mc^2\n\\]\n\n\\begin{equation}\na+b\n\\end{equation}\n\n\\Gamma_r \\vdash q_r\n\nwhere \\(q_r\\) is the goal.\n";
        let out = canonicalize_math_markdown(md);
        assert!(
            out.contains("<equation id=\"eq-p5-1\" format=\"latex\">"),
            "{out}"
        );
        assert!(out.contains("\\Gamma_r \\vdash"), "{out}");
        assert!(out.contains("\\tag{1}"), "{out}");
        assert!(
            out.contains("<equation id=\"eq-p5-2\" format=\"latex\">"),
            "{out}"
        );
        assert!(
            out.contains("<equation id=\"eq-p5-3\" format=\"latex\">"),
            "{out}"
        );
        assert!(
            out.contains("<equation id=\"eq-p5-4\" format=\"latex\">"),
            "{out}"
        );
        assert!(!out.contains("<equation id=\"eq-p5-5\""), "{out}");
        assert!(out.contains("Reconcile"), "{out}");
        assert!(out.contains("$q_r$"), "{out}");
        assert!(!out.contains("\\begin{equation}"), "{out}");
        assert!(!out.contains("\\["), "{out}");
        let again = canonicalize_math_markdown(&out);
        assert_eq!(
            again.matches("<equation").count(),
            out.matches("<equation").count()
        );
    }

    #[test]
    fn prose_page_is_unchanged() {
        let md = "<!-- edgequake-page:1 -->\nHello world.\n";
        assert_eq!(canonicalize_math_markdown(md), md);
    }

    #[test]
    fn prose_command_in_a_sentence_is_not_a_display_line() {
        let md = "\\alpha is a scalar used in the proof.\n";
        assert_eq!(canonicalize_math_markdown(md), md);
    }
}
