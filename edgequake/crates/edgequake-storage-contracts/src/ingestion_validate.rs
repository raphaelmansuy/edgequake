//! Pure admission for prepared ingestion batches (no SQL drivers).

use crate::error::{AccessError, AccessResult};
use crate::relational::{CommitReceipt, PreparedIngestionBatch, PreparedRecord};
use uuid::Uuid;

/// Hard cap shared by every authority adapter.
pub const MAX_BATCH_RECORDS: usize = 10_000;

/// Values derived once by admission, reused by adapters for typed binds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ValidatedBatch {
    pub tenant_id: Uuid,
    pub workspace_id: Uuid,
    pub batch_ordinal: i32,
    pub expected_revision: Option<i64>,
    pub expected_count: i32,
}

/// Admit a prepared command before any adapter opens a transaction.
pub fn validate_prepared_ingestion_batch(
    command: &PreparedIngestionBatch,
) -> AccessResult<ValidatedBatch> {
    let key_chars = command.idempotency_key.chars().count();
    if !(1..=256).contains(&key_chars) {
        return Err(AccessError::InvalidInput(
            "idempotency key must contain 1..=256 characters".into(),
        ));
    }
    if command.ingest_generation == 0 {
        return Err(AccessError::InvalidInput(
            "ingest generation must be greater than zero".into(),
        ));
    }
    if command.schema_version == 0 {
        return Err(AccessError::InvalidInput(
            "schema version must be greater than zero".into(),
        ));
    }
    checked_i64(command.ingest_generation, "ingest generation")?;
    // Reject widths Postgres INT columns cannot store so SQLite cannot admit
    // values that would fail on P0.
    let _schema_i32 = i32::try_from(command.schema_version).map_err(|_| {
        AccessError::InvalidInput("schema version exceeds i32".into())
    })?;
    let batch_ordinal = i32::try_from(command.batch_ordinal)
        .map_err(|_| AccessError::InvalidInput("batch ordinal exceeds i32".into()))?;
    let expected_revision = command
        .expected_revision
        .map(|value| checked_i64(value, "expected revision"))
        .transpose()?;

    let total = command
        .chunks
        .len()
        .checked_add(command.facts.len())
        .and_then(|value| value.checked_add(command.contributions.len()))
        .and_then(|value| value.checked_add(command.embeddings.len()))
        .ok_or_else(|| AccessError::InvalidInput("batch record count overflow".into()))?;
    if total > MAX_BATCH_RECORDS {
        return Err(AccessError::InvalidInput(format!(
            "batch contains {total} records; maximum is {MAX_BATCH_RECORDS}"
        )));
    }
    let expected_count = i32::try_from(total)
        .map_err(|_| AccessError::InvalidInput("batch record count exceeds i32".into()))?;

    let mut logical_keys = std::collections::HashSet::with_capacity(total);
    for (kind, records) in [
        ("chunk", command.chunks.as_slice()),
        ("fact", command.facts.as_slice()),
        ("contribution", command.contributions.as_slice()),
        ("embedding", command.embeddings.as_slice()),
    ] {
        for record in records {
            if record.revision == 0 {
                return Err(AccessError::InvalidInput(format!(
                    "{kind} {} revision must be greater than zero",
                    record.id
                )));
            }
            checked_i64(record.revision, "record revision")?;
            if !logical_keys.insert((kind, record.id, record.revision)) {
                let conflict = records.iter().find(|prior| {
                    prior.id == record.id
                        && prior.revision == record.revision
                        && !std::ptr::eq(*prior, record)
                });
                let detail = match conflict {
                    Some(prior) if prior.digest == record.digest => {
                        "byte-identical duplicate payload"
                    }
                    Some(_) => "conflicting payload digests for the same logical revision",
                    None => "duplicate logical revision within batch",
                };
                return Err(AccessError::InvalidInput(format!(
                    "duplicate {kind} logical revision {}:{} ({detail})",
                    record.id, record.revision
                )));
            }
        }
    }

    Ok(ValidatedBatch {
        tenant_id: command.scope.tenant().into_uuid(),
        workspace_id: command.scope.workspace().into_uuid(),
        batch_ordinal,
        expected_revision,
        expected_count,
    })
}

/// Convert a non-negative `u64` into a signed integer for BIGINT / INTEGER binds.
pub fn checked_i64(value: u64, field: &str) -> AccessResult<i64> {
    i64::try_from(value)
        .map_err(|_| AccessError::InvalidInput(format!("{field} exceeds signed 64-bit integer")))
}

/// Deterministic physical id for an immutable revision row.
pub fn physical_revision_id(
    command: &PreparedIngestionBatch,
    kind: &str,
    record: &PreparedRecord,
) -> Uuid {
    let key = format!(
        "{}:{}:{kind}:{}:{}",
        command.scope.tenant(),
        command.scope.workspace(),
        record.id,
        record.revision
    );
    Uuid::new_v5(&Uuid::NAMESPACE_OID, key.as_bytes())
}

