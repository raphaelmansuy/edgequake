//! SPEC-184 — an absolute relevance floor for a normal workspace.
//!
//! # The defect this closes
//!
//! A normal workspace fuses its two retrieval legs by reciprocal rank, and an RRF score is a
//! **rank signal, not a similarity**: first place is `1/(60+1) ≈ 0.0164` whether the match is
//! perfect or meaningless. So no threshold can be applied to `score` after fusion, and the
//! retriever correctly applies none ([`super::fuse`]).
//!
//! The consequence was that context carried *whatever retrieval returned*. On a corpus of
//! photographs that is the whole result set: an image chunk has no text body, so the lexical leg
//! is silent and the dense leg alone always returns `k` rows, in arbitrary order, at the noise
//! floor of cross-modal cosine. The screen showed thirteen pictures it had judged relevant — it
//! applies this same rule — while the model reasoned over twenty it was never told were dubious,
//! and answered "there is no textual evidence to answer questions about …".
//!
//! That answer was correct, and it should never have been asked.
//!
//! # The rule
//!
//! Keep a chunk when it has no measured similarity (nothing to judge), or when its similarity is
//! at least the floor. Judge **pictures more strictly** than text, because the floor's origin is a
//! measurement of text queries against pictures: a text query over a text corpus has a high
//! baseline similarity that says little about relevance, while the cross-modal floor is the
//! hardest case measured and the one where an unrunnable match actually hurts.
//!
//! Applying the floor only when a similarity exists keeps the fail-open properties the rest of the
//! pack relies on: a lexical-only hit, a stored row from before the field existed, or a graph
//! workspace that never recorded a cosine is kept, not silently dropped.

use edgequake_pipeline::MODALITY_IMAGE;

/// Minimum cosine for a **text** chunk to reach the prompt.
///
/// This is deliberately the same `0.1` the rest of the pack already applies to chunks
/// (`QueryEngineConfig::min_score`, applied in `modes::naive`), rather than a stricter number
/// invented here. Same-modality cosine is not calibrated across corpora or models, so a threshold
/// on it is a floor against obvious noise and nothing more; raising it silently would discard
/// legitimate evidence on corpora whose baselines are lower. The picture floor below is stricter
/// because it comes from a measurement, and this one does not.
pub const MIN_TEXT_SIMILARITY: f32 = 0.10;

/// Minimum cosine for an **image** chunk to reach the prompt.
///
/// This is the constant the web UI already uses (`MIN_IMAGE_SIMILARITY` in
/// `lib/citations/image-matches.ts`), taken from measurements on a 20-picture corpus embedded with
/// `gemini-embedding-2`:
///
/// ```text
///   "a golden retriever puppy"   0.715 0.610 0.569 0.530 0.522 … (noise floor ≈ 0.52)
///   "what does the handbook say" 0.473 0.467 0.466 …, nothing above 0.48
/// ```
///
/// Unrelated pairs never exceed ~0.48. Keeping the two sides equal means the grid and the prompt
/// agree about which pictures are evidence — the disagreement between them is what made the
/// original answer unreadable.
pub const MIN_IMAGE_SIMILARITY: f32 = 0.48;

/// How far below the best picture another may fall and still reach the prompt.
///
/// WHY this is needed on top of the absolute floor: the measurement above shows that the
/// *runner-up of a good query* (0.61, 0.57) is similar to the *second result of a query with no
/// answer at all* (0.47), so an absolute threshold alone cannot separate "a legitimate second
/// match" from "noise that came back because the store always returns k rows". The best match
/// sets the scale for the rest — the same rule, and the same 0.15, the image grid uses.
pub const IMAGE_SIMILARITY_MARGIN: f32 = 0.15;

/// The floor that applies to one chunk, by modality.
pub fn floor_for(is_image: bool) -> f32 {
    if is_image {
        MIN_IMAGE_SIMILARITY
    } else {
        MIN_TEXT_SIMILARITY
    }
}

/// What the gate decided, for the trace. Counts only, never chunk text.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct RelevanceGate {
    /// Chunks dropped because they fell below the floor.
    pub dropped: usize,
    /// Chunks kept.
    pub kept: usize,
    /// Chunks kept despite having no measured similarity.
    pub unmeasured_kept: usize,
}

