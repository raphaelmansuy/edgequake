//! Hermetic Normal hybrid eval: dense-only, lexical-only, and hybrid on one fixture.
//!
//! The embedder is a bag of words that collapses digit tokens (part numbers,
//! error codes) into one bucket, so dense search cannot tell `ERR-4419` from
//! `SKU-9088`. BM25 can. That is the split the report is for.
//!
//! This does not score a live corpus. The written report says so and points at
//! `make bench`.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use async_trait::async_trait;
use edgequake_query::eval::{mean, mrr_at_k, ndcg_at_k, recall_at_k};
use edgequake_query::normal::retriever::retrieve_normal;
use edgequake_query::normal::{NormalSearch, NormalWeights};
use edgequake_storage::error::{Result as StorageResult, StorageError};
use edgequake_storage::traits::{Bm25Request, MetadataFilter, VectorSearchResult, VectorStorage};
use serde::Deserialize;
use serde_json::{json, Value};

const DIM: usize = 256;
const FIXTURE: &str = include_str!("fixtures/spec180_normal_eval.json");

const CORPUS: &[(&str, &str)] = &[
    ("c1", "Refunds are processed within 30 days after approval."),
    ("c2", "Shipping usually arrives in five business days."),
    (
        "c3",
        "Fault code ERR-4419 means the sensor cable is disconnected from the controller port after a long diagnostic procedure.",
    ),
    ("c4", "Fault code SKU-9088 means battery low."),
    ("c5", "The warranty covers manufacturing defects for one year."),
    ("c6", "Office hours are nine to five on weekdays."),
];

#[derive(Debug, Deserialize)]
struct Fixture {
    k: usize,
    cases: Vec<Case>,
}

#[derive(Debug, Deserialize)]
struct Case {
    id: String,
    query: String,
    expect_chunks: Vec<String>,
    kind: String,
}

struct Corpus {
    rows: Vec<Row>,
}

struct Row {
    id: String,
    text: String,
    vector: Vec<f32>,
    tokens: Vec<String>,
}

fn tokenize(text: &str) -> Vec<String> {
    text.split(|c: char| !c.is_ascii_alphanumeric() && c != '-')
        .filter(|t| !t.is_empty())
        .map(|t| t.to_ascii_lowercase())
        .collect()
}

/// Common words get their own dimensions. A token with a digit shares dim 0
/// with every other code, so dense search cannot tell codes apart.
fn embed(text: &str) -> Vec<f32> {
    let mut v = vec![0.0_f32; DIM];
    for token in tokenize(text) {
        if token.chars().any(|c| c.is_ascii_digit()) {
            v[0] += 1.0;
        } else {
            let hash = token.bytes().fold(2_166_136_261_u32, |acc, byte| {
                acc.wrapping_mul(16_777_619) ^ u32::from(byte)
            });
            v[1 + (hash as usize % (DIM - 1))] += 1.0;
        }
    }
    let norm = v.iter().map(|x| x * x).sum::<f32>().sqrt();
    if norm > f32::EPSILON {
        for x in &mut v {
            *x /= norm;
        }
    }
    v
}

fn cosine(a: &[f32], b: &[f32]) -> f32 {
    a.iter().zip(b).map(|(x, y)| x * y).sum()
}

impl Corpus {
    fn new() -> Self {
        Self {
            rows: CORPUS
                .iter()
                .map(|(id, text)| Row {
                    id: (*id).to_string(),
                    text: (*text).to_string(),
                    vector: embed(text),
                    tokens: tokenize(text),
                })
                .collect(),
        }
    }

    fn dense(&self, query: &[f32], top_k: usize) -> Vec<VectorSearchResult> {
        let mut scored: Vec<(f32, &Row)> = self
            .rows
            .iter()
            .map(|row| (cosine(query, &row.vector), row))
            .collect();
        scored.sort_by(|a, b| {
            b.0.partial_cmp(&a.0)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| a.1.id.cmp(&b.1.id))
        });
        scored
            .into_iter()
            .take(top_k)
            .map(|(score, row)| hit(&row.id, score, &row.text))
            .collect()
    }

    fn bm25(&self, query: &str, top_k: usize) -> Vec<VectorSearchResult> {
        let q_tokens = tokenize(query);
        if q_tokens.is_empty() {
            return Vec::new();
        }
        let n = self.rows.len() as f32;
        let avgdl = self
            .rows
            .iter()
            .map(|row| row.tokens.len() as f32)
            .sum::<f32>()
            / n;
        let k1 = 1.2_f32;
        let b = 0.75_f32;
        let mut scored = Vec::new();
        for row in &self.rows {
            let mut score = 0.0_f32;
            let dl = row.tokens.len() as f32;
            for term in &q_tokens {
                let tf = row.tokens.iter().filter(|t| *t == term).count() as f32;
                if tf == 0.0 {
                    continue;
                }
                let df = self
                    .rows
                    .iter()
                    .filter(|other| other.tokens.iter().any(|t| t == term))
                    .count() as f32;
                let idf = ((n - df + 0.5) / (df + 0.5) + 1.0).ln();
                score += idf * (tf * (k1 + 1.0)) / (tf + k1 * (1.0 - b + b * dl / avgdl));
            }
            if score > 0.0 {
                scored.push((score, row));
            }
        }
        scored.sort_by(|a, b| {
            b.0.partial_cmp(&a.0)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| a.1.id.cmp(&b.1.id))
        });
        scored
            .into_iter()
            .take(top_k)
            .map(|(score, row)| hit(&row.id, score, &row.text))
            .collect()
    }
}

