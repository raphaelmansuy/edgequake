//! The two retrieval legs of a normal workspace, run together and fused.

use std::collections::HashMap;
use std::sync::Arc;

use edgequake_storage::traits::{Bm25Request, MetadataFilter, VectorSearchResult, VectorStorage};

use super::fuse::fuse_ranked;
use super::plan::QueryModality;
use super::relevance::{apply_floor, is_image_modality, Candidate, RelevanceGate};
use super::weights::NormalWeights;
use crate::context::RetrievedChunk;
use crate::error::Result;
use crate::helpers::build_chunk_from_result;

/// Everything one normal retrieval needs.
pub struct NormalSearch<'a> {
    pub query_text: &'a str,
    pub text_vector: Option<&'a [f32]>,
    pub image_vector: Option<&'a [f32]>,
    /// Tenant, workspace, document scope, `type=chunk`, embedding model and kind filters.
    pub scope: MetadataFilter,
    pub max_chunks: usize,
    /// Both legs fetch `max_chunks * candidate_multiplier` candidates.
    pub candidate_multiplier: usize,
    pub weights: NormalWeights,
}

/// Retrieval trace. Counts and labels only, never chunk text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NormalTrace {
    pub dense_hits: usize,
    pub bm25_hits: usize,
    pub fused_hits: usize,
    /// `postgres_bm25`, `none` (backend keeps no index) or `error` (lexical leg failed).
    pub bm25_backend: &'static str,
    pub modality: QueryModality,
    /// SPEC-184: what the absolute-relevance floor removed, and whether it emptied the context.
    pub relevance: RelevanceGate,
}

/// Fused chunks (best first, at most `2 * max_chunks`) and the trace.
pub struct NormalOutcome {
    pub chunks: Vec<RetrievedChunk>,
    pub trace: NormalTrace,
}

struct Legs {
    dense: Vec<Vec<VectorSearchResult>>,
    bm25: Vec<VectorSearchResult>,
    backend: &'static str,
}

/// Run dense and BM25 concurrently, then fuse by weighted RRF.
pub async fn retrieve_normal(
    vectors: &Arc<dyn VectorStorage>,
    search: &NormalSearch<'_>,
) -> Result<NormalOutcome> {
    let candidates = search
        .max_chunks
        .saturating_mul(search.candidate_multiplier.max(1))
        .max(1);
    let legs = run_legs(vectors, search, candidates).await?;
    Ok(fuse_legs(search, legs))
}

async fn run_legs(
    vectors: &Arc<dyn VectorStorage>,
    search: &NormalSearch<'_>,
    candidates: usize,
) -> Result<Legs> {
    let scope = Some(&search.scope);
    let dense_inputs: Vec<&[f32]> = [search.text_vector, search.image_vector]
        .into_iter()
        .flatten()
        .collect();
    let dense = futures::future::try_join_all(
        dense_inputs
            .iter()
            .map(|v| vectors.query_filtered(v, candidates, None, scope)),
    );
    let lexical = lexical_leg(vectors, search, candidates);
    let (dense, (bm25, backend)) = futures::join!(dense, lexical);
    Ok(Legs {
        dense: dense?,
        bm25,
        backend,
    })
}

async fn lexical_leg(
    vectors: &Arc<dyn VectorStorage>,
    search: &NormalSearch<'_>,
    candidates: usize,
) -> (Vec<VectorSearchResult>, &'static str) {
    if search.query_text.trim().is_empty() {
        return (Vec::new(), "postgres_bm25");
    }
    let req = Bm25Request {
        query_text: search.query_text,
        top_k: candidates,
        candidate_cap: candidates.saturating_mul(4),
    };
    match vectors
        .bm25_search_filtered(&req, Some(&search.scope))
        .await
    {
        Ok(Some(hits)) => (hits, "postgres_bm25"),
        Ok(None) => (Vec::new(), "none"),
        Err(e) => {
            // Dense still answers; the trace says the lexical leg was lost.
            tracing::warn!(error = %e, "BM25 leg failed; serving dense only");
            (Vec::new(), "error")
        }
    }
}

