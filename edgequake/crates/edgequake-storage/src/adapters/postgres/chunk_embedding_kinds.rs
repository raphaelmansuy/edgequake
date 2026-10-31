//! SPEC-180: chunk ANN restricted by document kind, MIME type and chunk source modality.
//!
//! A separate statement, so the tuned unfiltered / document / modality / id statements in
//! `chunk_embedding_index` keep their exact text. It joins `documents` for the two
//! document-level predicates and filters `chunks.source_modality` for the third.

use crate::traits::ChunkKinds;

/// Binds: `$1` vector, `$2` model, `$3` workspace, `$4` documents, `$5` tenant,
/// `$6` chunk modalities, `$7` vector type, `$8` candidate limit, `$9` source modalities,
/// `$10` document kinds, `$11` MIME types.
pub(crate) fn search_sql(dim: i32, cast: &str) -> String {
    format!(
        r#"
        SELECT ce.chunk_id,
               1.0 - ((ce.embedding::{cast}) <=> $1::{cast}) AS score
        FROM chunk_embeddings ce
        JOIN chunks c ON c.id = ce.chunk_id
        JOIN documents d ON d.id = c.document_id
        WHERE ce.model_id = $2
          AND ce.dimensions = {dim}
          AND ($3::uuid IS NULL OR ce.workspace_id = $3)
          AND ($4::uuid[] IS NULL OR c.document_id = ANY($4))
          AND ($5::uuid IS NULL OR c.tenant_id = $5)
          AND ($6::text[] IS NULL OR c.modality = ANY($6))
          AND ($7::text IS NULL OR lower($7) = 'chunk')
          AND ($9::text[] IS NULL OR c.source_modality = ANY($9))
          AND ($10::text[] IS NULL OR d.doc_kind = ANY($10))
          AND ($11::text[] IS NULL OR lower(d.content_type) = ANY($11))
        ORDER BY (ce.embedding::{cast}) <=> $1::{cast}
        LIMIT $8
        "#
    )
}

/// The three bind lists, lower-cased the way the columns are stored.
pub(crate) struct KindBinds {
    pub source_modalities: Option<Vec<String>>,
    pub doc_kinds: Option<Vec<String>>,
    pub mime_types: Option<Vec<String>>,
}

impl KindBinds {
    pub(crate) fn from_kinds(kinds: &ChunkKinds) -> Self {
        let lower = |list: &Option<Vec<String>>| {
            list.as_ref()
                .map(|l| l.iter().map(|v| v.trim().to_ascii_lowercase()).collect())
        };
        Self {
            source_modalities: lower(&kinds.source_modalities),
            doc_kinds: lower(&kinds.doc_kinds),
            mime_types: lower(&kinds.mime_types),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn statement_filters_all_three_kinds_and_keeps_scope() {
        let sql = search_sql(1536, "halfvec(1536)");
        assert!(sql.contains("c.source_modality = ANY($9)"));
        assert!(sql.contains("d.doc_kind = ANY($10)"));
        assert!(sql.contains("lower(d.content_type) = ANY($11)"));
        assert!(sql.contains("ce.workspace_id = $3"));
        assert!(sql.contains("LIMIT $8"));
    }

    #[test]
    fn binds_are_trimmed_and_lowercased() {
        let kinds = ChunkKinds {
            doc_kinds: Some(vec![" PDF ".into()]),
            mime_types: Some(vec!["Image/PNG".into()]),
            source_modalities: None,
        };
        let b = KindBinds::from_kinds(&kinds);
        assert_eq!(b.doc_kinds, Some(vec!["pdf".to_string()]));
        assert_eq!(b.mime_types, Some(vec!["image/png".to_string()]));
        assert!(b.source_modalities.is_none());
    }
}
