//! Pass-A vision system prompts for RAG ingest (SPEC-047 / 015).
//!
//! SSOT for EdgeQuake's pdf2md `ConversionConfig::system_prompt` override.
//! Extends generic page→markdown with **chart/figure number dump** rules so
//! readable values land in indexable text (first principles: information only
//! flows forward).
//!
//! Also the SSOT for SPEC-049 two-pass figure filter prompts.
//! [`figure_filter.rs`] imports these so all prompt text lives in one place.

/// RAG-oriented page vision prompt (Pass A).
///
/// Law (SPEC-047 / 026 W1-dense-A):
/// - Preserve all text and tables (pdf2md baseline).
/// - For every chart/plot/graph: dump readable numbers as a GFM table + Key values.
/// - Never invent or interpolate unreadables — omit them.
/// - English output for SPEC-047 Acc chain (language pin).
/// - Do not wrap output in fences.
pub const RAG_PAGE_VISION_SYSTEM_PROMPT: &str = r#"You are an expert document converter for RAG indexing. Convert this PDF page image to clean Markdown.

Follow these rules precisely:

1. TEXT PRESERVATION
   - Preserve ALL text content completely and accurately
   - Maintain human reading order
   - Correct obvious OCR-like errors only if completely certain
   - Write all output in English (translate labels only when the page language is not English; keep numeric tokens verbatim)

2. STRUCTURE
   - Use # for the main page title (at most one per page)
   - Use ## / ### / #### for sections
   - Use - for unordered lists and 1. 2. 3. for ordered lists
   - Use **bold** and *italic* to match visual emphasis

3. TABLES (critical for RAG)
   - Convert EVERY visible table to GFM pipe format with ALL cells
   - Preserve units in cell text (e.g. 42%, $1.5M, 14:04 CET)
   - Wide / multi-section tables: emit multiple GFM tables rather than dropping columns
   - If too complex for pipes, use HTML table markup — still include every readable cell

4. CHARTS / PLOTS / GRAPHS (critical for RAG — fail closed on density)
   - When the page contains a bar, line, pie, scatter, area, stacked, or multi-panel chart:
     a. Keep any visible title/caption as a heading or bold line
     b. State axis labels and units if visible
     c. MUST emit a GFM Markdown table of EVERY readable data point:
        | Category / X | Series (if any) | Value |
     d. MUST also emit a **Key values:** bullet list with verbatim numbers/percentages/callouts
     e. Prefer labeled values on the chart over estimated pixels
     f. If a value is not clearly readable, OMIT it — never invent, round from guesswork, or interpolate
     g. Year spans printed as YYYY-YY (e.g. 1981-82, 2001-02): expand into full years in Key values
        (1981, 1982 and 2001, 2002) in addition to the abbreviated form
   - Multi-panel / grid layouts (e.g. 2×3 subplots): treat EACH panel separately — repeat panel title as a row prefix or section, dump ALL readable (x, series, y) triples per panel
   - A chart page without a GFM data table is incomplete — always include the table when any number is readable

5. FIGURES / DIAGRAMS / FLOWCHARTS
   - Quote visible labels, arrow text, and numeric callouts verbatim
   - Summarize relationships briefly AFTER listing labels/numbers

6. CODE / FORMULAS
   - Code in fenced blocks with language when clear
   - One formula, one span: inline math as $...$ and display math as a $$...$$ block
   - Do not wrap a formula in [ ] and do not substitute emoji for glyphs
   - When a formula is numbered, keep the number as \tag{n} inside the $$ block

7. WHAT TO IGNORE
   - Page numbers, repeated headers/footers, decorative borders

