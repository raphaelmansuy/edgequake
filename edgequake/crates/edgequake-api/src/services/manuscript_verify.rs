//! SPEC-134 WP-9: grounding verify pass for manuscript-class conversions.
//!
//! First principle (LAW-134-1): pixels are the only ground truth. Manuscript
//! pages are where VLMs silently confabulate — fluent, plausible, entirely
//! fabricated text (measured 2026-08-20: pages 2-4 of the assessment document
//! were ~0% grounded while reading as domain-appropriate French). This pass
//! judges each page's Markdown against its page render, refines once on a low
//! verdict, and marks still-low pages honestly.
//!
//! Policy:
//! - Manuscript-class only (cost gate; print pages have embedded-text truth).
//! - One refine pass per low page — never a loop.
//! - Fail-open everywhere: judge error, timeout, unparseable JSON, or a
//!   missing page PNG → accept the Pass-A text and flag the run unverified.
//!   Verification must never block ingestion (LAW-134-5).

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use base64::Engine;
use edgequake_llm::{
    resolve_effective_temperature, ChatMessage, CompletionOptions, ImageData, LLMProvider,
};
use edgequake_pdf::PageModality;
use tracing::warn;

/// Default minimum grounded score (`EDGEQUAKE_PDF_MANUSCRIPT_VERIFY_MIN`).
pub const DEFAULT_MIN_GROUNDED_SCORE: f32 = 0.6;

/// Per-call timeout for judge/refine — a hung judge must not stall ingestion.
const PER_CALL_TIMEOUT: Duration = Duration::from_secs(120);

/// Backoff before the single judge retry (transient provider errors).
const JUDGE_RETRY_BACKOFF: Duration = Duration::from_millis(400);

fn classify_verify_error(err: &str) -> &'static str {
    let lower = err.to_ascii_lowercase();
    if lower.contains("too large")
        || lower.contains("image size")
        || lower.contains("payload too")
        || lower.contains("request too large")
    {
        "image_too_large"
    } else if lower.contains("page png missing") || lower.contains("read ") {
        "page_png_missing"
    } else if lower.contains("refine") {
        "refine_call_failed"
    } else {
        "judge_call_failed"
    }
}

fn record_fail_open(outcome: &mut VerifyOutcome, reason: &str) {
    outcome.fail_open = true;
    if outcome.fail_reason.is_none() {
        outcome.fail_reason = Some(reason.to_string());
    }
}

/// Honesty marker injected after the page marker when grounding stays low.
pub const GROUNDING_LOW_MARKER_PREFIX: &str = "<!-- grounding:low";

/// Provenance trace left in the index-bound text for a quarantined page.
pub const GROUNDING_QUARANTINED_MARKER: &str = "<!-- grounding:quarantined -->";

/// Judge verdict for one page (JSON contract with the judge model).
#[derive(Debug, Clone, PartialEq)]
pub struct GroundingVerdict {
    pub grounded_score: f32,
    pub invented: Vec<String>,
    pub missing: Vec<String>,
}

/// Aggregate outcome of the verify pass.
#[derive(Debug)]
pub struct VerifyOutcome {
    /// Markdown after refinement + honesty markers (input unchanged when the
    /// pass did not run).
    pub markdown: String,
    pub pages_judged: usize,
    pub pages_low_grounding: usize,
    pub pages_refined: usize,
    /// Mean final grounded score over judged pages.
    pub mean_score: Option<f32>,
    /// Whether the pass actually ran (manuscript-class + enabled).
    pub ran: bool,
    /// A judge/refine call failed, timed out, or was unparseable — content
    /// accepted unverified (fail-open).
    pub fail_open: bool,
    /// First fail-open cause (`judge_call_failed` / `page_png_missing` /
    /// `image_too_large` / `refine_call_failed`). None when the pass succeeded
    /// or did not run. Observability law: every degradation carries its reason.
    pub fail_reason: Option<String>,
}

/// Whether the verify pass is enabled (`EDGEQUAKE_PDF_MANUSCRIPT_VERIFY`,
/// default ON).
pub fn verify_enabled_from_env() -> bool {
    std::env::var("EDGEQUAKE_PDF_MANUSCRIPT_VERIFY")
        .map(|v| !matches!(v.to_lowercase().as_str(), "0" | "false" | "off" | "no"))
        .unwrap_or(true)
}

/// Minimum grounded score (`EDGEQUAKE_PDF_MANUSCRIPT_VERIFY_MIN`, default 0.6).
pub fn min_grounded_score_from_env() -> f32 {
    std::env::var("EDGEQUAKE_PDF_MANUSCRIPT_VERIFY_MIN")
        .ok()
        .and_then(|v| v.parse::<f32>().ok())
        .map(|v| v.clamp(0.0, 1.0))
        .unwrap_or(DEFAULT_MIN_GROUNDED_SCORE)
}

/// Parse the judge's JSON verdict. Tolerates code fences and surrounding
/// prose; returns `None` on anything unparseable (caller fails open).
pub fn parse_grounding_verdict(raw: &str) -> Option<GroundingVerdict> {
    let start = raw.find('{')?;
    let end = raw.rfind('}')?;
    let slice = raw.get(start..=end)?;
    let v: serde_json::Value = serde_json::from_str(slice).ok()?;
    let score = v.get("grounded_score")?.as_f64()? as f32;
    let strings = |key: &str| -> Vec<String> {
        v.get(key)
            .and_then(|a| a.as_array())
            .map(|a| {
                a.iter()
                    .filter_map(|x| x.as_str().map(|s| s.to_string()))
                    .collect()
            })
            .unwrap_or_default()
    };
    Some(GroundingVerdict {
        grounded_score: score.clamp(0.0, 1.0),
        invented: strings("invented"),
        missing: strings("missing"),
    })
}

/// One page's markdown span, delimited by `<!-- edgequake-page:N -->` markers.
#[derive(Debug)]
struct PageSection {
    page_num: usize,
    /// Byte range of the section (marker line included).
    range: std::ops::Range<usize>,
}

