/**
 * @fileoverview Visual lineage tree showing extraction pipeline
 *
 * @implements FEAT1076 - Extraction pipeline visualization
 * @implements FEAT1077 - Processing step timing display
 *
 * @see UC1507 - User views extraction lineage
 * @see UC1508 - User sees processing durations
 *
 * @enforces BR1076 - Sequential step layout
 * @enforces BR1077 - Status indicators per step
 */
// Visual lineage tree showing extraction pipeline
'use client';

import { Badge } from '@/components/ui/badge';
import { cn } from '@/lib/utils';
import type { DocumentLineage } from '@/types';
import { useWorkspaceCapabilities } from '@/hooks/use-workspace-capabilities';
import { Brain, Database, FileSearch, MinusCircle, Network, Upload } from 'lucide-react';

interface LineageTreeProps {
  lineage: DocumentLineage | null | undefined;
}

export function LineageTree({ lineage }: LineageTreeProps) {
  // SPEC-180: read the capability, never the profile name. A chunk-only workspace has no
  // extraction or graph step to report as completed.
  const extractsGraph = useWorkspaceCapabilities().extracts_graph;
  if (!lineage) return null;

  return (
    <div className="space-y-2">
      <LineageNode
        icon={<Upload className="h-3.5 w-3.5" />}
        label="Document Upload"
        status="completed"
      />
      <LineageConnector />
      {/* PDF extraction node — shown only for PDF documents */}
      {(lineage.pdf_vision_model || lineage.pdf_extraction_method) && (
        <>
          <LineageNode
            icon={<FileSearch className="h-3.5 w-3.5" />}
            label={`PDF → Markdown (${lineage.pdf_extraction_method === 'edgeparse' ? 'EdgeParse' : 'Vision'})`}
            details={[
              lineage.pdf_extraction_method,
              lineage.pdf_vision_model,
              lineage.pdf_extraction_warning,
            ]
              .filter(Boolean)
              .join(' · ')}
            status="completed"
          />
          <LineageConnector />
        </>
      )}
      <LineageNode
        icon={<FileSearch className="h-3.5 w-3.5" />}
        label="Content Extraction"
        details={
          lineage.chunking_strategy
            ? `${lineage.chunking_strategy} • ${
                // An image has no characters per chunk: the payload is pixels and the
                // stored body is the file name. Printing `undefined` promised a number
                // the ingest never produced.
                typeof lineage.avg_chunk_size === 'number' && lineage.avg_chunk_size > 0
                  ? `${lineage.avg_chunk_size} chars/chunk`
                  : 'no text body'
              }`
            : undefined
        }
        status="completed"
      />
      <LineageConnector />
      {/* SPEC-180: a chunk-only (classical RAG) workspace extracts no entities. Showing a
          completed Entity Extraction step is a claim the ingest never made — and the graph
          pane contradicts it two inches away. */}
      <LineageNode
        icon={<Brain className="h-3.5 w-3.5" />}
        label="Entity Extraction"
        details={
          extractsGraph
            ? lineage.llm_model
              ? `${lineage.llm_model} • ${lineage.entity_types?.length || 0} types`
              : undefined
            : 'Chunk-only workspace · no entities extracted'
        }
        duration={extractsGraph ? lineage.entity_extraction_ms : undefined}
        status={extractsGraph ? 'completed' : 'skipped'}
      />
      <LineageConnector />
      <LineageNode
        icon={<Network className="h-3.5 w-3.5" />}
        label="Relationship Mapping"
        details={
          extractsGraph
            ? 'Relationships mapped'
            : 'Not part of this retrieval type'
        }
        duration={extractsGraph ? lineage.relationship_extraction_ms : undefined}
        status={extractsGraph ? 'completed' : 'skipped'}
      />
      <LineageConnector />
      <LineageNode
        icon={<Database className="h-3.5 w-3.5" />}
        label="Indexing"
        details={lineage.embedding_model ? `${lineage.embedding_model} • ${lineage.embedding_dimensions}D` : undefined}
        status="completed"
      />
    </div>
  );
}

interface LineageNodeProps {
  icon: React.ReactNode;
  label: string;
  details?: string;
  duration?: number;
  /** `skipped`: a real step this workspace's profile does not run. */
  status: 'completed' | 'processing' | 'failed' | 'skipped';
}

function LineageNode({ icon, label, details, duration, status }: LineageNodeProps) {
  return (
    <div className="flex items-start gap-3 p-3 rounded-lg bg-muted/30 hover:bg-muted/50 transition-colors">
      <div
        className={cn(
          'flex items-center justify-center w-8 h-8 rounded-full shrink-0',
          status === 'completed' && 'bg-green-500/10 text-green-600 dark:text-green-400',
          status === 'processing' && 'bg-blue-500/10 text-blue-600 dark:text-blue-400',
          status === 'failed' && 'bg-red-500/10 text-red-600 dark:text-red-400',
          status === 'skipped' && 'bg-muted text-muted-foreground/70'
        )}
      >
        {icon}
      </div>
      <div className="flex-1 min-w-0">
        <div className="flex items-center justify-between mb-1">
          <span
            className={cn(
              'text-sm font-medium',
              status === 'skipped' && 'text-muted-foreground',
            )}
          >
            {label}
          </span>
          {status === 'skipped' ? (
            <MinusCircle aria-hidden className="h-3.5 w-3.5 shrink-0 text-muted-foreground/60" />
          ) : null}
          {duration && (
            <Badge variant="outline" className="text-xs">
              {formatDuration(duration)}
            </Badge>
          )}
        </div>
        {details && <p className="text-xs text-muted-foreground">{details}</p>}
      </div>
    </div>
  );
}

function LineageConnector() {
  return (
    <div className="flex justify-center">
      <div className="w-px h-4 bg-border" />
    </div>
  );
}

function formatDuration(ms: number): string {
  if (ms < 1000) return `${ms}ms`;
  return `${(ms / 1000).toFixed(1)}s`;
}
