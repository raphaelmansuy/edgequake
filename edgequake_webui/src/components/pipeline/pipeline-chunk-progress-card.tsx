"use client";

import { Badge } from "@/components/ui/badge";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Progress } from "@/components/ui/progress";
import { ScrollArea } from "@/components/ui/scroll-area";
import { useChunkProgress } from "@/hooks";
import { useCurrentTime } from "@/hooks/use-current-time";
import {
  formatDurationSeconds,
  formatPipelineCost,
  formatTokenCount,
} from "@/lib/pipeline/pipeline-formatters";
import {
  Brain,
  Cpu,
  DollarSign,
  FileText,
  Layers,
  Loader2,
  Timer,
  Zap,
} from "lucide-react";
import { useMemo } from "react";

export function PipelineChunkProgressCard() {
  const { chunkProgress, hasActiveProgress } = useChunkProgress();
  const now = useCurrentTime(1000);

  const activeProgress = useMemo(() => {
    return Array.from(chunkProgress.values())
      .filter((progress) => {
        const age = now - progress.lastUpdated.getTime();
        return age < 60_000;
      })
      .sort((a, b) => b.lastUpdated.getTime() - a.lastUpdated.getTime());
  }, [chunkProgress, now]);

  if (activeProgress.length === 0) {
    return null;
  }

  return (
    <Card>
      <CardHeader className="pb-2">
        <CardTitle className="text-lg flex items-center gap-2">
          <Layers className="h-5 w-5" />
          Chunk Progress
          {hasActiveProgress && (
            <Badge variant="outline" className="text-blue-500 border-blue-500 animate-pulse">
              <Loader2 className="h-3 w-3 mr-1 animate-spin" />
              Live
            </Badge>
          )}
        </CardTitle>
        <CardDescription>Real-time chunk-level extraction progress</CardDescription>
      </CardHeader>
      <CardContent>
        <ScrollArea className="h-64">
          <div className="space-y-4">
            {activeProgress.map((progress) => (
              <div
                key={progress.documentId}
                className="p-3 rounded-lg border bg-card space-y-3"
              >
                <div className="flex items-center justify-between">
                  <div className="flex items-center gap-2">
                    <FileText className="h-4 w-4 text-muted-foreground" />
                    <span className="text-sm font-medium truncate max-w-48">
                      {progress.documentId}
                    </span>
                  </div>
                  <Badge variant="secondary" className="text-xs">
                    {progress.percentComplete}%
                  </Badge>
                </div>

                <div className="space-y-1">
                  <div className="flex items-center justify-between text-xs text-muted-foreground">
                    <span className="flex items-center gap-1">
                      <Zap className="h-3 w-3" />
                      Chunk {progress.chunkIndex + 1} / {progress.totalChunks}
                    </span>
                    <span className="flex items-center gap-1">
                      <Timer className="h-3 w-3" />
                      ETA: {formatDurationSeconds(progress.etaSeconds)}
                    </span>
                  </div>
                  <Progress value={progress.percentComplete} className="h-2" />
                </div>

                {progress.chunkPreview && (
                  <div className="text-xs text-muted-foreground bg-muted/50 p-2 rounded">
                    <span className="text-foreground font-medium">Current: </span>
                    &quot;{progress.chunkPreview.slice(0, 80)}...&quot;
                  </div>
                )}

                <div className="grid grid-cols-3 gap-2 text-xs">
                  <div className="flex items-center gap-1 text-muted-foreground">
                    <Brain className="h-3 w-3" />
                    <span>In: {formatTokenCount(progress.tokensIn)}</span>
                  </div>
                  <div className="flex items-center gap-1 text-muted-foreground">
                    <Cpu className="h-3 w-3" />
                    <span>Out: {formatTokenCount(progress.tokensOut)}</span>
                  </div>
                  <div className="flex items-center gap-1 text-green-600">
                    <DollarSign className="h-3 w-3" />
                    <span>{formatPipelineCost(progress.costUsd)}</span>
                  </div>
                </div>
              </div>
            ))}
          </div>
        </ScrollArea>
      </CardContent>
    </Card>
  );
}
