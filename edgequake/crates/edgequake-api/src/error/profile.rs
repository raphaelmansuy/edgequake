//! SPEC-180: one mapping from a profile-law code to an HTTP status (DRY).
//!
//! Handlers, the MCP gateway and the upload path return `ProfileError`; this is the
//! only place that decides 400, 409 or 422 for a code.

use axum::http::StatusCode;
use edgequake_core::rag_profile::{codes, ProfileError};

use super::ApiError;

/// Status for a stable profile code (SPEC-180 10-api-contract).
pub fn status_for_profile_code(code: &str) -> StatusCode {
    match code {
        codes::INVALID_RAG_PROFILE
        | codes::INVALID_EMBEDDING_MODALITIES
        | codes::INVALID_FTS_REGCONFIG
        | codes::INVALID_DOC_KIND
        | codes::INVALID_SOURCE_MODALITY
        | codes::EXTRACTION_MODE_NOT_APPLICABLE
        | codes::QUERY_IMAGE_INVALID => StatusCode::BAD_REQUEST,
        codes::RAG_PROFILE_IMMUTABLE
        | codes::EMBEDDING_MODALITIES_IMMUTABLE
        | codes::FTS_REGCONFIG_IMMUTABLE
        | codes::PROFILE_HAS_NO_GRAPH
        | codes::REPROCESS_MODE_NOT_APPLICABLE => StatusCode::CONFLICT,
        _ => StatusCode::UNPROCESSABLE_ENTITY,
    }
}

impl From<ProfileError> for ApiError {
    fn from(err: ProfileError) -> Self {
        ApiError::Coded {
            status: status_for_profile_code(err.code),
            code: err.code,
            message: err.message,
        }
    }
}

/// Recover the coded error a core service wrapped as `code: message` text.
/// Returns `None` when the text does not start with a known profile code.
pub fn parse_tagged_profile_error(text: &str) -> Option<ApiError> {
    const KNOWN: [&str; 19] = [
        codes::INVALID_RAG_PROFILE,
        codes::INVALID_EMBEDDING_MODALITIES,
        codes::INVALID_FTS_REGCONFIG,
        codes::INVALID_DOC_KIND,
        codes::EXTRACTION_MODE_NOT_APPLICABLE,
        codes::MODALITIES_REQUIRE_NORMAL,
        codes::EMBEDDING_MODEL_NOT_MULTIMODAL,
        codes::EMBEDDING_DIMENSION_REJECTED,
        codes::RAG_PROFILE_IMMUTABLE,
        codes::EMBEDDING_MODALITIES_IMMUTABLE,
        codes::FTS_REGCONFIG_IMMUTABLE,
        codes::PROFILE_HAS_NO_GRAPH,
        codes::MODE_NOT_SUPPORTED_BY_PROFILE,
        codes::REPROCESS_MODE_NOT_APPLICABLE,
        codes::IMAGE_QUERY_NOT_SUPPORTED,
        codes::QUERY_IMAGE_INVALID,
        codes::NO_CHUNKS,
        codes::INVALID_SOURCE_MODALITY,
        codes::KIND_FILTER_NOT_SUPPORTED_BY_PROFILE,
    ];
    let body = text
        .trim_start_matches("Validation error: ")
        .trim_start_matches("Conflict: ");
    let (head, message) = body.split_once(": ")?;
    let code = KNOWN.iter().find(|c| **c == head.trim())?;
    Some(ApiError::Coded {
        status: status_for_profile_code(code),
        code,
        message: message.to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn statuses_follow_the_contract() {
        assert_eq!(
            status_for_profile_code(codes::INVALID_RAG_PROFILE),
            StatusCode::BAD_REQUEST
        );
        assert_eq!(
            status_for_profile_code(codes::RAG_PROFILE_IMMUTABLE),
            StatusCode::CONFLICT
        );
        assert_eq!(
            status_for_profile_code(codes::PROFILE_HAS_NO_GRAPH),
            StatusCode::CONFLICT
        );
        assert_eq!(
            status_for_profile_code(codes::MODALITIES_REQUIRE_NORMAL),
            StatusCode::UNPROCESSABLE_ENTITY
        );
        assert_eq!(
            status_for_profile_code(codes::MODE_NOT_SUPPORTED_BY_PROFILE),
            StatusCode::UNPROCESSABLE_ENTITY
        );
    }

    #[test]
    fn tagged_text_round_trips_to_a_coded_error() {
        let err = ProfileError::new(
            codes::FTS_REGCONFIG_IMMUTABLE,
            "fts_regconfig cannot change",
        );
        let core = err.clone().into_conflict().to_string();
        let api = parse_tagged_profile_error(&core).expect("known code");
        assert_eq!(api.status_code(), StatusCode::CONFLICT);
        assert_eq!(api.code(), codes::FTS_REGCONFIG_IMMUTABLE);
        assert!(parse_tagged_profile_error("something else: x").is_none());
        assert!(parse_tagged_profile_error("no colon here").is_none());
    }
}
