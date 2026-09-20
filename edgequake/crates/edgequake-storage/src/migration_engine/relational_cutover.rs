//! Relational-authority cutover rehearsal primitives.
//!
//! Data movement implementations may persist provider-specific cursors, but
//! orchestration uses this provider-neutral watermark and single-writer gate.

use serde::{Deserialize, Serialize};

use crate::StorageError;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RelationalAuthorityMode {
    SourceOnly,
    Exporting,
    Importing,
    TargetOnly,
    DualMaster,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RelationalWatermark {
    pub generation: u64,
    pub cursor: String,
    pub exported_rows: u64,
    pub imported_rows: u64,
}

impl RelationalWatermark {
    pub fn initial() -> Self {
        Self {
            generation: 1,
            cursor: String::new(),
            exported_rows: 0,
            imported_rows: 0,
        }
    }

    /// Export rehearsal stub: advances only the source-side durable count.
    pub fn after_export(&self, cursor: impl Into<String>, rows: u64) -> Result<Self, StorageError> {
        Ok(Self {
            generation: self.generation,
            cursor: cursor.into(),
            exported_rows: self
                .exported_rows
                .checked_add(rows)
                .ok_or_else(|| StorageError::InvalidData("export watermark overflow".into()))?,
            imported_rows: self.imported_rows,
        })
    }

    /// Import rehearsal stub: imported rows may never lead exported rows.
    pub fn after_import(&self, rows: u64) -> Result<Self, StorageError> {
        let imported_rows = self
            .imported_rows
            .checked_add(rows)
            .ok_or_else(|| StorageError::InvalidData("import watermark overflow".into()))?;
        if imported_rows > self.exported_rows {
            return Err(StorageError::InvalidData(
                "relational import watermark exceeds exported rows".into(),
            ));
        }
        Ok(Self {
            imported_rows,
            ..self.clone()
        })
    }
}

pub fn validate_authority_mode(mode: RelationalAuthorityMode) -> Result<(), StorageError> {
    if mode == RelationalAuthorityMode::DualMaster {
        return Err(StorageError::InvalidConfig(
            "relational cutover forbids dual-master authority".into(),
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dual_master_is_rejected() {
        assert!(validate_authority_mode(RelationalAuthorityMode::DualMaster).is_err());
        assert!(validate_authority_mode(RelationalAuthorityMode::TargetOnly).is_ok());
    }

    #[test]
    fn import_cannot_pass_export_watermark() {
        let watermark = RelationalWatermark::initial()
            .after_export("page-1", 10)
            .unwrap()
            .after_import(10)
            .unwrap();
        assert_eq!(watermark.exported_rows, watermark.imported_rows);
        assert!(watermark.after_import(1).is_err());
    }
}
