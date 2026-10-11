//! Grounding instructions for RAG answer generation (SPEC-047 Q1.2 / 020 A1).
//!
//! # SOLID
//!
//! - **SRP:** Prompt grounding policy only — no retrieval or truncation.
//! - **DRY:** Shared by text and vision prompt builders via [`grounding_instructions`].
//! - **OCP:** Policy text evolves here; callers stay unchanged.
//!
//! # Policy (020 Q11–Q12)
//!
//! Selective refusal ≠ maximal refusal:
//! - **Answer + cite** when a Document Chunk / KG fact supports the asked claim.
//! - **Refuse** only when no supporting evidence is in context.
//! - Prefer partial grounded answers over "Not answerable".
//! - Never ban honest refusal (019 Q8).

/// Instructions injected into system/text prompts so the LLM uses page/modality
/// headers and citation markers from [`crate::context_format`].
///
/// Calibrated (020 A1): entailment-first — answer when evidence supports;
/// refuse only when it does not.
///
/// W3-arith (026 / 032): when context explicitly states both a percentage and a
/// sample size that determine a count, composing `round(pct/100 * N)` is
/// grounded arithmetic — not external knowledge.
pub fn grounding_instructions() -> &'static str {
    r#"2b. Citations & Page Grounding (SPEC-142):
  - Document chunks are labeled `[N] (score: …) page=P doc="Title" modality=…` when available.
  - Prefer facts from chunks whose `page=` matches the question's likely evidence pages.
  - When a Document Chunk or Knowledge Graph fact SUPPORTS the asked claim, answer it and cite the supporting chunk as [N]. Do NOT refuse merely because the wording is imperfect or the answer is partial.
  - Prefer a partial answer that quotes what IS in context (with [N]) over "Not answerable".
  - Refuse with "Not answerable" / insufficient evidence ONLY when no Document Chunk and no Knowledge Graph fact supports the asked claim. Do NOT invent values from general knowledge.
  - When stating a concrete fact (number, name, date), cite the supporting chunk as [N].
  - Citation format: emit ONLY bracket indices like [1] or [1][2] after factual claims. Do NOT write page numbers, filenames, document titles, markdown links, or URLs — the system attaches verified document name + page links from retrieval metadata.
  - Do NOT invent [N] outside the list of Document Chunk headers provided in Context.
  - Few-shot shapes: (good) short fact then [1]; (multi) fact then [1][2]; (refusal) "Not answerable" with no [N].
  - Grounded arithmetic (W3-arith): if the question asks how many / a headcount / a count of people and the Context explicitly states BOTH (a) a percentage/rate and (b) a sample size N (e.g. "1,503 adults", "n=710") that together determine that count, you MUST compute count = round(percentage/100 × N), answer with that short integer (not the percentage), and cite the chunks that supplied the percentage and N. Worked example: Context has "Not good" = 36% and sample "1,503 adults" → answer 541 (not 36 or 36%). Do NOT invent missing percentages or sample sizes. Do NOT refuse merely because the count is not printed as a literal integer when both operands are present."#
}

/// 082: Acc `answer_style=gold` forbids citation markers — detect that extension.
pub fn is_gold_answer_extension(ext: Option<&str>) -> bool {
    let Some(raw) = ext.map(str::trim).filter(|s| !s.is_empty()) else {
        return false;
    };
    let lower = raw.to_ascii_lowercase();
    lower.contains("do not append citation markers")
        || lower.contains("plain answer text only")
        || (lower.contains("graphrag-bench") && lower.contains("accuracy scoring"))
}

/// Product path keeps citation mandates; Acc gold path uses entailment without `[N]`.
pub fn grounding_instructions_for(ext: Option<&str>) -> &'static str {
    if is_gold_answer_extension(ext) {
        grounding_instructions_gold_compat()
    } else {
        grounding_instructions()
    }
}

/// Entailment + arithmetic without citation-marker mandates (082 G1 / Acc gold).
pub fn grounding_instructions_gold_compat() -> &'static str {
    r#"2b. Evidence Grounding (gold-compatible — no citation markers):
  - Prefer facts from Document Chunks whose content matches the question's evidence need (page= labels are hints only).
  - When a Document Chunk or Knowledge Graph fact SUPPORTS the asked claim, answer it. Do NOT refuse merely because the wording is imperfect or the answer is partial.
  - Prefer a partial answer that states what IS in context over "Not answerable".
  - Refuse with "Not answerable" / insufficient evidence ONLY when no Document Chunk and no Knowledge Graph fact supports the asked claim. Do NOT invent values from general knowledge.
  - Do NOT append citation markers, chunk ids, brackets like [1] or [16], "(source)", or a References section — plain answer text only.
  - Grounded arithmetic (W3-arith): if the question asks how many / a headcount / a count of people and the Context explicitly states BOTH (a) a percentage/rate and (b) a sample size N (e.g. "1,503 adults", "n=710") that together determine that count, you MUST compute count = round(percentage/100 × N), answer with that short integer (not the percentage). Worked example: Context has "Not good" = 36% and sample "1,503 adults" → answer 541 (not 36 or 36%). Do NOT invent missing percentages or sample sizes. Do NOT refuse merely because the count is not printed as a literal integer when both operands are present."#
}