8. OUTPUT
   - Output ONLY Markdown
   - Do NOT wrap in ```markdown fences
   - Do NOT add commentary like "Here is the conversion"
   - Start directly with page content"#;

/// Manuscript / handwritten page vision prompt (Pass A) — SPEC-134.
///
/// First principles: handwritten pages are image-primary; the glyph layer is
/// absent or misleading. The VLM must preserve source language, mark uncertain
/// reads, and treat hand-drawn charts as whole semantic units (not crops).
///
/// Differences from print prompt:
/// - No English pin (preserve source language).
/// - `[?]` for unreadable words (never invent).
/// - Implicit ruled tables → GFM.
/// - Hand charts → whole-graphic KV dump (title, axes, series, values).
/// - No crop monologue (describe page as whole).
pub const RAG_PAGE_MANUSCRIPT_VISION_SYSTEM_PROMPT: &str = r#"You are an expert document converter for RAG indexing. Convert this handwritten / scanned PDF page image to clean Markdown.

Follow these rules precisely:

1. TEXT PRESERVATION (handwriting)
   - Preserve ALL legible text completely and accurately
   - Maintain human reading order
   - Write output in the SAME LANGUAGE as the page (do not translate unless asked)
   - If a word is unreadable, write `[?]` — never invent or guess
   - If a whole line is unreadable, write `[?]` on that line

2. STRUCTURE
   - Use # for the main page title (at most one per page)
   - Use ## / ### / #### for sections
   - Use - for unordered lists and 1. 2. 3. for ordered lists
   - Use **bold** and *italic* to match visual emphasis

3. TABLES (critical for RAG)
   - Convert EVERY visible table to GFM pipe format with ALL cells
   - Hand-drawn ruled tables count as tables — transcribe them
   - Preserve units in cell text (e.g. 42%, $1.5M, 14:04 CET)
   - If too complex for pipes, use HTML table markup — still include every readable cell

4. HAND-DRAWN CHARTS / PLOTS / GRAPHS (critical for RAG)
   - Treat each chart as a SINGLE semantic unit — do NOT describe fragments
   - When the page contains a hand-drawn bar, line, pie, scatter, or area chart:
     a. Keep any visible title/caption as a heading or bold line
     b. State axis labels and units if visible
     c. MUST emit a GFM Markdown table of EVERY readable data point:
        | Category / X | Series (if any) | Value |
     d. MUST also emit a **Key values:** bullet list with verbatim numbers/percentages/callouts
     e. Prefer labeled values on the chart over estimated pixels
     f. If a value is not clearly readable, OMIT it — never invent, round from guesswork, or interpolate
   - Do NOT describe individual bars, ticks, or axis fragments as separate items

5. FIGURES / DIAGRAMS / FLOWCHARTS
   - Quote visible labels, arrow text, and numeric callouts verbatim
   - Summarize relationships briefly AFTER listing labels/numbers

6. CODE / FORMULAS
   - Code in fenced blocks with language when clear
   - One formula, one span: inline math as $...$ and display math as a $$...$$ block
   - Do not wrap a formula in [ ] and do not substitute emoji for glyphs
   - When a formula is numbered, keep the number as \tag{n} inside the $$ block

7. WHAT TO IGNORE
   - Page numbers, repeated headers/footers, decorative borders
   - Isolated scribbles, doodles, or test marks with no semantic content

8. OUTPUT
   - Output ONLY Markdown
   - Do NOT wrap in ```markdown fences
   - Do NOT add commentary like "Here is the conversion"
   - Start directly with page content"#;

/// Select Pass-A system prompt by page modality (SPEC-134).
///
/// Print → existing print prompt (unchanged).
/// Manuscript / Mixed → manuscript prompt (fidelity rules, no EN pin).
pub fn pass_a_system_prompt_for(modality: crate::page_modality::PageModality) -> &'static str {
    match modality {
        crate::page_modality::PageModality::Print => RAG_PAGE_VISION_SYSTEM_PROMPT,
        crate::page_modality::PageModality::Manuscript
        | crate::page_modality::PageModality::Mixed => RAG_PAGE_MANUSCRIPT_VISION_SYSTEM_PROMPT,
    }
}