fn split_page_sections(markdown: &str) -> Vec<PageSection> {
    const MARKER: &str = "<!-- edgequake-page:";
    let mut markers: Vec<(usize, usize)> = Vec::new();
    let mut search_from = 0;
    while let Some(rel) = markdown[search_from..].find(MARKER) {
        let abs = search_from + rel;
        let num_start = abs + MARKER.len();
        let num_end = markdown[num_start..]
            .find("-->")
            .map(|e| num_start + e)
            .unwrap_or(num_start);
        if let Ok(n) = markdown[num_start..num_end].trim().parse::<usize>() {
            markers.push((abs, n));
        }
        search_from = num_end;
    }
    markers
        .iter()
        .enumerate()
        .map(|(i, &(off, n))| {
            let end = markers
                .get(i + 1)
                .map(|(o, _)| *o)
                .unwrap_or(markdown.len());
            PageSection {
                page_num: n,
                range: off..end,
            }
        })
        .collect()
}

/// Quarantine lane (SPEC-134 P0): drop the content of sections marked
/// `grounding:low` from the text bound for chunking / entity extraction.
///
/// Display != Index (LAW-134-4): the stored markdown keeps the marked content
/// for honest human review, but unverified text must not become knowledge-graph
/// beliefs (measured 2026-08-20: a `grounding:low score=0.00` page still
/// produced `TRACTION_TEST_RESULTS`-class entities). A quarantined page keeps
/// its page marker (provenance) and contributes nothing else.
///
/// No-op when no marker is present (print docs, verify disabled).
pub fn strip_low_grounding_sections(markdown: &str) -> String {
    if !markdown.contains(GROUNDING_LOW_MARKER_PREFIX) {
        return markdown.to_string();
    }
    let sections = split_page_sections(markdown);
    if sections.is_empty() {
        return markdown.to_string();
    }
    let mut out = String::with_capacity(markdown.len());
    out.push_str(&markdown[..sections.first().map(|s| s.range.start).unwrap_or(0)]);
    for section in &sections {
        let text = &markdown[section.range.clone()];
        if !text.contains(GROUNDING_LOW_MARKER_PREFIX) {
            out.push_str(text);
            continue;
        }
        let marker_line = match text.find('\n') {
            Some(idx) => &text[..idx + 1],
            None => text,
        };
        out.push_str(marker_line);
        out.push_str(GROUNDING_QUARANTINED_MARKER);
        out.push_str("\n\n");
    }
    out
}

/// Page render path — the viewer PNG written during conversion is the pixel
/// record the judge verifies against (`{assets_root}/assets/page-NNNN.png`).
fn page_png_path(assets_root: &Path, page_num: usize) -> PathBuf {
    assets_root
        .join("assets")
        .join(format!("page-{page_num:04}.png"))
}

fn load_page_image(png_path: &Path) -> std::io::Result<ImageData> {
    let bytes = std::fs::read(png_path)?;
    let b64 = base64::engine::general_purpose::STANDARD.encode(&bytes);
    Ok(ImageData::new(b64, "image/png"))
}

async fn judge_page(
    provider: &dyn LLMProvider,
    png_path: &Path,
    candidate_markdown: &str,
) -> Result<GroundingVerdict, String> {
    let image =
        load_page_image(png_path).map_err(|e| format!("read {}: {e}", png_path.display()))?;
    let opts = CompletionOptions {
        max_tokens: Some(500),
        temperature: resolve_effective_temperature(provider.model(), 0.0),
        ..Default::default()
    }
    .with_role_cache("grounding-judge", provider);
    let messages = vec![
        ChatMessage::system(edgequake_pdf::GROUNDING_JUDGE_SYSTEM),
        ChatMessage::user_with_images(
            edgequake_pdf::grounding_judge_user_prompt(candidate_markdown),
            vec![image],
        ),
    ];
    let response = tokio::time::timeout(PER_CALL_TIMEOUT, provider.chat(&messages, Some(&opts)))
        .await
        .map_err(|_| "judge call timed out".to_string())?
        .map_err(|e| format!("judge call failed: {e}"))?;
    parse_grounding_verdict(&response.content)
        .ok_or_else(|| "judge returned unparseable verdict".to_string())
}

/// One retry with backoff on judge failure (transient provider errors must
/// not disable the gate for a whole document — measured 2026-08-20: one
/// oversized-page failure failed-open all 4 pages with no reason recorded).
async fn judge_page_with_retry(
    provider: &dyn LLMProvider,
    png_path: &Path,
    candidate_markdown: &str,
) -> Result<GroundingVerdict, String> {
    match judge_page(provider, png_path, candidate_markdown).await {
        Ok(v) => Ok(v),
        Err(first) => {
            warn!(error = %first, "SPEC-134 verify: judge failed — retrying once");
            tokio::time::sleep(JUDGE_RETRY_BACKOFF).await;
            judge_page(provider, png_path, candidate_markdown)
                .await
                .map_err(|second| format!("{first}; retry: {second}"))
        }
    }
}

/// One refine pass: re-transcribe with the judge verdict as feedback. The
/// system prompt is the manuscript Pass-A SSOT — refine IS re-transcription.
async fn refine_page(
    provider: &dyn LLMProvider,
    png_path: &Path,
    candidate_markdown: &str,
    verdict: &GroundingVerdict,
) -> Result<String, String> {
    let image =
        load_page_image(png_path).map_err(|e| format!("read {}: {e}", png_path.display()))?;
    let opts = CompletionOptions {
        max_tokens: Some(4096),
        temperature: resolve_effective_temperature(provider.model(), 0.0),
        ..Default::default()
    }
    .with_role_cache("grounding-refine", provider);
    let messages = vec![
        ChatMessage::system(edgequake_pdf::pass_a_system_prompt_for(
            PageModality::Manuscript,
        )),
        ChatMessage::user_with_images(
            edgequake_pdf::grounding_refine_user_prompt(
                candidate_markdown,
                verdict.grounded_score,
                &verdict.invented,
                &verdict.missing,
            ),
            vec![image],
        ),
    ];
    let response = tokio::time::timeout(PER_CALL_TIMEOUT, provider.chat(&messages, Some(&opts)))
        .await
        .map_err(|_| "refine call timed out".to_string())?
        .map_err(|e| format!("refine call failed: {e}"))?;
    let refined = response.content.trim();
    if refined.is_empty() {
        return Err("refine returned empty content".to_string());
    }
    Ok(refined.to_string())
}

