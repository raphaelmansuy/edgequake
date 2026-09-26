/**
 * @module DocumentStatusBadge
 * @description Presentation-only status badge (icons, colors, labels).
 *
 * Domain helpers (normalize / display / terminal / processing) live exclusively
 * in `@/lib/documents/status-domain` (SPEC-099 LAW-099-1).
 *
 * @implements FEAT0004 - Processing status tracking per document
 * @implements UC0007 - User monitors document processing progress
 * @implements OODA-11 - Stage progress tooltip
 * @implements SPEC-099 F-099-01 - badge is presentation map only
 */
'use client';

import { Badge } from '@/components/ui/badge';
import {
    Tooltip,
    TooltipContent,
    TooltipProvider,
    TooltipTrigger,
} from '@/components/ui/tooltip';
import type { DocumentStatus } from '@/lib/documents/status-domain';
import {
    Brain,
    CheckCircle,
    Clock,
    Cpu,
    Database,
    BrushCleaning,
    FileText,
    GitMerge,
    Loader2,
    PauseCircle,
    Scissors,
    Search,
    StopCircle,
    Trash2,
    Upload,
    XCircle,
} from 'lucide-react';
import { memo, useMemo } from 'react';
import { useTranslation } from 'react-i18next';

export type { DocumentStatus };

/** Coarse statuses that share `documents.status.*` filter vocabulary (SPEC-122 DRY). */
const COARSE_STATUS_I18N_KEYS: Partial<Record<DocumentStatus, string>> = {
  pending: 'documents.status.pending',
  processing: 'documents.status.processing',
  completed: 'documents.status.completed',
  failed: 'documents.status.failed',
  partial_failure: 'documents.status.partial_failure',
  cancelled: 'documents.status.cancelled',
};

/**
 * Status configuration with icons, colors, and labels.
 *
 * Semantic families:
 *   Amber  → pending/waiting states
 *   Blue   → standard in-progress stages
 *   Purple → AI-powered processing stages
 *   Green  → success terminal states
 *   Red    → failure terminal states
 *   Orange → warning / partial outcomes
 */
const statusConfig = {
  // === PENDING / ADMISSION / DELETE ===
  cleaning: { icon: BrushCleaning, color: 'bg-rose-500', textColor: 'text-rose-600 dark:text-rose-400', label: 'Cleaning', animate: true },
  deleting: { icon: Trash2, color: 'bg-rose-500', textColor: 'text-rose-600 dark:text-rose-400', label: 'Deleting', animate: true },
  // SPEC-098 LAW-098-11: lifecycle failure ≠ pipeline Failed.
  delete_failed: { icon: XCircle, color: 'bg-rose-500', textColor: 'text-rose-600 dark:text-rose-400', label: 'Delete failed', animate: false },
  queued: { icon: Clock, color: 'bg-amber-500', textColor: 'text-amber-600 dark:text-amber-400', label: 'Queued', animate: true },
  pending: { icon: Clock, color: 'bg-amber-500', textColor: 'text-amber-600 dark:text-amber-400', label: 'Pending', animate: false },

  // === IN PROGRESS (Blue family — standard pipeline stages) ===
  uploading: { icon: Upload, color: 'bg-blue-500', textColor: 'text-blue-600 dark:text-blue-400', label: 'Uploading', animate: true },
  converting: { icon: FileText, color: 'bg-blue-500', textColor: 'text-blue-600 dark:text-blue-400', label: 'Converting', animate: true },
  preprocessing: { icon: Loader2, color: 'bg-blue-500', textColor: 'text-blue-600 dark:text-blue-400', label: 'Processing', animate: true },
  chunking: { icon: Scissors, color: 'bg-blue-500', textColor: 'text-blue-600 dark:text-blue-400', label: 'Chunking', animate: true },
  embedding: { icon: Cpu, color: 'bg-blue-500', textColor: 'text-blue-600 dark:text-blue-400', label: 'Embedding', animate: true },
  re_embedding: { icon: Cpu, color: 'bg-cyan-500', textColor: 'text-cyan-600 dark:text-cyan-400', label: 'Re-embedding', animate: true },
  storing: { icon: Database, color: 'bg-blue-500', textColor: 'text-blue-600 dark:text-blue-400', label: 'Storing', animate: true },
  projecting: { icon: Database, color: 'bg-indigo-500', textColor: 'text-indigo-600 dark:text-indigo-400', label: 'Projecting', animate: true },
  processing: { icon: Loader2, color: 'bg-blue-500', textColor: 'text-blue-600 dark:text-blue-400', label: 'Processing', animate: true },
  indexing: { icon: Database, color: 'bg-blue-500', textColor: 'text-blue-600 dark:text-blue-400', label: 'Indexing', animate: true },

  // === AI PROCESSING (Purple family — LLM-driven stages) ===
  extracting: { icon: Brain, color: 'bg-purple-500', textColor: 'text-purple-600 dark:text-purple-400', label: 'Extracting', animate: true },
  gleaning: { icon: Search, color: 'bg-purple-500', textColor: 'text-purple-600 dark:text-purple-400', label: 'Refining entities', animate: true },
  merging: { icon: GitMerge, color: 'bg-purple-500', textColor: 'text-purple-600 dark:text-purple-400', label: 'Updating knowledge graph', animate: true },
  summarizing: { icon: FileText, color: 'bg-purple-500', textColor: 'text-purple-600 dark:text-purple-400', label: 'Summarizing', animate: true },

  // === SUCCESS (Green) ===
  completed: { icon: CheckCircle, color: 'bg-green-500', textColor: 'text-green-600 dark:text-green-400', label: 'Completed', animate: false },
  indexed: { icon: CheckCircle, color: 'bg-green-500', textColor: 'text-green-600 dark:text-green-400', label: 'Indexed', animate: false },
  partial_success: { icon: CheckCircle, color: 'bg-green-500', textColor: 'text-green-600 dark:text-green-400', label: 'Partial', animate: false },

  // === FAILURE (Red) ===
  failed: { icon: XCircle, color: 'bg-red-500', textColor: 'text-red-600 dark:text-red-400', label: 'Failed', animate: false },

  // === WARNING / PARTIAL (Orange) ===
  partial_failure: { icon: XCircle, color: 'bg-orange-500', textColor: 'text-orange-600 dark:text-orange-400', label: 'Partial Failure', animate: false },
  cancelled: { icon: StopCircle, color: 'bg-orange-500', textColor: 'text-orange-600 dark:text-orange-400', label: 'Cancelled', animate: false },

  // === CANCEL IN FLIGHT (SPEC-057 P4) ===
  stopping: { icon: Loader2, color: 'bg-orange-500', textColor: 'text-orange-600 dark:text-orange-400', label: 'Stopping…', animate: true },
  cancelling: { icon: Loader2, color: 'bg-orange-500', textColor: 'text-orange-600 dark:text-orange-400', label: 'Cancelling…', animate: true },

  // === TASK LIFECYCLE (SPEC-099 EC-099-15) ===
  held: { icon: PauseCircle, color: 'bg-amber-500', textColor: 'text-amber-600 dark:text-amber-400', label: 'Held', animate: false },
  dead_letter: { icon: XCircle, color: 'bg-red-500', textColor: 'text-red-600 dark:text-red-400', label: 'Dead letter', animate: false },
} as const satisfies Record<
  DocumentStatus,
  {
    icon: typeof Clock;
    color: string;
    textColor: string;
    label: string;
    animate: boolean;
  }
