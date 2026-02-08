/**
 * @module DocumentStatusBadge
 * @description Status badge component for documents with appropriate icons and colors
 * 
 * @implements FEAT0004 - Processing status tracking per document
 * @implements UC0007 - User monitors document processing progress
 * @implements OODA-11 - Stage progress tooltip
 * 
 * Processing sub-states provide visibility into:
 * - chunking: Splitting document into chunks
 * - extracting: Running LLM entity extraction
 * - embedding: Generating vector embeddings
 * - indexing: Storing in graph/vector databases
 */
'use client';

import { Badge } from '@/components/ui/badge';
import {
    Tooltip,
    TooltipContent,
    TooltipProvider,
    TooltipTrigger,
} from '@/components/ui/tooltip';
import {
    Brain,
    CheckCircle,
    Clock,
    Cpu,
    Database,
    FileText,
    GitMerge,
    Loader2,
    Scissors,
    Search,
    StopCircle,
    Upload,
    XCircle,
} from 'lucide-react';
import { memo, useMemo } from 'react';

/**
 * Status configuration with icons, colors, and labels.
 * 
 * @implements SPEC-002: Unified Ingestion Pipeline
 * 
 * WHY: Each processing stage has distinct visual identity to reduce user anxiety.
 * All stages from UnifiedStage enum are represented here for consistent UX.
 * 
 * Unified Pipeline Stages (aligned with backend UnifiedStage):
 * uploading → converting? → preprocessing → chunking → extracting → gleaning
 *     → merging → summarizing → embedding → storing → completed/failed
 */
const statusConfig = {
  // === UNIFIED STAGES (SPEC-002) ===
  
  // Upload stage
  uploading: { icon: Upload, color: 'bg-blue-400', textColor: 'text-blue-500 dark:text-blue-300', label: 'Uploading', animate: true },
  
  // Conversion stage (PDF only)
  converting: { icon: FileText, color: 'bg-indigo-500', textColor: 'text-indigo-600 dark:text-indigo-400', label: 'Converting PDF', animate: true },
  
  // Processing stages
  preprocessing: { icon: Loader2, color: 'bg-blue-500', textColor: 'text-blue-600 dark:text-blue-400', label: 'Preprocessing', animate: true },
  chunking: { icon: Scissors, color: 'bg-blue-400', textColor: 'text-blue-500 dark:text-blue-300', label: 'Chunking', animate: true },
  extracting: { icon: Brain, color: 'bg-purple-500', textColor: 'text-purple-600 dark:text-purple-400', label: 'Extracting', animate: true },
  gleaning: { icon: Search, color: 'bg-purple-400', textColor: 'text-purple-500 dark:text-purple-300', label: 'Gleaning', animate: true },
  merging: { icon: GitMerge, color: 'bg-amber-500', textColor: 'text-amber-600 dark:text-amber-400', label: 'Merging', animate: true },
  summarizing: { icon: FileText, color: 'bg-orange-500', textColor: 'text-orange-600 dark:text-orange-400', label: 'Summarizing', animate: true },
  embedding: { icon: Cpu, color: 'bg-cyan-500', textColor: 'text-cyan-600 dark:text-cyan-400', label: 'Embedding', animate: true },
  storing: { icon: Database, color: 'bg-teal-500', textColor: 'text-teal-600 dark:text-teal-400', label: 'Storing', animate: true },
  
  // Terminal states
  completed: { icon: CheckCircle, color: 'bg-green-500', textColor: 'text-green-600 dark:text-green-400', label: 'Completed', animate: false },
  failed: { icon: XCircle, color: 'bg-red-500', textColor: 'text-red-600 dark:text-red-400', label: 'Failed', animate: false },
  partial_failure: { icon: XCircle, color: 'bg-orange-500', textColor: 'text-orange-600 dark:text-orange-400', label: 'Partial Failure', animate: false },
  // OODA-03: Partial success - some chunks extracted but not all
  partial_success: { icon: CheckCircle, color: 'bg-amber-500', textColor: 'text-amber-600 dark:text-amber-400', label: 'Partial', animate: false },
  
  // === LEGACY STAGES (backward compatibility) ===
  pending: { icon: Clock, color: 'bg-yellow-500', textColor: 'text-yellow-600 dark:text-yellow-400', label: 'Pending', animate: false },
  processing: { icon: Loader2, color: 'bg-blue-500', textColor: 'text-blue-600 dark:text-blue-400', label: 'Processing', animate: true },
  indexing: { icon: Database, color: 'bg-teal-500', textColor: 'text-teal-600 dark:text-teal-400', label: 'Indexing', animate: true },
  indexed: { icon: CheckCircle, color: 'bg-green-500', textColor: 'text-green-600 dark:text-green-400', label: 'Indexed', animate: false },
  cancelled: { icon: StopCircle, color: 'bg-orange-500', textColor: 'text-orange-600 dark:text-orange-400', label: 'Cancelled', animate: false },
} as const;