/// Verify one page section; returns the replacement section text.
///
/// Per-page verify result used when reassembling in document order.
struct PageVerifyResult {
    text: String,
    judged: bool,
    low: bool,
    refined: bool,
    score: Option<f32>,
    fail_reason: Option<String>,
}

/// Section shape: `<!-- edgequake-page:N -->\n<content>`. The marker line is
/// preserved verbatim; only the content is refined/replaced.
async fn verify_one_page_result(
    provider: &dyn LLMProvider,
    assets_root: &Path,
    page_num: usize,
    section_text: &str,
    min_score: f32,
) -> PageVerifyResult {
    let (marker_line, content) = match section_text.find('\n') {
        Some(idx) => section_text.split_at(idx + 1),
        None => (section_text, ""),
    };
    let png = page_png_path(assets_root, page_num);
    if !png.exists() {
        warn!(
            page = page_num,
            "SPEC-134 verify: page PNG missing — fail open"
        );
        return PageVerifyResult {
            text: section_text.to_string(),
            judged: false,
            low: false,
            refined: false,
            score: None,
            fail_reason: Some("page_png_missing".into()),
        };
    }
    let verdict = match judge_page_with_retry(provider, &png, content).await {
        Ok(v) => v,
        Err(e) => {
            warn!(page = page_num, error = %e, "SPEC-134 verify: judge failed — fail open");
            return PageVerifyResult {
                text: section_text.to_string(),
                judged: false,
                low: false,
                refined: false,
                score: None,
                fail_reason: Some(classify_verify_error(&e).to_string()),
            };
        }
    };
    if verdict.grounded_score >= min_score {
        return PageVerifyResult {
            text: section_text.to_string(),
            judged: true,
            low: false,
            refined: false,
            score: Some(verdict.grounded_score),
            fail_reason: None,
        };
    }
    let refined = match refine_page(provider, &png, content, &verdict).await {
        Ok(r) => r,
        Err(e) => {
            warn!(page = page_num, error = %e, "SPEC-134 verify: refine failed — fail open");
            return PageVerifyResult {
                text: format!(
                    "{marker_line}{GROUNDING_LOW_MARKER_PREFIX} score={:.2} -->\n{}",
                    verdict.grounded_score, content
                ),
                judged: true,
                low: true,
                refined: false,
                score: Some(verdict.grounded_score),
                fail_reason: Some("refine_call_failed".into()),
            };
        }
    };
    let (final_score, keep_marker, fail_reason) =
        match judge_page_with_retry(provider, &png, &refined).await {
            Ok(v2) => (v2.grounded_score, v2.grounded_score < min_score, None),
            Err(e) => {
                warn!(page = page_num, error = %e, "SPEC-134 verify: re-judge failed — fail open");
                (
                    verdict.grounded_score,
                    true,
                    Some(classify_verify_error(&e).to_string()),
                )
            }
        };
    let text = if keep_marker {
        format!(
            "{marker_line}{GROUNDING_LOW_MARKER_PREFIX} score={final_score:.2} -->\n\n{refined}\n\n"
        )
    } else {
        format!("{marker_line}\n{refined}\n\n")
    };
    PageVerifyResult {
        text,
        judged: true,
        low: true,
        refined: true,
        score: Some(final_score),
        fail_reason,
    }
}

/// Heartbeat while verifying/escalating: `(done, total, physical_page)`.
pub type ManuscriptPageProgressHook = Arc<dyn Fn(usize, usize, usize) + Send + Sync>;

const VERIFY_CONCURRENCY: usize = 2;
const ESCALATE_CONCURRENCY: usize = 2;

/// Shared knobs for bounded page work (verify + escalate).
#[derive(Debug, Clone)]
pub struct PageWorkLimits {
    pub concurrency: usize,
    pub per_call_timeout: Duration,
}

impl Default for PageWorkLimits {
    fn default() -> Self {
        Self {
            concurrency: VERIFY_CONCURRENCY,
            per_call_timeout: PER_CALL_TIMEOUT,
        }
    }
}

/// Run the grounding verify pass over manuscript-class conversion markdown.
///
/// `allowed_pages` (1-indexed) scopes work to authoritative manuscript pages in
/// mixed documents. When `None`, every marked page is eligible.
///
/// Returns the input unchanged (with `ran: false`) for print documents, when
/// disabled via env, or when no page markers are present.
pub async fn verify_manuscript_markdown(
    markdown: &str,
    modality: PageModality,
    assets_root: Option<&Path>,
    provider: Arc<dyn LLMProvider>,
) -> VerifyOutcome {
    verify_manuscript_markdown_scoped(markdown, modality, assets_root, provider, None, None).await
}

