'use client';

/**
 * Knowledge Injection Detail Page (SPEC-0002)
 *
 * View and edit a single knowledge injection entry.
 * Shows content, status, entity count, error messages, and allows editing.
 */

import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import {
    Dialog,
    DialogContent,
    DialogDescription,
    DialogFooter,
    DialogHeader,
    DialogTitle,
} from '@/components/ui/dialog';
import { Input } from '@/components/ui/input';
import { Skeleton } from '@/components/ui/skeleton';
import { Textarea } from '@/components/ui/textarea';
import {
    useDeleteInjection,
    useInjectionDetail,
    useUpdateInjection,
} from '@/hooks';
import useTenantContext from '@/hooks/use-tenant-context';
import {
    AlertCircle,
    ArrowLeft,
    BookOpen,
    CheckCircle,
    Clock,
    Loader2,
    Pencil,
    RefreshCw,
    Save,
    Trash2,
    X,
    XCircle,
} from 'lucide-react';
import Link from 'next/link';
import { useParams, useRouter } from 'next/navigation';
import { useEffect, useState } from 'react';
import { toast } from 'sonner';
import { ScrollArea } from '@/components/ui/scroll-area';

const statusConfig: Record<string, { icon: typeof CheckCircle; color: string; label: string }> = {
  completed: { icon: CheckCircle, color: 'text-green-500', label: 'Completed' },
  processing: { icon: Loader2, color: 'text-blue-500', label: 'Processing' },
  failed: { icon: XCircle, color: 'text-red-500', label: 'Failed' },
  pending: { icon: Clock, color: 'text-yellow-500', label: 'Pending' },
};