impl RelevanceGate {
    /// Did the floor remove everything? The caller turns this into an honest "no evidence"
    /// answer instead of asking a model to reason over an empty context.
    pub fn emptied(&self) -> bool {
        self.kept == 0 && self.dropped > 0
    }
}

/// A chunk with its relevance signal, so this module needs no dependency on the context type.
pub struct Candidate {
    pub similarity: Option<f32>,
    /// True when the chunk's source is a picture (no text body).
    pub is_image: bool,
}

/// Should this chunk reach the prompt when judged on its own?
///
/// `None` similarity means "not measured", which is kept: an absent score is not a bad score.
/// This is the absolute half of the rule; [`apply_floor`] adds the relative half.
pub fn is_relevant(candidate: &Candidate) -> bool {
    match candidate.similarity {
        None => true,
        Some(similarity) => similarity >= floor_for(candidate.is_image),
    }
}

/// Apply the full rule to a set of candidates, returning which to keep and what was decided.
///
/// Two halves, and both are necessary:
/// 1. **Absolute** — the picture floor rejects a query whose best match never got near the
///    corpus's noise ceiling.
/// 2. **Relative** — among pictures that clear it, only those within [`IMAGE_SIMILARITY_MARGIN`]
///    of the best match are the same kind of thing as the answer.
///
/// The relative half applies to pictures only. A text corpus's cosine baseline is high and
/// uncalibrated, so trimming text chunks against their own best would discard legitimate evidence.
///
/// Returns `Vec<bool>` aligned with `candidates`, plus the gate for the trace.
pub fn apply_floor(candidates: &[Candidate]) -> (Vec<bool>, RelevanceGate) {
    let best_picture = candidates
        .iter()
        .filter(|c| c.is_image)
        .filter_map(|c| c.similarity)
        .fold(None::<f32>, |acc, s| {
            Some(acc.map_or(s, |best| best.max(s)))
        });

    let mut keep = Vec::with_capacity(candidates.len());
    let mut gate = RelevanceGate::default();
    for candidate in candidates {
        let kept = if !is_relevant(candidate) {
            false
        } else {
            match (candidate.is_image, candidate.similarity, best_picture) {
                (true, Some(similarity), Some(best)) => {
                    similarity >= best - IMAGE_SIMILARITY_MARGIN
                }
                _ => true,
            }
        };
        if kept {
            gate.kept += 1;
            if candidate.similarity.is_none() {
                gate.unmeasured_kept += 1;
            }
        } else {
            gate.dropped += 1;
        }
        keep.push(kept);
    }
    (keep, gate)
}

