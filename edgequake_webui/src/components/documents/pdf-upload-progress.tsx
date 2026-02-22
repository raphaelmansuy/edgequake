/**
 * @module PdfUploadProgress
 * @description 6-phase PDF upload progress indicator component.
 * Shows real-time progress through all pipeline phases with ETA.
 *
 * @implements OODA-21: PdfUploadProgress component
 * @implements OODA-29: ErrorBanner integration
 * @implements UC0709: User sees estimated time remaining
 * @implements FEAT0606: Multi-phase progress tracking with ETA
 *
 * @enforces BR0302: Progress visible for all active uploads
 * @enforces BR0707: ETA updates based on actual processing time
 *
 * @see {@link specs/001-upload-pdf.md} Mission specification
 */

"use client";

import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Progress } from "@/components/ui/progress";
import {
    Tooltip,
    TooltipContent,
    TooltipProvider,
    TooltipTrigger,
} from "@/components/ui/tooltip";
import {
    type PhaseInfo,
    type PipelinePhase,
    usePdfProgress,
} from "@/hooks/use-pdf-progress";
import { cn } from "@/lib/utils";
import {
    AlertCircle,
    CheckCircle,
    Clock,
    FileText,
    Loader2,
    RefreshCw,
    StopCircle,
    XCircle,
} from "lucide-react";
import { useEffect, useMemo, useRef } from "react";
import { ErrorBanner } from "./error-banner";

// ============================================================================
// Types
// ============================================================================

interface PdfUploadProgressProps {
  /** Track ID for the PDF upload */
  trackId: string | null;
  /** Filename to display */
  filename?: string;
  /** Compact mode (single line) */
  compact?: boolean;
  /** Callback when completed */
  onComplete?: () => void;
  /** Callback when failed */
  onFailed?: (error: string) => void;
  /** Additional CSS classes */
  className?: string;
}

// ============================================================================
// Phase Icon Helper
// ============================================================================

const PHASE_ICONS: Record<PipelinePhase, React.ComponentType<{ className?: string }>> = {
  upload: FileText,
  pdf_conversion: FileText,
  chunking: FileText,
  embedding: FileText,
  extraction: FileText,
  graph_storage: FileText,
};

function getPhaseStatusIcon(status: PhaseInfo["status"]) {
  switch (status.type) {
    case "pending":
      return <Clock className="h-4 w-4 text-muted-foreground" />;
    case "active":
      return <Loader2 className="h-4 w-4 text-blue-500 animate-spin" />;
    case "completed":
      return <CheckCircle className="h-4 w-4 text-green-500" />;
    case "failed":
      return <XCircle className="h-4 w-4 text-red-500" />;
    default:
      return <Clock className="h-4 w-4 text-muted-foreground" />;
  }
}

function getStatusBadgeVariant(
  status: PhaseInfo["status"]["type"]
): "default" | "secondary" | "destructive" | "outline" {
  switch (status) {
    case "completed":
      return "default";
    case "active":
      return "secondary";
    case "failed":
      return "destructive";
    default:
      return "outline";
  }
}

// ============================================================================
// Sub-components
// ============================================================================

/**
 * Single phase indicator in the pipeline.
 */
function PhaseIndicator({
  phase,
  isLast,
}: {
  phase: PhaseInfo;
  isLast: boolean;
}) {
  const Icon = PHASE_ICONS[phase.phase];
  const statusIcon = getPhaseStatusIcon(phase.status);

  return (
    <TooltipProvider>
      <Tooltip>
        <TooltipTrigger asChild>
          <div className="flex flex-col items-center">
            {/* Phase circle */}
            <div
              className={cn(
                "relative flex h-10 w-10 items-center justify-center rounded-full border-2",
                phase.status.type === "pending" &&
                  "border-muted bg-muted/50",
                phase.status.type === "active" &&
                  "border-blue-500 bg-blue-50 dark:bg-blue-950",
                phase.status.type === "completed" &&
                  "border-green-500 bg-green-50 dark:bg-green-950",
                phase.status.type === "failed" &&
                  "border-red-500 bg-red-50 dark:bg-red-950"
              )}
            >
              {statusIcon}
            </div>
            {/* Phase label */}
            <span
              className={cn(
                "mt-1 text-xs font-medium",
                phase.status.type === "active" && "text-blue-600 dark:text-blue-400",
                phase.status.type === "completed" && "text-green-600 dark:text-green-400",
                phase.status.type === "failed" && "text-red-600 dark:text-red-400",
                phase.status.type === "pending" && "text-muted-foreground"
              )}
            >
              {phase.label}
            </span>
            {/* Progress for active phase */}
            {phase.status.type === "active" && (
              <span className="text-[10px] text-muted-foreground">
                {phase.status.current}/{phase.status.total}
              </span>
            )}
          </div>
        </TooltipTrigger>
        <TooltipContent>
          <div className="text-sm">
            <p className="font-medium">{phase.label}</p>
            <p className="text-muted-foreground">{phase.description}</p>
            {phase.status.type === "active" && (
              <>
                <p className="mt-1 text-blue-600">
                  Processing: {phase.status.current} of {phase.status.total} (
                  {Math.round(phase.status.percent)}%)
                </p>
                <p className="mt-1 text-blue-500 text-xs italic">{phase.message}</p>
              </>
            )}
            {phase.status.type === "failed" && (
              <p className="mt-1 text-red-600">Error: {phase.status.error}</p>
            )}
          </div>
        </TooltipContent>
      </Tooltip>
    </TooltipProvider>
  );
}

