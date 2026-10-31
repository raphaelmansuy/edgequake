//! SPEC-160 — extraction mode at upload admission (one path for every upload route).
//!
//! The words travel in the document metadata envelope (`extraction_mode`,
//! `decision_gate_preset`), the same way chunk fields do. Each route calls
//! [`UploadExtraction::from_envelope`] and [`admit_extraction`]. A bad word or a down
//! backend is a 422 *before* any document state is written (LAW-160-4, EC-160-12).

use std::collections::HashMap;

use edgequake_core::Workspace;
use edgequake_pipeline::extractor::decision::gate::META_DECISION_GATE_PRESET;
use edgequake_pipeline::extractor::decision::settings::parse_gate_preset_override;
use edgequake_pipeline::extractor::decision::{DecisionError, DecisionRuntime, GatePreset};
use edgequake_pipeline::{
    extraction_mode_from_metadata, parse_mode_override, resolve_extraction_mode_from_env,
    ExtractionMode, ExtractionModeSource, META_EXTRACTION_MODE,
};
use serde_json::{json, Map, Value};

use crate::error::{ApiError, ApiResult};
use crate::services::workspace_profile::load_workspace;
use crate::state::AppState;

/// Raw upload words, before validation.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct UploadExtraction {
    pub mode: Option<String>,
    pub gate_preset: Option<String>,
}

/// Collects the two SPEC-160 form fields from a multipart request.
///
/// Shared by the text/file/batch routes and both PDF routes (DRY). A field sent twice is
/// ambiguous, so [`Self::finish`] rejects it (EC-160-12).
#[derive(Debug, Clone, Default)]
pub struct ExtractionForm {
    words: UploadExtraction,
    duplicate: Option<&'static str>,
}

impl ExtractionForm {
    /// Field names this form consumes.
    pub const FIELDS: [&'static str; 2] = ["extraction_mode", "decision_gate_preset"];

    /// Take a field if it is one of ours. Returns false for any other name.
    pub fn ingest(&mut self, name: &str, text: &str) -> bool {
        let (slot, field) = match name {
            "extraction_mode" => (&mut self.words.mode, "extraction_mode"),
            "decision_gate_preset" => (&mut self.words.gate_preset, "decision_gate_preset"),
            _ => return false,
        };
        if slot.is_some() {
            self.duplicate.get_or_insert(field);
        }
        *slot = Some(text.trim().to_string());
        true
    }

    /// The collected words, or a 422 when a field was sent twice.
    pub fn finish(&self) -> ApiResult<UploadExtraction> {
        match self.duplicate {
            Some(field) => Err(ApiError::unprocessable_coded(
                "invalid_extraction_mode",
                format!("The form field '{field}' was sent more than once."),
            )),
            None => Ok(self.words.clone()),
        }
    }
}

/// Upload words after validation. `None` means "inherit".
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ParsedUploadExtraction {
    pub mode: Option<ExtractionMode>,
    pub gate_preset: Option<GatePreset>,
}

/// What admission decided for a document.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AdmittedExtraction {
    pub mode: ExtractionMode,
    pub source: ExtractionModeSource,
}

impl AdmittedExtraction {
    pub fn mode_str(&self) -> &'static str {
        self.mode.as_str()
    }

    pub fn source_str(&self) -> &'static str {
        self.source.as_str()
    }
}

impl UploadExtraction {
    /// Read the two words from a metadata envelope.
    pub fn from_envelope(envelope: Option<&Value>) -> Self {
        let text = |key: &str| {
            envelope
                .and_then(|m| m.get(key))
                .and_then(Value::as_str)
                .map(str::to_string)
        };
        Self {
            mode: text(META_EXTRACTION_MODE),
            gate_preset: text(META_DECISION_GATE_PRESET),
        }
    }

    /// Write set words into an envelope. A set word wins over an envelope key.
    pub fn merge_into(&self, envelope: Option<Value>) -> Option<Value> {
        if self.mode.is_none() && self.gate_preset.is_none() {
            return envelope;
        }
        let mut obj: Map<String, Value> = match envelope {
            Some(Value::Object(o)) => o,
            _ => Map::new(),
        };
        if let Some(m) = &self.mode {
            obj.insert(META_EXTRACTION_MODE.into(), json!(m));
        }
        if let Some(g) = &self.gate_preset {
            obj.insert(META_DECISION_GATE_PRESET.into(), json!(g));
        }
        Some(Value::Object(obj))
    }

