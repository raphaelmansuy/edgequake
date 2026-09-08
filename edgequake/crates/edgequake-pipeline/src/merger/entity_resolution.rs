//! SPEC-091 RM2 — entity resolution ladder (LAW-RM7 / RM-AC-07).
//!
//! Stages (conservative, precision over recall):
//! 1. Exact normalized name-key (always)
//! 2. Optional embedding similarity when `EDGEQUAKE_ENTITY_EMBED_ER=on`
//! 3. Optional LLM adjudicate when `EDGEQUAKE_ER_LLM=on` (default off)
//!
//! String fuzzy (`EDGEQUAKE_ENTITY_FUZZY`) remains a prefilter only.

use std::sync::atomic::{AtomicU64, Ordering};

pub const ENTITY_EMBED_ER_ENV: &str = "EDGEQUAKE_ENTITY_EMBED_ER";
pub const ER_LLM_ENV: &str = "EDGEQUAKE_ER_LLM";

/// Default cosine similarity threshold for embed ER (conservative).
pub const DEFAULT_EMBED_ER_THRESHOLD: f32 = 0.92;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErDecision {
    ExactMatch,
    EmbedMerge,
    LlmMerge,
    CreateNew,
}

static MERGE_EXACT: AtomicU64 = AtomicU64::new(0);
static MERGE_EMBED: AtomicU64 = AtomicU64::new(0);
static MERGE_LLM: AtomicU64 = AtomicU64::new(0);
static CREATE_NEW: AtomicU64 = AtomicU64::new(0);

#[allow(dead_code)] // metrics for /health / ops dashboards
pub fn er_merge_exact_total() -> u64 {
    MERGE_EXACT.load(Ordering::Relaxed)
}
#[allow(dead_code)]
pub fn er_merge_embed_total() -> u64 {
    MERGE_EMBED.load(Ordering::Relaxed)
}
#[allow(dead_code)]
pub fn er_merge_llm_total() -> u64 {
    MERGE_LLM.load(Ordering::Relaxed)
}
#[allow(dead_code)]
pub fn er_create_new_total() -> u64 {
    CREATE_NEW.load(Ordering::Relaxed)
}

/// Parse an ops on/off flag (`on` / `1` / `true` / `yes`). Unset or empty is off.
pub fn parse_er_on_flag(raw: Option<&str>) -> bool {
    matches!(
        raw.unwrap_or("").trim().to_ascii_lowercase().as_str(),
        "on" | "1" | "true" | "yes"
    )
}

/// Embed / LLM ER gates. Production reads env once; tests pass literals.
/// Process env is not an SSOT the ladder may re-read (cargo `--lib` is one process).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ErLadderPolicy {
    pub embed_er: bool,
    pub llm_er: bool,
}

impl ErLadderPolicy {
    pub const OFF: Self = Self {
        embed_er: false,
        llm_er: false,
    };

    pub const EMBED_ONLY: Self = Self {
        embed_er: true,
        llm_er: false,
    };

    pub fn from_env() -> Self {
        Self {
            embed_er: entity_embed_er_enabled(),
            llm_er: er_llm_enabled(),
        }
    }
}

pub fn entity_embed_er_enabled() -> bool {
    parse_er_on_flag(std::env::var(ENTITY_EMBED_ER_ENV).ok().as_deref())
}

pub fn er_llm_enabled() -> bool {
    parse_er_on_flag(std::env::var(ER_LLM_ENV).ok().as_deref())
}

/// Cosine similarity for equal-length embeddings.
pub fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    if a.is_empty() || a.len() != b.len() {
        return 0.0;
    }
    let mut dot = 0.0f32;
    let mut na = 0.0f32;
    let mut nb = 0.0f32;
    for (x, y) in a.iter().zip(b.iter()) {
        dot += x * y;
        na += x * x;
        nb += y * y;
    }
    let denom = (na.sqrt() * nb.sqrt()).max(1e-12);
    dot / denom
}

