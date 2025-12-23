'use client';

import { Skeleton } from '@/components/ui/skeleton';

/**
 * Skeleton loader for the document table.
 * Shows a loading placeholder that matches the shape of the document list.
 */
export function DocumentTableSkeleton() {
  return (
    <div className="space-y-3">
      {/* Header row */}
      <div className="flex items-center gap-4 px-4">
        <Skeleton className="h-8 w-[200px]" />
        <Skeleton className="h-8 w-[100px]" />
        <div className="flex-1" />
        <Skeleton className="h-8 w-[120px]" />
      </div>
      {/* Table rows */}
      {Array.from({ length: 5 }).map((_, i) => (
        <div
          key={i}
          className="flex items-center gap-4 p-4 border rounded-lg animate-pulse"
          style={{ animationDelay: `${i * 100}ms` }}
        >
          <Skeleton className="h-5 w-5 rounded" />
          <Skeleton className="h-4 flex-1 max-w-[300px]" />
          <Skeleton className="h-6 w-[80px] rounded-full" />
          <Skeleton className="h-4 w-[60px]" />
          <Skeleton className="h-4 w-[100px]" />
          <Skeleton className="h-8 w-8 rounded" />
        </div>
      ))}
    </div>
  );
}

/**
 * Skeleton loader for the graph viewer.
 * Shows a loading placeholder for the knowledge graph visualization.
 */
export function GraphViewerSkeleton() {
  return (
    <div className="flex-1 relative bg-muted/10">
      {/* Graph area */}
      <Skeleton className="absolute inset-4 rounded-lg" />
      
      {/* Controls overlay */}
      <div className="absolute bottom-4 left-4 flex flex-col gap-2">
        <Skeleton className="h-10 w-10 rounded" />
        <Skeleton className="h-10 w-10 rounded" />
        <Skeleton className="h-10 w-10 rounded" />
      </div>
      
      {/* Zoom controls */}
      <div className="absolute top-4 right-4 flex flex-col gap-2">
        <Skeleton className="h-8 w-8 rounded" />
        <Skeleton className="h-8 w-8 rounded" />
      </div>
      
      {/* Legend */}
      <div className="absolute bottom-4 right-4">
        <Skeleton className="h-32 w-48 rounded-lg" />
      </div>
    </div>
  );
}

/**
 * Skeleton loader for the query interface.
 * Shows a loading placeholder for the chat/query view.
 */
export function QueryInterfaceSkeleton() {
  return (
    <div className="flex flex-col h-full">
      {/* Messages area */}
      <div className="flex-1 p-4 space-y-4">
        {/* User message */}
        <div className="flex items-start gap-3 justify-end">
          <Skeleton className="h-12 w-2/3 rounded-lg" />
          <Skeleton className="h-8 w-8 rounded-full" />
        </div>
        {/* Assistant message */}
        <div className="flex items-start gap-3">
          <Skeleton className="h-8 w-8 rounded-full" />
          <div className="space-y-2 flex-1 max-w-3xl">
            <Skeleton className="h-4 w-full" />
            <Skeleton className="h-4 w-5/6" />
            <Skeleton className="h-4 w-4/6" />
          </div>
        </div>
        {/* Another user message */}
        <div className="flex items-start gap-3 justify-end">
          <Skeleton className="h-10 w-1/2 rounded-lg" />
          <Skeleton className="h-8 w-8 rounded-full" />
        </div>
      </div>
      
      {/* Input area */}
      <div className="p-4 border-t">
        <Skeleton className="h-12 w-full rounded-lg" />
      </div>
    </div>
  );
}

/**
 * Skeleton loader for the dashboard.
 * Shows a loading placeholder for the main dashboard view.
 */
export function DashboardSkeleton() {
  return (
    <div className="p-6 space-y-6">
      {/* Title */}
      <div className="space-y-2">
        <Skeleton className="h-8 w-[200px]" />
        <Skeleton className="h-4 w-[300px]" />
      </div>
      
      {/* Stats cards */}
      <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-4">
        {Array.from({ length: 4 }).map((_, i) => (
          <Skeleton key={i} className="h-24 rounded-lg" />
        ))}
      </div>
      
      {/* Main content grid */}
      <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
        <Skeleton className="h-[300px] rounded-lg" />
        <Skeleton className="h-[300px] rounded-lg" />
      </div>
    </div>
  );
}

/**
 * Skeleton loader for settings page.
 */
export function SettingsSkeleton() {
  return (
    <div className="p-6 max-w-4xl mx-auto space-y-6">
      {/* Title */}
      <div className="space-y-2">
        <Skeleton className="h-8 w-[150px]" />
        <Skeleton className="h-4 w-[250px]" />
      </div>
      
      {/* Settings cards */}
      {Array.from({ length: 3 }).map((_, i) => (
        <div key={i} className="border rounded-lg p-6 space-y-4">
          <div className="flex items-center gap-2">
            <Skeleton className="h-5 w-5" />
            <Skeleton className="h-6 w-[120px]" />
          </div>
          <Skeleton className="h-4 w-[200px]" />
          <div className="space-y-3">
            <div className="flex justify-between items-center">
              <Skeleton className="h-4 w-[150px]" />
              <Skeleton className="h-8 w-[100px]" />
            </div>
            <div className="flex justify-between items-center">
              <Skeleton className="h-4 w-[120px]" />
              <Skeleton className="h-6 w-10 rounded-full" />
            </div>
          </div>
        </div>
      ))}
    </div>
  );
}
