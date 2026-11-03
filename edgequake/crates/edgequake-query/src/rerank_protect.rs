//! First-stage protect slots for cross-encoder Acc recovery (SPEC-001).
//!
//! Pure CE reorder can drop Mix-RRF evidence that Complex/Summarize need.
//! Guaranteed inclusion keeps those chunks in the set while preserving CE
//! ordering for the LLM (avoid stuffing noisy first-stage ranks at the front).

use std::collections::HashSet;

use crate::context::RetrievedChunk;

/// Read protect count from `EDGEQUAKE_RERANK_PROTECT_FIRST` (default 0 = off).
pub fn protect_first_from_env() -> usize {
    std::env::var("EDGEQUAKE_RERANK_PROTECT_FIRST")
        .ok()
        .and_then(|v| v.parse::<usize>().ok())
        .unwrap_or(0)
}

/// Ensure the first `protect_n` first-stage chunks appear in the CE result set.
///
/// Final order follows `ce_ranked` (CE quality signal). Missing protected chunks
/// are inserted by replacing the lowest CE-ranked non-protected slot.
pub fn blend_protect_first(
    original: &[RetrievedChunk],
    ce_ranked: Vec<RetrievedChunk>,
    protect_n: usize,
    top_k: usize,
) -> Vec<RetrievedChunk> {
    if protect_n == 0 || top_k == 0 {
        let mut out = ce_ranked;
        out.truncate(top_k);
        return out;
    }
    if original.is_empty() {
        let mut out = ce_ranked;
        out.truncate(top_k);
        return out;
    }

    let protect_n = protect_n.min(top_k).min(original.len());
    let protected: HashSet<String> = original
        .iter()
        .take(protect_n)
        .map(|c| c.id.clone())
        .collect();

    let mut out: Vec<RetrievedChunk> = Vec::with_capacity(top_k);
    let mut seen: HashSet<String> = HashSet::new();
    for chunk in ce_ranked {
        if out.len() >= top_k {
            break;
        }
        if seen.insert(chunk.id.clone()) {
            out.push(chunk);
        }
    }

    for chunk in original.iter().take(protect_n) {
        if seen.contains(&chunk.id) {
            continue;
        }
        if out.len() < top_k {
            seen.insert(chunk.id.clone());
            out.push(chunk.clone());
            continue;
        }
        // Replace lowest-priority non-protected slot (end of CE list).
        if let Some(pos) = out.iter().rposition(|c| !protected.contains(&c.id)) {
            seen.remove(&out[pos].id);
            seen.insert(chunk.id.clone());
            out[pos] = chunk.clone();
        }
    }

    out.truncate(top_k);
    out
}

