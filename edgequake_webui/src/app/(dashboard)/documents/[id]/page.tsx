'use client';

import { MarkdownRenderer } from '@/components/query/markdown-renderer';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { ScrollArea } from '@/components/ui/scroll-area';
import { Skeleton } from '@/components/ui/skeleton';
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs';
import {
    Tooltip,
    TooltipContent,
    TooltipProvider,
    TooltipTrigger,
} from '@/components/ui/tooltip';
import { getDocument } from '@/lib/api/edgequake';
import { useTenantStore } from '@/stores/use-tenant-store';
import { useQuery } from '@tanstack/react-query';
import { format, formatDistanceToNow } from 'date-fns';
import {
    AlertCircle,
    ArrowLeft,
    Brain,
    Calendar,
    CheckCircle,
    Clock,
    Code2,
    Copy,
    Eye,
    FileCode,
    FileText,
    HardDrive,
    Hash,
    Link2,
    Loader2,
    Network,
    RefreshCw,
    Tag,
    Timer,
    XCircle,
} from 'lucide-react';
import Link from 'next/link';
import { useParams, useRouter } from 'next/navigation';
import { useCallback } from 'react';
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

function formatFileSize(bytes: number | undefined): string {
  if (!bytes) return 'Unknown';
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  return `${(bytes / (1024 * 1024)).toFixed(2)} MB`;
}

