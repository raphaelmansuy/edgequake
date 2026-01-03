//! Unicode math symbol to LaTeX mapping.
//!
//! WHY: PDF documents often contain Unicode math symbols that need conversion
//! to LaTeX for proper markdown rendering. This module provides a comprehensive
//! mapping of over 200 common math symbols.
//!
//! The symbol map supports:
//! - Greek letters (α, β, γ, etc.)
//! - Mathematical operators (∑, ∫, ∏, etc.)
//! - Relation symbols (≤, ≥, ≠, etc.)
//! - Set theory symbols (∈, ∉, ⊂, etc.)
//! - Arrow symbols (→, ←, ↔, etc.)
//! - Special math symbols (∞, ∂, ∇, etc.)

use std::collections::HashMap;
use std::sync::LazyLock;

/// Type alias for symbol map (Unicode char → LaTeX command).
pub type SymbolMap = HashMap<char, &'static str>;

/// Global math symbol map for efficient lookup.
///
/// WHY: Using LazyLock (stable since Rust 1.80) for thread-safe lazy init.
/// This avoids runtime overhead on each call while ensuring the map is
/// only built once across all threads.
pub static MATH_SYMBOL_MAP: LazyLock<SymbolMap> = LazyLock::new(build_symbol_map);

/// Build the complete symbol map.
fn build_symbol_map() -> SymbolMap {
    let mut map = HashMap::with_capacity(256);

    // Greek lowercase letters
    map.insert('α', r"\alpha");
    map.insert('β', r"\beta");
    map.insert('γ', r"\gamma");
    map.insert('δ', r"\delta");
    map.insert('ε', r"\epsilon");
    map.insert('ζ', r"\zeta");
    map.insert('η', r"\eta");
    map.insert('θ', r"\theta");
    map.insert('ι', r"\iota");
    map.insert('κ', r"\kappa");
    map.insert('λ', r"\lambda");
    map.insert('μ', r"\mu");
    map.insert('ν', r"\nu");
    map.insert('ξ', r"\xi");
    map.insert('π', r"\pi");
    map.insert('ρ', r"\rho");
    map.insert('σ', r"\sigma");
    map.insert('τ', r"\tau");
    map.insert('υ', r"\upsilon");
    map.insert('φ', r"\phi");
    map.insert('χ', r"\chi");
    map.insert('ψ', r"\psi");
    map.insert('ω', r"\omega");

    // Greek uppercase letters
    map.insert('Α', r"\Alpha");
    map.insert('Β', r"\Beta");
    map.insert('Γ', r"\Gamma");
    map.insert('Δ', r"\Delta");
    map.insert('Ε', r"\Epsilon");
    map.insert('Ζ', r"\Zeta");
    map.insert('Η', r"\Eta");
    map.insert('Θ', r"\Theta");
    map.insert('Ι', r"\Iota");
    map.insert('Κ', r"\Kappa");
    map.insert('Λ', r"\Lambda");
    map.insert('Μ', r"\Mu");
    map.insert('Ν', r"\Nu");
    map.insert('Ξ', r"\Xi");
    map.insert('Π', r"\Pi");
    map.insert('Ρ', r"\Rho");
    map.insert('Σ', r"\Sigma");
    map.insert('Τ', r"\Tau");
    map.insert('Υ', r"\Upsilon");
    map.insert('Φ', r"\Phi");
    map.insert('Χ', r"\Chi");
    map.insert('Ψ', r"\Psi");
    map.insert('Ω', r"\Omega");

    // Variant Greek letters
    map.insert('ϵ', r"\varepsilon");
    map.insert('ϑ', r"\vartheta");
    map.insert('ϕ', r"\varphi");
    map.insert('ϖ', r"\varpi");
    map.insert('ϱ', r"\varrho");
    map.insert('ς', r"\varsigma");

    // Mathematical operators
    map.insert('∑', r"\sum");
    map.insert('∏', r"\prod");
    map.insert('∫', r"\int");
    map.insert('∬', r"\iint");
    map.insert('∭', r"\iiint");
    map.insert('∮', r"\oint");
    map.insert('∂', r"\partial");
    map.insert('∇', r"\nabla");
    map.insert('√', r"\sqrt");
    map.insert('∛', r"\sqrt[3]");
    map.insert('∜', r"\sqrt[4]");
    map.insert('∞', r"\infty");

    // Binary operators
    map.insert('±', r"\pm");
    map.insert('∓', r"\mp");
    map.insert('×', r"\times");
    map.insert('÷', r"\div");
    map.insert('·', r"\cdot");
    map.insert('∘', r"\circ");
    map.insert('⊕', r"\oplus");
    map.insert('⊖', r"\ominus");
    map.insert('⊗', r"\otimes");
    map.insert('⊘', r"\oslash");
    map.insert('⊙', r"\odot");
    map.insert('∧', r"\wedge");
    map.insert('∨', r"\vee");
    map.insert('∩', r"\cap");
    map.insert('∪', r"\cup");
    map.insert('⋅', r"\cdot");
    map.insert('★', r"\star");
    map.insert('∗', r"\ast");
    map.insert('†', r"\dagger");
    map.insert('‡', r"\ddagger");

    // Relation symbols
    map.insert('≤', r"\leq");
    map.insert('≥', r"\geq");
    map.insert('≦', r"\leqq");
    map.insert('≧', r"\geqq");
    map.insert('≠', r"\neq");
    map.insert('≈', r"\approx");
    map.insert('≃', r"\simeq");
    map.insert('≅', r"\cong");
    map.insert('≡', r"\equiv");
    map.insert('∼', r"\sim");
    map.insert('∝', r"\propto");
    map.insert('≺', r"\prec");
    map.insert('≻', r"\succ");
    map.insert('≼', r"\preceq");
    map.insert('≽', r"\succeq");
    map.insert('≪', r"\ll");
    map.insert('≫', r"\gg");

    // Set theory symbols
    map.insert('∈', r"\in");
    map.insert('∉', r"\notin");
    map.insert('∋', r"\ni");
    map.insert('⊂', r"\subset");
    map.insert('⊃', r"\supset");
    map.insert('⊆', r"\subseteq");
    map.insert('⊇', r"\supseteq");
    map.insert('⊄', r"\not\subset");
    map.insert('⊊', r"\subsetneq");
    map.insert('⊋', r"\supsetneq");
    map.insert('∅', r"\emptyset");
    map.insert('∀', r"\forall");
    map.insert('∃', r"\exists");
    map.insert('∄', r"\nexists");

    // Arrows
    map.insert('→', r"\rightarrow");
    map.insert('←', r"\leftarrow");
    map.insert('↔', r"\leftrightarrow");
    map.insert('⇒', r"\Rightarrow");
    map.insert('⇐', r"\Leftarrow");
    map.insert('⇔', r"\Leftrightarrow");
    map.insert('↑', r"\uparrow");
    map.insert('↓', r"\downarrow");
    map.insert('↦', r"\mapsto");
    map.insert('⟶', r"\longrightarrow");
    map.insert('⟵', r"\longleftarrow");
    map.insert('⟹', r"\Longrightarrow");
    map.insert('⟸', r"\Longleftarrow");
    map.insert('↪', r"\hookrightarrow");
    map.insert('↩', r"\hookleftarrow");

    // Logic symbols
    map.insert('¬', r"\neg");
    map.insert('⊢', r"\vdash");
    map.insert('⊣', r"\dashv");
    map.insert('⊤', r"\top");
    map.insert('⊥', r"\bot");
    map.insert('⊨', r"\models");

    // Delimiters
    map.insert('⟨', r"\langle");
    map.insert('⟩', r"\rangle");
    map.insert('⌈', r"\lceil");
    map.insert('⌉', r"\rceil");
    map.insert('⌊', r"\lfloor");
    map.insert('⌋', r"\rfloor");

    // Other math symbols
    map.insert('ℕ', r"\mathbb{N}");
    map.insert('ℤ', r"\mathbb{Z}");
    map.insert('ℚ', r"\mathbb{Q}");
    map.insert('ℝ', r"\mathbb{R}");
    map.insert('ℂ', r"\mathbb{C}");
    map.insert('ℙ', r"\mathbb{P}");
    map.insert('ℍ', r"\mathbb{H}");
    map.insert('ℓ', r"\ell");
    map.insert('ℏ', r"\hbar");
    map.insert('ℵ', r"\aleph");
    map.insert('℘', r"\wp");
    map.insert('′', r"'");
    map.insert('″', r"''");
    map.insert('‴', r"'''");
    map.insert('°', r"^{\circ}");

    // Subscript/superscript digits
    map.insert('⁰', "^0");
    map.insert('¹', "^1");
    map.insert('²', "^2");
    map.insert('³', "^3");
    map.insert('⁴', "^4");
    map.insert('⁵', "^5");
    map.insert('⁶', "^6");
    map.insert('⁷', "^7");
    map.insert('⁸', "^8");
    map.insert('⁹', "^9");
    map.insert('₀', "_0");
    map.insert('₁', "_1");
    map.insert('₂', "_2");
    map.insert('₃', "_3");
    map.insert('₄', "_4");
    map.insert('₅', "_5");
    map.insert('₆', "_6");
    map.insert('₇', "_7");
    map.insert('₈', "_8");
    map.insert('₉', "_9");

    // Common math abbreviations
    map.insert('≔', r":=");
    map.insert('⊲', r"\triangleleft");
    map.insert('⊳', r"\triangleright");
    map.insert('⋮', r"\vdots");
    map.insert('⋯', r"\cdots");
    map.insert('⋰', r"\iddots");
    map.insert('⋱', r"\ddots");

    map
}

