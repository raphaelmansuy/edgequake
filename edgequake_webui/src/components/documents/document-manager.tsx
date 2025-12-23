'use client';

import { Alert, AlertDescription, AlertTitle } from '@/components/ui/alert';
import {
    AlertDialog,
    AlertDialogAction,
    AlertDialogCancel,
    AlertDialogContent,
    AlertDialogDescription,
    AlertDialogFooter,
    AlertDialogHeader,
    AlertDialogTitle,
    AlertDialogTrigger,
} from '@/components/ui/alert-dialog';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import {
    DropdownMenu,
    DropdownMenuContent,
    DropdownMenuItem,
    DropdownMenuTrigger,
} from '@/components/ui/dropdown-menu';
import { Progress } from '@/components/ui/progress';
import { ScrollArea } from '@/components/ui/scroll-area';
import { Skeleton } from '@/components/ui/skeleton';
import {
    Table,
    TableBody,
    TableCell,
    TableHead,
    TableHeader,
    TableRow,
} from '@/components/ui/table';
import {
    deleteAllDocuments,
    deleteDocument,
    getDocuments,
    getPipelineStatus,
    reprocessDocument,
    uploadDocument,
} from '@/lib/api/edgequake';
import type { Document } from '@/types';
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { formatDistanceToNow } from 'date-fns';
import {
    AlertCircle,
    CheckCircle,
    Clock,
    FileSearch,
    FileText,
    Loader2,
    MoreVertical,
    RefreshCw,
    Sparkles,
    Trash2,
    Upload,
    X,
    XCircle,
} from 'lucide-react';
import { useCallback, useState } from 'react';
import { useDropzone } from 'react-dropzone';
import { useTranslation } from 'react-i18next';
import { toast } from 'sonner';
import { useRouter } from 'next/navigation';
import { BatchProgressCard } from './batch-progress-card';
import { DocumentFilters, type DocStatus, type SortDirection, type SortField } from './document-filters';
import { PaginationControls } from './pagination-controls';
import { PipelineStatusDialog } from './pipeline-status-dialog';

// Track upload progress and errors for files
interface UploadingFile {
  file: File;
  progress: number;
  status: 'pending' | 'reading' | 'uploading' | 'extracting' | 'success' | 'error';
  error?: string;
  phase?: string; // Human-readable phase description
}

const statusConfig = {
  pending: { icon: Clock, color: 'bg-yellow-500', label: 'Pending', animate: false },
  processing: { icon: Loader2, color: 'bg-blue-500', label: 'Processing', animate: true },
  completed: { icon: CheckCircle, color: 'bg-green-500', label: 'Completed', animate: false },
  indexed: { icon: CheckCircle, color: 'bg-green-500', label: 'Indexed', animate: false },
  failed: { icon: XCircle, color: 'bg-red-500', label: 'Failed', animate: false },
} as const;

type DocumentStatus = keyof typeof statusConfig;

function StatusBadge({ status }: { status: DocumentStatus }) {
  const config = statusConfig[status] || statusConfig.completed;
  const Icon = config.icon;

  return (
    <Badge variant="outline" className="gap-1">
      <Icon className={`h-3 w-3 ${config.animate ? 'animate-spin' : ''}`} />
      {config.label}
    </Badge>
  );
}

