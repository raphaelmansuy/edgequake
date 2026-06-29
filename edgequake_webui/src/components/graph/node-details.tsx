/**
 * @module NodeDetails
 * @description Entity detail panel showing node properties and relationships.
 * Supports editing, deletion, and relationship exploration.
 * 
 * @implements UC0102 - View entity details in side panel
 * @implements UC0103 - Edit entity properties inline
 * @implements UC0105 - Delete entity with cascade confirmation
 * @implements FEAT0203 - Entity property editing
 * @implements FEAT0204 - Relationship navigation
 * 
 * @enforces BR0201 - Panel syncs with graph selection
 * @enforces BR0202 - Edit saves trigger graph refresh
 * @enforces BR0203 - Deletion shows impact analysis
 * 
 * @see {@link docs/use_cases.md} UC0102-0105
 */
'use client';

import {
    AlertDialog,
    AlertDialogAction,
    AlertDialogCancel,
    AlertDialogContent,
    AlertDialogDescription,
    AlertDialogFooter,
    AlertDialogHeader,
    AlertDialogTitle,
} from '@/components/ui/alert-dialog';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { Separator } from '@/components/ui/separator';
import {
    Tooltip,
    TooltipContent,
    TooltipProvider,
    TooltipTrigger,
} from '@/components/ui/tooltip';
import { deleteEntity } from '@/lib/api/edgequake';
import { formatEntityLabel, formatEntityType, getEntityTypeColor } from '@/lib/graph/label-utils';
import { cn } from '@/lib/utils';
import { useGraphStore } from '@/stores/use-graph-store';
import { useSelectedWorkspace } from '@/stores/use-tenant-store';
import type { GraphEdge, GraphNode } from '@/types';
import { useQueryClient } from '@tanstack/react-query';
import { formatDistanceToNow } from 'date-fns';
import {
    ArrowLeft,
    ArrowRight,
    Calendar,
    Check,
    ChevronDown,
    ChevronRight,
    Copy,
    Edit,
    ExternalLink,
    GitMerge,
    Hash,
    Info,
    Link2,
    Sparkles,
    Trash2
} from 'lucide-react';
import { useCallback, useState } from 'react';
import { toast } from 'sonner';
import { EntityEditDialog } from './entity-edit-dialog';
import { RelationshipEditDialog } from './relationship-edit-dialog';

interface NodeDetailsProps {
  node: GraphNode;
}

// Color palette for entity types - matches graph-renderer.tsx
// Expandable Property Value Component
function PropertyValue({ 
  label, 
  value, 
  copyable = true 
}: { 
  label: string; 
  value: string; 
  copyable?: boolean;
}) {
  const [isExpanded, setIsExpanded] = useState(false);
  const [copied, setCopied] = useState(false);
  
  const isLong = value.length > 20;
  const displayValue = isExpanded ? value : (isLong ? `${value.slice(0, 20)}...` : value);
  
  const handleCopy = useCallback(async () => {
    await navigator.clipboard.writeText(value);
    setCopied(true);
    toast.success('Copied to clipboard');
    setTimeout(() => setCopied(false), 2000);
  }, [value]);
  
  return (
    <div className="flex justify-between text-xs gap-2 group py-1 min-w-0">
      <span className="text-muted-foreground shrink-0 text-[11px]">{label}</span>
      <div className="flex items-center gap-1 min-w-0 flex-1 justify-end">
        <span
          className={cn(
            "font-mono text-[10px] bg-background/50 px-1.5 py-0.5 rounded transition-all",
            isLong && "cursor-pointer hover:bg-muted",
            isExpanded ? "break-all whitespace-normal" : "truncate min-w-0"
          )}
          onClick={isLong ? () => setIsExpanded(!isExpanded) : undefined}
          title={isLong ? (isExpanded ? "Click to collapse" : "Click to expand") : value}
        >
          {displayValue}
        </span>
        {isLong && (
          <Button
            variant="ghost"
            size="icon"
            className="h-4 w-4 shrink-0 opacity-60 hover:opacity-100"
            onClick={() => setIsExpanded(!isExpanded)}
          >
            {isExpanded ? <ChevronDown className="h-2.5 w-2.5" /> : <ChevronRight className="h-2.5 w-2.5" />}
          </Button>
        )}
        {copyable && (
          <Button
            variant="ghost"
            size="icon"
            className="h-5 w-5 shrink-0 opacity-0 group-hover:opacity-100 transition-opacity"
            onClick={handleCopy}
          >
            {copied ? <Check className="h-3 w-3 text-green-500" /> : <Copy className="h-3 w-3" />}
          </Button>
        )}
      </div>
    </div>
  );
}

