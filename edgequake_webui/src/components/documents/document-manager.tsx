/**
 * @module DocumentManager
 * @description Document ingestion and management interface.
 * Supports file upload, progress tracking, status monitoring, and batch operations.
 * 
 * @implements UC0001 - User uploads documents for ingestion
 * @implements UC0007 - User monitors document processing progress
 * @implements UC0008 - User reprocesses failed documents
 * @implements UC0009 - User deletes documents from knowledge graph
 * @implements FEAT0001 - Document ingestion with entity extraction
 * @implements FEAT0003 - Batch document processing
 * @implements FEAT0004 - Processing status tracking per document
 * @implements FEAT0602 - Real-time progress indicators
 * 
 * @enforces BR0302 - Failed documents can be reprocessed
 * @enforces BR0303 - Document deletion cascades to related entities
 * @enforces BR0305 - Cost tracking per document ingestion
 * 
 * @see {@link docs/use_cases.md} UC0001, UC0007-UC0009
 * @see {@link docs/features.md} FEAT0001, FEAT0003
 */
'use client';

import { RightPanel } from '@/components/layout/right-panel';
import { Alert, AlertDescription, AlertTitle } from '@/components/ui/alert';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { Checkbox } from '@/components/ui/checkbox';
import { Input } from '@/components/ui/input';
import {
    Table,
    TableBody,
    TableCell,
    TableHead,
    TableHeader,
    TableRow,
} from '@/components/ui/table';
import {
    cancelTask,
    deleteAllDocuments,
    deleteDocument,
    getDocuments,
    getPipelineStatus,
    reprocessDocument,
    uploadDocument,
    uploadPdfDocument,
    type DocumentsListResult,
} from '@/lib/api/edgequake';
import { cn } from '@/lib/utils';
import { useTenantStore } from '@/stores/use-tenant-store';
import type { Document } from '@/types';
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { formatDistanceToNow } from 'date-fns';
import {
    AlertCircle,
    File,
    FileCode,
    FileImage,
    FileSpreadsheet,
    FileText,
    FileType,
    Loader2,
    RefreshCw,
    Search,
    X,
} from 'lucide-react';
import { useRouter } from 'next/navigation';
import { useCallback, useEffect, useState } from 'react';
import { useDropzone } from 'react-dropzone';
import { useTranslation } from 'react-i18next';
import { toast } from 'sonner';
import { ClearDocumentsDialog } from './clear-documents-dialog';
import { BatchActionsBar } from './batch-actions-bar';
import { ConnectionBanner } from './connection-banner';
import { ConnectionStatus } from './connection-status';
import { CostCell } from './cost-cell';
import { DocumentActionsMenu } from './document-actions-menu';
import { DocumentDropzone } from './document-dropzone';
import { DocumentTableStates } from './document-table-states';
import { QuickActionButtons } from './quick-action-buttons';
import { DocumentFilters, type DocStatus, type SortDirection, type SortField } from './document-filters';
import { DocumentPreviewPanel } from './document-preview-panel';
import { DocumentViewerDialog } from './document-viewer-dialog';
import { EnhancedStatusBadge } from './enhanced-status-badge';
import { ErrorMessagePopover } from './error-message-popover';
import { PaginationControls } from './pagination-controls';
import { PipelineStatusDialog } from './pipeline-status-dialog';
import { ProcessingStatusSummary } from './processing-status-summary';
import { ReprocessFailedButton } from './reprocess-failed-button';
import { UploadProgressList } from './upload-progress-list';
import type { UploadingFile } from './types';
import { useStuckDetection } from '@/hooks/use-stuck-detection';
import { useDocumentWebSocket } from '@/hooks/use-document-websocket';

/**
 * OODA-30: File type icon helper
 * WHY: Visual distinction helps users quickly identify document types
 */
function getFileTypeIcon(fileName: string | undefined | null) {
  if (!fileName) return { icon: File, color: 'text-muted-foreground' };
  const ext = fileName.split('.').pop()?.toLowerCase();
  switch (ext) {
    case 'pdf':
      return { icon: FileText, color: 'text-red-500' };
    case 'doc':
    case 'docx':
      return { icon: FileType, color: 'text-blue-500' };
    case 'xls':
    case 'xlsx':
    case 'csv':
      return { icon: FileSpreadsheet, color: 'text-green-500' };
    case 'md':
    case 'markdown':
      return { icon: FileCode, color: 'text-purple-500' };
    case 'txt':
      return { icon: FileText, color: 'text-gray-500' };
    case 'html':
    case 'htm':
    case 'json':
    case 'xml':
      return { icon: FileCode, color: 'text-orange-500' };
    case 'jpg':
    case 'jpeg':
    case 'png':
    case 'gif':
    case 'webp':
      return { icon: FileImage, color: 'text-pink-500' };
    default:
      return { icon: File, color: 'text-muted-foreground' };
  }
}