export function DocumentManager() {
  const { t } = useTranslation();
  const queryClient = useQueryClient();
  const router = useRouter();
  // TODO: Implement bulk selection in future
  // const [selectedIds, setSelectedIds] = useState<Set<string>>(new Set());
  
  // Pagination state
  const [currentPage, setCurrentPage] = useState(1);
  const [pageSize, setPageSize] = useState(20);
  
  // Filter and sort state
  const [statusFilter, setStatusFilter] = useState<DocStatus>('all');
  const [sortField, setSortField] = useState<SortField>('created_at');
  const [sortDirection, setSortDirection] = useState<SortDirection>('desc');
  
  // Pipeline status dialog state
  const [pipelineDialogOpen, setPipelineDialogOpen] = useState(false);

  // Upload progress tracking state
  const [uploadingFiles, setUploadingFiles] = useState<UploadingFile[]>([]);
  const [isUploading, setIsUploading] = useState(false);
  
  // Track ID for batch progress (Phase 2) - async processing enabled
  const [activeTrackId, setActiveTrackId] = useState<string | null>(null);

  const { data, isLoading, isError, error, refetch } = useQuery({
    queryKey: ['documents', currentPage, pageSize, statusFilter],
    queryFn: () => getDocuments({ 
      page: currentPage, 
      page_size: pageSize,
      status: statusFilter === 'all' ? undefined : statusFilter,
    }),
    refetchInterval: 5000, // Poll for status updates
  });
  
  // Pipeline status query
  const { data: pipelineStatus } = useQuery({
    queryKey: ['pipeline-status'],
    queryFn: getPipelineStatus,
    refetchInterval: 2000,
  });

  // Enhanced upload handler with progress tracking
  const handleFilesUpload = useCallback(
    async (files: File[]) => {
      if (files.length === 0) return;
      
      setIsUploading(true);
      
      // Generate a shared track_id for this batch
      const trackId = `upload_${Date.now()}_${Math.random().toString(36).slice(2, 10)}`;
      
      // Initialize upload state for all files
      const initialFiles: UploadingFile[] = files.map((file) => ({
        file,
        progress: 0,
        status: 'pending' as const,
        phase: 'Waiting...',
      }));
      setUploadingFiles(initialFiles);

      // Show loading toast
      const toastId = toast.loading(
        t('documents.upload.inProgress', { count: files.length }) || `Uploading ${files.length} file(s)...`,
        { duration: Infinity }
      );

      let successCount = 0;
      let errorCount = 0;

      // Process files sequentially for better feedback
      for (let i = 0; i < files.length; i++) {
        const file = files[i];
        
        // Phase 1: Reading file
        setUploadingFiles((prev) =>
          prev.map((f, idx) =>
            idx === i ? { ...f, status: 'reading' as const, progress: 10, phase: t('documents.upload.reading', 'Reading file...') } : f
          )
        );

        try {
          // Read file content
          const text = await file.text();
          
          // Phase 2: Uploading to server
          setUploadingFiles((prev) =>
            prev.map((f, idx) =>
              idx === i ? { ...f, status: 'uploading' as const, progress: 40, phase: t('documents.upload.uploading', 'Uploading to server...') } : f
            )
          );

          // Upload to server with filename as title
          // Using async processing - documents are queued and processed in background
          // by the WorkerPool. Status updates via track_id polling.
          const response = await uploadDocument({ 
            content: text, 
            source_type: 'text',
            title: file.name, // Use filename as title
            async_processing: true, // Use async processing (worker pool enabled)
            track_id: trackId, // Track ID for grouping
          });
          
          // Check for duplicate (Phase 4)
          if (response.duplicate_of) {
            // Show duplicate warning
            toast.warning(
              t('documents.upload.duplicate', '{{name}} is a duplicate (existing: {{id}})', {
                name: file.name,
                id: response.duplicate_of.slice(0, 8),
              }),
              { duration: 4000 }
            );
            
            // Mark as duplicate (treat as success but with warning)
            setUploadingFiles((prev) =>
              prev.map((f, idx) =>
                idx === i ? { 
                  ...f, 
                  status: 'success' as const, 
                  progress: 100, 
                  phase: t('documents.upload.duplicateSkipped', 'Duplicate (skipped)'),
                } : f
              )
            );
            successCount++;
            continue; // Skip to next file
          }
          
          // Phase 3: Extraction queued
          setUploadingFiles((prev) =>
            prev.map((f, idx) =>
              idx === i ? { 
                ...f, 
                status: 'extracting' as const, 
                progress: 80, 
                phase: response.task_id 
                  ? t('documents.upload.queued', 'Queued for extraction (Task: {{taskId}})', { taskId: response.task_id.slice(0, 8) })
                  : t('documents.upload.extracting', 'Processing...'),
              } : f
            )
          );
          
          // Brief delay to show extraction phase
          await new Promise(resolve => setTimeout(resolve, 300));
          
          // Mark as complete
          setUploadingFiles((prev) =>
            prev.map((f, idx) =>
              idx === i ? { ...f, status: 'success' as const, progress: 100, phase: t('documents.upload.complete', 'Complete!') } : f
            )
          );
          
          successCount++;
        } catch (error) {
          // Mark as error
          const errorMessage = error instanceof Error ? error.message : 'Upload failed';
          setUploadingFiles((prev) =>
            prev.map((f, idx) =>
              idx === i ? { ...f, status: 'error' as const, progress: 100, error: errorMessage, phase: t('common.failed', 'Failed') } : f
            )
          );
          
          errorCount++;
        }
      }

      // Update toast with final result
      if (errorCount === 0) {
        toast.success(
          t('documents.upload.success', { count: successCount }) || `Successfully uploaded ${successCount} file(s)`,
          { 
            id: toastId, 
            duration: 5000,
            action: {
              label: t('documents.upload.viewInGraph', 'View in Graph'),
              onClick: () => router.push('/graph'),
            },
          }
        );
      } else if (successCount === 0) {
        toast.error(
          t('documents.upload.allFailed', { count: errorCount }) || `All ${errorCount} file(s) failed to upload`,
          { 
            id: toastId, 
            duration: 5000,
            action: {
              label: t('common.retry', 'Retry'),
              onClick: () => {
                // Reset and allow user to try again
                setUploadingFiles([]);
              },
            },
          }
        );
      } else {
        toast.warning(
          t('documents.upload.partial', { success: successCount, failed: errorCount }) || 
            `Uploaded ${successCount} file(s), ${errorCount} failed`,
          { 
            id: toastId, 
            duration: 5000,
            action: {
              label: t('documents.upload.viewInGraph', 'View in Graph'),
              onClick: () => router.push('/graph'),
            },
          }
        );
      }

      // Refresh documents list
      queryClient.invalidateQueries({ queryKey: ['documents'] });
      setIsUploading(false);

      // Set the active track ID to show batch progress card (for async processing)
      if (successCount > 0) {
        setActiveTrackId(trackId);
      }

      // Clear upload list after a delay
      setTimeout(() => {
        setUploadingFiles([]);
      }, 3000);
    },
    [queryClient, t, router]
  );

  // Remove a file from the upload list
  const removeUploadingFile = useCallback((index: number) => {
    setUploadingFiles((prev) => prev.filter((_, i) => i !== index));
  }, []);

  // Maximum file size: 10MB
  const MAX_FILE_SIZE = 10 * 1024 * 1024;

  const onDrop = useCallback(
    async (acceptedFiles: File[], fileRejections: readonly { file: File; errors: readonly { code: string; message: string }[] }[]) => {
      // Handle rejected files (too large or wrong type)
      for (const rejection of fileRejections) {
        const errorMessages = rejection.errors.map(e => {
          if (e.code === 'file-too-large') {
            const sizeMB = (rejection.file.size / (1024 * 1024)).toFixed(2);
            return t('documents.upload.fileTooLarge', 'File "{{name}}" is too large ({{size}}MB). Maximum size is 10MB.', {
              name: rejection.file.name,
              size: sizeMB,
            });
          }
          if (e.code === 'file-invalid-type') {
            return t('documents.upload.invalidType', 'File "{{name}}" has an unsupported format. Supported: TXT, MD, JSON.', {
              name: rejection.file.name,
            });
          }
          return e.message;
        }).join(', ');
        
        toast.error(errorMessages);
      }
      
      // Process accepted files
      if (acceptedFiles.length > 0) {
        await handleFilesUpload(acceptedFiles);
      }
    },
    [handleFilesUpload, t]
  );

  const deleteMutation = useMutation({
    mutationFn: deleteDocument,
    onSuccess: () => {
      toast.success(t('documents.delete.success', 'Document deleted'), {
        duration: 4000,
        description: t('documents.delete.successDesc', 'The document has been permanently removed.'),
      });
      queryClient.invalidateQueries({ queryKey: ['documents'] });
    },
    onError: (error) => {
      toast.error(t('documents.delete.failed', 'Delete failed'), {
        description: error instanceof Error ? error.message : t('common.unknownError', 'Unknown error'),
        action: {
          label: t('common.retry', 'Retry'),
          onClick: () => {
            // User can retry from the UI
          },
        },
      });
    },
  });

  const deleteAllMutation = useMutation({
    mutationFn: deleteAllDocuments,
    onSuccess: (data) => {
      toast.success(t('documents.deleteAll.success', { count: data.deleted_count }) || `Deleted ${data.deleted_count} documents`, {
        duration: 4000,
      });
      queryClient.invalidateQueries({ queryKey: ['documents'] });
    },
    onError: (error) => {
      toast.error(t('documents.deleteAll.failed', 'Delete all failed'), {
        description: error instanceof Error ? error.message : t('common.unknownError', 'Unknown error'),
        action: {
          label: t('common.retry', 'Retry'),
          onClick: () => deleteAllMutation.mutate(),
        },
      });
    },
  });

  const reprocessMutation = useMutation({
    mutationFn: reprocessDocument,
    onSuccess: () => {
      toast.success(t('documents.reprocess.success', 'Document queued for reprocessing'), {
        duration: 4000,
        action: {
          label: t('documents.viewStatus', 'View Status'),
          onClick: () => setPipelineDialogOpen(true),
        },
      });
      queryClient.invalidateQueries({ queryKey: ['documents'] });
    },
    onError: (error) => {
      toast.error(t('documents.reprocess.failed', 'Reprocess failed'), {
        description: error instanceof Error ? error.message : t('common.unknownError', 'Unknown error'),
        action: {
          label: t('common.retry', 'Retry'),
          onClick: () => {
            // User can retry from the UI
          },
        },
      });
    },
  });

  const { getRootProps, getInputProps, isDragActive } = useDropzone({
    onDrop,
    accept: {
      'text/plain': ['.txt'],
      'text/markdown': ['.md'],
      'application/json': ['.json'],
    },
    maxSize: MAX_FILE_SIZE, // 10MB limit
  });

  // Filter documents client-side (fallback when backend doesn't support filtering)
  const filterDocuments = (docs: Document[]): Document[] => {
    if (statusFilter === 'all') return docs;
    return docs.filter((doc) => {
      const docStatus = doc.status || 'completed'; // Default to completed if no status
      return docStatus === statusFilter;
    });
  };

  // Sort documents client-side for now
  const sortDocuments = (docs: Document[]): Document[] => {
    return [...docs].sort((a, b) => {
      let aVal: string | number | Date = '';
      let bVal: string | number | Date = '';
      
      switch (sortField) {
        case 'title':
          aVal = a.title || a.file_name || a.id;
          bVal = b.title || b.file_name || b.id;
          break;
        case 'created_at':
          aVal = new Date(a.created_at || 0);
          bVal = new Date(b.created_at || 0);
          break;
        case 'status':
          aVal = a.status || '';
          bVal = b.status || '';
          break;
        case 'entity_count':
          aVal = a.entity_count ?? a.chunk_count ?? 0;
          bVal = b.entity_count ?? b.chunk_count ?? 0;
          break;
      }
      
      if (aVal < bVal) return sortDirection === 'asc' ? -1 : 1;
      if (aVal > bVal) return sortDirection === 'asc' ? 1 : -1;
      return 0;
    });
  };

  const documents = sortDocuments(filterDocuments(data?.items || []));
  const allDocuments = data?.items || [];
  const totalPages = Math.ceil(documents.length / pageSize);
  const totalCount = documents.length;
  
  // Use server-side status counts from API response (more efficient)
  // Fall back to client-side calculation if not available
  const serverStatusCounts = data?.status_counts;
  const statusCounts: Record<DocStatus, number> = serverStatusCounts ? {
    all: allDocuments.length,
    pending: serverStatusCounts.pending,
    processing: serverStatusCounts.processing,
    completed: serverStatusCounts.completed,
    failed: serverStatusCounts.failed,
  } : {
    all: allDocuments.length,
    pending: allDocuments.filter((d) => d.status === 'pending').length,
    processing: allDocuments.filter((d) => d.status === 'processing').length,
    completed: allDocuments.filter((d) => !d.status || d.status === 'completed' || d.status === 'indexed').length,
    failed: allDocuments.filter((d) => d.status === 'failed').length,
  };

  if (isError) {
    return (
      <div className="p-6">
        <Alert variant="destructive">
          <AlertCircle className="h-4 w-4" />
          <AlertTitle>Error loading documents</AlertTitle>
          <AlertDescription>
            {error instanceof Error ? error.message : 'Failed to load documents'}
            <Button variant="link" className="ml-2 p-0" onClick={() => refetch()}>
              Try again
            </Button>
          </AlertDescription>
        </Alert>
      </div>
    );
  }

  return (
    <div className="p-6 space-y-6">
      {/* Header */}
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-2xl font-bold">{t('documents.title')}</h1>
          <p className="text-muted-foreground">
            {t('documents.subtitle')}
          </p>
        </div>
        <div className="flex items-center gap-2">
          {/* Pipeline Status */}
          {pipelineStatus?.is_busy && (
            <Button
              variant="outline"
              size="sm"
              onClick={() => setPipelineDialogOpen(true)}
              className="gap-1 text-orange-500"
            >
              <Loader2 className="h-4 w-4 animate-spin" />
              {t('pipeline.busy')}
            </Button>
          )}
          <PipelineStatusDialog
            open={pipelineDialogOpen}
            onOpenChange={setPipelineDialogOpen}
          />
          <Button variant="outline" size="sm" onClick={() => refetch()}>
            <RefreshCw className="h-4 w-4 mr-1" />
            {t('documents.refresh')}
          </Button>
          {documents.length > 0 && (
            <AlertDialog>
              <AlertDialogTrigger asChild>
                <Button variant="destructive" size="sm">
                  <Trash2 className="h-4 w-4 mr-1" />
                  {t('documents.clearAll')}
                </Button>
              </AlertDialogTrigger>
              <AlertDialogContent>
                <AlertDialogHeader>
                  <AlertDialogTitle>{t('documents.deleteConfirm')}</AlertDialogTitle>
                  <AlertDialogDescription>
                    {t('documents.deleteConfirmDescription', { count: totalCount })}
                  </AlertDialogDescription>
                </AlertDialogHeader>
                <AlertDialogFooter>
                  <AlertDialogCancel>{t('common.cancel')}</AlertDialogCancel>
                  <AlertDialogAction
                    onClick={() => deleteAllMutation.mutate()}
                    className="bg-destructive text-destructive-foreground"
                  >
                    {t('common.delete')}
                  </AlertDialogAction>
                </AlertDialogFooter>
              </AlertDialogContent>
            </AlertDialog>
          )}
        </div>
      </div>
      
      {/* Filters */}
      <DocumentFilters
        status={statusFilter}
        onStatusChange={setStatusFilter}
        sortField={sortField}
        onSortFieldChange={setSortField}
        sortDirection={sortDirection}
        onSortDirectionChange={setSortDirection}
        statusCounts={statusCounts}
      />

      {/* Upload Zone */}
      <Card>
        <CardContent className="p-6">
          <div
            {...getRootProps()}
            className={`
              border-2 border-dashed rounded-lg p-8 text-center cursor-pointer transition-colors
              ${isDragActive
                ? 'border-primary bg-primary/5'
                : 'border-muted-foreground/25 hover:border-primary/50'
              }
            `}
          >
            <input {...getInputProps()} />
            <Upload className="h-10 w-10 mx-auto text-muted-foreground mb-4" />
            {isDragActive ? (
              <p className="text-lg">Drop files here...</p>
            ) : (
              <>
                <p className="text-lg">Drag & drop files or click to upload</p>
                <p className="text-sm text-muted-foreground mt-1">
                  Supports TXT, MD, JSON files
                </p>
              </>
            )}
          </div>
          
          {/* Uploading Files List */}
          {uploadingFiles.length > 0 && (
            <div className="mt-4 space-y-3">
              {/* Overall Progress Header */}
              <div className="flex items-center justify-between">
                <div className="flex items-center gap-2">
                  <h4 className="text-sm font-semibold">
                    {isUploading ? (
                      <span className="flex items-center gap-2">
                        <Loader2 className="h-4 w-4 animate-spin" />
                        {t('documents.upload.processing', 'Processing Files')}
                      </span>
                    ) : (
                      <span className="flex items-center gap-2 text-green-600 dark:text-green-400">
                        <CheckCircle className="h-4 w-4" />
                        {t('documents.upload.complete', 'Upload Complete')}
                      </span>
                    )}
                  </h4>
                </div>
                <span className="text-xs text-muted-foreground">
                  {uploadingFiles.filter(f => f.status === 'success').length}/{uploadingFiles.length} {t('documents.upload.filesComplete', 'files complete')}
                </span>
              </div>
              
              {/* Phase Legend */}
              {isUploading && (
                <div className="flex items-center gap-4 text-xs text-muted-foreground bg-muted/50 rounded-lg px-3 py-2">
                  <span className="flex items-center gap-1.5">
                    <span className="h-2 w-2 rounded-full bg-amber-500" />
                    {t('documents.upload.phase.reading', 'Reading')}
                  </span>
                  <span className="text-muted-foreground/50">→</span>
                  <span className="flex items-center gap-1.5">
                    <span className="h-2 w-2 rounded-full bg-blue-500" />
                    {t('documents.upload.phase.uploading', 'Uploading')}
                  </span>
                  <span className="text-muted-foreground/50">→</span>
                  <span className="flex items-center gap-1.5">
                    <span className="h-2 w-2 rounded-full bg-purple-500" />
                    {t('documents.upload.phase.extracting', 'Extracting')}
                  </span>
                  <span className="text-muted-foreground/50">→</span>
                  <span className="flex items-center gap-1.5">
                    <span className="h-2 w-2 rounded-full bg-green-500" />
                    {t('documents.upload.phase.done', 'Done')}
                  </span>
                </div>
              )}
              
              <ScrollArea className="max-h-48">
                <div className="space-y-2">
                  {uploadingFiles.map((uploadFile, index) => (
                    <div
                      key={`${uploadFile.file.name}-${index}`}
                      className="flex items-center gap-3 p-3 rounded-lg border bg-card"
                    >
                      <div className="flex-shrink-0">
                        {uploadFile.status === 'success' ? (
                          <CheckCircle className="h-5 w-5 text-green-500" />
                        ) : uploadFile.status === 'error' ? (
                          <XCircle className="h-5 w-5 text-red-500" />
                        ) : uploadFile.status === 'extracting' ? (
                          <Sparkles className="h-5 w-5 text-purple-500 animate-pulse" />
                        ) : uploadFile.status === 'uploading' ? (
                          <Upload className="h-5 w-5 text-blue-500 animate-bounce" />
                        ) : uploadFile.status === 'reading' ? (
                          <FileSearch className="h-5 w-5 text-amber-500 animate-pulse" />
                        ) : (
                          <Clock className="h-5 w-5 text-muted-foreground" />
                        )}
                      </div>
                      <div className="flex-1 min-w-0">
                        <p className="text-sm font-medium truncate">{uploadFile.file.name}</p>
                        <div className="flex items-center gap-2">
                          <p className="text-xs text-muted-foreground">
                            {(uploadFile.file.size / 1024).toFixed(1)} KB
                          </p>
                          {uploadFile.phase && uploadFile.status !== 'success' && uploadFile.status !== 'error' && (
                            <span className={`text-xs font-medium ${
                              uploadFile.status === 'reading' ? 'text-amber-500' :
                              uploadFile.status === 'uploading' ? 'text-blue-500' :
                              uploadFile.status === 'extracting' ? 'text-purple-500' :
                              'text-muted-foreground'
                            }`}>
                              • {uploadFile.phase}
                            </span>
                          )}
                        </div>
                        {(uploadFile.status === 'reading' || uploadFile.status === 'uploading' || uploadFile.status === 'extracting') && (
                          <div className="relative mt-1">
                            <Progress value={uploadFile.progress} className="h-1.5" />
                            <div className="absolute inset-0 overflow-hidden rounded-full">
                              <div 
                                className={`h-full transition-all duration-300 ${
                                  uploadFile.status === 'reading' ? 'bg-amber-400/30' :
                                  uploadFile.status === 'uploading' ? 'bg-blue-400/30' :
                                  'bg-purple-400/30'
                                }`}
                                style={{ 
                                  width: '30%',
                                  animation: 'shimmer 1s ease-in-out infinite',
                                  transform: `translateX(${uploadFile.progress * 3}%)`
                                }}
                              />
                            </div>
                          </div>
                        )}
                        {uploadFile.error && (
                          <p className="text-xs text-red-500 mt-1">{uploadFile.error}</p>
                        )}
                      </div>
                      <Button
                        variant="ghost"
                        size="icon"
                        className="h-6 w-6 flex-shrink-0"
                        onClick={() => removeUploadingFile(index)}
                      >
                        <X className="h-4 w-4" />
                      </Button>
                    </div>
                  ))}
                </div>
              </ScrollArea>
            </div>
          )}
        </CardContent>
      </Card>

      {/* Batch Progress Card (Phase 2) - Async processing enabled */}
      {activeTrackId && !isUploading && (
        <BatchProgressCard
          trackId={activeTrackId}
          onClose={() => setActiveTrackId(null)}
          onComplete={() => {
            queryClient.invalidateQueries({ queryKey: ['documents'] });
            setTimeout(() => setActiveTrackId(null), 5000);
          }}
        />
      )}

      {/* Documents Table */}
      <Card>
        <CardHeader>
          <CardTitle className="flex items-center gap-2">
            <FileText className="h-5 w-5" />
            Documents ({documents.length})
          </CardTitle>
        </CardHeader>
        <CardContent>
          {isLoading ? (
            <div className="space-y-2">
              {[...Array(3)].map((_, i) => (
                <Skeleton key={i} className="h-12 w-full" />
              ))}
            </div>
          ) : documents.length === 0 ? (
            <div className="text-center py-8 text-muted-foreground">
              <FileText className="h-12 w-12 mx-auto mb-4 opacity-50" />
              <p>No documents yet</p>
              <p className="text-sm">Upload documents to build your knowledge graph</p>
            </div>
          ) : (
            <Table>
              <TableHeader>
                <TableRow>
                  <TableHead>Title</TableHead>
                  <TableHead>Status</TableHead>
                  <TableHead>Entities</TableHead>
                  <TableHead>Created</TableHead>
                  <TableHead className="w-[50px]"></TableHead>
                </TableRow>
              </TableHeader>
              <TableBody>
                {documents.map((doc) => (
                  <TableRow key={doc.id}>
                    <TableCell className="font-medium">
                      {doc.title || doc.file_name || `Document ${doc.id.slice(0, 8)}`}
                    </TableCell>
                    <TableCell>
                      <StatusBadge status={doc.status || 'completed'} />
                    </TableCell>
                    <TableCell>{doc.entity_count ?? doc.chunk_count ?? '-'}</TableCell>
                    <TableCell className="text-muted-foreground">
                      {doc.created_at 
                        ? formatDistanceToNow(new Date(doc.created_at), { addSuffix: true })
                        : '-'}
                    </TableCell>
                    <TableCell>
                      <DropdownMenu>
                        <DropdownMenuTrigger asChild>
                          <Button variant="ghost" size="icon" className="h-8 w-8">
                            <MoreVertical className="h-4 w-4" />
                          </Button>
                        </DropdownMenuTrigger>
                        <DropdownMenuContent align="end">
                          <DropdownMenuItem onClick={() => reprocessMutation.mutate(doc.id)}>
                            <RefreshCw className="h-4 w-4 mr-2" />
                            {t('documents.actions.reprocess')}
                          </DropdownMenuItem>
                          <DropdownMenuItem
                            onClick={() => deleteMutation.mutate(doc.id)}
                            className="text-destructive"
                          >
                            <Trash2 className="h-4 w-4 mr-2" />
                            {t('documents.actions.delete')}
                          </DropdownMenuItem>
                        </DropdownMenuContent>
                      </DropdownMenu>
                    </TableCell>
                  </TableRow>
                ))}
              </TableBody>
            </Table>
          )}
          
          {/* Pagination */}
          {documents.length > 0 && (
            <div className="mt-4">
              <PaginationControls
                currentPage={currentPage}
                totalPages={totalPages}
                pageSize={pageSize}
                onPageChange={setCurrentPage}
                onPageSizeChange={(newSize) => {
                  setPageSize(newSize);
                  setCurrentPage(1); // Reset to first page when changing page size
                }}
              />
            </div>
          )}
        </CardContent>
      </Card>
    </div>
  );
}

export default DocumentManager;
