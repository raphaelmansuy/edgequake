/**
 * Stage Indicator Component
 * 
 * Pipeline stage visualization with progress.
 * Based on WebUI Specification Document WEBUI-004 (13-webui-components.md)
 *
 * @implements FEAT1060 - Pipeline stage visualization
 * @implements FEAT1061 - Stage progress tracking
 *
 * @see UC1401 - User monitors ingestion progress
 * @see UC1402 - User identifies current processing stage
 *
 * @enforces BR1060 - Color-coded status indicators
 * @enforces BR1061 - Horizontal/vertical layout variants
 */

'use client';

import { Tooltip, TooltipContent, TooltipTrigger } from '@/components/ui/tooltip';
import { cn } from '@/lib/utils';
import type { IngestionStage } from '@/types/ingestion';
import { AlertCircle, Check, Circle, Loader2 } from 'lucide-react';

export interface Stage {
  id: IngestionStage;
  label: string;
  status: 'pending' | 'running' | 'completed' | 'failed';
  progress?: number; // 0-100 for running stage
  duration?: number; // ms
  message?: string;
}

interface StageIndicatorProps {
  stages: Stage[];
  currentStage: IngestionStage;
  variant?: 'horizontal' | 'vertical';
  showDetails?: boolean;
  className?: string;
}

/**
 * Stage labels for display - aligned with UnifiedStage (SPEC-002).
 * Includes both new unified stages and legacy aliases.
 */
const STAGE_LABELS: Record<IngestionStage, string> = {
  // Unified stages (SPEC-002)
  uploading: 'Uploading',
  converting: 'Converting PDF',
  preprocessing: 'Pre-process',
  chunking: 'Chunking',
  extracting: 'Extracting',
  gleaning: 'Gleaning',
  merging: 'Merging',
  summarizing: 'Summarizing',
  embedding: 'Embedding',
  storing: 'Storing',
  completed: 'Completed',
  failed: 'Failed',
  // Legacy aliases
  pending: 'Pending',
  indexing: 'Indexing',
};

// Stage icons
function StageIcon({ status }: { status: Stage['status'] }) {
  const baseClass = 'h-5 w-5';
  
  switch (status) {
    case 'completed':
      return <Check className={cn(baseClass, 'text-green-500')} />;
    case 'running':
      return <Loader2 className={cn(baseClass, 'text-blue-500 animate-spin')} />;
    case 'failed':
      return <AlertCircle className={cn(baseClass, 'text-red-500')} />;
    case 'pending':
    default:
      return <Circle className={cn(baseClass, 'text-muted-foreground/50')} />;
  }
}

// Format duration in human-readable format
function formatDuration(ms: number): string {
  if (ms < 1000) return `${ms}ms`;
  if (ms < 60000) return `${(ms / 1000).toFixed(1)}s`;
  return `${(ms / 60000).toFixed(1)}m`;
}

/**
 * Displays pipeline stages with status indicators.
 * 
 * Variants:
 * - horizontal: Timeline view for desktop
 * - vertical: Stacked view for mobile/sidebar
 */
export function StageIndicator({
  stages,
  currentStage: _currentStage, // eslint-disable-line @typescript-eslint/no-unused-vars
  variant = 'horizontal',
  showDetails = true,
  className,
}: StageIndicatorProps) {
  if (variant === 'vertical') {
    return (
      <div className={cn('space-y-2', className)}>
        {stages.map((stage, index) => (
          <VerticalStage
            key={stage.id}
            stage={stage}
            isLast={index === stages.length - 1}
            showDetails={showDetails}
          />
        ))}
      </div>
    );
  }

  return (
    <div className={cn('flex items-center gap-2', className)}>
      {stages.map((stage, index) => (
        <HorizontalStage
          key={stage.id}
          stage={stage}
          isLast={index === stages.length - 1}
          showDetails={showDetails}
        />
      ))}
    </div>
  );
}