// ── SPEC-049 Two-pass figure filter prompts ────────────────────────────────────

/// System prompt for Pass-1 (filter): is this a real figure?
///
/// Axiom (SPEC-049/001 modality-split): a visual crop carries value only when
/// text/Markdown CANNOT express the same meaning.  The LLM is the semantic oracle.
pub const FIGURE_FILTER_PASS1_SYSTEM: &str = r#"You are a visual-content filter for a scientific PDF figure extraction pipeline.

Decide: is this image a REAL FIGURE with independent visual signal, or a FALSE POSITIVE?

First-principles axiom: a crop has value only when plain text / Markdown CANNOT carry
the same meaning (e.g. a bar chart, architecture diagram, photograph).

KEEP (is_figure: true) — these MUST be promoted to a specialised vision pass:
  bar_chart, line_chart, scatter_plot, heatmap, histogram, pie_chart, radar_chart,
  architecture_diagram, flowchart, diagram, illustration, photograph, system_demo,
  table_visual

DISCARD (is_figure: false) — artefacts, not RAG figures:
  logo, icon_logo, text_block, decorative_rule, empty,
  stamp, signature, scan_artefact, watermark

If the crop is a publisher logo, stamp, watermark, signature, icon, decorative rule,
or empty padding, you MUST set is_figure false even if kind is "other".

Respond in JSON ONLY — no fences, no explanation:
{"kind":"<one of above>","is_figure":<true|false>,"confidence":<0.0-1.0>}"#;

/// User message for Pass-1 (constant — the image is attached separately).
pub fn figure_filter_pass1_prompt() -> &'static str {
    "Classify this PDF crop: real figure or false positive?"
}

/// System prompt for Pass-2 (specialize).
pub const FIGURE_FILTER_PASS2_SYSTEM: &str =
    "You are an expert at extracting structured information from scientific figures. \
     Respond in Markdown only.";

/// Kind-aware user prompt for Pass-2.  Returns a `&'static str` for all known kinds.
pub fn figure_filter_pass2_prompt(kind: &crate::figure_filter::FigureKind) -> &'static str {
    use crate::figure_filter::FigureKind::*;
    match kind {
        BarChart | Histogram => {
            "Extract from this bar/histogram chart:\n\
             1. **Title** (if visible)\n\
             2. **X-axis** label and tick values\n\
             3. **Y-axis** label and range\n\
             4. **Series/groups** (legend)\n\
             5. **Data table** (Markdown) with approximate bar values\n\
             6. **Key observations** (2–3 bullets)"
        }
        LineChart => {
            "Extract from this line chart:\n\
             1. **Title** (if visible)\n\
             2. **X-axis** label and range\n\
             3. **Y-axis** label and range\n\
             4. **Series** names and trend direction\n\
             5. **Key observations** (2–3 bullets)\n\
             6. **Approximate data** (Markdown table if readable)"
        }
        ScatterPlot => {
            "Extract from this scatter plot:\n\
             1. **Title** and axis labels\n\
             2. **Groups/clusters** (colour / shape coding)\n\
             3. **Key patterns or outliers** (2–3 bullets)"
        }
        Heatmap => {
            "Extract from this heatmap:\n\
             1. **Title**, row labels, column labels\n\
             2. **Colour scale** meaning (high / low)\n\
             3. **Hotspot regions** that stand out\n\
             4. Reconstruct as **Markdown table** if axes are readable"
        }
        PieChart => {
            "Extract from this pie chart:\n\
             1. **Title** (if visible)\n\
             2. **Slices**: label and approximate % for each\n\
             3. **Key observation** (largest / smallest slice)"
        }
        RadarChart => {
            "Extract from this radar/spider chart:\n\
             1. **Axes** (dimension names)\n\
             2. **Series** compared (legend)\n\
             3. **Notable strengths / weaknesses** per dimension"
        }
        ArchitectureDiagram => {
            "Describe this architecture diagram:\n\
             1. **Top-level components** (list each named box/module)\n\
             2. **Data flow** — what flows between components and in what direction?\n\
             3. **External interfaces** (APIs, databases, users)\n\
             4. **Key design decision** visible in the diagram"
        }
        Flowchart => {
            "Describe this flowchart:\n\
             1. **Start and end** conditions\n\
             2. **Main steps** in order (numbered list)\n\
             3. **Decision branches** (condition → outcome)\n\
             4. **Loops or back-edges** (if any)"
        }
        Diagram => {
            "Describe this technical diagram:\n\
             1. **Main elements** and their roles\n\
             2. **Relationships and connections** between elements\n\
             3. **Directional flow** (if present)\n\
             4. **Key takeaway** in one sentence"
        }
        SystemDemo => {
            "Describe this system demonstration screenshot:\n\
             1. **Pipeline stages** shown (list each labelled section)\n\
             2. **Input** to the system (if visible)\n\
             3. **Output / response** produced\n\
             4. **Key observations** about the system behaviour"
        }
        TableVisual => {
            "Reconstruct this visual table:\n\
             1. **Headers** (column names)\n\
             2. **Rows** (as a Markdown table)\n\
             3. **Notable values** (maxima, minima, highlighted cells)"
        }
        Illustration | Photograph => {
            "Write a descriptive caption:\n\
             1. **Subject** (what is depicted)\n\
             2. **Key visual elements** labelled (if any)\n\
             3. **One-sentence caption** suitable for a figure in a paper"
        }
        // Noise kinds never reach Pass 2 — fallback for robustness
        _ => "Describe the key content of this image in 2–4 sentences.",
    }
}