export type DocumentStatus = keyof typeof statusConfig;

/**
 * OODA-11 + SPEC-002: Processing stages in order with descriptions
 * Unified to match backend UnifiedStage enum.
 */
const PROCESSING_STAGES = [
  { key: 'uploading', label: 'Uploading', description: 'Uploading file to server' },
  { key: 'converting', label: 'Converting', description: 'Converting PDF to Markdown' },
  { key: 'preprocessing', label: 'Preprocessing', description: 'Validating and preparing document' },
  { key: 'chunking', label: 'Chunking', description: 'Splitting document into chunks' },
  { key: 'extracting', label: 'Extracting', description: 'Running LLM entity extraction' },
  { key: 'gleaning', label: 'Gleaning', description: 'Second pass for missed entities' },
  { key: 'merging', label: 'Merging', description: 'Merging into knowledge graph' },
  { key: 'summarizing', label: 'Summarizing', description: 'Generating descriptions' },
  { key: 'embedding', label: 'Embedding', description: 'Generating vector embeddings' },
  { key: 'storing', label: 'Storing', description: 'Storing in graph & vector databases' },
] as const;

/**
 * Get stage progress info for a status
 */
function getStageProgress(status: DocumentStatus): { current: number; total: number; description: string } | null {
  const stageIndex = PROCESSING_STAGES.findIndex(s => s.key === status);
  if (stageIndex >= 0) {
    return {
      current: stageIndex + 1,
      total: PROCESSING_STAGES.length,
      description: PROCESSING_STAGES[stageIndex].description,
    };
  }
  if (status === 'processing') {
    return { current: 1, total: PROCESSING_STAGES.length, description: 'Starting processing...' };
  }
  return null;
}

/**
 * Check if a status represents an active processing state
 */
export function isProcessingStatus(status: DocumentStatus): boolean {
  return ['processing', 'chunking', 'extracting', 'embedding', 'indexing'].includes(status);
}

/**
 * Check if a status represents a terminal (final) state
 */
export function isTerminalStatus(status: DocumentStatus): boolean {
  return ['completed', 'indexed', 'failed', 'partial_failure', 'partial_success', 'cancelled'].includes(status);
}

/**
 * Map legacy/unknown status to known status
 * WHY: Backward compatibility with older backends
 */
export function normalizeStatus(status: string | undefined | null): DocumentStatus {
  if (!status) return 'pending';
  const normalized = status.toLowerCase();
  if (normalized in statusConfig) return normalized as DocumentStatus;
  // Map unknown processing states to generic 'processing'
  if (normalized.includes('process')) return 'processing';
  return 'pending';
}

/**
 * Get the best status to display for a document.
 * 
 * @implements SPEC-002: Unified Ingestion Pipeline
 * 
 * Prefers current_stage (new unified field) over status (legacy field).
 * Falls back to status if current_stage is not available.
 */
export function getDocumentDisplayStatus(doc: {
  current_stage?: string | null;
  status?: string | null;
}): DocumentStatus {
  // SPEC-002: Prefer unified current_stage over legacy status
  if (doc.current_stage) {
    return normalizeStatus(doc.current_stage);
  }
  // Fallback to legacy status field
  return normalizeStatus(doc.status);
}