// Horizontal stage component
function HorizontalStage({
  stage,
  isLast,
  showDetails,
}: {
  stage: Stage;
  isLast: boolean;
  showDetails: boolean;
}) {
  const statusColor = {
    completed: 'border-green-500 bg-green-50 dark:bg-green-950/30',
    running: 'border-blue-500 bg-blue-50 dark:bg-blue-950/30',
    failed: 'border-red-500 bg-red-50 dark:bg-red-950/30',
    pending: 'border-muted-foreground/30 bg-muted/30',
  };

  const connectorColor = {
    completed: 'bg-green-500',
    running: 'bg-blue-500',
    failed: 'bg-red-500',
    pending: 'bg-muted-foreground/30',
  };

  return (
    <>
      <Tooltip>
        <TooltipTrigger asChild>
          <div
            className={cn(
              'flex flex-col items-center gap-1 min-w-[60px]',
              stage.status === 'running' && 'scale-105 transition-transform'
            )}
          >
            {/* Stage circle with icon */}
            <div
              className={cn(
                'flex items-center justify-center w-10 h-10 rounded-full border-2',
                statusColor[stage.status]
              )}
            >
              <StageIcon status={stage.status} />
            </div>
            
            {/* Stage label */}
            <span className="text-xs font-medium text-center">
              {STAGE_LABELS[stage.id] || stage.label}
            </span>
            
            {/* Details: duration or progress */}
            {showDetails && (
              <span className="text-xs text-muted-foreground">
                {stage.status === 'running' && stage.progress !== undefined
                  ? `${Math.round(stage.progress)}%`
                  : stage.duration
                    ? formatDuration(stage.duration)
                    : '—'}
              </span>
            )}
          </div>
        </TooltipTrigger>
        
        <TooltipContent>
          <div className="text-sm">
            <p className="font-medium">{STAGE_LABELS[stage.id] || stage.label}</p>
            {stage.message && <p className="text-muted-foreground">{stage.message}</p>}
            {stage.duration && <p>Duration: {formatDuration(stage.duration)}</p>}
            {stage.status === 'running' && stage.progress !== undefined && (
              <p>Progress: {Math.round(stage.progress)}%</p>
            )}
          </div>
        </TooltipContent>
      </Tooltip>
      
      {/* Connector line */}
      {!isLast && (
        <div
          className={cn(
            'h-0.5 w-6 flex-shrink-0',
            connectorColor[stage.status]
          )}
        />
      )}
    </>
  );
}

// Vertical stage component
function VerticalStage({
  stage,
  isLast,
  showDetails,
}: {
  stage: Stage;
  isLast: boolean;
  showDetails: boolean;
}) {
  const statusBorder = {
    completed: 'border-l-green-500',
    running: 'border-l-blue-500',
    failed: 'border-l-red-500',
    pending: 'border-l-muted-foreground/30',
  };

  const statusBg = {
    completed: 'bg-green-50 dark:bg-green-950/20',
    running: 'bg-blue-50 dark:bg-blue-950/20',
    failed: 'bg-red-50 dark:bg-red-950/20',
    pending: 'bg-muted/20',
  };

  return (
    <div className="relative">
      <div
        className={cn(
          'flex items-start gap-3 p-3 rounded-lg border-l-4',
          statusBorder[stage.status],
          statusBg[stage.status]
        )}
      >
        {/* Icon */}
        <StageIcon status={stage.status} />
        
        {/* Content */}
        <div className="flex-1 min-w-0">
          <div className="flex items-center justify-between gap-2">
            <span className="font-medium text-sm">
              {STAGE_LABELS[stage.id] || stage.label}
            </span>
            <span className="text-xs text-muted-foreground">
              {stage.status === 'running' && stage.progress !== undefined
                ? `${Math.round(stage.progress)}%`
                : stage.duration
                  ? formatDuration(stage.duration)
                  : stage.status === 'pending'
                    ? 'pending'
                    : '—'}
            </span>
          </div>
          
          {showDetails && stage.message && (
            <p className="text-xs text-muted-foreground mt-1 truncate">
              {stage.message}
            </p>
          )}
          
          {/* Progress bar for running stage */}
          {stage.status === 'running' && stage.progress !== undefined && (
            <div className="mt-2 h-1 bg-muted rounded-full overflow-hidden">
              <div
                className="h-full bg-blue-500 transition-all duration-300"
                style={{ width: `${stage.progress}%` }}
              />
            </div>
          )}
        </div>
      </div>
      
      {/* Vertical connector */}
      {!isLast && (
        <div
          className={cn(
            'absolute left-[1.125rem] top-full w-0.5 h-2',
            stage.status === 'completed' ? 'bg-green-500' : 'bg-muted-foreground/30'
          )}
        />
      )}
    </div>
  );
}

/**
 * Creates default stages array from ingestion stages.
 */
export function createDefaultStages(currentStage?: IngestionStage): Stage[] {
  const allStages: IngestionStage[] = [
    'preprocessing',
    'chunking',
    'extracting',
    'gleaning',
    'merging',
    'summarizing',
    'indexing',
  ];

  const currentIndex = currentStage ? allStages.indexOf(currentStage) : -1;

  return allStages.map((id, index) => ({
    id,
    label: STAGE_LABELS[id],
    status: index < currentIndex
      ? 'completed'
      : index === currentIndex
        ? 'running'
        : 'pending',
  }));
}

export default StageIndicator;