/// Strip trailing `### References` blocks and inline `[N]` markers for Acc gold F1.
pub fn strip_gold_citation_artifacts(answer: &str) -> String {
    let mut text = answer.trim().to_string();
    if text.is_empty() {
        return text;
    }
    // Drop References section (LR-style or accidental).
    for marker in [
        "### References",
        "## References",
        "### references",
        "## references",
    ] {
        if let Some(idx) = text.find(marker) {
            text.truncate(idx);
            text = text.trim_end().to_string();
            break;
        }
    }
    // Remove bracket citation tokens like [1], [16], [N].
    let mut out = String::with_capacity(text.len());
    let chars: Vec<char> = text.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        if chars[i] == '[' {
            let mut j = i + 1;
            let mut digits = 0usize;
            while j < chars.len() && chars[j].is_ascii_digit() {
                digits += 1;
                j += 1;
            }
            if digits > 0 && j < chars.len() && chars[j] == ']' {
                i = j + 1;
                // Drop a single trailing space after the marker if present.
                if i < chars.len() && chars[i] == ' ' {
                    i += 1;
                }
                continue;
            }
        }
        out.push(chars[i]);
        i += 1;
    }
    // Collapse leftover double spaces from removals.
    out.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// 081 F4: true when `answer` contains a contiguous 3-token span also present in
/// admitted Document Chunks (or entity name/description).
pub fn answer_has_context_span(answer: &str, context_corpus: &str) -> bool {
    let ans = normalize_span_text(answer);
    let corpus = normalize_span_text(context_corpus);
    if ans.is_empty() || corpus.is_empty() {
        return false;
    }
    let words: Vec<&str> = ans
        .split_whitespace()
        .filter(|w| w.chars().count() >= 2)
        .collect();
    // Very short answers: treat as grounded if any content token hits corpus.
    if words.len() < 3 {
        return words.iter().any(|w| corpus.contains(w));
    }
    words
        .windows(3)
        .map(|w| w.join(" "))
        .any(|span| corpus.contains(&span))
}

/// Fraction of answer content tokens (len≥4) that appear in the context corpus.
/// Used as a soft generation-miss detector — mirrors F1 membership threshold 0.15.
pub fn answer_context_token_coverage(answer: &str, context_corpus: &str) -> f32 {
    let ans = normalize_span_text(answer);
    let corpus = normalize_span_text(context_corpus);
    if ans.is_empty() || corpus.is_empty() {
        return 0.0;
    }
    let tokens: Vec<&str> = ans
        .split_whitespace()
        .filter(|w| w.chars().count() >= 4)
        .collect();
    if tokens.is_empty() {
        return 1.0;
    }
    let hits = tokens.iter().filter(|t| corpus.contains(*t)).count();
    hits as f32 / tokens.len() as f32
}

/// Retry when coverage is below the F1 membership cut — gold-in-context but
/// answer barely touches admitted text. Paraphrase-heavy answers usually clear this.
pub fn needs_groundedness_retry(answer: &str, context_corpus: &str) -> bool {
    const MIN_COVERAGE: f32 = 0.15;
    answer_context_token_coverage(answer, context_corpus) < MIN_COVERAGE
}

/// `EDGEQUAKE_ANSWER_GROUNDED_RETRY=1` enables the 081 F4 post-generate retry.
/// Default off after medical-mid Acc regression vs E2-B5 (`T022412Z`).
pub fn grounded_retry_enabled() -> bool {
    matches!(
        std::env::var("EDGEQUAKE_ANSWER_GROUNDED_RETRY")
            .unwrap_or_default()
            .trim()
            .to_ascii_lowercase()
            .as_str(),
        "1" | "true" | "yes" | "on"
    )
}