/// Ensure chunks whose ids are in `protect_ids` appear in the CE result set.
///
/// 039: topic-admitted chunk survival — CE may hard-drop them via
/// `min_rerank_score`; re-insert from `original` (pre-CE Mix) by replacing the
/// lowest CE-ranked non-protected slot. Final order follows `ce_ranked` first.
pub fn blend_protect_ids(
    original: &[RetrievedChunk],
    ce_ranked: Vec<RetrievedChunk>,
    protect_ids: &[String],
    top_k: usize,
) -> Vec<RetrievedChunk> {
    if protect_ids.is_empty() || top_k == 0 {
        let mut out = ce_ranked;
        out.truncate(top_k);
        return out;
    }
    if original.is_empty() {
        let mut out = ce_ranked;
        out.truncate(top_k);
        return out;
    }

    let want: HashSet<&str> = protect_ids.iter().map(|s| s.as_str()).collect();
    let by_id: std::collections::HashMap<&str, &RetrievedChunk> =
        original.iter().map(|c| (c.id.as_str(), c)).collect();

    let mut out: Vec<RetrievedChunk> = Vec::with_capacity(top_k);
    let mut seen: HashSet<String> = HashSet::new();
    for chunk in ce_ranked {
        if out.len() >= top_k {
            break;
        }
        if seen.insert(chunk.id.clone()) {
            out.push(chunk);
        }
    }

    for id in protect_ids {
        if seen.contains(id) {
            continue;
        }
        let Some(chunk) = by_id.get(id.as_str()) else {
            continue;
        };
        if out.len() < top_k {
            seen.insert(chunk.id.clone());
            out.push((*chunk).clone());
            continue;
        }
        if let Some(pos) = out.iter().rposition(|c| !want.contains(c.id.as_str())) {
            seen.remove(&out[pos].id);
            seen.insert(chunk.id.clone());
            out[pos] = (*chunk).clone();
        }
    }

    out.truncate(top_k);
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::context::RetrievedChunk;

    fn chunk(id: &str, score: f32) -> RetrievedChunk {
        RetrievedChunk::new(id, format!("content-{id}"), score)
    }

    #[test]
    fn protect_zero_is_pure_ce() {
        let original = vec![chunk("a", 1.0), chunk("b", 0.9), chunk("c", 0.8)];
        let ce = vec![chunk("c", 0.99), chunk("b", 0.5), chunk("a", 0.1)];
        let out = blend_protect_first(&original, ce, 0, 2);
        assert_eq!(out.len(), 2);
        assert_eq!(out[0].id, "c");
        assert_eq!(out[1].id, "b");
    }

    #[test]
    fn protect_preserves_ce_order_and_includes_missing() {
        let original = vec![
            chunk("a", 1.0),
            chunk("b", 0.9),
            chunk("c", 0.8),
            chunk("d", 0.7),
        ];
        // CE buries a,b (first-stage top-2) at the bottom
        let ce = vec![
            chunk("d", 0.99),
            chunk("c", 0.98),
            chunk("x", 0.97),
            chunk("y", 0.96),
            chunk("b", 0.1),
            chunk("a", 0.05),
        ];
        let out = blend_protect_first(&original, ce, 2, 4);
        assert_eq!(out.len(), 4);
        // CE order kept for survivors; a,b forced in by replacing x,y
        let ids: HashSet<_> = out.iter().map(|c| c.id.as_str()).collect();
        assert!(ids.contains("a"));
        assert!(ids.contains("b"));
        assert!(ids.contains("d"));
        assert!(ids.contains("c"));
        // Leading order still CE-preferred
        assert_eq!(out[0].id, "d");
        assert_eq!(out[1].id, "c");
    }

    #[test]
    fn protect_noop_when_already_present() {
        let original = vec![chunk("a", 1.0), chunk("b", 0.9)];
        let ce = vec![chunk("a", 0.99), chunk("b", 0.98), chunk("c", 0.5)];
        let out = blend_protect_first(&original, ce, 2, 3);
        assert_eq!(
            out.iter().map(|c| c.id.as_str()).collect::<Vec<_>>(),
            vec!["a", "b", "c"]
        );
    }

    #[test]
    fn protect_ids_reinserts_ce_dropped_topic() {
        let original = vec![
            chunk("topic", 1.0),
            chunk("noise", 0.9),
            chunk("other", 0.8),
        ];
        // CE drops topic (below min) — only noise/other survive ranking list
        let ce = vec![chunk("noise", 0.99), chunk("other", 0.98)];
        let out = blend_protect_ids(&original, ce, &["topic".into()], 2);
        let ids: HashSet<_> = out.iter().map(|c| c.id.as_str()).collect();
        assert!(ids.contains("topic"));
        assert_eq!(out.len(), 2);
    }

    #[test]
    fn an_image_chunk_survives_the_text_reranker() {
        // EC-180-55 / LAW-180-9: a cross-encoder scores text, and an image chunk has none. Its
        // fused position is what judged it, so it must come back even when the reranker drops it.
        // This is the guarantee the normal pipeline relies on when it protects image ids.
        let mut picture = chunk("picture", 0.7);
        picture.source_modality = Some("image".into());
        let original = vec![picture, chunk("text", 0.9)];

        // The reranker returns only text: the picture scored below the threshold and vanished.
        let ce = vec![chunk("text", 0.99)];
        let image_ids: Vec<String> = original
            .iter()
            .filter(|c| c.is_image_chunk())
            .map(|c| c.id.clone())
            .collect();
        assert_eq!(image_ids, vec!["picture".to_string()]);

        let out = blend_protect_ids(&original, ce, &image_ids, 2);
        let ids: HashSet<_> = out.iter().map(|c| c.id.as_str()).collect();
        assert!(
            ids.contains("picture"),
            "an image that fusion selected must not be dropped by a text reranker"
        );
        assert!(ids.contains("text"), "and the reranked text stays too");
    }
}
