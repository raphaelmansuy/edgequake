/**
 * @fileoverview Recent activity feed showing document processing status
 *
 * @implements FEAT1020 - Activity feed with document status
 * @implements FEAT1021 - Processing status indicators
 *
 * @see UC1105 - User monitors document ingestion status
 * @see UC1106 - User reviews recent uploads
 *
 * @enforces BR1020 - Status derived from the shared document status domain
 * @enforces BR1021 - Empty state with call-to-action
 */
'use client';

import { Card, CardAction, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { Skeleton } from '@/components/ui/skeleton';
import { buildActivityItem } from '@/lib/dashboard/activity-model';
import type { Document } from '@/types';
import { ArrowRight, FileText } from 'lucide-react';
import Link from 'next/link';
import { useMemo, type ReactNode } from 'react';
import { useTranslation } from 'react-i18next';
import { ActivityRow } from './activity-row';

/** Rows shown before deferring to the Documents page (no inner scroller). */
export const ACTIVITY_MAX_ROWS = 6;

/**
 * SPEC-100: reserve enough height for the 6-row skeleton so empty ↔ loading
 * does not shift the page (CLS). Measured skeleton card ≈ 480px at 1280×900.
 */
export const ACTIVITY_CARD_MIN_HEIGHT_PX = 480;

interface RecentActivityProps {
  documents: Document[];
  /** Workspace-wide document count, for the "View all" link. */
  total?: number;
  isLoading?: boolean;
  /** Optional action/status aligned with the section title (e.g. SystemStatus). */
  headerAction?: ReactNode;
}

function ActivitySkeleton() {
  // Title + time placeholders fit inside the reserved content height, including
  // when the first response is empty. Detail and footer arrive with real rows.
  return (
    <div className="divide-y divide-transparent" data-testid="spec100-dashboard-activity-skeleton">
      {Array.from({ length: ACTIVITY_MAX_ROWS }).map((_, i) => (
        <div key={i} className="flex min-h-14 items-start gap-3 px-3 py-2.5">
          <Skeleton className="mt-0.5 h-8 w-8 rounded-md" />
          <div className="flex-1 space-y-1.5">
            <Skeleton className="h-4 w-44" />
            <Skeleton className="h-3 w-24" />
          </div>
          <Skeleton className="h-5 w-20" />
        </div>
      ))}
    </div>
  );
}

function EmptyActivity() {
  const { t } = useTranslation();
  return (
    <div className="flex min-h-[336px] flex-col items-center justify-center py-8 text-center">
      <div className="mb-3 flex h-12 w-12 items-center justify-center rounded-full bg-muted">
        <FileText className="h-6 w-6 text-muted-foreground" />
      </div>
      <p className="text-sm text-muted-foreground">
        {t('dashboard.recentActivity.noActivity', 'No recent activity')}
      </p>
      <Link href="/documents" className="mt-2 text-sm text-primary hover:underline">
        {t('dashboard.recentActivity.uploadFirst', 'Upload your first document')}
      </Link>
    </div>
  );
}

export function RecentActivity({ documents, total, isLoading, headerAction }: RecentActivityProps) {
  const { t } = useTranslation();
  const items = useMemo(
    () => documents.slice(0, ACTIVITY_MAX_ROWS).map(buildActivityItem),
    [documents],
  );
  const count = total ?? documents.length;

  return (
    <Card
      data-testid="spec100-dashboard-activity"
      className="gap-3 py-4"
      style={{ minHeight: ACTIVITY_CARD_MIN_HEIGHT_PX }}
    >
      <CardHeader className="pb-1">
        <CardTitle>{t('dashboard.recentActivity.title', 'Recent Activity')}</CardTitle>
        <CardDescription className="text-xs">
          {t('dashboard.recentActivity.subtitle', 'Latest document uploads and processing')}
        </CardDescription>
        {headerAction ? (
          // Below the title on phones so a late status label can't re-wrap the heading.
          <CardAction className="col-start-1 row-span-1 row-start-3 justify-self-start sm:col-start-2 sm:row-span-2 sm:row-start-1 sm:justify-self-end">
            {headerAction}
          </CardAction>
        ) : null}
      </CardHeader>
      {/* SPEC-100: min-h keeps empty↔list↔skeleton from shifting the page (CLS). */}
      <CardContent className="min-h-[336px] px-2">
        {isLoading ? (
          <ActivitySkeleton />
        ) : items.length === 0 ? (
          <EmptyActivity />
        ) : (
          <div className="divide-y divide-border/50" data-testid="dashboard-activity-list">
            {items.map((item) => (
              <ActivityRow key={item.id} item={item} />
            ))}
          </div>
        )}
      </CardContent>
      {/* Reserve footer height in every state so skeleton ↔ empty stays within CLS budget. */}
      {isLoading || items.length === 0 ? (
        <div className="min-h-[2.75rem]" aria-hidden="true" />
      ) : (
        <div className="min-h-8 border-t px-4 pt-3">
          <Link
            href="/documents"
            data-testid="dashboard-activity-view-all"
            className="inline-flex items-center gap-1 text-sm font-medium text-primary hover:underline focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring rounded"
          >
            {t('dashboard.recentActivity.viewAll', {
              count,
              defaultValue_one: 'View all {{count}} document',
              defaultValue_other: 'View all {{count}} documents',
            })}
            <ArrowRight className="h-3.5 w-3.5" aria-hidden="true" />
          </Link>
        </div>
      )}
    </Card>
  );
}
