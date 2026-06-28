/**
 * @module DocumentPreviewPanel
 * @description Side panel for quick document preview and actions.
 * Shows status, content preview, cost breakdown, and quick actions.
 * 
 * @implements UC0011 - User previews document in panel
 * @implements FEAT0633 - Content preview with line numbers
 * @implements FEAT0634 - Cost breakdown visualization
 * @implements FEAT0635 - Quick action buttons
 * 
 * @enforces BR0622 - Preview loads without full content fetch
 * @enforces BR0305 - Cost displayed per document
 * 
 * @see {@link docs/features.md} FEAT0633-0635
 */
'use client';

import { StreamingMarkdownRenderer } from '@/components/query/markdown';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { Card, CardContent } from '@/components/ui/card';
import { Separator } from '@/components/ui/separator';
import { Skeleton } from '@/components/ui/skeleton';
import {
    Tooltip,
    TooltipContent,
    TooltipProvider,
    TooltipTrigger,
} from '@/components/ui/tooltip';
import { getDocument } from '@/lib/api/edgequake';
import { categorizeError, getCategoryColor, type ErrorCategory } from '@/lib/error-categories';
import { getEffectiveErrorMessage } from '@/lib/utils/document-status';
import type { Document } from '@/types';
import { useQuery } from '@tanstack/react-query';
import { formatDistanceToNow } from 'date-fns';
import {
    AlertCircle,
    Brain,
    Calendar,
    CheckCircle,
    ChevronDown,
    ChevronUp,
    Clock,
    Copy,
    Cpu,
    Database,
    ExternalLink,
    Eye,
    FileText,
    FileWarning,
    HardDrive,
    Loader2,
    Network,
    RefreshCw,
    StopCircle,
    Trash2,
    Wifi,
    XCircle,
    Zap
} from 'lucide-react';
import { useCallback, useMemo, useRef, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { toast } from 'sonner';

const statusConfig = {
  pending: { icon: Clock, color: 'text-yellow-500', bg: 'bg-yellow-500/10', label: 'Pending' },
  processing: { icon: Loader2, color: 'text-blue-500', bg: 'bg-blue-500/10', label: 'Processing' },
  completed: { icon: CheckCircle, color: 'text-green-500', bg: 'bg-green-500/10', label: 'Completed' },
  indexed: { icon: CheckCircle, color: 'text-green-500', bg: 'bg-green-500/10', label: 'Indexed' },
  failed: { icon: XCircle, color: 'text-red-500', bg: 'bg-red-500/10', label: 'Failed' },
  partial_failure: { icon: XCircle, color: 'text-orange-500', bg: 'bg-orange-500/10', label: 'Partial Failure' },
  cancelled: { icon: StopCircle, color: 'text-gray-500', bg: 'bg-gray-500/10', label: 'Cancelled' },
} as const;

type DocumentStatus = keyof typeof statusConfig;

/**
 * OODA-21: Get icon component for error category
 */
function getCategoryIconComponent(category: ErrorCategory) {
  switch (category) {
    case 'llm': return Brain;
    case 'embedding': return Cpu;
    case 'storage': return Database;
    case 'pipeline': return FileWarning;
    case 'network': return Wifi;
    default: return AlertCircle;
  }
}

interface DocumentPreviewPanelProps {
  /** The document to preview */
  document: Document | null;
  /** Called when the document should be deleted */
  onDelete?: (documentId: string) => void;
  /** Called when the document should be reprocessed */
  onReprocess?: (documentId: string) => void;
  /** Called when user wants to view full document */
  onViewFull?: (document: Document) => void;
  /** Called when user wants to view in graph */
  onViewInGraph?: (document: Document) => void;
  /** Whether delete action is loading */
  isDeleting?: boolean;
  /** Whether reprocess action is loading */
  isReprocessing?: boolean;
}

function formatFileSize(bytes: number | undefined): string {
  if (!bytes) return 'Unknown';
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  return `${(bytes / (1024 * 1024)).toFixed(2)} MB`;
}

function formatCost(cost: number | undefined): string {
  if (cost === undefined || cost === null) return '-';
  if (cost === 0) return 'Free';
  if (cost < 0.0001) return '< $0.0001';
  if (cost < 0.01) return `$${cost.toFixed(4)}`;
  return `$${cost.toFixed(2)}`;
}

function formatTokens(tokens: number | undefined): string {
  if (tokens === undefined || tokens === null) return '-';
  if (tokens >= 1_000_000) return `${(tokens / 1_000_000).toFixed(1)}M`;
  if (tokens >= 1_000) return `${(tokens / 1_000).toFixed(1)}K`;
  return tokens.toLocaleString();
}

function getCostColor(cost: number | undefined): string {
  if (cost === undefined || cost === null || cost === 0) return 'text-muted-foreground';
  if (cost < 0.001) return 'text-green-500';
  if (cost < 0.01) return 'text-blue-500';
  if (cost < 0.1) return 'text-yellow-500';
  return 'text-orange-500';
}

export function DocumentPreviewPanel({
  document,
  onDelete,
  onReprocess,
  onViewFull,
  onViewInGraph,
  isDeleting = false,
  isReprocessing = false,
}: DocumentPreviewPanelProps) {
  const { t } = useTranslation();
  const [showFullContent, setShowFullContent] = useState(false);
  // Ref to scroll the content card back into view when collapsing (Show Less)
  const contentCardRef = useRef<HTMLDivElement>(null);

  // Fetch full document for content preview
  const { data: fullDocument, isLoading: isLoadingContent } = useQuery({
    queryKey: ['document', document?.id],
    queryFn: () => (document ? getDocument(document.id) : Promise.resolve(null)),
    enabled: !!document?.id,
    staleTime: 5 * 60 * 1000, // 5 minutes
  });

  /**
   * OODA-21: Error categorization for better error display
   * WHY: Categorized errors with suggestions help users understand and resolve issues
   */
  const errorInfo = useMemo(() => {
    if (!document) return null;
    const errorMessage = getEffectiveErrorMessage(document);
    if (!errorMessage) return null;
    return categorizeError(errorMessage);
  }, [document]);

  const handleCopyId = useCallback(async () => {
    if (!document) return;
    try {
      await navigator.clipboard.writeText(document.id);
      toast.success(t('documents.preview.idCopied', 'Document ID copied to clipboard'));
    } catch {
      toast.error(t('common.copyFailed', 'Failed to copy'));
    }
  }, [document, t]);

  const handleCopyContent = useCallback(async () => {
    const content = fullDocument?.content || document?.content_summary;
    if (!content) return;
    try {
      await navigator.clipboard.writeText(content);
      toast.success(t('documents.preview.contentCopied', 'Content copied to clipboard'));
    } catch {
      toast.error(t('common.copyFailed', 'Failed to copy'));
    }
  }, [fullDocument, document, t]);

  if (!document) {
    return (
      <div
        className="flex flex-col items-center justify-center h-full text-center p-6"
        role="status"
        aria-label={t('documents.preview.noSelection', 'No Document Selected')}
      >
        <div className="rounded-full bg-muted p-4 mb-4" aria-hidden="true">
          <FileText className="h-8 w-8 text-muted-foreground" />
        </div>
        <h3 className="font-medium mb-2">{t('documents.preview.noSelection', 'No Document Selected')}</h3>
        <p className="text-sm text-muted-foreground max-w-[200px]">
          {t('documents.preview.selectHint', 'Select a document from the list to preview its details')}
        </p>
      </div>
    );
  }

  const status = (document.status || 'completed') as DocumentStatus;
  const statusInfo = statusConfig[status] || statusConfig.completed;
  const StatusIcon = statusInfo.icon;
  const isProcessing = status === 'processing';
  const isFailed = status === 'failed' || status === 'partial_failure';
  const isCancelled = status === 'cancelled';

  const contentPreview = fullDocument?.content || document?.content_summary || '';
  const previewLength = 500;
  const hasMoreContent = contentPreview.length > previewLength;
  const displayContent = showFullContent ? contentPreview : contentPreview.slice(0, previewLength);

  return (
    <article
      className="space-y-4"
      aria-label={document.title || document.file_name || t('documents.preview.title', 'Document Preview')}
    >
      {/* RP-05: Document title shown once, wraps instead of truncating.
         RP-02: Full title readable, entities + time on a compact meta line. */}
      <div className="space-y-1.5">
        <h3 className="font-semibold text-sm leading-snug break-words">
          {document.title || document.file_name || `Document ${document.id.slice(0, 8)}`}
        </h3>
        <div className="flex items-center flex-wrap gap-2">
          <Badge
            variant="outline"
            className={`gap-1 text-xs ${statusInfo.color}`}
          >
            <StatusIcon className={`h-3 w-3 ${isProcessing ? 'animate-spin' : ''}`} />
            {statusInfo.label}
          </Badge>
          {document.entity_count !== undefined && document.entity_count > 0 && (
            <span className="text-xs text-muted-foreground">
              {document.entity_count.toLocaleString()} entities
            </span>
          )}
          {document.created_at && (
            <span className="text-xs text-muted-foreground">
              {formatDistanceToNow(new Date(document.created_at), { addSuffix: true })}
            </span>
          )}
        </div>
      </div>

      {/* Details section — RP-04: lowercase label */}
      <div className="space-y-2">
        <p className="text-[11px] font-semibold uppercase tracking-wider text-muted-foreground/70">
          {t('documents.preview.metadata', 'Details')}
        </p>
        <div className="grid gap-2">
          {/* ID */}
          <div className="flex items-center justify-between gap-3">
            <span className="text-xs text-muted-foreground flex items-center gap-1 shrink-0">
              <FileText className="h-3 w-3" />
              ID
            </span>
            <div className="flex items-center gap-1 min-w-0">
              <code className="text-xs bg-muted px-1.5 py-0.5 rounded font-mono truncate max-w-[120px]" title={document.id}>
                {document.id.slice(0, 12)}...
              </code>
              <TooltipProvider>
                <Tooltip>
                  <TooltipTrigger asChild>
                    <Button variant="ghost" size="icon" className="h-5 w-5 shrink-0" onClick={handleCopyId}>
                      <Copy className="h-3 w-3" />
                    </Button>
                  </TooltipTrigger>
                  <TooltipContent>{t('common.copy', 'Copy ID')}</TooltipContent>
                </Tooltip>
              </TooltipProvider>
            </div>
          </div>

          {/* Size */}
          {(document.file_size || document.content_length) && (
            <div className="flex items-center justify-between gap-3">
              <span className="text-xs text-muted-foreground flex items-center gap-1 shrink-0">
                <HardDrive className="h-3 w-3" />
                {t('documents.preview.size', 'Size')}
              </span>
              <span className="text-xs font-medium tabular-nums">{formatFileSize(document.file_size || document.content_length)}</span>
            </div>
          )}

          {/* Created */}
          {document.created_at && (
            <div className="flex items-center justify-between gap-3">
              <span className="text-xs text-muted-foreground flex items-center gap-1 shrink-0">
                <Calendar className="h-3 w-3" />
                {t('documents.preview.created', 'Created')}
              </span>
              <TooltipProvider>
                <Tooltip>
                  <TooltipTrigger asChild>
                    <span className="text-xs font-medium cursor-help tabular-nums">
                      {formatDistanceToNow(new Date(document.created_at), { addSuffix: true })}
                    </span>
                  </TooltipTrigger>
                  <TooltipContent>
                    {new Date(document.created_at).toLocaleString()}
                  </TooltipContent>
                </Tooltip>
              </TooltipProvider>
            </div>
          )}

          {/* OODA-46: Updated timestamp */}
          {document.updated_at && document.updated_at !== document.created_at && (
            <div className="flex items-center justify-between gap-3">
              <span className="text-xs text-muted-foreground flex items-center gap-1 shrink-0">
                <Clock className="h-3 w-3" />
                {t('documents.preview.updated', 'Updated')}
              </span>
              <TooltipProvider>
                <Tooltip>
                  <TooltipTrigger asChild>
                    <span className="text-xs font-medium cursor-help tabular-nums">
                      {formatDistanceToNow(new Date(document.updated_at), { addSuffix: true })}
                    </span>
                  </TooltipTrigger>
                  <TooltipContent>
                    {new Date(document.updated_at).toLocaleString()}
                  </TooltipContent>
                </Tooltip>
              </TooltipProvider>
            </div>
          )}

          {/* Entities — hidden from Details since it's shown in the meta line above */}

          {/* OODA-33: File Size Display — remove duplicate (already shown in Size row) */}
        </div>
      </div>

      {/* Cost Information */}
      {(document.cost_usd !== undefined || document.total_tokens !== undefined) && (
        <div className="space-y-2">
          <p className="text-[11px] font-semibold uppercase tracking-wider text-muted-foreground/70">
            {t('documents.preview.processingCost', 'Cost')}
          </p>
            
            <Card className="bg-muted/30 border-none">
              <CardContent className="p-3 space-y-2">
                {/* Total Cost */}
                <div className="flex items-center justify-between">
                  <span className="text-sm text-muted-foreground">{t('documents.preview.totalCost', 'Total Cost')}</span>
                  <span className={`text-sm font-semibold ${getCostColor(document.cost_usd)}`}>
                    {formatCost(document.cost_usd)}
                  </span>
                </div>

                {/* Tokens */}
                {document.total_tokens !== undefined && (
                  <div className="flex items-center justify-between">
                    <span className="text-sm text-muted-foreground flex items-center gap-1">
                      <Zap className="h-3 w-3" aria-hidden="true" />
                      {t('documents.preview.totalTokens', 'Total Tokens')}
                    </span>
                    <span className="text-sm font-medium">
                      {formatTokens(document.total_tokens)}
                    </span>
                  </div>
                )}

                {/* Token Breakdown */}
                {(document.input_tokens !== undefined || document.output_tokens !== undefined) && (
                  <div className="pt-1 border-t border-border/50 space-y-1">
                    {document.input_tokens !== undefined && (
                      <div className="flex items-center justify-between text-xs">
                        <span className="text-muted-foreground">{t('documents.preview.inputTokens', 'Input Tokens')}</span>
                        <span className="font-mono">{formatTokens(document.input_tokens)}</span>
                      </div>
                    )}
                    {document.output_tokens !== undefined && (
                      <div className="flex items-center justify-between text-xs">
                        <span className="text-muted-foreground">{t('documents.preview.outputTokens', 'Output Tokens')}</span>
                        <span className="font-mono">{formatTokens(document.output_tokens)}</span>
                      </div>
                    )}
                  </div>
                )}

                {/* Model Info — LLM, Embedding */}
                {(document.llm_model || document.embedding_model) && (
                  <div className="pt-1 border-t border-border/50 space-y-1">
                    {document.llm_model && (
                      <div className="flex items-center justify-between text-xs">
                        <span className="text-muted-foreground">{t('documents.preview.llmModel', 'LLM Model')}</span>
                        <code className="bg-muted px-1.5 py-0.5 rounded text-[10px] max-w-[60%] truncate" title={document.llm_model}>
                          {document.llm_model}
                        </code>
                      </div>
                    )}
                    {document.embedding_model && (
                      <div className="flex items-center justify-between text-xs">
                        <span className="text-muted-foreground">{t('documents.preview.embedding', 'Embedding')}</span>
                        <code className="bg-muted px-1.5 py-0.5 rounded text-[10px] max-w-[60%] truncate" title={document.embedding_model}>
                          {document.embedding_model}
                        </code>
                      </div>
                    )}
                  </div>
                )}

                {/* PDF extraction lineage */}
                {(document.lineage?.pdf_vision_model || document.lineage?.pdf_extraction_method) && (
                  <div className="pt-1 border-t border-border/50 space-y-1">
                    {document.lineage?.pdf_vision_model && (
                      <div className="flex items-center justify-between text-xs">
                        <span className="text-muted-foreground flex items-center gap-1">
                          <Eye className="h-3 w-3" aria-hidden="true" />
                          {t('documents.preview.visionModel', 'Vision Model')}
                        </span>
                        <code
                          className="bg-violet-500/10 text-violet-700 dark:text-violet-300 px-1.5 py-0.5 rounded text-[10px] max-w-[60%] truncate"
                          title={document.lineage.pdf_vision_model}
                        >
                          {document.lineage.pdf_vision_model}
                        </code>
                      </div>
                    )}
                    {document.lineage?.pdf_extraction_method && (
                      <div className="flex items-center justify-between text-xs">
                        <span className="text-muted-foreground">
                          {t('documents.preview.extractionMethod', 'Extraction Method')}
                        </span>
                        <Badge variant="outline" className="text-[10px] h-4 capitalize">
                          {document.lineage.pdf_extraction_method === 'edgeparse'
                            ? 'EdgeParse'
                            : document.lineage.pdf_extraction_method}
                        </Badge>
                      </div>
                    )}
                    {document.lineage?.pdf_extraction_warning && (
                      <div className="rounded-md border border-amber-500/30 bg-amber-500/10 px-2 py-1.5 text-[11px] text-amber-800 dark:text-amber-200">
                        {document.lineage.pdf_extraction_warning}
                      </div>
                    )}
                  </div>
                )}
              </CardContent>
            </Card>
        </div>
      )}

      <Separator />

      {/* Content Preview */}
      <div className="space-y-2">
        <div className="flex items-center justify-between">
          <p className="text-[11px] font-semibold uppercase tracking-wider text-muted-foreground/70">
            {t('documents.preview.content', 'Content')}
          </p>
          {(fullDocument?.content || document?.content_summary) && (
            <Button variant="ghost" size="sm" className="h-7 text-xs" onClick={handleCopyContent}>
              <Copy className="h-3 w-3 mr-1" />
              {t('common.copy', 'Copy')}
            </Button>
          )}
        </div>

        <Card className="bg-muted/30" ref={contentCardRef}>
          <CardContent className="p-3">
            {isLoadingContent ? (
              <div className="space-y-2">
                <Skeleton className="h-4 w-full" />
                <Skeleton className="h-4 w-3/4" />
                <Skeleton className="h-4 w-1/2" />
              </div>
            ) : (fullDocument?.content || document?.content_summary) ? (
              <div className="space-y-2">
                {/* WHY: StreamingMarkdownRenderer handles HTML (<sup>, <sub>),
                    LaTeX ($...$, $$...$$), GFM tables, code blocks, and more.
                    The collapsed view limits height with a fade-out overlay.
                    No internal pre/scroll — the outer RightPanel ScrollArea handles overflow. */}
                <div
                  className="relative text-xs overflow-hidden"
                  style={!showFullContent ? { maxHeight: '200px' } : undefined}
                >
                  <div className="[&_p]:text-xs [&_h1]:text-sm [&_h2]:text-sm [&_h3]:text-xs [&_li]:text-xs [&_td]:text-xs [&_th]:text-xs prose-spacing-tight">
                    <StreamingMarkdownRenderer content={displayContent} />
                  </div>
                  {/* Fade overlay when collapsed */}
                  {!showFullContent && hasMoreContent && (
                    <div className="absolute bottom-0 left-0 right-0 h-10 bg-linear-to-t from-card/90 to-transparent pointer-events-none" />
                  )}
                </div>
                {hasMoreContent && (
                  <Button
                    variant="ghost"
                    size="sm"
                    className="w-full h-7 text-xs"
                    onClick={() => {
                      const next = !showFullContent;
                      setShowFullContent(next);
                      // Scroll the card into view when collapsing so layout appears clean
                      if (!next) {
                        requestAnimationFrame(() => {
                          contentCardRef.current?.scrollIntoView({
                            behavior: 'smooth',
                            block: 'nearest',
                          });
                        });
                      }
                    }}
                  >
                    {showFullContent ? (
                      <>
                        <ChevronUp className="h-3 w-3 mr-1" />
                        {t('documents.preview.showLess', 'Show Less')}
                      </>
                    ) : (
                      <>
                        <ChevronDown className="h-3 w-3 mr-1" />
                        {t('documents.preview.showMore', 'Show More')} ({contentPreview.length - previewLength} more chars)
                      </>
                    )}
                  </Button>
                )}
              </div>
            ) : (
              <p className="text-xs text-muted-foreground italic">
                {t('documents.preview.noContent', 'No content available')}
              </p>
            )}
          </CardContent>
        </Card>
      </div>

      {/* Error Info - OODA-21: Enhanced with categorization */}
      {/* WHY: Show error details for failed, partial_failure, and cancelled documents */}
      {(isFailed || isCancelled) && errorInfo && (
        (() => {
          const CategoryIcon = getCategoryIconComponent(errorInfo.category);
          const categoryColors = getCategoryColor(errorInfo.category);
          return (
            <>
              <Separator />
              <div className="space-y-3">
                <div className="flex items-center justify-between">
                  <h4 className={`text-sm font-medium flex items-center gap-1.5 ${categoryColors.text}`}>
                    <CategoryIcon className="h-4 w-4" />
                    {errorInfo.categoryLabel}
                  </h4>
                  {errorInfo.isTransient && (
                    <Badge variant="outline" className="text-[10px] text-green-600 border-green-200">
                      {t('documents.preview.retryable', 'Retryable')}
                    </Badge>
                  )}
                </div>
                
                <Card className={`${categoryColors.bg} ${categoryColors.border} border`}>
                  <CardContent className="p-3 space-y-2">
                    {/* Summary */}
                    <p className={`text-sm font-medium ${categoryColors.text}`}>
                      {errorInfo.summary}
                    </p>
                    
                    {/* Suggestion */}
                    <p className="text-xs text-muted-foreground">
                      💡 {errorInfo.suggestion}
                    </p>
                    
                    {/* Technical Details (collapsed) */}
                    <details className="text-xs">
                      <summary className="cursor-pointer text-muted-foreground hover:text-foreground select-none">
                        {t('documents.preview.technicalDetails', 'Technical details')}
                      </summary>
                      <code className="block mt-1 p-2 bg-muted/50 rounded text-[10px] break-all">
                        {errorInfo.originalMessage}
                      </code>
                    </details>
                  </CardContent>
                </Card>
                
                {/* Retry button for transient errors */}
                {errorInfo.isTransient && onReprocess && (
                  <Button
                    variant="outline"
                    size="sm"
                    className="w-full"
                    onClick={() => document?.id && onReprocess(document.id)}
                    disabled={isReprocessing}
                  >
                    {isReprocessing ? (
                      <Loader2 className="h-3.5 w-3.5 mr-1.5 animate-spin" />
                    ) : (
                      <RefreshCw className="h-3.5 w-3.5 mr-1.5" />
                    )}
                    {t('documents.actions.retryNow', 'Retry Now')}
                  </Button>
                )}
              </div>
            </>
          );
        })()
      )}

      {/* Cancelled info banner - shows when cancelled without error details */}
      {isCancelled && !errorInfo && (
        <>
          <Separator />
          <Card className="bg-gray-500/10 border-gray-200 border">
            <CardContent className="p-3 space-y-2">
              <div className="flex items-center gap-2">
                <StopCircle className="h-4 w-4 text-gray-500" />
                <p className="text-sm font-medium text-gray-700 dark:text-gray-300">
                  {t('documents.cancelled.title', 'Processing was cancelled')}
                </p>
              </div>
              <p className="text-xs text-muted-foreground">
                {t('documents.cancelled.hint', 'You can reprocess this document to resume extraction.')}
              </p>
              {onReprocess && (
                <Button
                  variant="outline"
                  size="sm"
                  className="w-full"
                  onClick={() => onReprocess(document.id)}
                  disabled={isReprocessing}
                >
                  {isReprocessing ? (
                    <Loader2 className="h-3.5 w-3.5 mr-1.5 animate-spin" />
                  ) : (
                    <RefreshCw className="h-3.5 w-3.5 mr-1.5" />
                  )}
                  {t('documents.actions.retryNow', 'Retry Now')}
                </Button>
              )}
            </CardContent>
          </Card>
        </>
      )}

      <Separator />

      {/* Actions */}
      <div className="space-y-2">
        <h4 className="text-[11px] font-semibold uppercase tracking-wider text-muted-foreground/70">
          {t('documents.preview.actions', 'Actions')}
        </h4>
        
        <div className="grid grid-cols-2 gap-2">
          {onViewFull && (
            <Button
              variant="outline"
              size="sm"
              className="h-9"
              onClick={() => onViewFull(document)}
            >
              <Eye className="h-3.5 w-3.5 mr-1.5" />
              {t('documents.actions.view', 'View')}
            </Button>
          )}
          
          {onViewInGraph && (
            <Button
              variant="outline"
              size="sm"
              className="h-9"
              onClick={() => onViewInGraph(document)}
            >
              <Network className="h-3.5 w-3.5 mr-1.5" />
              {t('documents.actions.graph', 'Graph')}
            </Button>
          )}
          
          {onReprocess && (
            <Button
              variant="outline"
              size="sm"
              className="h-9"
              onClick={() => onReprocess(document.id)}
              disabled={isReprocessing || isProcessing}
            >
              {isReprocessing ? (
                <Loader2 className="h-3.5 w-3.5 mr-1.5 animate-spin" />
              ) : (
                <RefreshCw className="h-3.5 w-3.5 mr-1.5" />
              )}
              {t('documents.actions.reprocess', 'Reprocess')}
            </Button>
          )}
          
          {onDelete && (
            <Button
              variant="outline"
              size="sm"
              className="h-9 text-destructive hover:text-destructive hover:bg-destructive/10"
              onClick={() => onDelete(document.id)}
              disabled={isDeleting}
            >
              {isDeleting ? (
                <Loader2 className="h-3.5 w-3.5 mr-1.5 animate-spin" />
              ) : (
                <Trash2 className="h-3.5 w-3.5 mr-1.5" />
              )}
              {t('documents.actions.delete', 'Delete')}
            </Button>
          )}
        </div>
        
        <Button
          variant="ghost"
          size="sm"
          className="w-full h-8 text-xs"
          onClick={() => window.open(`/documents/${document.id}`, '_blank')}
          aria-label={t('documents.actions.openInNewTab', 'Open in New Tab')}
        >
          <ExternalLink className="h-3 w-3 mr-1.5" aria-hidden="true" />
          {t('documents.actions.openInNewTab', 'Open in New Tab')}
        </Button>
      </div>
    </article>
  );
}

export default DocumentPreviewPanel;
