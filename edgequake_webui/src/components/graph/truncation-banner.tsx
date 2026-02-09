'use client';

import { Button } from '@/components/ui/button';
import { useGraphStore } from '@/stores/use-graph-store';
import { AlertTriangle, ChevronRight, Database } from 'lucide-react';
import { useCallback } from 'react';

interface TruncationBannerProps {
  /** Callback when "Load More" is clicked */
  onLoadMore?: () => void;
  /** Whether loading more is in progress */
  isLoading?: boolean;
}

/**
 * TruncationBanner - Visual feedback when graph data is truncated.
 * 
 * Shows:
 * - Current visible node count
 * - Total node count in storage
 * - Percentage of data shown
 * - Optional "Load More" action
 */
export function TruncationBanner({ onLoadMore, isLoading }: TruncationBannerProps) {
  const { 
    isTruncated, 
    nodes,
    totalNodesInStorage, 
    maxNodes,
    setMaxNodes,
  } = useGraphStore();

  const handleLoadMore = useCallback(() => {
    if (onLoadMore) {
      onLoadMore();
    } else {
      // WHY: Enforce max 500 nodes for performance - graphs become unreadable beyond this
      const newMax = Math.min(maxNodes * 1.5, 500);
      setMaxNodes(Math.round(newMax));
    }
  }, [onLoadMore, maxNodes, setMaxNodes]);

  // Don't show if not truncated or no data
  if (!isTruncated || totalNodesInStorage === 0) {
    return null;
  }

  const visibleCount = nodes.length;
  const percentage = Math.round((visibleCount / totalNodesInStorage) * 100);
  const hasMore = visibleCount < totalNodesInStorage;

  return (
    <div className="absolute bottom-4 left-1/2 -translate-x-1/2 z-50">
      <div className="flex items-center gap-3 bg-amber-500/90 dark:bg-amber-600/90 text-white px-4 py-2 rounded-full shadow-lg backdrop-blur-sm">
        <AlertTriangle className="h-4 w-4 shrink-0" />
        
        <div className="flex items-center gap-2 text-sm font-medium">
          <span className="tabular-nums">
            {visibleCount.toLocaleString()}
          </span>
          <span className="opacity-80">of</span>
          <span className="tabular-nums">
            {totalNodesInStorage.toLocaleString()}
          </span>
          <span className="opacity-80">nodes</span>
          <span className="text-xs opacity-70 ml-1">
            ({percentage}%)
          </span>
        </div>

        {hasMore && (
          <Button
            variant="secondary"
            size="sm"
            className="h-6 text-xs px-2 bg-white/20 hover:bg-white/30 text-white border-0"
            onClick={handleLoadMore}
            disabled={isLoading}
          >
            {isLoading ? (
              <div className="h-3 w-3 animate-spin rounded-full border-2 border-white border-t-transparent" />
            ) : (
              <>
                Load More
                <ChevronRight className="h-3 w-3 ml-1" />
              </>
            )}
          </Button>
        )}
      </div>
    </div>
  );
}

/**
 * Compact version for toolbar display.
 */
export function TruncationIndicator() {
  const { 
    isTruncated, 
    nodes,
    totalNodesInStorage, 
  } = useGraphStore();

  if (!isTruncated || totalNodesInStorage === 0) {
    return null;
  }

  const visibleCount = nodes.length;
  const percentage = Math.round((visibleCount / totalNodesInStorage) * 100);

  return (
    <div className="flex items-center gap-1.5 px-2 py-1 rounded-md bg-amber-100 dark:bg-amber-900/50 text-amber-700 dark:text-amber-300 text-xs">
      <Database className="h-3 w-3" />
      <span className="tabular-nums font-medium">
        {visibleCount.toLocaleString()} / {totalNodesInStorage.toLocaleString()}
      </span>
      <span className="opacity-70">({percentage}%)</span>
    </div>
  );
}