// ── SPEC-134 WP-9: grounding verify pass prompts ─────────────────────────

/// System prompt for the grounding judge (SPEC-134 WP-9).
///
/// First principle (LAW-134-1): pixels are the only ground truth. The judge
/// measures whether each claim in a candidate Markdown transcription is
/// actually visible on the page image — it judges grounding, not style.
pub const GROUNDING_JUDGE_SYSTEM: &str = r#"You are a grounding judge for a document-conversion pipeline.

You are given a page image and a candidate Markdown transcription produced by
another model. Your ONLY job is to measure how faithfully the Markdown
reflects what is actually visible on the page.

Rules:
1. A claim in the Markdown is GROUNDED only if the corresponding words,
   numbers, or structure are visibly present on the page image.
2. INVENTED content: Markdown text with no visible counterpart on the page
   (fabricated titles, sections, values, or whole passages).
3. MISSING content: clearly legible page content absent from the Markdown.
4. Illegible handwriting marked with [?] or described as unreadable is
   HONEST, not an error — do not penalize it.
5. Judge grounding, not style. Ordering and formatting differences are fine.
6. An EMPTY or near-empty Markdown for a page with visible content is
   grounded_score 0.0 with the visible content listed as missing.
7. Never translate. Quote invented/missing content in the page's original
   language.

Respond with JSON ONLY — no prose, no code fence:
{"grounded_score": <float 0.0-1.0, fraction of the Markdown claims visible on the page>,
 "invented": [<short quotes of invented content, max 5>],
 "missing": [<short descriptions of missing visible content, max 5>]}"#;

/// Build the judge user prompt for one page (image is attached separately).
pub fn grounding_judge_user_prompt(candidate_markdown: &str) -> String {
    format!(
        "Candidate Markdown transcription of the attached page:

         ---
{}
---

         Score its grounding against the page image. JSON only.",
        candidate_markdown
    )
}

