'use client';

import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { ScrollArea } from '@/components/ui/scroll-area';
import { Separator } from '@/components/ui/separator';
import {
    Tooltip,
    TooltipContent,
    TooltipProvider,
    TooltipTrigger,
} from '@/components/ui/tooltip';
import { useGraphStore } from '@/stores/use-graph-store';
import type { GraphEdge, GraphNode } from '@/types';
import { useQueryClient } from '@tanstack/react-query';
import { formatDistanceToNow } from 'date-fns';
import {
    ArrowLeft,
    ArrowRight,
    Calendar,
    Copy,
    Edit,
    ExternalLink,
    GitMerge,
    Hash,
    Info,
    Link2,
    Sparkles,
    Trash2,
    X
} from 'lucide-react';
import { useState } from 'react';
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

export function NodeDetails({ node }: NodeDetailsProps) {
  const { selectNode, focusNode, edges, nodes, toggleNodeDetails } = useGraphStore();
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
    <Card className="shadow-lg">
      <CardHeader className="pb-2">
        <div className="flex items-start justify-between gap-2">
          <div className="space-y-1.5 flex-1 min-w-0">
            <div className="flex items-center gap-2">
              <div 
                className="w-3 h-3 rounded-full shrink-0"
                style={{ backgroundColor: typeColor }}
              />
              <CardTitle className="text-base font-semibold truncate">
                {node.label}
              </CardTitle>
              <TooltipProvider>
                <Tooltip>
                  <TooltipTrigger asChild>
                    <Button
                      variant="ghost"
                      size="icon"
                      className="h-5 w-5 shrink-0"
                      onClick={handleCopyLabel}
                    >
                      <Copy className="h-3 w-3" />
                    </Button>
                  </TooltipTrigger>
                  <TooltipContent>Copy label</TooltipContent>
                </Tooltip>
              </TooltipProvider>
              <TooltipProvider>
                <Tooltip>
                  <TooltipTrigger asChild>
                    <Button
                      variant="ghost"
                      size="icon"
                      className="h-5 w-5 shrink-0"
                      onClick={() => setShowEntityEdit(true)}
                    >
                      <Edit className="h-3 w-3" />
                    </Button>
                  </TooltipTrigger>
                  <TooltipContent>Edit entity</TooltipContent>
                </Tooltip>
              </TooltipProvider>
            </div>
            <Badge 
              variant="outline" 
              className="text-[10px] font-medium"
              style={{ borderColor: typeColor, color: typeColor }}
            >
              {node.node_type || 'ENTITY'}
            </Badge>
          </div>
          <Button
            variant="ghost"
            size="icon"
            className="h-6 w-6 shrink-0"
            onClick={() => toggleNodeDetails()}
          >
            <X className="h-4 w-4" />
          </Button>
        </div>
      </CardHeader>
      
      <CardContent className="space-y-3 pt-0">
        {/* Description */}
        {node.description && (
          <div className="bg-muted/50 rounded-md p-2.5">
            <div className="flex items-center gap-1.5 mb-1">
              <Info className="h-3 w-3 text-muted-foreground" />
              <h4 className="text-[10px] font-medium text-muted-foreground uppercase tracking-wider">
                Description
              </h4>
            </div>
            <p className="text-xs leading-relaxed">{node.description}</p>
          </div>
        )}

        {/* Properties */}
        {node.properties && Object.keys(node.properties).length > 0 && (
          <div>
            <div className="flex items-center gap-1.5 mb-2">
              <Sparkles className="h-3 w-3 text-muted-foreground" />
              <h4 className="text-[10px] font-medium text-muted-foreground uppercase tracking-wider">
                Properties
              </h4>
            </div>
            <div className="space-y-1 bg-muted/30 rounded-md p-2">
              {Object.entries(node.properties).map(([key, value]) => (
                <div key={key} className="flex justify-between text-xs gap-2">
                  <span className="text-muted-foreground shrink-0">{key}</span>
                  <span className="truncate font-medium text-right">{String(value)}</span>
                </div>
              ))}
            </div>
          </div>
        )}

        {/* Metadata */}
        <div>
          <div className="flex items-center gap-1.5 mb-2">
            <Hash className="h-3 w-3 text-muted-foreground" />
            <h4 className="text-[10px] font-medium text-muted-foreground uppercase tracking-wider">
              Metadata
            </h4>
          </div>
          <div className="space-y-1.5 text-xs bg-muted/30 rounded-md p-2">
            <div className="flex items-center justify-between gap-2">
              <span className="text-muted-foreground flex items-center gap-1">
                <Hash className="h-3 w-3" /> ID
              </span>
              <div className="flex items-center gap-1">
                <span className="font-mono text-[10px] bg-background px-1.5 py-0.5 rounded">
                  {node.id.slice(0, 12)}...
                </span>
                <Button
                  variant="ghost"
                  size="icon"
                  className="h-5 w-5"
                  onClick={handleCopyId}
                >
                  <Copy className="h-3 w-3" />
                </Button>
              </div>
            </div>
            {node.degree !== undefined && (
              <div className="flex items-center justify-between">
                <span className="text-muted-foreground flex items-center gap-1">
                  <Link2 className="h-3 w-3" /> Connections
                </span>
                <Badge variant="secondary" className="h-5 text-[10px]">
                  {node.degree}
                </Badge>
              </div>
            )}
            {node.created_at && (
              <div className="flex items-center justify-between">
                <span className="text-muted-foreground flex items-center gap-1">
                  <Calendar className="h-3 w-3" /> Created
                </span>
                <span className="text-[10px]">
                  {formatDistanceToNow(new Date(node.created_at), { addSuffix: true })}
                </span>
              </div>
            )}
          </div>
        </div>

        <Separator />

        {/* Relationships */}
        <div>
          <div className="flex items-center justify-between mb-2">
            <div className="flex items-center gap-1.5">
              <Link2 className="h-3 w-3 text-muted-foreground" />
              <h4 className="text-[10px] font-medium text-muted-foreground uppercase tracking-wider">
                Relationships
              </h4>
            </div>
            <Badge variant="outline" className="h-4 text-[9px]">
              {connectedEdges.length}
            </Badge>
          </div>
          <ScrollArea className="h-36">
            <div className="space-y-1">
              {relatedNodes.length === 0 ? (
                <p className="text-xs text-muted-foreground text-center py-4">
                  No connections
                </p>
              ) : (
                relatedNodes.map(({ edge, isSource, node: relatedNode, nodeId, label, type }, index) => {
                  const relationColor = TYPE_COLORS[type.toUpperCase()] || TYPE_COLORS.DEFAULT;
                  
                  return (
                    <div
                      key={edge.id || `edge-${index}`}
                      className="flex items-center gap-2 text-xs cursor-pointer hover:bg-muted p-1.5 rounded-md transition-colors group"
                    >
                      <div className="flex items-center shrink-0">
                        {isSource ? (
                          <ArrowRight className="h-3 w-3 text-blue-500" />
                        ) : (
                          <ArrowLeft className="h-3 w-3 text-green-500" />
                        )}
                      </div>
                      <Badge 
                        variant="secondary" 
                        className="text-[9px] font-normal shrink-0 max-w-[80px] truncate cursor-pointer hover:bg-secondary/80"
                        onClick={(e) => {
                          e.stopPropagation();
                          setSelectedEdge(edge);
                          setShowRelationshipEdit(true);
                        }}
                        title="Click to edit relationship"
                      >
                        {edge.relationship_type}
                      </Badge>
                      <div 
                        className="flex items-center gap-1.5 flex-1 min-w-0"
                        onClick={() => focusNode(nodeId)}
                      >
                        <div 
                          className="w-2 h-2 rounded-full shrink-0"
                          style={{ backgroundColor: relationColor }}
                        />
                        <span className="truncate group-hover:underline">{label}</span>
                      </div>
                      <Button
                        variant="ghost"
                        size="icon"
                        className="h-5 w-5 opacity-0 group-hover:opacity-100 shrink-0"
                        onClick={(e) => {
                          e.stopPropagation();
                          setSelectedEdge(edge);
                          setShowRelationshipEdit(true);
                        }}
                        title="Edit relationship"
                      >
                        <Edit className="h-3 w-3" />
                      </Button>
                      <ExternalLink 
                        className="h-3 w-3 text-muted-foreground opacity-0 group-hover:opacity-100 shrink-0 cursor-pointer" 
                        onClick={() => focusNode(nodeId)}
                      />
                    </div>
                  );
                })
              )}
            </div>
          </ScrollArea>
        </div>

        <Separator />

        {/* Actions */}
        <div className="flex gap-2">
          <TooltipProvider>
            <Tooltip>
              <TooltipTrigger asChild>
                <Button 
                  variant="outline" 
                  size="sm" 
                  className="flex-1 h-8"
                  onClick={() => setShowEntityEdit(true)}
                >
                  <Edit className="h-3.5 w-3.5 mr-1.5" />
                  Edit
                </Button>
              </TooltipTrigger>
              <TooltipContent>Edit entity details</TooltipContent>
            </Tooltip>
          </TooltipProvider>
          <TooltipProvider>
            <Tooltip>
              <TooltipTrigger asChild>
                <Button variant="outline" size="sm" className="flex-1 h-8">
                  <GitMerge className="h-3.5 w-3.5 mr-1.5" />
                  Merge
                </Button>
              </TooltipTrigger>
              <TooltipContent>Merge with another entity</TooltipContent>
            </Tooltip>
          </TooltipProvider>
          <TooltipProvider>
            <Tooltip>
              <TooltipTrigger asChild>
                <Button variant="outline" size="sm" className="flex-1 h-8 text-destructive hover:text-destructive">
                  <Trash2 className="h-3.5 w-3.5 mr-1.5" />
                  Delete
                </Button>
              </TooltipTrigger>
              <TooltipContent>Delete this entity</TooltipContent>
            </Tooltip>
          </TooltipProvider>
        </div>
      </CardContent>

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
    </Card>
  );
}

export default NodeDetails;