/// Classify a chunk's stored modalitys string. Shared so the retriever and the pipeline agree.
pub fn is_image_modality(modality: Option<&str>, source_modality: Option<&str>) -> bool {
    modality == Some(MODALITY_IMAGE)
        || source_modality == Some(MODALITY_IMAGE)
        || modality.is_some_and(|m| m.eq_ignore_ascii_case(MODALITY_IMAGE))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn text(similarity: Option<f32>) -> Candidate {
        Candidate {
            similarity,
            is_image: false,
        }
    }

    fn picture(similarity: Option<f32>) -> Candidate {
        Candidate {
            similarity,
            is_image: true,
        }
    }

    #[test]
    fn the_measured_noise_floor_is_rejected_for_pictures() {
        // The exact numbers in the module docs: a text query with no answer never exceeds ~0.48.
        for noise in [0.473, 0.467, 0.466, 0.40] {
            assert!(!is_relevant(&picture(Some(noise))), "{noise} is noise");
        }
        // And a real cross-modal match is kept.
        for real in [0.715, 0.610, 0.569] {
            assert!(is_relevant(&picture(Some(real))), "{real} matched");
        }
    }

    #[test]
    fn a_picture_faces_a_stricter_floor_than_text() {
        assert!(floor_for(true) > floor_for(false));
        // 0.40 clears the text floor and fails the picture floor: the asymmetry in one assertion.
        assert!(is_relevant(&text(Some(0.40))));
        assert!(!is_relevant(&picture(Some(0.40))));
    }

    #[test]
    fn the_text_floor_is_the_one_the_pack_already_uses() {
        // Not a new number: the same 0.1 that `min_score` applies to chunks elsewhere. Inventing a
        // stricter value here would silently drop evidence on corpora with lower baselines.
        assert_eq!(MIN_TEXT_SIMILARITY, 0.10);
        assert!(is_relevant(&text(Some(MIN_TEXT_SIMILARITY))));
        assert!(!is_relevant(&text(Some(0.05))));
    }

    #[test]
    fn an_unmeasured_similarity_is_kept_never_dropped() {
        // A lexical-only hit, or a stored row from before the field existed. Absent is not bad.
        assert!(is_relevant(&text(None)));
        assert!(is_relevant(&picture(None)));
    }

    #[test]
    fn the_gate_reports_an_emptied_context_so_the_caller_can_say_so() {
        let gate = RelevanceGate {
            dropped: 20,
            kept: 0,
            unmeasured_kept: 0,
        };
        assert!(gate.emptied());
        // Dropping some but keeping others is a normal query, not an empty one.
        let partial = RelevanceGate {
            dropped: 19,
            kept: 1,
            unmeasured_kept: 0,
        };
        assert!(!partial.emptied());
        // A corpus that was empty to begin with is not "emptied by the gate".
        assert!(!RelevanceGate::default().emptied());
    }

    #[test]
    fn modality_detection_accepts_either_field() {
        assert!(is_image_modality(Some("image"), None));
        assert!(is_image_modality(None, Some("image")));
        assert!(is_image_modality(Some("IMAGE"), None));
        assert!(!is_image_modality(Some("table"), Some("text")));
        assert!(!is_image_modality(None, None));
    }

    fn pictures(sims: &[f32]) -> Vec<Candidate> {
        sims.iter().map(|s| picture(Some(*s))).collect()
    }

    #[test]
    fn the_relative_margin_keeps_the_measured_runner_ups_and_drops_the_rest() {
        // The row from the module docs: a good query. Best 0.715 admits 0.610 (Δ0.105) and
        // 0.569 (Δ0.146), and stops at 0.530 (Δ0.185) — exactly the split a reader makes.
        let candidates = pictures(&[0.715, 0.610, 0.569, 0.530, 0.522]);
        let (keep, gate) = apply_floor(&candidates);
        assert_eq!(keep, vec![true, true, true, false, false]);
        assert_eq!(gate.kept, 3);
        assert_eq!(gate.dropped, 2);
        assert!(!gate.emptied());
    }

    #[test]
    fn a_query_with_no_answer_loses_every_picture() {
        // The other row: "what does the handbook say" against the same corpus. Nothing clears the
        // absolute floor, so the context is empty and the caller can say why.
        let candidates = pictures(&[0.473, 0.467, 0.466, 0.44]);
        let (keep, gate) = apply_floor(&candidates);
        assert_eq!(keep, vec![false, false, false, false]);
        assert!(gate.emptied(), "all four were noise");
    }

    #[test]
    fn the_relative_half_does_not_trim_text_chunks_against_each_other() {
        // Same-modality cosine is not calibrated the way cross-modal is; trimming text against its
        // own best would discard legitimate evidence. Only the absolute floor applies.
        let candidates = vec![text(Some(0.80)), text(Some(0.42)), text(Some(0.31))];
        let (keep, gate) = apply_floor(&candidates);
        assert_eq!(keep, vec![true, true, true]);
        assert_eq!(gate.dropped, 0);
    }

    #[test]
    fn an_unmeasured_picture_does_not_set_the_scale() {
        // A picture with no cosine cannot define "the best"; it is kept on its own terms, and the
        // measured ones are still judged against the best *measured* value.
        let candidates = vec![
            Candidate {
                similarity: None,
                is_image: true,
            },
            picture(Some(0.70)),
            picture(Some(0.40)),
        ];
        let (keep, gate) = apply_floor(&candidates);
        assert_eq!(keep, vec![true, true, false]);
        assert_eq!(gate.unmeasured_kept, 1);
        assert_eq!(gate.dropped, 1);
    }

    #[test]
    fn a_mixed_context_judges_each_modality_by_its_own_rule() {
        // 0.20 is below the picture floor but well above the text floor: the chunk's kind decides,
        // and the text chunk of the same score survives.
        let candidates = vec![text(Some(0.20)), picture(Some(0.20))];
        let (keep, _) = apply_floor(&candidates);
        assert_eq!(keep, vec![true, false]);
    }

    #[test]
    fn an_empty_corpus_is_not_reported_as_emptied_by_the_gate() {
        let (keep, gate) = apply_floor(&[]);
        assert!(keep.is_empty());
        assert!(
            !gate.emptied(),
            "nothing was dropped; there was nothing to drop"
        );
    }
}
