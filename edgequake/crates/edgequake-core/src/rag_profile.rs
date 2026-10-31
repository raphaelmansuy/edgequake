//! SPEC-180: the retrieval profile of a workspace and the capabilities it implies.
//!
//! `RagProfile` is chosen when a workspace is created and never changes
//! (LAW-180-1). Every gate in ingest, query, delete, MCP and the UI asks
//! [`ProfileCapabilities`] a question; nothing outside this module maps the
//! enum to a boolean (LAW-180-2). The module is pure: no SQL, no HTTP.

use serde::{Deserialize, Serialize};

/// Stable error codes of the profile contract (shared by API and MCP, DRY).
pub mod codes {
    pub const INVALID_RAG_PROFILE: &str = "invalid_rag_profile";
    pub const INVALID_EMBEDDING_MODALITIES: &str = "invalid_embedding_modalities";
    pub const INVALID_FTS_REGCONFIG: &str = "invalid_fts_regconfig";
    pub const INVALID_DOC_KIND: &str = "invalid_doc_kind";
    pub const EXTRACTION_MODE_NOT_APPLICABLE: &str = "extraction_mode_not_applicable";
    pub const MODALITIES_REQUIRE_NORMAL: &str = "modalities_require_normal";
    pub const EMBEDDING_MODEL_NOT_MULTIMODAL: &str = "embedding_model_not_multimodal";
    pub const EMBEDDING_DIMENSION_REJECTED: &str = "embedding_dimension_rejected";
    pub const RAG_PROFILE_IMMUTABLE: &str = "rag_profile_immutable";
    pub const EMBEDDING_MODALITIES_IMMUTABLE: &str = "embedding_modalities_immutable";
    pub const FTS_REGCONFIG_IMMUTABLE: &str = "fts_regconfig_immutable";
    pub const PROFILE_HAS_NO_GRAPH: &str = "profile_has_no_graph";
    pub const MODE_NOT_SUPPORTED_BY_PROFILE: &str = "mode_not_supported_by_profile";
    pub const REPROCESS_MODE_NOT_APPLICABLE: &str = "reprocess_mode_not_applicable";
    pub const IMAGE_QUERY_NOT_SUPPORTED: &str = "image_query_not_supported";
    pub const QUERY_IMAGE_INVALID: &str = "query_image_invalid";
    pub const NO_CHUNKS: &str = "no_chunks";
    pub const INVALID_SOURCE_MODALITY: &str = "invalid_source_modality";
    pub const KIND_FILTER_NOT_SUPPORTED_BY_PROFILE: &str = "kind_filter_not_supported_by_profile";
}

/// A coded violation of the profile contract.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("{message}")]
pub struct ProfileError {
    /// Stable machine code, one of [`codes`].
    pub code: &'static str,
    /// One sentence an operator can show.
    pub message: String,
}

impl ProfileError {
    pub fn new(code: &'static str, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }

    /// `code: message`, the form services put in their error text.
    pub fn tagged(&self) -> String {
        format!("{}: {}", self.code, self.message)
    }

    /// Service error for a bad create/update value (maps to 400/422).
    pub fn into_validation(self) -> crate::Error {
        crate::Error::validation(self.tagged())
    }

    /// Service error for a change the profile law forbids (maps to 409).
    pub fn into_conflict(self) -> crate::Error {
        crate::Error::conflict(self.tagged())
    }
}

/// How a workspace stores and retrieves evidence.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RagProfile {
    /// Hybrid GraphRAG. The default; today's behavior.
    #[default]
    Graph,
    /// Chunks, dense embeddings and BM25. No knowledge graph.
    Normal,
}

impl RagProfile {
    pub const ALL: [RagProfile; 2] = [RagProfile::Graph, RagProfile::Normal];

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Graph => "graph",
            Self::Normal => "normal",
        }
    }

    /// Case-insensitive. Unknown words, including `none`, `chunk_only` and the
    /// empty string, are an error: they are neither inherit nor `graph`.
    pub fn parse(raw: &str) -> Result<Self, ProfileError> {
        match raw.trim().to_ascii_lowercase().as_str() {
            "graph" => Ok(Self::Graph),
            "normal" => Ok(Self::Normal),
            other => Err(ProfileError::new(
                codes::INVALID_RAG_PROFILE,
                format!("rag_profile must be 'graph' or 'normal', got '{other}'"),
            )),
        }
    }

    /// Capabilities of this profile for a given embedding modality set.
    pub fn capabilities(self, modalities: EmbeddingModalities) -> ProfileCapabilities {
        ProfileCapabilities::new(self, modalities)
    }
}

