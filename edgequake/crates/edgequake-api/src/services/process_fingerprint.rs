//! Ingest process fingerprint (SPEC-046 EQ-046-14).
//!
//! First principle: if chunking or multimodal options change, previously
//! extracted KG artifacts are **stale** and must be purged before re-ingest
//! (LightRAG `_purge_stale_extraction_if_resuming`).
//!
//! DRY: single fingerprint format shared by admission, prepare, and reanalyze.

use edgequake_pipeline::ExtractionMode;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};

/// KV metadata field storing the last successful ingest fingerprint.
pub const PROCESS_FINGERPRINT_FIELD: &str = "ingest_process_fingerprint";

/// Inputs that define whether derived KG/chunks are still valid.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProcessFingerprintInput {
    /// Chunk strategy (`recursive`, `semantic`, …).
    pub chunking_strategy: String,
    /// Chunk token size (0 = default / unknown).
    pub chunk_token_size: usize,
    /// Chunk overlap (0 = default / unknown).
    pub chunk_overlap: usize,
    /// Multimodal process_options string (`i`, `ite`, …).
    pub multimodal_process_options: String,
    /// Optional content hash for extra safety.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub content_hash: String,
    /// SPEC-160: resolved extraction mode word. Empty means `llm`.
    ///
    /// Only a non-default mode enters the digest, so every pre-SPEC-160
    /// fingerprint stays valid (LAW-160-9). A mode change marks a document stale.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub extraction_mode: String,
    /// SPEC-180: `rp=normal` for a normal workspace. Empty means graph, so every
    /// pre-SPEC-180 fingerprint stays valid (LAW-180-12).
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub profile_segment: String,
}

impl ProcessFingerprintInput {
    pub fn new(
        chunking_strategy: impl Into<String>,
        chunk_token_size: usize,
        chunk_overlap: usize,
        multimodal_process_options: impl Into<String>,
    ) -> Self {
        Self {
            chunking_strategy: chunking_strategy.into(),
            chunk_token_size,
            chunk_overlap,
            multimodal_process_options: multimodal_process_options.into(),
            content_hash: String::new(),
            extraction_mode: String::new(),
            profile_segment: String::new(),
        }
    }

    /// Build from upload/prepare fields (DRY SSOT for admission + prepare + reanalyze).
    pub fn from_ingest_fields(
        chunking_strategy: &str,
        chunk_token_size: Option<usize>,
        chunk_overlap: Option<usize>,
        multimodal_process_options: Option<&str>,
    ) -> Self {
        Self::new(
            chunking_strategy,
            chunk_token_size.unwrap_or(0),
            chunk_overlap.unwrap_or(0),
            multimodal_process_options.unwrap_or(""),
        )
    }

    /// Read strategy / options / mm flags from document metadata JSON.
    pub fn from_document_metadata(metadata: &Value) -> Self {
        let strategy = metadata
            .get("chunking_strategy")
            .or_else(|| metadata.get("chunk_strategy"))
            .and_then(|v| v.as_str())
            .unwrap_or("recursive");
        let token_size = metadata
            .get("chunk_options")
            .and_then(|o| o.get("chunk_token_size"))
            .and_then(|v| v.as_u64())
            .map(|n| n as usize);
        let overlap = metadata
            .get("chunk_options")
            .and_then(|o| o.get("chunk_overlap_token_size"))
            .and_then(|v| v.as_u64())
            .map(|n| n as usize);
        let mm = metadata
            .get("multimodal_process_options")
            .and_then(|v| v.as_str());
        let mode = metadata
            .get(edgequake_pipeline::META_EXTRACTION_MODE)
            .and_then(|v| v.as_str())
            .and_then(ExtractionMode::parse)
            .unwrap_or_default();
        Self::from_ingest_fields(strategy, token_size, overlap, mm).with_extraction_mode(mode)
    }

    /// Record the resolved extraction mode (SPEC-160). `Llm` keeps the legacy digest.
    pub fn with_extraction_mode(mut self, mode: ExtractionMode) -> Self {
        self.extraction_mode = match mode {
            ExtractionMode::Llm => String::new(),
            other => other.as_str().to_string(),
        };
        self
    }

