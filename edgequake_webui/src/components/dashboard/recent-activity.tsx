/**
 * @fileoverview Recent activity feed showing document processing status
 *
 * @implements FEAT1020 - Activity feed with document status
 * @implements FEAT1021 - Processing status indicators
 *
 * @see UC1105 - User monitors document ingestion status
 * @see UC1106 - User reviews recent uploads
 *
 * @enforces BR1020 - Status icons with animation for processing
 * @enforces BR1021 - Empty state with call-to-action
 */
'use client';

import { Badge } from '@/components/ui/badge';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { ScrollArea } from '@/components/ui/scroll-area';
import { Skeleton } from '@/components/ui/skeleton';
import type { Document } from '@/types';
import { formatDistanceToNow } from 'date-fns';
import { CheckCircle, Clock, FileText, Loader2, StopCircle, XCircle } from 'lucide-react';
import Link from 'next/link';
import { useTranslation } from 'react-i18next';

interface RecentActivityProps {
  documents: Document[];
  isLoading?: boolean;
}

const statusConfig = {
  pending: { icon: Clock, color: 'text-yellow-500', label: 'Pending', animate: false },
  processing: { icon: Loader2, color: 'text-blue-500', label: 'Processing', animate: true },
  completed: { icon: CheckCircle, color: 'text-green-500', label: 'Completed', animate: false },
  indexed: { icon: CheckCircle, color: 'text-green-500', label: 'Indexed', animate: false },
  failed: { icon: XCircle, color: 'text-red-500', label: 'Failed', animate: false },
  partial_failure: { icon: XCircle, color: 'text-orange-500', label: 'Partial Failure', animate: false },
  cancelled: { icon: StopCircle, color: 'text-gray-500', label: 'Cancelled', animate: false },
} as const;

export function RecentActivity({ documents, isLoading }: RecentActivityProps) {
  const { t } = useTranslation();

  if (isLoading) {
    return (
      <Card>
        <CardHeader>
          <CardTitle className="text-lg">{t('dashboard.recentActivity.title', 'Recent Activity')}</CardTitle>
          <CardDescription>
            {t('dashboard.recentActivity.subtitle', 'Latest document uploads and processing')}
          </CardDescription>
        </CardHeader>
        <CardContent>
          <div className="space-y-3">
            {Array.from({ length: 5 }).map((_, i) => (
              <div key={i} className="flex items-center gap-3 p-3 rounded-lg border">
                <Skeleton className="h-8 w-8 rounded" />
                <div className="flex-1 space-y-1">
                  <Skeleton className="h-4 w-32" />
                  <Skeleton className="h-3 w-24" />
                </div>
                <Skeleton className="h-5 w-16" />
              </div>
            ))}
          </div>
        </CardContent>
      </Card>
    );
  }

  return (
    <Card>
      <CardHeader>
        <CardTitle className="text-lg">{t('dashboard.recentActivity.title', 'Recent Activity')}</CardTitle>
        <CardDescription>
          {t('dashboard.recentActivity.subtitle', 'Latest document uploads and processing')}
        </CardDescription>
      </CardHeader>
      <CardContent>
        {documents.length === 0 ? (
          <div className="flex flex-col items-center justify-center py-8 text-center">
            <div className="w-12 h-12 rounded-full bg-muted flex items-center justify-center mb-3">
              <FileText className="h-6 w-6 text-muted-foreground" />
            </div>
            <p className="text-sm text-muted-foreground">
              {t('dashboard.recentActivity.noActivity', 'No recent activity')}
            </p>
            <Link 
              href="/documents" 
              className="text-sm text-primary hover:underline mt-2"
            >
              {t('dashboard.recentActivity.uploadFirst', 'Upload your first document')}
            </Link>
          </div>
        ) : (
          <ScrollArea className="h-[300px] pr-4">
            <div className="space-y-2 py-1">
              {documents.map((doc) => {
                const status = doc.status || 'completed';
                const config = statusConfig[status as keyof typeof statusConfig] || statusConfig.completed;
                const StatusIcon = config.icon;

                return (
                  <Link
                    key={doc.id}
                    href={`/documents?id=${doc.id}`}
                    className="flex items-center gap-3 p-3 rounded-lg border transition-colors hover:bg-muted/50"
                  >
                    <div className="flex h-8 w-8 items-center justify-center rounded bg-muted">
                      <FileText className="h-4 w-4 text-muted-foreground" />
                    </div>
                    <div className="flex-1 min-w-0">
                      <p className="text-sm font-medium truncate">
                        {doc.title || doc.file_name || 'Untitled'}
                      </p>
                      <p className="text-xs text-muted-foreground">
                        {doc.created_at 
                          ? formatDistanceToNow(new Date(doc.created_at), { addSuffix: true })
                          : 'Unknown'}
                      </p>
                    </div>
                    <Badge variant="outline" className="shrink-0 gap-1">
                      <StatusIcon 
                        className={`h-3 w-3 ${config.color} ${config.animate ? 'animate-spin' : ''}`} 
                      />
                      <span className="text-xs">{config.label}</span>
                    </Badge>
                  </Link>
                );
              })}
            </div>
          </ScrollArea>
        )}
      </CardContent>
    </Card>
  );
}
