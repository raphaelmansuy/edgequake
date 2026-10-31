//! Retrieval metrics for the Normal hybrid harness.
//!
//! Pure functions over ranked ids. They do not touch a model or a database.
//! A hermetic fixture proves the plumbing; the live number is `make bench`.

use std::collections::HashSet;

/// Fraction of `relevant` ids present in the first `k` ranks.
/// An empty relevant set is a vacuous 1.
pub fn recall_at_k(ranked: &[String], relevant: &[String], k: usize) -> f32 {
    if relevant.is_empty() {
        return 1.0;
    }
    let top: HashSet<&str> = ranked.iter().take(k).map(String::as_str).collect();
    let hits = relevant
        .iter()
        .filter(|id| top.contains(id.as_str()))
        .count();
    hits as f32 / relevant.len() as f32
}

/// Reciprocal rank of the first relevant id inside `k`, else 0.
pub fn mrr_at_k(ranked: &[String], relevant: &[String], k: usize) -> f32 {
    let relevant: HashSet<&str> = relevant.iter().map(String::as_str).collect();
    ranked
        .iter()
        .take(k)
        .position(|id| relevant.contains(id.as_str()))
        .map(|rank| 1.0 / (rank as f32 + 1.0))
        .unwrap_or(0.0)
}

/// Binary nDCG at `k`. An empty relevant set is a vacuous 1.
pub fn ndcg_at_k(ranked: &[String], relevant: &[String], k: usize) -> f32 {
    if relevant.is_empty() {
        return 1.0;
    }
    let relevant: HashSet<&str> = relevant.iter().map(String::as_str).collect();
    let dcg = dcg_at(ranked, &relevant, k);
    let ideal: Vec<String> = relevant.iter().map(|id| (*id).to_string()).collect();
    let idcg = dcg_at(&ideal, &relevant, k);
    if idcg <= f32::EPSILON {
        0.0
    } else {
        dcg / idcg
    }
}

fn dcg_at(ranked: &[String], relevant: &HashSet<&str>, k: usize) -> f32 {
    ranked
        .iter()
        .take(k)
        .enumerate()
        .map(|(i, id)| {
            let gain = if relevant.contains(id.as_str()) {
                1.0
            } else {
                0.0
            };
            gain / ((i as f32) + 2.0).log2()
        })
        .sum()
}

/// Mean of a non-empty slice. Empty is 0.
pub fn mean(values: &[f32]) -> f32 {
    if values.is_empty() {
        0.0
    } else {
        values.iter().sum::<f32>() / values.len() as f32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ids(xs: &[&str]) -> Vec<String> {
        xs.iter().map(|s| (*s).to_string()).collect()
    }

    #[test]
    fn recall_counts_hits_inside_k() {
        let ranked = ids(&["a", "b", "c"]);
        let relevant = ids(&["b", "c"]);
        assert!((recall_at_k(&ranked, &relevant, 1) - 0.0).abs() < 1e-6);
        assert!((recall_at_k(&ranked, &relevant, 2) - 0.5).abs() < 1e-6);
        assert!((recall_at_k(&ranked, &relevant, 3) - 1.0).abs() < 1e-6);
        assert!((recall_at_k(&ranked, &[], 3) - 1.0).abs() < 1e-6);
    }

    #[test]
    fn mrr_is_the_first_hit() {
        let ranked = ids(&["a", "b", "c"]);
        assert!((mrr_at_k(&ranked, &ids(&["b"]), 3) - 0.5).abs() < 1e-6);
        assert!((mrr_at_k(&ranked, &ids(&["b"]), 1) - 0.0).abs() < 1e-6);
        assert!((mrr_at_k(&ranked, &ids(&["missing"]), 3) - 0.0).abs() < 1e-6);
    }

    #[test]
    fn ndcg_is_one_for_a_perfect_prefix() {
        let ranked = ids(&["a", "b"]);
        assert!((ndcg_at_k(&ranked, &ids(&["a"]), 2) - 1.0).abs() < 1e-5);
        let swapped = ids(&["b", "a"]);
        let score = ndcg_at_k(&swapped, &ids(&["a"]), 2);
        assert!(score < 1.0 && score > 0.0, "{score}");
    }
}
