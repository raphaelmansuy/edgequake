'use client';

/**
 * SPEC-146 share-mode badge — same Badge family as ClassificationChip.
 */

import { Badge } from '@/components/ui/badge';
import {
  shareModeBadgeVariant,
  shareModeLabel,
} from '@/lib/security/security-fields';
import { cn } from '@/lib/utils';
import { useTranslation } from 'react-i18next';

export function ShareModeBadge({
  value,
  className,
}: {
  value?: string | null;
  className?: string;
}) {
  const { t } = useTranslation();
  const key = value ?? 'workspace';
  return (
    <Badge
      variant={shareModeBadgeVariant(key)}
      className={cn(
        'shrink-0 whitespace-nowrap text-[10px] px-1.5 py-0',
        className,
      )}
      data-testid="spec146-share-chip"
    >
      {t(`security.shareMode.${key}`, shareModeLabel(key))}
    </Badge>
  );
}
