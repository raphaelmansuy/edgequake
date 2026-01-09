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

import { cn } from '@/lib/utils';
import type { Document } from '@/types';
import { Clock, FileText, Link2, Network } from 'lucide-react';

interface KeyStatsProps {
  document: Document;
}

export function KeyStats({ document }: KeyStatsProps) {
  return (
    <div className="grid grid-cols-2 gap-3">
      <StatCard
        icon={<FileText className="h-4 w-4" />}
        label="Chunks"
        value={document.chunk_count ?? '-'}
        color="blue"
      />
      <StatCard
        icon={<Network className="h-4 w-4" />}
        label="Entities"
        value={document.entity_count ?? '-'}
        color="purple"
      />
      <StatCard
        icon={<Link2 className="h-4 w-4" />}
        label="Relations"
        value={document.relationship_count ?? '-'}
        color="green"
      />
      <StatCard
        icon={<Clock className="h-4 w-4" />}
        label="Processed"
        value={formatDuration(document.lineage?.processing_duration_ms)}
        color="orange"
      />
    </div>
  );
}

interface StatCardProps {
  icon: React.ReactNode;
  label: string;
  value: string | number;
  color: 'blue' | 'purple' | 'green' | 'orange';
}

function StatCard({ icon, label, value, color }: StatCardProps) {
  const colorClasses = {
    blue: 'bg-blue-500/10 text-blue-600 dark:text-blue-400 border-blue-200 dark:border-blue-900',
    purple: 'bg-purple-500/10 text-purple-600 dark:text-purple-400 border-purple-200 dark:border-purple-900',
    green: 'bg-green-500/10 text-green-600 dark:text-green-400 border-green-200 dark:border-green-900',
    orange: 'bg-orange-500/10 text-orange-600 dark:text-orange-400 border-orange-200 dark:border-orange-900',
  };

  return (
    <div 
      className={cn(
        'flex flex-col gap-1.5 p-3 rounded-lg border bg-card',
        'hover:scale-[1.02] hover:-translate-y-0.5',
        'transition-all duration-200 ease-out cursor-default',
        'shadow-sm hover:shadow-md'
      )}
    >
      <div className={cn('flex items-center gap-1.5 text-xs font-medium', colorClasses[color].split(' ').filter(c => c.includes('text')))}>
        <div className={cn('p-1 rounded', colorClasses[color].split(' ').filter(c => c.includes('bg')))}>
          {icon}
        </div>
        <span>{label}</span>
      </div>
      <div className="text-2xl font-bold tabular-nums">{value}</div>
    </div>
  );
}

function formatDuration(ms?: number): string {
  if (!ms) return '-';
  if (ms < 1000) return `${ms}ms`;
  if (ms < 60000) return `${(ms / 1000).toFixed(1)}s`;
  return `${(ms / 60000).toFixed(1)}m`;
}