fn hit(id: &str, score: f32, content: &str) -> VectorSearchResult {
    VectorSearchResult {
        id: id.into(),
        score,
        metadata: json!({"content": content}),
    }
}

#[async_trait]
impl VectorStorage for Corpus {
    fn namespace(&self) -> &str {
        "spec180-eval"
    }
    fn dimension(&self) -> usize {
        DIM
    }
    async fn initialize(&self) -> StorageResult<()> {
        Ok(())
    }
    async fn finalize(&self) -> StorageResult<()> {
        Ok(())
    }
    async fn upsert(&self, _: &[(String, Vec<f32>, serde_json::Value)]) -> StorageResult<()> {
        Ok(())
    }
    async fn delete(&self, _: &[String]) -> StorageResult<()> {
        Ok(())
    }
    async fn delete_entity(&self, _: &str) -> StorageResult<()> {
        Ok(())
    }
    async fn delete_entity_relations(&self, _: &str) -> StorageResult<()> {
        Ok(())
    }
    async fn get_by_id(&self, id: &str) -> StorageResult<Option<Vec<f32>>> {
        Ok(self
            .rows
            .iter()
            .find(|r| r.id == id)
            .map(|r| r.vector.clone()))
    }
    async fn get_by_ids(&self, ids: &[String]) -> StorageResult<Vec<(String, Vec<f32>)>> {
        Ok(self
            .rows
            .iter()
            .filter(|r| ids.iter().any(|id| id == &r.id))
            .map(|r| (r.id.clone(), r.vector.clone()))
            .collect())
    }
    async fn is_empty(&self) -> StorageResult<bool> {
        Ok(self.rows.is_empty())
    }
    async fn count(&self) -> StorageResult<usize> {
        Ok(self.rows.len())
    }
    async fn clear(&self) -> StorageResult<()> {
        Ok(())
    }
    async fn clear_workspace(&self, _: &uuid::Uuid) -> StorageResult<usize> {
        Ok(0)
    }
    async fn delete_by_document(&self, _: &str) -> StorageResult<usize> {
        Ok(0)
    }
    async fn query(
        &self,
        embedding: &[f32],
        top_k: usize,
        _: Option<&[String]>,
    ) -> StorageResult<Vec<VectorSearchResult>> {
        Ok(self.dense(embedding, top_k))
    }
    async fn query_filtered(
        &self,
        embedding: &[f32],
        top_k: usize,
        _: Option<&[String]>,
        _: Option<&MetadataFilter>,
    ) -> StorageResult<Vec<VectorSearchResult>> {
        Ok(self.dense(embedding, top_k))
    }
    async fn bm25_search_filtered(
        &self,
        req: &Bm25Request<'_>,
        _: Option<&MetadataFilter>,
    ) -> StorageResult<Option<Vec<VectorSearchResult>>> {
        if req.query_text.trim().is_empty() {
            return Err(StorageError::InvalidData("empty bm25 query".into()));
        }
        Ok(Some(self.bm25(req.query_text, req.top_k)))
    }
}

#[derive(Clone, Copy)]
struct Config {
    name: &'static str,
    dense: f32,
    bm25: f32,
    use_dense: bool,
}

fn relevant_ids(case: &Case) -> Vec<String> {
    CORPUS
        .iter()
        .filter(|(_, text)| {
            case.expect_chunks.iter().any(|marker| {
                text.to_ascii_lowercase()
                    .contains(&marker.to_ascii_lowercase())
            })
        })
        .map(|(id, _)| (*id).to_string())
        .collect()
}

