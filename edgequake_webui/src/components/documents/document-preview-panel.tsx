'use client';

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
import type { Document } from '@/types';
import { useQuery } from '@tanstack/react-query';
import { formatDistanceToNow } from 'date-fns';
import {
    AlertCircle,
    Calendar,
    CheckCircle,
    ChevronDown,
    ChevronUp,
    Clock,
    Copy,
    ExternalLink,
    Eye,
    FileText,
    HardDrive,
    Loader2,
    Network,
    RefreshCw,
    Trash2,
    XCircle,
} from 'lucide-react';
import { useCallback, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { toast } from 'sonner';

const statusConfig = {
  pending: { icon: Clock, color: 'text-yellow-500', bg: 'bg-yellow-500/10', label: 'Pending' },
  processing: { icon: Loader2, color: 'text-blue-500', bg: 'bg-blue-500/10', label: 'Processing' },
  completed: { icon: CheckCircle, color: 'text-green-500', bg: 'bg-green-500/10', label: 'Completed' },
  indexed: { icon: CheckCircle, color: 'text-green-500', bg: 'bg-green-500/10', label: 'Indexed' },
  failed: { icon: XCircle, color: 'text-red-500', bg: 'bg-red-500/10', label: 'Failed' },
} as const;

type DocumentStatus = keyof typeof statusConfig;

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

  // Fetch full document for content preview
  const { data: fullDocument, isLoading: isLoadingContent } = useQuery({
    queryKey: ['document', document?.id],
    queryFn: () => (document ? getDocument(document.id) : Promise.resolve(null)),
    enabled: !!document?.id,
    staleTime: 5 * 60 * 1000, // 5 minutes
  });

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
      <div className="flex flex-col items-center justify-center h-full text-center p-6">
        <div className="rounded-full bg-muted p-4 mb-4">
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
  const isFailed = status === 'failed';

  const contentPreview = fullDocument?.content || document?.content_summary || '';
  const previewLength = 500;
  const hasMoreContent = contentPreview.length > previewLength;
  const displayContent = showFullContent ? contentPreview : contentPreview.slice(0, previewLength);

  return (
    <div className="space-y-4">
      {/* Document Header */}
      <div className="space-y-2">
        <div className="flex items-start gap-3">
          <div className={`rounded-lg p-2.5 ${statusInfo.bg}`}>
            <FileText className={`h-5 w-5 ${statusInfo.color}`} />
          </div>
          <div className="flex-1 min-w-0">
            <h3 className="font-semibold text-base leading-tight truncate">
              {document.title || document.file_name || `Document ${document.id.slice(0, 8)}`}
            </h3>
            <div className="flex items-center gap-2 mt-1">
              <Badge
                variant="outline"
                className={`gap-1 ${statusInfo.color}`}
              >
                <StatusIcon className={`h-3 w-3 ${isProcessing ? 'animate-spin' : ''}`} />
                {statusInfo.label}
              </Badge>
            </div>
          </div>
        </div>
      </div>

      <Separator />

      {/* Metadata */}
      <div className="space-y-3">
        <h4 className="text-sm font-medium text-muted-foreground uppercase tracking-wider">
          {t('documents.preview.metadata', 'Details')}
        </h4>
        
        <div className="grid gap-2">
          {/* ID */}
          <div className="flex items-center justify-between">
            <span className="text-sm text-muted-foreground flex items-center gap-1.5">
              <FileText className="h-3.5 w-3.5" />
              ID
            </span>
            <div className="flex items-center gap-1">
              <code className="text-xs bg-muted px-1.5 py-0.5 rounded font-mono">
                {document.id.slice(0, 12)}...
              </code>
              <TooltipProvider>
                <Tooltip>
                  <TooltipTrigger asChild>
                    <Button variant="ghost" size="icon" className="h-6 w-6" onClick={handleCopyId}>
                      <Copy className="h-3 w-3" />
                    </Button>
                  </TooltipTrigger>
                  <TooltipContent>{t('common.copy', 'Copy')}</TooltipContent>
                </Tooltip>
              </TooltipProvider>
            </div>
          </div>

          {/* Size */}
          {(document.file_size || document.content_length) && (
            <div className="flex items-center justify-between">
              <span className="text-sm text-muted-foreground flex items-center gap-1.5">
                <HardDrive className="h-3.5 w-3.5" />
                {t('documents.preview.size', 'Size')}
              </span>
              <span className="text-sm font-medium">{formatFileSize(document.file_size || document.content_length)}</span>
            </div>
          )}

          {/* Created */}
          {document.created_at && (
            <div className="flex items-center justify-between">
              <span className="text-sm text-muted-foreground flex items-center gap-1.5">
                <Calendar className="h-3.5 w-3.5" />
                {t('documents.preview.created', 'Created')}
              </span>
              <TooltipProvider>
                <Tooltip>
                  <TooltipTrigger asChild>
                    <span className="text-sm font-medium cursor-help">
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

          {/* Entities */}
          {(document.entity_count || document.chunk_count) && (
            <div className="flex items-center justify-between">
              <span className="text-sm text-muted-foreground flex items-center gap-1.5">
                <Network className="h-3.5 w-3.5" />
                {t('documents.preview.entities', 'Entities')}
              </span>
              <span className="text-sm font-medium">
                {document.entity_count ?? document.chunk_count ?? 0}
              </span>
            </div>
          )}
        </div>
      </div>

      <Separator />

      {/* Content Preview */}
      <div className="space-y-3">
        <div className="flex items-center justify-between">
          <h4 className="text-sm font-medium text-muted-foreground uppercase tracking-wider">
            {t('documents.preview.content', 'Content Preview')}
          </h4>
          {(fullDocument?.content || document?.content_summary) && (
            <Button variant="ghost" size="sm" className="h-7 text-xs" onClick={handleCopyContent}>
              <Copy className="h-3 w-3 mr-1" />
              {t('common.copy', 'Copy')}
            </Button>
          )}
        </div>

        <Card className="bg-muted/30">
          <CardContent className="p-3">
            {isLoadingContent ? (
              <div className="space-y-2">
                <Skeleton className="h-4 w-full" />
                <Skeleton className="h-4 w-3/4" />
                <Skeleton className="h-4 w-1/2" />
              </div>
            ) : (fullDocument?.content || document?.content_summary) ? (
              <div className="space-y-2">
                <pre className="text-xs text-muted-foreground whitespace-pre-wrap font-mono leading-relaxed max-h-[200px] overflow-y-auto">
                  {displayContent}
                  {!showFullContent && hasMoreContent && '...'}
                </pre>
                {hasMoreContent && (
                  <Button
                    variant="ghost"
                    size="sm"
                    className="w-full h-7 text-xs"
                    onClick={() => setShowFullContent(!showFullContent)}
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

      {/* Error Info */}
      {isFailed && document.error_message && (
        <>
          <Separator />
          <div className="space-y-2">
            <h4 className="text-sm font-medium text-destructive flex items-center gap-1.5">
              <AlertCircle className="h-4 w-4" />
              {t('documents.preview.error', 'Processing Error')}
            </h4>
            <Card className="bg-destructive/5 border-destructive/20">
              <CardContent className="p-3">
                <p className="text-xs text-destructive">{document.error_message}</p>
              </CardContent>
            </Card>
          </div>
        </>
      )}

      <Separator />

      {/* Actions */}
      <div className="space-y-2">
        <h4 className="text-sm font-medium text-muted-foreground uppercase tracking-wider">
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
        >
          <ExternalLink className="h-3 w-3 mr-1.5" />
          {t('documents.actions.openInNewTab', 'Open in New Tab')}
        </Button>
      </div>
    </div>
  );
}

export default DocumentPreviewPanel;
