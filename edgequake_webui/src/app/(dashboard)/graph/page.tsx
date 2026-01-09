/**
 * @module GraphPage
 * @description Knowledge graph visualization page route.
 *
 * @implements FEAT0601 - Interactive graph visualization
 * @see GraphViewer component for full implementation
 */
'use client';

import { Skeleton } from '@/components/ui/skeleton';
import { useGraphStore } from '@/stores/use-graph-store';
import dynamic from 'next/dynamic';
import { useSearchParams } from 'next/navigation';
import { useEffect } from 'react';

// Dynamic import for GraphViewer since it uses browser APIs (Sigma.js)
const GraphViewer = dynamic(
  () => import('@/components/graph/graph-viewer'),
  {
    ssr: false,
    loading: () => (
      <div className="flex h-full">
        <div className="flex-1 flex flex-col">
          <div className="flex items-center justify-between border-b px-4 py-2">
            <div className="flex items-center gap-2">
              <Skeleton className="h-6 w-32" />
              <Skeleton className="h-4 w-24" />
            </div>
            <div className="flex items-center gap-1">
              <Skeleton className="h-8 w-8" />
              <Skeleton className="h-8 w-8" />
              <Skeleton className="h-8 w-8" />
            </div>
          </div>
          <div className="flex-1 flex items-center justify-center">
            <Skeleton className="h-64 w-64 rounded-full" />
          </div>
        </div>
        <div className="w-80 border-l bg-card p-4 space-y-4">
          <Skeleton className="h-32 w-full" />
          <Skeleton className="h-48 w-full" />
        </div>
      </div>
    ),
  }
);

// Dynamic import for tour wrapper (client-only)
const GraphTourWrapper = dynamic(
  () => import('@/components/graph/graph-tour-wrapper'),
  { ssr: false }
);

export default function GraphPage() {
  const searchParams = useSearchParams();
  const { setSearchQuery, setStartNode, nodes } = useGraphStore();
  
  // Handle URL parameters for deep linking from query results
  useEffect(() => {
    const entities = searchParams.get('entities');
    const focus = searchParams.get('focus');
    const entity = searchParams.get('entity');
    
    // If entities filter is provided, set as search query
    if (entities) {
      // Use the first entity as a search filter
      const entityList = entities.split(',');
      if (entityList.length > 0) {
        setSearchQuery(entityList[0]);
      }
    }
    
    // If focus or entity is specified, try to set it as the start node
    const targetEntity = focus || entity;
    if (targetEntity && nodes.length > 0) {
      // Find matching node
      const matchingNode = nodes.find(
        n => n.label?.toLowerCase() === targetEntity.toLowerCase() ||
             n.id?.toLowerCase() === targetEntity.toLowerCase()
      );
      if (matchingNode) {
        setStartNode(matchingNode.id);
      }
    }
  }, [searchParams, setSearchQuery, setStartNode, nodes]);
  
  return (
    <div className="h-full overflow-hidden">
      <GraphTourWrapper>
        <GraphViewer />
      </GraphTourWrapper>
    </div>
  );
}