/// Scoped verify with optional allowed-page set and per-page progress heartbeats.
pub async fn verify_manuscript_markdown_scoped(
    markdown: &str,
    modality: PageModality,
    assets_root: Option<&Path>,
    provider: Arc<dyn LLMProvider>,
    allowed_pages: Option<&[usize]>,
    progress: Option<&ManuscriptPageProgressHook>,
) -> VerifyOutcome {
    let mut outcome = VerifyOutcome {
        markdown: markdown.to_string(),
        pages_judged: 0,
        pages_low_grounding: 0,
        pages_refined: 0,
        mean_score: None,
        ran: false,
        fail_open: false,
        fail_reason: None,
    };
    if !modality.is_manuscript_like() || !verify_enabled_from_env() {
        return outcome;
    }
    outcome.ran = true;
    let min_score = min_grounded_score_from_env();
    let sections = split_page_sections(markdown);
    if sections.is_empty() {
        return outcome;
    }

    let allowed: Option<std::collections::HashSet<usize>> =
        allowed_pages.map(|p| p.iter().copied().collect());

    // Collect work items preserving section index for ordered rebuild.
    let mut work: Vec<(usize, usize, String)> = Vec::new();
    for (idx, section) in sections.iter().enumerate() {
        if let Some(ref set) = allowed {
            if !set.contains(&section.page_num) {
                continue;
            }
        }
        work.push((
            idx,
            section.page_num,
            markdown[section.range.clone()].to_string(),
        ));
    }

    let work_total = work.len();
    let mut results: Vec<Option<PageVerifyResult>> = (0..sections.len()).map(|_| None).collect();

    if let Some(root) = assets_root {
        let root = root.to_path_buf();
        use futures::stream::{self, StreamExt};
        let mut stream = stream::iter(work.into_iter().enumerate())
            .map(|(done_idx, (section_idx, page_num, section_text))| {
                let provider = Arc::clone(&provider);
                let root = root.clone();
                async move {
                    let result = verify_one_page_result(
                        provider.as_ref(),
                        &root,
                        page_num,
                        &section_text,
                        min_score,
                    )
                    .await;
                    (done_idx, section_idx, page_num, result)
                }
            })
            .buffer_unordered(VERIFY_CONCURRENCY);

        let mut completed = 0usize;
        while let Some((done_idx, section_idx, page_num, result)) = stream.next().await {
            let _ = done_idx;
            completed += 1;
            if let Some(hook) = progress {
                hook(completed, work_total, page_num);
            }
            results[section_idx] = Some(result);
        }
    } else {
        for (section_idx, page_num, _) in &work {
            let _ = page_num;
            results[*section_idx] = Some(PageVerifyResult {
                text: markdown[sections[*section_idx].range.clone()].to_string(),
                judged: false,
                low: false,
                refined: false,
                score: None,
                fail_reason: Some("page_png_missing".into()),
            });
            if let Some(hook) = progress {
                hook(1, work_total.max(1), *page_num);
            }
        }
        record_fail_open(&mut outcome, "page_png_missing");
    }

    let mut scores: Vec<f32> = Vec::new();
    let mut rebuilt = String::with_capacity(markdown.len() + 256);
    let prefix_end = sections.first().map(|s| s.range.start).unwrap_or(0);
    rebuilt.push_str(&markdown[..prefix_end]);
    for (idx, section) in sections.iter().enumerate() {
        let section_text = &markdown[section.range.clone()];
        match results[idx].take() {
            Some(r) => {
                if r.judged {
                    outcome.pages_judged += 1;
                }
                if r.low {
                    outcome.pages_low_grounding += 1;
                }
                if r.refined {
                    outcome.pages_refined += 1;
                }
                if let Some(s) = r.score {
                    scores.push(s);
                }
                if let Some(reason) = r.fail_reason.as_deref() {
                    record_fail_open(&mut outcome, reason);
                }
                rebuilt.push_str(&r.text);
            }
            None => rebuilt.push_str(section_text),
        }
    }
    outcome.markdown = rebuilt;
    if !scores.is_empty() {
        outcome.mean_score = Some(scores.iter().sum::<f32>() / scores.len() as f32);
    }
    outcome
}

// ============================================================================
// SPEC-134 WP-2: empty-page escalation
// ============================================================================

/// Outcome of the empty-page escalation pass.
#[derive(Debug)]
pub struct EscalationOutcome {
    /// Markdown with recovered page bodies merged in (input unchanged when no
    /// empty page was found or the pass is disabled).
    pub markdown: String,
    /// Pages successfully re-transcribed by the escalation call.
    pub pages_escalated: Vec<usize>,
    /// Pages whose escalation call failed or returned empty — the honest
    /// placeholder stays, and the failure is visible in document metadata.
    pub pages_failed: Vec<usize>,
}

/// Whether empty-page escalation is enabled
/// (`EDGEQUAKE_PDF_EMPTY_PAGE_RETRY`, default ON).
pub fn empty_page_retry_enabled_from_env() -> bool {
    std::env::var("EDGEQUAKE_PDF_EMPTY_PAGE_RETRY")
        .map(|v| !matches!(v.to_lowercase().as_str(), "0" | "false" | "off" | "no"))
        .unwrap_or(true)
}

/// Body needs empty-page retry: placeholder is present and there is no
/// remaining prose (markdown images do not count). Slice E: prepended crops
/// used to defeat exact-match; containment + no-prose still retries.
fn section_needs_empty_escalation(body: &str) -> bool {
    if !body.contains(edgequake_pdf::EMPTY_VISION_PAGE_PLACEHOLDER) {
        return false;
    }
    let without_placeholder = body.replace(edgequake_pdf::EMPTY_VISION_PAGE_PLACEHOLDER, "");
    let without_images = strip_markdown_images(&without_placeholder);
    without_images.trim().is_empty()
}

fn strip_markdown_images(md: &str) -> String {
    let mut out = String::with_capacity(md.len());
    let mut rest = md;
    while let Some(start) = rest.find("![") {
        out.push_str(&rest[..start]);
        let after = &rest[start..];
        if let Some(end) = after.find(')') {
            rest = &after[end + 1..];
        } else {
            rest = "";
            break;
        }
    }
    out.push_str(rest);
    out
}

/// Detect pages whose body is exactly the empty placeholder while a page
/// render exists on disk, and re-OCR them with the escalation provider.
///
/// Calibration law: a page that yields nothing while pixels exist is an
/// acquisition failure (measured 2026-08-20: an 11.9MB page PNG exceeded the
/// provider limit and silently became a placeholder). The escalation call
/// re-prompts with the modality-routed system prompt; callers pass a stronger
/// model when configured. Runs before the grounding verify pass so recovered
/// content is verified like everything else.
/// Re-OCR pages whose body is only the empty-vision placeholder.
///
/// `allowed_pages` scopes retries to authoritative manuscript pages in mixed
/// documents (print pages must not be re-OCR'd because of stitch placeholders).
pub async fn escalate_empty_pages(
    markdown: &str,
    assets_root: &Path,
    provider: Arc<dyn LLMProvider>,
    modality: PageModality,
) -> EscalationOutcome {
    escalate_empty_pages_scoped(markdown, assets_root, provider, modality, None, None).await
}

