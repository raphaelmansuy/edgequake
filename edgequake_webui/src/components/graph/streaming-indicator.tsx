"use client";

import { Badge } from "@/components/ui/badge";
import { Card } from "@/components/ui/card";
import { Progress } from "@/components/ui/progress";
import type { GraphStreamProgress } from "@/hooks/use-graph-stream";
import { cn } from "@/lib/utils";
import { CheckCircle2, Loader2, XCircle } from "lucide-react";

interface StreamingIndicatorProps {
  /** Current streaming progress */
  progress: GraphStreamProgress;
  /** Additional CSS classes */
  className?: string;
  /** Whether to show in compact mode */
  compact?: boolean;
}

/**
 * Visual indicator for graph streaming progress.
 * 
 * Shows current phase, progress bar, and statistics during streaming.
 * Automatically hides when streaming is idle or complete.
 */
export function StreamingIndicator({
  progress,
  className,
  compact = false,
}: StreamingIndicatorProps) {
  // Don't show when idle
  if (progress.phase === "idle") {
    return null;
  }

  // Calculate progress percentage
  const progressPercent =
    progress.totalNodes > 0
      ? Math.round((progress.nodesLoaded / progress.totalNodes) * 100)
      : 0;

  // Phase-specific icon and label
  const phaseInfo = getPhaseInfo(progress.phase);

  // Compact version for mobile
  if (compact) {
    return (
      <div
        className={cn(
          "flex items-center gap-2 px-3 py-1.5 rounded-full bg-background/95 backdrop-blur border shadow-sm",
          className
        )}
      >
        {phaseInfo.icon}
        <span className="text-xs font-medium">{progress.nodesLoaded}</span>
        <Progress value={progressPercent} className="w-16 h-1.5" />
      </div>
    );
  }

  // Full version
  return (
    <Card
      className={cn(
        "p-3 flex items-center gap-3 bg-background/95 backdrop-blur shadow-lg border",
        progress.phase === "error" && "border-destructive",
        progress.phase === "complete" && "border-green-500",
        className
      )}
    >
      {/* Phase icon */}
      {phaseInfo.icon}

      {/* Content */}
      <div className="flex-1 min-w-0">
        <div className="flex items-center gap-2">
          <span className="text-sm font-medium">{phaseInfo.label}</span>
          {progress.phase === "nodes" && (
            <Badge variant="secondary" className="text-xs">
              Batch {progress.batchNumber}/{progress.totalBatches}
            </Badge>
          )}
        </div>
        
        {/* Progress details */}
        <div className="flex items-center gap-2 mt-1">
          {progress.phase !== "connecting" && progress.phase !== "error" && (
            <>
              <span className="text-xs text-muted-foreground">
                {progress.nodesLoaded} / {progress.totalNodes} nodes
              </span>
              {progress.edgesLoaded > 0 && (
                <>
                  <span className="text-xs text-muted-foreground">•</span>
                  <span className="text-xs text-muted-foreground">
                    {progress.edgesLoaded} edges
                  </span>
                </>
              )}
              {progress.durationMs > 0 && (
                <>
                  <span className="text-xs text-muted-foreground">•</span>
                  <span className="text-xs text-muted-foreground">
                    {(progress.durationMs / 1000).toFixed(1)}s
                  </span>
                </>
              )}
            </>
          )}
          {progress.phase === "error" && (
            <span className="text-xs text-destructive">
              {progress.errorMessage || "Stream failed"}
            </span>
          )}
        </div>
      </div>

      {/* Progress bar */}
      {progress.phase !== "error" && progress.phase !== "complete" && (
        <Progress value={progressPercent} className="w-24 h-2" />
      )}
    </Card>
  );
}

function getPhaseInfo(phase: GraphStreamProgress["phase"]) {
  switch (phase) {
    case "connecting":
      return {
        icon: <Loader2 className="h-4 w-4 animate-spin text-muted-foreground" />,
        label: "Connecting...",
      };
    case "metadata":
      return {
        icon: <Loader2 className="h-4 w-4 animate-spin text-blue-500" />,
        label: "Preparing graph...",
      };
    case "nodes":
      return {
        icon: <Loader2 className="h-4 w-4 animate-spin text-primary" />,
        label: "Loading nodes...",
      };
    case "edges":
      return {
        icon: <Loader2 className="h-4 w-4 animate-spin text-primary" />,
        label: "Loading edges...",
      };
    case "complete":
      return {
        icon: <CheckCircle2 className="h-4 w-4 text-green-500" />,
        label: "Complete",
      };
    case "error":
      return {
        icon: <XCircle className="h-4 w-4 text-destructive" />,
        label: "Error",
      };
    default:
      return {
        icon: null,
        label: "",
      };
  }
}

/**
 * Minimal streaming indicator that just shows a loading bar at the top of the container.
 */
export function StreamingProgressBar({
  progress,
  className,
}: {
  progress: GraphStreamProgress;
  className?: string;
}) {
  if (progress.phase === "idle" || progress.phase === "complete") {
    return null;
  }

  const progressPercent =
    progress.totalNodes > 0
      ? Math.round((progress.nodesLoaded / progress.totalNodes) * 100)
      : 0;

  return (
    <div className={cn("w-full", className)}>
      <Progress
        value={progress.phase === "error" ? 100 : progressPercent}
        className={cn(
          "h-1 rounded-none",
          progress.phase === "error" && "[&>div]:bg-destructive"
        )}
      />
    </div>
  );
}

export default StreamingIndicator;
