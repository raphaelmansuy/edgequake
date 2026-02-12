/**
 * @module GraphPage
 * @description Knowledge graph visualization page route.
 *
 * @implements FEAT0601 - Interactive graph visualization
 * @see GraphViewer component for full implementation
 */
'use client';

import { GraphLoadingOverlay } from '@/components/graph/graph-loading-overlay';
import { useGraphStore } from '@/stores/use-graph-store';
import dynamic from 'next/dynamic';
import { useSearchParams } from 'next/navigation';
import { useEffect } from 'react';

/**
 * WHY: Both dynamic imports use GraphLoadingOverlay as their loading fallback.
 * Without this, the main content area is completely EMPTY during JS bundle loading
 * (GraphTourWrapper had no fallback → rendered null → blank screen for seconds).
 * The overlay provides immediate visual feedback from the moment the user navigates
 * to /graph, through bundle loading, through data fetching, to final graph render.
 */
const GraphLoadingFallback = () => (
  <div className="relative h-full w-full">
    <GraphLoadingOverlay visible={true} phase="Loading graph viewer..." />
  </div>
);

// Dynamic import for GraphViewer since it uses browser APIs (Sigma.js)
const GraphViewer = dynamic(
  () => import('@/components/graph/graph-viewer'),
  {
    ssr: false,
    loading: GraphLoadingFallback,
  }
);

// Dynamic import for tour wrapper (client-only)
// WHY: Previously had NO loading fallback → rendered null → empty main area
const GraphTourWrapper = dynamic(
  () => import('@/components/graph/graph-tour-wrapper'),
  {
    ssr: false,
    loading: GraphLoadingFallback,
  }
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