interface StatusBadgeProps {
  status: DocumentStatus;
  /** Optional tooltip with more details */
  tooltip?: string;
  /** Optional custom stage message from backend (e.g., "Converting PDF: page 5/10 (50%)") */
  stageMessage?: string;
  /** Optional stage progress (0.0 to 1.0) */
  stageProgressValue?: number;
  /** Compact mode (icon only) */
  compact?: boolean;
  /** Disable tooltip (for use in other tooltips) */
  disableTooltip?: boolean;
}

export const StatusBadge = memo(function StatusBadge({ 
  status, 
  tooltip,
  stageMessage,
  stageProgressValue,
  compact = false,
  disableTooltip = false,
}: StatusBadgeProps) {
  const config = statusConfig[status] || statusConfig.pending;
  const Icon = config.icon;
  
  // OODA-11: Calculate stage progress for processing states
  const stageProgress = useMemo(() => getStageProgress(status), [status]);

  /**
   * OODA-17: Enhanced processing animation
   * WHY: Pulse animation on entire badge provides clearer visual feedback
   * that processing is ongoing, reducing user uncertainty.
   */
  const badge = (
    <Badge 
      variant="outline" 
      className={`gap-1 ${config.textColor} border-current cursor-default ${
        config.animate ? 'animate-pulse' : ''
      }`}
      data-testid="status-badge"
    >
      <Icon className={`h-3 w-3 ${config.animate ? 'animate-spin' : ''}`} />
      {!compact && config.label}
    </Badge>
  );

  // For non-processing states or when tooltip disabled, return simple badge
  if (!stageProgress || disableTooltip) {
    return tooltip ? <span title={tooltip}>{badge}</span> : badge;
  }

  // For processing states, show rich tooltip with stage progress
  return (
    <TooltipProvider delayDuration={300}>
      <Tooltip>
        <TooltipTrigger asChild>
          {badge}
        </TooltipTrigger>
        <TooltipContent 
          side="top" 
          className="max-w-xs"
          data-testid="status-badge-tooltip"
        >
          <div className="space-y-2">
            {/* Stage progress header */}
            <div className="flex items-center justify-between gap-4">
              <span className="font-medium">{config.label}</span>
              <span className="text-xs text-muted-foreground">
                Step {stageProgress.current}/{stageProgress.total}
              </span>
            </div>
            
            {/* Custom stage message from backend (if available) */}
            {stageMessage && (
              <p className="text-xs font-medium text-foreground">
                {stageMessage}
              </p>
            )}
            
            {/* Stage description (fallback if no custom message) */}
            {!stageMessage && (
              <p className="text-xs text-muted-foreground">
                {stageProgress.description}
              </p>
            )}
            
            {/* Progress percentage bar (if available) */}
            {typeof stageProgressValue === 'number' && (
              <div className="space-y-1">
                <div className="flex justify-between text-[10px] text-muted-foreground">
                  <span>Progress</span>
                  <span>{Math.round(stageProgressValue * 100)}%</span>
                </div>
                <div className="h-2 bg-muted rounded-full overflow-hidden">
                  <div 
                    className="h-full bg-primary transition-all duration-300"
                    style={{ width: `${stageProgressValue * 100}%` }}
                  />
                </div>
              </div>
            )}
            
            {/* Visual progress bar showing all stages */}
            <div className="flex gap-1">
              {PROCESSING_STAGES.map((stage, index) => (
                <div
                  key={stage.key}
                  className={`h-1 flex-1 rounded-full ${
                    index < stageProgress.current
                      ? 'bg-primary'
                      : 'bg-muted'
                  }`}
                  title={stage.label}
                />
              ))}
            </div>
            
            {/* Stage names */}
            <div className="flex justify-between text-[10px] text-muted-foreground">
              {PROCESSING_STAGES.map((stage, index) => (
                <span 
                  key={stage.key}
                  className={index + 1 === stageProgress.current ? 'font-medium text-foreground' : ''}
                >
                  {stage.label}
                </span>
              ))}
            </div>
          </div>
        </TooltipContent>
      </Tooltip>
    </TooltipProvider>
  );
});
