/**
 * SPEC-168 — one keycap look for the whole app (palette, help, sidebar, header).
 */
'use client';

import { isApplePlatform } from '@/lib/keyboard/dom';
import { MOD } from '@/lib/keyboard/catalog';
import { cn } from '@/lib/utils';
import { useSyncExternalStore, type ReactNode } from 'react';
import { useTranslation } from 'react-i18next';

const subscribe = () => () => {};

/**
 * Platform-aware modifier glyph. Server and first client render agree on
 * "Ctrl"; the snapshot flips to ⌘ on Apple platforms without hydration warnings.
 */
export function useModLabel(): string {
  const apple = useSyncExternalStore(
    subscribe,
    () => isApplePlatform(),
    () => false,
  );
  return apple ? '⌘' : 'Ctrl';
}

export function Kbd({ children, className }: { children: ReactNode; className?: string }) {
  return (
    <kbd
      className={cn(
        'inline-flex h-5 min-w-5 items-center justify-center rounded border bg-muted px-1.5',
        'font-sans text-[11px] font-medium leading-none text-muted-foreground',
        className,
      )}
    >
      {children}
    </kbd>
  );
}

/** Renders a key list, resolving the `Mod` placeholder to the platform glyph. */
export function KeyCombo({
  keys,
  kind = 'combo',
  className,
}: {
  keys: readonly string[];
  kind?: 'combo' | 'sequence' | 'single';
  className?: string;
}) {
  const mod = useModLabel();
  const { t } = useTranslation();
  return (
    <span className={cn('inline-flex items-center gap-1', className)}>
      {keys.map((k, i) => (
        <span key={`${k}-${i}`} className="inline-flex items-center gap-1">
          {i > 0 && kind === 'sequence' && (
            <span className="text-[10px] text-muted-foreground">{t('keyboard.then', 'then')}</span>
          )}
          <Kbd>{k === MOD ? mod : k}</Kbd>
        </span>
      ))}
    </span>
  );
}
