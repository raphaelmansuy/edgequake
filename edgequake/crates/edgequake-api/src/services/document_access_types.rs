//! SPEC-180 canonical access envelope shared by every ingestion adapter.
use edgequake_core::clearance::{rank_from_name, ObjectLabel};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::error::{ApiError, ApiResult};

pub const MAX_SECURITY_TAGS: usize = 32;
pub const MAX_ACCESS_BYTES: usize = 16 * 1024;

/// Omission is a legacy request; an explicit null is a malformed typed envelope.
pub fn deserialize_optional_access<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> Result<Option<DocumentAccess>, D::Error> {
    DocumentAccess::deserialize(deserializer).map(Some)
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct DocumentAccess {
    pub classification: String,
    pub compartment_ids: Vec<Uuid>,
    pub expected_policy_epoch: i64,
}

impl DocumentAccess {
    pub fn normalize(&self) -> ApiResult<Self> {
        if rank_from_name(&self.classification).is_none() {
            return Err(ApiError::unprocessable_coded(
                "INVALID_CLASSIFICATION",
                "Choose a valid sensitivity",
            ));
        }
        let mut normalized = self.clone();
        normalized.compartment_ids.sort();
        normalized.compartment_ids.dedup();
        if normalized.compartment_ids.len() > MAX_SECURITY_TAGS {
            return Err(ApiError::unprocessable_coded(
                "UNASSIGNABLE_TAG",
                "Choose at most 32 security tags",
            ));
        }
        Ok(normalized)
    }

    pub fn label(&self, uploader: Option<Uuid>) -> ApiResult<ObjectLabel> {
        let normalized = self.normalize()?;
        let rank = rank_from_name(&normalized.classification).ok_or_else(|| {
            ApiError::unprocessable_coded("INVALID_CLASSIFICATION", "Choose a valid sensitivity")
        })?;
        let mut label = ObjectLabel::new(rank, normalized.compartment_ids);
        label.uploaded_by = uploader;
        Ok(label)
    }
}

/// Multipart parsing rejects repeated, malformed, oversized and partial envelopes.
#[derive(Debug, Default, Clone)]
pub struct AccessForm {
    raw: Option<String>,
    repeated: bool,
}
impl AccessForm {
    pub fn ingest(&mut self, value: &str) {
        self.repeated |= self.raw.is_some();
        self.raw = Some(value.to_owned());
    }
    pub fn finish(&self) -> ApiResult<Option<DocumentAccess>> {
        if self.repeated
            || self
                .raw
                .as_ref()
                .is_some_and(|raw| raw.len() > MAX_ACCESS_BYTES)
        {
            return Err(ApiError::bad_request_coded(
                "INVALID_ACCESS_PAYLOAD",
                "Invalid or repeated access field",
            ));
        }
        self.raw
            .as_deref()
            .map(|raw| {
                let access: DocumentAccess = serde_json::from_str(raw).map_err(|_| {
                    ApiError::bad_request_coded("INVALID_ACCESS_PAYLOAD", "Invalid access field")
                })?;
                access.normalize()
            })
            .transpose()
    }
}

pub fn validate_security_metadata(metadata: Option<&serde_json::Value>) -> ApiResult<()> {
    if metadata.is_some_and(|value| {
        [
            "access",
            "classification",
            "classification_rank",
            "compartment_ids",
            "uploaded_by",
            "steward",
        ]
        .iter()
        .any(|key| value.get(key).is_some())
    }) {
        return Err(ApiError::unprocessable_coded(
            "RESERVED_SECURITY_METADATA",
            "Security attributes belong in the typed access field",
        ));
    }
    Ok(())
}

pub fn broadens(before: &ObjectLabel, after: &ObjectLabel) -> bool {
    matches!((before.rank, after.rank), (Some(a), Some(b)) if b < a)
        || !before.compartments.is_subset(&after.compartments)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn access_form_rejects_null_partial_repeated_and_oversized() {
        for raw in ["null", "{}", "{\"classification\":\"public\"}"] {
            let mut form = AccessForm::default();
            form.ingest(raw);
            assert!(form.finish().is_err());
        }
        let mut form = AccessForm::default();
        form.ingest(&" ".repeat(MAX_ACCESS_BYTES + 1));
        assert!(form.finish().is_err());
        form.ingest("{}");
        assert!(form.finish().is_err());
    }
    #[test]
    fn canonical_set_and_mixed_change() {
        let id = Uuid::new_v4();
        let access = DocumentAccess {
            classification: "confidential".into(),
            compartment_ids: vec![id, id],
            expected_policy_epoch: 1,
        };
        assert_eq!(access.normalize().unwrap().compartment_ids, vec![id]);
        assert!(broadens(
            &ObjectLabel::new(2, [id]),
            &ObjectLabel::new(4, [])
        ));
        assert!(!broadens(
            &ObjectLabel::new(2, [id]),
            &ObjectLabel::new(4, [id])
        ));
    }
}
