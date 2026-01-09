/**
 * @fileoverview Quick action navigation cards for common tasks
 *
 * @implements FEAT1010 - Quick action shortcuts
 * @implements FEAT1011 - Dashboard navigation widgets
 *
 * @see UC1103 - User navigates to documents/query/graph
 * @see UC1104 - User accesses primary workflows quickly
 *
 * @enforces BR1010 - Internationalized action labels
 * @enforces BR1011 - Accessible keyboard navigation
 */
'use client';

import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { cn } from '@/lib/utils';
import { FileText, MessageSquare, Network } from 'lucide-react';
import Link from 'next/link';
import { useTranslation } from 'react-i18next';

const actions = [
  {
    id: 'upload',
    href: '/documents',
    icon: FileText,
    labelKey: 'dashboard.quickActions.upload',
    descriptionKey: 'dashboard.quickActions.uploadDesc',
    color: 'text-blue-500',
    bgColor: 'bg-blue-500/10 hover:bg-blue-500/20',
  },
  {
    id: 'query',
    href: '/query',
    icon: MessageSquare,
    labelKey: 'dashboard.quickActions.query',
    descriptionKey: 'dashboard.quickActions.queryDesc',
    color: 'text-purple-500',
    bgColor: 'bg-purple-500/10 hover:bg-purple-500/20',
  },
  {
    id: 'graph',
    href: '/graph',
    icon: Network,
    labelKey: 'dashboard.quickActions.graph',
    descriptionKey: 'dashboard.quickActions.graphDesc',
    color: 'text-green-500',
    bgColor: 'bg-green-500/10 hover:bg-green-500/20',
  },
];

export function QuickActions() {
  const { t } = useTranslation();

  return (
    <Card className="border-0 shadow-sm">
      <CardHeader className="pb-3">
        <CardTitle className="text-base">{t('dashboard.quickActions.title', 'Quick Actions')}</CardTitle>
        <CardDescription className="text-xs">
          {t('dashboard.quickActions.subtitle', 'Get started with common tasks')}
        </CardDescription>
      </CardHeader>
      <CardContent className="pt-0">
        <div className="grid gap-3 sm:grid-cols-3">
          {actions.map((action) => {
            const Icon = action.icon;
            return (
              <Link
                key={action.id}
                href={action.href}
                className={cn(
                  'flex flex-col items-center justify-center gap-2 rounded-lg border p-4 transition-all duration-200',
                  action.bgColor,
                  'hover:border-primary/50 hover:shadow-md hover:-translate-y-0.5'
                )}
              >
                <div className={cn('rounded-full p-2.5', action.bgColor)}>
                  <Icon className={cn('h-5 w-5', action.color)} />
                </div>
                <div className="text-center">
                  <p className="text-sm font-medium">{t(action.labelKey)}</p>
                  <p className="text-[11px] text-muted-foreground mt-0.5 line-clamp-2">
                    {t(action.descriptionKey)}
                  </p>
                </div>
              </Link>
            );
          })}
        </div>
      </CardContent>
    </Card>
  );
}