export default function KnowledgeDetailPage() {
  const router = useRouter();
  const params = useParams();
  const injectionId = params.id as string;
  const { selectedWorkspaceId } = useTenantContext();

  const { data, isLoading, refetch } = useInjectionDetail(selectedWorkspaceId, injectionId);
  const updateMutation = useUpdateInjection(selectedWorkspaceId ?? '', injectionId);
  const deleteMutation = useDeleteInjection(selectedWorkspaceId ?? '');

  const [editing, setEditing] = useState(false);
  const [editName, setEditName] = useState('');
  const [editContent, setEditContent] = useState('');
  const [deleteOpen, setDeleteOpen] = useState(false);

  // Auto-refresh while processing
  useEffect(() => {
    if (data?.status === 'processing') {
      const interval = setInterval(() => refetch(), 3000);
      return () => clearInterval(interval);
    }
  }, [data?.status, refetch]);

  const startEditing = () => {
    if (data) {
      setEditName(data.name);
      setEditContent(data.content);
      setEditing(true);
    }
  };

  const cancelEditing = () => {
    setEditing(false);
    if (data) {
      setEditName(data.name);
      setEditContent(data.content);
    }
  };

  const handleSave = async () => {
    if (!editName.trim()) {
      toast.error('Name is required');
      return;
    }
    try {
      const nameChanged = editName.trim() !== data?.name;
      const contentChanged = editContent !== data?.content;
      if (!nameChanged && !contentChanged) {
        setEditing(false);
        return;
      }
      await updateMutation.mutateAsync({
        name: nameChanged ? editName.trim() : undefined,
        content: contentChanged ? editContent : undefined,
      });
      toast.success(contentChanged ? 'Updated — re-processing entities' : 'Name updated');
      setEditing(false);
    } catch (err) {
      toast.error(`Failed to update: ${err instanceof Error ? err.message : 'Unknown error'}`);
    }
  };

  const handleRetry = async () => {
    if (!data) return;
    try {
      await updateMutation.mutateAsync({ content: data.content });
      toast.success('Re-processing injection');
    } catch (err) {
      toast.error(`Failed to retry: ${err instanceof Error ? err.message : 'Unknown error'}`);
    }
  };

  const handleDelete = async () => {
    try {
      await deleteMutation.mutateAsync(injectionId);
      toast.success('Injection deleted');
      router.push('/knowledge');
    } catch (err) {
      toast.error(`Failed to delete: ${err instanceof Error ? err.message : 'Unknown error'}`);
    }
  };

  if (isLoading) {
    return (
      <div className="flex flex-col gap-6 p-6">
        <Skeleton className="h-8 w-64" />
        <Skeleton className="h-4 w-48" />
        <Skeleton className="h-64 w-full" />
      </div>
    );
  }

  if (!data) {
    return (
      <div className="flex flex-col items-center justify-center gap-4 p-12">
        <AlertCircle className="h-12 w-12 text-muted-foreground" />
        <h2 className="text-lg font-semibold">Injection not found</h2>
        <Link href="/knowledge">
          <Button variant="outline">
            <ArrowLeft className="h-4 w-4 mr-2" />
            Back to Knowledge
          </Button>
        </Link>
      </div>
    );
  }

  const statusCfg = statusConfig[data.status] ?? statusConfig.pending;
  const StatusIcon = statusCfg.icon;

  return (
    <ScrollArea className="h-full">
      <div className="flex flex-col gap-6 p-6">
      {/* Header */}
      <div className="flex items-center gap-3">
        <Link href="/knowledge">
          <Button variant="ghost" size="icon">
            <ArrowLeft className="h-4 w-4" />
          </Button>
        </Link>
        <div className="flex-1">
          {editing ? (
            <Input
              value={editName}
              onChange={(e) => setEditName(e.target.value)}
              className="text-xl font-bold h-auto py-1"
              maxLength={100}
              autoFocus
            />
          ) : (
            <h1 className="text-2xl font-bold flex items-center gap-2">
              <BookOpen className="h-6 w-6" />
              {data.name}
            </h1>
          )}
        </div>
        <div className="flex items-center gap-2">
          {!editing && (
            <>
              <Button variant="outline" size="sm" onClick={startEditing}>
                <Pencil className="h-4 w-4 mr-1" />
                Edit
              </Button>
              {data.status === 'failed' && (
                <Button
                  variant="outline"
                  size="sm"
                  onClick={handleRetry}
                  disabled={updateMutation.isPending}
                >
                  <RefreshCw className="h-4 w-4 mr-1" />
                  Retry
                </Button>
              )}
              <Button
                variant="ghost"
                size="sm"
                className="text-destructive"
                onClick={() => setDeleteOpen(true)}
              >
                <Trash2 className="h-4 w-4 mr-1" />
                Delete
              </Button>
            </>
          )}
          {editing && (
            <>
              <Button variant="outline" size="sm" onClick={cancelEditing}>
                <X className="h-4 w-4 mr-1" />
                Cancel
              </Button>
              <Button
                size="sm"
                onClick={handleSave}
                disabled={updateMutation.isPending || !editName.trim()}
              >
                <Save className="h-4 w-4 mr-1" />
                {updateMutation.isPending ? 'Saving...' : 'Save'}
              </Button>
            </>
          )}
        </div>
      </div>

      {/* Status bar */}
      <div className="flex items-center gap-4 text-sm">
        <Badge
          variant={data.status === 'completed' ? 'default' : data.status === 'failed' ? 'destructive' : 'secondary'}
        >
          <StatusIcon className={`h-3 w-3 mr-1 ${data.status === 'processing' ? 'animate-spin' : ''}`} />
          {statusCfg.label}
        </Badge>
        <span className="text-muted-foreground">{data.entity_count} entities extracted</span>
        <span className="text-muted-foreground">v{data.version}</span>
        <span className="text-muted-foreground">
          Updated {new Date(data.updated_at).toLocaleString()}
        </span>
      </div>

      {/* Error alert */}
      {data.status === 'failed' && data.error && (
        <Card className="border-destructive/50 bg-destructive/5">
          <CardContent className="flex items-start gap-3 py-4">
            <AlertCircle className="h-5 w-5 text-destructive shrink-0 mt-0.5" />
            <div>
              <p className="font-medium text-destructive">Processing failed</p>
              <p className="text-sm text-muted-foreground mt-1 font-mono">{data.error}</p>
            </div>
          </CardContent>
        </Card>
      )}

      {/* Content */}
      <Card>
        <CardHeader>
          <CardTitle className="text-base">Content</CardTitle>
        </CardHeader>
        <CardContent>
          {editing ? (
            <div className="space-y-2">
              <Textarea
                value={editContent}
                onChange={(e) => setEditContent(e.target.value)}
                rows={16}
                className="font-mono text-sm"
              />
              <p className="text-xs text-muted-foreground">
                {editContent.length.toLocaleString()} / 102,400 characters
              </p>
            </div>
          ) : (
            <pre className="whitespace-pre-wrap font-mono text-sm text-muted-foreground bg-muted/30 rounded-md p-4 max-h-[600px] overflow-auto">
              {data.content}
            </pre>
          )}
        </CardContent>
      </Card>

      {/* Metadata */}
      <Card>
        <CardHeader>
          <CardTitle className="text-base">Details</CardTitle>
        </CardHeader>
        <CardContent>
          <dl className="grid grid-cols-2 gap-x-6 gap-y-3 text-sm md:grid-cols-4">
            <div>
              <dt className="text-muted-foreground">Source Type</dt>
              <dd className="font-medium">{data.source_type}</dd>
            </div>
            <div>
              <dt className="text-muted-foreground">Version</dt>
              <dd className="font-medium">{data.version}</dd>
            </div>
            <div>
              <dt className="text-muted-foreground">Created</dt>
              <dd className="font-medium">{new Date(data.created_at).toLocaleString()}</dd>
            </div>
            <div>
              <dt className="text-muted-foreground">Updated</dt>
              <dd className="font-medium">{new Date(data.updated_at).toLocaleString()}</dd>
            </div>
          </dl>
        </CardContent>
      </Card>

      {/* Delete Confirmation Dialog */}
      <Dialog open={deleteOpen} onOpenChange={setDeleteOpen}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>Delete &ldquo;{data.name}&rdquo;?</DialogTitle>
            <DialogDescription>
              This will remove the injection and its extracted entities from the knowledge graph.
            </DialogDescription>
          </DialogHeader>
          <DialogFooter>
            <Button variant="outline" onClick={() => setDeleteOpen(false)}>
              Cancel
            </Button>
            <Button
              variant="destructive"
              onClick={handleDelete}
              disabled={deleteMutation.isPending}
            >
              {deleteMutation.isPending ? 'Deleting...' : 'Delete'}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </div>
    </ScrollArea>
  );
}