/**
 * Connector line between phases.
 */
function PhaseConnector({
  completed,
  active,
}: {
  completed: boolean;
  active: boolean;
}) {
  return (
    <div
      className={cn(
        "h-0.5 flex-1 mx-1 transition-colors",
        completed && "bg-green-500",
        active && "bg-blue-500",
        !completed && !active && "bg-muted"
      )}
    />
  );
}

/**
 * ETA display with countdown.
 */
function EtaDisplay({ seconds }: { seconds: number | null }) {
  const formatted = useMemo(() => {
    if (seconds === null || seconds <= 0) return null;
    if (seconds < 60) return `~${seconds}s`;
    const minutes = Math.floor(seconds / 60);
    const secs = seconds % 60;
    if (minutes < 60) return `~${minutes}m ${secs}s`;
    const hours = Math.floor(minutes / 60);
    const mins = minutes % 60;
    return `~${hours}h ${mins}m`;
  }, [seconds]);

  if (!formatted) return null;

  return (
    <div className="flex items-center gap-1 text-sm text-muted-foreground">
      <Clock className="h-4 w-4" />
      <span>{formatted} remaining</span>
    </div>
  );
}

/**
 * OODA-29: Extract error code from error message for classification.
 */
function extractErrorCode(errorMessage: string): string {
  const lowerMsg = errorMessage.toLowerCase();
  if (lowerMsg.includes("timeout")) return "timeout_error";
  if (lowerMsg.includes("network")) return "network_error";
  if (lowerMsg.includes("rate limit") || lowerMsg.includes("429")) return "rate_limit";
  if (lowerMsg.includes("parse") || lowerMsg.includes("corrupt")) return "parse_error";
  if (lowerMsg.includes("llm") || lowerMsg.includes("openai")) return "llm_error";
  if (lowerMsg.includes("storage") || lowerMsg.includes("database")) return "storage_error";
  return "unknown_error";
}

/**
 * OODA-29: Get the name of the failed phase from phase info.
 */
function getFailedPhaseName(phases: PhaseInfo[]): string | undefined {
  const failedPhase = phases.find(p => p.status.type === "failed");
  return failedPhase?.label;
}

// ============================================================================
// Main Component
// ============================================================================

/**
 * PDF upload progress component with 6-phase visualization.
 *
 * @example
 * ```tsx
 * <PdfUploadProgress
 *   trackId={uploadTrackId}
 *   filename="document.pdf"
 *   onComplete={() => toast.success("Upload complete!")}
 * />
 * ```
 */