    /// Validate both words. 422 with a stable code on the first bad word.
    pub fn parse(&self) -> ApiResult<ParsedUploadExtraction> {
        let mode = parse_mode_override(self.mode.as_deref(), ExtractionModeSource::Document)
            .map_err(|e| ApiError::unprocessable_coded("invalid_extraction_mode", e.to_string()))?;
        let gate_preset = parse_gate_preset_override(self.gate_preset.as_deref()).map_err(|e| {
            ApiError::unprocessable_coded("invalid_decision_gate_preset", e.to_string())
        })?;
        Ok(ParsedUploadExtraction { mode, gate_preset })
    }
}

impl ParsedUploadExtraction {
    /// The words as one JSON object for payloads that carry no metadata map.
    /// `None` when both words inherit.
    pub fn task_object(&self) -> Option<Value> {
        let keys = self.task_keys();
        (!keys.is_empty())
            .then(|| Value::Object(keys.into_iter().map(|(k, v)| (k.to_string(), v)).collect()))
    }

    /// Canonical words for the task payload. Absent words stay absent (inherit).
    pub fn task_keys(&self) -> Vec<(&'static str, Value)> {
        let mut keys = Vec::new();
        if let Some(m) = self.mode {
            keys.push((META_EXTRACTION_MODE, json!(m.as_str())));
        }
        if let Some(g) = self.gate_preset {
            keys.push((META_DECISION_GATE_PRESET, json!(g.as_str())));
        }
        keys
    }
}

/// Map a decision problem to the 422 the API contract promises.
pub fn decision_to_api(err: DecisionError) -> ApiError {
    ApiError::unprocessable_coded(err.code(), err.to_string())
}

/// Resolve the mode for a new document and, for `decision`, check the backend now.
///
/// Fast fail: the worker checks again before the first chunk (EC-160-16).
pub async fn admit_extraction(
    state: &AppState,
    workspace_id: &str,
    upload: &UploadExtraction,
) -> ApiResult<AdmittedExtraction> {
    let workspace = load_workspace(state, workspace_id).await;
    let caps = workspace
        .as_ref()
        .map(Workspace::capabilities)
        .unwrap_or_default();
    // SPEC-180: a normal workspace builds no graph, so no mode applies (400, not 422).
    caps.check_extraction_mode(upload.mode.as_deref())?;
    if !caps.extracts_graph {
        return Ok(AdmittedExtraction {
            mode: ExtractionMode::Llm,
            source: ExtractionModeSource::Default,
        });
    }
    let parsed = upload.parse()?;
    let ws_meta = workspace.map(|ws| ws.metadata).unwrap_or_default();
    let ws_word = extraction_mode_from_metadata(&ws_meta);
    let doc_word = parsed.mode.map(|m| m.as_str());
    let resolved = resolve_extraction_mode_from_env(doc_word, ws_word.as_deref())
        .map_err(|e| ApiError::unprocessable_coded("invalid_extraction_mode", e.to_string()))?;
    if resolved.mode.needs_decision_backend() {
        check_decision_backend(&state.decision, &ws_meta, &parsed).await?;
    }
    Ok(AdmittedExtraction {
        mode: resolved.mode,
        source: resolved.source,
    })
}

