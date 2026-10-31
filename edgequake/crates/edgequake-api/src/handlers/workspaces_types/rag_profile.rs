//! SPEC-180: workspace DTO fragments for the retrieval profile.
//!
//! Flattened into the create / update request and the response, so the three
//! fields are declared once (DRY). Parsing and every rule live in
//! `edgequake_core::rag_profile`; this file only moves them across the wire.

use edgequake_core::rag_profile::{
    check_create_combination, EmbeddingModalities, FtsRegconfig, ProfileCapabilities, ProfileError,
    RagProfile,
};
use edgequake_core::Workspace;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// Request fragment. Every field is optional. On create, absent means the default.
/// On update, **presence** is the error (409): the profile never changes.
#[derive(Debug, Clone, Default, Serialize, Deserialize, ToSchema)]
pub struct RagProfileFieldsDto {
    /// `graph` (default) or `normal`. Immutable after create.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rag_profile: Option<String>,
    /// `text` (default) or `text,image`. Immutable. `text,image` needs `normal`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub embedding_modalities: Option<String>,
    /// `english` (default), `simple`, `french`, `german` or `spanish`. Immutable.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fts_regconfig: Option<String>,
}

/// The parsed, defaulted, cross-checked create values.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ResolvedProfile {
    pub profile: RagProfile,
    pub modalities: EmbeddingModalities,
    pub regconfig: FtsRegconfig,
}

impl ResolvedProfile {
    pub fn capabilities(&self) -> ProfileCapabilities {
        ProfileCapabilities::new(self.profile, self.modalities)
    }
}

impl RagProfileFieldsDto {
    /// Parse and cross-check for create (400 for a bad word, 422 for a bad pair).
    pub fn resolve_for_create(&self) -> Result<ResolvedProfile, ProfileError> {
        let profile = self
            .rag_profile
            .as_deref()
            .map(RagProfile::parse)
            .transpose()?
            .unwrap_or_default();
        let modalities = self
            .embedding_modalities
            .as_deref()
            .map(EmbeddingModalities::parse)
            .transpose()?
            .unwrap_or_default();
        let regconfig = self
            .fts_regconfig
            .as_deref()
            .map(FtsRegconfig::parse)
            .transpose()?
            .unwrap_or_default();
        check_create_combination(profile, modalities)?;
        Ok(ResolvedProfile {
            profile,
            modalities,
            regconfig,
        })
    }

    /// Carry the presence markers to the core update request.
    pub fn into_update_markers(self) -> (Option<String>, Option<String>, Option<String>) {
        (
            self.rag_profile,
            self.embedding_modalities,
            self.fts_regconfig,
        )
    }
}

/// Capability booleans the UI reads. The UI never re-derives the matrix (10-api-contract).
#[derive(Debug, Clone, Copy, Serialize, Deserialize, ToSchema, PartialEq, Eq)]
pub struct ProfileCapabilitiesDto {
    pub extracts_graph: bool,
    pub embeds_images: bool,
    pub writes_bm25: bool,
    pub queries_graph: bool,
    pub allows_graph_modes: bool,
    pub shows_graph_ui: bool,
}

impl From<ProfileCapabilities> for ProfileCapabilitiesDto {
    fn from(c: ProfileCapabilities) -> Self {
        Self {
            extracts_graph: c.extracts_graph,
            embeds_images: c.embeds_images,
            writes_bm25: c.writes_bm25,
            queries_graph: c.queries_graph,
            allows_graph_modes: c.allows_graph_modes,
            shows_graph_ui: c.shows_graph_ui,
        }
    }
}

/// Response fragment.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct RagProfileView {
    /// `graph` or `normal`.
    pub rag_profile: String,
    /// `text` or `text,image`.
    pub embedding_modalities: String,
    /// Text-search configuration of the BM25 index.
    pub fts_regconfig: String,
    /// Read-only booleans for the UI.
    pub rag_profile_capabilities: ProfileCapabilitiesDto,
}

impl Default for RagProfileView {
    fn default() -> Self {
        Self::from_workspace_fields(
            RagProfile::default(),
            EmbeddingModalities::default(),
            FtsRegconfig::default(),
        )
    }
}

impl RagProfileView {
    pub fn from_workspace(workspace: &Workspace) -> Self {
        Self::from_workspace_fields(
            workspace.rag_profile,
            workspace.embedding_modalities,
            workspace.fts_regconfig,
        )
    }

    fn from_workspace_fields(
        profile: RagProfile,
        modalities: EmbeddingModalities,
        regconfig: FtsRegconfig,
    ) -> Self {
        Self {
            rag_profile: profile.as_str().to_string(),
            embedding_modalities: modalities.as_str().to_string(),
            fts_regconfig: regconfig.as_str().to_string(),
            rag_profile_capabilities: profile.capabilities(modalities).into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use edgequake_core::rag_profile::codes;

    fn dto(p: Option<&str>, m: Option<&str>, f: Option<&str>) -> RagProfileFieldsDto {
        RagProfileFieldsDto {
            rag_profile: p.map(str::to_string),
            embedding_modalities: m.map(str::to_string),
            fts_regconfig: f.map(str::to_string),
        }
    }

    #[test]
    fn absent_fields_resolve_to_graph_text_english() {
        let r = dto(None, None, None).resolve_for_create().unwrap();
        assert_eq!(r.profile, RagProfile::Graph);
        assert_eq!(r.modalities, EmbeddingModalities::Text);
        assert_eq!(r.regconfig, FtsRegconfig::English);
    }

    #[test]
    fn bad_words_carry_their_codes() {
        let cases = [
            (
                dto(Some("chunk_only"), None, None),
                codes::INVALID_RAG_PROFILE,
            ),
            (
                dto(None, Some("audio"), None),
                codes::INVALID_EMBEDDING_MODALITIES,
            ),
            (
                dto(None, None, Some("klingon")),
                codes::INVALID_FTS_REGCONFIG,
            ),
            (
                dto(Some("graph"), Some("text,image"), None),
                codes::MODALITIES_REQUIRE_NORMAL,
            ),
        ];
        for (d, code) in cases {
            assert_eq!(d.resolve_for_create().unwrap_err().code, code);
        }
    }

    #[test]
    fn normal_text_image_resolves() {
        let r = dto(Some("normal"), Some("text,image"), Some("french"))
            .resolve_for_create()
            .unwrap();
        assert!(r.capabilities().embeds_images);
        assert_eq!(r.regconfig, FtsRegconfig::French);
    }

    #[test]
    fn view_reports_capabilities() {
        let view = RagProfileView::default();
        assert_eq!(view.rag_profile, "graph");
        assert!(view.rag_profile_capabilities.shows_graph_ui);
        let normal = RagProfileView::from_workspace_fields(
            RagProfile::Normal,
            EmbeddingModalities::TextImage,
            FtsRegconfig::Simple,
        );
        assert!(!normal.rag_profile_capabilities.shows_graph_ui);
        assert!(normal.rag_profile_capabilities.embeds_images);
        assert_eq!(normal.embedding_modalities, "text,image");
    }

    #[test]
    fn flatten_keeps_the_wire_names() {
        let d: RagProfileFieldsDto =
            serde_json::from_str(r#"{"rag_profile":"normal","embedding_modalities":"text"}"#)
                .unwrap();
        assert_eq!(d.rag_profile.as_deref(), Some("normal"));
        let json = serde_json::to_value(RagProfileView::default()).unwrap();
        assert!(json.get("rag_profile_capabilities").is_some());
    }
}