impl std::fmt::Display for RagProfile {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// What a workspace embeds. Fixed at creation (LAW-180-5).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum EmbeddingModalities {
    #[default]
    #[serde(rename = "text")]
    Text,
    #[serde(rename = "text,image")]
    TextImage,
}

impl EmbeddingModalities {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Text => "text",
            Self::TextImage => "text,image",
        }
    }

    pub fn includes_image(self) -> bool {
        matches!(self, Self::TextImage)
    }

    pub fn parse(raw: &str) -> Result<Self, ProfileError> {
        let normalized: String = raw
            .trim()
            .to_ascii_lowercase()
            .split(',')
            .map(str::trim)
            .collect::<Vec<_>>()
            .join(",");
        match normalized.as_str() {
            "text" => Ok(Self::Text),
            "text,image" => Ok(Self::TextImage),
            other => Err(ProfileError::new(
                codes::INVALID_EMBEDDING_MODALITIES,
                format!("embedding_modalities must be 'text' or 'text,image', got '{other}'"),
            )),
        }
    }
}

/// Postgres text-search configuration chosen for a workspace. Immutable.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FtsRegconfig {
    #[default]
    English,
    Simple,
    French,
    German,
    Spanish,
}

impl FtsRegconfig {
    pub const ALL: [FtsRegconfig; 5] = [
        Self::English,
        Self::Simple,
        Self::French,
        Self::German,
        Self::Spanish,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            Self::English => "english",
            Self::Simple => "simple",
            Self::French => "french",
            Self::German => "german",
            Self::Spanish => "spanish",
        }
    }

    pub fn parse(raw: &str) -> Result<Self, ProfileError> {
        match raw.trim().to_ascii_lowercase().as_str() {
            "english" => Ok(Self::English),
            "simple" => Ok(Self::Simple),
            "french" => Ok(Self::French),
            "german" => Ok(Self::German),
            "spanish" => Ok(Self::Spanish),
            other => Err(ProfileError::new(
                codes::INVALID_FTS_REGCONFIG,
                format!("fts_regconfig must be one of english, simple, french, german, spanish; got '{other}'"),
            )),
        }
    }
}

/// Family of a source file (LAW-180-10). Mirrors the generated `documents.doc_kind`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DocKind {
    Pdf,
    Image,
    Markdown,
    Text,
    Office,
    Other,
}

impl DocKind {
    pub const ALL: [DocKind; 6] = [
        Self::Pdf,
        Self::Image,
        Self::Markdown,
        Self::Text,
        Self::Office,
        Self::Other,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Pdf => "pdf",
            Self::Image => "image",
            Self::Markdown => "markdown",
            Self::Text => "text",
            Self::Office => "office",
            Self::Other => "other",
        }
    }

    pub fn parse(raw: &str) -> Result<Self, ProfileError> {
        let lower = raw.trim().to_ascii_lowercase();
        Self::ALL
            .into_iter()
            .find(|kind| kind.as_str() == lower)
            .ok_or_else(|| {
                ProfileError::new(
                    codes::INVALID_DOC_KIND,
                    format!(
                        "doc_kinds must be among pdf, image, markdown, text, office, other; got '{lower}'"
                    ),
                )
            })
    }

    /// Parse a whole list; the first bad word wins. Empty means "no constraint".
    pub fn parse_all<I, S>(raw: I) -> Result<Vec<Self>, ProfileError>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        raw.into_iter().map(|w| Self::parse(w.as_ref())).collect()
    }

    /// Same mapping as the SQL `CASE` in migration 225 (kept in step by a test).
    pub fn from_content_type(content_type: &str) -> Self {
        let ct = content_type.trim().to_ascii_lowercase();
        if ct.starts_with("image/") {
            Self::Image
        } else if ct == "application/pdf" {
            Self::Pdf
        } else if ct == "text/markdown" || ct == "text/x-markdown" {
            Self::Markdown
        } else if ct.starts_with("text/") {
            Self::Text
        } else if OFFICE_CONTENT_TYPES.contains(&ct.as_str()) {
            Self::Office
        } else {
            Self::Other
        }
    }
}

