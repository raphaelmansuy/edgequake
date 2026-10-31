'use client';

import { Button } from '@/components/ui/button';

/** A titled block of label/value rows on the wizard review step. */
export function Group({
  title,
  onEdit,
  editLabel,
  children,
  testId,
}: {
  title: string;
  onEdit?: () => void;
  editLabel: string;
  children: React.ReactNode;
  testId: string;
}) {
  return (
    <div className="rounded-md border p-2.5 space-y-1.5" data-testid={testId}>
      <div className="flex items-center justify-between gap-2">
        <h4 className="text-xs font-semibold uppercase tracking-wide text-muted-foreground">
          {title}
        </h4>
        {onEdit ? (
          <Button
            type="button"
            variant="link"
            className="h-auto p-0 text-xs"
            onClick={onEdit}
            data-testid={`${testId}-edit`}
          >
            {editLabel}
          </Button>
        ) : null}
      </div>
      <dl className="space-y-1">{children}</dl>
    </div>
  );
}

/** One label/value line. */
export function Row({
  label,
  value,
  mono,
}: {
  label: string;
  value: string;
  mono?: boolean;
}) {
  return (
    <div className="flex gap-2 text-sm items-start">
      <dt className="w-32 shrink-0 text-muted-foreground">{label}</dt>
      <dd className={`font-medium break-all flex-1 ${mono ? 'font-mono text-xs' : ''}`}>
        {value || '—'}
      </dd>
    </div>
  );
}