/// Scoped empty-page escalation with optional allowed-page set and heartbeats.
pub async fn escalate_empty_pages_scoped(
    markdown: &str,
    assets_root: &Path,
    provider: Arc<dyn LLMProvider>,
    modality: PageModality,
    allowed_pages: Option<&[usize]>,
    progress: Option<&ManuscriptPageProgressHook>,
) -> EscalationOutcome {
    escalate_empty_pages_bounded(
        markdown,
        assets_root,
        provider,
        modality,
        allowed_pages,
        progress,
        PageWorkLimits {
            concurrency: ESCALATE_CONCURRENCY,
            per_call_timeout: PER_CALL_TIMEOUT,
        },
        None,
    )
    .await
}

/// Bounded empty-page escalation with injectable limits and optional cancel.
#[allow(clippy::too_many_arguments)]
pub async fn escalate_empty_pages_bounded(
    markdown: &str,
    assets_root: &Path,
    provider: Arc<dyn LLMProvider>,
    modality: PageModality,
    allowed_pages: Option<&[usize]>,
    progress: Option<&ManuscriptPageProgressHook>,
    limits: PageWorkLimits,
    cancel: Option<&tokio_util::sync::CancellationToken>,
) -> EscalationOutcome {
    let mut outcome = EscalationOutcome {
        markdown: markdown.to_string(),
        pages_escalated: Vec::new(),
        pages_failed: Vec::new(),
    };
    if !empty_page_retry_enabled_from_env() {
        return outcome;
    }
    let sections = split_page_sections(markdown);
    if sections.is_empty() {
        return outcome;
    }
    let allowed: Option<std::collections::HashSet<usize>> =
        allowed_pages.map(|p| p.iter().copied().collect());

    let mut candidates: Vec<(usize, usize, String, String)> = Vec::new();
    for (idx, section) in sections.iter().enumerate() {
        if let Some(ref set) = allowed {
            if !set.contains(&section.page_num) {
                continue;
            }
        }
        let section_text = &markdown[section.range.clone()];
        let (marker_line, body) = match section_text.find('\n') {
            Some(i) => section_text.split_at(i + 1),
            None => (section_text, ""),
        };
        if section_needs_empty_escalation(body) {
            candidates.push((
                idx,
                section.page_num,
                marker_line.to_string(),
                section_text.to_string(),
            ));
        }
    }
    let work_total = candidates.len();
    if work_total == 0 {
        return outcome;
    }

    let mut results: Vec<Option<(EscalationPageStatus, String)>> =
        (0..sections.len()).map(|_| None).collect();
    let root = assets_root.to_path_buf();
    let concurrency = limits.concurrency.max(1);
    use futures::stream::{self, StreamExt};
    let mut stream = stream::iter(candidates.into_iter().enumerate())
        .map(
            |(done_idx, (section_idx, page_num, marker_line, section_text))| {
                let provider = Arc::clone(&provider);
                let root = root.clone();
                let cancel = cancel.cloned();
                let timeout = limits.per_call_timeout;
                async move {
                    if cancel.as_ref().is_some_and(|c| c.is_cancelled()) {
                        return (
                            done_idx,
                            section_idx,
                            page_num,
                            EscalationPageStatus::Skipped,
                            section_text,
                        );
                    }
                    let png = page_png_path(&root, page_num);
                    if !png.exists() {
                        return (
                            done_idx,
                            section_idx,
                            page_num,
                            EscalationPageStatus::Skipped,
                            section_text,
                        );
                    }
                    match reocr_page_with_timeout(provider.as_ref(), &png, modality, timeout).await
                    {
                        Ok(content) => {
                            let rebuilt = format!("{marker_line}\n{content}\n\n");
                            (
                                done_idx,
                                section_idx,
                                page_num,
                                EscalationPageStatus::Recovered,
                                rebuilt,
                            )
                        }
                        Err(e) => {
                            warn!(
                                page = page_num,
                                error = %e,
                                "SPEC-134 escalation: empty-page re-OCR failed — placeholder kept"
                            );
                            (
                                done_idx,
                                section_idx,
                                page_num,
                                EscalationPageStatus::Failed,
                                section_text,
                            )
                        }
                    }
                }
            },
        )
        .buffer_unordered(concurrency);

    let mut completed = 0usize;
    while let Some((_done_idx, section_idx, page_num, status, text)) = stream.next().await {
        completed += 1;
        match status {
            EscalationPageStatus::Recovered => outcome.pages_escalated.push(page_num),
            EscalationPageStatus::Failed => outcome.pages_failed.push(page_num),
            EscalationPageStatus::Skipped => {}
        }
        results[section_idx] = Some((status, text));
        if let Some(hook) = progress {
            hook(completed, work_total, page_num);
        }
        if cancel.is_some_and(|c| c.is_cancelled()) {
            break;
        }
    }

    outcome.pages_failed.sort_unstable();
    outcome.pages_failed.dedup();
    outcome.pages_escalated.sort_unstable();
    outcome.pages_escalated.dedup();

    let mut rebuilt = String::with_capacity(markdown.len() + 256);
    let prefix_end = sections.first().map(|s| s.range.start).unwrap_or(0);
    rebuilt.push_str(&markdown[..prefix_end]);
    for (idx, section) in sections.iter().enumerate() {
        let section_text = &markdown[section.range.clone()];
        match results[idx].take() {
            Some((_, text)) => rebuilt.push_str(&text),
            None => rebuilt.push_str(section_text),
        }
    }
    outcome.markdown = rebuilt;
    outcome
}

#[derive(Debug, Clone, Copy)]
enum EscalationPageStatus {
    Recovered,
    Failed,
    Skipped,
}

