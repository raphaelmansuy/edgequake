//! Maximal marginal relevance for a Normal workspace.
//!
//! Two overlapping windows of the same paragraph can both survive fusion and
//! then spend the token budget twice. MMR keeps the relevant chunk and drops
//! the near-copy. A chunk with no vector (an image, or a read miss) is never
//! dropped for diversity: it competes on relevance alone.

/// Cosine at or above this is the same passage written twice. Similar but
/// distinct chunks (below this) both stay when there is room for them.
const DUPLICATE_COSINE: f32 = 0.995;

/// `EDGEQUAKE_NORMAL_MMR`. Default on. `0` / `false` / `off` / `no` disables.
pub const MMR_ENV: &str = "EDGEQUAKE_NORMAL_MMR";
/// `EDGEQUAKE_NORMAL_MMR_LAMBDA`. `1` is pure relevance, `0` is pure diversity.
pub const MMR_LAMBDA_ENV: &str = "EDGEQUAKE_NORMAL_MMR_LAMBDA";
pub const DEFAULT_MMR_LAMBDA: f32 = 0.7;

/// One candidate. `relevance` is the fused score; this function normalises it.
#[derive(Debug, Clone)]
pub struct MmrCandidate {
    pub id: String,
    pub relevance: f32,
    pub vector: Option<Vec<f32>>,
}

/// `true` unless the env explicitly turns MMR off.
pub fn mmr_enabled(raw: Option<&str>) -> bool {
    !matches!(
        raw.map(str::trim).map(str::to_ascii_lowercase).as_deref(),
        Some("0") | Some("false") | Some("off") | Some("no")
    )
}

pub fn mmr_enabled_from_env() -> bool {
    mmr_enabled(std::env::var(MMR_ENV).ok().as_deref())
}

/// A missing or unusable value is the default. MMR is an optimisation and a
/// bad knob must not fail the query.
pub fn mmr_lambda(raw: Option<&str>) -> f32 {
    let Some(raw) = raw.map(str::trim).filter(|r| !r.is_empty()) else {
        return DEFAULT_MMR_LAMBDA;
    };
    match raw.parse::<f32>() {
        Ok(v) if v.is_finite() && (0.0..=1.0).contains(&v) => v,
        _ => {
            tracing::warn!(
                value = raw,
                default = DEFAULT_MMR_LAMBDA,
                "{MMR_LAMBDA_ENV} must be between 0 and 1; using the default"
            );
            DEFAULT_MMR_LAMBDA
        }
    }
}

pub fn mmr_lambda_from_env() -> f32 {
    mmr_lambda(std::env::var(MMR_LAMBDA_ENV).ok().as_deref())
}

/// Select up to `keep` candidates.
///
/// - relevance is scaled to 0..1 from the scores in this batch;
/// - diversity is the max cosine to anything already picked (`0` when the
///   vector is missing, so the item competes on relevance alone);
/// - a near-copy (cosine ≥ 0.995) never consumes a slot;
/// - `keep >= len` with no near-copies returns the input unchanged;
/// - no vectors at all returns the input unchanged;
/// - ties break by id, then by input order.
pub fn mmr_select(
    mut candidates: Vec<MmrCandidate>,
    lambda: f32,
    keep: usize,
) -> Vec<MmrCandidate> {
    if candidates.len() <= 1 {
        return candidates;
    }
    if !candidates.iter().any(usable_vector) {
        return candidates;
    }
    let lambda = lambda.clamp(0.0, 1.0);
    normalize_relevance(&mut candidates);
    let keep = keep.max(1);
    if keep >= candidates.len() {
        return drop_near_copies(candidates);
    }
    fill_by_mmr(candidates, lambda, keep)
}

fn usable_vector(c: &MmrCandidate) -> bool {
    c.vector
        .as_ref()
        .is_some_and(|v| v.iter().any(|x| *x != 0.0))
}

fn normalize_relevance(candidates: &mut [MmrCandidate]) {
    let mut min = f32::MAX;
    let mut max = f32::MIN;
    for c in candidates.iter() {
        let r = finite(c.relevance);
        min = min.min(r);
        max = max.max(r);
    }
    let span = max - min;
    for c in candidates.iter_mut() {
        let r = finite(c.relevance);
        c.relevance = if span <= f32::EPSILON {
            1.0
        } else {
            (r - min) / span
        };
    }
}