/**
 * OODA-32: Highlight search matches in text
 * WHY: Visual feedback shows which part of title matched the search
 */
function highlightMatches(text: string, query: string): React.ReactNode {
  if (!query.trim()) return text;
  const regex = new RegExp(`(${query.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')})`, 'gi');
  const parts = text.split(regex);
  return parts.map((part, i) =>
    regex.test(part) ? (
      <mark key={i} className="bg-yellow-200 dark:bg-yellow-700 px-0.5 rounded">
        {part}
      </mark>
    ) : (
      part
    )
  );
}

export function DocumentManager() {
  const { t } = useTranslation();
  const queryClient = useQueryClient();
  const router = useRouter();
  
  // Get tenant context for query key
  const { selectedTenantId, selectedWorkspaceId } = useTenantStore();

  // CRITICAL DEBUG: Log tenant/workspace changes
  useEffect(() => {
    console.log('[DocumentManager] Tenant/Workspace context:', {
      selectedTenantId,
      selectedWorkspaceId,
      timestamp: new Date().toISOString(),
    });
  }, [selectedTenantId, selectedWorkspaceId]);
  
  // Bulk selection state
  const [selectedIds, setSelectedIds] = useState<Set<string>>(new Set());
  
  // Selected document for preview panel
  const [selectedDocument, setSelectedDocument] = useState<Document | null>(null);
  const [previewPanelOpen, setPreviewPanelOpen] = useState(false);
  
  // SPEC-002: Document viewer dialog state for PDF/Markdown side-by-side view
  const [viewerDialogOpen, setViewerDialogOpen] = useState(false);
  const [viewerPdfId, setViewerPdfId] = useState<string | null>(null);
  
  // Search state
  const [searchQuery, setSearchQuery] = useState('');
  
  // Pagination state
  const [currentPage, setCurrentPage] = useState(1);
  // OODA-29: Initialize pageSize from localStorage for persistence
  const [pageSize, setPageSize] = useState(() => {
    if (typeof window === 'undefined') return 20;
    try {
      const stored = localStorage.getItem('edgequake:documents:prefs');
      const parsed = stored ? JSON.parse(stored) : null;
      const size = parsed?.pageSize;
      return [10, 20, 50, 100].includes(size) ? size : 20;
    } catch { return 20; }
  });
  
  // Filter and sort state
  // OODA-24/28: Initialize from localStorage for persistence
  const [statusFilter, setStatusFilter] = useState<DocStatus>(() => {
    if (typeof window === 'undefined') return 'all';
    try {
      const stored = localStorage.getItem('edgequake:documents:prefs');
      const parsed = stored ? JSON.parse(stored) : null;
      return (parsed?.statusFilter as DocStatus) || 'all';
    } catch { return 'all'; }
  });
  const [sortField, setSortField] = useState<SortField>(() => {
    if (typeof window === 'undefined') return 'created_at';
    try {
      const stored = localStorage.getItem('edgequake:documents:prefs');
      const parsed = stored ? JSON.parse(stored) : null;
      return (parsed?.sortField as SortField) || 'created_at';
    } catch { return 'created_at'; }
  });
  const [sortDirection, setSortDirection] = useState<SortDirection>(() => {
    if (typeof window === 'undefined') return 'desc';
    try {
      const stored = localStorage.getItem('edgequake:documents:prefs');
      const parsed = stored ? JSON.parse(stored) : null;
      return (parsed?.sortDirection as SortDirection) || 'desc';
    } catch { return 'desc'; }
  });
  
  // Pipeline status dialog state
  const [pipelineDialogOpen, setPipelineDialogOpen] = useState(false);

  // Upload progress tracking state
  const [uploadingFiles, setUploadingFiles] = useState<UploadingFile[]>([]);
  const [isUploading, setIsUploading] = useState(false);

  // OODA-42 COMPLETE: WebSocket-based real-time updates (NO POLLING)
  // WHY: Users want instant document status updates without polling overhead
  // HOW: Subscribe to WebSocket events for all processing documents
  const { data, isLoading, isError, error, refetch } = useQuery({
    queryKey: ['documents', selectedTenantId, selectedWorkspaceId, currentPage, pageSize, statusFilter],
    queryFn: () => getDocuments({ 
      page: currentPage, 
      page_size: pageSize,
      status: statusFilter === 'all' ? undefined : statusFilter,
    }),
    // NO polling - WebSocket provides real-time updates
    refetchInterval: false,
  });
  
  // Pipeline status query
  // OODA-37: Include workspace in queryKey for proper isolation
  // CRITICAL: Pass tenant_id and workspace_id to getPipelineStatus for multi-tenancy isolation
  const { data: pipelineStatus } = useQuery({
    queryKey: ['pipeline-status', selectedTenantId, selectedWorkspaceId],
    queryFn: () => getPipelineStatus(selectedTenantId ?? undefined, selectedWorkspaceId ?? undefined),
    refetchInterval: 2000,
  });

  // OODA-05: WebSocket subscription for real-time document status updates
  // WHY: Extracted to useDocumentWebSocket hook for SRP compliance
  useDocumentWebSocket(data?.items, queryClient);

  // OODA-04: Detect stuck documents using extracted hook
  useStuckDetection(data?.items, {
    timeout: 30000,
    checkInterval: 30000,
  });

  // Enhanced upload handler with progress tracking
  const handleFilesUpload = useCallback(
    async (files: File[]) => {
      if (files.length === 0) return;
      
      // Auto-switch to 'all' filter so processing documents are visible
      setStatusFilter('all');
      
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
          // Phase 2: Uploading to server
          setUploadingFiles((prev) =>
            prev.map((f, idx) =>
              idx === i ? { ...f, status: 'uploading' as const, progress: 40, phase: t('documents.upload.uploading', 'Uploading to server...') } : f
            )
          );

          let response: { document_id?: string; pdf_id?: string; duplicate_of?: string; task_id?: string; track_id?: string };

          // Check if file is PDF - route to PDF upload endpoint
          // OODA-22: Track isPdf flag for enhanced progress component
          const isPdfFile = file.type === 'application/pdf';
          
          if (isPdfFile) {
            // Upload PDF file directly (multipart/form-data)
            const pdfResponse = await uploadPdfDocument(file, {
              title: file.name,
              enable_vision: true, // Enable vision extraction by default for PDFs
              track_id: trackId, // Pass batch tracking ID
            });
            
            // Map PdfUploadResponse to compatible format
            response = {
              document_id: pdfResponse.document_id,
              pdf_id: pdfResponse.pdf_id,
              duplicate_of: pdfResponse.duplicate_of,
              task_id: pdfResponse.task_id,
              track_id: pdfResponse.track_id, // Use track_id from response
            };
            
            // OODA-42: Optimistic update for PDF upload
            // WHY: PDFs must appear immediately in documents panel (same as markdown)
            // The backend creates the document record async, but we add it to cache now
            // FIX: Include tenant_id and workspace_id for multi-tenant filtering
            if (pdfResponse.pdf_id && !pdfResponse.duplicate_of) {
              const optimisticDoc: Document = {
                id: pdfResponse.pdf_id, // Use pdf_id as temporary ID until document_id is assigned
                title: file.name,
                file_name: file.name,
                file_size: file.size,
                source_type: 'pdf',
                status: 'processing',
                mime_type: 'application/pdf',
                created_at: new Date().toISOString(),
                pdf_id: pdfResponse.pdf_id,
                track_id: pdfResponse.track_id,
                tenant_id: selectedTenantId ?? undefined,
                workspace_id: selectedWorkspaceId ?? undefined,
              };
              
              // Add to the SPECIFIC tenant/workspace query cache for instant visibility
              // IMPORTANT: Must match exact query key to appear immediately
              queryClient.setQueriesData<DocumentsListResult>(
                { queryKey: ['documents', selectedTenantId, selectedWorkspaceId] },
                (old) => {
                  if (!old || !old.items || !Array.isArray(old.items)) return old;
                  // Check if document already exists (by pdf_id)
                  const exists = old.items.some(d => d.pdf_id === pdfResponse.pdf_id || d.id === pdfResponse.pdf_id);
                  if (exists) return old;
                  return {
                    ...old,
                    items: [optimisticDoc, ...old.items],
                    total: (old.total ?? 0) + 1,
                  };
                }
              );
            }
            
            // OODA-22: Store track_id and isPdf flag for enhanced progress tracking
            setUploadingFiles((prev) =>
              prev.map((f, idx) =>
                idx === i ? { 
                  ...f, 
                  trackId: pdfResponse.track_id,
                  isPdf: true,
                } : f
              )
            );
          } else {
            // Read text file content
            const text = await file.text();
            
            // Upload text document with async processing
            const textResponse = await uploadDocument({ 
              content: text, 
              source_type: 'text',
              title: file.name,
              async_processing: true,
              track_id: trackId,
            });
            
            response = textResponse;
            
            // OODA-42 EXTENDED: Optimistic update for text/markdown files (same as PDF)
            // WHY: Text files must also appear immediately in documents panel
            // FIX: Include tenant_id and workspace_id for multi-tenant filtering
            if (textResponse.document_id && !textResponse.duplicate_of) {
              const optimisticDoc: Document = {
                id: textResponse.document_id,
                title: file.name,
                file_name: file.name,
                file_size: file.size,
                source_type: 'text',
                status: 'processing',
                mime_type: file.type || 'text/plain',
                created_at: new Date().toISOString(),
                track_id: textResponse.track_id,
                tenant_id: selectedTenantId ?? undefined,
                workspace_id: selectedWorkspaceId ?? undefined,
              };
              
              // Add to the SPECIFIC tenant/workspace query cache for instant visibility
              // IMPORTANT: Must match exact query key to appear immediately
              queryClient.setQueriesData<DocumentsListResult>(
                { queryKey: ['documents', selectedTenantId, selectedWorkspaceId] },
                (old) => {
                  if (!old || !old.items || !Array.isArray(old.items)) return old;
                  // Check if document already exists (by document_id)
                  const exists = old.items.some(d => d.id === textResponse.document_id);
                  if (exists) return old;
                  return {
                    ...old,
                    items: [optimisticDoc, ...old.items],
                    total: (old.total ?? 0) + 1,
                  };
                }
              );
            }
          }
          
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

      // Clear upload list after a delay
      setTimeout(() => {
        setUploadingFiles([]);
      }, 3000);
    },
    [queryClient, t, router, selectedWorkspaceId]
  );

  // Remove a file from the upload list
  const removeUploadingFile = useCallback((index: number) => {
    setUploadingFiles((prev) => prev.filter((_, i) => i !== index));
  }, []);

  // OODA-06: Callbacks for UploadProgressList component
  // WHY: Mark PDF upload as successful when PdfUploadProgress completes
  const handleUploadComplete = useCallback((index: number) => {
    setUploadingFiles((prev) =>
      prev.map((f, idx) =>
        idx === index ? { ...f, status: 'success' as const, progress: 100 } : f
      )
    );
  }, []);

  // WHY: Mark PDF upload as failed when PdfUploadProgress reports error
  const handleUploadFailed = useCallback((index: number, error: string) => {
    setUploadingFiles((prev) =>
      prev.map((f, idx) =>
        idx === index ? { ...f, status: 'error' as const, error } : f
      )
    );
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
            return t('documents.upload.invalidType', 'File "{{name}}" has an unsupported format. Supported: TXT, MD, JSON, PDF.', {
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
    mutationFn: (documentId: string) => reprocessDocument(documentId, true),
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

  // Cancel mutation for stopping document processing
  const cancelMutation = useMutation({
    mutationFn: async (trackId: string) => {
      await cancelTask(trackId);
    },
    onSuccess: () => {
      toast.success(t('documents.cancel.success', 'Document processing cancelled'), {
        duration: 4000,
        description: t('documents.cancel.successDesc', 'The extraction has been stopped.'),
      });
      queryClient.invalidateQueries({ queryKey: ['documents'] });
    },
    onError: (error) => {
      toast.error(t('documents.cancel.failed', 'Cancel failed'), {
        description: error instanceof Error ? error.message : t('documents.cancel.failedDesc', 'Could not cancel processing. It may have already completed.'),
      });
    },
  });

  const { getRootProps, getInputProps, isDragActive, open: openFileDialog } = useDropzone({
    onDrop,
    accept: {
      'text/plain': ['.txt'],
      'text/markdown': ['.md'],
      'application/json': ['.json'],
      'application/pdf': ['.pdf'],
    },
    maxSize: MAX_FILE_SIZE, // 10MB limit
    noClick: false, // Allow click on dropzone
  });

  // Filter documents client-side (fallback when backend doesn't support filtering)
  const filterDocuments = (docs: Document[]): Document[] => {
    let filtered = docs;
    
    // Apply search filter
    if (searchQuery.trim()) {
      const query = searchQuery.toLowerCase().trim();
      filtered = filtered.filter((doc) => {
        const title = doc.title?.toLowerCase() || '';
        const fileName = doc.file_name?.toLowerCase() || '';
        return title.includes(query) || fileName.includes(query) || doc.id.includes(query);
      });
    }
    
    // Apply status filter
    if (statusFilter !== 'all') {
      filtered = filtered.filter((doc) => {
        const docStatus = doc.status || 'completed';
        return docStatus === statusFilter;
      });
    }
    
    return filtered;
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
    partial_failure: serverStatusCounts.partial_failure || 0,
    cancelled: serverStatusCounts.cancelled || 0,
  } : {
    all: allDocuments.length,
    pending: allDocuments.filter((d) => d.status === 'pending').length,
    processing: allDocuments.filter((d) => d.status === 'processing').length,
    completed: allDocuments.filter((d) => !d.status || d.status === 'completed' || d.status === 'indexed').length,
    failed: allDocuments.filter((d) => d.status === 'failed').length,
    partial_failure: allDocuments.filter((d) => d.status === 'partial_failure').length,
    cancelled: allDocuments.filter((d) => d.status === 'cancelled').length,
  };

  // Bulk selection handlers
  const handleSelectAll = useCallback((checked: boolean) => {
    if (checked) {
      setSelectedIds(new Set(documents.map(d => d.id)));
    } else {
      setSelectedIds(new Set());
    }
  }, [documents]);

  const handleSelectOne = useCallback((docId: string, checked: boolean) => {
    setSelectedIds(prev => {
      const next = new Set(prev);
      if (checked) {
        next.add(docId);
      } else {
        next.delete(docId);
      }
      return next;
    });
  }, []);

  // OODA-07: Clear selection callback for BatchActionsBar
  const handleClearSelection = useCallback(() => {
    setSelectedIds(new Set());
  }, []);

  const handleBulkDelete = useCallback(async () => {
    const idsToDelete = Array.from(selectedIds);
    let successCount = 0;
    let errorCount = 0;

    for (const id of idsToDelete) {
      try {
        await deleteDocument(id);
        successCount++;
      } catch {
        errorCount++;
      }
    }

    if (successCount > 0) {
      toast.success(t('documents.bulk.deleteSuccess', { count: successCount }) || `Deleted ${successCount} document(s)`);
      queryClient.invalidateQueries({ queryKey: ['documents'] });
    }
    if (errorCount > 0) {
      toast.error(t('documents.bulk.deleteFailed', { count: errorCount }) || `Failed to delete ${errorCount} document(s)`);
    }
    setSelectedIds(new Set());
  }, [selectedIds, queryClient, t]);

  const handleBulkReprocess = useCallback(async () => {
    const idsToReprocess = Array.from(selectedIds);
    let successCount = 0;
    let errorCount = 0;

    // Get documents to find their track_ids
    const documents = data?.items || [];
    
    for (const id of idsToReprocess) {
      try {
        const doc = documents.find(d => d.id === id);
        if (!doc?.track_id) {
          errorCount++;
          continue;
        }
        await reprocessDocument(doc.track_id);
        successCount++;
      } catch {
        errorCount++;
      }
    }

    if (successCount > 0) {
      toast.success(t('documents.bulk.reprocessSuccess', { count: successCount }) || `Queued ${successCount} document(s) for reprocessing`);
      queryClient.invalidateQueries({ queryKey: ['documents'] });
    }
    if (errorCount > 0) {
      toast.error(t('documents.bulk.reprocessFailed', { count: errorCount }) || `Failed to queue ${errorCount} document(s)`);
    }
    setSelectedIds(new Set());
  }, [selectedIds, data, queryClient, t]);

  // Document selection for preview panel
  const handleDocumentClick = useCallback((doc: Document) => {
    setSelectedDocument(doc);
    setPreviewPanelOpen(true);
  }, []);

  /**
   * OODA-41: Double-click to navigate to document detail page
   * WHY: Power users expect double-click for primary navigation action
   * SPEC-002: Navigate to dedicated document detail page, not dialog
   */
  const handleDocumentDoubleClick = useCallback((doc: Document) => {
    router.push(`/documents/${doc.id}`);
  }, [router]);

  /**
   * OODA-41: Navigate to document detail page (for View Details button)
   * WHY: Users need explicit link to dedicated document view
   */
  const handleViewDetails = useCallback((doc: Document) => {
    router.push(`/documents/${doc.id}`);
  }, [router]);

  const handlePreviewClose = useCallback(() => {
    setSelectedDocument(null);
    setPreviewPanelOpen(false);
  }, []);

  const handleViewInGraph = useCallback((doc: Document) => {
    router.push(`/graph?entity=${encodeURIComponent(doc.id)}`);
  }, [router]);

  /**
   * SPEC-002: Open PDF viewer dialog for PDF documents
   * WHY: Users need to view original PDF alongside extracted markdown
   */
  const handleViewPdf = useCallback((doc: Document) => {
    // Use pdf_id if available, otherwise try to derive from source_type
    const pdfId = doc.pdf_id || (doc.source_type === 'pdf' ? doc.id : null);
    if (pdfId) {
      setViewerPdfId(pdfId);
      setViewerDialogOpen(true);
    } else {
      // Fallback to standard document view
      router.push(`/documents/${doc.id}`);
    }
  }, [router]);

  /**
   * OODA-19: Keyboard shortcuts for power users
   * WHY: Keyboard shortcuts improve efficiency and accessibility
   * 
   * Shortcuts:
   * - Escape: Clear selection or close preview panel
   * - Ctrl/Cmd + A: Select all documents
   * - R: Refresh document list (when not in input)
   */
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      // Skip if in input field or textarea
      const target = e.target as HTMLElement;
      const tagName = target.tagName.toUpperCase();
      if (tagName === 'INPUT' || tagName === 'TEXTAREA' || target.isContentEditable) {
        return;
      }

      // Escape: Clear selection or close preview panel
      if (e.key === 'Escape') {
        if (previewPanelOpen) {
          handlePreviewClose();
        } else if (selectedIds.size > 0) {
          setSelectedIds(new Set());
        }
        return;
      }

      // Ctrl/Cmd + A: Select all documents
      if ((e.metaKey || e.ctrlKey) && e.key.toLowerCase() === 'a') {
        e.preventDefault(); // Prevent browser select all
        handleSelectAll(true);
        return;
      }

      // R: Refresh documents (single key, no modifier)
      if (e.key.toLowerCase() === 'r' && !e.metaKey && !e.ctrlKey && !e.altKey) {
        refetch();
        toast.info(t('documents.refresh.triggered', 'Refreshing documents...'), { duration: 1000 });
        return;
      }
    };

    document.addEventListener('keydown', handleKeyDown);
    return () => document.removeEventListener('keydown', handleKeyDown);
  }, [previewPanelOpen, selectedIds.size, handlePreviewClose, handleSelectAll, refetch, t]);

  /**
   * OODA-24: Persist sort preferences to localStorage
   * WHY: Users expect their filter/sort preferences to persist across sessions
   */
  useEffect(() => {
    try {
      localStorage.setItem('edgequake:documents:prefs', JSON.stringify({
        statusFilter,
        sortField,
        sortDirection,
        pageSize, // OODA-29: Also persist page size preference
      }));
    } catch {
      // Ignore localStorage errors (e.g., in incognito mode)
    }
  }, [statusFilter, sortField, sortDirection, pageSize]);

  /**
   * OODA-26: Update page title with document count
   * WHY: Users can see document count without switching tabs
   */
  useEffect(() => {
    const baseTitle = 'Documents - EdgeQuake';
    const count = totalCount || 0;
    const processing = pipelineStatus?.running_tasks || 0;
    
    if (processing > 0) {
      document.title = `⏳ Processing (${processing}) | Documents (${count}) - EdgeQuake`;
    } else if (count > 0) {
      document.title = `Documents (${count}) - EdgeQuake`;
    } else {
      document.title = baseTitle;
    }
    
    return () => { document.title = baseTitle; };
  }, [totalCount, pipelineStatus?.running_tasks]);

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
    <div className="flex h-full overflow-hidden">
      {/* Main Content - Flex column for proper scroll zones */}
      <div className="flex-1 flex flex-col min-h-0 overflow-hidden">
        {/* Fixed Header Zone */}
        <div className="shrink-0 px-4 pt-4 space-y-3 bg-background">
          {/* OODA-02: Connection status banner when disconnected */}
          <ConnectionBanner />
          
          {/* Header - Compact */}
          <header className="flex items-center justify-between gap-3 flex-wrap">
            <div className="space-y-0.5">
              <div className="flex items-center gap-2">
                <h1 className="text-xl font-semibold tracking-tight">{t('documents.title')}</h1>
                {/* OODA-39: Document count badge */}
                {totalCount > 0 && (
                  <Badge variant="secondary" className="text-xs font-normal">
                    {totalCount}
                  </Badge>
                )}
                {/* OODA-30: WebSocket connection status indicator */}
                <ConnectionStatus compact={true} />
              </div>
              <p className="text-sm text-muted-foreground">
                {t('documents.subtitle')}
              </p>
            </div>
          <div className="flex items-center gap-2 flex-wrap">
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
              tenantId={selectedTenantId ?? undefined}
              workspaceId={selectedWorkspaceId ?? undefined}
            />
            
            {/* Reprocess Failed Button (GAP-UI-002) */}
            <ReprocessFailedButton
              failedCount={statusCounts.failed}
              onReprocessStarted={(trackId) => {
                setPipelineDialogOpen(true);
              }}
            />
          
          <Button variant="outline" size="sm" onClick={() => refetch()}>
            <RefreshCw className="h-4 w-4 mr-1" />
            {t('documents.refresh')}
          </Button>
          
          {/* Clear Documents Dialog (GAP-UI-009) */}
          <ClearDocumentsDialog
            documentCount={totalCount}
            onCleared={() => refetch()}
          />
        </div>
      </header>
      
      {/* Search and Filters */}
      <div className="flex flex-col sm:flex-row sm:items-center gap-3 pb-3 border-b">
        <div className="relative flex-1 max-w-md">
          <Search className="absolute left-3 top-1/2 -translate-y-1/2 h-4 w-4 text-muted-foreground" />
          <Input
            placeholder={t('documents.search.placeholder', 'Search documents...')}
            value={searchQuery}
            onChange={(e) => setSearchQuery(e.target.value)}
            className="pl-9 pr-8 h-9"
          />
          {/* OODA-36: Clear search button */}
          {searchQuery && (
            <button
              type="button"
              onClick={() => setSearchQuery('')}
              className="absolute right-2 top-1/2 -translate-y-1/2 p-1 rounded hover:bg-muted transition-colors"
              aria-label="Clear search"
            >
              <X className="h-3.5 w-3.5 text-muted-foreground" />
            </button>
          )}
        </div>
        <DocumentFilters
          status={statusFilter}
          onStatusChange={setStatusFilter}
          sortField={sortField}
          onSortFieldChange={setSortField}
          sortDirection={sortDirection}
          onSortDirectionChange={setSortDirection}
          statusCounts={statusCounts}
        />
      </div>

      {/* OODA-11: Processing Status Summary - Extracted to ProcessingStatusSummary component */}
      {pipelineStatus && (
        <ProcessingStatusSummary
          pipelineStatus={pipelineStatus}
          documents={documents}
          onOpenDetails={() => setPipelineDialogOpen(true)}
        />
      )}

      {/* OODA-08: Compact Upload Zone - Extracted to DocumentDropzone component */}
      <DocumentDropzone
        getRootProps={getRootProps}
        getInputProps={getInputProps}
        isDragActive={isDragActive}
      />

      {/* OODA-07: Bulk Actions Bar - Extracted to BatchActionsBar component */}
      <BatchActionsBar
        selectedCount={selectedIds.size}
        onReprocess={handleBulkReprocess}
        onDelete={handleBulkDelete}
        onClear={handleClearSelection}
      />

      {/* OODA-06: Upload Progress - Extracted to UploadProgressList component */}
      <UploadProgressList
        uploadingFiles={uploadingFiles}
        isUploading={isUploading}
        onRemove={removeUploadingFile}
        onComplete={handleUploadComplete}
        onFailed={handleUploadFailed}
      />

      </div>

      {/* Scrollable Documents Table Zone */}
      <div className="flex-1 min-h-0 overflow-auto">
        <div className="px-4 py-3">
          {/* Table Header */}
          <div className="flex items-center gap-2 mb-3">
            <FileText className="h-4 w-4 text-muted-foreground" />
            <span className="text-sm font-medium">Documents ({documents.length})</span>
          </div>
          
          {/* OODA-12: Loading skeleton and empty state - Extracted to DocumentTableStates */}
          <DocumentTableStates
            isLoading={isLoading}
            isEmpty={documents.length === 0}
            onUploadClick={openFileDialog}
          />
          
          {!isLoading && documents.length > 0 && (
            <div className="border rounded-lg overflow-hidden shadow-sm">
              <Table>
                <TableHeader className="bg-muted/50 sticky top-0 z-10">
                  <TableRow className="hover:bg-transparent">
                    <TableHead className="w-[40px]">
                      <Checkbox
                        checked={selectedIds.size === documents.length && documents.length > 0}
                        onCheckedChange={(checked) => handleSelectAll(!!checked)}
                        aria-label={t('documents.bulk.selectAll', 'Select all')}
                      />
                    </TableHead>
                    <TableHead>Title</TableHead>
                    <TableHead>Status</TableHead>
                    <TableHead className="text-center">Entities</TableHead>
                    <TableHead className="text-center">Cost</TableHead>
                    <TableHead>Created</TableHead>
                    <TableHead className="w-[100px]"></TableHead>
                  </TableRow>
                </TableHeader>
                <TableBody>
                  {documents.map((doc, index) => (
                    <TableRow 
                      key={doc.id}
                      className={cn(
                        "cursor-pointer transition-colors duration-150",
                        "hover:bg-primary/5 dark:hover:bg-primary/10",
                        selectedDocument?.id === doc.id && "bg-primary/10 dark:bg-primary/15 ring-1 ring-primary/20",
                        index % 2 === 0 ? "bg-background" : "bg-muted/20",
                        // OODA-25: Failed documents highlight
                        doc.status === 'failed' && "bg-red-50/50 dark:bg-red-950/20 border-l-4 border-l-red-500",
                        doc.status === 'partial_failure' && "bg-orange-50/50 dark:bg-orange-950/20 border-l-4 border-l-orange-500"
                      )}
                      onClick={() => handleDocumentClick(doc)}
                      onDoubleClick={() => handleDocumentDoubleClick(doc)}
                    >
                      <TableCell onClick={(e) => e.stopPropagation()}>
                        <Checkbox
                          checked={selectedIds.has(doc.id)}
                          onCheckedChange={(checked) => handleSelectOne(doc.id, !!checked)}
                          aria-label={t('documents.bulk.select', 'Select')}
                        />
                      </TableCell>
                      <TableCell className="font-medium">
                        <div className="flex flex-col gap-0.5">
                          {/* OODA-30: File type icon for visual identification */}
                          <div className="flex items-center gap-2">
                            {(() => {
                              const { icon: FileIcon, color } = getFileTypeIcon(doc.file_name);
                              return <FileIcon className={cn("h-4 w-4 shrink-0", color)} />;
                            })()}
                            {/* OODA-32: Highlight search matches */}
                            <span className="truncate">
                              {highlightMatches(
                                doc.title || doc.file_name || `Document ${doc.id.slice(0, 8)}`,
                                searchQuery
                              )}
                            </span>
                          </div>
                          {/* OODA-05: Enhanced error display with copy and retry */}
                          {(doc.status === 'failed' || doc.status === 'partial_failure') && doc.error_message && (
                            <ErrorMessagePopover
                              message={doc.error_message}
                              documentId={doc.id}
                              onRetry={() => reprocessMutation.mutate(doc.id)}
                              isRetrying={reprocessMutation.isPending}
                            />
                          )}
                        </div>
                      </TableCell>
                      <TableCell>
                        <div className="flex flex-col gap-1">
                          <EnhancedStatusBadge document={doc} />
                          {/* Show stage_message below badge for better visibility during PDF conversion */}
                          {doc.stage_message && doc.current_stage === 'converting' && (
                            <span className="text-xs text-muted-foreground truncate">
                              {doc.stage_message}
                            </span>
                          )}
                        </div>
                      </TableCell>
                      <TableCell className="text-center">{doc.entity_count ?? doc.chunk_count ?? '-'}</TableCell>
                      <TableCell className="text-center">
                        <CostCell 
                          document={doc}
                          size="sm" 
                        />
                      </TableCell>
                      <TableCell className="text-muted-foreground">
                        {doc.created_at 
                          ? (
                            <div className="flex items-center gap-1.5">
                              {/* OODA-34: "New" indicator for documents created within 1 hour */}
                              {new Date().getTime() - new Date(doc.created_at).getTime() < 3600000 && (
                                <span className="text-xs font-medium text-green-600 dark:text-green-400 animate-pulse">
                                  NEW
                                </span>
                              )}
                              <span>{formatDistanceToNow(new Date(doc.created_at), { addSuffix: true })}</span>
                            </div>
                          )
                          : '-'}
                      </TableCell>
                      <TableCell onClick={(e) => e.stopPropagation()}>
                        {/* OODA-10: Quick action buttons - Extracted to QuickActionButtons */}
                        <QuickActionButtons
                          doc={doc}
                          onViewDetails={handleViewDetails}
                          onPreview={handleDocumentClick}
                          onViewInGraph={handleViewInGraph}
                          onRetry={(id) => reprocessMutation.mutate(id)}
                          isRetrying={reprocessMutation.isPending}
                        >
                          {/* OODA-09: Actions dropdown - Extracted to DocumentActionsMenu */}
                          <DocumentActionsMenu
                            doc={doc}
                            onViewPdf={handleViewPdf}
                            onCancel={(trackId) => cancelMutation.mutate(trackId)}
                            onReprocess={(id) => reprocessMutation.mutate(id)}
                            onDelete={(id) => deleteMutation.mutate(id)}
                            isCancelling={cancelMutation.isPending}
                          />
                        </QuickActionButtons>
                      </TableCell>
                    </TableRow>
                  ))}
                </TableBody>
              </Table>
            </div>
          )}
        </div>
      </div>
          
      {/* Fixed Pagination Footer */}
      {documents.length > 0 && (
        <div className="shrink-0 px-4 py-3 border-t bg-background">
          {/* OODA-37: Show filtered vs total count when filtering */}
          {(searchQuery || statusFilter !== 'all') && (
            <p className="text-xs text-muted-foreground mb-2 text-center">
              Showing {documents.length} of {totalCount} documents
              {statusFilter !== 'all' && ` (${statusFilter})`}
              {searchQuery && ` matching "${searchQuery}"`}
            </p>
          )}
          <PaginationControls
            currentPage={currentPage}
            totalPages={totalPages}
            pageSize={pageSize}
            onPageChange={setCurrentPage}
            onPageSizeChange={(newSize) => {
              setPageSize(newSize);
              setCurrentPage(1);
            }}
          />
        </div>
      )}
      </div>

      {/* Right Panel - Document Preview */}
      <RightPanel
        isOpen={previewPanelOpen}
        onToggle={() => setPreviewPanelOpen(!previewPanelOpen)}
        onClose={handlePreviewClose}
        title={selectedDocument ? (selectedDocument.title || selectedDocument.file_name || `Document ${selectedDocument.id.slice(0, 8)}`) : t('documents.preview.title', 'Document Preview')}
        subtitle={selectedDocument?.id ? `ID: ${selectedDocument.id.slice(0, 12)}...` : undefined}
        width="wide"
        showCollapsedBar={true}
        collapsedLabel={t('documents.preview.panelLabel', 'Preview')}
        headerIcon={<FileText className="h-4 w-4" />}
      >
        <DocumentPreviewPanel
          document={selectedDocument}
          onDelete={(id) => {
            deleteMutation.mutate(id);
            handlePreviewClose();
          }}
          onReprocess={(id) => reprocessMutation.mutate(id)}
          onViewFull={(doc) => {
            // OODA-41: Always navigate to document detail page
            // WHY: Per SPEC-002, use dedicated page instead of dialog
            router.push(`/documents/${doc.id}`);
          }}
          onViewInGraph={handleViewInGraph}
          isDeleting={deleteMutation.isPending}
          isReprocessing={reprocessMutation.isPending}
        />
      </RightPanel>

      {/* SPEC-002: PDF/Markdown Viewer Dialog */}
      <DocumentViewerDialog
        open={viewerDialogOpen}
        onOpenChange={setViewerDialogOpen}
        pdfId={viewerPdfId}
      />
    </div>
  );
}

export default DocumentManager;
