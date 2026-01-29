/**
 * @module FailedChunksCard
 * @description Displays failed chunks during document processing with error details.
 *
 * @implements SPEC-003: Chunk-level resilience with failure visibility
 * @implements UC2305: System continues processing when chunks fail
 *
 * WHY: When using process_with_resilience, some chunks may fail while
 * others succeed. This component shows which chunks failed and why,
 * enabling users to understand partial failures and potentially retry.
 */

'use client';

import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import {
    Card,
    CardContent,
    CardDescription,
    CardHeader,
    CardTitle,
} from '@/components/ui/card';
import {
    Collapsible,
    CollapsibleContent,
    CollapsibleTrigger,
} from '@/components/ui/collapsible';
import {
    Tooltip,
    TooltipContent,
    TooltipProvider,
    TooltipTrigger,
} from '@/components/ui/tooltip';
import type { ChunkFailureInfo } from '@/hooks/use-chunk-progress';
import { cn } from '@/lib/utils';
import {
    AlertTriangle,
    ChevronDown,
    ChevronUp,
    Clock,
    RefreshCw,
    XCircle,
} from 'lucide-react';
import { useState } from 'react';

interface FailedChunksCardProps {
  /** Document ID */
  documentId: string;
  /** List of failed chunks */
  failedChunks: ChunkFailureInfo[];
  /** Total chunks in document */
  totalChunks: number;
  /** Successful chunk count */
  successfulChunks: number;
  /** Callback when retry is requested */
  onRetry?: (chunkIndices: number[]) => void;
  /** Whether retry is in progress */
  isRetrying?: boolean;
  /** Custom class name */
  className?: string;
}

/**
 * Displays a summary of failed chunks with expandable error details.
 *
 * Features:
 * - Summary badge showing failure count
 * - Expandable list of individual failures
 * - Timeout vs error type indicators
 * - Retry button (when onRetry provided)
 */