export function PdfUploadProgress({
  trackId,
  filename,
  compact = false,
  onComplete,
  onFailed,
  className,
}: PdfUploadProgressProps) {
  const {
    progress,
    isLoading,
    phases,
    currentPhaseIndex,
    overallPercent,
    etaSeconds,
    retry,
    cancel,
    isRetrying,
    isCancelling,
    error,
  } = usePdfProgress(trackId);

  // WHY: useEffect (not useMemo) because calling parent setState during render
  // causes "Cannot update a component while rendering a different component"
  // and eventually "Maximum update depth exceeded".
  // The ref prevents double-firing on re-renders with unstable callback refs.
  const completionFiredRef = useRef(false);
  const failureFiredRef = useRef(false);

  useEffect(() => {
    if (progress?.status === "completed" && onComplete && !completionFiredRef.current) {
      completionFiredRef.current = true;
      onComplete();
    }
    if (progress?.status === "failed" && onFailed && progress.error && !failureFiredRef.current) {
      failureFiredRef.current = true;
      onFailed(progress.error);
    }
  }, [progress?.status, progress?.error, onComplete, onFailed]);

  if (!trackId) {
    return null;
  }

  if (isLoading) {
    return (
      <Card className={cn("animate-pulse", className)}>
        <CardContent className="flex items-center gap-4 py-4">
          <Loader2 className="h-5 w-5 animate-spin text-muted-foreground" />
          <span className="text-sm text-muted-foreground">
            Loading progress...
          </span>
        </CardContent>
      </Card>
    );
  }

  if (error) {
    return (
      <Card className={cn("border-red-200 bg-red-50/50", className)}>
        <CardContent className="flex items-center gap-4 py-4">
          <AlertCircle className="h-5 w-5 text-red-500" />
          <span className="text-sm text-red-600">
            Failed to load progress: {error.message}
          </span>
        </CardContent>
      </Card>
    );
  }

  const displayFilename = filename || progress?.filename || "PDF Document";
  const isFailed = progress?.status === "failed";
  const isCompleted = progress?.status === "completed";
  const isProcessing = progress?.status === "processing";

  // Compact mode: single line with progress bar
  if (compact) {
    return (
      <div className={cn("flex items-center gap-4", className)}>
        <div className="flex-1">
          <div className="flex items-center justify-between mb-1">
            <span className="text-sm font-medium truncate max-w-[200px]">
              {displayFilename}
            </span>
            <span className="text-xs text-muted-foreground">
              {overallPercent}%
            </span>
          </div>
          <Progress value={overallPercent} className="h-2" />
        </div>
        {isFailed && (
          <Button
            size="sm"
            variant="outline"
            onClick={() => retry()}
            disabled={isRetrying}
          >
            {isRetrying ? (
              <Loader2 className="h-4 w-4 animate-spin" />
            ) : (
              <RefreshCw className="h-4 w-4" />
            )}
          </Button>
        )}
        {isProcessing && (
          <Button
            size="sm"
            variant="ghost"
            onClick={() => cancel()}
            disabled={isCancelling}
          >
            {isCancelling ? (
              <Loader2 className="h-4 w-4 animate-spin" />
            ) : (
              <StopCircle className="h-4 w-4" />
            )}
          </Button>
        )}
      </div>
    );
  }

  // Full mode: card with phase timeline
  return (
    <Card className={cn(isFailed && "border-red-200", className)}>
      <CardHeader className="pb-2">
        <div className="flex items-center justify-between">
          <CardTitle className="text-base font-medium flex items-center gap-2">
            <FileText className="h-5 w-5 text-muted-foreground" />
            <span className="truncate max-w-[300px]">{displayFilename}</span>
          </CardTitle>
          <div className="flex items-center gap-2">
            {isCompleted && (
              <Badge variant="default" className="bg-green-500">
                Complete
              </Badge>
            )}
            {isFailed && (
              <Badge variant="destructive">Failed</Badge>
            )}
            {isProcessing && (
              <Badge variant="secondary">Processing</Badge>
            )}
            <EtaDisplay seconds={etaSeconds} />
          </div>
        </div>
      </CardHeader>
      <CardContent className="space-y-4">
        {/* Overall progress bar */}
        <div>
          <div className="flex justify-between text-sm mb-1">
            <span className="text-muted-foreground">Overall Progress</span>
            <span className="font-medium">{overallPercent}%</span>
          </div>
          <Progress
            value={overallPercent}
            className={cn(
              "h-2",
              isFailed && "bg-red-100 [&>div]:bg-red-500"
            )}
          />
        </div>

        {/* Phase timeline */}
        <div className="flex items-start justify-between">
          {phases.map((phase, index) => (
            <div key={phase.phase} className="flex items-center">
              <PhaseIndicator phase={phase} isLast={index === phases.length - 1} />
              {index < phases.length - 1 && (
                <PhaseConnector
                  completed={phase.status.type === "completed"}
                  active={phase.status.type === "active"}
                />
              )}
            </div>
          ))}
        </div>

        {/* Live progress message for active phase */}
        {isProcessing && (
          <p className="text-xs text-center text-muted-foreground min-h-[1rem]">
            {phases.find((p) => p.status.type === "active")?.message ?? "Processing..."}
          </p>
        )}

        {/* OODA-29: Enhanced error display using ErrorBanner */}
        {isFailed && progress?.error && (
          <ErrorBanner
            error={{
              code: extractErrorCode(progress.error),
              message: progress.error,
              phase: getFailedPhaseName(phases),
              recoverable: true,
            }}
            filename={displayFilename}
            onRetry={() => retry()}
            isRetrying={isRetrying}
            compact={true}
          />
        )}

        {/* Action buttons - only show Cancel when processing (Retry is in ErrorBanner) */}
        {isProcessing && (
          <div className="flex justify-end gap-2">
            <Button
              size="sm"
              variant="outline"
              onClick={() => cancel()}
              disabled={isCancelling}
            >
              {isCancelling ? (
                <>
                  <Loader2 className="h-4 w-4 mr-2 animate-spin" />
                  Cancelling...
                </>
              ) : (
                <>
                  <StopCircle className="h-4 w-4 mr-2" />
                  Cancel
                </>
              )}
            </Button>
          </div>
        )}
      </CardContent>
    </Card>
  );
}

export default PdfUploadProgress;