/// Parse `source_modalities` filter words: `text` or `image`, case-insensitive.
pub fn parse_source_modalities<I, S>(raw: I) -> Result<Vec<String>, ProfileError>
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    raw.into_iter()
        .map(|word| {
            let lower = word.as_ref().trim().to_ascii_lowercase();
            match lower.as_str() {
                "text" | "image" => Ok(lower),
                _ => Err(ProfileError::new(
                    codes::INVALID_SOURCE_MODALITY,
                    format!("source_modalities must be among text, image; got '{lower}'"),
                )),
            }
        })
        .collect()
}

/// MIME types stored as `office`. One list: SQL and Rust both derive from it.
pub const OFFICE_CONTENT_TYPES: [&str; 6] = [
    "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
    "application/msword",
    "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
    "application/vnd.ms-excel",
    "application/vnd.openxmlformats-officedocument.presentationml.presentation",
    "application/vnd.ms-powerpoint",
];

/// Query modes a client may send (wire words), used to judge them per profile.
const GRAPH_ONLY_MODES: [&str; 4] = ["local", "global", "hybrid", "mix"];

/// Every gate of the pack, as booleans. Constructed once per job or request.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProfileCapabilities {
    pub extracts_graph: bool,
    pub runs_gleaning: bool,
    pub runs_summary_llm: bool,
    pub merges_graph: bool,
    pub refreshes_communities: bool,
    pub embeds_entities: bool,
    pub embeds_relationships: bool,
    pub embeds_chunks: bool,
    pub embeds_images: bool,
    pub writes_bm25: bool,
    pub queries_graph: bool,
    pub allows_graph_modes: bool,
    pub shows_graph_ui: bool,
    pub judges_extract_usage: bool,
    pub judges_summary_usage: bool,
    pub zero_entities_is_failure: bool,
    pub skips_graph_on_delete: bool,
}

/// The capabilities every caller assumes when a workspace cannot be read: `graph`.
impl Default for ProfileCapabilities {
    fn default() -> Self {
        Self::new(RagProfile::default(), EmbeddingModalities::default())
    }
}

impl ProfileCapabilities {
    /// The only mapping from profile to booleans (LAW-180-2).
    pub fn new(profile: RagProfile, modalities: EmbeddingModalities) -> Self {
        let graph = profile == RagProfile::Graph;
        Self {
            extracts_graph: graph,
            runs_gleaning: graph,
            runs_summary_llm: graph,
            merges_graph: graph,
            refreshes_communities: graph,
            embeds_entities: graph,
            embeds_relationships: graph,
            embeds_chunks: true,
            embeds_images: !graph && modalities.includes_image(),
            writes_bm25: !graph,
            queries_graph: graph,
            allows_graph_modes: graph,
            shows_graph_ui: graph,
            judges_extract_usage: graph,
            judges_summary_usage: graph,
            zero_entities_is_failure: graph,
            skips_graph_on_delete: !graph,
        }
    }

    /// Is a wire query `mode` acceptable? Omitted is always acceptable
    /// (graph: Mix, normal: `NormalRetriever`). `naive` and `bypass` always are.
    pub fn check_query_mode(&self, mode: Option<&str>) -> Result<(), ProfileError> {
        let Some(mode) = mode.map(|m| m.trim().to_ascii_lowercase()) else {
            return Ok(());
        };
        if self.allows_graph_modes || !GRAPH_ONLY_MODES.contains(&mode.as_str()) {
            return Ok(());
        }
        Err(ProfileError::new(
            codes::MODE_NOT_SUPPORTED_BY_PROFILE,
            format!(
                "mode '{mode}' needs a knowledge graph; this workspace supports 'naive' (chunks), 'bypass' (chat) or no mode"
            ),
        ))
    }

    /// Entity, relationship and community surfaces are absent on a normal workspace.
    pub fn require_graph(&self) -> Result<(), ProfileError> {
        if self.queries_graph {
            return Ok(());
        }
        Err(profile_has_no_graph())
    }