>;

/**
 * OODA-11 + SPEC-002: Processing stages in order with descriptions
 */
const PROCESSING_STAGES = [
  { key: 'uploading', label: 'Uploading', description: 'Uploading file to server' },
  { key: 'converting', label: 'Converting', description: 'Converting PDF to Markdown' },
  { key: 'preprocessing', label: 'Preprocessing', description: 'Validating and preparing document' },
  { key: 'chunking', label: 'Chunking', description: 'Splitting document into chunks' },
  { key: 'extracting', label: 'Extracting', description: 'Running LLM entity extraction' },
  { key: 'gleaning', label: 'Refining entities', description: 'Second pass for missed entities' },
  { key: 'merging', label: 'Updating knowledge graph', description: 'Merging into knowledge graph' },
  { key: 'summarizing', label: 'Summarizing', description: 'Generating descriptions' },
  { key: 'embedding', label: 'Embedding', description: 'Generating vector embeddings' },
  { key: 're_embedding', label: 'Re-embedding', description: 'Re-generating embeddings after slim checkpoint' },
  { key: 'storing', label: 'Storing', description: 'Storing in graph & vector databases' },
  { key: 'projecting', label: 'Projecting', description: 'Applying projections to graph and vectors' },
] as const;

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
  const { t } = useTranslation();
  const config = statusConfig[status] ?? statusConfig.pending;
  const Icon = config.icon;
  const i18nKey = COARSE_STATUS_I18N_KEYS[status];
  const label = i18nKey ? t(i18nKey, config.label) : config.label;
  
  const stageProgress = useMemo(() => getStageProgress(status), [status]);

  /**
   * MI-06: Differentiated animation strategy.
   *   animate:true  → badge pulses (subtle, whole pill)
   *   AI stages     → icon also spins (stronger processing signal)
   *   terminal      → no animation
   */
  const AI_STAGES = new Set(['extracting', 'gleaning', 'merging', 'summarizing']);
  const spinIcon = AI_STAGES.has(status) && config.animate;
  const pulseBadge = config.animate;

  const badge = (
    <Badge
      variant="outline"
      className={`max-w-full min-w-0 gap-1 truncate ${config.textColor} border-current cursor-default${pulseBadge ? ' motion-safe:animate-pulse' : ''}`}
      data-testid="status-badge"
      title={label}
    >
      <Icon className={`h-3 w-3 shrink-0${spinIcon ? ' animate-spin' : ''}`} />
      {!compact && <span className="truncate">{label}</span>}
    </Badge>
  );

  if (!stageProgress || disableTooltip) {
    return tooltip ? <span title={tooltip}>{badge}</span> : badge;
  }

  return (
    <TooltipProvider delayDuration={300}>
      <Tooltip delayDuration={300}>
        <TooltipTrigger asChild>
          {badge}
        </TooltipTrigger>
        <TooltipContent 
          side="top" 
          className="max-w-xs"
          data-testid="status-badge-tooltip"
        >
          <div className="space-y-2">
            <div className="flex items-center justify-between gap-4">
              <span className="font-medium">{config.label}</span>
              <span className="text-xs text-foreground/90">
                Step {stageProgress.current}/{stageProgress.total}
              </span>
            </div>
            
            {stageMessage && (
              <p className="text-xs font-medium text-foreground">
                {stageMessage}
              </p>
            )}
            
            {!stageMessage && (
              <p className="text-xs text-muted-foreground">
                {stageProgress.description}
              </p>
            )}
            
            {typeof stageProgressValue === 'number' && (
              <div className="space-y-1">
                <div className="flex justify-between text-[10px] text-foreground/90">
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
            
            <div className="flex justify-between text-[10px] text-muted-foreground">
              {PROCESSING_STAGES.map((stage, index) => (
                <span 
                  key={stage.key}
                  className={
                    index + 1 === stageProgress.current
                      ? 'font-medium text-foreground'
                      : 'text-foreground/90'
                  }
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