fn finite(v: f32) -> f32 {
    if v.is_finite() {
        v
    } else {
        0.0
    }
}

/// Input order is the retrieval order (best first). Drop a later near-copy.
/// If nothing is a near-copy, the output is the input.
fn drop_near_copies(candidates: Vec<MmrCandidate>) -> Vec<MmrCandidate> {
    let mut kept: Vec<MmrCandidate> = Vec::with_capacity(candidates.len());
    for candidate in candidates {
        if is_near_copy(&candidate, &kept) {
            continue;
        }
        kept.push(candidate);
    }
    kept
}

fn fill_by_mmr(candidates: Vec<MmrCandidate>, lambda: f32, keep: usize) -> Vec<MmrCandidate> {
    let mut remaining = candidates;
    let mut kept: Vec<MmrCandidate> = Vec::with_capacity(keep.min(remaining.len()));
    while kept.len() < keep && !remaining.is_empty() {
        let mut best_at = 0usize;
        let mut best_score = f32::MIN;
        let mut best_id = String::new();
        for (i, candidate) in remaining.iter().enumerate() {
            if is_near_copy(candidate, &kept) {
                continue;
            }
            let score = mmr_score(candidate, &kept, lambda);
            let better = score > best_score + 1e-6
                || ((score - best_score).abs() <= 1e-6
                    && (best_id.is_empty() || candidate.id < best_id));
            if better {
                best_at = i;
                best_score = score;
                best_id = candidate.id.clone();
            }
        }
        // Every remaining item is a near-copy of something already kept.
        if best_id.is_empty() {
            break;
        }
        kept.push(remaining.remove(best_at));
    }
    kept
}

fn is_near_copy(candidate: &MmrCandidate, kept: &[MmrCandidate]) -> bool {
    max_cosine(candidate, kept) >= DUPLICATE_COSINE
}

fn mmr_score(candidate: &MmrCandidate, kept: &[MmrCandidate], lambda: f32) -> f32 {
    // Missing vector → max cosine 0, so the score is relevance alone.
    lambda * candidate.relevance - (1.0 - lambda) * max_cosine(candidate, kept)
}

fn max_cosine(candidate: &MmrCandidate, kept: &[MmrCandidate]) -> f32 {
    let Some(vector) = candidate.vector.as_deref().filter(|v| !v.is_empty()) else {
        return 0.0;
    };
    kept.iter()
        .filter_map(|k| k.vector.as_deref())
        .map(|other| cosine(vector, other).max(0.0))
        .fold(0.0_f32, f32::max)
}