export function NodeDetails({ node }: NodeDetailsProps) {
  const { focusNode, edges, nodes } = useGraphStore();
  const queryClient = useQueryClient();
  // FIX #174: Read workspace entity types for the edit dialog dropdown
  const workspace = useSelectedWorkspace();
  
  // Dialog states
  const [showEntityEdit, setShowEntityEdit] = useState(false);
  const [entityDialogMode, setEntityDialogMode] = useState<'edit' | 'merge'>('edit');
  const [showRelationshipEdit, setShowRelationshipEdit] = useState(false);
  const [selectedEdge, setSelectedEdge] = useState<GraphEdge | null>(null);
  // FIX #175: Delete confirmation dialog state
  const [showDeleteConfirm, setShowDeleteConfirm] = useState(false);
  const [isDeleting, setIsDeleting] = useState(false);

  const connectedEdges = edges.filter(
    (e) => e.source === node.id || e.target === node.id
  );

  // Get related nodes with their labels
  const relatedNodes = connectedEdges.map((edge) => {
    const isSource = edge.source === node.id;
    const otherNodeId = isSource ? edge.target : edge.source;
    const otherNode = nodes.find((n) => n.id === otherNodeId);
    
    return {
      edge,
      isSource,
      node: otherNode,
      nodeId: otherNodeId,
      label: otherNode?.label || otherNodeId.slice(0, 12),
      type: otherNode?.node_type || 'UNKNOWN',
    };
  });

  const handleCopyId = () => {
    navigator.clipboard.writeText(node.id);
    toast.success('Entity ID copied to clipboard');
  };

  const handleCopyLabel = () => {
    navigator.clipboard.writeText(node.label);
    toast.success('Entity label copied to clipboard');
  };

  // FIX #175: Delete entity handler
  const handleDeleteEntity = async () => {
    setIsDeleting(true);
    try {
      await deleteEntity(node.id);
      queryClient.invalidateQueries({ queryKey: ['graph'] });
      toast.success(`Entity "${node.label}" deleted`);
      setShowDeleteConfirm(false);
    } catch (err) {
      toast.error(`Failed to delete entity: ${err instanceof Error ? err.message : 'Unknown error'}`);
    } finally {
      setIsDeleting(false);
    }
  };

  const typeColor = getEntityTypeColor(node.node_type);

  return (
    <div className="space-y-2.5">
      {/* Header - Compact, no card styling */}
      <div className="flex items-start justify-between gap-2">
        <div className="space-y-1 flex-1 min-w-0">
          <div className="flex items-center gap-2">
            <div 
              className="w-2.5 h-2.5 rounded-full shrink-0 ring-2 ring-background shadow-sm"
              style={{ backgroundColor: typeColor }}
            />
            <h4 className="text-sm font-semibold truncate">
              {formatEntityLabel(node.label)}
            </h4>
            <TooltipProvider>
              <Tooltip>
                <TooltipTrigger asChild>
                  <Button
                    variant="ghost"
                    size="icon"
                    className="h-5 w-5 shrink-0 hover:bg-muted/80"
                    aria-label={`Copy label for ${node.label}`}
                    onClick={handleCopyLabel}
                  >
                    <Copy className="h-3 w-3" />
                  </Button>
                </TooltipTrigger>
                <TooltipContent>Copy label</TooltipContent>
              </Tooltip>
            </TooltipProvider>
          </div>
          <Badge 
            variant="outline" 
            className="text-[10px] font-medium px-2 py-0.5"
            style={{ borderColor: typeColor, color: typeColor, backgroundColor: `${typeColor}10` }}
          >
            {formatEntityType(node.node_type || 'ENTITY')}
          </Badge>
        </div>
      </div>
      
      {/* Content - Inherits scrolling from parent ScrollArea */}
      <div className="space-y-2.5">
            {/* Description */}
            {node.description && (
              <div className="bg-muted/30 rounded-md p-2 border border-border/30">
                <div className="flex items-center gap-1 mb-1">
                  <Info className="h-3 w-3 text-muted-foreground" />
                  <h5 className="text-[10px] font-semibold text-muted-foreground uppercase tracking-wider">
                    Description
                  </h5>
                </div>
                <p className="wrap-break-word text-xs leading-relaxed text-foreground/90">{node.description}</p>
              </div>
            )}

            {/* Properties */}
            {node.properties && Object.keys(node.properties).length > 0 && (
              <div>
                <div className="flex items-center justify-between mb-1.5">
                  <div className="flex items-center gap-1">
                    <Sparkles className="h-3 w-3 text-muted-foreground" />
                    <h5 className="text-[10px] font-semibold text-muted-foreground uppercase tracking-wider">
                      Properties
                    </h5>
                  </div>
                  <TooltipProvider>
                    <Tooltip>
                      <TooltipTrigger asChild>
                        <Button
                          variant="ghost"
                          size="sm"
                          className="h-5 text-[9px] px-1.5"
                          onClick={() => {
                            const allProps = Object.entries(node.properties || {})
                              .map(([k, v]) => `${k}: ${v}`)
                              .join('\n');
                            navigator.clipboard.writeText(allProps);
                            toast.success('All properties copied');
                          }}
                        >
                          <Copy className="h-2.5 w-2.5 mr-0.5" />
                          Copy All
                        </Button>
                      </TooltipTrigger>
                      <TooltipContent>Copy all properties</TooltipContent>
                    </Tooltip>
                  </TooltipProvider>
                </div>
                <div className="bg-muted/20 rounded-md p-2 space-y-0 border border-border/20">
                  {Object.entries(node.properties).map(([key, value]) => (
                    <PropertyValue key={key} label={key} value={String(value)} />
                  ))}
                </div>
              </div>
            )}

            {/* Metadata */}
            <div>
              <div className="flex items-center gap-1 mb-1.5">
                <Hash className="h-3 w-3 text-muted-foreground" />
                <h5 className="text-[10px] font-semibold text-muted-foreground uppercase tracking-wider">
                  Metadata
                </h5>
              </div>
              <div className="bg-muted/20 rounded-md p-2 space-y-1.5 border border-border/20">
                <div className="flex items-center justify-between gap-2 text-xs">
                  <span className="text-muted-foreground flex items-center gap-1">
                    <Hash className="h-2.5 w-2.5" /> ID
                  </span>
                  <div className="flex items-center gap-1">
                    <span className="font-mono text-[9px] bg-background/50 px-1.5 py-0.5 rounded">
                      {node.id.slice(0, 10)}...
                    </span>
                    <Button
                      variant="ghost"
                      size="icon"
                      className="h-4 w-4"
                      aria-label={`Copy entity ID for ${node.label}`}
                      onClick={handleCopyId}
                    >
                      <Copy className="h-2.5 w-2.5" />
                    </Button>
                  </div>
                </div>
                {node.degree !== undefined && (
                  <div className="flex items-center justify-between text-xs">
                    <span className="text-muted-foreground flex items-center gap-1">
                      <Link2 className="h-2.5 w-2.5" /> Connections
                    </span>
                    <Badge variant="secondary" className="h-4 text-[9px] font-semibold px-1.5">
                      {node.degree}
                    </Badge>
                  </div>
                )}
                {node.created_at && (
                  <div className="flex items-center justify-between text-xs">
                    <span className="text-muted-foreground flex items-center gap-1">
                      <Calendar className="h-2.5 w-2.5" /> Created
                    </span>
                    <span className="text-[10px] font-medium">
                      {formatDistanceToNow(new Date(node.created_at), { addSuffix: true })}
                    </span>
                  </div>
                )}
              </div>
            </div>

            <Separator className="my-1" />

            {/* Relationships */}
            <div>
              <div className="flex items-center justify-between mb-1.5">
                <div className="flex items-center gap-1">
                  <Link2 className="h-3 w-3 text-muted-foreground" />
                  <h5 className="text-[10px] font-semibold text-muted-foreground uppercase tracking-wider">
                    Relationships
                  </h5>
                </div>
                <Badge variant="outline" className="h-4 text-[9px] font-semibold px-1.5">
                  {connectedEdges.length}
                </Badge>
              </div>
              <div className="bg-muted/20 rounded-md border border-border/20 overflow-hidden">
                <div className="max-h-40 overflow-y-auto">
                  <div className="p-1.5 space-y-0.5">
                    {relatedNodes.length === 0 ? (
                      <p className="text-[10px] text-muted-foreground text-center py-4">
                        No connections found
                      </p>
                    ) : (
                      relatedNodes.map(({ edge, isSource, nodeId, label, type }, index) => {
                        const relationColor = getEntityTypeColor(type);
                        
                        return (
                          <div
                            key={edge.id || `edge-${index}`}
                            className="flex items-center gap-1.5 text-[10px] cursor-pointer hover:bg-muted/50 p-1.5 rounded-md transition-all group"
                          >
                            <div className="flex items-center shrink-0">
                              {isSource ? (
                                <div className="flex items-center gap-0.5 text-blue-500">
                                  <ArrowRight className="h-2.5 w-2.5" />
                                </div>
                              ) : (
                                <div className="flex items-center gap-0.5 text-green-500">
                                  <ArrowLeft className="h-2.5 w-2.5" />
                                </div>
                              )}
                            </div>
                            <button
                              type="button"
                              className="h-4 max-w-17.5 shrink-0 truncate rounded-md bg-secondary px-1 text-[8px] font-normal text-secondary-foreground transition-colors hover:bg-secondary/80 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring"
                              aria-label={`Open relationship ${edge.relationship_type}`}
                              title={`Open relationship ${edge.relationship_type}`}
                              onClick={(e) => {
                                e.stopPropagation();
                                setSelectedEdge(edge);
                                setShowRelationshipEdit(true);
                              }}
                            >
                              {edge.relationship_type}
                            </button>
                            <div 
                              className="flex items-center gap-1 flex-1 min-w-0"
                              onClick={() => focusNode(nodeId)}
                            >
                              <div 
                                className="w-1.5 h-1.5 rounded-full shrink-0"
                                style={{ backgroundColor: relationColor }}
                              />
                              <span className="truncate group-hover:underline font-medium">{label}</span>
                            </div>
                            <button
                              type="button"
                              className="shrink-0 rounded p-0.5 text-muted-foreground opacity-0 transition-opacity group-hover:opacity-100 group-focus-within:opacity-100 focus-visible:opacity-100"
                              aria-label={`Focus entity ${label}`}
                              title={`Focus entity ${label}`}
                              onClick={() => focusNode(nodeId)}
                            >
                              <ExternalLink className="h-2.5 w-2.5" />
                            </button>
                          </div>
                        );
                      })
                    )}
                  </div>
                </div>
              </div>
            </div>

        <Separator className="my-1" />

        {/* Actions - Accessible and responsive */}
        <div className="grid grid-cols-1 gap-2 pt-1 sm:grid-cols-3" role="group" aria-label={`Entity actions for ${node.label}`}>
          <TooltipProvider>
            <Tooltip>
              <TooltipTrigger asChild>
                <Button 
                  variant="outline" 
                  size="sm" 
                  className="h-8 w-full text-xs font-medium hover:border-primary/50 hover:bg-primary/10"
                  aria-label={`Edit entity ${node.label}`}
                  title={`Edit entity ${node.label}`}
                  onClick={() => {
                    setEntityDialogMode('edit');
                    setShowEntityEdit(true);
                  }}
                >
                  <Edit className="h-3 w-3 mr-1" />
                  Edit
                </Button>
              </TooltipTrigger>
              <TooltipContent>Edit entity details</TooltipContent>
            </Tooltip>
          </TooltipProvider>
          <TooltipProvider>
            <Tooltip>
              <TooltipTrigger asChild>
                <Button 
                  variant="outline" 
                  size="sm" 
                  className="h-8 w-full text-xs font-medium hover:border-purple-500/50 hover:bg-purple-500/10"
                  aria-label={`Merge entity ${node.label}`}
                  title={`Merge entity ${node.label}`}
                  onClick={() => {
                    setEntityDialogMode('merge');
                    setShowEntityEdit(true);
                  }}
                >
                  <GitMerge className="h-3 w-3 mr-1" />
                  Merge
                </Button>
              </TooltipTrigger>
              <TooltipContent>Merge with another entity</TooltipContent>
            </Tooltip>
          </TooltipProvider>
          <TooltipProvider>
            <Tooltip>
              <TooltipTrigger asChild>
                <Button 
                  variant="outline" 
                  size="sm" 
                  className="h-8 w-full text-xs font-medium text-destructive hover:border-destructive/50 hover:bg-destructive/10"
                  aria-label={`Delete entity ${node.label}`}
                  title={`Delete entity ${node.label}`}
                  onClick={() => setShowDeleteConfirm(true)}
                >
                  <Trash2 className="h-3 w-3 mr-1" />
                  Delete
                </Button>
              </TooltipTrigger>
              <TooltipContent>Delete this entity</TooltipContent>
            </Tooltip>
          </TooltipProvider>
        </div>
      </div>

      {/* Entity Edit Dialog */}
      <EntityEditDialog
        open={showEntityEdit}
        onOpenChange={setShowEntityEdit}
        node={node}
        mode={entityDialogMode}
        otherEntities={nodes
          .filter((candidate) => candidate.id !== node.id)
          .map((candidate) => ({
            id: candidate.id,
            label: candidate.label,
            entity_type: candidate.node_type || 'UNKNOWN',
          }))}
        workspaceEntityTypes={workspace?.entity_types}
        onUpdated={() => {
          queryClient.invalidateQueries({ queryKey: ['graph'] });
          toast.success(
            entityDialogMode === 'merge'
              ? 'Entity merged successfully'
              : 'Entity updated successfully'
          );
        }}
      />

      {/* Relationship Edit Dialog */}
      {selectedEdge && (
        <RelationshipEditDialog
          open={showRelationshipEdit}
          onOpenChange={(open) => {
            setShowRelationshipEdit(open);
            if (!open) setSelectedEdge(null);
          }}
          edge={selectedEdge}
          onUpdated={() => {
            queryClient.invalidateQueries({ queryKey: ['graph'] });
            toast.success('Relationship updated successfully');
          }}
        />
      )}
      {/* FIX #175: Delete Confirmation Dialog */}
      <AlertDialog open={showDeleteConfirm} onOpenChange={setShowDeleteConfirm}>
        <AlertDialogContent>
          <AlertDialogHeader>
            <AlertDialogTitle>Delete Entity</AlertDialogTitle>
            <AlertDialogDescription>
              Are you sure you want to delete &quot;{node.label}&quot;? This will also remove all relationships connected to this entity. This action cannot be undone.
            </AlertDialogDescription>
          </AlertDialogHeader>
          <AlertDialogFooter>
            <AlertDialogCancel autoFocus disabled={isDeleting}>Cancel</AlertDialogCancel>
            <AlertDialogAction
              onClick={handleDeleteEntity}
              disabled={isDeleting}
              className="bg-destructive text-destructive-foreground hover:bg-destructive/90"
            >
              {isDeleting ? 'Deleting...' : 'Delete'}
            </AlertDialogAction>
          </AlertDialogFooter>
        </AlertDialogContent>
      </AlertDialog>
    </div>
  );
}
export default NodeDetails;