export default function DocumentViewPage() {
  const { t } = useTranslation();
  const router = useRouter();
  const params = useParams();
  const documentId = params.id as string;
  const { selectedWorkspaceId } = useTenantStore();

  // Fetch document details
  const { data: document, isLoading, isError, error, refetch } = useQuery({
    queryKey: ['document', documentId, selectedWorkspaceId],
    queryFn: () => getDocument(documentId),
    enabled: !!documentId && !!selectedWorkspaceId,
    staleTime: 30 * 1000, // 30 seconds
    refetchOnMount: 'always',
  });

  const handleCopyId = useCallback(async () => {
    try {
      await navigator.clipboard.writeText(documentId);
      toast.success(t('documents.preview.idCopied', 'Document ID copied to clipboard'));
    } catch {
      toast.error(t('common.copyFailed', 'Failed to copy'));
    }
  }, [documentId, t]);

  const handleCopyContent = useCallback(async () => {
    if (!document?.content) return;
    try {
      await navigator.clipboard.writeText(document.content);
      toast.success(t('documents.preview.contentCopied', 'Content copied to clipboard'));
    } catch {
      toast.error(t('common.copyFailed', 'Failed to copy'));
    }
  }, [document, t]);

  const handleViewInGraph = useCallback(() => {
    if (document) {
      router.push(`/graph?highlight=${document.id}`);
    }
  }, [document, router]);

  // Loading state
  if (isLoading) {
    return (
      <div className="flex flex-col h-full">
        {/* Header */}
        <div className="shrink-0 p-4 border-b bg-background">
          <div className="flex items-center gap-3">
            <Button variant="ghost" size="icon" asChild>
              <Link href="/documents">
                <ArrowLeft className="h-4 w-4" />
              </Link>
            </Button>
            <Skeleton className="h-6 w-48" />
          </div>
        </div>
        {/* Content skeleton */}
        <div className="flex-1 p-4 space-y-4">
          <Skeleton className="h-32 w-full" />
          <Skeleton className="h-64 w-full" />
        </div>
      </div>
    );
  }

  // Error state
  if (isError || !document) {
    return (
      <div className="flex flex-col h-full">
        {/* Header */}
        <div className="shrink-0 p-4 border-b bg-background">
          <div className="flex items-center gap-3">
            <Button variant="ghost" size="icon" asChild>
              <Link href="/documents">
                <ArrowLeft className="h-4 w-4" />
              </Link>
            </Button>
            <h1 className="text-lg font-semibold">Document Not Found</h1>
          </div>
        </div>
        {/* Error content */}
        <div className="flex-1 flex items-center justify-center p-4">
          <div className="text-center max-w-md">
            <div className="rounded-full bg-red-500/10 p-4 w-fit mx-auto mb-4">
              <AlertCircle className="h-8 w-8 text-red-500" />
            </div>
            <h2 className="text-xl font-semibold mb-2">Document Not Found</h2>
            <p className="text-muted-foreground mb-4">
              {(error as Error)?.message || 'The document you are looking for could not be found or you may not have access to it.'}
            </p>
            <div className="flex gap-2 justify-center">
              <Button variant="outline" onClick={() => refetch()}>
                <RefreshCw className="h-4 w-4 mr-2" />
                Retry
              </Button>
              <Button asChild>
                <Link href="/documents">Back to Documents</Link>
              </Button>
            </div>
          </div>
        </div>
      </div>
    );
  }

  const status = (document.status || 'completed') as DocumentStatus;
  const statusInfo = statusConfig[status] || statusConfig.completed;
  const StatusIcon = statusInfo.icon;
  const isFailed = status === 'failed';

  return (
    <div className="flex flex-col h-full overflow-hidden">
      {/* Fixed Header */}
      <div className="shrink-0 p-4 border-b bg-background">
        <div className="flex items-center justify-between">
          <div className="flex items-center gap-3">
            <Button variant="ghost" size="icon" asChild>
              <Link href="/documents">
                <ArrowLeft className="h-4 w-4" />
              </Link>
            </Button>
            <div>
              <h1 className="text-lg font-semibold truncate max-w-md">
                {document.title || document.file_name || `Document ${document.id.slice(0, 8)}`}
              </h1>
              <div className="flex items-center gap-2 text-sm text-muted-foreground">
                <code className="text-xs">{document.id.slice(0, 12)}...</code>
                <Button variant="ghost" size="icon" className="h-5 w-5" onClick={handleCopyId}>
                  <Copy className="h-3 w-3" />
                </Button>
              </div>
            </div>
          </div>
          <div className="flex items-center gap-2">
            <Badge className={`${statusInfo.bg} ${statusInfo.color} border-0`}>
              <StatusIcon className={`h-3 w-3 mr-1 ${status === 'processing' ? 'animate-spin' : ''}`} />
              {statusInfo.label}
            </Badge>
            <Button variant="outline" size="sm" onClick={handleViewInGraph}>
              <Network className="h-4 w-4 mr-2" />
              View in Graph
            </Button>
          </div>
        </div>
      </div>

      {/* Scrollable Content */}
      <ScrollArea className="flex-1">
        <div className="p-4 space-y-4">
          {/* Error Message (if failed) */}
          {isFailed && document.error_message && (
            <Card className="border-red-200 bg-red-50 dark:border-red-900 dark:bg-red-950/50">
              <CardContent className="py-3 px-4">
                <div className="flex items-start gap-3">
                  <AlertCircle className="h-5 w-5 text-red-500 shrink-0 mt-0.5" />
                  <div>
                    <p className="font-medium text-red-700 dark:text-red-300">Processing Failed</p>
                    <p className="text-sm text-red-600 dark:text-red-400">{document.error_message}</p>
                  </div>
                </div>
              </CardContent>
            </Card>
          )}

          {/* Metadata Cards */}
          <div className="grid grid-cols-2 md:grid-cols-5 gap-4">
            <Card>
              <CardContent className="py-3 px-4">
                <div className="flex items-center gap-2 text-muted-foreground mb-1">
                  <FileText className="h-4 w-4" />
                  <span className="text-xs font-medium">Chunks</span>
                </div>
                <p className="text-xl font-semibold">{document.chunk_count ?? '-'}</p>
              </CardContent>
            </Card>
            <Card>
              <CardContent className="py-3 px-4">
                <div className="flex items-center gap-2 text-muted-foreground mb-1">
                  <Network className="h-4 w-4" />
                  <span className="text-xs font-medium">Entities</span>
                </div>
                <p className="text-xl font-semibold">{document.entity_count ?? '-'}</p>
              </CardContent>
            </Card>
            <Card>
              <CardContent className="py-3 px-4">
                <div className="flex items-center gap-2 text-muted-foreground mb-1">
                  <Link2 className="h-4 w-4" />
                  <span className="text-xs font-medium">Relations</span>
                </div>
                <p className="text-xl font-semibold">{document.relationship_count ?? '-'}</p>
              </CardContent>
            </Card>
            <Card>
              <CardContent className="py-3 px-4">
                <div className="flex items-center gap-2 text-muted-foreground mb-1">
                  <HardDrive className="h-4 w-4" />
                  <span className="text-xs font-medium">File Size</span>
                </div>
                <p className="text-xl font-semibold">{formatFileSize(document.file_size)}</p>
              </CardContent>
            </Card>
            <Card>
              <CardContent className="py-3 px-4">
                <div className="flex items-center gap-2 text-muted-foreground mb-1">
                  <Calendar className="h-4 w-4" />
                  <span className="text-xs font-medium">Created</span>
                </div>
                <p className="text-sm font-semibold">
                  {document.created_at 
                    ? formatDistanceToNow(new Date(document.created_at), { addSuffix: true })
                    : '-'}
                </p>
              </CardContent>
            </Card>
          </div>

          {/* Document Details */}
          <Card>
            <CardHeader className="py-3 px-4">
              <CardTitle className="text-sm font-medium">Details</CardTitle>
            </CardHeader>
            <CardContent className="py-0 px-4 pb-4">
              <div className="grid grid-cols-2 gap-x-8 gap-y-3 text-sm">
                <div>
                  <span className="text-muted-foreground">File Name</span>
                  <p className="font-medium truncate">{document.file_name || 'N/A'}</p>
                </div>
                <div>
                  <span className="text-muted-foreground">MIME Type</span>
                  <p className="font-medium">{document.mime_type || 'Unknown'}</p>
                </div>
                <div>
                  <span className="text-muted-foreground">Source Type</span>
                  <p className="font-medium capitalize">{document.source_type || 'Unknown'}</p>
                </div>
                <div>
                  <span className="text-muted-foreground">Content Length</span>
                  <p className="font-medium">{document.content_length?.toLocaleString() ?? '-'} characters</p>
                </div>
                {document.created_at && (
                  <div>
                    <span className="text-muted-foreground">Created At</span>
                    <p className="font-medium">{format(new Date(document.created_at), 'PPpp')}</p>
                  </div>
                )}
                {document.processed_at && (
                  <div>
                    <span className="text-muted-foreground">Processed At</span>
                    <p className="font-medium">{format(new Date(document.processed_at), 'PPpp')}</p>
                  </div>
                )}
                {document.track_id && (
                  <div className="col-span-2">
                    <span className="text-muted-foreground">Track ID</span>
                    <p className="font-mono text-xs">{document.track_id}</p>
                  </div>
                )}
              </div>
            </CardContent>
          </Card>

          {/* Extraction Lineage */}
          {(document.lineage || document.relationship_count !== undefined) && (
            <Card>
              <CardHeader className="py-3 px-4">
                <CardTitle className="text-sm font-medium flex items-center gap-2">
                  <Brain className="h-4 w-4 text-muted-foreground" />
                  Extraction Lineage
                </CardTitle>
              </CardHeader>
              <CardContent className="py-0 px-4 pb-4">
                <div className="grid grid-cols-2 md:grid-cols-3 gap-4 mb-4">
                  {/* Relationships */}
                  <div className="flex items-center gap-2 p-3 bg-muted/50 rounded-lg">
                    <Link2 className="h-4 w-4 text-muted-foreground" />
                    <div>
                      <p className="text-xs text-muted-foreground">Relationships</p>
                      <p className="text-lg font-semibold">{document.relationship_count ?? 0}</p>
                    </div>
                  </div>
                  
                  {/* Processing Duration */}
                  {document.lineage?.processing_duration_ms && (
                    <div className="flex items-center gap-2 p-3 bg-muted/50 rounded-lg">
                      <Timer className="h-4 w-4 text-muted-foreground" />
                      <div>
                        <p className="text-xs text-muted-foreground">Processing Time</p>
                        <p className="text-lg font-semibold">
                          {document.lineage.processing_duration_ms > 1000 
                            ? `${(document.lineage.processing_duration_ms / 1000).toFixed(1)}s`
                            : `${document.lineage.processing_duration_ms}ms`}
                        </p>
                      </div>
                    </div>
                  )}
                  
                  {/* Avg Chunk Size */}
                  {document.lineage?.avg_chunk_size && (
                    <div className="flex items-center gap-2 p-3 bg-muted/50 rounded-lg">
                      <Hash className="h-4 w-4 text-muted-foreground" />
                      <div>
                        <p className="text-xs text-muted-foreground">Avg Chunk Size</p>
                        <p className="text-lg font-semibold">{document.lineage.avg_chunk_size.toLocaleString()} chars</p>
                      </div>
                    </div>
                  )}
                </div>

                <div className="grid grid-cols-2 gap-x-8 gap-y-3 text-sm">
                  {/* LLM Model */}
                  {document.lineage?.llm_model && (
                    <div>
                      <span className="text-muted-foreground">LLM Model</span>
                      <p className="font-medium font-mono text-xs">{document.lineage.llm_model}</p>
                    </div>
                  )}
                  
                  {/* Embedding Model */}
                  {document.lineage?.embedding_model && (
                    <div>
                      <span className="text-muted-foreground">Embedding Model</span>
                      <p className="font-medium font-mono text-xs">{document.lineage.embedding_model}</p>
                    </div>
                  )}
                  
                  {/* Embedding Dimensions */}
                  {document.lineage?.embedding_dimensions && (
                    <div>
                      <span className="text-muted-foreground">Embedding Dimensions</span>
                      <p className="font-medium">{document.lineage.embedding_dimensions}</p>
                    </div>
                  )}
                  
                  {/* Chunking Strategy */}
                  {document.lineage?.chunking_strategy && (
                    <div>
                      <span className="text-muted-foreground">Chunking Strategy</span>
                      <p className="font-medium">{document.lineage.chunking_strategy}</p>
                    </div>
                  )}
                </div>

                {/* Entity Types */}
                {document.lineage?.entity_types && document.lineage.entity_types.length > 0 && (
                  <div className="mt-4">
                    <span className="text-sm text-muted-foreground">Entity Types Extracted</span>
                    <div className="flex flex-wrap gap-2 mt-2">
                      {document.lineage.entity_types.map((type) => (
                        <Badge key={type} variant="secondary" className="text-xs">
                          {type}
                        </Badge>
                      ))}
                    </div>
                  </div>
                )}

                {/* Relationship Types */}
                {document.lineage?.relationship_types && document.lineage.relationship_types.length > 0 && (
                  <div className="mt-4">
                    <span className="text-sm text-muted-foreground">Relationship Types</span>
                    <div className="flex flex-wrap gap-2 mt-2">
                      {document.lineage.relationship_types.map((type) => (
                        <Badge key={type} variant="outline" className="text-xs">
                          {type}
                        </Badge>
                      ))}
                    </div>
                  </div>
                )}

                {/* Keywords */}
                {document.lineage?.keywords && document.lineage.keywords.length > 0 && (
                  <div className="mt-4">
                    <span className="text-sm text-muted-foreground flex items-center gap-1">
                      <Tag className="h-3.5 w-3.5" />
                      Keywords ({document.lineage.keywords.length})
                    </span>
                    <div className="flex flex-wrap gap-1.5 mt-2">
                      {document.lineage.keywords.slice(0, 30).map((keyword) => (
                        <Badge key={keyword} variant="outline" className="text-xs font-normal">
                          {keyword}
                        </Badge>
                      ))}
                      {document.lineage.keywords.length > 30 && (
                        <Badge variant="secondary" className="text-xs">
                          +{document.lineage.keywords.length - 30} more
                        </Badge>
                      )}
                    </div>
                  </div>
                )}
              </CardContent>
            </Card>
          )}

          {/* Document Content */}
          <Card>
            <CardHeader className="py-3 px-4">
              <div className="flex items-center justify-between">
                <CardTitle className="text-sm font-medium flex items-center gap-2">
                  <FileCode className="h-4 w-4 text-muted-foreground" />
                  {t('documents.view.content', 'Document Content')}
                </CardTitle>
                {document.content && (
                  <TooltipProvider>
                    <Tooltip>
                      <TooltipTrigger asChild>
                        <Button variant="ghost" size="sm" onClick={handleCopyContent}>
                          <Copy className="h-3.5 w-3.5 mr-1.5" />
                          {t('common.copy', 'Copy')}
                        </Button>
                      </TooltipTrigger>
                      <TooltipContent>
                        {t('documents.view.copyContent', 'Copy document content to clipboard')}
                      </TooltipContent>
                    </Tooltip>
                  </TooltipProvider>
                )}
              </div>
            </CardHeader>
            <CardContent className="py-0 px-4 pb-4">
              {document.content ? (
                <Tabs defaultValue="rendered" className="w-full">
                  <TabsList className="grid w-full max-w-xs grid-cols-2 mb-4">
                    <TabsTrigger value="rendered" className="flex items-center gap-1.5 text-xs">
                      <Eye className="h-3.5 w-3.5" />
                      {t('documents.view.rendered', 'Rendered')}
                    </TabsTrigger>
                    <TabsTrigger value="raw" className="flex items-center gap-1.5 text-xs">
                      <Code2 className="h-3.5 w-3.5" />
                      {t('documents.view.raw', 'Raw')}
                    </TabsTrigger>
                  </TabsList>
                  
                  <TabsContent value="rendered" className="mt-0">
                    <div className="bg-card border rounded-xl p-6 max-h-[70vh] overflow-auto shadow-sm">
                      <article className="prose prose-sm dark:prose-invert max-w-none prose-headings:font-semibold prose-h1:text-2xl prose-h2:text-xl prose-h3:text-lg prose-p:text-foreground/90 prose-a:text-primary prose-code:bg-muted prose-code:px-1.5 prose-code:py-0.5 prose-code:rounded prose-pre:bg-muted prose-pre:border prose-blockquote:border-l-primary">
                        <MarkdownRenderer
                          content={document.content}
                          enableMath={true}
                          enableMermaid={true}
                          className="text-sm leading-relaxed"
                        />
                      </article>
                    </div>
                  </TabsContent>
                  
                  <TabsContent value="raw" className="mt-0">
                    <div className="bg-muted/50 rounded-xl p-4 max-h-[70vh] overflow-auto border font-mono">
                      <pre className="text-xs whitespace-pre-wrap break-words text-foreground/80 leading-relaxed">
                        {document.content}
                      </pre>
                    </div>
                  </TabsContent>
                </Tabs>
              ) : document.content_summary ? (
                <div>
                  <div className="flex items-center gap-2 mb-3">
                    <Badge variant="secondary" className="text-xs">
                      {t('documents.view.preview', 'Preview')}
                    </Badge>
                    <span className="text-xs text-muted-foreground">
                      {t('documents.view.first200Chars', 'First 200 characters')}
                    </span>
                  </div>
                  <div className="bg-muted/50 rounded-xl p-4 border">
                    <div className="prose prose-sm dark:prose-invert max-w-none">
                      <MarkdownRenderer
                        content={document.content_summary}
                        enableMath={false}
                        enableMermaid={false}
                        className="text-sm"
                      />
                    </div>
                  </div>
                </div>
              ) : (
                <div className="py-8 text-center">
                  <FileText className="h-10 w-10 mx-auto text-muted-foreground/40 mb-3" />
                  <p className="text-sm text-muted-foreground italic">
                    {t('documents.view.noContent', 'No content available')}
                  </p>
                </div>
              )}
            </CardContent>
          </Card>
        </div>
      </ScrollArea>
    </div>
  );
}
