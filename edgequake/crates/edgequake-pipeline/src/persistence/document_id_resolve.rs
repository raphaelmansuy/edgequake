//! SPEC-118 — resolve pipeline document ids into relational `DocumentId`s.
//!
//! Knowledge injection uses composite artifact ids (`injection::{ws}::{id}`) for
//! graph provenance and citation exclusion (SPEC-0002). Relational chunk /
//! embedding writers require a UUID FK into `public.documents` (SPEC-091).
//! Wave B6 already stores injection rows under the bare injection UUID — this
//! helper bridges the two identities (LAW-118-1..3).

use edgequake_storage::traits::domain::DocumentId;
use edgequake_storage::StorageError;
use uuid::Uuid;

const INJECTION_PREFIX: &str = "injection::";

/// True when `raw` is the SPEC-0002 injection composite document id.
pub fn is_injection_composite_document_id(raw: &str) -> bool {
    raw.starts_with(INJECTION_PREFIX)
}

/// Resolve a pipeline `document_id` string into a relational [`DocumentId`].
///
/// - bare UUID → `DocumentId`
/// - `injection::{anything}::{uuid}` → trailing UUID segment
/// - otherwise → `StorageError::InvalidData` (fail-closed for unknown ids)
pub fn resolve_relational_document_id(raw: &str) -> Result<DocumentId, StorageError> {
    if let Ok(u) = Uuid::parse_str(raw) {
        return Ok(DocumentId::new(u));
    }
    if let Some(mapped) = map_injection_composite(raw) {
        return Ok(DocumentId::new(mapped));
    }
    Err(StorageError::InvalidData(format!(
        "invalid uuid '{raw}': not a bare UUID or injection::{{ws}}::{{uuid}} composite"
    )))
}

fn map_injection_composite(raw: &str) -> Option<Uuid> {
    let rest = raw.strip_prefix(INJECTION_PREFIX)?;
    // Require at least one `::` separator so we have workspace + injection id.
    let (_workspace, injection_id) = rest.rsplit_once("::")?;
    if injection_id.is_empty() || injection_id.contains("::") {
        return None;
    }
    Uuid::parse_str(injection_id).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn contract_spec118_resolve_bare_uuid() {
        let id = Uuid::new_v4();
        let got = resolve_relational_document_id(&id.to_string()).expect("bare uuid");
        assert_eq!(got.into_uuid(), id);
        assert!(!is_injection_composite_document_id(&id.to_string()));
    }

    #[test]
    fn contract_spec118_resolve_injection_composite() {
        let ws = Uuid::new_v4();
        let inj = Uuid::new_v4();
        let raw = format!("injection::{ws}::{inj}");
        let got = resolve_relational_document_id(&raw).expect("injection composite");
        assert_eq!(got.into_uuid(), inj);
        assert!(is_injection_composite_document_id(&raw));
    }

    #[test]
    fn contract_spec118_resolve_rejects_garbage() {
        let err = resolve_relational_document_id("not-a-uuid").unwrap_err();
        assert!(matches!(err, StorageError::InvalidData(_)));
    }

    #[test]
    fn contract_spec118_resolve_rejects_malformed_injection() {
        for raw in [
            "injection::only-one-part",
            "injection::",
            "injection::ws::",
            "injection::ws::not-a-uuid",
            "injection::a::b::c",
        ] {
            let err = resolve_relational_document_id(raw).unwrap_err();
            assert!(
                matches!(err, StorageError::InvalidData(_)),
                "expected InvalidData for {raw}"
            );
        }
    }

    #[test]
    fn contract_spec118_issue376_length_85_shape() {
        // Exact shape from GitHub #376 (len 85).
        let raw =
            "injection::00000000-0000-0000-0000-000000000000::3fc4a415-33e7-4a38-88d9-86ae6b8bb36e";
        assert_eq!(raw.len(), 85);
        let got = resolve_relational_document_id(raw).expect("map issue shape");
        assert_eq!(
            got.into_uuid(),
            Uuid::parse_str("3fc4a415-33e7-4a38-88d9-86ae6b8bb36e").unwrap()
        );
    }
}