async fn check_decision_backend(
    runtime: &DecisionRuntime,
    ws_meta: &HashMap<String, Value>,
    parsed: &ParsedUploadExtraction,
) -> ApiResult<()> {
    let doc_meta = parsed
        .gate_preset
        .map(|g| json!({ META_DECISION_GATE_PRESET: g.as_str() }));
    let settings = runtime
        .effective_settings(ws_meta, doc_meta.as_ref())
        .map_err(decision_to_api)?;
    let prepared = runtime.prepare(settings).map_err(decision_to_api)?;
    DecisionRuntime::admit(&prepared)
        .await
        .map(|_| ())
        .map_err(decision_to_api)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn words(mode: Option<&str>, gate: Option<&str>) -> UploadExtraction {
        UploadExtraction {
            mode: mode.map(str::to_string),
            gate_preset: gate.map(str::to_string),
        }
    }

    // T-160-U73 — words parse to canonical values; inherit words become None.
    #[test]
    fn parse_words() {
        let p = words(Some(" Decision "), Some("RECALL")).parse().unwrap();
        assert_eq!(p.mode, Some(ExtractionMode::Decision));
        assert_eq!(p.gate_preset, Some(GatePreset::Recall));
        let inherit = words(Some("inherit"), Some("")).parse().unwrap();
        assert_eq!(inherit, ParsedUploadExtraction::default());
        assert_eq!(UploadExtraction::default().parse().unwrap(), inherit);
    }

    // T-160-U78 — the form takes its own fields only; a repeat is a 422.
    #[test]
    fn form_collects_and_rejects_repeats() {
        let mut f = ExtractionForm::default();
        assert!(!f.ingest("title", "x"));
        assert!(f.ingest("extraction_mode", " decision "));
        assert!(f.ingest("decision_gate_preset", "recall"));
        let w = f.finish().unwrap();
        assert_eq!(w.mode.as_deref(), Some("decision"));
        assert_eq!(w.gate_preset.as_deref(), Some("recall"));
        assert!(f.ingest("extraction_mode", "llm"));
        let err = f.finish().unwrap_err();
        assert_eq!(err.code(), "invalid_extraction_mode");
        assert!(err.to_string().contains("more than once"));
    }

    // T-160-U74 — bad words are 422 with their own code (EC-160-12).
    #[test]
    fn bad_words_are_422() {
        let e = words(Some("tev1"), None).parse().unwrap_err();
        assert_eq!(e.code(), "invalid_extraction_mode");
        assert_eq!(
            e.status_code(),
            axum::http::StatusCode::UNPROCESSABLE_ENTITY
        );
        assert!(e.to_string().contains("tev1"));
        let g = words(None, Some("loose")).parse().unwrap_err();
        assert_eq!(g.code(), "invalid_decision_gate_preset");
    }

    // T-160-U75 — envelope round trip; a set word beats an envelope key.
    #[test]
    fn envelope_round_trip() {
        let env = json!({"extraction_mode": "llm", "title": "t"});
        assert_eq!(
            UploadExtraction::from_envelope(Some(&env)).mode.as_deref(),
            Some("llm")
        );
        let merged = words(Some("decision"), None).merge_into(Some(env)).unwrap();
        assert_eq!(merged["extraction_mode"], "decision");
        assert_eq!(merged["title"], "t");
        assert!(words(None, None).merge_into(None).is_none());
        let created = words(None, Some("strict")).merge_into(None).unwrap();
        assert_eq!(created["decision_gate_preset"], "strict");
    }

    // T-160-U79 — payload object only when something is set.
    #[test]
    fn task_object_shape() {
        assert!(ParsedUploadExtraction::default().task_object().is_none());
        let obj = words(Some("decision"), Some("strict"))
            .parse()
            .unwrap()
            .task_object()
            .unwrap();
        assert_eq!(
            obj,
            json!({"extraction_mode": "decision", "decision_gate_preset": "strict"})
        );
    }

    // T-160-U76 — task keys carry canonical words only when set.
    #[test]
    fn task_keys_are_canonical() {
        let keys = words(Some(" DECISION"), None).parse().unwrap().task_keys();
        assert_eq!(keys, vec![("extraction_mode", json!("decision"))]);
        assert!(ParsedUploadExtraction::default().task_keys().is_empty());
    }

    // T-160-U77 — decision problems keep their stable code at 422.
    #[test]
    fn decision_errors_map_to_422() {
        for (err, code) in [
            (DecisionError::Disabled, "decision_disabled"),
            (DecisionError::NotActivated, "decision_not_activated"),
            (
                DecisionError::Unavailable("x".into()),
                "decision_backend_unavailable",
            ),
            (
                DecisionError::ModelMissing("tev1:0.8b".into()),
                "decision_model_missing",
            ),
        ] {
            let api = decision_to_api(err);
            assert_eq!(api.code(), code);
            assert_eq!(
                api.status_code(),
                axum::http::StatusCode::UNPROCESSABLE_ENTITY
            );
        }
    }
}