fn normalize_span_text(s: &str) -> String {
    s.chars()
        .map(|c| {
            if c.is_alphanumeric() || c.is_whitespace() {
                c
            } else {
                ' '
            }
        })
        .collect::<String>()
        .to_lowercase()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

/// True if policy text still allows honest "Not answerable" (019 Q8 / 020 reject ban).
pub fn allows_honest_refusal(instructions: &str) -> bool {
    let lower = instructions.to_lowercase();
    (lower.contains("not answerable") || lower.contains("insufficient"))
        && !lower.contains("never say not answerable")
}

/// True if policy is entailment-first (answer when evidence supports) — 020 A1.
pub fn is_entailment_first(instructions: &str) -> bool {
    let lower = instructions.to_lowercase();
    lower.contains("supports the asked claim")
        && lower.contains("prefer a partial answer")
        && lower.contains("only when no")
}

/// True if policy permits grounded %×N composition (026 / 032 W3-arith).
pub fn allows_grounded_arithmetic(instructions: &str) -> bool {
    let lower = instructions.to_lowercase();
    lower.contains("grounded arithmetic")
        && (lower.contains("sample size") || lower.contains("percentage"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn grounding_mentions_page_and_refusal() {
        let g = grounding_instructions();
        assert!(g.contains("page="));
        assert!(g.contains("[N]"));
        assert!(g.contains("Do NOT write page numbers"));
        assert!(allows_honest_refusal(g));
    }

    #[test]
    fn grounding_is_entailment_first_calibrated() {
        let g = grounding_instructions();
        assert!(
            is_entailment_first(g),
            "020 A1 requires answer-when-supported / refuse-only-when-absent; got:\n{g}"
        );
        assert!(allows_honest_refusal(g));
    }

    #[test]
    fn grounding_allows_w3_arith() {
        let g = grounding_instructions();
        assert!(
            allows_grounded_arithmetic(g),
            "032 W3-arith requires grounded %×N composition; got:\n{g}"
        );
        assert!(
            g.contains("541") && g.to_lowercase().contains("must compute"),
            "W3-arith-v2 requires worked example + MUST compute; got:\n{g}"
        );
        assert!(allows_honest_refusal(g));
    }

    #[test]
    fn grounding_normalizes_unicode_case() {
        assert!(answer_has_context_span("ÉCOLE À PARIS", "école à paris"));
        assert_eq!(
            answer_context_token_coverage("CAFÉ ÉCOLE", "café école"),
            1.0
        );
        assert!(!needs_groundedness_retry("CAFÉ ÉCOLE", "café école"));
        assert!(answer_has_context_span("ΣΧΟΛΕΊΟ", "σχολείο"));
        assert_eq!(normalize_span_text("İSTANBUL!"), "i\u{307}stanbul");
    }

    #[test]
    fn answer_span_detects_shared_trigram() {
        let corpus = "Patients with stage IIIA non-small cell lung cancer received cisplatin.";
        assert!(answer_has_context_span(
            "Guidelines mention stage IIIA non-small cell lung cancer patients.",
            corpus
        ));
        assert!(!answer_has_context_span(
            "Treatment relies on immunotherapy alone without naming agents.",
            corpus
        ));
    }

    #[test]
    fn coverage_retry_triggers_only_on_ungrounded_answers() {
        let corpus =
            "Patients with stage IIIA non-small cell lung cancer received cisplatin chemotherapy.";
        assert!(!needs_groundedness_retry(
            "Stage IIIA non-small cell lung cancer received cisplatin chemotherapy.",
            corpus
        ));
        assert!(needs_groundedness_retry(
            "Something completely unrelated about weather patterns today.",
            corpus
        ));
    }

    #[test]
    fn gold_extension_detects_acc_gold_rules() {
        assert!(is_gold_answer_extension(Some(
            "You are answering a GraphRAG-Bench medical question for accuracy scoring.\n\
             7) Do NOT append citation markers, chunk ids, or brackets like [1], [16]"
        )));
        assert!(!is_gold_answer_extension(Some(
            "Be helpful and cite sources."
        )));
        assert!(!is_gold_answer_extension(None));
    }

    #[test]
    fn gold_compat_grounding_keeps_entailment_without_cite_mandate() {
        let g = grounding_instructions_gold_compat();
        assert!(is_entailment_first(g));
        assert!(allows_grounded_arithmetic(g));
        assert!(allows_honest_refusal(g));
        assert!(g.to_lowercase().contains("plain answer text only"));
        assert!(
            !g.contains("cite the supporting chunk as [N]"),
            "gold compat must not mandate [N] citations"
        );
        assert_eq!(
            grounding_instructions_for(Some("Do NOT append citation markers")),
            grounding_instructions_gold_compat()
        );
    }

    #[test]
    fn strip_gold_citations_removes_refs_and_brackets() {
        let raw = "Cisplatin is first-line [3] for stage IIIA.\n\n### References\n* [3] Doc";
        let cleaned = strip_gold_citation_artifacts(raw);
        assert_eq!(cleaned, "Cisplatin is first-line for stage IIIA.");
        assert!(!cleaned.contains('['));
        assert!(!cleaned.to_lowercase().contains("references"));
    }
}
