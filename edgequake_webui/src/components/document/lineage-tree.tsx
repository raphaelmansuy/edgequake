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
import { Brain, Database, FileSearch, Network, Upload } from 'lucide-react';

interface LineageTreeProps {
  lineage: DocumentLineage | null | undefined;
}

export function LineageTree({ lineage }: LineageTreeProps) {
  if (!lineage) return null;

  return (
    <div className="space-y-2">
      <LineageNode
        icon={<Upload className="h-3.5 w-3.5" />}
        label="Document Upload"
        status="completed"
      />
      <LineageConnector />
      {/* SPEC-040: PDF Vision Extraction node — shown only for PDF documents */}
      {lineage.pdf_vision_model && (
        <>
          <LineageNode
            icon={<FileSearch className="h-3.5 w-3.5" />}
            label="PDF → Markdown (Vision LLM)"
            details={`${lineage.pdf_extraction_method ?? 'vision'} · ${lineage.pdf_vision_model}`}
            status="completed"
          />
          <LineageConnector />
        </>
      )}
      <LineageNode
        icon={<FileSearch className="h-3.5 w-3.5" />}
        label="Content Extraction"
        details={lineage.chunking_strategy ? `${lineage.chunking_strategy} • ${lineage.avg_chunk_size} chars/chunk` : undefined}
        status="completed"
      />
      <LineageConnector />
      <LineageNode
        icon={<Brain className="h-3.5 w-3.5" />}
        label="Entity Extraction"
        details={lineage.llm_model ? `${lineage.llm_model} • ${lineage.entity_types?.length || 0} types` : undefined}
        duration={lineage.entity_extraction_ms}
        status="completed"
      />
      <LineageConnector />
      <LineageNode
        icon={<Network className="h-3.5 w-3.5" />}
        label="Relationship Mapping"
        details={`Relationships mapped`}
        duration={lineage.relationship_extraction_ms}
        status="completed"
      />
      <LineageConnector />
      <LineageNode
        icon={<Database className="h-3.5 w-3.5" />}
        label="Graph Indexing"
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
  status: 'completed' | 'processing' | 'failed';
}

function LineageNode({ icon, label, details, duration, status }: LineageNodeProps) {
  return (
    <div className="flex items-start gap-3 p-3 rounded-lg bg-muted/30 hover:bg-muted/50 transition-colors">
      <div
        className={cn(
          'flex items-center justify-center w-8 h-8 rounded-full shrink-0',
          status === 'completed' && 'bg-green-500/10 text-green-600 dark:text-green-400',
          status === 'processing' && 'bg-blue-500/10 text-blue-600 dark:text-blue-400',
          status === 'failed' && 'bg-red-500/10 text-red-600 dark:text-red-400'
        )}
      >
        {icon}
      </div>
      <div className="flex-1 min-w-0">
        <div className="flex items-center justify-between mb-1">
          <span className="text-sm font-medium">{label}</span>
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