async fn rank(store: &Arc<Corpus>, case: &Case, config: Config) -> Vec<String> {
    let vector = embed(&case.query);
    let search = NormalSearch {
        query_text: &case.query,
        text_vector: config.use_dense.then_some(vector.as_slice()),
        image_vector: None,
        scope: MetadataFilter::default(),
        max_chunks: CORPUS.len(),
        candidate_multiplier: 1,
        weights: NormalWeights {
            dense: config.dense,
            bm25: config.bm25,
        },
    };
    let outcome = retrieve_normal(&(store.clone() as Arc<dyn VectorStorage>), &search)
        .await
        .expect("retrieve");
    outcome.chunks.into_iter().map(|c| c.id).collect()
}

fn score_block(rows: &[(String, Vec<String>, Vec<String>)], k: usize) -> Value {
    let recall: Vec<f32> = rows
        .iter()
        .map(|(_, ranked, relevant)| recall_at_k(ranked, relevant, k))
        .collect();
    let mrr: Vec<f32> = rows
        .iter()
        .map(|(_, ranked, relevant)| mrr_at_k(ranked, relevant, k))
        .collect();
    let ndcg: Vec<f32> = rows
        .iter()
        .map(|(_, ranked, relevant)| ndcg_at_k(ranked, relevant, k))
        .collect();
    let exact = |values: &[f32]| {
        let picked: Vec<f32> = rows
            .iter()
            .zip(values)
            .filter(|((kind, _, _), _)| kind == "exact")
            .map(|(_, v)| *v)
            .collect();
        mean(&picked)
    };
    let paraphrase = |values: &[f32]| {
        let picked: Vec<f32> = rows
            .iter()
            .zip(values)
            .filter(|((kind, _, _), _)| kind == "paraphrase")
            .map(|(_, v)| *v)
            .collect();
        mean(&picked)
    };
    json!({
        "recall_at_k": mean(&recall),
        "mrr_at_k": mean(&mrr),
        "ndcg_at_k": mean(&ndcg),
        "exact_recall_at_k": exact(&recall),
        "paraphrase_recall_at_k": paraphrase(&recall),
    })
}

#[tokio::test]
async fn hybrid_is_scored_against_each_leg() {
    let fixture: Fixture = serde_json::from_str(FIXTURE).expect("fixture");
    let store = Arc::new(Corpus::new());
    let configs = [
        Config {
            name: "dense_only",
            dense: 1.0,
            bm25: 0.0,
            use_dense: true,
        },
        Config {
            name: "lexical_only",
            dense: 0.0,
            bm25: 1.0,
            use_dense: false,
        },
        Config {
            name: "hybrid",
            dense: 1.0,
            bm25: 1.25,
            use_dense: true,
        },
    ];
    let mut report = json!({
        "disclaimer": "Hermetic fixture with a synthetic embedder. It proves the plumbing and catches regressions. It is not the live corpus score; that number is `make bench`.",
        "k": fixture.k,
        "configurations": {}
    });
    let mut by_name: HashMap<&str, Value> = HashMap::new();
    for config in configs {
        let mut rows = Vec::new();
        for case in &fixture.cases {
            let ranked = rank(&store, case, config).await;
            let relevant = relevant_ids(case);
            assert!(!relevant.is_empty(), "{} matched no chunk", case.id);
            rows.push((case.kind.clone(), ranked, relevant));
        }
        let block = score_block(&rows, fixture.k);
        by_name.insert(config.name, block.clone());
        report["configurations"][config.name] = block;
    }
    let path = report_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).expect("target dir");
    }
    std::fs::write(&path, serde_json::to_string_pretty(&report).unwrap()).expect("write report");

    for name in ["dense_only", "lexical_only", "hybrid"] {
        assert!(
            by_name[name]["recall_at_k"].is_number(),
            "{name} was not scored"
        );
    }
    let hybrid = by_name["hybrid"]["recall_at_k"].as_f64().unwrap();
    let dense = by_name["dense_only"]["recall_at_k"].as_f64().unwrap();
    assert!(
        hybrid + 0.02 >= dense,
        "hybrid {hybrid} fell behind dense {dense}"
    );
    let hybrid_exact = by_name["hybrid"]["exact_recall_at_k"].as_f64().unwrap();
    let dense_exact = by_name["dense_only"]["exact_recall_at_k"].as_f64().unwrap();
    assert!(
        hybrid_exact > dense_exact,
        "BM25 did not help the exact subset: hybrid {hybrid_exact} dense {dense_exact}"
    );
    let written = std::fs::read_to_string(&path).unwrap();
    assert!(written.contains("make bench"));
    assert!(written.contains("dense_only"));
}

fn report_path() -> PathBuf {
    if let Ok(dir) = std::env::var("CARGO_TARGET_DIR") {
        return PathBuf::from(dir).join("spec180-normal-hybrid.json");
    }
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../target/spec180-normal-hybrid.json")
}
