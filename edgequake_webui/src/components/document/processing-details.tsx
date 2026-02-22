/**
 * @fileoverview Processing details component showing LLM and embedding info
 *
 * @implements FEAT1082 - Model information display
 * @implements FEAT1083 - Entity type badges
 *
 * @see UC1513 - User views processing configuration
 * @see UC1514 - User sees entity types extracted
 *
 * @enforces BR1082 - Monospace for model names
 * @enforces BR1083 - Badge layout for entity types
 */
// Processing details component
'use client';

import { Badge } from '@/components/ui/badge';
import type { DocumentLineage } from '@/types';

interface ProcessingDetailsProps {
  lineage: DocumentLineage | null | undefined;
}

export function ProcessingDetails({ lineage }: ProcessingDetailsProps) {
  if (!lineage) return null;

  return (
    <div className="space-y-4">
      {/* Model Information */}
      <div className="grid gap-y-3 text-sm">
        {/* SPEC-040: Vision LLM used for PDF→Markdown extraction */}
        {lineage.pdf_vision_model && (
          <DetailRow
            label="Vision LLM (PDF)"
            value={`${lineage.pdf_vision_model}${lineage.pdf_extraction_method ? ` · ${lineage.pdf_extraction_method}` : ''}`}
            mono
          />
        )}
        {lineage.llm_model && (
          <DetailRow label="LLM Model" value={lineage.llm_model} mono />
        )}
        {lineage.embedding_model && (
          <DetailRow label="Embedding Model" value={lineage.embedding_model} mono />
        )}
        {lineage.embedding_dimensions && (
          <DetailRow label="Embedding Dimensions" value={lineage.embedding_dimensions.toString()} />
        )}
        {lineage.chunking_strategy && (
          <DetailRow label="Chunking Strategy" value={lineage.chunking_strategy} />
        )}
        {lineage.avg_chunk_size && (
          <DetailRow label="Avg Chunk Size" value={`${lineage.avg_chunk_size} chars`} />
        )}
      </div>

      {/* Entity Types */}
      {lineage.entity_types && lineage.entity_types.length > 0 && (
        <div>
          <span className="text-sm text-muted-foreground block mb-2">Entity Types Extracted</span>
          <div className="flex flex-wrap gap-1.5">
            {lineage.entity_types.map((type: string) => (
              <Badge key={type} variant="secondary" className="text-xs">
                {type}
              </Badge>
            ))}
          </div>
        </div>
      )}

      {/* Relationship Types */}
      {lineage.relationship_types && lineage.relationship_types.length > 0 && (
        <div>
          <span className="text-sm text-muted-foreground block mb-2">Relationship Types</span>
          <div className="flex flex-wrap gap-1.5">
            {lineage.relationship_types.map((type: string) => (
              <Badge key={type} variant="outline" className="text-xs">
                {type}
              </Badge>
            ))}
          </div>
        </div>
      )}

      {/* Keywords */}
      {lineage.keywords && lineage.keywords.length > 0 && (
        <div>
          <span className="text-sm text-muted-foreground block mb-2">
            Keywords ({lineage.keywords.length})
          </span>
          <div className="flex flex-wrap gap-1">
            {lineage.keywords.slice(0, 20).map((keyword: string) => (
              <Badge key={keyword} variant="outline" className="text-xs font-normal">
                {keyword}
              </Badge>
            ))}
            {lineage.keywords.length > 20 && (
              <Badge variant="secondary" className="text-xs">
                +{lineage.keywords.length - 20} more
              </Badge>
            )}
          </div>
        </div>
      )}
    </div>
  );
}

interface DetailRowProps {
  label: string;
  value: string;
  mono?: boolean;
}

function DetailRow({ label, value, mono }: DetailRowProps) {
  return (
    <div>
      <span className="text-muted-foreground block text-xs mb-0.5">{label}</span>
      <p className={`font-medium ${mono ? 'font-mono text-xs' : ''}`}>{value}</p>
    </div>
  );
}