/// Resolve identity after an exact name-key miss.
///
/// `candidates` are (entity_key, embedding) pairs already blocked (e.g. same
/// type / fuzzy prefilter). Returns the key to merge into, or `None` → create.
///
/// Policy is an argument so parallel tests never mutate process env.
pub fn resolve_after_exact_miss(
    mention_embedding: Option<&[f32]>,
    candidates: &[(String, Vec<f32>)],
    threshold: f32,
    llm_says_same: Option<bool>,
    policy: ErLadderPolicy,
) -> (ErDecision, Option<String>) {
    if policy.embed_er {
        if let Some(emb) = mention_embedding {
            let mut best: Option<(f32, &str)> = None;
            for (key, cand) in candidates {
                let sim = cosine_similarity(emb, cand);
                if sim >= threshold {
                    match best {
                        Some((s, _)) if sim <= s => {}
                        _ => best = Some((sim, key.as_str())),
                    }
                }
            }
            if let Some((_, key)) = best {
                MERGE_EMBED.fetch_add(1, Ordering::Relaxed);
                return (ErDecision::EmbedMerge, Some(key.to_string()));
            }
        }
    }

    if policy.llm_er {
        if let Some(true) = llm_says_same {
            if let Some((key, _)) = candidates.first() {
                MERGE_LLM.fetch_add(1, Ordering::Relaxed);
                return (ErDecision::LlmMerge, Some(key.clone()));
            }
        }
    }

    CREATE_NEW.fetch_add(1, Ordering::Relaxed);
    (ErDecision::CreateNew, None)
}

pub fn record_exact_merge() {
    MERGE_EXACT.fetch_add(1, Ordering::Relaxed);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn contract_spec091_er_on_flag_parse() {
        assert!(!parse_er_on_flag(None));
        assert!(!parse_er_on_flag(Some("")));
        assert!(!parse_er_on_flag(Some("off")));
        assert!(parse_er_on_flag(Some("on")));
        assert!(parse_er_on_flag(Some("1")));
        assert!(parse_er_on_flag(Some("TRUE")));
        assert!(parse_er_on_flag(Some(" yes ")));
    }

    #[test]
    fn contract_spec091_er_ladder_exact_prefer_create_when_off() {
        let (d, k) = resolve_after_exact_miss(
            Some(&[1.0, 0.0]),
            &[("OTHER".into(), vec![1.0, 0.0])],
            DEFAULT_EMBED_ER_THRESHOLD,
            None,
            ErLadderPolicy::OFF,
        );
        assert_eq!(d, ErDecision::CreateNew);
        assert!(k.is_none());
    }

    #[test]
    fn contract_spec091_er_ladder_embed_merge() {
        let (d, k) = resolve_after_exact_miss(
            Some(&[1.0, 0.0]),
            &[("ACME".into(), vec![0.99, 0.01])],
            0.9,
            None,
            ErLadderPolicy::EMBED_ONLY,
        );
        assert_eq!(d, ErDecision::EmbedMerge);
        assert_eq!(k.as_deref(), Some("ACME"));
    }

    #[test]
    fn contract_spec091_er_llm_default_off() {
        assert!(!parse_er_on_flag(None));
        let (d, k) = resolve_after_exact_miss(
            Some(&[1.0, 0.0]),
            &[("ACME".into(), vec![1.0, 0.0])],
            0.9,
            Some(true),
            ErLadderPolicy::OFF,
        );
        assert_eq!(d, ErDecision::CreateNew);
        assert!(k.is_none());
    }

    #[test]
    fn contract_spec091_er_ladder_llm_merge_when_on() {
        let (d, k) = resolve_after_exact_miss(
            Some(&[1.0, 0.0]),
            &[("ACME".into(), vec![0.0, 1.0])],
            0.9,
            Some(true),
            ErLadderPolicy {
                embed_er: false,
                llm_er: true,
            },
        );
        assert_eq!(d, ErDecision::LlmMerge);
        assert_eq!(k.as_deref(), Some("ACME"));
    }
}
