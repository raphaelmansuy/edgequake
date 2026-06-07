//! Document lineage construction from chunks and extractions.

use crate::chunker::TextChunk;
use crate::extractor::ExtractionResult;
use crate::lineage::{DocumentLineage, ExtractionMetadata, LineageBuilder, SourceSpan};

use super::super::{Pipeline, ProcessingStats};

impl Pipeline {
    pub(in crate::pipeline) fn build_lineage(
        &self,
        document_id: &str,
        chunks: &[TextChunk],
        extractions: &[ExtractionResult],
        stats: &ProcessingStats,
    ) -> Option<DocumentLineage> {
        if !self.config.enable_lineage_tracking {
            return None;
        }

        let job_id = uuid::Uuid::new_v4().to_string();
        let mut builder = LineageBuilder::new(document_id, document_id, &job_id);

        // Record chunks with their line numbers
        for chunk in chunks {
            let metadata = ExtractionMetadata::new(stats.llm_model.as_deref().unwrap_or("unknown"));
            builder.record_chunk(
                &chunk.id,
                chunk.index,
                chunk.start_line,
                chunk.end_line,
                chunk.start_offset,
                chunk.end_offset,
                metadata,
            );
        }

        // Record entities and relationships from extractions
        for extraction in extractions {
            for entity in &extraction.entities {
                let entity_id = format!("{}_{}", extraction.source_chunk_id, entity.name);
                let span = SourceSpan::new(0, 0, 0, 0);
                builder.record_entity(
                    &entity_id,
                    &entity.name,
                    &extraction.source_chunk_id,
                    span,
                    &entity.description,
                );
            }

            for rel in &extraction.relationships {
                let rel_id = format!(
                    "{}_{}_{}",
                    extraction.source_chunk_id, rel.source, rel.target
                );
                let span = SourceSpan::new(0, 0, 0, 0);
                builder.record_relationship(
                    &rel_id,
                    &rel.source,
                    &rel.target,
                    &rel.relation_type,
                    &extraction.source_chunk_id,
                    span,
                    &rel.description,
                );
            }
        }

        Some(builder.build())
    }
}