fn cosine(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() || a.is_empty() {
        return 0.0;
    }
    let mut dot = 0.0_f32;
    let mut na = 0.0_f32;
    let mut nb = 0.0_f32;
    for i in 0..a.len() {
        dot += a[i] * b[i];
        na += a[i] * a[i];
        nb += b[i] * b[i];
    }
    if na <= f32::EPSILON || nb <= f32::EPSILON {
        return 0.0;
    }
    (dot / (na.sqrt() * nb.sqrt())).clamp(-1.0, 1.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cand(id: &str, relevance: f32, vector: Option<Vec<f32>>) -> MmrCandidate {
        MmrCandidate {
            id: id.into(),
            relevance,
            vector,
        }
    }

    fn ids(xs: &[MmrCandidate]) -> Vec<&str> {
        xs.iter().map(|c| c.id.as_str()).collect()
    }

    #[test]
    fn identical_duplicate_keeps_the_best_score() {
        let out = mmr_select(
            vec![
                cand("a", 0.9, Some(vec![1.0, 0.0])),
                cand("b", 0.4, Some(vec![1.0, 0.0])),
            ],
            0.7,
            2,
        );
        assert_eq!(ids(&out), ["a"]);
    }

    #[test]
    fn similar_but_distinct_chunks_both_survive() {
        // Cosine ≈ 0.89, under the near-copy cut. Room for both.
        let out = mmr_select(
            vec![
                cand("a", 0.9, Some(vec![1.0, 0.0])),
                cand("b", 0.8, Some(vec![0.8, 0.6])),
            ],
            0.7,
            2,
        );
        assert_eq!(ids(&out), ["a", "b"]);
    }

    #[test]
    fn missing_vectors_keep_input_order() {
        let out = mmr_select(
            vec![
                cand("b", 0.2, None),
                cand("a", 0.9, None),
                cand("c", 0.5, Some(vec![])),
            ],
            0.0,
            1,
        );
        assert_eq!(ids(&out), ["b", "a", "c"]);
    }

    #[test]
    fn keep_covering_everyone_is_a_no_op_without_copies() {
        let input = vec![
            cand("b", 0.2, Some(vec![1.0, 0.0])),
            cand("a", 0.9, Some(vec![0.0, 1.0])),
        ];
        let out = mmr_select(input.clone(), 0.0, 5);
        assert_eq!(ids(&out), ["b", "a"]);
    }

    #[test]
    fn a_single_candidate_is_unchanged() {
        let out = mmr_select(vec![cand("only", 0.4, Some(vec![1.0]))], 0.0, 1);
        assert_eq!(ids(&out), ["only"]);
    }

    #[test]
    fn an_image_without_a_vector_is_never_dropped() {
        let out = mmr_select(
            vec![
                cand("text", 0.9, Some(vec![1.0, 0.0])),
                cand("image", 0.5, None),
                cand("copy", 0.8, Some(vec![1.0, 0.0])),
            ],
            0.7,
            2,
        );
        assert!(ids(&out).contains(&"image"), "{:?}", ids(&out));
        assert!(!ids(&out).contains(&"copy"), "{:?}", ids(&out));
    }

    #[test]
    fn lambda_one_is_pure_relevance() {
        let out = mmr_select(
            vec![
                cand("low", 0.1, Some(vec![1.0, 0.0])),
                cand("high", 0.9, Some(vec![0.0, 1.0])),
                cand("mid", 0.4, Some(vec![0.7, 0.7])),
            ],
            1.0,
            2,
        );
        assert_eq!(ids(&out), ["high", "mid"]);
    }

    #[test]
    fn lambda_zero_picks_diversity_after_the_first() {
        // First pick is still the most relevant (nothing selected yet).
        // The next pick is the one least similar to it, not the next score.
        let out = mmr_select(
            vec![
                cand("best", 1.0, Some(vec![1.0, 0.0])),
                cand("near", 0.8, Some(vec![0.95, 0.3122])),
                cand("far", 0.2, Some(vec![0.0, 1.0])),
            ],
            0.0,
            2,
        );
        assert_eq!(ids(&out), ["best", "far"]);
    }

    #[test]
    fn order_is_stable_across_runs() {
        let once = mmr_select(
            vec![
                cand("b", 0.5, Some(vec![1.0, 0.0, 0.0])),
                cand("a", 0.5, Some(vec![0.0, 1.0, 0.0])),
                cand("c", 0.5, Some(vec![0.0, 0.0, 1.0])),
            ],
            0.7,
            2,
        );
        let first = ids(&once);
        for _ in 0..20 {
            let again = mmr_select(
                vec![
                    cand("b", 0.5, Some(vec![1.0, 0.0, 0.0])),
                    cand("a", 0.5, Some(vec![0.0, 1.0, 0.0])),
                    cand("c", 0.5, Some(vec![0.0, 0.0, 1.0])),
                ],
                0.7,
                2,
            );
            assert_eq!(ids(&again), first);
        }
    }

    #[test]
    fn ties_break_by_id() {
        let out = mmr_select(
            vec![
                cand("b", 0.5, Some(vec![1.0, 0.0])),
                cand("a", 0.5, Some(vec![0.0, 1.0])),
            ],
            1.0,
            1,
        );
        assert_eq!(ids(&out), ["a"]);
    }

    #[test]
    fn env_defaults_on_and_rejects_only_the_off_words() {
        assert!(mmr_enabled(None));
        assert!(mmr_enabled(Some("")));
        assert!(mmr_enabled(Some("1")));
        assert!(!mmr_enabled(Some("0")));
        assert!(!mmr_enabled(Some("false")));
        assert!(!mmr_enabled(Some("OFF")));
        assert_eq!(mmr_lambda(None), DEFAULT_MMR_LAMBDA);
        assert_eq!(mmr_lambda(Some("0")), 0.0);
        assert_eq!(mmr_lambda(Some("1")), 1.0);
        assert_eq!(mmr_lambda(Some("nope")), DEFAULT_MMR_LAMBDA);
        assert_eq!(mmr_lambda(Some("1.5")), DEFAULT_MMR_LAMBDA);
    }
}
