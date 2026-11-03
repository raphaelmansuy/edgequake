//! Weighted reciprocal-rank fusion with a stable order.
//!
//! The scoring is [`crate::fusion::reciprocal_rank_fusion`] with `k = 60`
//! ([`crate::fusion::RRF_K`]); only the tie-break is added here, because that function
//! sorts a `HashMap` by score alone and equal scores would otherwise come out in a
//! different order from run to run. No `min_score` is applied after fusion: an RRF
//! score is a rank signal, not a similarity (SPEC-180 09).

use std::collections::HashMap;

use crate::fusion::{reciprocal_rank_fusion, RRF_K};

/// A fused chunk id with its RRF score.
#[derive(Debug, Clone, PartialEq)]
pub struct FusedHit {
    pub id: String,
    pub score: f32,
}

/// Fuse ranked id lists. `lists[0]` is the primary dense list: it breaks score ties
/// (better dense rank first), then the id ascending.
pub fn fuse_ranked(lists: &[Vec<String>], weights: &[f32]) -> Vec<FusedHit> {
    let dense_rank: HashMap<&str, usize> = lists
        .first()
        .map(|l| {
            l.iter()
                .enumerate()
                .map(|(i, id)| (id.as_str(), i))
                .collect()
        })
        .unwrap_or_default();
    let mut fused = reciprocal_rank_fusion(lists, weights, RRF_K);
    fused.sort_by(|a, b| {
        b.1.partial_cmp(&a.1)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| {
                let ra = dense_rank.get(a.0.as_str()).copied().unwrap_or(usize::MAX);
                let rb = dense_rank.get(b.0.as_str()).copied().unwrap_or(usize::MAX);
                ra.cmp(&rb)
            })
            .then_with(|| a.0.cmp(&b.0))
    });
    fused
        .into_iter()
        .map(|(id, score)| FusedHit { id, score })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ids(hits: &[FusedHit]) -> Vec<&str> {
        hits.iter().map(|h| h.id.as_str()).collect()
    }

    fn list(v: &[&str]) -> Vec<String> {
        v.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn consensus_beats_a_single_list_win() {
        let hits = fuse_ranked(
            &[list(&["a", "b", "c"]), list(&["d", "b", "e"])],
            &[1.0, 1.0],
        );
        assert_eq!(ids(&hits)[0], "b", "b is second in both lists");
    }

    #[test]
    fn one_empty_list_keeps_the_other_order() {
        let hits = fuse_ranked(&[list(&["x", "y", "z"]), vec![]], &[1.0, 1.25]);
        assert_eq!(ids(&hits), ["x", "y", "z"]);
        let hits = fuse_ranked(&[vec![], list(&["x", "y"])], &[1.0, 1.25]);
        assert_eq!(ids(&hits), ["x", "y"]);
    }

    #[test]
    fn ties_prefer_dense_rank_then_id() {
        // a: dense 0, lexical 1. b: dense 1, lexical 0. Equal weights give equal scores.
        let hits = fuse_ranked(&[list(&["a", "b"]), list(&["b", "a"])], &[1.0, 1.0]);
        assert_eq!(hits[0].score, hits[1].score);
        assert_eq!(ids(&hits), ["a", "b"]);
        // Both only lexical: ids ascending.
        let hits = fuse_ranked(&[vec![], list(&["q"]), list(&["p"])], &[1.0, 1.0, 1.0]);
        assert_eq!(ids(&hits), ["p", "q"]);
    }

    #[test]
    fn bm25_weight_can_outrank_a_dense_win() {
        let hits = fuse_ranked(&[list(&["d", "l"]), list(&["l", "d"])], &[1.0, 1.25]);
        assert_eq!(ids(&hits)[0], "l");
    }

    #[test]
    fn order_is_stable_across_runs() {
        let a = list(&["1", "2", "3", "4", "5"]);
        let b = list(&["5", "4", "3", "2", "1"]);
        let first = fuse_ranked(&[a.clone(), b.clone()], &[1.0, 1.0]);
        for _ in 0..20 {
            assert_eq!(first, fuse_ranked(&[a.clone(), b.clone()], &[1.0, 1.0]));
        }
    }

    #[test]
    fn every_rrf_score_is_kept_even_when_it_is_far_below_a_similarity_threshold() {
        // LAW-180-6 / F-180-12 / EC-180-36: an RRF score is a rank signal, not a cosine. With
        // k = 60 even a first place is 1/61 ≈ 0.0164, so applying `min_score` (default 0.1, the
        // cosine threshold) after fusion would empty a perfectly good result set. `fuse_ranked`
        // therefore must not filter, and a deep list position must still come back.
        let deep: Vec<String> = (0..200).map(|i| format!("c{i}")).collect();
        let hits = fuse_ranked(std::slice::from_ref(&deep), &[1.0]);
        assert_eq!(hits.len(), 200, "fusion drops nothing");

        let last = hits.last().expect("at least one hit");
        assert!(
            last.score < 0.1,
            "the tail of a long list scores below the cosine threshold ({}); \
             filtering it would delete real hits",
            last.score
        );
        assert!(last.score > 0.0, "and it is still a positive rank score");
        assert_eq!(hits[0].id, "c0");
        assert_eq!(last.id, "c199");
    }
}