fn fuse_legs(search: &NormalSearch<'_>, legs: Legs) -> NormalOutcome {
    let mut lookup: HashMap<String, RetrievedChunk> = HashMap::new();
    let mut lists: Vec<Vec<String>> = Vec::new();
    let mut weights: Vec<f32> = Vec::new();
    for hits in legs.dense.iter().chain(std::iter::once(&legs.bm25)) {
        for hit in hits {
            lookup
                .entry(hit.id.clone())
                .or_insert_with(|| build_chunk_from_result(hit));
        }
    }
    // SPEC-183: keep the dense cosine, which fusion would otherwise discard.
    //
    // `VectorSearchResult::score` from the dense leg *is* a similarity on the query's own
    // scale, while the fused `score` this function returns is only a reciprocal rank. Anything
    // that needs to decide "is this hit actually relevant?" — the image grid, the confidence
    // badge — needs the former, and it exists here for free. A hit found only by BM25 has no
    // cosine, and stays `None` rather than being given a fabricated one.
    let mut similarity: HashMap<&str, f32> = HashMap::new();
    for hits in &legs.dense {
        for hit in hits {
            // Two dense lists (text and image vectors) can name the same chunk; keep the better
            // similarity, because that is the evidence the stronger leg matched on.
            similarity
                .entry(hit.id.as_str())
                .and_modify(|best| *best = best.max(hit.score))
                .or_insert(hit.score);
        }
    }
    let dense_hits: usize = legs.dense.iter().map(Vec::len).sum();
    for hits in &legs.dense {
        lists.push(hits.iter().map(|h| h.id.clone()).collect());
        weights.push(search.weights.dense);
    }
    lists.push(legs.bm25.iter().map(|h| h.id.clone()).collect());
    weights.push(search.weights.bm25);

    let keep = search.max_chunks.saturating_mul(2).max(1);
    // Fuse first, so the floor sees the whole candidate set: the relative half of the rule needs
    // the best match to set the scale for the rest.
    let ordered: Vec<RetrievedChunk> = fuse_ranked(&lists, &weights)
        .into_iter()
        .filter_map(|hit| {
            let sim = similarity.get(hit.id.as_str()).copied();
            lookup.remove(&hit.id).map(|mut chunk| {
                chunk.score = hit.score;
                chunk.similarity = sim;
                chunk
            })
        })
        .take(keep)
        .collect();

    // SPEC-184: the floor belongs here, where the cosine is still in hand. `score` is an RRF rank
    // and cannot be thresholded after fusion; `similarity` can.
    let candidates: Vec<Candidate> = ordered
        .iter()
        .map(|chunk| Candidate {
            similarity: chunk.similarity,
            is_image: is_image_modality(
                chunk.modality.as_deref(),
                chunk.source_modality.as_deref(),
            ),
        })
        .collect();
    let (keep_flags, gate) = apply_floor(&candidates);
    let chunks: Vec<RetrievedChunk> = ordered
        .into_iter()
        .zip(keep_flags)
        .filter_map(|(chunk, keep)| keep.then_some(chunk))
        .collect();

    let trace = NormalTrace {
        dense_hits,
        bm25_hits: legs.bm25.len(),
        fused_hits: chunks.len(),
        bm25_backend: legs.backend,
        modality: modality_of(search),
        relevance: gate,
    };
    NormalOutcome { chunks, trace }
}