    /// Document-kind, MIME and source-modality filters narrow chunks, which only a
    /// normal workspace retrieves directly. `present` is "the request set any of them".
    pub fn check_kind_filters(&self, present: bool) -> Result<(), ProfileError> {
        if !present || !self.queries_graph {
            return Ok(());
        }
        Err(ProfileError::new(
            codes::KIND_FILTER_NOT_SUPPORTED_BY_PROFILE,
            "doc_kinds, mime_types and source_modalities apply to normal workspaces only",
        ))
    }

    /// An upload-time `extraction_mode` only means something when a graph is built.
    pub fn check_extraction_mode(&self, word: Option<&str>) -> Result<(), ProfileError> {
        let set = word
            .map(|w| w.trim().to_ascii_lowercase())
            .is_some_and(|w| !w.is_empty() && w != "inherit" && w != "none");
        if self.extracts_graph || !set {
            return Ok(());
        }
        Err(ProfileError::new(
            codes::EXTRACTION_MODE_NOT_APPLICABLE,
            "extraction_mode does not apply: this workspace has no knowledge graph",
        ))
    }

    /// `entities_only` and `merge_only` rebuild a graph. A normal workspace re-embeds only.
    pub fn check_reprocess_mode(&self, mode: &str) -> Result<(), ProfileError> {
        let graph_only = matches!(
            mode.trim().to_ascii_lowercase().as_str(),
            "entities" | "entities_only" | "extract" | "merge" | "merge_only" | "kg_only"
        );
        if self.extracts_graph || !graph_only {
            return Ok(());
        }
        Err(ProfileError::new(
            codes::REPROCESS_MODE_NOT_APPLICABLE,
            "this reprocess mode rebuilds a knowledge graph; this workspace has none",
        ))
    }

    /// An image query needs an embedding model that accepts images.
    pub fn check_image_query(&self) -> Result<(), ProfileError> {
        if self.embeds_images {
            return Ok(());
        }
        Err(ProfileError::new(
            codes::IMAGE_QUERY_NOT_SUPPORTED,
            "this workspace embeds text only; search by image needs 'text and images'",
        ))
    }

    /// An image upload to a normal workspace needs an embedding model that accepts images
    /// (EC-180-18): no caption is ever generated in its place.
    pub fn check_image_upload(&self) -> Result<(), ProfileError> {
        if self.embeds_images {
            return Ok(());
        }
        Err(ProfileError::new(
            codes::EMBEDDING_MODEL_NOT_MULTIMODAL,
            "this workspace embeds text only; uploading an image needs 'text and images'",
        ))
    }

    /// Fingerprint segment (`rp=graph|normal`). Keeps checkpoints of another shape out.
    pub fn fingerprint_segment(&self) -> &'static str {
        if self.extracts_graph {
            "rp=graph"
        } else {
            "rp=normal"
        }
    }
}

/// The 409 an entity, neighbourhood or graph-image call returns on a normal workspace.
pub fn profile_has_no_graph() -> ProfileError {
    ProfileError::new(
        codes::PROFILE_HAS_NO_GRAPH,
        "This workspace has no knowledge graph. It retrieves by embeddings and keywords.",
    )
}

/// Create-time cross-field rules (LAW-180-5): `text,image` needs `normal`.
pub fn check_create_combination(
    profile: RagProfile,
    modalities: EmbeddingModalities,
) -> Result<(), ProfileError> {
    if profile == RagProfile::Graph && modalities.includes_image() {
        return Err(ProfileError::new(
            codes::MODALITIES_REQUIRE_NORMAL,
            "'text and images' needs a normal workspace: graph workspaces embed text only",
        ));
    }
    Ok(())
}

/// Dimensions the image-capable model may be created with (finite HNSW set).
pub const MULTIMODAL_DIMENSIONS: [usize; 3] = [768, 1536, 3072];

