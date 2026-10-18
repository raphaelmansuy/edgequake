//! Math spans are notation, not prose.
//!
//! Answer models sometimes re-serialize `$$...$$` as `\[...\]` or as a bare
//! `[` / `]` block. Citation rewrite only understands numeric `[N]`. This module
//! restores KaTeX delimiters before that rewrite, and tells the rewrite which
//! spans to leave untouched.

/// Restore display/inline delimiters the answer renderer understands.
///
/// Existing `$` / `$$` spans and fenced code are copied through. `\[...\]`
/// becomes a `$$` block, `\(...\)` becomes `$...$`, and a multi-line `[` / `]`
/// block whose body contains a LaTeX command becomes a `$$` block. Numeric
/// `[N]` citations and markdown links are left alone.
pub fn normalize_math_spans(input: &str) -> String {
    if input.is_empty() {
        return String::new();
    }
    let chars: Vec<char> = input.chars().collect();
    let mut out = String::with_capacity(input.len() + 16);
    let mut i = 0usize;
    while i < chars.len() {
        if starts_with_fence(&chars, i) {
            i = copy_fence(&chars, i, &mut out);
            continue;
        }
        if let Some(end) = existing_math_end(&chars, i) {
            push_slice(&chars, i, end, &mut out);
            i = end;
            continue;
        }
        if let Some((end, body)) = match_escaped_delim(&chars, i, '[', ']') {
            if contains_latex_command(&body) {
                push_display(&mut out, &body, "");
                i = end;
                continue;
            }
        }
        if let Some((end, body)) = match_escaped_delim(&chars, i, '(', ')') {
            if !body.is_empty() {
                out.push('$');
                out.push_str(body.trim());
                out.push('$');
                i = end;
                continue;
            }
        }
        if let Some((end, body, indent)) = match_bare_bracket_block(&chars, i) {
            push_display(&mut out, &body, &indent);
            i = end;
            continue;
        }
        out.push(chars[i]);
        i += 1;
    }
    out
}

/// Exclusive end of a math span starting at `i`, when one is present.
///
/// Used by citation rewrite so `[N]` and prose page scrub never see inside
/// `$`, `$$`, `\(`, or `\[`.
pub fn math_span_end(chars: &[char], i: usize) -> Option<usize> {
    if escaped(chars, i) {
        return None;
    }
    existing_math_end(chars, i).or_else(|| {
        match_escaped_delim(chars, i, '[', ']')
            .map(|(end, _)| end)
            .or_else(|| match_escaped_delim(chars, i, '(', ')').map(|(end, _)| end))
    })
}

fn push_display(out: &mut String, body: &str, indent: &str) {
    out.push_str(indent);
    out.push_str("$$\n");
    out.push_str(body.trim());
    out.push('\n');
    out.push_str(indent);
    out.push_str("$$\n");
}

fn push_slice(chars: &[char], start: usize, end: usize, out: &mut String) {
    for c in &chars[start..end] {
        out.push(*c);
    }
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

fn escaped(chars: &[char], i: usize) -> bool {
    i > 0 && chars[i - 1] == '\\'
}

fn line_start(chars: &[char], i: usize) -> bool {
    i == 0 || chars[i - 1] == '\n'
}

/// `$...$` on one line, or `$$...$$` across lines. Unclosed dollars are not a span.
fn existing_math_end(chars: &[char], i: usize) -> Option<usize> {
    if chars.get(i) != Some(&'$') || escaped(chars, i) {
        return None;
    }
    let display = chars.get(i + 1) == Some(&'$');
    let mut j = if display { i + 2 } else { i + 1 };
    while j < chars.len() {
        if !display && chars[j] == '\n' {
            return None;
        }
        if chars[j] == '$' && !escaped(chars, j) {
            if display {
                if chars.get(j + 1) == Some(&'$') {
                    return Some(j + 2);
                }
            } else if chars.get(j + 1) != Some(&'$') {
                return Some(j + 1);
            }
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

/// Multi-line block whose first line is `[` and whose close line is `]`.
fn match_bare_bracket_block(chars: &[char], i: usize) -> Option<(usize, String, String)> {
    if !line_start(chars, i) {
        return None;
    }
    let mut j = i;
    while j < chars.len() && (chars[j] == ' ' || chars[j] == '\t') {
        j += 1;
    }
    let indent: String = chars[i..j].iter().collect();
    if chars.get(j) != Some(&'[') {
        return None;
    }
    let mut k = j + 1;
    while k < chars.len() && (chars[k] == ' ' || chars[k] == '\t') {
        k += 1;
    }
    if chars.get(k) != Some(&'\n') {
        return None;
    }
    let body_start = k + 1;
    let mut p = body_start;
    while p < chars.len() {
        if line_start(chars, p) {
            let mut q = p;
            while q < chars.len() && (chars[q] == ' ' || chars[q] == '\t') {
                q += 1;
            }
            if chars.get(q) == Some(&']') {
                let mut r = q + 1;
                while r < chars.len() && (chars[r] == ' ' || chars[r] == '\t') {
                    r += 1;
                }
                if r == chars.len() || chars[r] == '\n' {
                    let body: String = chars[body_start..p].iter().collect();
                    let trimmed = body.trim();
                    if trimmed.is_empty()
                        || !contains_latex_command(trimmed)
                        || trimmed.contains("](")
                    {
                        return None;
                    }
                    let end = if r < chars.len() && chars[r] == '\n' {
                        r + 1
                    } else {
                        r
                    };
                    return Some((end, trimmed.to_string(), indent));
                }
            }
        }
        p += 1;
    }
    None
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
    fn bare_bracket_euler_formula_becomes_display_math() {
        let raw = "- such as:\n  [\n  \\Gamma_r \\vdash \\left( \\bigwedge_{o \\in \\text{Prem}(r)} q_o \\right) \\implies q_r,\n  ]\n";
        let out = normalize_math_spans(raw);
        assert!(out.contains("$$"), "got: {out}");
        assert!(out.contains("\\Gamma_r \\vdash"), "got: {out}");
        assert!(out.contains("\\implies"), "got: {out}");
        assert!(!out.contains("\n  [\n"), "bare brackets remain: {out}");
    }

    #[test]
    fn existing_inline_math_is_unchanged() {
        let raw = "where $q_r$ is the goal and $q_o$ are the premises";
        assert_eq!(normalize_math_spans(raw), raw);
    }

    #[test]
    fn citations_and_links_are_not_math() {
        assert_eq!(
            normalize_math_spans("See [1] and [1, 2]."),
            "See [1] and [1, 2]."
        );
        assert_eq!(
            normalize_math_spans("Read [notes](https://example.test)."),
            "Read [notes](https://example.test)."
        );
    }

    #[test]
    fn backslash_delimiters_become_dollars() {
        let out = normalize_math_spans("Inline \\(q_r\\) and\n\\[\n\\alpha + \\beta\n\\]\n");
        assert!(out.contains("$q_r$"), "got: {out}");
        assert!(out.contains("$$"), "got: {out}");
        assert!(out.contains("\\alpha + \\beta"), "got: {out}");
        assert!(!out.contains("\\["), "got: {out}");
    }

    #[test]
    fn fenced_code_is_not_rewritten() {
        let raw = "```\n\\[\n\\alpha\n\\]\n```\n";
        assert_eq!(normalize_math_spans(raw), raw);
    }
}