export function FailedChunksCard({
  documentId,
  failedChunks,
  totalChunks,
  successfulChunks,
  onRetry,
  isRetrying = false,
  className,
}: FailedChunksCardProps) {
  const [isOpen, setIsOpen] = useState(false);

  if (failedChunks.length === 0) {
    return null;
  }

  const failureRate = ((failedChunks.length / totalChunks) * 100).toFixed(1);
  const successRate = ((successfulChunks / totalChunks) * 100).toFixed(1);
  const timeoutCount = failedChunks.filter((f) => f.wasTimeout).length;
  const errorCount = failedChunks.length - timeoutCount;

  const handleRetryAll = () => {
    if (onRetry) {
      onRetry(failedChunks.map((f) => f.chunkIndex));
    }
  };

  const handleRetryOne = (chunkIndex: number) => {
    if (onRetry) {
      onRetry([chunkIndex]);
    }
  };

  return (
    <Card
      className={cn('border-yellow-500/50 bg-yellow-50/50 dark:bg-yellow-950/10', className)}
    >
      <CardHeader className="pb-3">
        <div className="flex items-center justify-between">
          <div className="flex items-center gap-2">
            <AlertTriangle className="h-5 w-5 text-yellow-600" />
            <CardTitle className="text-base">Partial Processing</CardTitle>
          </div>
          <div className="flex items-center gap-2">
            <Badge variant="secondary" className="bg-green-100 text-green-700">
              {successfulChunks}/{totalChunks} OK ({successRate}%)
            </Badge>
            <Badge variant="destructive">
              {failedChunks.length} Failed ({failureRate}%)
            </Badge>
          </div>
        </div>
        <CardDescription>
          Some chunks failed during extraction but the document was partially processed.
        </CardDescription>
      </CardHeader>

      <CardContent className="pt-0">
        <Collapsible open={isOpen} onOpenChange={setIsOpen}>
          <div className="flex items-center justify-between">
            <CollapsibleTrigger asChild>
              <Button variant="ghost" size="sm" className="gap-1 p-0 h-auto">
                {isOpen ? (
                  <>
                    <ChevronUp className="h-4 w-4" />
                    Hide details
                  </>
                ) : (
                  <>
                    <ChevronDown className="h-4 w-4" />
                    Show {failedChunks.length} failed chunk
                    {failedChunks.length > 1 ? 's' : ''}
                  </>
                )}
              </Button>
            </CollapsibleTrigger>

            {onRetry && (
              <Button
                variant="outline"
                size="sm"
                onClick={handleRetryAll}
                disabled={isRetrying}
                className="gap-1"
              >
                {isRetrying ? (
                  <>
                    <RefreshCw className="h-3 w-3 animate-spin" />
                    Retrying...
                  </>
                ) : (
                  <>
                    <RefreshCw className="h-3 w-3" />
                    Retry All
                  </>
                )}
              </Button>
            )}
          </div>

          <CollapsibleContent className="mt-3">
            <div className="space-y-2">
              {/* Summary of failure types */}
              <div className="flex gap-2 text-sm text-muted-foreground mb-3">
                {timeoutCount > 0 && (
                  <div className="flex items-center gap-1">
                    <Clock className="h-3 w-3" />
                    {timeoutCount} timeout{timeoutCount > 1 ? 's' : ''}
                  </div>
                )}
                {errorCount > 0 && (
                  <div className="flex items-center gap-1">
                    <XCircle className="h-3 w-3" />
                    {errorCount} error{errorCount > 1 ? 's' : ''}
                  </div>
                )}
              </div>

              {/* Individual failures */}
              <div className="space-y-2 max-h-60 overflow-y-auto">
                {failedChunks.map((failure) => (
                  <FailedChunkItem
                    key={failure.chunkIndex}
                    failure={failure}
                    onRetry={onRetry ? () => handleRetryOne(failure.chunkIndex) : undefined}
                    isRetrying={isRetrying}
                  />
                ))}
              </div>
            </div>
          </CollapsibleContent>
        </Collapsible>
      </CardContent>
    </Card>
  );
}

interface FailedChunkItemProps {
  failure: ChunkFailureInfo;
  onRetry?: () => void;
  isRetrying?: boolean;
}

function FailedChunkItem({ failure, onRetry, isRetrying }: FailedChunkItemProps) {
  return (
    <div className="flex items-start justify-between gap-2 p-2 rounded-md bg-background border">
      <div className="flex-1 min-w-0">
        <div className="flex items-center gap-2 mb-1">
          <Badge variant="outline" className="font-mono text-xs">
            Chunk {failure.chunkIndex + 1}
          </Badge>
          {failure.wasTimeout ? (
            <TooltipProvider>
              <Tooltip>
                <TooltipTrigger>
                  <Badge variant="secondary" className="bg-orange-100 text-orange-700 text-xs">
                    <Clock className="h-3 w-3 mr-1" />
                    Timeout
                  </Badge>
                </TooltipTrigger>
                <TooltipContent>
                  Request timed out after {failure.retryAttempts} retry attempts
                </TooltipContent>
              </Tooltip>
            </TooltipProvider>
          ) : (
            <Badge variant="secondary" className="bg-red-100 text-red-700 text-xs">
              <XCircle className="h-3 w-3 mr-1" />
              Error
            </Badge>
          )}
          <span className="text-xs text-muted-foreground">
            {failure.retryAttempts} retries
          </span>
        </div>
        <p className="text-xs text-muted-foreground truncate" title={failure.errorMessage}>
          {failure.errorMessage}
        </p>
      </div>
      {onRetry && (
        <Button
          variant="ghost"
          size="icon"
          className="h-6 w-6 shrink-0"
          onClick={onRetry}
          disabled={isRetrying}
        >
          <RefreshCw className={cn('h-3 w-3', isRetrying && 'animate-spin')} />
        </Button>
      )}
    </div>
  );
}

export default FailedChunksCard;