    /// SPEC-180: record the profile shape. A graph workspace keeps the legacy digest.
    pub fn with_profile(mut self, caps: &edgequake_core::rag_profile::ProfileCapabilities) -> Self {
        self.profile_segment = if caps.extracts_graph {
            String::new()
        } else {
            caps.fingerprint_segment().to_string()
        };
        self
    }

    pub fn with_content_hash(mut self, hash: impl Into<String>) -> Self {
        self.content_hash = hash.into();
        self
    }

    /// Stable hex digest (SHA-256 of canonical JSON).
    pub fn digest(&self) -> String {
        let mut canonical = format!(
            "cs={}|cts={}|co={}|mm={}|ch={}",
            self.chunking_strategy.trim().to_ascii_lowercase(),
            self.chunk_token_size,
            self.chunk_overlap,
            self.multimodal_process_options.trim().to_ascii_lowercase(),
            self.content_hash.trim()
        );
        let mode = self.extraction_mode.trim().to_ascii_lowercase();
        if !mode.is_empty() && mode != ExtractionMode::Llm.as_str() {
            canonical.push_str("|em=");
            canonical.push_str(&mode);
        }
        if !self.profile_segment.is_empty() {
            canonical.push('|');
            canonical.push_str(&self.profile_segment);
        }
        let mut hasher = Sha256::new();
        hasher.update(canonical.as_bytes());
        format!("{:x}", hasher.finalize())
    }
}

/// Read stored fingerprint from document metadata JSON.
pub fn resolve_fingerprint_from_metadata(metadata: &Value) -> Option<String> {
    metadata
        .as_object()
        .and_then(|obj| obj.get(PROCESS_FINGERPRINT_FIELD))
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .map(str::to_string)
}

/// Write fingerprint into a metadata object.
pub fn apply_fingerprint_to_metadata(obj: &mut serde_json::Map<String, Value>, digest: &str) {
    if !digest.is_empty() {
        obj.insert(
            PROCESS_FINGERPRINT_FIELD.to_string(),
            Value::String(digest.to_string()),
        );
    }
}

/// True when stored fingerprint differs from the new one.
///
/// First ingest (no stored fp) → false (nothing to purge).
pub fn fingerprint_is_stale(stored: Option<&str>, new_digest: &str) -> bool {
    match stored {
        None | Some("") => false,
        Some(prev) => prev != new_digest,
    }
}

/// True when we should purge derived data before reprocessing.
///
/// First ingest (no stored fp) → false. Changed options → true.
pub fn should_purge_stale_extraction(
    stored: Option<&str>,
    new_input: &ProcessFingerprintInput,
) -> bool {
    fingerprint_is_stale(stored, &new_input.digest())
}