/// Decode a stored mutation receipt and verify it matches the command.
pub fn decode_commit_receipt(
    command: &PreparedIngestionBatch,
    digest: &[u8],
    receipt: &[u8],
) -> AccessResult<CommitReceipt> {
    if digest != command.canonical_digest {
        return Err(AccessError::Conflict(format!(
            "idempotency key {} was already used with another digest",
            command.idempotency_key
        )));
    }
    let decoded: CommitReceipt = serde_json::from_slice(receipt).map_err(|error| {
        AccessError::CorruptData(format!(
            "stored receipt {} is invalid: {error}",
            command.idempotency_key
        ))
    })?;
    if decoded.request_key != command.idempotency_key
        || decoded.command_digest != command.canonical_digest
    {
        return Err(AccessError::CorruptData(format!(
            "stored receipt {} does not match its mutation row",
            command.idempotency_key
        )));
    }
    Ok(decoded)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ids::{DocumentId, TenantId, WorkspaceId};
    use crate::relational::PreparedRecord;
    use crate::scope::AccessScope;
    use uuid::Uuid;

    fn base_command() -> PreparedIngestionBatch {
        PreparedIngestionBatch {
            scope: AccessScope::new(TenantId::new(Uuid::nil()), WorkspaceId::new(Uuid::nil())),
            document_id: DocumentId::new(Uuid::nil()),
            ingest_generation: 1,
            batch_ordinal: 0,
            expected_revision: Some(0),
            idempotency_key: "key-1".into(),
            schema_version: 1,
            canonical_digest: [1; 32],
            chunks: Vec::new(),
            facts: Vec::new(),
            contributions: Vec::new(),
            embeddings: Vec::new(),
        }
    }

    fn record(id: Uuid, revision: u64, digest: [u8; 32]) -> PreparedRecord {
        PreparedRecord {
            id,
            revision,
            digest,
            payload: b"payload".to_vec(),
        }
    }

    #[test]
    fn admits_valid_batch() {
        let mut command = base_command();
        let id = Uuid::new_v4();
        command.facts.push(record(id, 1, [2; 32]));
        assert!(validate_prepared_ingestion_batch(&command).is_ok());
    }

    #[test]
    fn rejects_generation_zero() {
        let mut command = base_command();
        command.ingest_generation = 0;
        let err = validate_prepared_ingestion_batch(&command).unwrap_err();
        assert!(matches!(err, AccessError::InvalidInput(_)));
    }

    #[test]
    fn rejects_revision_zero() {
        let mut command = base_command();
        command.facts.push(record(Uuid::new_v4(), 0, [2; 32]));
        let err = validate_prepared_ingestion_batch(&command).unwrap_err();
        assert!(err.to_string().contains("revision must be greater than zero"));
    }

    #[test]
    fn rejects_identical_duplicate() {
        let mut command = base_command();
        let id = Uuid::new_v4();
        let digest = [9; 32];
        command.facts.push(record(id, 1, digest));
        command.facts.push(record(id, 1, digest));
        let err = validate_prepared_ingestion_batch(&command).unwrap_err();
        assert!(err.to_string().contains("byte-identical duplicate payload"));
    }

    #[test]
    fn rejects_conflicting_digest_duplicate() {
        let mut command = base_command();
        let id = Uuid::new_v4();
        command.facts.push(record(id, 1, [1; 32]));
        command.facts.push(record(id, 1, [2; 32]));
        let err = validate_prepared_ingestion_batch(&command).unwrap_err();
        assert!(err
            .to_string()
            .contains("conflicting payload digests for the same logical revision"));
    }

    #[test]
    fn rejects_overflow_batch_size() {
        let mut command = base_command();
        command.facts = (0..=MAX_BATCH_RECORDS)
            .map(|i| record(Uuid::from_u128(i as u128 + 1), 1, [3; 32]))
            .collect();
        let err = validate_prepared_ingestion_batch(&command).unwrap_err();
        assert!(err.to_string().contains("maximum is"));
    }

    #[test]
    fn decode_receipt_requires_matching_digest() {
        let command = base_command();
        let receipt = CommitReceipt {
            request_key: command.idempotency_key.clone(),
            command_digest: command.canonical_digest,
            document_generation: 1,
            committed: Vec::new(),
            manifest_id: Uuid::nil(),
            durable_commit_token: "t".into(),
        };
        let bytes = serde_json::to_vec(&receipt).unwrap();
        assert!(decode_commit_receipt(&command, &command.canonical_digest, &bytes).is_ok());
        assert!(matches!(
            decode_commit_receipt(&command, &[0; 32], &bytes),
            Err(AccessError::Conflict(_))
        ));
    }
}
