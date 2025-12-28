/**
 * GitHub-style Alert Component
 * 
 * Renders GitHub-flavored markdown alerts with appropriate styling:
 * - NOTE (blue): Informational content
 * - TIP (green): Helpful tips and best practices
 * - WARNING (yellow): Important warnings
 * - CAUTION (red): Critical warnings
 * - IMPORTANT (purple): Important information
 */
'use client';

import { cn } from '@/lib/utils';
import {
    AlertTriangle,
    Info,
    Lightbulb,
    OctagonAlert,
    Star
} from 'lucide-react';
import { memo } from 'react';
import type { AlertType } from './utils/configure-marked';

interface GitHubAlertProps {
  type: AlertType;
  children: React.ReactNode;
  className?: string;
}

/**
 * Alert configuration with colors, icons, and labels
 */
const alertConfig: Record<AlertType, {
  icon: typeof Info;
  label: string;
  containerClass: string;
  iconClass: string;
  borderClass: string;
}> = {
  note: {
    icon: Info,
    label: 'Note',
    containerClass: 'bg-blue-50 dark:bg-blue-950/30',
    iconClass: 'text-blue-600 dark:text-blue-400',
    borderClass: 'border-blue-300 dark:border-blue-700',
  },
  tip: {
    icon: Lightbulb,
    label: 'Tip',
    containerClass: 'bg-emerald-50 dark:bg-emerald-950/30',
    iconClass: 'text-emerald-600 dark:text-emerald-400',
    borderClass: 'border-emerald-300 dark:border-emerald-700',
  },
  warning: {
    icon: AlertTriangle,
    label: 'Warning',
    containerClass: 'bg-amber-50 dark:bg-amber-950/30',
    iconClass: 'text-amber-600 dark:text-amber-400',
    borderClass: 'border-amber-300 dark:border-amber-700',
  },
  caution: {
    icon: OctagonAlert,
    label: 'Caution',
    containerClass: 'bg-red-50 dark:bg-red-950/30',
    iconClass: 'text-red-600 dark:text-red-400',
    borderClass: 'border-red-300 dark:border-red-700',
  },
  important: {
    icon: Star,
    label: 'Important',
    containerClass: 'bg-purple-50 dark:bg-purple-950/30',
    iconClass: 'text-purple-600 dark:text-purple-400',
    borderClass: 'border-purple-300 dark:border-purple-700',
  },
};

/**
 * GitHub-style alert component
 */
export const GitHubAlert = memo(function GitHubAlert({
  type,
  children,
  className,
}: GitHubAlertProps) {
  const config = alertConfig[type] || alertConfig.note;
  const Icon = config.icon;
  
  return (
    <div
      className={cn(
        'my-4 rounded-lg border-l-4 p-4',
        config.containerClass,
        config.borderClass,
        className
      )}
      role="alert"
    >
      <div className="flex items-start gap-3">
        <Icon 
          className={cn('h-5 w-5 mt-0.5 flex-shrink-0', config.iconClass)} 
          aria-hidden="true"
        />
        <div className="flex-1 min-w-0">
          <p 
            className={cn(
              'font-semibold text-sm mb-1',
              config.iconClass
            )}
          >
            {config.label}
          </p>
          <div className="text-sm text-foreground/80 prose-sm">
            {children}
          </div>
        </div>
      </div>
    </div>
  );
});

export default GitHubAlert;