/// Reject a dimension outside the finite set the ANN indexes cover.
pub fn check_multimodal_dimension(dimension: usize) -> Result<(), ProfileError> {
    if MULTIMODAL_DIMENSIONS.contains(&dimension) {
        return Ok(());
    }
    Err(ProfileError::new(
        codes::EMBEDDING_DIMENSION_REJECTED,
        format!("dimension {dimension} is not supported; use 768, 1536 or 3072"),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn normal(images: bool) -> ProfileCapabilities {
        let m = if images {
            EmbeddingModalities::TextImage
        } else {
            EmbeddingModalities::Text
        };
        RagProfile::Normal.capabilities(m)
    }

    #[test]
    fn profile_parses_case_insensitively_and_rejects_inherit_words() {
        assert_eq!(RagProfile::parse(" Normal ").unwrap(), RagProfile::Normal);
        assert_eq!(RagProfile::parse("GRAPH").unwrap(), RagProfile::Graph);
        for bad in ["none", "chunk_only", "", "inherit", "mix"] {
            let err = RagProfile::parse(bad).unwrap_err();
            assert_eq!(err.code, codes::INVALID_RAG_PROFILE, "{bad}");
        }
    }

    #[test]
    fn default_profile_is_graph() {
        assert_eq!(RagProfile::default(), RagProfile::Graph);
        let caps = RagProfile::default().capabilities(EmbeddingModalities::Text);
        assert!(caps.extracts_graph && caps.queries_graph && caps.shows_graph_ui);
        assert!(!caps.writes_bm25 && !caps.embeds_images);
    }

    #[test]
    fn normal_profile_has_no_graph_work() {
        let caps = normal(false);
        assert!(!caps.extracts_graph);
        assert!(!caps.runs_summary_llm);
        assert!(!caps.merges_graph);
        assert!(!caps.refreshes_communities);
        assert!(!caps.embeds_entities && !caps.embeds_relationships);
        assert!(caps.embeds_chunks && caps.writes_bm25);
        assert!(!caps.queries_graph && !caps.allows_graph_modes && !caps.shows_graph_ui);
        assert!(!caps.judges_extract_usage && !caps.judges_summary_usage);
        assert!(!caps.zero_entities_is_failure && caps.skips_graph_on_delete);
    }

    #[test]
    fn images_need_normal_and_modalities() {
        assert!(!normal(false).embeds_images);
        assert!(normal(true).embeds_images);
        let graph = RagProfile::Graph.capabilities(EmbeddingModalities::TextImage);
        assert!(
            !graph.embeds_images,
            "graph never embeds images in this pack"
        );
    }

    #[test]
    fn modalities_parse_and_normalise_spacing() {
        assert_eq!(
            EmbeddingModalities::parse("Text, Image").unwrap(),
            EmbeddingModalities::TextImage
        );
        assert_eq!(
            EmbeddingModalities::parse("text").unwrap(),
            EmbeddingModalities::Text
        );
        for bad in ["image", "text,audio", ""] {
            assert_eq!(
                EmbeddingModalities::parse(bad).unwrap_err().code,
                codes::INVALID_EMBEDDING_MODALITIES
            );
        }
        assert_eq!(EmbeddingModalities::TextImage.as_str(), "text,image");
    }

    #[test]
    fn modalities_serialize_as_wire_words() {
        assert_eq!(
            serde_json::to_string(&EmbeddingModalities::TextImage).unwrap(),
            "\"text,image\""
        );
        let back: EmbeddingModalities = serde_json::from_str("\"text\"").unwrap();
        assert_eq!(back, EmbeddingModalities::Text);
    }

    #[test]
    fn fts_regconfig_is_a_closed_list() {
        for cfg in FtsRegconfig::ALL {
            assert_eq!(FtsRegconfig::parse(cfg.as_str()).unwrap(), cfg);
        }
        assert_eq!(
            FtsRegconfig::parse("klingon").unwrap_err().code,
            codes::INVALID_FTS_REGCONFIG
        );
    }

    #[test]
    fn graph_modes_are_refused_on_normal() {
        let caps = normal(false);
        assert!(caps.check_query_mode(None).is_ok());
        assert!(caps.check_query_mode(Some("naive")).is_ok());
        assert!(caps.check_query_mode(Some("bypass")).is_ok());
        for mode in ["local", "global", "hybrid", "MIX"] {
            let err = caps.check_query_mode(Some(mode)).unwrap_err();
            assert_eq!(err.code, codes::MODE_NOT_SUPPORTED_BY_PROFILE, "{mode}");
        }
        let graph = RagProfile::Graph.capabilities(EmbeddingModalities::Text);
        assert!(graph.check_query_mode(Some("mix")).is_ok());
    }

    #[test]
    fn extraction_mode_is_rejected_only_on_normal_when_set() {
        let caps = normal(false);
        assert!(caps.check_extraction_mode(None).is_ok());
        assert!(caps.check_extraction_mode(Some("")).is_ok());
        assert!(caps.check_extraction_mode(Some("inherit")).is_ok());
        assert_eq!(
            caps.check_extraction_mode(Some("llm")).unwrap_err().code,
            codes::EXTRACTION_MODE_NOT_APPLICABLE
        );
        let graph = RagProfile::Graph.capabilities(EmbeddingModalities::Text);
        assert!(graph.check_extraction_mode(Some("decision")).is_ok());
    }

    #[test]
    fn graph_reprocess_modes_are_refused_on_normal() {
        let caps = normal(false);
        assert!(caps.check_reprocess_mode("full").is_ok());
        assert_eq!(
            caps.check_reprocess_mode("entities_only").unwrap_err().code,
            codes::REPROCESS_MODE_NOT_APPLICABLE
        );
        assert!(caps.check_reprocess_mode("merge_only").is_err());
    }

    #[test]
    fn image_query_needs_image_embedding() {
        assert_eq!(
            normal(false).check_image_query().unwrap_err().code,
            codes::IMAGE_QUERY_NOT_SUPPORTED
        );
        assert!(normal(true).check_image_query().is_ok());
    }

    #[test]
    fn require_graph_names_the_code() {
        assert_eq!(
            normal(false).require_graph().unwrap_err().code,
            codes::PROFILE_HAS_NO_GRAPH
        );
        assert!(RagProfile::Graph
            .capabilities(EmbeddingModalities::Text)
            .require_graph()
            .is_ok());
    }

    #[test]
    fn modalities_require_normal_on_create() {
        assert_eq!(
            check_create_combination(RagProfile::Graph, EmbeddingModalities::TextImage)
                .unwrap_err()
                .code,
            codes::MODALITIES_REQUIRE_NORMAL
        );
        assert!(
            check_create_combination(RagProfile::Normal, EmbeddingModalities::TextImage).is_ok()
        );
        assert!(check_create_combination(RagProfile::Graph, EmbeddingModalities::Text).is_ok());
    }

    #[test]
    fn multimodal_dimensions_are_a_finite_set() {
        for d in MULTIMODAL_DIMENSIONS {
            assert!(check_multimodal_dimension(d).is_ok());
        }
        for d in [128, 1024, 2048, 4096] {
            assert_eq!(
                check_multimodal_dimension(d).unwrap_err().code,
                codes::EMBEDDING_DIMENSION_REJECTED
            );
        }
    }

    #[test]
    fn fingerprint_segments_differ_by_shape() {
        assert_eq!(normal(false).fingerprint_segment(), "rp=normal");
        assert_eq!(
            RagProfile::Graph
                .capabilities(EmbeddingModalities::Text)
                .fingerprint_segment(),
            "rp=graph"
        );
    }

    #[test]
    fn doc_kind_matches_the_sql_mapping() {
        let cases = [
            ("image/png", DocKind::Image),
            ("IMAGE/JPEG", DocKind::Image),
            ("application/pdf", DocKind::Pdf),
            ("text/markdown", DocKind::Markdown),
            ("text/x-markdown", DocKind::Markdown),
            ("text/plain", DocKind::Text),
            ("text/csv", DocKind::Text),
            ("application/msword", DocKind::Office),
            (OFFICE_CONTENT_TYPES[2], DocKind::Office),
            ("application/zip", DocKind::Other),
            ("", DocKind::Other),
        ];
        for (ct, kind) in cases {
            assert_eq!(DocKind::from_content_type(ct), kind, "{ct}");
        }
    }

    #[test]
    fn doc_kind_parse_rejects_unknown_words() {
        assert_eq!(DocKind::parse(" PDF ").unwrap(), DocKind::Pdf);
        assert_eq!(
            DocKind::parse("spreadsheet").unwrap_err().code,
            codes::INVALID_DOC_KIND
        );
        let kinds = DocKind::parse_all(["pdf", "image"]).unwrap();
        assert_eq!(kinds, vec![DocKind::Pdf, DocKind::Image]);
        assert!(DocKind::parse_all(["pdf", "zip"]).is_err());
        assert!(DocKind::parse_all(Vec::<String>::new()).unwrap().is_empty());
    }
}
