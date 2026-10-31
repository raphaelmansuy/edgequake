/**
 * @fileoverview Key stats card (sticky at top of sidebar)
 *
 * @implements FEAT1086 - Key statistics summary
 * @implements FEAT1087 - Processing duration display
 *
 * @see UC1517 - User sees document statistics at glance
 * @see UC1518 - User views processing time
 *
 * @enforces BR1086 - Color-coded stat cards
 * @enforces BR1087 - Sticky positioning in sidebar
 */
// Key stats card (sticky at top of sidebar)
'use client';

import { useWorkspaceCapabilities } from '@/hooks/use-workspace-capabilities';
import { cn } from '@/lib/utils';
import type { Document } from '@/types';
import { Clock, FileText, Link2, Network } from 'lucide-react';

interface KeyStatsProps {
  document: Document;
}

export function KeyStats({ document }: KeyStatsProps) {
  // SPEC-180: a chunk-only (classical RAG) workspace has no entities or relationships. Two
  // permanent "0" cards imply an extraction that failed; they simply do not apply.
  const extractsGraph = useWorkspaceCapabilities().extracts_graph;
  return (
    <div className="eq-metric-grid gap-2">
      <StatCard
        icon={<FileText className="h-4 w-4" />}
        label="Chunks"
        value={document.chunk_count ?? '-'}
        color="blue"
        testId="key-stats-chunks"
      />
      {extractsGraph ? (
        <>
          <StatCard
            icon={<Network className="h-4 w-4" />}
            label="Entities"
            value={document.entity_count ?? '-'}
            color="purple"
            testId="key-stats-entities"
          />
          <StatCard
            icon={<Link2 className="h-4 w-4" />}
            label="Relations"
            value={document.relationship_count ?? '-'}
            color="green"
            testId="key-stats-relations"
          />
        </>
      ) : null}
      <StatCard
        icon={<Clock className="h-4 w-4" />}
        label="Processed"
        value={formatDuration(document.lineage?.processing_duration_ms)}
        color="orange"
        testId="key-stats-processed"
      />
    </div>
  );
}

interface StatCardProps {
  icon: React.ReactNode;
  label: string;
  value: string | number;
  color: 'blue' | 'purple' | 'green' | 'orange';
  testId?: string;
}

function StatCard({ icon, label, value, color, testId }: StatCardProps) {
  const colorClasses = {
    blue: 'bg-blue-500/10 text-blue-600 dark:text-blue-400 border-blue-200 dark:border-blue-900',
    purple: 'bg-purple-500/10 text-purple-600 dark:text-purple-400 border-purple-200 dark:border-purple-900',
    green: 'bg-green-500/10 text-green-600 dark:text-green-400 border-green-200 dark:border-green-900',
    orange: 'bg-orange-500/10 text-orange-600 dark:text-orange-400 border-orange-200 dark:border-orange-900',
  };

  return (
    <div 
      data-testid={testId}
      className={cn(
        'flex min-w-0 flex-col gap-1 overflow-hidden p-2.5 rounded-lg border bg-card',
        'transition-all duration-200 ease-out cursor-default',
      )}
    >
      <div className={cn('flex items-center gap-1.5 text-xs font-medium', colorClasses[color].split(' ').filter(c => c.includes('text')))}>
        <div className={cn('p-1 rounded', colorClasses[color].split(' ').filter(c => c.includes('bg')))}>
          {icon}
        </div>
        <span>{label}</span>
      </div>
      <div className="text-xl font-bold tabular-nums">{value}</div>
    </div>
  );
}

function formatDuration(ms?: number): string {
  if (!ms) return '-';
  if (ms < 1000) return `${ms}ms`;
  if (ms < 60000) return `${(ms / 1000).toFixed(1)}s`;
  return `${(ms / 60000).toFixed(1)}m`;
}
