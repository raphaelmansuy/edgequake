'use client';

/**
 * Knowledge Injection Page (SPEC-0002)
 *
 * Allows users to inject domain glossaries, acronyms, and definitions
 * to enrich the knowledge graph without polluting query citations.
 */

import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import {
    Dialog,
    DialogContent,
    DialogDescription,
    DialogFooter,
    DialogHeader,
    DialogTitle,
    DialogTrigger,
} from '@/components/ui/dialog';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Skeleton } from '@/components/ui/skeleton';
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs';
import { Textarea } from '@/components/ui/textarea';
import {
    injectionKeys,
    useCreateInjection,
    useCreateInjectionFile,
    useDeleteInjection,
    useInjections,
} from '@/hooks';
import useTenantContext from '@/hooks/use-tenant-context';
import { getInjection } from '@/lib/api/edgequake';
import { useQueryClient } from '@tanstack/react-query';
import { AlertCircle, BookOpen, FileText, Plus, Trash2, Upload } from 'lucide-react';
import Link from 'next/link';
import { useRef, useState } from 'react';
import { toast } from 'sonner';
import { ScrollArea } from '@/components/ui/scroll-area';

/** Poll until status !== 'processing', show entity count toast, then call onDone. */
async function pollForCompletion(
  workspaceId: string,
  injectionId: string,
  injectionName: string,
  onDone?: () => void,
) {
  const MAX_POLLS = 60; // 5 min max (60 × 5s)
  for (let i = 0; i < MAX_POLLS; i++) {
    await new Promise((r) => setTimeout(r, 5000));
    try {
      const detail = await getInjection(workspaceId, injectionId);
      if (detail.status === 'completed') {
        toast.success(
          `${detail.entity_count} entities extracted from "${injectionName}"`,
        );
        onDone?.();
        return;
      }
      if (detail.status === 'failed') {
        toast.error(`Processing failed: ${detail.error ?? 'Unknown error'}`);
        onDone?.();
        return;
      }
    } catch {
      // ignore transient errors
    }
  }
}

