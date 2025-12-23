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
    FileText,
    Loader2,
    MoreVertical,
    RefreshCw,
    Trash2,
    Upload,
    XCircle,
} from 'lucide-react';
import { useCallback, useState } from 'react';
import { useDropzone } from 'react-dropzone';
import { useTranslation } from 'react-i18next';
import { toast } from 'sonner';
import { DocumentFilters, type DocStatus, type SortDirection, type SortField } from './document-filters';
import { PaginationControls } from './pagination-controls';
import { PipelineStatusDialog } from './pipeline-status-dialog';

const statusConfig = {
  pending: { icon: Clock, color: 'bg-yellow-500', label: 'Pending', animate: false },
  processing: { icon: Loader2, color: 'bg-blue-500', label: 'Processing', animate: true },
  completed: { icon: CheckCircle, color: 'bg-green-500', label: 'Completed', animate: false },
  failed: { icon: XCircle, color: 'bg-red-500', label: 'Failed', animate: false },
} as const;

type DocumentStatus = keyof typeof statusConfig;

function StatusBadge({ status }: { status: DocumentStatus }) {
  const config = statusConfig[status];
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

  const uploadMutation = useMutation({
    mutationFn: async (content: string) => {
      return uploadDocument({ content, source_type: 'text' });
    },
    onSuccess: () => {
      toast.success('Document uploaded successfully');
      queryClient.invalidateQueries({ queryKey: ['documents'] });
    },
    onError: (error) => {
      toast.error(`Upload failed: ${error instanceof Error ? error.message : 'Unknown error'}`);
    },
  });

  const deleteMutation = useMutation({
    mutationFn: deleteDocument,
    onSuccess: () => {
      toast.success('Document deleted');
      queryClient.invalidateQueries({ queryKey: ['documents'] });
    },
    onError: (error) => {
      toast.error(`Delete failed: ${error instanceof Error ? error.message : 'Unknown error'}`);
    },
  });

  const deleteAllMutation = useMutation({
    mutationFn: deleteAllDocuments,
    onSuccess: (data) => {
      toast.success(`Deleted ${data.deleted_count} documents`);
      queryClient.invalidateQueries({ queryKey: ['documents'] });
    },
    onError: (error) => {
      toast.error(`Delete all failed: ${error instanceof Error ? error.message : 'Unknown error'}`);
    },
  });

  const reprocessMutation = useMutation({
    mutationFn: reprocessDocument,
    onSuccess: () => {
      toast.success('Document queued for reprocessing');
      queryClient.invalidateQueries({ queryKey: ['documents'] });
    },
    onError: (error) => {
      toast.error(`Reprocess failed: ${error instanceof Error ? error.message : 'Unknown error'}`);
    },
  });

  const onDrop = useCallback(
    async (acceptedFiles: File[]) => {
      for (const file of acceptedFiles) {
        const text = await file.text();
        uploadMutation.mutate(text);
      }
    },
    [uploadMutation]
  );

  const { getRootProps, getInputProps, isDragActive } = useDropzone({
    onDrop,
    accept: {
      'text/plain': ['.txt'],
      'text/markdown': ['.md'],
      'application/json': ['.json'],
    },
  });

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

  const documents = sortDocuments(data?.items || []);
  const totalPages = Math.ceil((data?.total || 0) / pageSize);
  const totalCount = data?.total || documents.length;

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
                  <AlertDialogTitle>{t('documents.deleteAllTitle')}</AlertDialogTitle>
                  <AlertDialogDescription>
                    {t('documents.deleteAllDescription', { count: totalCount })}
                  </AlertDialogDescription>
                </AlertDialogHeader>
                <AlertDialogFooter>
                  <AlertDialogCancel>{t('common.cancel')}</AlertDialogCancel>
                  <AlertDialogAction
                    onClick={() => deleteAllMutation.mutate()}
                    className="bg-destructive text-destructive-foreground"
                  >
                    {t('documents.deleteAll')}
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
        </CardContent>
      </Card>

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
                            {t('documents.reprocess')}
                          </DropdownMenuItem>
                          <DropdownMenuItem
                            onClick={() => deleteMutation.mutate(doc.id)}
                            className="text-destructive"
                          >
                            <Trash2 className="h-4 w-4 mr-2" />
                            {t('documents.delete')}
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
