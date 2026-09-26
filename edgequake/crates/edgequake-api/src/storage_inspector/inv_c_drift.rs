//! INV-C drift rule: relational `documents.entity_count` vs AGE lineage count.
//!
//! `entity_count` is the extraction *mention* total (per-chunk entities summed
//! in `aggregate_extraction_stats`), while AGE counts *distinct* nodes whose
//! lineage names the document. An entity mentioned in several chunks makes
//! `pg > age` by design, so equality is not an invariant. Only two shapes are
//! real drift:
//! - a finished document with mentions but no node in its scoped graph
//!   (the SPEC-149 "document filter shows an empty graph" class);
//! - more distinct nodes than mentions, which extraction cannot produce.

/// Why a sampled document is flagged by INV-C.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum EntityCountDrift {
    MissingGraph,
    ExceedsMentions,
}

impl EntityCountDrift {
    pub(super) fn label(self) -> &'static str {
        match self {
            Self::MissingGraph => "graph missing",
            Self::ExceedsMentions => "nodes exceed mentions",
        }
    }
}

/// Statuses whose graph must be fully projected.
fn graph_expected(status: &str) -> bool {
    matches!(
        status.to_ascii_lowercase().as_str(),
        "indexed" | "completed"
    )
}

/// Classify one sampled document; `None` means consistent.
///
/// `pg_mentions == 0` is never drift: the column predates SPEC-021 refreshes
/// on some deployments and a zero says nothing about the graph.
pub(super) fn entity_count_drift(
    status: &str,
    pg_mentions: i64,
    age_distinct: i64,
) -> Option<EntityCountDrift> {
    if pg_mentions <= 0 {
        return None;
    }
    if age_distinct == 0 && graph_expected(status) {
        return Some(EntityCountDrift::MissingGraph);
    }
    (age_distinct > pg_mentions).then_some(EntityCountDrift::ExceedsMentions)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn repeated_mentions_are_not_drift() {
        assert_eq!(entity_count_drift("indexed", 1209, 847), None);
        assert_eq!(entity_count_drift("indexed", 18, 18), None);
    }

    #[test]
    fn finished_document_with_empty_graph_is_missing_graph() {
        assert_eq!(
            entity_count_drift("indexed", 3992, 0),
            Some(EntityCountDrift::MissingGraph)
        );
        assert_eq!(
            entity_count_drift("Completed", 5, 0),
            Some(EntityCountDrift::MissingGraph)
        );
    }

    #[test]
    fn failed_documents_may_lack_a_graph() {
        assert_eq!(entity_count_drift("failed", 40, 0), None);
        assert_eq!(entity_count_drift("partial_failure", 40, 0), None);
    }

    #[test]
    fn more_nodes_than_mentions_is_drift_for_any_terminal_status() {
        assert_eq!(
            entity_count_drift("failed", 3, 9),
            Some(EntityCountDrift::ExceedsMentions)
        );
    }

    #[test]
    fn zero_mentions_never_flags() {
        assert_eq!(entity_count_drift("indexed", 0, 0), None);
        assert_eq!(entity_count_drift("indexed", 0, 12), None);
    }
}