/// Reanalyze / resume heuristic (LightRAG `_purge_stale_extraction_if_resuming`).
///
/// - Stored fingerprint differs → purge
/// - No stored fingerprint but caller explicitly changed process options → purge
///   (upgrade path before fingerprints existed)
pub fn should_purge_on_reanalyze(
    stored: Option<&str>,
    new_input: &ProcessFingerprintInput,
    explicit_options_change: bool,
) -> bool {
    if fingerprint_is_stale(stored, &new_input.digest()) {
        return true;
    }
    matches!(stored, None | Some("")) && explicit_options_change
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn digest_stable_for_same_input() {
        let a = ProcessFingerprintInput::new("recursive", 800, 100, "ite");
        let b = ProcessFingerprintInput::new("Recursive", 800, 100, "ITE");
        assert_eq!(a.digest(), b.digest());
    }

    #[test]
    fn digest_changes_when_strategy_changes() {
        let a = ProcessFingerprintInput::new("recursive", 800, 100, "");
        let b = ProcessFingerprintInput::new("semantic", 800, 100, "");
        assert_ne!(a.digest(), b.digest());
    }

    #[test]
    fn stale_when_options_change() {
        let input = ProcessFingerprintInput::new("recursive", 800, 100, "i");
        let dig = input.digest();
        assert!(!should_purge_stale_extraction(None, &input));
        assert!(!should_purge_stale_extraction(Some(&dig), &input));
        let changed = ProcessFingerprintInput::new("semantic", 800, 100, "i");
        assert!(should_purge_stale_extraction(Some(&dig), &changed));
    }

    #[test]
    fn reanalyze_purges_when_explicit_options_and_no_fingerprint() {
        let input = ProcessFingerprintInput::new("recursive", 800, 100, "ite");
        assert!(!should_purge_on_reanalyze(None, &input, false));
        assert!(should_purge_on_reanalyze(None, &input, true));
    }

    #[test]
    fn from_document_metadata_reads_fields() {
        let meta = json!({
            "chunking_strategy": "semantic",
            "chunk_options": { "chunk_token_size": 900, "chunk_overlap_token_size": 50 },
            "multimodal_process_options": "i"
        });
        let fp = ProcessFingerprintInput::from_document_metadata(&meta);
        assert_eq!(fp.chunking_strategy, "semantic");
        assert_eq!(fp.chunk_token_size, 900);
        assert_eq!(fp.chunk_overlap, 50);
        assert_eq!(fp.multimodal_process_options, "i");
    }

    // T-160-U12 — the llm digest equals the pre-SPEC-160 digest (LAW-160-9).
    #[test]
    fn spec160_llm_digest_matches_legacy_format() {
        let legacy = "577ce8421ecad7ad56ee9c5e71bdacdd4d4d14fe1469ecfaaace76a46494a6dc";
        let llm = ProcessFingerprintInput::new("recursive", 800, 100, "ite")
            .with_extraction_mode(ExtractionMode::Llm);
        assert_eq!(llm.digest(), legacy);
        assert_eq!(
            ProcessFingerprintInput::new("recursive", 800, 100, "ite").digest(),
            legacy
        );
    }

    // T-160-U13 — a mode change makes the document stale (EC-160-27).
    #[test]
    fn spec160_decision_mode_changes_digest() {
        let base = ProcessFingerprintInput::new("recursive", 800, 100, "ite");
        let decision = base.clone().with_extraction_mode(ExtractionMode::Decision);
        assert_ne!(base.digest(), decision.digest());
        assert!(should_purge_stale_extraction(
            Some(&base.digest()),
            &decision
        ));
        assert!(should_purge_stale_extraction(
            Some(&decision.digest()),
            &base
        ));
    }

    // T-160-U14 — metadata read: decision counts, unknown or inherit words mean llm.
    #[test]
    fn spec160_from_document_metadata_reads_mode() {
        let decision = json!({ "extraction_mode": "decision" });
        assert_eq!(
            ProcessFingerprintInput::from_document_metadata(&decision).extraction_mode,
            "decision"
        );
        for meta in [
            json!({ "extraction_mode": "inherit" }),
            json!({ "extraction_mode": "x" }),
            json!({}),
        ] {
            let fp = ProcessFingerprintInput::from_document_metadata(&meta);
            assert!(fp.extraction_mode.is_empty());
        }
    }

    // T-160-U15 — a legacy JSON fingerprint without the field still deserializes.
    #[test]
    fn spec160_legacy_json_deserializes() {
        let fp: ProcessFingerprintInput = serde_json::from_value(json!({
            "chunking_strategy": "recursive", "chunk_token_size": 800,
            "chunk_overlap": 100, "multimodal_process_options": "ite"
        }))
        .unwrap();
        assert!(fp.extraction_mode.is_empty());
    }

    #[test]
    fn metadata_roundtrip() {
        let mut obj = serde_json::Map::new();
        apply_fingerprint_to_metadata(&mut obj, "abc123");
        let meta = Value::Object(obj);
        assert_eq!(
            resolve_fingerprint_from_metadata(&meta).as_deref(),
            Some("abc123")
        );
        assert!(resolve_fingerprint_from_metadata(&json!({})).is_none());
    }

    #[test]
    fn profile_segment_keeps_graph_digest_and_changes_normal() {
        use edgequake_core::rag_profile::{EmbeddingModalities, RagProfile};
        let graph = RagProfile::Graph.capabilities(EmbeddingModalities::Text);
        let normal = RagProfile::Normal.capabilities(EmbeddingModalities::Text);
        let base = ProcessFingerprintInput::new("recursive", 800, 100, "");
        assert_eq!(base.digest(), base.clone().with_profile(&graph).digest());
        assert_ne!(base.digest(), base.clone().with_profile(&normal).digest());
    }
}