fn modality_of(search: &NormalSearch<'_>) -> QueryModality {
    match (search.text_vector.is_some(), search.image_vector.is_some()) {
        (true, true) => QueryModality::Both,
        (false, true) => QueryModality::Image,
        _ => QueryModality::Text,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use edgequake_storage::error::Result as StorageResult;
    use serde_json::json;

    /// Storage double with canned dense and BM25 hits.
    struct Canned {
        dense: Vec<VectorSearchResult>,
        bm25: Option<StorageResult<Option<Vec<VectorSearchResult>>>>,
    }

    fn hit(id: &str, score: f32, content: &str) -> VectorSearchResult {
        VectorSearchResult {
            id: id.into(),
            score,
            metadata: json!({"content": content}),
        }
    }

    #[async_trait]
    impl VectorStorage for Canned {
        async fn initialize(&self) -> StorageResult<()> {
            Ok(())
        }
        async fn finalize(&self) -> StorageResult<()> {
            Ok(())
        }
        async fn upsert(&self, _d: &[(String, Vec<f32>, serde_json::Value)]) -> StorageResult<()> {
            Ok(())
        }
        async fn query(
            &self,
            _e: &[f32],
            _k: usize,
            _f: Option<&[String]>,
        ) -> StorageResult<Vec<VectorSearchResult>> {
            Ok(self.dense.clone())
        }
        async fn delete(&self, _ids: &[String]) -> StorageResult<()> {
            Ok(())
        }
        async fn delete_entity(&self, _e: &str) -> StorageResult<()> {
            Ok(())
        }
        async fn delete_entity_relations(&self, _e: &str) -> StorageResult<()> {
            Ok(())
        }
        async fn get_by_id(&self, _id: &str) -> StorageResult<Option<Vec<f32>>> {
            Ok(None)
        }
        async fn get_by_ids(&self, _ids: &[String]) -> StorageResult<Vec<(String, Vec<f32>)>> {
            Ok(vec![])
        }
        async fn is_empty(&self) -> StorageResult<bool> {
            Ok(false)
        }
        async fn count(&self) -> StorageResult<usize> {
            Ok(0)
        }
        async fn clear(&self) -> StorageResult<()> {
            Ok(())
        }
        async fn clear_workspace(&self, _w: &uuid::Uuid) -> StorageResult<usize> {
            Ok(0)
        }
        async fn delete_by_document(&self, _d: &str) -> StorageResult<usize> {
            Ok(0)
        }
        fn dimension(&self) -> usize {
            3
        }
        fn namespace(&self) -> &str {
            "t"
        }
        async fn query_filtered(
            &self,
            _e: &[f32],
            _k: usize,
            _f: Option<&[String]>,
            _m: Option<&MetadataFilter>,
        ) -> StorageResult<Vec<VectorSearchResult>> {
            Ok(self.dense.clone())
        }
        async fn bm25_search_filtered(
            &self,
            _r: &Bm25Request<'_>,
            _m: Option<&MetadataFilter>,
        ) -> StorageResult<Option<Vec<VectorSearchResult>>> {
            match &self.bm25 {
                Some(Ok(v)) => Ok(v.clone()),
                Some(Err(e)) => Err(edgequake_storage::error::StorageError::Database(
                    e.to_string(),
                )),
                None => Ok(None),
            }
        }
    }

    fn search<'a>(q: &'a str, v: &'a [f32]) -> NormalSearch<'a> {
        NormalSearch {
            query_text: q,
            text_vector: Some(v),
            image_vector: None,
            scope: MetadataFilter::default(),
            max_chunks: 2,
            candidate_multiplier: 5,
            weights: NormalWeights::default(),
        }
    }

    #[tokio::test]
    async fn lexical_hit_joins_the_dense_list() {
        let store: Arc<dyn VectorStorage> = Arc::new(Canned {
            dense: vec![hit("a", 0.9, "alpha"), hit("b", 0.8, "beta")],
            bm25: Some(Ok(Some(vec![hit("b", 3.0, "beta"), hit("c", 2.0, "")]))),
        });
        let out = retrieve_normal(&store, &search("beta", &[1.0, 0.0, 0.0]))
            .await
            .unwrap();
        let ids: Vec<&str> = out.chunks.iter().map(|c| c.id.as_str()).collect();
        assert_eq!(ids[0], "b", "found by both legs");
        assert_eq!(out.trace.dense_hits, 2);
        assert_eq!(out.trace.bm25_hits, 2);
        assert_eq!(out.trace.bm25_backend, "postgres_bm25");
        assert!(out.chunks.len() <= 4);
    }

    #[tokio::test]
    async fn backend_without_an_index_serves_dense_only() {
        let store: Arc<dyn VectorStorage> = Arc::new(Canned {
            dense: vec![hit("a", 0.9, "alpha"), hit("b", 0.8, "beta")],
            bm25: None,
        });
        let out = retrieve_normal(&store, &search("alpha", &[1.0, 0.0, 0.0]))
            .await
            .unwrap();
        let ids: Vec<&str> = out.chunks.iter().map(|c| c.id.as_str()).collect();
        assert_eq!(ids, ["a", "b"]);
        assert_eq!(out.trace.bm25_backend, "none");
        assert_eq!(out.trace.bm25_hits, 0);
    }

    #[tokio::test]
    async fn a_failing_lexical_leg_does_not_fail_the_query() {
        let store: Arc<dyn VectorStorage> = Arc::new(Canned {
            dense: vec![hit("a", 0.9, "alpha")],
            bm25: Some(Err(edgequake_storage::error::StorageError::Database(
                "boom".into(),
            ))),
        });
        let out = retrieve_normal(&store, &search("alpha", &[1.0, 0.0, 0.0]))
            .await
            .unwrap();
        assert_eq!(out.chunks.len(), 1);
        assert_eq!(out.trace.bm25_backend, "error");
    }

    #[tokio::test]
    async fn blank_text_skips_the_lexical_leg() {
        let store: Arc<dyn VectorStorage> = Arc::new(Canned {
            dense: vec![hit("a", 0.9, "alpha")],
            bm25: Some(Ok(Some(vec![hit("zzz", 9.0, "never")]))),
        });
        let out = retrieve_normal(&store, &search("   ", &[1.0, 0.0, 0.0]))
            .await
            .unwrap();
        assert_eq!(out.trace.bm25_hits, 0);
        assert_eq!(out.chunks[0].id, "a");
    }

    #[tokio::test]
    async fn fusion_keeps_the_dense_similarity_it_used_to_discard() {
        // WHY: the fused `score` is a reciprocal rank (~0.0164 for any first place), so it cannot
        // say whether a hit is actually relevant. The dense cosine can, and it already existed —
        // fusion threw it away. This pins that it now survives to the caller.
        let store: Arc<dyn VectorStorage> = Arc::new(Canned {
            dense: vec![hit("strong", 0.82, "alpha"), hit("weak", 0.11, "beta")],
            bm25: Some(Ok(Some(vec![hit("lexical", 3.0, "gamma")]))),
        });
        let out = retrieve_normal(&store, &search("alpha", &[1.0, 0.0, 0.0]))
            .await
            .unwrap();

        let by_id = |id: &str| out.chunks.iter().find(|c| c.id == id).expect(id);
        assert_eq!(by_id("strong").similarity, Some(0.82));
        assert_eq!(by_id("weak").similarity, Some(0.11));
        // A lexical-only hit has no cosine. It must stay unknown rather than become 0.0, which
        // a reader would interpret as "certainly irrelevant".
        assert_eq!(by_id("lexical").similarity, None);
        // The fused rank is still what orders them.
        assert!(by_id("strong").score > 0.0);
    }

    /// A picture hit: empty body, `image` modality — the shape the ingest path writes.
    fn image_hit(id: &str, score: f32) -> VectorSearchResult {
        VectorSearchResult {
            id: id.into(),
            score,
            metadata: json!({"content": "", "modality": "image"}),
        }
    }

    #[tokio::test]
    async fn the_floor_keeps_an_irrelevant_picture_query_out_of_the_prompt() {
        // The defect this closes, reproduced: a text query against a picture corpus. Every hit is
        // noise below the cross-modal floor. Before the gate all four reached the model, which
        // then had to explain that it could see no text; now the context is empty and the trace
        // says why.
        let store: Arc<dyn VectorStorage> = Arc::new(Canned {
            dense: vec![
                image_hit("p1", 0.473),
                image_hit("p2", 0.467),
                image_hit("p3", 0.466),
                image_hit("p4", 0.440),
            ],
            bm25: None,
        });
        let out = retrieve_normal(&store, &search("taxe au giga", &[1.0, 0.0, 0.0]))
            .await
            .unwrap();
        assert!(out.chunks.is_empty(), "no noise may reach the prompt");
        assert_eq!(out.trace.relevance.dropped, 4);
        assert_eq!(out.trace.relevance.kept, 0);
        assert!(out.trace.relevance.emptied());
    }

    #[tokio::test]
    async fn a_good_picture_query_keeps_its_matches_and_trims_the_rest() {
        // "a golden retriever puppy" against the same corpus: the real matches survive, including
        // the runner-ups a reader would call the same kind of thing.
        let store: Arc<dyn VectorStorage> = Arc::new(Canned {
            dense: vec![
                image_hit("p1", 0.715),
                image_hit("p2", 0.610),
                image_hit("p3", 0.569),
                image_hit("p4", 0.530),
            ],
            bm25: None,
        });
        let out = retrieve_normal(&store, &search("golden retriever puppy", &[1.0, 0.0, 0.0]))
            .await
            .unwrap();
        let ids: Vec<&str> = out.chunks.iter().map(|c| c.id.as_str()).collect();
        assert_eq!(ids, ["p1", "p2", "p3"], "the bicycle (0.530) is dropped");
        assert_eq!(out.trace.relevance.dropped, 1);
        assert!(!out.trace.relevance.emptied());
    }

    #[tokio::test]
    async fn an_unmeasured_chunk_is_never_dropped() {
        // A lexical-only hit carries no cosine. Dropping it would be a regression dressed up as
        // strictness, so an absent similarity is kept and counted separately.
        let store: Arc<dyn VectorStorage> = Arc::new(Canned {
            dense: vec![hit("dense", 0.60, "alpha")],
            bm25: Some(Ok(Some(vec![hit("lexical", 3.0, "beta")]))),
        });
        let out = retrieve_normal(&store, &search("alpha", &[1.0, 0.0, 0.0]))
            .await
            .unwrap();
        let ids: Vec<&str> = out.chunks.iter().map(|c| c.id.as_str()).collect();
        assert!(
            ids.contains(&"lexical"),
            "a lexical hit has no cosine to judge"
        );
        assert_eq!(out.trace.relevance.unmeasured_kept, 1);
    }

    #[tokio::test]
    async fn two_dense_lists_keep_the_better_similarity() {
        // A text query and an image query can retrieve the same chunk. The stronger evidence
        // wins, so the grid does not hide a picture the image leg matched well.
        let store: Arc<dyn VectorStorage> = Arc::new(Canned {
            dense: vec![hit("shared", 0.31, "mixed")],
            bm25: None,
        });
        let mut s = search("q", &[1.0, 0.0, 0.0]);
        let image = [0.0f32, 1.0, 0.0];
        s.image_vector = Some(&image);
        let out = retrieve_normal(&store, &s).await.unwrap();
        let shared = out.chunks.iter().find(|c| c.id == "shared").unwrap();
        assert_eq!(
            shared.similarity,
            Some(0.31),
            "both legs report 0.31 here; the max is what must survive"
        );
    }

    #[tokio::test]
    async fn image_only_query_uses_one_dense_list() {
        let store: Arc<dyn VectorStorage> = Arc::new(Canned {
            dense: vec![hit("img", 0.7, "")],
            bm25: None,
        });
        let mut s = search("", &[]);
        s.text_vector = None;
        s.image_vector = Some(&[0.0, 1.0, 0.0]);
        let out = retrieve_normal(&store, &s).await.unwrap();
        assert_eq!(out.trace.modality, QueryModality::Image);
        assert_eq!(out.chunks[0].id, "img");
    }
}
