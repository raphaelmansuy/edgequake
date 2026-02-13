/**
 * @fileoverview Enhanced metadata display fetching additional metadata from API
 *
 * WHY: The Document type only carries a subset of metadata fields. The KV storage
 * contains richer metadata (PDF extraction details, checksums, chunk stats) that
 * the standard document list endpoint doesn't return. This component fetches the
 * dedicated /documents/:id/metadata endpoint to display all available fields.
 *
 * @implements FEAT1086 - Enhanced metadata display from KV storage
 * @see UC1517 - User views complete document metadata
 * @enforces BR1086 - All metadata fields displayed in organized grid
 */
'use client';

import { Badge } from '@/components/ui/badge';
import { useDocumentMetadata } from '@/hooks/use-lineage';
import { Loader2 } from 'lucide-react';

interface EnhancedMetadataProps {
  documentId: string;
}

/**
 * Fields already shown by other components (SourceInfoGrid, ProcessingDetails).
 * We skip these to avoid duplication.
 */
const SKIP_FIELDS = new Set([
  'id',
  'document_id',
  'content',
  'status',
  'error_message',
  'title',
  'file_name',
  'file_size',
  'mime_type',
  'content_summary',
  'content_length',
  'content_hash',
  'created_at',
  'updated_at',
  'processed_at',
  'lineage',
  'track_id',
  'tenant_id',
  'workspace_id',
  'chunk_count',
  'entity_count',
  'relationship_count',
  'cost_usd',
  'input_tokens',
  'output_tokens',
  'total_tokens',
  'llm_model',
  'embedding_model',
  'current_stage',
  'stage_progress',
  'stage_message',
  'pdf_id',
  'source_type',
  // OODA-10 fields already shown in SourceInfoGrid
  'document_type',
  'sha256_checksum',
  'page_count',
  'file_size_bytes',
]);

/**
 * Format field name from snake_case to Title Case.
 */
function formatFieldName(key: string): string {
  return key
    .split('_')
    .map((w) => w.charAt(0).toUpperCase() + w.slice(1))
    .join(' ');
}

/**
 * Format a metadata value for display.
 */
function formatValue(value: unknown): string {
  if (value === null || value === undefined) return '-';
  if (typeof value === 'boolean') return value ? 'Yes' : 'No';
  if (typeof value === 'number') return value.toLocaleString();
  if (typeof value === 'string') {
    // Truncate long strings
    if (value.length > 64) return value.slice(0, 61) + '...';
    return value;
  }
  if (Array.isArray(value)) {
    if (value.length === 0) return '(empty)';
    return `[${value.length} items]`;
  }
  if (typeof value === 'object') return JSON.stringify(value).slice(0, 64);
  return String(value);
}

/**
 * Check if a value looks like a hash/checksum for monospace display.
 */
function isHashLike(key: string, value: unknown): boolean {
  if (typeof value !== 'string') return false;
  return (
    key.includes('hash') ||
    key.includes('checksum') ||
    key.includes('sha') ||
    key.includes('model') ||
    key.includes('provider') ||
    key.includes('_id')
  );
}

export function EnhancedMetadata({ documentId }: EnhancedMetadataProps) {
  const { data: metadata, isLoading, error } = useDocumentMetadata(documentId);

  if (isLoading) {
    return (
      <div className="flex items-center gap-2 text-sm text-muted-foreground p-2">
        <Loader2 className="h-3.5 w-3.5 animate-spin" />
        Loading metadata...
      </div>
    );
  }

  if (error || !metadata) {
    return (
      <p className="text-xs text-muted-foreground p-2">
        No enhanced metadata available
      </p>
    );
  }

  // Extract fields not already shown by other components
  const extraFields = Object.entries(metadata as Record<string, unknown>)
    .filter(([key]) => !SKIP_FIELDS.has(key))
    .filter(([, value]) => value !== null && value !== undefined && value !== '');

  if (extraFields.length === 0) {
    return (
      <p className="text-xs text-muted-foreground p-2">
        All metadata is displayed in other sections
      </p>
    );
  }

  return (
    <div className="space-y-3">
      <div className="grid gap-y-3 text-sm">
        {extraFields.map(([key, value]) => (
          <div key={key}>
            <span className="text-muted-foreground block text-xs mb-0.5">
              {formatFieldName(key)}
            </span>
            {Array.isArray(value) && value.length > 0 ? (
              <div className="flex flex-wrap gap-1">
                {(value as string[]).slice(0, 10).map((item, i) => (
                  <Badge key={`${key}-${i}`} variant="outline" className="text-xs font-normal">
                    {String(item)}
                  </Badge>
                ))}
                {value.length > 10 && (
                  <Badge variant="secondary" className="text-xs">
                    +{value.length - 10} more
                  </Badge>
                )}
              </div>
            ) : (
              <p
                className={`font-medium truncate ${
                  isHashLike(key, value) ? 'font-mono text-xs' : ''
                }`}
              >
                {formatValue(value)}
              </p>
            )}
          </div>
        ))}
      </div>
    </div>
  );
}
