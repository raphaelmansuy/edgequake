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
    <Card>
      <CardHeader>
        <CardTitle className="text-lg">{t('dashboard.quickActions.title', 'Quick Actions')}</CardTitle>
        <CardDescription>
          {t('dashboard.quickActions.subtitle', 'Get started with common tasks')}
        </CardDescription>
      </CardHeader>
      <CardContent>
        <div className="grid gap-4 sm:grid-cols-3">
          {actions.map((action) => {
            const Icon = action.icon;
            return (
              <Link
                key={action.id}
                href={action.href}
                className={cn(
                  'flex flex-col items-center justify-center gap-3 rounded-lg border p-6 transition-all duration-200',
                  action.bgColor,
                  'hover:border-primary/50 hover:shadow-md hover:-translate-y-0.5'
                )}
              >
                <div className={cn('rounded-full p-3', action.bgColor)}>
                  <Icon className={cn('h-6 w-6', action.color)} />
                </div>
                <div className="text-center">
                  <p className="font-medium">{t(action.labelKey)}</p>
                  <p className="text-xs text-muted-foreground mt-1">
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
