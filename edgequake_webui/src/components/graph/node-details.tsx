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

import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { Separator } from '@/components/ui/separator';
import {
    Tooltip,
    TooltipContent,
    TooltipProvider,
    TooltipTrigger,
} from '@/components/ui/tooltip';
import { cn } from '@/lib/utils';
import { useGraphStore } from '@/stores/use-graph-store';
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
const TYPE_COLORS: Record<string, string> = {
  PERSON: '#3b82f6',
  ORGANIZATION: '#10b981',
  LOCATION: '#f59e0b',
  EVENT: '#ef4444',
  CONCEPT: '#8b5cf6',
  DOCUMENT: '#6366f1',
  DEFAULT: '#64748b',
};

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
  const { selectNode, focusNode, edges, nodes } = useGraphStore();
  const queryClient = useQueryClient();
  
  // Dialog states
  const [showEntityEdit, setShowEntityEdit] = useState(false);
  const [showRelationshipEdit, setShowRelationshipEdit] = useState(false);
  const [selectedEdge, setSelectedEdge] = useState<GraphEdge | null>(null);

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

  const typeColor = TYPE_COLORS[node.node_type?.toUpperCase()] || TYPE_COLORS.DEFAULT;

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
              {node.label}
            </h4>
            <TooltipProvider>
              <Tooltip>
                <TooltipTrigger asChild>
                  <Button
                    variant="ghost"
                    size="icon"
                    className="h-5 w-5 shrink-0 hover:bg-muted/80"
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
            {node.node_type || 'ENTITY'}
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
                <p className="text-xs leading-relaxed text-foreground/90 break-words">{node.description}</p>
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
                <div className="max-h-[160px] overflow-y-auto">
                  <div className="p-1.5 space-y-0.5">
                    {relatedNodes.length === 0 ? (
                      <p className="text-[10px] text-muted-foreground text-center py-4">
                        No connections found
                      </p>
                    ) : (
                      relatedNodes.map(({ edge, isSource, node: relatedNode, nodeId, label, type }, index) => {
                        const relationColor = TYPE_COLORS[type.toUpperCase()] || TYPE_COLORS.DEFAULT;
                        
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
                            <Badge 
                              variant="secondary" 
                              className="text-[8px] font-normal shrink-0 max-w-[70px] truncate px-1 h-4 cursor-pointer hover:bg-secondary/80"
                              onClick={(e) => {
                                e.stopPropagation();
                                setSelectedEdge(edge);
                                setShowRelationshipEdit(true);
                              }}
                            >
                              {edge.relationship_type}
                            </Badge>
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
                            <ExternalLink 
                              className="h-2.5 w-2.5 text-muted-foreground opacity-0 group-hover:opacity-100 shrink-0 cursor-pointer transition-opacity" 
                              onClick={() => focusNode(nodeId)}
                            />
                          </div>
                        );
                      })
                    )}
                  </div>
                </div>
              </div>
            </div>

        <Separator className="my-1" />

        {/* Actions - More compact */}
        <div className="flex gap-1.5 pt-1">
          <TooltipProvider>
            <Tooltip>
              <TooltipTrigger asChild>
                <Button 
                  variant="outline" 
                  size="sm" 
                  className="flex-1 h-7 text-[10px] font-medium hover:bg-primary/10 hover:border-primary/50"
                  onClick={() => setShowEntityEdit(true)}
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
                  className="flex-1 h-7 text-[10px] font-medium hover:bg-purple-500/10 hover:border-purple-500/50"
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
                  className="flex-1 h-7 text-[10px] font-medium text-destructive hover:bg-destructive/10 hover:border-destructive/50"
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
        onUpdated={() => {
          queryClient.invalidateQueries({ queryKey: ['graph'] });
          toast.success('Entity updated successfully');
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
    </div>
  );
}
export default NodeDetails;