/// Single-page re-transcription call for escalation (DRY with the verify
/// pass's refine: same image load, same timeout, same prompt SSOT).
async fn reocr_page_with_timeout(
    provider: &dyn LLMProvider,
    png_path: &Path,
    modality: PageModality,
    timeout: Duration,
) -> Result<String, String> {
    let image =
        load_page_image(png_path).map_err(|e| format!("read {}: {e}", png_path.display()))?;
    let opts = CompletionOptions {
        max_tokens: Some(4096),
        temperature: resolve_effective_temperature(provider.model(), 0.0),
        ..Default::default()
    }
    .with_role_cache("empty-page-escalation", provider);
    let messages = vec![
        ChatMessage::system(edgequake_pdf::pass_a_system_prompt_for(modality)),
        ChatMessage::user_with_images(
            edgequake_pdf::EMPTY_PAGE_ESCALATION_USER_PROMPT,
            vec![image],
        ),
    ];
    let response = tokio::time::timeout(timeout, provider.chat(&messages, Some(&opts)))
        .await
        .map_err(|_| "escalation call timed out".to_string())?
        .map_err(|e| format!("escalation call failed: {e}"))?;
    let content = response.content.trim();
    if content.is_empty() {
        return Err("escalation returned empty content".to_string());
    }
    Ok(content.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use edgequake_llm::MockProvider;

    #[test]
    fn parse_clean_verdict() {
        let v =
            parse_grounding_verdict(r#"{"grounded_score": 0.9, "invented": ["x"], "missing": []}"#)
                .expect("clean JSON parses");
        assert_eq!(v.grounded_score, 0.9);
        assert_eq!(v.invented, vec!["x".to_string()]);
        assert!(v.missing.is_empty());
    }

    #[test]
    fn parse_fenced_verdict() {
        let raw = "Here is my assessment:\n```json\n{\"grounded_score\": 0.3, \"invented\": [], \"missing\": [\"intro\"]}\n```\nDone.";
        let v = parse_grounding_verdict(raw).expect("fenced JSON parses");
        assert_eq!(v.grounded_score, 0.3);
        assert_eq!(v.missing, vec!["intro".to_string()]);
    }

    #[test]
    fn parse_garbage_fails_open() {
        assert!(parse_grounding_verdict("not json at all").is_none());
        assert!(parse_grounding_verdict("{\"other\": 1}").is_none());
        assert!(parse_grounding_verdict("").is_none());
    }

    #[test]
    fn parse_clamps_score_to_unit_interval() {
        let v = parse_grounding_verdict(r#"{"grounded_score": 4.2}"#).expect("parses");
        assert_eq!(v.grounded_score, 1.0);
    }

    #[test]
    fn split_sections_by_page_marker() {
        let md = "pre\n<!-- edgequake-page:1 -->\nfirst\n<!-- edgequake-page:2 -->\nsecond\n";
        let sections = split_page_sections(md);
        assert_eq!(sections.len(), 2);
        assert_eq!(sections[0].page_num, 1);
        assert_eq!(sections[1].page_num, 2);
        assert!(md[sections[0].range.clone()].contains("first"));
        assert!(md[sections[1].range.clone()].contains("second"));
        assert!(!md[sections[0].range.clone()].contains("second"));
    }

    #[test]
    fn split_sections_empty_without_markers() {
        assert!(split_page_sections("no markers here").is_empty());
    }

    /// Write a tiny valid PNG so the verifier finds pixels on disk.
    fn write_tiny_png(root: &Path, page_num: usize) {
        let dir = root.join("assets");
        std::fs::create_dir_all(&dir).unwrap();
        // 1x1 transparent PNG (68 bytes).
        const PNG: [u8; 68] = [
            0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a, 0x00, 0x00, 0x00, 0x0d, 0x49, 0x48,
            0x44, 0x52, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x08, 0x06, 0x00, 0x00,
            0x00, 0x1f, 0x15, 0xc4, 0x89, 0x0d, 0x00, 0x00, 0x00, 0x0a, 0x49, 0x44, 0x41, 0x54,
            0x78, 0x9c, 0x62, 0x00, 0x01, 0x00, 0x00, 0x05, 0x00, 0x01, 0x0d, 0x0a, 0x2d, 0xb4,
            0x00, 0x00, 0x00, 0x00, 0x49, 0x45, 0x4e, 0x44, 0xae, 0x42, 0x60, 0x82,
        ];
        std::fs::write(dir.join(format!("page-{page_num:04}.png")), PNG).unwrap();
    }

    #[tokio::test]
    async fn print_modality_skips_verify() {
        let mock = MockProvider::new();
        let out = verify_manuscript_markdown(
            "<!-- edgequake-page:1 -->\ntext",
            PageModality::Print,
            None,
            Arc::new(mock),
        )
        .await;
        assert!(!out.ran);
        assert_eq!(out.markdown, "<!-- edgequake-page:1 -->\ntext");
    }

    #[tokio::test]
    async fn high_grounding_passes_unmodified() {
        let dir = std::env::temp_dir().join(format!("mv-{}", uuid::Uuid::new_v4()));
        write_tiny_png(&dir, 1);
        let mock = MockProvider::new();
        mock.add_response(r#"{"grounded_score": 0.95, "invented": [], "missing": []}"#)
            .await;
        let md = "<!-- edgequake-page:1 -->\n# Real content\n";
        let out = verify_manuscript_markdown(
            md,
            PageModality::Manuscript,
            Some(dir.as_path()),
            Arc::new(mock),
        )
        .await;
        assert!(out.ran && !out.fail_open);
        assert_eq!(out.pages_judged, 1);
        assert_eq!(out.pages_low_grounding, 0);
        assert_eq!(out.markdown, md);
        assert_eq!(out.mean_score, Some(0.95));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[tokio::test]
    async fn low_grounding_refines_then_passes() {
        let dir = std::env::temp_dir().join(format!("mv-{}", uuid::Uuid::new_v4()));
        write_tiny_png(&dir, 1);
        let mock = MockProvider::new();
        // judge: low → refine → re-judge: high
        mock.add_response(r#"{"grounded_score": 0.2, "invented": ["fake"], "missing": []}"#)
            .await;
        mock.add_response("# Corrected transcription").await;
        mock.add_response(r#"{"grounded_score": 0.9, "invented": [], "missing": []}"#)
            .await;
        let md = "<!-- edgequake-page:1 -->\nfabricated\n";
        let out = verify_manuscript_markdown(
            md,
            PageModality::Manuscript,
            Some(dir.as_path()),
            Arc::new(mock),
        )
        .await;
        assert_eq!(out.pages_low_grounding, 1);
        assert_eq!(out.pages_refined, 1);
        assert!(out.markdown.contains("# Corrected transcription"));
        assert!(!out.markdown.contains(GROUNDING_LOW_MARKER_PREFIX));
        assert_eq!(out.mean_score, Some(0.9));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[tokio::test]
    async fn persistent_low_grounding_is_marked_honestly() {
        let dir = std::env::temp_dir().join(format!("mv-{}", uuid::Uuid::new_v4()));
        write_tiny_png(&dir, 1);
        let mock = MockProvider::new();
        mock.add_response(r#"{"grounded_score": 0.1, "invented": ["all"], "missing": []}"#)
            .await;
        mock.add_response("best effort text").await;
        mock.add_response(r#"{"grounded_score": 0.3, "invented": [], "missing": ["most"]}"#)
            .await;
        let md = "<!-- edgequake-page:1 -->\nfabricated\n";
        let out = verify_manuscript_markdown(
            md,
            PageModality::Manuscript,
            Some(dir.as_path()),
            Arc::new(mock),
        )
        .await;
        assert!(out.markdown.contains("best effort text"));
        assert!(
            out.markdown.contains("<!-- grounding:low score=0.30 -->"),
            "still-low page must carry the honesty marker: {}",
            out.markdown
        );
        assert_eq!(out.mean_score, Some(0.3));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[tokio::test]
    async fn judge_error_fails_open_unmodified() {
        let dir = std::env::temp_dir().join(format!("mv-{}", uuid::Uuid::new_v4()));
        write_tiny_png(&dir, 1);
        let mock = MockProvider::new();
        mock.add_response("total garbage, no JSON").await;
        mock.add_response("still garbage").await; // retry also fails
        let md = "<!-- edgequake-page:1 -->\ncontent\n";
        let out = verify_manuscript_markdown(
            md,
            PageModality::Manuscript,
            Some(dir.as_path()),
            Arc::new(mock),
        )
        .await;
        assert!(out.fail_open);
        assert_eq!(out.fail_reason.as_deref(), Some("judge_call_failed"));
        assert_eq!(out.markdown, md, "fail-open must return the original text");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[tokio::test]
    async fn judge_retries_once_then_succeeds() {
        let dir = std::env::temp_dir().join(format!("mv-{}", uuid::Uuid::new_v4()));
        write_tiny_png(&dir, 1);
        let mock = MockProvider::new();
        mock.add_response("total garbage, no JSON").await;
        mock.add_response(r#"{"grounded_score": 0.92, "invented": [], "missing": []}"#)
            .await;
        let md = "<!-- edgequake-page:1 -->\n# Real content\n";
        let out = verify_manuscript_markdown(
            md,
            PageModality::Manuscript,
            Some(dir.as_path()),
            Arc::new(mock),
        )
        .await;
        assert!(
            !out.fail_open,
            "retry must recover a transient judge failure"
        );
        assert!(out.fail_reason.is_none());
        assert_eq!(out.pages_judged, 1);
        assert_eq!(out.mean_score, Some(0.92));
        assert_eq!(out.markdown, md);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[tokio::test]
    async fn empty_page_markdown_is_judged_not_silently_accepted() {
        // Observed failure mode (2026-08-20): qwen returns empty page markdown
        // at 3600px. Empty output for a page with visible content must be
        // judged low-grounding and refined — never silently accepted.
        let dir = std::env::temp_dir().join(format!("mv-{}", uuid::Uuid::new_v4()));
        write_tiny_png(&dir, 1);
        let mock = MockProvider::new();
        // Judge sees empty candidate + page image → score 0 with missing list.
        mock.add_response(
            r#"{"grounded_score": 0.0, "invented": [], "missing": ["all visible content"]}"#,
        )
        .await;
        mock.add_response("# Recovered transcription").await;
        mock.add_response(r#"{"grounded_score": 0.85, "invented": [], "missing": []}"#)
            .await;
        let md = "<!-- edgequake-page:1 -->

";
        let out = verify_manuscript_markdown(
            md,
            PageModality::Manuscript,
            Some(dir.as_path()),
            Arc::new(mock),
        )
        .await;
        assert_eq!(out.pages_low_grounding, 1, "empty page must be judged low");
        assert_eq!(out.pages_refined, 1);
        assert!(out.markdown.contains("# Recovered transcription"));
        assert!(!out.markdown.contains(GROUNDING_LOW_MARKER_PREFIX));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn quarantine_strips_marked_section_keeps_others() {
        let md = "<!-- edgequake-page:1 -->\n# Real table\n\n| a | b |\n\n<!-- edgequake-page:2 -->\n<!-- grounding:low score=0.00 -->\n\n# Fabricated histogram\n\n<!-- edgequake-page:3 -->\n# Grounded page\n";
        let out = strip_low_grounding_sections(md);
        assert!(out.contains("# Real table"), "grounded page 1 kept");
        assert!(
            !out.contains("# Fabricated histogram"),
            "marked page 2 content removed: {out}"
        );
        assert!(
            out.contains("<!-- edgequake-page:2 -->"),
            "page marker kept"
        );
        assert!(out.contains(GROUNDING_QUARANTINED_MARKER));
        assert!(out.contains("# Grounded page"), "grounded page 3 kept");
        assert!(!out.contains(GROUNDING_LOW_MARKER_PREFIX));
    }

    #[test]
    fn quarantine_noop_without_markers() {
        let md = "<!-- edgequake-page:1 -->\nplain content\n";
        assert_eq!(strip_low_grounding_sections(md), md);
    }

    #[test]
    fn quarantine_handles_marker_on_last_page() {
        let md = "<!-- edgequake-page:1 -->\nkept\n<!-- edgequake-page:2 -->\n<!-- grounding:low score=0.10 -->\n\nfabricated tail";
        let out = strip_low_grounding_sections(md);
        assert!(out.contains("kept"));
        assert!(!out.contains("fabricated tail"));
        assert!(out.contains("<!-- edgequake-page:2 -->"));
    }

    #[tokio::test]
    async fn missing_page_png_fails_open_unmodified() {
        let dir = std::env::temp_dir().join(format!("mv-{}", uuid::Uuid::new_v4()));
        let mock = MockProvider::new();
        let md = "<!-- edgequake-page:1 -->\ncontent\n";
        let out = verify_manuscript_markdown(
            md,
            PageModality::Manuscript,
            Some(dir.as_path()),
            Arc::new(mock),
        )
        .await;
        assert!(out.fail_open);
        assert_eq!(out.fail_reason.as_deref(), Some("page_png_missing"));
        assert_eq!(out.markdown, md);
    }

    #[tokio::test]
    async fn escalation_recovers_empty_page() {
        let dir = std::env::temp_dir().join(format!("esc-{}", uuid::Uuid::new_v4()));
        write_tiny_png(&dir, 2);
        let mock = MockProvider::new();
        mock.add_response("# Recovered fatigue table\n\n| Acier | L10^7 |")
            .await;
        let md = "<!-- edgequake-page:1 -->\n# Real content\n\n<!-- edgequake-page:2 -->\n\n*[No text extracted for this page; see page image below.]*\n";
        let out = escalate_empty_pages(md, &dir, Arc::new(mock), PageModality::Manuscript).await;
        assert_eq!(out.pages_escalated, vec![2]);
        assert!(out.pages_failed.is_empty());
        assert!(out.markdown.contains("# Recovered fatigue table"));
        assert!(out.markdown.contains("# Real content"));
        assert!(!out.markdown.contains("No text extracted"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[tokio::test]
    async fn escalation_keeps_placeholder_and_records_failure() {
        let dir = std::env::temp_dir().join(format!("esc-{}", uuid::Uuid::new_v4()));
        write_tiny_png(&dir, 1);
        let mock = MockProvider::new();
        mock.add_response("").await; // provider degraded: empty response
        let md = "<!-- edgequake-page:1 -->\n\n*[No text extracted for this page; see page image below.]*\n";
        let out = escalate_empty_pages(md, &dir, Arc::new(mock), PageModality::Manuscript).await;
        assert!(out.pages_escalated.is_empty());
        assert_eq!(out.pages_failed, vec![1]);
        assert_eq!(out.markdown, md, "placeholder kept on failure");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[tokio::test]
    async fn escalation_noop_without_empty_pages() {
        let dir = std::env::temp_dir().join(format!("esc-{}", uuid::Uuid::new_v4()));
        let mock = MockProvider::new();
        let md = "<!-- edgequake-page:1 -->\ncontent\n";
        let out = escalate_empty_pages(md, &dir, Arc::new(mock), PageModality::Manuscript).await;
        assert!(out.pages_escalated.is_empty() && out.pages_failed.is_empty());
        assert_eq!(out.markdown, md);
    }

    #[tokio::test]
    async fn escalation_skips_when_page_png_missing() {
        let dir = std::env::temp_dir().join(format!("esc-{}", uuid::Uuid::new_v4()));
        let mock = MockProvider::new();
        let md = "<!-- edgequake-page:1 -->\n\n*[No text extracted for this page; see page image below.]*\n";
        let out = escalate_empty_pages(md, &dir, Arc::new(mock), PageModality::Manuscript).await;
        assert!(out.pages_escalated.is_empty() && out.pages_failed.is_empty());
        assert_eq!(out.markdown, md);
    }

    #[tokio::test]
    async fn escalation_runs_when_placeholder_has_prepended_crop() {
        let dir = std::env::temp_dir().join(format!("esc-{}", uuid::Uuid::new_v4()));
        write_tiny_png(&dir, 1);
        let mock = MockProvider::new();
        mock.add_response("# Recovered from crop-defeated placeholder")
            .await;
        let md = "<!-- edgequake-page:1 -->\n\n![scan](assets/page-0001-fig-01.png)\n\n*[No text extracted for this page; see page image below.]*\n";
        let out = escalate_empty_pages(md, &dir, Arc::new(mock), PageModality::Manuscript).await;
        assert_eq!(out.pages_escalated, vec![1]);
        assert!(out
            .markdown
            .contains("# Recovered from crop-defeated placeholder"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[tokio::test]
    async fn verify_scoped_skips_print_pages_in_mixed_doc() {
        let dir = std::env::temp_dir().join(format!("mv-{}", uuid::Uuid::new_v4()));
        write_tiny_png(&dir, 2);
        let mock = MockProvider::new();
        // Only page 2 should be judged.
        mock.add_response(r#"{"grounded_score": 0.95, "invented": [], "missing": []}"#)
            .await;
        let md =
            "<!-- edgequake-page:1 -->\nprint content\n\n<!-- edgequake-page:2 -->\nms content\n";
        let out = verify_manuscript_markdown_scoped(
            md,
            PageModality::Mixed,
            Some(dir.as_path()),
            Arc::new(mock),
            Some(&[2usize]),
            None,
        )
        .await;
        assert!(out.ran);
        assert_eq!(out.pages_judged, 1);
        assert!(out.markdown.contains("print content"));
        assert!(out.markdown.contains("ms content"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[tokio::test]
    async fn escalate_scoped_skips_print_placeholders() {
        let dir = std::env::temp_dir().join(format!("esc-{}", uuid::Uuid::new_v4()));
        write_tiny_png(&dir, 1);
        write_tiny_png(&dir, 2);
        let mock = MockProvider::new();
        mock.add_response("# Recovered manuscript page").await;
        let placeholder = edgequake_pdf::EMPTY_VISION_PAGE_PLACEHOLDER;
        let md = format!(
            "<!-- edgequake-page:1 -->\n\n{placeholder}\n\n<!-- edgequake-page:2 -->\n\n{placeholder}\n"
        );
        let out = escalate_empty_pages_scoped(
            &md,
            &dir,
            Arc::new(mock),
            PageModality::Mixed,
            Some(&[2usize]),
            None,
        )
        .await;
        assert_eq!(out.pages_escalated, vec![2]);
        assert!(out.markdown.contains("# Recovered manuscript page"));
        // Print page 1 placeholder must remain (not re-OCR'd).
        assert!(out.markdown.contains("<!-- edgequake-page:1 -->"));
        let _ = std::fs::remove_dir_all(&dir);
    }
}
