'use client';

/**
 * SPEC-168 — "which key" popup shown for ~1s after pressing `g`.
 * Fixed-position overlay: never moves page content (CLS-safe).
 */
import { Kbd } from '@/components/shared/kbd';
import { DESTINATIONS } from '@/lib/keyboard/destinations';
import { useTranslation } from 'react-i18next';

export function ChordHint({ armed }: { armed: boolean }) {
  const { t } = useTranslation();
  if (!armed) return null;

  return (
    <div
      role="status"
      aria-live="polite"
      data-testid="chord-hint"
      className="pointer-events-none fixed inset-x-0 bottom-6 z-[60] flex justify-center px-4"
    >
      <div className="max-w-[min(42rem,100%)] rounded-xl border bg-popover/95 px-4 py-3 text-popover-foreground shadow-lg backdrop-blur">
        <p className="mb-2 text-xs font-medium text-muted-foreground">
          {t('keyboard.chordPrompt', 'Go to…')}
        </p>
        <ul className="grid grid-cols-2 gap-x-6 gap-y-1.5 sm:grid-cols-5">
          {DESTINATIONS.map((d) => (
            <li key={d.href} className="flex items-center gap-2 text-sm">
              <Kbd>{d.chord.toUpperCase()}</Kbd>
              <span className="truncate">{t(d.labelKey, d.fallback)}</span>
            </li>
          ))}
        </ul>
      </div>
    </div>
  );
}
