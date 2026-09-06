'use client';

/**
 * SPEC-146 classification chip — Badge variants only (no parallel chip library).
 */

import { Badge } from '@/components/ui/badge';
import {
  classificationBadgeVariant,
  classificationLabel,
} from '@/lib/security/security-fields';
import { cn } from '@/lib/utils';
import { useTranslation } from 'react-i18next';

export function ClassificationChip({
  value,
  className,
}: {
  value?: string | null;
  className?: string;
}) {
  const { t } = useTranslation();
  const key = (value ?? 'internal').toLowerCase();
  return (
    <Badge
      variant={classificationBadgeVariant(key)}
      className={cn(
        'shrink-0 whitespace-nowrap text-[10px] px-1.5 py-0',
        className,
      )}
      data-testid="spec146-class-chip"
    >
      {t(`security.classification.${key}`, classificationLabel(key))}
    </Badge>
  );
}