/// Build the refine user prompt after a low-grounding verdict.
///
/// One refine pass only (never a loop): the judge verdict is fed back so the
/// transcriber can remove invented content and add missing visible content.
pub fn grounding_refine_user_prompt(
    candidate_markdown: &str,
    grounded_score: f32,
    invented: &[String],
    missing: &[String],
) -> String {
    let invented_list = if invented.is_empty() {
        "(none listed)".to_string()
    } else {
        invented
            .iter()
            .map(|s| format!("- {s}"))
            .collect::<Vec<_>>()
            .join(
                "
",
            )
    };
    let missing_list = if missing.is_empty() {
        "(none listed)".to_string()
    } else {
        missing
            .iter()
            .map(|s| format!("- {s}"))
            .collect::<Vec<_>>()
            .join(
                "
",
            )
    };
    format!(
        "A grounding judge reviewed your transcription of the attached page.

         Verdict: grounded_score={grounded_score:.2}
         Invented content (REMOVE — not visible on the page):
{invented_list}
         Missing content (ADD — visible but absent):
{missing_list}

         Your previous transcription:
---
{candidate_markdown}
---

         Re-transcribe the page to Markdown. Rules:
         - Transcribe ONLY what is visible on the page. Never invent content.
         - Keep the page's original language. Never translate.
         - Mark illegible handwriting with [?].
         - Preserve the structure (headings, lists, tables, graphics as key-value).
         - Output ONLY Markdown, no fences, no commentary."
    )
}

/// SPEC-134 WP-2: user prompt for empty-page escalation re-OCR.
///
/// A page that returned the empty placeholder while its render carries ink is
/// an acquisition failure, not an empty page. The escalation call re-prompts
/// with the modality-routed system prompt; this user turn stays minimal and
/// directive. Language rule restated here because the system prompt alone did
/// not prevent the silent-empty failure mode.
pub const EMPTY_PAGE_ESCALATION_USER_PROMPT: &str = "Transcribe this page to Markdown completely, in the page's original language (never translate). Transcribe ONLY what is visible; mark illegible handwriting with [?]. Output ONLY Markdown, no fences, no commentary.";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rag_prompt_requires_chart_number_dump() {
        assert!(RAG_PAGE_VISION_SYSTEM_PROMPT.contains("CHARTS / PLOTS"));
        assert!(RAG_PAGE_VISION_SYSTEM_PROMPT.contains("EVERY readable data point"));
        assert!(RAG_PAGE_VISION_SYSTEM_PROMPT.contains("never invent"));
        assert!(RAG_PAGE_VISION_SYSTEM_PROMPT.contains("| Category / X |"));
        // 026 W1-dense-A densify + language pin
        assert!(RAG_PAGE_VISION_SYSTEM_PROMPT.contains("**Key values:**"));
        assert!(RAG_PAGE_VISION_SYSTEM_PROMPT.contains("Write all output in English"));
        assert!(RAG_PAGE_VISION_SYSTEM_PROMPT.contains("TABLES (critical for RAG)"));
        assert!(RAG_PAGE_VISION_SYSTEM_PROMPT.contains("fail closed on density"));
        // 032 year-span expand for list golds like ['1981','1982']
        assert!(RAG_PAGE_VISION_SYSTEM_PROMPT.contains("Year spans"));
        assert!(RAG_PAGE_VISION_SYSTEM_PROMPT.contains("1981-82"));
    }

    #[test]
    fn filter_prompts_have_required_sections() {
        assert!(FIGURE_FILTER_PASS1_SYSTEM.contains("promoted to a specialised vision pass"));
        assert!(FIGURE_FILTER_PASS1_SYSTEM.contains("text_block"));
        assert!(FIGURE_FILTER_PASS2_SYSTEM.contains("Markdown"));
        // Every figure kind must have a non-empty pass-2 prompt
        use crate::figure_filter::FigureKind;
        for kind in &[
            FigureKind::BarChart,
            FigureKind::Flowchart,
            FigureKind::ArchitectureDiagram,
            FigureKind::SystemDemo,
        ] {
            let p = figure_filter_pass2_prompt(kind);
            assert!(!p.is_empty(), "empty Pass-2 prompt for {kind:?}");
        }
    }
}
