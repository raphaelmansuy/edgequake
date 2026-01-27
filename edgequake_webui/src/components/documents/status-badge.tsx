/**
 * @module DocumentStatusBadge
 * @description Status badge component for documents with appropriate icons and colors
 */
'use client';

import { Badge } from '@/components/ui/badge';
import {
  CheckCircle,
  Clock,
  Loader2,
  StopCircle,
  XCircle,
} from 'lucide-react';
import { memo } from 'react';

const statusConfig = {
  pending: { icon: Clock, color: 'bg-yellow-500', label: 'Pending', animate: false },
  processing: { icon: Loader2, color: 'bg-blue-500', label: 'Processing', animate: true },
  completed: { icon: CheckCircle, color: 'bg-green-500', label: 'Completed', animate: false },
  indexed: { icon: CheckCircle, color: 'bg-green-500', label: 'Indexed', animate: false },
  failed: { icon: XCircle, color: 'bg-red-500', label: 'Failed', animate: false },
  cancelled: { icon: StopCircle, color: 'bg-orange-500', label: 'Cancelled', animate: false },
} as const;

export type DocumentStatus = keyof typeof statusConfig;

interface StatusBadgeProps {
  status: DocumentStatus;
}

export const StatusBadge = memo(function StatusBadge({ status }: StatusBadgeProps) {
  const config = statusConfig[status] || statusConfig.completed;
  const Icon = config.icon;

  return (
    <Badge variant="outline" className="gap-1">
      <Icon className={`h-3 w-3 ${config.animate ? 'animate-spin' : ''}`} />
      {config.label}
    </Badge>
  );
});