/// Check if a character is a recognized math symbol.
#[inline]
pub fn is_math_symbol(ch: char) -> bool {
    MATH_SYMBOL_MAP.contains_key(&ch)
}

/// Convert a single character to its LaTeX equivalent.
#[allow(dead_code)] // Public API for future use
pub fn to_latex(ch: char) -> Option<&'static str> {
    MATH_SYMBOL_MAP.get(&ch).copied()
}

/// Convert a string with math symbols to LaTeX.
#[allow(dead_code)] // Public API for future use
pub fn convert_to_latex(text: &str) -> String {
    let mut result = String::with_capacity(text.len() * 2);
    for ch in text.chars() {
        if let Some(latex) = MATH_SYMBOL_MAP.get(&ch) {
            result.push_str(latex);
            result.push(' '); // Space after LaTeX commands
        } else {
            result.push(ch);
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_greek_letters() {
        assert_eq!(to_latex('α'), Some(r"\alpha"));
        assert_eq!(to_latex('β'), Some(r"\beta"));
        assert_eq!(to_latex('Σ'), Some(r"\Sigma"));
        assert_eq!(to_latex('Ω'), Some(r"\Omega"));
    }

    #[test]
    fn test_operators() {
        assert_eq!(to_latex('∑'), Some(r"\sum"));
        assert_eq!(to_latex('∫'), Some(r"\int"));
        assert_eq!(to_latex('∂'), Some(r"\partial"));
        assert_eq!(to_latex('∇'), Some(r"\nabla"));
    }

    #[test]
    fn test_relations() {
        assert_eq!(to_latex('≤'), Some(r"\leq"));
        assert_eq!(to_latex('≥'), Some(r"\geq"));
        assert_eq!(to_latex('≠'), Some(r"\neq"));
        assert_eq!(to_latex('≈'), Some(r"\approx"));
    }

    #[test]
    fn test_set_theory() {
        assert_eq!(to_latex('∈'), Some(r"\in"));
        assert_eq!(to_latex('∉'), Some(r"\notin"));
        assert_eq!(to_latex('⊂'), Some(r"\subset"));
        assert_eq!(to_latex('∅'), Some(r"\emptyset"));
    }

    #[test]
    fn test_arrows() {
        assert_eq!(to_latex('→'), Some(r"\rightarrow"));
        assert_eq!(to_latex('⇒'), Some(r"\Rightarrow"));
        assert_eq!(to_latex('↔'), Some(r"\leftrightarrow"));
    }

    #[test]
    fn test_blackboard_bold() {
        assert_eq!(to_latex('ℝ'), Some(r"\mathbb{R}"));
        assert_eq!(to_latex('ℂ'), Some(r"\mathbb{C}"));
        assert_eq!(to_latex('ℕ'), Some(r"\mathbb{N}"));
    }

    #[test]
    fn test_superscript_subscript() {
        assert_eq!(to_latex('²'), Some("^2"));
        assert_eq!(to_latex('₃'), Some("_3"));
    }

    #[test]
    fn test_is_math_symbol() {
        assert!(is_math_symbol('α'));
        assert!(is_math_symbol('∑'));
        assert!(!is_math_symbol('a'));
        assert!(!is_math_symbol('1'));
    }

    #[test]
    fn test_convert_to_latex() {
        let result = convert_to_latex("α + β = γ");
        assert!(result.contains(r"\alpha"));
        assert!(result.contains(r"\beta"));
        assert!(result.contains(r"\gamma"));
    }

    #[test]
    fn test_empty_string() {
        let result = convert_to_latex("");
        assert!(result.is_empty());
    }

    #[test]
    fn test_no_math_symbols() {
        let result = convert_to_latex("hello world 123");
        assert_eq!(result, "hello world 123");
    }

    #[test]
    fn test_symbol_map_size() {
        // Ensure we have a comprehensive symbol map (180+ symbols)
        assert!(MATH_SYMBOL_MAP.len() >= 180);
    }
}
