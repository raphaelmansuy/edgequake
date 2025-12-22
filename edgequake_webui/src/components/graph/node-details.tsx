'use client';

import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { ScrollArea } from '@/components/ui/scroll-area';
import { Separator } from '@/components/ui/separator';
import { useGraphStore } from '@/stores/use-graph-store';
import type { GraphNode } from '@/types';
import { formatDistanceToNow } from 'date-fns';
import { GitMerge, Trash2, X } from 'lucide-react';

interface NodeDetailsProps {
  node: GraphNode;
}

export function NodeDetails({ node }: NodeDetailsProps) {
  const { selectNode, focusNode, edges } = useGraphStore();

  const connectedEdges = edges.filter(
    (e) => e.source === node.id || e.target === node.id
  );

  const relatedNodes = new Set(
    connectedEdges.map((e) => (e.source === node.id ? e.target : e.source))
  );

  return (
    <Card>
      <CardHeader className="pb-3">
        <div className="flex items-start justify-between">
          <div className="space-y-1">
            <CardTitle className="text-base font-semibold">{node.label}</CardTitle>
            <Badge variant="outline">{node.node_type}</Badge>
          </div>
          <Button
            variant="ghost"
            size="icon"
            className="h-6 w-6"
            onClick={() => selectNode(null)}
          >
            <X className="h-4 w-4" />
          </Button>
        </div>
      </CardHeader>
      <CardContent className="space-y-4">
        {/* Description */}
        {node.description && (
          <div>
            <h4 className="text-xs font-medium text-muted-foreground mb-1">Description</h4>
            <p className="text-sm">{node.description}</p>
          </div>
        )}

        {/* Properties */}
        {node.properties && Object.keys(node.properties).length > 0 && (
          <div>
            <h4 className="text-xs font-medium text-muted-foreground mb-1">Properties</h4>
            <div className="space-y-1">
              {Object.entries(node.properties).map(([key, value]) => (
                <div key={key} className="flex justify-between text-sm">
                  <span className="text-muted-foreground">{key}</span>
                  <span>{String(value)}</span>
                </div>
              ))}
            </div>
          </div>
        )}

        {/* Metadata */}
        <div>
          <h4 className="text-xs font-medium text-muted-foreground mb-1">Metadata</h4>
          <div className="space-y-1 text-sm">
            <div className="flex justify-between">
              <span className="text-muted-foreground">ID</span>
              <span className="font-mono text-xs">{node.id.slice(0, 8)}...</span>
            </div>
            <div className="flex justify-between">
              <span className="text-muted-foreground">Sources</span>
              <span>{node.source_ids.length} documents</span>
            </div>
            <div className="flex justify-between">
              <span className="text-muted-foreground">Created</span>
              <span>{formatDistanceToNow(new Date(node.created_at), { addSuffix: true })}</span>
            </div>
          </div>
        </div>

        <Separator />

        {/* Relationships */}
        <div>
          <h4 className="text-xs font-medium text-muted-foreground mb-2">
            Relationships ({connectedEdges.length})
          </h4>
          <ScrollArea className="h-32">
            <div className="space-y-2">
              {connectedEdges.map((edge) => {
                const isSource = edge.source === node.id;
                const otherNodeId = isSource ? edge.target : edge.source;
                
                return (
                  <div
                    key={edge.id}
                    className="flex items-center gap-2 text-sm cursor-pointer hover:bg-muted p-1 rounded"
                    onClick={() => focusNode(otherNodeId)}
                  >
                    <span className={isSource ? 'text-blue-500' : 'text-green-500'}>
                      {isSource ? '→' : '←'}
                    </span>
                    <span className="text-muted-foreground text-xs">{edge.relationship_type}</span>
                    <span className="flex-1 truncate">{otherNodeId.slice(0, 12)}...</span>
                  </div>
                );
              })}
            </div>
          </ScrollArea>
        </div>

        <Separator />

        {/* Actions */}
        <div className="flex gap-2">
          <Button variant="outline" size="sm" className="flex-1">
            <GitMerge className="h-4 w-4 mr-1" />
            Merge
          </Button>
          <Button variant="outline" size="sm" className="flex-1 text-destructive">
            <Trash2 className="h-4 w-4 mr-1" />
            Delete
          </Button>
        </div>
      </CardContent>
    </Card>
  );
}

export default NodeDetails;