export default function KnowledgePage() {
  const { selectedWorkspaceId } = useTenantContext();
  const { data, isLoading } = useInjections(selectedWorkspaceId);
  const queryClient = useQueryClient();
  const createMutation = useCreateInjection(selectedWorkspaceId ?? '');
  const createFileMutation = useCreateInjectionFile(selectedWorkspaceId ?? '');
  const deleteMutation = useDeleteInjection(selectedWorkspaceId ?? '');

  const [dialogOpen, setDialogOpen] = useState(false);
  const [name, setName] = useState('');
  const [content, setContent] = useState('');

  // File upload state
  const [fileName, setFileName] = useState('');
  const [selectedFile, setSelectedFile] = useState<File | null>(null);
  const fileInputRef = useRef<HTMLInputElement>(null);

  const [deleteTarget, setDeleteTarget] = useState<string | null>(null);

  const resetDialog = () => {
    setName('');
    setContent('');
    setFileName('');
    setSelectedFile(null);
    if (fileInputRef.current) fileInputRef.current.value = '';
  };

  const handleCreate = async () => {
    if (!name.trim() || !content.trim()) {
      toast.error('Name and content are required');
      return;
    }
    try {
      const result = await createMutation.mutateAsync({ name: name.trim(), content: content.trim() });
      toast.success('Processing started — entities will be extracted shortly');
      setDialogOpen(false);
      resetDialog();
      const wsId = selectedWorkspaceId ?? 'default';
      void pollForCompletion(wsId, result.injection_id, name.trim(), () => {
        void queryClient.invalidateQueries({ queryKey: injectionKeys.list(wsId) });
      });
    } catch (err) {
      toast.error(`Failed to create injection: ${err instanceof Error ? err.message : 'Unknown error'}`);
    }
  };

  const handleCreateFile = async () => {
    if (!selectedFile) {
      toast.error('Please select a file');
      return;
    }
    try {
      const fileNameForInjection = fileName.trim() || selectedFile.name.replace(/\.[^.]+$/, '');
      const result = await createFileMutation.mutateAsync({
        name: fileNameForInjection,
        file: selectedFile,
      });
      toast.success('Processing started — entities will be extracted shortly');
      setDialogOpen(false);
      resetDialog();
      const wsId = selectedWorkspaceId ?? 'default';
      void pollForCompletion(wsId, result.injection_id, fileNameForInjection, () => {
        void queryClient.invalidateQueries({ queryKey: injectionKeys.list(wsId) });
      });
    } catch (err) {
      toast.error(`Failed to upload file: ${err instanceof Error ? err.message : 'Unknown error'}`);
    }
  };

  const handleDelete = async (injectionId: string) => {
    try {
      await deleteMutation.mutateAsync(injectionId);
      toast.success('Injection deleted');
      setDeleteTarget(null);
    } catch (err) {
      toast.error(`Failed to delete: ${err instanceof Error ? err.message : 'Unknown error'}`);
    }
  };

  const statusColor = (status: string) => {
    switch (status) {
      case 'completed': return 'default';
      case 'processing': return 'secondary';
      case 'failed': return 'destructive';
      default: return 'outline';
    }
  };

  return (
    <ScrollArea className="h-full">
      <div className="flex flex-col gap-6 p-6">
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-2xl font-bold flex items-center gap-2">
            <BookOpen className="h-6 w-6" />
            Knowledge Injection
          </h1>
          <p className="text-muted-foreground mt-1">
            Inject domain glossaries, acronyms, and definitions to enrich your knowledge graph.
          </p>
        </div>

        <Dialog open={dialogOpen} onOpenChange={setDialogOpen}>
          <DialogTrigger asChild>
            <Button>
              <Plus className="h-4 w-4 mr-2" />
              New Injection
            </Button>
          </DialogTrigger>
          <DialogContent className="max-w-2xl">
            <DialogHeader>
              <DialogTitle>Create Knowledge Injection</DialogTitle>
              <DialogDescription>
                Add domain context to enrich the knowledge graph.
                Injected knowledge improves retrieval but never appears as a source citation.
              </DialogDescription>
            </DialogHeader>

            <Tabs defaultValue="text" className="mt-2">
              <TabsList className="w-full">
                <TabsTrigger value="text" className="flex-1 gap-2">
                  <FileText className="h-4 w-4" />
                  Text
                </TabsTrigger>
                <TabsTrigger value="file" className="flex-1 gap-2">
                  <Upload className="h-4 w-4" />
                  File
                </TabsTrigger>
              </TabsList>

              {/* ── Text tab ── */}
              <TabsContent value="text" className="space-y-4 py-4">
                <div className="space-y-2">
                  <Label htmlFor="injection-name">Name</Label>
                  <Input
                    id="injection-name"
                    placeholder="e.g., Manufacturing Glossary"
                    value={name}
                    onChange={(e) => setName(e.target.value)}
                    maxLength={100}
                  />
                </div>
                <div className="space-y-2">
                  <Label htmlFor="injection-content">Content</Label>
                  <Textarea
                    id="injection-content"
                    placeholder="OEE: Overall Equipment Effectiveness, a measure of manufacturing productivity&#10;MTBF: Mean Time Between Failures&#10;..."
                    value={content}
                    onChange={(e) => setContent(e.target.value)}
                    rows={12}
                    className="font-mono text-sm"
                  />
                  <p className="text-xs text-muted-foreground">
                    {content.length.toLocaleString()} / 102,400 characters
                  </p>
                </div>
                <DialogFooter>
                  <Button variant="outline" onClick={() => { setDialogOpen(false); resetDialog(); }}>
                    Cancel
                  </Button>
                  <Button
                    onClick={handleCreate}
                    disabled={createMutation.isPending || !name.trim() || !content.trim()}
                  >
                    {createMutation.isPending ? 'Creating...' : 'Create'}
                  </Button>
                </DialogFooter>
              </TabsContent>

              {/* ── File tab ── */}
              <TabsContent value="file" className="space-y-4 py-4">
                <div className="space-y-2">
                  <Label htmlFor="file-injection-name">Name (optional)</Label>
                  <Input
                    id="file-injection-name"
                    placeholder="Defaults to filename"
                    value={fileName}
                    onChange={(e) => setFileName(e.target.value)}
                    maxLength={100}
                  />
                </div>
                <div className="space-y-2">
                  <Label>File</Label>
                  <div
                    className="flex flex-col items-center justify-center border-2 border-dashed rounded-lg p-8 gap-3 cursor-pointer hover:border-primary/50 transition-colors"
                    onClick={() => fileInputRef.current?.click()}
                  >
                    {selectedFile ? (
                      <>
                        <FileText className="h-8 w-8 text-primary" />
                        <p className="text-sm font-medium">{selectedFile.name}</p>
                        <p className="text-xs text-muted-foreground">
                          {(selectedFile.size / 1024).toFixed(1)} KB
                        </p>
                      </>
                    ) : (
                      <>
                        <Upload className="h-8 w-8 text-muted-foreground" />
                        <p className="text-sm text-muted-foreground">
                          Click to select a file
                        </p>
                        <p className="text-xs text-muted-foreground">
                          Supported: .txt, .md, .csv, .json — max 10 MB
                        </p>
                      </>
                    )}
                  </div>
                  <input
                    ref={fileInputRef}
                    type="file"
                    className="hidden"
                    accept=".txt,.md,.csv,.json"
                    onChange={(e) => {
                      const file = e.target.files?.[0] ?? null;
                      if (file) {
                        const ext = file.name.split('.').pop()?.toLowerCase() ?? '';
                        if (!['txt', 'md', 'csv', 'json'].includes(ext)) {
                          toast.error(`Unsupported file type ".${ext}". Allowed: .txt, .md, .csv, .json`);
                          e.target.value = '';
                          return;
                        }
                        if (file.size > 10 * 1024 * 1024) {
                          toast.error('File too large. Maximum size is 10 MB');
                          e.target.value = '';
                          return;
                        }
                      }
                      setSelectedFile(file);
                    }}
                  />
                </div>
                <DialogFooter>
                  <Button variant="outline" onClick={() => { setDialogOpen(false); resetDialog(); }}>
                    Cancel
                  </Button>
                  <Button
                    onClick={handleCreateFile}
                    disabled={createFileMutation.isPending || !selectedFile}
                  >
                    {createFileMutation.isPending ? 'Uploading...' : 'Upload'}
                  </Button>
                </DialogFooter>
              </TabsContent>
            </Tabs>
          </DialogContent>
        </Dialog>
      </div>

      {isLoading ? (
        <div className="grid gap-4 md:grid-cols-2 lg:grid-cols-3">
          {[1, 2, 3].map((i) => (
            <Card key={i}>
              <CardHeader>
                <Skeleton className="h-5 w-40" />
                <Skeleton className="h-4 w-24" />
              </CardHeader>
              <CardContent>
                <Skeleton className="h-4 w-full" />
              </CardContent>
            </Card>
          ))}
        </div>
      ) : !data?.items?.length ? (
        <Card className="border-dashed">
          <CardContent className="flex flex-col items-center justify-center py-12">
            <BookOpen className="h-12 w-12 text-muted-foreground mb-4" />
            <h3 className="text-lg font-semibold mb-1">No knowledge injections yet</h3>
            <p className="text-muted-foreground text-center max-w-md">
              Inject domain-specific glossaries and definitions to improve search quality.
              Injected knowledge enriches the graph but won&apos;t appear in citations.
            </p>
            <Button className="mt-4" onClick={() => setDialogOpen(true)}>
              <Plus className="h-4 w-4 mr-2" />
              Create your first injection
            </Button>
          </CardContent>
        </Card>
      ) : (
        <div className="grid gap-4 md:grid-cols-2 lg:grid-cols-3">
          {data.items.map((item) => (
            <Link key={item.injection_id} href={`/knowledge/${item.injection_id}`}>
              <Card className="cursor-pointer hover:border-primary/50 transition-colors">
                <CardHeader className="pb-3">
                  <div className="flex items-start justify-between">
                    <CardTitle className="text-base">{item.name}</CardTitle>
                    <Badge variant={statusColor(item.status)}>{item.status}</Badge>
                  </div>
                  <CardDescription className="text-xs">
                    {item.entity_count} entities &middot; {item.source_type}
                  </CardDescription>
                </CardHeader>
                <CardContent>
                  {item.status === 'failed' && item.error && (
                    <div className="flex items-start gap-2 mb-3 text-xs text-destructive">
                      <AlertCircle className="h-3.5 w-3.5 shrink-0 mt-0.5" />
                      <span className="line-clamp-2">{item.error}</span>
                    </div>
                  )}
                  <div className="flex items-center justify-between text-xs text-muted-foreground">
                    <span>
                      {new Date(item.created_at).toLocaleDateString()}
                    </span>
                    <Dialog
                      open={deleteTarget === item.injection_id}
                      onOpenChange={(open) => setDeleteTarget(open ? item.injection_id : null)}
                    >
                      <DialogTrigger asChild>
                        <Button
                          variant="ghost"
                          size="icon"
                          className="h-7 w-7"
                          onClick={(e) => e.stopPropagation()}
                        >
                          <Trash2 className="h-3.5 w-3.5 text-destructive" />
                        </Button>
                      </DialogTrigger>
                      <DialogContent onClick={(e) => e.stopPropagation()}>
                        <DialogHeader>
                          <DialogTitle>Delete &ldquo;{item.name}&rdquo;?</DialogTitle>
                          <DialogDescription>
                            This will remove the injection and its extracted entities from the knowledge graph.
                          </DialogDescription>
                        </DialogHeader>
                        <DialogFooter>
                          <Button variant="outline" onClick={() => setDeleteTarget(null)}>
                            Cancel
                          </Button>
                          <Button
                            variant="destructive"
                            onClick={() => handleDelete(item.injection_id)}
                            disabled={deleteMutation.isPending}
                          >
                            {deleteMutation.isPending ? 'Deleting...' : 'Delete'}
                          </Button>
                        </DialogFooter>
                      </DialogContent>
                    </Dialog>
                  </div>
                </CardContent>
              </Card>
            </Link>
          ))}
        </div>
      )}
    </div>
    </ScrollArea>
  );
}
